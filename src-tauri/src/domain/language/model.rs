use serde::{Deserialize, Serialize};

use crate::domain::common::ids::LanguageCode;
use crate::domain::common::time::AppTimestamp;

pub const LEGACY_DEFAULT_LANGUAGE: &str = "default";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TextLanguageKind {
    Builtin,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextLanguageProfile {
    pub id: LanguageCode,
    pub label: String,
    pub kind: TextLanguageKind,
    #[serde(default)]
    pub hidden: bool,
    pub last_used_at: Option<AppTimestamp>,
}
