use std::path::PathBuf;

use crate::bootstrap::app_state::AppState;
use crate::domain::common::error::AppResult;

pub fn build_app_state(app_data_dir: PathBuf) -> AppResult<AppState> {
    AppState::new(app_data_dir)
}
