use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::domain::card::model::{
    Attribute, CardEntity, CardListRow, CardTexts, CardUpdateInput, LinkData, MonsterFlag, Ot,
    Pendulum, PrimaryType, Race, SpellSubtype, TrapSubtype,
};
use crate::domain::common::ids::{CardId, LanguageCode, PackId, WorkspaceId};
use crate::domain::resource::model::CardAssetState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardSortFieldDto {
    Code,
    Name,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDirectionDto {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardListRowDto {
    pub id: CardId,
    pub code: u32,
    pub name: String,
    pub desc: String,
    pub primary_type: PrimaryType,
    pub subtype_display: String,
    pub atk: Option<i32>,
    pub def: Option<i32>,
    pub level: Option<i32>,
    pub has_image: bool,
    pub has_script: bool,
    pub has_field_image: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardTextsDto {
    pub name: String,
    pub desc: String,
    pub strings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendulumDto {
    pub left_scale: i32,
    pub right_scale: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkDataDto {
    pub markers: Vec<crate::domain::card::model::LinkMarker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditableCardDto {
    pub id: CardId,
    pub code: u32,
    pub alias: u32,
    pub setcode: u64,
    pub ot: Ot,
    pub category: u64,
    pub primary_type: PrimaryType,
    pub texts: BTreeMap<LanguageCode, CardTextsDto>,
    pub monster_flags: Option<Vec<MonsterFlag>>,
    pub atk: Option<i32>,
    pub def: Option<i32>,
    pub race: Option<Race>,
    pub attribute: Option<Attribute>,
    pub level: Option<i32>,
    pub pendulum: Option<PendulumDto>,
    pub link: Option<LinkDataDto>,
    pub spell_subtype: Option<SpellSubtype>,
    pub trap_subtype: Option<TrapSubtype>,
    pub created_at: crate::domain::common::time::AppTimestamp,
    pub updated_at: crate::domain::common::time::AppTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDetailDto {
    pub card: EditableCardDto,
    pub asset_state: CardAssetState,
    pub available_languages: Vec<LanguageCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardListPageDto {
    pub items: Vec<CardListRowDto>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCardsInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub keyword: Option<String>,
    pub sort_by: CardSortFieldDto,
    pub sort_direction: SortDirectionDto,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCardInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestCodeInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub preferred_start: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSuggestionDto {
    pub suggested_code: Option<u32>,
    pub warnings: Vec<crate::domain::common::issue::ValidationIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCardInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card: CardUpdateInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCardInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
    pub card: CardUpdateInput,
}

impl From<CardListRow> for CardListRowDto {
    fn from(value: CardListRow) -> Self {
        Self {
            id: value.id,
            code: value.code,
            name: value.name,
            desc: value.desc,
            primary_type: value.primary_type,
            subtype_display: value.subtype_display,
            atk: value.atk,
            def: value.def,
            level: value.level,
            has_image: value.has_image,
            has_script: value.has_script,
            has_field_image: value.has_field_image,
        }
    }
}

impl From<CardTexts> for CardTextsDto {
    fn from(value: CardTexts) -> Self {
        Self {
            name: value.name,
            desc: value.desc,
            strings: value.strings,
        }
    }
}

impl From<Pendulum> for PendulumDto {
    fn from(value: Pendulum) -> Self {
        Self {
            left_scale: value.left_scale,
            right_scale: value.right_scale,
        }
    }
}

impl From<LinkData> for LinkDataDto {
    fn from(value: LinkData) -> Self {
        Self {
            markers: value.markers,
        }
    }
}

impl From<CardEntity> for EditableCardDto {
    fn from(value: CardEntity) -> Self {
        Self {
            id: value.id,
            code: value.code,
            alias: value.alias,
            setcode: value.setcode,
            ot: value.ot,
            category: value.category,
            primary_type: value.primary_type,
            texts: value
                .texts
                .into_iter()
                .map(|(language, texts)| (language, texts.into()))
                .collect(),
            monster_flags: value.monster_flags,
            atk: value.atk,
            def: value.def,
            race: value.race,
            attribute: value.attribute,
            level: value.level,
            pendulum: value.pendulum.map(Into::into),
            link: value.link.map(Into::into),
            spell_subtype: value.spell_subtype,
            trap_subtype: value.trap_subtype,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
