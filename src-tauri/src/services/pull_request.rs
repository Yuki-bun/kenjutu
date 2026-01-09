use git2::{Delta, DiffLineType as Git2DiffLineType, Oid};

use crate::db::DB;
use crate::errors::{CommandError, Result};
use crate::models::{
    CommitDiff, DiffHunk, DiffLine, DiffLineType, FileChangeStatus, FileDiff, GetPullResponse,
    PRCommit, PullRequest,
};
use crate::services::GitHubService;

pub struct PullRequestService;

impl PullRequestService {
    pub async fn list_pull_requests(
        github: &GitHubService,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<PullRequest>> {
        let prs = github.list_pull_requests(owner, repo).await?;
        Ok(prs.into_iter().map(PullRequest::from).collect())
    }

    pub async fn get_pull_request_details(
        github: &GitHubService,
        db: &mut DB,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<GetPullResponse> {
        let pr = github.get_pull_request(owner, repo, pr_number).await?;
        let gh_repo = github.get_repository(owner, repo).await?;

        let repo_node_id = gh_repo.node_id.ok_or_else(|| {
            log::error!("Got null node id");
            CommandError::Internal
        })?;

        let repo_dir = db
            .find_local_repo(&repo_node_id)
            .await
            .map_err(|err| {
                log::error!("DB error: {err}");
                CommandError::Internal
            })?
            .ok_or_else(|| CommandError::bad_input("Please set local repository to review PR"))?;

        let repository = git2::Repository::open(&repo_dir.local_dir).map_err(|err| {
            log::error!("Could not find local repository: {err}");
            CommandError::bad_input(
                "Could not connect to repository set by user. Please reset local repository for this repository",
            )
        })?;

        let head_sha = Oid::from_str(&pr.head.sha).map_err(|err| {
            log::error!("GitHub gave me a wrong hash: {err}");
            CommandError::Internal
        })?;
        let base_sha = Oid::from_str(&pr.base.sha).map_err(|err| {
            log::error!("GitHub gave me a wrong hash: {err}");
            CommandError::Internal
        })?;

        let mut walker = repository.revwalk().map_err(|err| {
            log::error!("Failed to initiate rev walker: {err}");
            CommandError::Internal
        })?;
        let range = format!("{}..{}", base_sha, head_sha);
        walker.push_range(&range).map_err(|err| {
            log::error!("Failed to push range to walker: {err}");
            CommandError::Internal
        })?;

        let mut commits: Vec<PRCommit> = Vec::new();
        for oid in walker {
            let oid = oid.map_err(|err| {
                log::error!("Walker error: {err}");
                CommandError::Internal
            })?;
            let commit = repository.find_commit(oid).map_err(|err| {
                log::error!("Could not find commit: {err}");
                CommandError::Internal
            })?;

            let change_id = commit
                .header_field_bytes("change-id")
                .ok()
                .and_then(|buf| buf.as_str().map(String::from));

            let commit = PRCommit {
                change_id,
                sha: oid.to_string(),
                summary: commit.summary().unwrap_or("").to_string(),
                description: commit.body().unwrap_or("").to_string(),
            };
            commits.push(commit);
        }

        Ok(GetPullResponse {
            title: pr.title.unwrap_or_default(),
            body: pr.body.unwrap_or_default(),
            base_branch: pr.base.ref_field,
            head_branch: pr.head.ref_field,
            commits,
        })
    }

    fn process_line(line: git2::DiffLine) -> (DiffLine, u32, u32) {
        let line_type = Self::map_line_type(line.origin_value());
        let content = String::from_utf8_lossy(line.content()).to_string();

        // Count additions and deletions
        let (additions, deletions) = match line.origin_value() {
            Git2DiffLineType::Addition => (1, 0),
            Git2DiffLineType::Deletion => (0, 1),
            _ => (0, 0),
        };

        let diff_line = DiffLine {
            line_type,
            old_lineno: line.old_lineno(),
            new_lineno: line.new_lineno(),
            content,
        };

        (diff_line, additions, deletions)
    }

    fn process_hunk(patch: &git2::Patch, hunk_idx: usize) -> Result<(DiffHunk, u32, u32)> {
        let (hunk, hunk_lines_count) = patch.hunk(hunk_idx).map_err(|err| {
            log::error!("Failed to get hunk: {err}");
            CommandError::Internal
        })?;

        let mut lines = Vec::new();
        let mut hunk_additions = 0u32;
        let mut hunk_deletions = 0u32;

        // Process lines in this hunk
        for line_idx in 0..hunk_lines_count {
            let line = patch.line_in_hunk(hunk_idx, line_idx).map_err(|err| {
                log::error!("Failed to get line: {err}");
                CommandError::Internal
            })?;

            let (diff_line, add, del) = Self::process_line(line);
            hunk_additions += add;
            hunk_deletions += del;
            lines.push(diff_line);
        }

        let header = String::from_utf8_lossy(hunk.header()).to_string();

        let diff_hunk = DiffHunk {
            old_start: hunk.old_start(),
            old_lines: hunk.old_lines(),
            new_start: hunk.new_start(),
            new_lines: hunk.new_lines(),
            header,
            lines,
        };

        Ok((diff_hunk, hunk_additions, hunk_deletions))
    }

    fn process_patch(patch: git2::Patch) -> Result<FileDiff> {
        let delta = patch.delta();
        let old_file = delta.old_file();
        let new_file = delta.new_file();

        let old_path = old_file.path().map(|p| p.to_string_lossy().to_string());
        let new_path = new_file.path().map(|p| p.to_string_lossy().to_string());

        let status = Self::map_delta_status(delta.status());
        let is_binary = old_file.is_binary() || new_file.is_binary();

        let mut additions = 0u32;
        let mut deletions = 0u32;
        let mut hunks = Vec::new();

        // Process all hunks
        for hunk_idx in 0..patch.num_hunks() {
            let (hunk, add, del) = Self::process_hunk(&patch, hunk_idx)?;
            additions += add;
            deletions += del;
            hunks.push(hunk);
        }

        Ok(FileDiff {
            old_path,
            new_path,
            status,
            additions,
            deletions,
            is_binary,
            hunks,
        })
    }

    pub fn get_commit_diff(repository: &git2::Repository, commit_sha: &str) -> Result<CommitDiff> {
        // Find commit
        let oid = Oid::from_str(commit_sha).map_err(|err| {
            log::error!("Invalid commit SHA: {err}");
            CommandError::bad_input("Invalid commit SHA")
        })?;

        let commit = repository.find_commit(oid).map_err(|err| {
            log::error!("Could not find commit: {err}");
            CommandError::Internal
        })?;

        // Get commit tree and parent tree
        let commit_tree = commit.tree().map_err(|err| {
            log::error!("Could not get commit tree: {err}");
            CommandError::Internal
        })?;

        let parent_tree = if commit.parent_count() > 0 {
            let parent = commit.parent(0).map_err(|err| {
                log::error!("Could not get parent commit: {err}");
                CommandError::Internal
            })?;
            Some(parent.tree().map_err(|err| {
                log::error!("Could not get parent tree: {err}");
                CommandError::Internal
            })?)
        } else {
            None
        };

        let mut diff_opts = git2::DiffOptions::new();
        diff_opts
            .context_lines(3)
            .interhunk_lines(0)
            .ignore_whitespace(false);

        let diff = repository
            .diff_tree_to_tree(
                parent_tree.as_ref(),
                Some(&commit_tree),
                Some(&mut diff_opts),
            )
            .map_err(|err| {
                log::error!("Failed to generate diff: {err}");
                CommandError::Internal
            })?;

        // Process all file patches
        let mut files: Vec<FileDiff> = Vec::new();
        for (delta_idx, _) in diff.deltas().enumerate() {
            let patch = git2::Patch::from_diff(&diff, delta_idx).map_err(|err| {
                log::error!("Failed to get patch: {err}");
                CommandError::Internal
            })?;
            if let Some(patch) = patch {
                files.push(Self::process_patch(patch)?);
            }
        }

        Ok(CommitDiff {
            commit_sha: commit_sha.to_string(),
            files,
        })
    }

    fn map_delta_status(status: Delta) -> FileChangeStatus {
        match status {
            Delta::Added => FileChangeStatus::Added,
            Delta::Deleted => FileChangeStatus::Deleted,
            Delta::Modified => FileChangeStatus::Modified,
            Delta::Renamed => FileChangeStatus::Renamed,
            Delta::Copied => FileChangeStatus::Copied,
            Delta::Typechange => FileChangeStatus::Typechange,
            _ => FileChangeStatus::Modified, // Default for untracked, ignored, etc.
        }
    }

    fn map_line_type(line_type: Git2DiffLineType) -> DiffLineType {
        match line_type {
            Git2DiffLineType::Context | Git2DiffLineType::ContextEOFNL => DiffLineType::Context,
            Git2DiffLineType::Addition => DiffLineType::Addition,
            Git2DiffLineType::Deletion => DiffLineType::Deletion,
            Git2DiffLineType::AddEOFNL => DiffLineType::AddEofnl,
            Git2DiffLineType::DeleteEOFNL => DiffLineType::DelEofnl,
            _ => DiffLineType::Context, // Default for file headers, hunk headers, etc.
        }
    }
}
