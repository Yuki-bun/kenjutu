use serde::Serialize;
use specta::Type;

#[derive(Clone, Debug, Serialize, Type)]
#[serde(tag = "type")]
pub enum CommandError {
    BadInput { description: String },
    Internal,
}

impl CommandError {
    pub fn bad_input(description: impl Into<String>) -> Self {
        Self::BadInput {
            description: description.into(),
        }
    }
}

pub type Result<T, E = CommandError> = std::result::Result<T, E>;

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::BadInput { description } => write!(f, "Got bad input: {description}"),
            CommandError::Internal => write!(f, "Internal Error"),
        }
    }
}

impl std::error::Error for CommandError {}

impl From<git2::Error> for CommandError {
    fn from(err: git2::Error) -> Self {
        log::error!("Git error: {}", err);
        Self::Internal
    }
}
