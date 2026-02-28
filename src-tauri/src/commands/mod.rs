mod auth;
mod comments;
mod jj;
mod pr;
mod repo;
pub mod settings;

pub use auth::*;
pub use comments::*;
pub use jj::*;
pub use pr::*;
pub use repo::*;
pub use settings::{get_ssh_settings, set_ssh_settings};

use serde::Serialize;
use specta::Type;

use crate::services::auth as auth_svc;
use kenjutu_core::services::{diff, git, jj as jj_svc};
use kenjutu_types::InvalidChangeIdError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Serialize, Type, thiserror::Error)]
#[serde(tag = "type")]
pub enum Error {
    #[error("{message}")]
    BadInput { message: String },

    #[error("Repository error: {message}")]
    Repository { message: String },

    #[error("Git operation failed: {message}")]
    Git { message: String },

    #[error("Jujutu operation failed: {message}")]
    Jj { message: String },

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Internal error")]
    Internal,

    #[error("Marker commit error: {message}")]
    MarkerCommit { message: String },

    #[error("Comment commit error: {message}")]
    CommentCommit { message: String },

    #[error("SSH authentication failed: {message}")]
    SshAuth { message: String },
}

impl Error {
    pub fn bad_input(message: impl Into<String>) -> Self {
        Self::BadInput {
            message: message.into(),
        }
    }
}

impl From<git::Error> for Error {
    fn from(err: git::Error) -> Self {
        log::error!("Git error: {err}");
        match err {
            git::Error::RepoNotFound(path) => Error::Repository {
                message: format!("Repository not found: {path}"),
            },
            git::Error::CommitNotFound(sha) => Error::Git {
                message: format!("Commit not found: {sha}"),
            },
            git::Error::Git2(e) => Error::Git {
                message: e.message().to_string(),
            },
            git::Error::SshAuth(msg) => Error::SshAuth { message: msg },
        }
    }
}

impl From<diff::Error> for Error {
    fn from(err: diff::Error) -> Self {
        log::error!("Diff error: {err}");
        match err {
            diff::Error::FileNotFound(path) => Error::FileNotFound { path },
            diff::Error::Git(e) => e.into(),
            diff::Error::Git2(e) => Error::Git {
                message: e.message().to_string(),
            },
            diff::Error::MarkerCommit(e) => Error::MarkerCommit {
                message: e.to_string(),
            },
            diff::Error::Internal(msg) => {
                log::error!("Internal diff error: {msg}");
                Error::Internal
            }
        }
    }
}

impl From<auth_svc::Error> for Error {
    fn from(err: auth_svc::Error) -> Self {
        log::error!("Auth error: {err}");
        match err {
            auth_svc::Error::Http(_) | auth_svc::Error::DeviceCodeRequest(_) => Error::Internal,
        }
    }
}

impl From<jj_svc::Error> for Error {
    fn from(err: jj_svc::Error) -> Self {
        log::error!("Jj error: {err}");
        match err {
            jj_svc::Error::Command(msg) => Error::Jj {
                message: format!("Failed to run jj: {msg}"),
            },
            jj_svc::Error::JjFailed(msg) => Error::Jj {
                message: format!("Failed to run jj: {msg}"),
            },
            jj_svc::Error::Parse(_) => Error::Internal,
        }
    }
}

impl From<InvalidChangeIdError> for Error {
    fn from(err: InvalidChangeIdError) -> Self {
        log::error!("Invalid change ID error: {err}");
        Error::BadInput {
            message: format!("Invalid change ID: {err}"),
        }
    }
}
