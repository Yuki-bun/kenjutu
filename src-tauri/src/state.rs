use std::sync::Arc;
use tokio::sync::{OnceCell, SetError};

use crate::errors::{CommandError, Result};
use crate::App;

#[derive(Clone)]
pub struct AppState {
    inner: Arc<OnceCell<App>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(OnceCell::new()),
        }
    }

    #[allow(clippy::result_large_err)]
    pub fn set(&self, app: App) -> std::result::Result<(), SetError<App>> {
        self.inner.set(app)
    }

    pub async fn get(&self) -> Result<&App> {
        self.inner.get().ok_or(CommandError::NotInitialized)
    }
}
