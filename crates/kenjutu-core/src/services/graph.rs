use kenjutu_types::ChangeId;

use crate::models::{CommitGraph, CommitRow, EdgeType, ElisionRow, GraphEdge, GraphRow, JjCommit};
use crate::services::jj::{self, Error};

/// Node characters that jj uses in graph gutters.
const NODE_CHARS: &[char] = &['@', '○', '◆', '●', '◉'];

/// Fetch jj log with graph output and parse it into a structured `CommitGraph`.
pub fn get_log_graph(local_dir: &str) -> jj::Result<CommitGraph> {
    // Use explicit \x00 concatenation instead of separate() because
    // separate() skips empty fields, changing the field count.
    let template = r#""\x01" ++ change_id ++ "\x00" ++ commit_id ++ "\x00" ++ description.escape_json() ++ "\x00" ++ author.name() ++ "\x00" ++ author.email() ++ "\x00" ++ author.timestamp() ++ "\x00" ++ immutable ++ "\x00" ++ current_working_copy ++ "\x00" ++ parents.map(|p| p.change_id()).join(",") ++ "\n""#;

    let mut cmd =
        jj::jj_command().ok_or_else(|| Error::Command("jj executable not found".to_string()))?;
    let output = cmd
        .args([
            "log",
            "--color",
            "never",
            "-r",
            "mutable() | ancestors(mutable(), 2)",
            "-T",
            template,
        ])
        .current_dir(local_dir)
        .output()
        .map_err(|e| Error::Command(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::JjFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_graph_output(&stdout)
}

// ── Raw line classification ─────────────────────────────────────────

/// A classified line from jj's graph output before layout processing.
enum RawLine {
    /// A commit node line: gutter + structured commit data (separated by \x01)
    Commit {
        gutter: String,
        commit: Box<JjCommit>,
    },
    /// A continuation line containing graph characters (e.g. "├─╮", "│")
    Continuation { line: String },
    /// An elision marker line (starts with "~")
    Elision { column: usize },
}

/// Parse jj graph output into classified raw lines.
fn parse_raw_lines(output: &str) -> jj::Result<Vec<RawLine>> {
    let mut lines = Vec::new();

    for line in output.lines() {
        if let Some(marker_pos) = line.find('\x01') {
            let gutter = line[..marker_pos].to_string();
            let data = &line[marker_pos + 1..];
            let commit = parse_commit_fields(data)?;
            lines.push(RawLine::Commit {
                gutter,
                commit: Box::new(commit),
            });
        } else {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with('~') {
                let col = find_char_column(line, '~');
                lines.push(RawLine::Elision { column: col });
            } else {
                lines.push(RawLine::Continuation {
                    line: line.to_string(),
                });
            }
        }
    }

    Ok(lines)
}

/// Parse the \x00-separated commit data after the \x01 marker.
fn parse_commit_fields(data: &str) -> jj::Result<JjCommit> {
    let parts: Vec<&str> = data.split('\x00').collect();
    if parts.len() < 9 {
        return Err(Error::Parse(format!(
            "Expected 9 fields, got {}",
            parts.len()
        )));
    }

    let change_id = ChangeId::try_from(parts[0]).map_err(|e| Error::Parse(e.to_string()))?;

    let parents: Vec<ChangeId> = parts[8]
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| ChangeId::try_from(s).map_err(|e| Error::Parse(e.to_string())))
        .collect::<jj::Result<Vec<ChangeId>>>()?;

    let full_description =
        serde_json::from_str::<String>(parts[2]).map_err(|e| Error::Parse(e.to_string()))?;

    let (summary, description) = match full_description.split_once('\n') {
        Some((first, rest)) => (first.to_string(), rest.trim_start().to_string()),
        None => (full_description, String::new()),
    };

    Ok(JjCommit {
        change_id,
        commit_id: parts[1].to_string(),
        summary,
        description,
        author: parts[3].to_string(),
        email: parts[4].to_string(),
        timestamp: parts[5].to_string(),
        is_immutable: parts[6] == "true",
        is_working_copy: parts[7] == "true",
        parents,
    })
}

// ── Gutter analysis helpers ─────────────────────────────────────────

/// Find the column index of a node character in a gutter string.
/// Columns are 2-char wide in jj output (symbol + space).
fn find_node_column(gutter: &str) -> usize {
    for (byte_pos, ch) in gutter.char_indices() {
        if NODE_CHARS.contains(&ch) {
            return char_position_to_column(gutter, byte_pos);
        }
    }
    0
}

/// Find the column of a specific character in a line.
fn find_char_column(line: &str, target: char) -> usize {
    for (byte_pos, ch) in line.char_indices() {
        if ch == target {
            return char_position_to_column(line, byte_pos);
        }
    }
    0
}

/// Convert a byte position in a string to a column index.
/// jj's gutter uses 2 display-width positions per column.
/// We count the number of display-width characters before this position.
fn char_position_to_column(s: &str, byte_pos: usize) -> usize {
    let prefix = &s[..byte_pos];
    let char_count = prefix.chars().count();
    char_count / 2
}

/// Find columns with vertical pass-through characters (│) in a gutter string,
/// excluding the node column itself.
fn find_passing_columns(gutter: &str, node_column: usize) -> Vec<usize> {
    let mut columns = Vec::new();
    for (byte_pos, ch) in gutter.char_indices() {
        if ch == '│' {
            let col = char_position_to_column(gutter, byte_pos);
            if col != node_column {
                columns.push(col);
            }
        }
    }
    columns.sort();
    columns.dedup();
    columns
}

/// Analyze a continuation line to find fork/merge patterns.
/// Returns a list of fork/merge events found on this line.
///
/// A single continuation line can contain multiple events when jj renders
/// multi-parent merges or multi-way forks:
///
/// Fork patterns (edge splits from left to right):
/// - `├─╮`     — single fork from col(├) to col(╮)
/// - `├─┬─╮`   — double fork from col(├) to col(┬) AND col(╮)
/// - `├─┬─┬─╮` — triple fork, etc.
///
/// Merge patterns (edge joins from right into left):
/// - `├─╯`     — single merge from col(╯) into col(├)
/// - `├─┴─╯`   — double merge from col(┴) AND col(╯) into col(├)
/// - `╰─┤`     — left-pointing single merge
/// - `╰─┴─┤`   — left-pointing double merge
fn analyze_continuation(line: &str) -> Vec<ContinuationInfo> {
    let mut results = Vec::new();
    let chars: Vec<(usize, char)> = line.char_indices().collect();

    // Collect positions of all interesting characters
    let mut branch_start = None; // ├ position (right-pointing branch)
    let mut fork_ends = Vec::new(); // ╮ positions
    let mut merge_ends = Vec::new(); // ╯ positions
    let mut t_junctions = Vec::new(); // ┬ positions (fork T-junctions)
    let mut inv_t_junctions = Vec::new(); // ┴ positions (merge T-junctions)
    let mut merge_left_start = None; // ╰ position (left-pointing branch)
    let mut merge_left_end = None; // ┤ position

    for &(byte_pos, ch) in &chars {
        match ch {
            '├' => branch_start = Some(byte_pos),
            '╮' => fork_ends.push(byte_pos),
            '╯' => merge_ends.push(byte_pos),
            '┬' => t_junctions.push(byte_pos),
            '┴' => inv_t_junctions.push(byte_pos),
            '╰' => merge_left_start = Some(byte_pos),
            '┤' => merge_left_end = Some(byte_pos),
            _ => {}
        }
    }

    if let Some(start_pos) = branch_start {
        let start_col = char_position_to_column(line, start_pos);

        // Fork events: ├ → ┬ and ├ → ╮
        // Each ┬ and ╮ is a separate fork target from the ├ column
        for &t_pos in &t_junctions {
            let t_col = char_position_to_column(line, t_pos);
            results.push(ContinuationInfo::Fork {
                from_column: start_col,
                to_column: t_col,
            });
        }
        for &end_pos in &fork_ends {
            let end_col = char_position_to_column(line, end_pos);
            results.push(ContinuationInfo::Fork {
                from_column: start_col,
                to_column: end_col,
            });
        }

        // Merge events: ╯ → ├ and ┴ → ├
        // Each ┴ and ╯ is a separate merge source into the ├ column
        for &t_pos in &inv_t_junctions {
            let t_col = char_position_to_column(line, t_pos);
            results.push(ContinuationInfo::MergeBack {
                from_column: t_col,
                into_column: start_col,
            });
        }
        for &end_pos in &merge_ends {
            let end_col = char_position_to_column(line, end_pos);
            results.push(ContinuationInfo::MergeBack {
                from_column: end_col,
                into_column: start_col,
            });
        }
    }

    // Handle left-pointing patterns: ╰─┤, ╰─┴─┤
    if let Some(end_pos) = merge_left_end {
        let end_col = char_position_to_column(line, end_pos);

        if let Some(start_pos) = merge_left_start {
            let start_col = char_position_to_column(line, start_pos);
            results.push(ContinuationInfo::MergeBack {
                from_column: start_col,
                into_column: end_col,
            });
        }

        // ┴ junctions between ╰ and ┤ also merge into ┤
        for &t_pos in &inv_t_junctions {
            let t_col = char_position_to_column(line, t_pos);
            results.push(ContinuationInfo::MergeBack {
                from_column: t_col,
                into_column: end_col,
            });
        }
    }

    results
}

enum ContinuationInfo {
    /// A fork: edge splits from `from_column` to a new `to_column`
    Fork {
        from_column: usize,
        to_column: usize,
    },
    /// An edge merges back: `from_column` merges into `into_column`
    MergeBack {
        from_column: usize,
        into_column: usize,
    },
}

/// Find columns with pass-through │ on a continuation line.
fn continuation_passing_columns(line: &str) -> Vec<usize> {
    let mut columns = Vec::new();
    for (byte_pos, ch) in line.char_indices() {
        if ch == '│' {
            let col = char_position_to_column(line, byte_pos);
            columns.push(col);
        }
    }
    columns.sort();
    columns.dedup();
    columns
}

// ── Graph building ──────────────────────────────────────────────────

/// Parse jj graph output into a structured CommitGraph.
fn parse_graph_output(output: &str) -> jj::Result<CommitGraph> {
    let raw_lines = parse_raw_lines(output)?;
    build_graph(raw_lines)
}

/// Build a CommitGraph from classified raw lines.
///
/// Strategy:
/// 1. Walk raw lines top to bottom, assigning row indices to commit and elision lines.
/// 2. Collect continuation lines between rows to determine edges.
/// 3. Use an "active columns" tracker: for each column, track which commit's
///    downward edge currently occupies it.
fn build_graph(raw_lines: Vec<RawLine>) -> jj::Result<CommitGraph> {
    // First pass: group raw lines into "row blocks".
    // Each block has one commit or elision as the head, followed by zero or more
    // continuation lines before the next commit/elision.
    let blocks = group_into_blocks(raw_lines);

    // Second pass: build rows with edges.
    let mut rows: Vec<GraphRow> = Vec::new();
    let mut max_columns: usize = 0;

    // active_columns[col] = Some(row_index) means column `col` has an active
    // downward edge coming from the commit at `row_index`.
    let mut active_columns: Vec<Option<usize>> = Vec::new();

    for block in &blocks {
        let row_index = rows.len();

        match &block.head {
            BlockHead::Commit { gutter, commit } => {
                let column = find_node_column(gutter);
                let passing = find_passing_columns(gutter, column);

                if column + 1 > max_columns {
                    max_columns = column + 1;
                }
                for &c in &passing {
                    if c + 1 > max_columns {
                        max_columns = c + 1;
                    }
                }

                // This commit now occupies its column. Mark it as active
                // (its downward edges will be resolved by continuations below it).
                while active_columns.len() <= column {
                    active_columns.push(None);
                }
                active_columns[column] = Some(row_index);

                // Also mark passing columns as active (they carry edges from
                // earlier commits through this row).
                for &c in &passing {
                    while active_columns.len() <= c {
                        active_columns.push(None);
                    }
                    // Don't overwrite — the active edge is from an earlier commit.
                }

                rows.push(GraphRow::Commit(Box::new(CommitRow {
                    commit: *commit.clone(),
                    column,
                    row: row_index,
                    edges: Vec::new(), // filled in below
                    passing_columns: passing,
                })));
            }
            BlockHead::Elision { column } => {
                let column = *column;
                if column + 1 > max_columns {
                    max_columns = column + 1;
                }

                // Find which columns have pass-through lines at this elision.
                // Look at continuation lines (if any) or the active columns.
                let passing = if block.continuations.is_empty() {
                    // Use active columns minus this elision's column
                    active_columns
                        .iter()
                        .enumerate()
                        .filter(|(c, val)| val.is_some() && *c != column)
                        .map(|(c, _)| c)
                        .collect()
                } else {
                    let mut p = Vec::new();
                    for cont in &block.continuations {
                        for c in continuation_passing_columns(cont) {
                            if c != column && !p.contains(&c) {
                                p.push(c);
                            }
                        }
                    }
                    p.sort();
                    p
                };

                // Clear this column's active edge since elision is terminal.
                if column < active_columns.len() {
                    active_columns[column] = None;
                }

                rows.push(GraphRow::Elision(ElisionRow {
                    row: row_index,
                    column,
                    passing_columns: passing,
                }));
            }
        }

        // Process continuation lines to detect forks and merges.
        for cont in &block.continuations {
            for info in analyze_continuation(cont) {
                match info {
                    ContinuationInfo::Fork {
                        from_column,
                        to_column,
                    } => {
                        // A fork means from_column's edge now also extends to to_column.
                        while active_columns.len() <= to_column {
                            active_columns.push(None);
                        }
                        // The new column carries an edge from the same source as from_column.
                        active_columns[to_column] = active_columns
                            .get(from_column)
                            .copied()
                            .flatten()
                            .or(Some(row_index));
                        if to_column + 1 > max_columns {
                            max_columns = to_column + 1;
                        }
                    }
                    ContinuationInfo::MergeBack {
                        from_column,
                        into_column: _,
                    } => {
                        // from_column merges into into_column. Clear from_column.
                        if from_column < active_columns.len() {
                            active_columns[from_column] = None;
                        }
                        // Trim trailing Nones
                        while active_columns.last() == Some(&None) {
                            active_columns.pop();
                        }
                    }
                }
            }
        }
    }

    // Third pass: compute edges for each commit row.
    // For each commit, look at the blocks below it to find where its edges land.
    resolve_edges(&mut rows, &blocks);

    Ok(CommitGraph { rows, max_columns })
}

/// A block is a commit/elision head followed by zero or more continuation lines.
struct Block {
    head: BlockHead,
    continuations: Vec<String>,
}

enum BlockHead {
    Commit {
        gutter: String,
        commit: Box<JjCommit>,
    },
    Elision {
        column: usize,
    },
}

/// Group raw lines into blocks (each headed by a commit or elision).
fn group_into_blocks(raw_lines: Vec<RawLine>) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();

    for raw in raw_lines {
        match raw {
            RawLine::Commit { gutter, commit } => {
                blocks.push(Block {
                    head: BlockHead::Commit { gutter, commit },
                    continuations: Vec::new(),
                });
            }
            RawLine::Elision { column } => {
                blocks.push(Block {
                    head: BlockHead::Elision { column },
                    continuations: Vec::new(),
                });
            }
            RawLine::Continuation { line } => {
                if let Some(last) = blocks.last_mut() {
                    last.continuations.push(line);
                }
            }
        }
    }

    blocks
}

