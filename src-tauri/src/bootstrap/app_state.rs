use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::domain::common::error::{AppError, AppResult};
use crate::runtime::sessions::SessionManager;

#[derive(Debug)]
pub struct AppState {
    app_data_dir: PathBuf,
    pub sessions: RwLock<SessionManager>,
}

impl AppState {
    pub fn new(app_data_dir: PathBuf) -> AppResult<Self> {
        std::fs::create_dir_all(&app_data_dir).map_err(|source| {
            AppError::from_io("bootstrap.app_data_dir_create_failed", source)
                .with_detail("path", app_data_dir.display().to_string())
        })?;

        Ok(Self {
            app_data_dir,
            sessions: RwLock::new(SessionManager::default()),
        })
    }

    pub fn app_data_dir(&self) -> &Path {
        &self.app_data_dir
    }
}
