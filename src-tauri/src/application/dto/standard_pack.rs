use serde::{Deserialize, Serialize};

use crate::application::dto::card::{CardListRowDto, EditableCardDto};
use crate::application::dto::strings::PackStringEntryDto;
use crate::domain::card::model::{
    Attribute, LinkMarker, MonsterFlag, Ot, PrimaryType, Race, SpellSubtype, TrapSubtype,
};
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CardFilterMatchModeDto {
    #[default]
    Any,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SetcodeFilterModeDto {
    Exact,
    #[default]
    Base,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct NumericRangeFilterDto {
    pub min: Option<i64>,
    pub max: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StandardCardSearchFiltersDto {
    pub codes: Option<Vec<u32>>,
    pub code_range: Option<NumericRangeFilterDto>,
    pub aliases: Option<Vec<u32>>,
    pub alias_range: Option<NumericRangeFilterDto>,
    pub ots: Option<Vec<Ot>>,

    pub name_contains: Option<String>,
    pub desc_contains: Option<String>,

    pub primary_types: Option<Vec<PrimaryType>>,
    pub races: Option<Vec<Race>>,
    pub attributes: Option<Vec<Attribute>>,

    pub monster_flags: Option<Vec<MonsterFlag>>,
    pub monster_flag_match: Option<CardFilterMatchModeDto>,

    pub spell_subtypes: Option<Vec<SpellSubtype>>,
    pub trap_subtypes: Option<Vec<TrapSubtype>>,

    pub pendulum_left_scale: Option<NumericRangeFilterDto>,
    pub pendulum_right_scale: Option<NumericRangeFilterDto>,

    pub link_markers: Option<Vec<LinkMarker>>,
    pub link_marker_match: Option<CardFilterMatchModeDto>,

    pub setcodes: Option<Vec<u16>>,
    pub setcode_mode: Option<SetcodeFilterModeDto>,
    pub setcode_match: Option<CardFilterMatchModeDto>,

    pub category_masks: Option<Vec<u64>>,
    pub category_match: Option<CardFilterMatchModeDto>,

    pub atk: Option<NumericRangeFilterDto>,
    pub def: Option<NumericRangeFilterDto>,
    pub level: Option<NumericRangeFilterDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StandardPackIndexStateDto {
    NotConfigured,
    MissingSource,
    MissingLanguage,
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
    pub source_language: Option<String>,
    pub indexed_at: Option<AppTimestamp>,
    pub card_count: usize,
    pub state: StandardPackIndexStateDto,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchStandardCardsInput {
    pub keyword: Option<String>,
    #[serde(default)]
    pub filters: Option<StandardCardSearchFiltersDto>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListStandardSetnamesInput {
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardSetnameEntryDto {
    pub key: u32,
    pub value: String,
}
