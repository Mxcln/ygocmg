use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::domain::common::error::{AppError, AppResult};
use crate::domain::namespace::model::StandardNamespaceBaseline;
use crate::runtime::confirmation_cache::ConfirmationCache;
use crate::runtime::sessions::SessionManager;

#[derive(Debug)]
pub struct AppState {
    app_data_dir: PathBuf,
    pub standard_baseline: StandardNamespaceBaseline,
    pub sessions: RwLock<SessionManager>,
    pub confirmation_cache: RwLock<ConfirmationCache>,
}

impl AppState {
    pub fn new(app_data_dir: PathBuf) -> AppResult<Self> {
        std::fs::create_dir_all(&app_data_dir).map_err(|source| {
            AppError::from_io("bootstrap.app_data_dir_create_failed", source)
                .with_detail("path", app_data_dir.display().to_string())
        })?;

        Ok(Self {
            app_data_dir,
            standard_baseline: crate::infrastructure::standard_baseline::load_standard_namespace_baseline(),
            sessions: RwLock::new(SessionManager::default()),
            confirmation_cache: RwLock::new(ConfirmationCache::default()),
        })
    }

    pub fn app_data_dir(&self) -> &Path {
        &self.app_data_dir
    }
}
