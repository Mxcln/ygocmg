use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::common::ids::LanguageCode;
use crate::domain::language::model::TextLanguageProfile;
use crate::domain::language::rules::default_text_language_catalog;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlobalConfig {
    pub app_language: LanguageCode,
    pub ygopro_path: Option<PathBuf>,
    pub external_text_editor_path: Option<PathBuf>,
    pub custom_code_recommended_min: u32,
    pub custom_code_recommended_max: u32,
    pub custom_code_min_gap: u32,
    #[serde(default = "default_shell_sidebar_width")]
    pub shell_sidebar_width: u32,
    #[serde(default = "default_shell_window_width")]
    pub shell_window_width: u32,
    #[serde(default = "default_shell_window_height")]
    pub shell_window_height: u32,
    #[serde(default)]
    pub shell_window_is_maximized: bool,
    #[serde(default = "default_text_language_catalog")]
    pub text_language_catalog: Vec<TextLanguageProfile>,
    #[serde(default)]
    pub standard_pack_source_language: Option<LanguageCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlobalConfigFile {
    pub schema_version: u32,
    pub data: GlobalConfig,
}

fn default_shell_sidebar_width() -> u32 {
    150
}

fn default_shell_window_width() -> u32 {
    960
}

fn default_shell_window_height() -> u32 {
    640
}