/// Resolve edges for each commit row by looking at subsequent rows and
/// the continuation lines between them.
fn resolve_edges(rows: &mut [GraphRow], blocks: &[Block]) {
    // Build a map from change_id to row index for quick lookup.
    let mut change_id_to_row: std::collections::HashMap<ChangeId, usize> =
        std::collections::HashMap::new();
    for row in rows.iter() {
        if let GraphRow::Commit(cr) = row {
            change_id_to_row.insert(cr.commit.change_id, cr.row);
        }
    }

    // For each commit row, determine its edges.
    let row_count = rows.len();
    for i in 0..row_count {
        let (column, parents, _row_idx) = match &rows[i] {
            GraphRow::Commit(cr) => (cr.column, cr.commit.parents.clone(), cr.row),
            GraphRow::Elision(_) => continue,
        };

        let mut edges = Vec::new();

        // Examine the continuation lines in this block (block index == row index
        // since each block produces one row).
        let continuations = &blocks[i].continuations;

        // Collect fork/merge info from continuations.
        let mut fork_targets: Vec<usize> = Vec::new(); // columns forked to
        let mut merge_sources: Vec<(usize, usize)> = Vec::new(); // (from_col, into_col)

        for cont in continuations {
            for info in analyze_continuation(cont) {
                match info {
                    ContinuationInfo::Fork { to_column, .. } => {
                        fork_targets.push(to_column);
                    }
                    ContinuationInfo::MergeBack {
                        from_column,
                        into_column,
                    } => {
                        merge_sources.push((from_column, into_column));
                    }
                }
            }
        }

        if parents.is_empty() {
            // No parents — check if the next row in the same column is an elision.
            if let Some(elision_row) = find_next_row_in_column(rows, i + 1, column) {
                if matches!(&rows[elision_row], GraphRow::Elision(_)) {
                    edges.push(GraphEdge {
                        from_column: column,
                        to_row: elision_row,
                        to_column: column,
                        edge_type: EdgeType::Elided,
                    });
                }
            }
        } else {
            // Process each parent.
            for (parent_idx, parent_id) in parents.iter().enumerate() {
                if let Some(&target_row) = change_id_to_row.get(parent_id) {
                    // Parent is in the visible set.
                    let target_col = match &rows[target_row] {
                        GraphRow::Commit(cr) => cr.column,
                        GraphRow::Elision(er) => er.column,
                    };

                    let edge_type = if parent_idx > 0 {
                        EdgeType::Merge
                    } else if target_col == column {
                        EdgeType::Straight
                    } else {
                        EdgeType::CrossColumn
                    };

                    edges.push(GraphEdge {
                        from_column: column,
                        to_row: target_row,
                        to_column: target_col,
                        edge_type,
                    });
                } else {
                    // Parent is not in the visible set — look for an elision row
                    // below this commit in the same column (or a forked column).
                    let search_col = if parent_idx > 0 && !fork_targets.is_empty() {
                        fork_targets.get(parent_idx - 1).copied().unwrap_or(column)
                    } else {
                        column
                    };

                    if let Some(elision_row) = find_next_elision_in_column(rows, i + 1, search_col)
                    {
                        let elision_col = match &rows[elision_row] {
                            GraphRow::Elision(er) => er.column,
                            _ => search_col,
                        };
                        edges.push(GraphEdge {
                            from_column: column,
                            to_row: elision_row,
                            to_column: elision_col,
                            edge_type: EdgeType::Elided,
                        });
                    }
                    // If no elision found, the edge simply has no target in the visible graph.
                    // This can happen at the very bottom of the log.
                }
            }
        }

        // Deduplicate edges (e.g. if both parents resolve to the same elision).
        edges.dedup_by(|a, b| a.to_row == b.to_row && a.to_column == b.to_column);

        if let GraphRow::Commit(cr) = &mut rows[i] {
            cr.edges = edges;
        }
    }
}

