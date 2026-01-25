mod auth;
mod jj;
mod pr;
mod repo;

pub use auth::*;
pub use jj::*;
pub use pr::*;
pub use repo::*;

use serde::Serialize;
use specta::Type;

use crate::db;
use crate::services::{auth as auth_svc, diff, git, jj as jj_svc};

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

    #[error("Database error: {message}")]
    Database { message: String },

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Internal error")]
    Internal,
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
            git::Error::InvalidSha(sha) => Error::BadInput {
                message: format!("Invalid SHA: {sha}"),
            },
            git::Error::CommitNotFound(sha) => Error::Git {
                message: format!("Commit not found: {sha}"),
            },
            git::Error::Git2(e) => Error::Git {
                message: e.message().to_string(),
            },
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
            diff::Error::Db(e) => e.into(),
        }
    }
}

impl From<db::Error> for Error {
    fn from(err: db::Error) -> Self {
        log::error!("Database error: {err}");
        Error::Database {
            message: err.to_string(),
        }
    }
}

impl From<auth_svc::Error> for Error {
    fn from(err: auth_svc::Error) -> Self {
        log::error!("Auth error: {err}");
        match err {
            auth_svc::Error::OpenUrl(_) => Error::Internal,
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
