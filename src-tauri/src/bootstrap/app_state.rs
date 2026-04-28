use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::domain::common::error::{AppError, AppResult};
use crate::domain::namespace::model::StandardNamespaceBaseline;
use crate::runtime::confirmation_cache::ConfirmationCache;
use crate::runtime::events::{NoopEventBus, SharedEventBus};
use crate::runtime::jobs::JobRuntime;
use crate::runtime::sessions::SessionManager;
use crate::runtime::standard_pack_cache::StandardPackIndexCache;

#[derive(Clone)]
pub struct AppState {
    app_data_dir: PathBuf,
    pub standard_baseline: StandardNamespaceBaseline,
    pub sessions: Arc<RwLock<SessionManager>>,
    pub confirmation_cache: Arc<RwLock<ConfirmationCache>>,
    pub standard_pack_index_cache: StandardPackIndexCache,
    pub jobs: JobRuntime,
    pub event_bus: SharedEventBus,
}

impl AppState {
    pub fn new(app_data_dir: PathBuf) -> AppResult<Self> {
        Self::with_event_bus(app_data_dir, Arc::new(NoopEventBus))
    }

    pub fn with_event_bus(app_data_dir: PathBuf, event_bus: SharedEventBus) -> AppResult<Self> {
        std::fs::create_dir_all(&app_data_dir).map_err(|source| {
            AppError::from_io("bootstrap.app_data_dir_create_failed", source)
                .with_detail("path", app_data_dir.display().to_string())
        })?;
        let jobs = JobRuntime::new(Arc::clone(&event_bus));

        Ok(Self {
            app_data_dir,
            standard_baseline:
                crate::infrastructure::standard_baseline::load_standard_namespace_baseline(),
            sessions: Arc::new(RwLock::new(SessionManager::default())),
            confirmation_cache: Arc::new(RwLock::new(ConfirmationCache::default())),
            standard_pack_index_cache: StandardPackIndexCache::default(),
            jobs,
            event_bus,
        })
    }

    pub fn app_data_dir(&self) -> &Path {
        &self.app_data_dir
    }
}

impl fmt::Debug for AppState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AppState")
            .field("app_data_dir", &self.app_data_dir)
            .field("standard_baseline", &self.standard_baseline)
            .field("sessions", &self.sessions)
            .field("confirmation_cache", &self.confirmation_cache)
            .field("standard_pack_index_cache", &self.standard_pack_index_cache)
            .field("jobs", &self.jobs)
            .finish_non_exhaustive()
    }
}
