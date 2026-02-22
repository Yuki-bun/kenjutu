pub mod commit_log;
pub mod diff_panel;
pub mod review;

use kenjutu_core::models::JjCommit;

pub enum ScreenOutcome {
    Continue,
    Quit,
    EnterReview(JjCommit),
    ExitReview,
    Error(String),
}
