use serde::{Deserialize, Serialize};

use crate::domain::common::ids::{ConfirmationToken, LanguageCode, PackId, WorkspaceId};
use crate::domain::strings::model::{
    PackStringEntry, PackStringKind, PackStringRecord,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackStringEntryDto {
    pub kind: PackStringKind,
    pub key: u32,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackStringsPageDto {
    pub language: LanguageCode,
    pub items: Vec<PackStringEntryDto>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackStringValueDto {
    pub language: LanguageCode,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackStringRecordDto {
    pub kind: PackStringKind,
    pub key: u32,
    pub values: Vec<PackStringValueDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackStringRecordDetailDto {
    pub record: PackStringRecordDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPackStringsInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub language: LanguageCode,
    pub kind_filter: Option<PackStringKind>,
    pub key_filter: Option<u32>,
    pub keyword: Option<String>,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertPackStringInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub language: LanguageCode,
    pub entry: PackStringEntryDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPackStringInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub kind: PackStringKind,
    pub key: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertPackStringRecordInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub record: PackStringRecordDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePackStringsInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub entries: Vec<PackStringKeyDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemovePackStringTranslationInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub kind: PackStringKind,
    pub key: u32,
    pub language: LanguageCode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackStringKeyDto {
    pub kind: PackStringKind,
    pub key: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeletePackStringsResultDto {
    pub deleted_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmPackStringsWriteInput {
    pub confirmation_token: ConfirmationToken,
}

impl From<PackStringEntry> for PackStringEntryDto {
    fn from(value: PackStringEntry) -> Self {
        Self {
            kind: value.kind,
            key: value.key,
            value: value.value,
        }
    }
}

impl From<PackStringEntryDto> for PackStringEntry {
    fn from(value: PackStringEntryDto) -> Self {
        Self {
            kind: value.kind,
            key: value.key,
            value: value.value,
        }
    }
}

impl From<PackStringRecord> for PackStringRecordDto {
    fn from(value: PackStringRecord) -> Self {
        Self {
            kind: value.kind,
            key: value.key,
            values: value
                .values
                .into_iter()
                .map(|(language, value)| PackStringValueDto { language, value })
                .collect(),
        }
    }
}

impl From<PackStringRecordDto> for PackStringRecord {
    fn from(value: PackStringRecordDto) -> Self {
        Self {
            kind: value.kind,
            key: value.key,
            values: value
                .values
                .into_iter()
                .map(|item| (item.language, item.value))
                .collect(),
        }
    }
}
