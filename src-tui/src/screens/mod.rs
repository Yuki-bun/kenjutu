pub mod commit_log;
pub mod diff_panel;
pub mod review;

use crate::jj_graph::GraphCommit;

pub enum ScreenOutcome {
    Continue,
    Quit,
    EnterReview(GraphCommit),
    ExitReview,
    Error(String),
}
