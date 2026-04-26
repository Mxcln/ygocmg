use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::common::ids::LanguageCode;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlobalConfig {
    pub app_language: LanguageCode,
    pub ygopro_path: Option<PathBuf>,
    pub external_text_editor_path: Option<PathBuf>,
    pub custom_code_recommended_min: u32,
    pub custom_code_recommended_max: u32,
    pub custom_code_min_gap: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlobalConfigFile {
    pub schema_version: u32,
    pub data: GlobalConfig,
}
