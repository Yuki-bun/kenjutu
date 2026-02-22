use anyhow::Result;
use git2::Repository;
use kenjutu_core::models::{FileEntry, FileDiff};
use kenjutu_types::CommitId;

use crate::data::{self, GraphCommit, GraphRow};

#[derive(Clone, Copy, PartialEq)]
pub enum Focus {
    Side,
    Diff,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SidePanel {
    Commits,
    Files,
}

pub struct App {
    pub repo: Repository,
    pub focus: Focus,
    pub side_panel: SidePanel,

    // Commits
    pub commits: Vec<GraphCommit>,
    pub graph_rows: Vec<GraphRow>,
    pub selected_commit: usize,

    // Files for selected commit
    pub files: Vec<FileEntry>,
    pub selected_file: usize,

    // Diff for selected file
    pub diff: Option<FileDiff>,
    pub diff_scroll: usize,
    pub diff_total_lines: usize,

    pub error_msg: Option<String>,
}

impl App {
    pub fn new(repo_path: &str) -> Result<Self> {
        let repo = Repository::open(repo_path)?;
        let commits = data::load_commits(&repo, 200)?;
        let graph_rows = data::compute_graph_layout(&commits);

        let mut app = App {
            repo,
            focus: Focus::Side,
            side_panel: SidePanel::Commits,
            commits,
            graph_rows,
            selected_commit: 0,
            files: Vec::new(),
            selected_file: 0,
            diff: None,
            diff_scroll: 0,
            diff_total_lines: 0,
            error_msg: None,
        };

        if !app.commits.is_empty() {
            app.load_files_for_selected_commit();
        }

        Ok(app)
    }

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Side => Focus::Diff,
            Focus::Diff => Focus::Side,
        };
    }

    pub fn cycle_focus_back(&mut self) {
        self.cycle_focus();
    }

    pub fn side_up(&mut self) {
        match self.side_panel {
            SidePanel::Commits => {
                if self.selected_commit > 0 {
                    self.selected_commit -= 1;
                    self.load_files_for_selected_commit();
                }
            }
            SidePanel::Files => {
                if self.selected_file > 0 {
                    self.selected_file -= 1;
                    self.load_diff_for_selected_file();
                }
            }
        }
    }

    pub fn side_down(&mut self) {
        match self.side_panel {
            SidePanel::Commits => {
                if self.selected_commit + 1 < self.commits.len() {
                    self.selected_commit += 1;
                    self.load_files_for_selected_commit();
                }
            }
            SidePanel::Files => {
                if self.selected_file + 1 < self.files.len() {
                    self.selected_file += 1;
                    self.load_diff_for_selected_file();
                }
            }
        }
    }

    pub fn side_select(&mut self) {
        match self.side_panel {
            SidePanel::Commits => {
                self.load_files_for_selected_commit();
                self.side_panel = SidePanel::Files;
            }
            SidePanel::Files => {
                self.load_diff_for_selected_file();
                self.focus = Focus::Diff;
            }
        }
    }

    pub fn diff_scroll_up(&mut self, amount: usize) {
        self.diff_scroll = self.diff_scroll.saturating_sub(amount);
    }

    pub fn diff_scroll_down(&mut self, amount: usize) {
        if self.diff_total_lines > 0 {
            self.diff_scroll = (self.diff_scroll + amount).min(self.diff_total_lines.saturating_sub(1));
        }
    }

    pub fn diff_scroll_to_end(&mut self) {
        if self.diff_total_lines > 0 {
            self.diff_scroll = self.diff_total_lines.saturating_sub(1);
        }
    }

    fn load_files_for_selected_commit(&mut self) {
        self.error_msg = None;
        if self.selected_commit >= self.commits.len() {
            return;
        }
        let commit_id = self.commits[self.selected_commit].commit_id;
        match data::load_file_list(&self.repo, commit_id) {
            Ok(files) => {
                self.files = files;
                self.selected_file = 0;
                self.diff = None;
                self.diff_scroll = 0;
                self.diff_total_lines = 0;
                if !self.files.is_empty() {
                    self.load_diff_for_selected_file();
                }
            }
            Err(e) => {
                self.error_msg = Some(format!("Failed to load files: {e}"));
                self.files.clear();
                self.diff = None;
            }
        }
    }

    fn load_diff_for_selected_file(&mut self) {
        self.error_msg = None;
        if self.selected_file >= self.files.len() || self.selected_commit >= self.commits.len() {
            return;
        }

        let commit_id = self.commits[self.selected_commit].commit_id;
        let file = &self.files[self.selected_file];

        let file_path = file
            .new_path
            .as_deref()
            .or(file.old_path.as_deref())
            .unwrap_or("");

        let old_path = if file.new_path.is_some() && file.old_path.is_some() && file.new_path != file.old_path {
            file.old_path.as_deref()
        } else {
            None
        };

        match data::load_file_diff(&self.repo, commit_id, file_path, old_path) {
            Ok(file_diff) => {
                self.diff_total_lines = count_diff_lines(&file_diff);
                self.diff = Some(file_diff);
                self.diff_scroll = 0;
            }
            Err(e) => {
                self.error_msg = Some(format!("Failed to load diff: {e}"));
                self.diff = None;
                self.diff_total_lines = 0;
            }
        }
    }

    #[allow(dead_code)]
    pub fn selected_commit_id(&self) -> Option<CommitId> {
        self.commits.get(self.selected_commit).map(|c| c.commit_id)
    }
}

fn count_diff_lines(diff: &FileDiff) -> usize {
    let mut count = 0;
    for hunk in &diff.hunks {
        count += 1; // hunk header
        count += hunk.lines.len();
    }
    count
}
