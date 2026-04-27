use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::common::ids::{CardId, PackId, WorkspaceId};
use crate::domain::resource::model::CardAssetState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CardAssetStateDto {
    pub has_image: bool,
    pub has_script: bool,
    pub has_field_image: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportMainImageInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
    pub source_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteMainImageInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFieldImageInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
    pub source_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFieldImageInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEmptyScriptInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportScriptInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
    pub source_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteScriptInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenScriptExternalInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
}

impl From<CardAssetState> for CardAssetStateDto {
    fn from(value: CardAssetState) -> Self {
        Self {
            has_image: value.has_image,
            has_script: value.has_script,
            has_field_image: value.has_field_image,
        }
    }
}
