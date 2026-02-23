use kenjutu_types::InvalidChangeIdError;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use crate::models::JjStatus;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to run jj command: {0}")]
    Command(String),

    #[error("jj command failed: {0}")]
    JjFailed(String),

    #[error("Failed to parse output: {0}")]
    Parse(String),
}

impl From<InvalidChangeIdError> for Error {
    fn from(err: InvalidChangeIdError) -> Self {
        Error::Parse(err.to_string())
    }
}

static JJ_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

fn find_jj_executable() -> Option<PathBuf> {
    JJ_PATH
        .get_or_init(|| {
            let mut candidates: Vec<PathBuf> = vec![
                PathBuf::from("/opt/homebrew/bin/jj"),
                PathBuf::from("/usr/local/bin/jj"),
                PathBuf::from("/run/current-system/sw/bin/jj"),
            ];

            if let Some(home) = dirs::home_dir() {
                candidates.push(home.join(".cargo/bin/jj"));
                candidates.push(home.join(".nix-profile/bin/jj"));
            }

            for path in &candidates {
                if path.exists() {
                    log::info!("Found jj executable at: {}", path.display());
                    return Some(path.clone());
                }
            }

            if Command::new("jj")
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                log::info!("Found jj executable in PATH");
                return Some(PathBuf::from("jj"));
            }

            log::warn!("jj executable not found in any known location");
            None
        })
        .clone()
}

pub(crate) fn jj_command() -> Option<Command> {
    find_jj_executable().map(Command::new)
}

/// Check if jj CLI is installed
pub fn is_installed() -> bool {
    find_jj_executable().is_some()
}

/// Check if directory is a jj repository
pub fn is_jj_repo(local_dir: &str) -> bool {
    jj_command()
        .map(|mut cmd| {
            cmd.args(["root"])
                .current_dir(local_dir)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

/// Get jj status for a directory
pub fn get_status(local_dir: &str) -> JjStatus {
    JjStatus {
        is_installed: is_installed(),
        is_jj_repo: is_jj_repo(local_dir),
    }
}
