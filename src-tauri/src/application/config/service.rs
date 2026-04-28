use crate::bootstrap::AppState;
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::issue::ValidationIssue;
use crate::domain::config::model::GlobalConfig;
use crate::domain::config::rules::{default_global_config, validate_global_config};
use crate::infrastructure::json_store;

pub struct ConfigService<'a> {
    state: &'a AppState,
}

impl<'a> ConfigService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub fn load(&self) -> AppResult<GlobalConfig> {
        json_store::load_global_config(self.state.app_data_dir())
    }

    pub fn save(&self, config: &GlobalConfig) -> AppResult<Vec<ValidationIssue>> {
        let issues = validate_global_config(config);
        if issues
            .iter()
            .any(|issue| matches!(issue.level, crate::domain::common::issue::IssueLevel::Error))
        {
            return Err(AppError::new(
                "config.validation_failed",
                "global config contains validation errors",
            ));
        }

        json_store::save_global_config(self.state.app_data_dir(), config)?;
        Ok(issues)
    }

    pub fn ensure_initialized(&self) -> AppResult<GlobalConfig> {
        let path = json_store::global_config_path(self.state.app_data_dir());
        if path.exists() {
            return self.load();
        }

        let config = default_global_config();
        json_store::save_global_config(self.state.app_data_dir(), &config)?;
        Ok(config)
    }
}
