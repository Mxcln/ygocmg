use tauri::{AppHandle, Emitter};

use crate::domain::common::error::{AppError, AppResult};
use crate::runtime::events::{AppEvent, AppEventBus, JOB_FINISHED_EVENT, JOB_PROGRESS_EVENT};

pub struct TauriEventBus {
    app_handle: AppHandle,
}

impl TauriEventBus {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl AppEventBus for TauriEventBus {
    fn publish(&self, event: AppEvent) -> AppResult<()> {
        match event {
            AppEvent::JobProgress(payload) => self
                .app_handle
                .emit(JOB_PROGRESS_EVENT, payload)
                .map_err(|source| AppError::new("event.emit_failed", source.to_string()))?,
            AppEvent::JobFinished(payload) => self
                .app_handle
                .emit(JOB_FINISHED_EVENT, payload)
                .map_err(|source| AppError::new("event.emit_failed", source.to_string()))?,
        }
        Ok(())
    }
}
