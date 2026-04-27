use std::path::PathBuf;
use std::sync::Arc;

use crate::bootstrap::app_state::AppState;
use crate::domain::common::error::AppResult;
use crate::runtime::events::SharedEventBus;

pub fn build_app_state(app_data_dir: PathBuf) -> AppResult<AppState> {
    AppState::new(app_data_dir)
}

pub fn build_app_state_with_event_bus(
    app_data_dir: PathBuf,
    event_bus: SharedEventBus,
) -> AppResult<AppState> {
    AppState::with_event_bus(app_data_dir, Arc::clone(&event_bus))
}
