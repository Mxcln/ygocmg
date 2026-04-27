use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::common::ids::{LanguageCode, PackId, PreviewToken, WorkspaceId};
use crate::domain::common::issue::ValidationIssue;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewExportBundleInput {
    pub workspace_id: WorkspaceId,
    pub pack_ids: Vec<PackId>,
    pub export_language: LanguageCode,
    pub output_dir: PathBuf,
    pub output_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExportPreviewDto {
    pub pack_count: usize,
    pub card_count: usize,
    pub main_image_count: usize,
    pub field_image_count: usize,
    pub script_count: usize,
    pub warning_count: usize,
    pub error_count: usize,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteExportBundleInput {
    pub preview_token: PreviewToken,
}
