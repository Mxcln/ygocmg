use serde::{Deserialize, Serialize};

use crate::application::dto::card::{CardListRowDto, EditableCardDto};
use crate::application::dto::strings::PackStringEntryDto;
use crate::domain::common::time::AppTimestamp;
use crate::domain::resource::model::CardAssetState;
use crate::domain::strings::model::PackStringKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StandardCardSortFieldDto {
    Code,
    Name,
    Type,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StandardStringSortFieldDto {
    Kind,
    Key,
    Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StandardPackIndexStateDto {
    NotConfigured,
    MissingSource,
    MissingIndex,
    Stale,
    Ready,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardPackStatusDto {
    pub configured: bool,
    pub ygopro_path: Option<String>,
    pub cdb_path: Option<String>,
    pub index_exists: bool,
    pub schema_mismatch: bool,
    pub stale: bool,
    pub indexed_at: Option<AppTimestamp>,
    pub card_count: usize,
    pub state: StandardPackIndexStateDto,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchStandardCardsInput {
    pub keyword: Option<String>,
    pub sort_by: StandardCardSortFieldDto,
    pub sort_direction: crate::application::dto::card::SortDirectionDto,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardCardPageDto {
    pub items: Vec<CardListRowDto>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
    pub ygopro_path: Option<String>,
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetStandardCardInput {
    pub code: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardCardDetailDto {
    pub card: EditableCardDto,
    pub asset_state: CardAssetState,
    pub available_languages: Vec<String>,
    pub ygopro_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchStandardStringsInput {
    pub kind_filter: Option<PackStringKind>,
    pub key_filter: Option<u32>,
    pub keyword: Option<String>,
    pub sort_by: StandardStringSortFieldDto,
    pub sort_direction: crate::application::dto::card::SortDirectionDto,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardStringsPageDto {
    pub language: String,
    pub items: Vec<PackStringEntryDto>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
    pub revision: u64,
}
