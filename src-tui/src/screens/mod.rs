pub mod commit_log;
pub mod review;

use crate::jj_graph::GraphCommit;

pub enum ScreenOutcome {
    Continue,
    Quit,
    EnterReview(GraphCommit),
    ExitReview,
    Error(String),
}