/// Find the next row at or after `start` that occupies the given column.
fn find_next_row_in_column(rows: &[GraphRow], start: usize, column: usize) -> Option<usize> {
    for (i, row) in rows.iter().enumerate().skip(start) {
        let col = match row {
            GraphRow::Commit(cr) => cr.column,
            GraphRow::Elision(er) => er.column,
        };
        if col == column {
            return Some(i);
        }
        // Also check if this row has a passing line in the target column
        let passing = match row {
            GraphRow::Commit(cr) => &cr.passing_columns,
            GraphRow::Elision(er) => &er.passing_columns,
        };
        if passing.contains(&column) {
            continue; // The column passes through, keep looking
        }
    }
    None
}

/// Find the next elision row at or after `start` in the given column.
fn find_next_elision_in_column(rows: &[GraphRow], start: usize, column: usize) -> Option<usize> {
    for (i, row) in rows.iter().enumerate().skip(start) {
        match row {
            GraphRow::Elision(er) if er.column == column => return Some(i),
            GraphRow::Commit(cr) if cr.column == column => return None, // hit a commit first
            _ => continue,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_repo::TestRepo;

    /// Helper: get the commit graph for a test repo.
    fn graph_for(repo: &TestRepo) -> CommitGraph {
        get_log_graph(repo.path()).expect("get_log_graph should succeed")
    }

    /// Helper: collect all CommitRows from a graph.
    fn commit_rows(graph: &CommitGraph) -> Vec<&CommitRow> {
        graph
            .rows
            .iter()
            .filter_map(|r| match r {
                GraphRow::Commit(cr) => Some(cr.as_ref()),
                _ => None,
            })
            .collect()
    }

    /// Helper: collect all ElisionRows from a graph.
    fn elision_rows(graph: &CommitGraph) -> Vec<&ElisionRow> {
        graph
            .rows
            .iter()
            .filter_map(|r| match r {
                GraphRow::Elision(er) => Some(er),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn linear_history() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("a.txt", "a").unwrap();
        repo.commit("first").unwrap();
        repo.write_file("b.txt", "b").unwrap();
        repo.commit("second").unwrap();

        let graph = graph_for(&repo);
        let commits = commit_rows(&graph);

        // Should have at least: working copy, "second", "first", root
        assert!(commits.len() >= 4);

        // All commits should be in column 0 (linear history)
        for cr in &commits {
            assert_eq!(cr.column, 0, "linear history should all be in column 0");
        }

        // All edges should be Straight (same column, direct parent)
        for cr in &commits {
            for edge in &cr.edges {
                assert!(
                    matches!(edge.edge_type, EdgeType::Straight | EdgeType::Elided),
                    "linear history edges should be Straight or Elided, got {:?}",
                    edge.edge_type
                );
            }
        }

        // No passing columns in linear history
        for cr in &commits {
            assert!(
                cr.passing_columns.is_empty(),
                "linear history should have no passing columns"
            );
        }
    }

    #[test]
    fn single_branch() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("a.txt", "a").unwrap();
        let base = repo.commit("base").unwrap();

        // Create two branches from "base"
        repo.new_revision(base.created.change_id).unwrap();
        repo.write_file("b.txt", "b").unwrap();
        repo.commit("branch-a").unwrap();

        repo.new_revision(base.created.change_id).unwrap();
        repo.write_file("c.txt", "c").unwrap();
        repo.commit("branch-b").unwrap();

        let graph = graph_for(&repo);
        let commits = commit_rows(&graph);

        // At least one commit should be in column > 0
        let has_branched = commits.iter().any(|cr| cr.column > 0);
        assert!(
            has_branched,
            "branching should produce commits in column > 0"
        );

        // At least one commit should have non-empty passing_columns
        let has_passing = commits.iter().any(|cr| !cr.passing_columns.is_empty());
        assert!(has_passing, "branching should produce pass-through columns");

        // At least one edge should be CrossColumn
        let has_cross = commits
            .iter()
            .flat_map(|cr| &cr.edges)
            .any(|e| matches!(e.edge_type, EdgeType::CrossColumn));
        assert!(has_cross, "branching should produce CrossColumn edges");
    }

    #[test]
    fn merge_commit() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("a.txt", "a").unwrap();
        let base = repo.commit("base").unwrap();

        // Create two branches
        repo.new_revision(base.created.change_id).unwrap();
        repo.write_file("b.txt", "b").unwrap();
        let branch_a = repo.commit("branch-a").unwrap();

        repo.new_revision(base.created.change_id).unwrap();
        repo.write_file("c.txt", "c").unwrap();
        let branch_b = repo.commit("branch-b").unwrap();

        // Merge them
        repo.merge(
            &[branch_a.created.change_id, branch_b.created.change_id],
            "merge",
        )
        .unwrap();

        let graph = graph_for(&repo);
        let commits = commit_rows(&graph);

        // Find the merge commit
        let merge = commits
            .iter()
            .find(|cr| cr.commit.summary == "merge")
            .expect("merge commit should be present");

        // Merge commit should have at least 2 edges
        assert!(
            merge.edges.len() >= 2,
            "merge commit should have at least 2 edges, got {}",
            merge.edges.len()
        );

        // At least one edge should be Merge type
        let has_merge = merge
            .edges
            .iter()
            .any(|e| matches!(e.edge_type, EdgeType::Merge));
        assert!(has_merge, "merge commit should have a Merge edge");
    }

    #[test]
    fn elision_detected() {
        let repo = TestRepo::new().unwrap();

        // Create several commits.
        repo.write_file("a.txt", "a").unwrap();
        repo.commit("first").unwrap();
        repo.write_file("b.txt", "b").unwrap();
        let second = repo.commit("second").unwrap();
        repo.write_file("c.txt", "c").unwrap();
        repo.commit("third").unwrap();
        repo.write_file("d.txt", "d").unwrap();
        repo.commit("fourth").unwrap();

        // Make "second" (and its ancestors) immutable by setting it as an
        // immutable head. The revset `mutable() | ancestors(mutable(), 2)`
        // will then show mutable commits + up to 2 immutable ancestors,
        // eliding anything deeper.
        let change_id = second.created.change_id.to_string();
        repo.jj_config_set("revset-aliases.\"immutable_heads()\"", &change_id)
            .unwrap();

        let graph = graph_for(&repo);
        let elisions = elision_rows(&graph);

        assert!(
            !elisions.is_empty(),
            "should have at least one elision row when history is truncated"
        );

        // Find the commit just above the elision
        for er in &elisions {
            if er.row > 0 {
                // The previous row should be a commit with an Elided edge
                if let GraphRow::Commit(cr) = &graph.rows[er.row - 1] {
                    let has_elided = cr
                        .edges
                        .iter()
                        .any(|e| matches!(e.edge_type, EdgeType::Elided));
                    assert!(
                        has_elided,
                        "commit above elision at row {} should have an Elided edge",
                        er.row - 1
                    );
                }
            }
        }
    }

    #[test]
    fn working_copy_present() {
        let repo = TestRepo::new().unwrap();
        let graph = graph_for(&repo);
        let commits = commit_rows(&graph);

        let wc = commits.iter().find(|cr| cr.commit.is_working_copy);
        assert!(wc.is_some(), "working copy should be in the graph");
    }

    #[test]
    fn immutable_included() {
        let repo = TestRepo::new().unwrap();
        let graph = graph_for(&repo);
        let commits = commit_rows(&graph);

        let immutable = commits.iter().find(|cr| cr.commit.is_immutable);
        assert!(
            immutable.is_some(),
            "at least one immutable commit should be present"
        );
    }

    #[test]
    fn row_indices_sequential() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("a.txt", "a").unwrap();
        repo.commit("first").unwrap();
        repo.write_file("b.txt", "b").unwrap();
        repo.commit("second").unwrap();

        let graph = graph_for(&repo);

        // Row indices should be 0, 1, 2, ... matching position in the vec.
        for (i, row) in graph.rows.iter().enumerate() {
            let row_idx = match row {
                GraphRow::Commit(cr) => cr.row,
                GraphRow::Elision(er) => er.row,
            };
            assert_eq!(row_idx, i, "row index should match position in the vec");
        }
    }

    #[test]
    fn max_columns_correct() {
        let repo = TestRepo::new().unwrap();
        let graph = graph_for(&repo);

        // max_columns should be at least 1
        assert!(graph.max_columns >= 1);

        // max_columns should be >= the maximum column used by any row
        let actual_max = graph
            .rows
            .iter()
            .map(|r| match r {
                GraphRow::Commit(cr) => {
                    let passing_max = cr.passing_columns.iter().max().copied().unwrap_or(0);
                    std::cmp::max(cr.column + 1, passing_max + 1)
                }
                GraphRow::Elision(er) => {
                    let passing_max = er.passing_columns.iter().max().copied().unwrap_or(0);
                    std::cmp::max(er.column + 1, passing_max + 1)
                }
            })
            .max()
            .unwrap_or(1);

        assert!(
            graph.max_columns >= actual_max,
            "max_columns ({}) should be >= actual max ({})",
            graph.max_columns,
            actual_max
        );
    }

    #[test]
    fn multiple_siblings_from_same_parent() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("a.txt", "a").unwrap();
        let base = repo.commit("base").unwrap();

        // Create multiple branches from "base" — mimics the topology:
        //   @  working-copy
        //   │ ○  test3
        //   ├─╯
        //   │ ○  test2
        //   ├─╯
        //   │ ○  test
        //   ├─╯
        //   ◆  base (immutable)
        repo.new_revision(base.created.change_id).unwrap();
        repo.write_file("b.txt", "b").unwrap();
        repo.commit("test").unwrap();

        repo.new_revision(base.created.change_id).unwrap();
        repo.write_file("c.txt", "c").unwrap();
        repo.commit("test2").unwrap();

        repo.new_revision(base.created.change_id).unwrap();
        repo.write_file("d.txt", "d").unwrap();
        repo.commit("test3").unwrap();

        // Go back to base so working copy is in column 0
        repo.new_revision(base.created.change_id).unwrap();

        let graph = graph_for(&repo);
        let commits = commit_rows(&graph);

        // The branched commits (test, test2, test3) should be in column 1
        for name in &["test", "test2", "test3"] {
            let cr = commits
                .iter()
                .find(|cr| cr.commit.summary == *name)
                .unwrap_or_else(|| panic!("commit {:?} should be present", name));
            assert_eq!(
                cr.column, 1,
                "commit {:?} should be in column 1, got column {}",
                name, cr.column
            );
        }

        // Each branched commit should have a CrossColumn edge to the base
        for name in &["test", "test2", "test3"] {
            let cr = commits
                .iter()
                .find(|cr| cr.commit.summary == *name)
                .unwrap();
            let has_cross_column_edge = cr.edges.iter().any(|e| {
                matches!(e.edge_type, EdgeType::CrossColumn)
                    && e.from_column == 1
                    && e.to_column == 0
            });
            assert!(
                has_cross_column_edge,
                "commit {:?} should have CrossColumn edge from col 1 to col 0, edges: {:?}",
                name, cr.edges
            );
        }
    }

    #[test]
    fn three_parent_merge() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("a.txt", "a").unwrap();
        let base = repo.commit("base").unwrap();

        // Create three branches from "base"
        repo.new_revision(base.created.change_id).unwrap();
        repo.write_file("b.txt", "b").unwrap();
        let branch_a = repo.commit("branch-a").unwrap();

        repo.new_revision(base.created.change_id).unwrap();
        repo.write_file("c.txt", "c").unwrap();
        let branch_b = repo.commit("branch-b").unwrap();

        repo.new_revision(base.created.change_id).unwrap();
        repo.write_file("d.txt", "d").unwrap();
        let branch_c = repo.commit("branch-c").unwrap();

        // Create a 3-parent merge: jj renders this as ├─┬─╮
        repo.merge(
            &[
                branch_a.created.change_id,
                branch_b.created.change_id,
                branch_c.created.change_id,
            ],
            "three-way-merge",
        )
        .unwrap();

        let graph = graph_for(&repo);
        let commits = commit_rows(&graph);

        // Find the merge commit
        let merge = commits
            .iter()
            .find(|cr| cr.commit.summary == "three-way-merge")
            .expect("merge commit should be present");

        // The merge should have at least 3 edges (one per parent)
        assert!(
            merge.edges.len() >= 3,
            "3-parent merge should have >= 3 edges, got {}",
            merge.edges.len()
        );

        // At least 2 edges should be Merge type (parent index > 0)
        let merge_edges: Vec<_> = merge
            .edges
            .iter()
            .filter(|e| matches!(e.edge_type, EdgeType::Merge))
            .collect();
        assert!(
            merge_edges.len() >= 2,
            "3-parent merge should have >= 2 Merge-type edges, got {}",
            merge_edges.len()
        );

        // max_columns should be at least 3 (columns 0, 1, 2 for the fork)
        assert!(
            graph.max_columns >= 3,
            "3-parent merge should use at least 3 columns, got {}",
            graph.max_columns
        );
    }

    #[test]
    fn edges_point_to_valid_rows() {
        let repo = TestRepo::new().unwrap();
        repo.write_file("a.txt", "a").unwrap();
        repo.commit("first").unwrap();

        let graph = graph_for(&repo);
        let row_count = graph.rows.len();

        for row in &graph.rows {
            if let GraphRow::Commit(cr) = row {
                for edge in &cr.edges {
                    assert!(
                        edge.to_row < row_count,
                        "edge target row {} should be < row count {}",
                        edge.to_row,
                        row_count
                    );
                    assert!(
                        edge.to_row > cr.row,
                        "edge target row {} should be > source row {} (edges go downward)",
                        edge.to_row,
                        cr.row
                    );
                }
            }
        }
    }
}
