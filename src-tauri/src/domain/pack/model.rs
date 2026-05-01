use serde::{Deserialize, Serialize};

use crate::domain::common::ids::{LanguageCode, PackId};
use crate::domain::common::time::AppTimestamp;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackKind {
    Standard,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackMetadata {
    pub id: PackId,
    pub kind: PackKind,
    pub name: String,
    pub pack_code: Option<String>,
    pub author: String,
    pub version: String,
    pub description: Option<String>,
    pub created_at: AppTimestamp,
    pub updated_at: AppTimestamp,
    pub display_language_order: Vec<LanguageCode>,
    pub default_export_language: Option<LanguageCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackMetadataFile {
    pub schema_version: u32,
    pub data: PackMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackOverview {
    pub id: PackId,
    pub kind: PackKind,
    pub name: String,
    pub author: String,
    pub version: String,
    pub card_count: usize,
    pub updated_at: AppTimestamp,
}
