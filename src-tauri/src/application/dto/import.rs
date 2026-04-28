use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::common::ids::{LanguageCode, PackId, PreviewToken, WorkspaceId};
use crate::domain::common::issue::ValidationIssue;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewImportPackInput {
    pub workspace_id: WorkspaceId,
    pub new_pack_name: String,
    pub new_pack_author: String,
    pub new_pack_version: String,
    pub new_pack_description: Option<String>,
    pub display_language_order: Vec<LanguageCode>,
    pub default_export_language: Option<LanguageCode>,
    pub cdb_path: PathBuf,
    pub pics_dir: Option<PathBuf>,
    pub field_pics_dir: Option<PathBuf>,
    pub script_dir: Option<PathBuf>,
    pub strings_conf_path: Option<PathBuf>,
    pub source_language: LanguageCode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportPreviewDto {
    pub target_pack_id: PackId,
    pub target_pack_name: String,
    pub card_count: usize,
    pub warning_count: usize,
    pub error_count: usize,
    pub missing_main_image_count: usize,
    pub missing_script_count: usize,
    pub missing_field_image_count: usize,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteImportPackInput {
    pub preview_token: PreviewToken,
}
