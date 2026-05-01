use std::collections::{BTreeMap, BTreeSet};

use crate::application::dto::card::{
    CardDetailDto, CardFilterMatchModeDto, CardListPageDto, CardListRowDto, CardSearchFiltersDto,
    CardSortFieldDto, CodeSuggestionDto, GetCardInput, ListCardsInput, NumericRangeFilterDto,
    SetcodeFilterModeDto, SortDirectionDto, SuggestCodeInput,
};
use crate::application::standard_pack::repository::{
    SqliteStandardPackRepository, StandardPackRepository,
};
use crate::bootstrap::AppState;
use crate::domain::card::code::{
    CodePolicy, CodeValidationContext, STANDARD_RESERVED_CODE_MAX, suggest_next_code,
};
use crate::domain::card::model::{CardEntity, CardListRow};
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::issue::ValidationIssue;
use crate::domain::config::rules::default_global_config;
use crate::domain::namespace::model::{
    WorkspaceNamespaceIndex, build_pack_strings_namespace_index,
};
use crate::infrastructure::json_store;
use crate::infrastructure::pack_locator;

pub struct CardService<'a> {
    state: &'a AppState,
}

impl<'a> CardService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub fn list_cards(&self, input: ListCardsInput) -> AppResult<CardListPageDto> {
        let pack = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            &input.workspace_id,
            &input.pack_id,
        )?;

        let keyword = input.keyword.unwrap_or_default().trim().to_lowercase();
        let filters = input.filters.map(normalize_card_search_filters);
        let cards_by_id = pack
            .cards
            .iter()
            .map(|card| (card.id.as_str(), card))
            .collect::<BTreeMap<_, _>>();
        let mut rows = pack
            .card_list_cache
            .iter()
            .filter_map(|row| cards_by_id.get(row.id.as_str()).map(|card| (row, *card)))
            .filter(|(row, card)| {
                matches_keyword(row, &keyword)
                    && filters
                        .as_ref()
                        .map_or(true, |filters| matches_card_filters(card, row, filters))
            })
            .map(|(row, _)| CardListRowDto::from(row.clone()))
            .collect::<Vec<_>>();

        match input.sort_by {
            CardSortFieldDto::Code => rows.sort_by(|left, right| left.code.cmp(&right.code)),
            CardSortFieldDto::Name => rows.sort_by(|left, right| left.name.cmp(&right.name)),
        }

        if matches!(input.sort_direction, SortDirectionDto::Desc) {
            rows.reverse();
        }

        let page_size = input.page_size.max(1);
        let page = input.page.max(1);
        let total = rows.len() as u64;
        let start = ((page - 1) as usize).saturating_mul(page_size as usize);
        let items = if start >= rows.len() {
            Vec::new()
        } else {
            rows.into_iter()
                .skip(start)
                .take(page_size as usize)
                .collect()
        };

        Ok(CardListPageDto {
            items,
            page,
            page_size,
            total,
            pack_path: pack.pack_path.to_string_lossy().to_string(),
            revision: pack.revision,
        })
    }

    pub fn get_card(&self, input: GetCardInput) -> AppResult<CardDetailDto> {
        let pack = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            &input.workspace_id,
            &input.pack_id,
        )?;
        let card = pack
            .cards
            .iter()
            .find(|card| card.id == input.card_id)
            .cloned()
            .ok_or_else(|| AppError::new("card.not_found", "card was not found"))?;
        let asset_state = pack
            .asset_index
            .get(&input.card_id)
            .cloned()
            .unwrap_or_default();
        let available_languages = card.texts.keys().cloned().collect::<Vec<_>>();

        Ok(CardDetailDto {
            card: card.into(),
            asset_state,
            available_languages,
            pack_path: pack.pack_path.to_string_lossy().to_string(),
        })
    }

    pub fn suggest_code(&self, input: SuggestCodeInput) -> AppResult<CodeSuggestionDto> {
        crate::application::pack::service::ensure_workspace_matches(
            self.state,
            &input.workspace_id,
        )?;
        let context = self.build_code_context(&input.pack_id, None)?;
        let suggested_code = suggest_next_code(&context, input.preferred_start);
        let warnings = suggested_code
            .map(|code| self.suggestion_warnings(&context, code))
            .unwrap_or_default();
        Ok(CodeSuggestionDto {
            suggested_code,
            warnings,
        })
    }

    pub fn build_code_context(
        &self,
        pack_id: &str,
        exclude_card_id: Option<&str>,
    ) -> AppResult<CodeValidationContext> {
        let config = json_store::load_global_config(self.state.app_data_dir())
            .unwrap_or_else(|_| default_global_config());
        let workspace_path =
            crate::application::workspace::service::WorkspaceService::new(self.state)
                .current_workspace_path()?;
        let workspace_meta = json_store::load_workspace_meta(&workspace_path)?;
        let inventory = crate::application::pack::service::load_pack_inventory(&workspace_path)?;

        let mut current_pack_codes = BTreeSet::new();
        let mut other_custom_codes = BTreeSet::new();
        for current_pack_id in workspace_meta.pack_order {
            let pack_path = pack_locator::resolve_pack_path(&inventory, &current_pack_id)?;
            let cards = json_store::load_cards(&pack_path).unwrap_or_default();
            for card in cards {
                if current_pack_id == pack_id && exclude_card_id == Some(card.id.as_str()) {
                    continue;
                }
                if current_pack_id == pack_id {
                    current_pack_codes.insert(card.code);
                } else {
                    other_custom_codes.insert(card.code);
                }
            }
        }

        let baseline = SqliteStandardPackRepository::new(self.state)
            .namespace_baseline()
            .unwrap_or_else(|_| self.state.standard_baseline.clone());

        Ok(CodeValidationContext {
            policy: CodePolicy {
                reserved_max: STANDARD_RESERVED_CODE_MAX,
                recommended_min: config.custom_code_recommended_min,
                recommended_max: config.custom_code_recommended_max,
                hard_max: 268_435_455,
                min_gap: config.custom_code_min_gap,
            },
            current_pack_codes,
            other_custom_codes,
            standard_codes: baseline.standard_codes,
        })
    }

    pub fn build_workspace_namespace_index(
        &self,
        exclude_pack_id: Option<&str>,
    ) -> AppResult<WorkspaceNamespaceIndex> {
        let workspace_path =
            crate::application::workspace::service::WorkspaceService::new(self.state)
                .current_workspace_path()?;
        let workspace_meta = json_store::load_workspace_meta(&workspace_path)?;
        let inventory = crate::application::pack::service::load_pack_inventory(&workspace_path)?;

        let mut index = WorkspaceNamespaceIndex::default();
        for current_pack_id in workspace_meta.pack_order {
            if exclude_pack_id == Some(current_pack_id.as_str()) {
                continue;
            }

            let pack_path = pack_locator::resolve_pack_path(&inventory, &current_pack_id)?;
            let cards = json_store::load_cards(&pack_path).unwrap_or_default();
            let strings = json_store::load_pack_strings(&pack_path).unwrap_or_default();

            index.codes_by_pack.insert(
                current_pack_id.clone(),
                cards.into_iter().map(|card| card.code).collect(),
            );
            index.strings_by_pack.insert(
                current_pack_id,
                build_pack_strings_namespace_index(&strings),
            );
        }

        Ok(index)
    }

    fn suggestion_warnings(
        &self,
        context: &CodeValidationContext,
        suggested_code: u32,
    ) -> Vec<ValidationIssue> {
        crate::domain::card::code::validate_card_code(suggested_code, context)
            .into_iter()
            .filter(|issue| {
                matches!(
                    issue.level,
                    crate::domain::common::issue::IssueLevel::Warning
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, Default)]
struct NormalizedCardSearchFilters {
    codes: Vec<u32>,
    code_range: Option<NumericRangeFilterDto>,
    aliases: Vec<u32>,
    alias_range: Option<NumericRangeFilterDto>,
    ots: Vec<crate::domain::card::model::Ot>,
    name_contains: Option<String>,
    desc_contains: Option<String>,
    primary_types: Vec<crate::domain::card::model::PrimaryType>,
    races: Vec<crate::domain::card::model::Race>,
    attributes: Vec<crate::domain::card::model::Attribute>,
    monster_flags: Vec<crate::domain::card::model::MonsterFlag>,
    monster_flag_match: CardFilterMatchModeDto,
    spell_subtypes: Vec<crate::domain::card::model::SpellSubtype>,
    trap_subtypes: Vec<crate::domain::card::model::TrapSubtype>,
    pendulum_left_scale: Option<NumericRangeFilterDto>,
    pendulum_right_scale: Option<NumericRangeFilterDto>,
    link_markers: Vec<crate::domain::card::model::LinkMarker>,
    link_marker_match: CardFilterMatchModeDto,
    setcodes: Vec<u16>,
    setcode_mode: SetcodeFilterModeDto,
    setcode_match: CardFilterMatchModeDto,
    category_masks: Vec<u64>,
    category_match: CardFilterMatchModeDto,
    atk: Option<NumericRangeFilterDto>,
    def: Option<NumericRangeFilterDto>,
    level: Option<NumericRangeFilterDto>,
}

fn normalize_card_search_filters(filters: CardSearchFiltersDto) -> NormalizedCardSearchFilters {
    NormalizedCardSearchFilters {
        codes: unique_u32(filters.codes),
        code_range: normalize_range(filters.code_range),
        aliases: unique_u32(filters.aliases),
        alias_range: normalize_range(filters.alias_range),
        ots: unique_values(filters.ots),
        name_contains: normalize_contains(filters.name_contains),
        desc_contains: normalize_contains(filters.desc_contains),
        primary_types: unique_values(filters.primary_types),
        races: unique_values(filters.races),
        attributes: unique_values(filters.attributes),
        monster_flags: unique_values(filters.monster_flags),
        monster_flag_match: filters.monster_flag_match.unwrap_or_default(),
        spell_subtypes: unique_values(filters.spell_subtypes),
        trap_subtypes: unique_values(filters.trap_subtypes),
        pendulum_left_scale: normalize_range(filters.pendulum_left_scale),
        pendulum_right_scale: normalize_range(filters.pendulum_right_scale),
        link_markers: unique_values(filters.link_markers),
        link_marker_match: filters.link_marker_match.unwrap_or_default(),
        setcodes: filters
            .setcodes
            .unwrap_or_default()
            .into_iter()
            .filter(|value| *value != 0)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        setcode_mode: filters.setcode_mode.unwrap_or_default(),
        setcode_match: filters.setcode_match.unwrap_or_default(),
        category_masks: filters
            .category_masks
            .unwrap_or_default()
            .into_iter()
            .filter(|value| *value != 0)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        category_match: filters.category_match.unwrap_or_default(),
        atk: normalize_range(filters.atk),
        def: normalize_range(filters.def),
        level: normalize_range(filters.level),
    }
}

fn matches_keyword(row: &CardListRow, keyword: &str) -> bool {
    if keyword.is_empty() {
        return true;
    }
    row.name.to_lowercase().contains(keyword)
        || row.desc.to_lowercase().contains(keyword)
        || row.code.to_string().contains(keyword)
        || row.subtype_display.to_lowercase().contains(keyword)
        || primary_type_text(&row.primary_type).contains(keyword)
}

fn matches_card_filters(
    card: &CardEntity,
    row: &CardListRow,
    filters: &NormalizedCardSearchFilters,
) -> bool {
    if !filters.codes.is_empty() && !filters.codes.contains(&card.code) {
        return false;
    }
    if !matches_u32_range(card.code, filters.code_range.as_ref()) {
        return false;
    }
    if !filters.aliases.is_empty() && !filters.aliases.contains(&card.alias) {
        return false;
    }
    if !matches_u32_range(card.alias, filters.alias_range.as_ref()) {
        return false;
    }
    if !filters.ots.is_empty() && !filters.ots.contains(&card.ot) {
        return false;
    }
    if !matches_contains(&row.name, filters.name_contains.as_deref()) {
        return false;
    }
    if !matches_contains(&row.desc, filters.desc_contains.as_deref()) {
        return false;
    }
    if !filters.primary_types.is_empty() && !filters.primary_types.contains(&card.primary_type) {
        return false;
    }
    if !matches_optional_enum(card.race.as_ref(), &filters.races) {
        return false;
    }
    if !matches_optional_enum(card.attribute.as_ref(), &filters.attributes) {
        return false;
    }
    if !matches_collection(
        card.monster_flags.as_deref().unwrap_or_default(),
        &filters.monster_flags,
        &filters.monster_flag_match,
    ) {
        return false;
    }
    if !matches_optional_enum(card.spell_subtype.as_ref(), &filters.spell_subtypes) {
        return false;
    }
    if !matches_optional_enum(card.trap_subtype.as_ref(), &filters.trap_subtypes) {
        return false;
    }
    if !matches_pendulum(card, filters) {
        return false;
    }
    if !matches_collection(
        card.link
            .as_ref()
            .map(|link| link.markers.as_slice())
            .unwrap_or_default(),
        &filters.link_markers,
        &filters.link_marker_match,
    ) {
        return false;
    }
    if !matches_setcodes(card, filters) {
        return false;
    }
    if !matches_category(card.category, filters) {
        return false;
    }
    matches_i32_option_range(card.atk, filters.atk.as_ref())
        && matches_i32_option_range(card.def, filters.def.as_ref())
        && matches_i32_option_range(card.level, filters.level.as_ref())
}

fn matches_contains(value: &str, needle: Option<&str>) -> bool {
    needle.map_or(true, |needle| value.to_lowercase().contains(needle))
}

fn matches_optional_enum<T: PartialEq>(value: Option<&T>, filters: &[T]) -> bool {
    filters.is_empty() || value.is_some_and(|value| filters.contains(value))
}

fn matches_collection<T: PartialEq>(
    values: &[T],
    filters: &[T],
    match_mode: &CardFilterMatchModeDto,
) -> bool {
    if filters.is_empty() {
        return true;
    }
    match match_mode {
        CardFilterMatchModeDto::Any => filters.iter().any(|filter| values.contains(filter)),
        CardFilterMatchModeDto::All => filters.iter().all(|filter| values.contains(filter)),
    }
}

fn matches_pendulum(card: &CardEntity, filters: &NormalizedCardSearchFilters) -> bool {
    if filters.pendulum_left_scale.is_none() && filters.pendulum_right_scale.is_none() {
        return true;
    }
    let Some(pendulum) = &card.pendulum else {
        return false;
    };
    matches_i32_range(pendulum.left_scale, filters.pendulum_left_scale.as_ref())
        && matches_i32_range(pendulum.right_scale, filters.pendulum_right_scale.as_ref())
}

fn matches_setcodes(card: &CardEntity, filters: &NormalizedCardSearchFilters) -> bool {
    if filters.setcodes.is_empty() {
        return true;
    }
    let selected = match filters.setcode_mode {
        SetcodeFilterModeDto::Exact => filters.setcodes.clone(),
        SetcodeFilterModeDto::Base => filters
            .setcodes
            .iter()
            .map(|value| value & 0x0fff)
            .filter(|value| *value != 0)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    };
    if selected.is_empty() {
        return true;
    }
    let card_values = match filters.setcode_mode {
        SetcodeFilterModeDto::Exact => card
            .setcodes
            .iter()
            .copied()
            .filter(|value| *value != 0)
            .collect::<BTreeSet<_>>(),
        SetcodeFilterModeDto::Base => card
            .setcodes
            .iter()
            .map(|value| value & 0x0fff)
            .filter(|value| *value != 0)
            .collect::<BTreeSet<_>>(),
    };
    match filters.setcode_match {
        CardFilterMatchModeDto::Any => selected.iter().any(|value| card_values.contains(value)),
        CardFilterMatchModeDto::All => selected.iter().all(|value| card_values.contains(value)),
    }
}

fn matches_category(category: u64, filters: &NormalizedCardSearchFilters) -> bool {
    let combined = filters
        .category_masks
        .iter()
        .fold(0u64, |acc, value| acc | *value);
    if combined == 0 {
        return true;
    }
    match filters.category_match {
        CardFilterMatchModeDto::Any => (category & combined) != 0,
        CardFilterMatchModeDto::All => (category & combined) == combined,
    }
}

fn matches_u32_range(value: u32, range: Option<&NumericRangeFilterDto>) -> bool {
    matches_i64_range(value as i64, range)
}

fn matches_i32_range(value: i32, range: Option<&NumericRangeFilterDto>) -> bool {
    matches_i64_range(value as i64, range)
}

fn matches_i32_option_range(value: Option<i32>, range: Option<&NumericRangeFilterDto>) -> bool {
    range.map_or(true, |range| {
        value.is_some_and(|value| matches_i64_range(value as i64, Some(range)))
    })
}

fn matches_i64_range(value: i64, range: Option<&NumericRangeFilterDto>) -> bool {
    let Some(range) = range else {
        return true;
    };
    if range.min.is_some_and(|min| value < min) {
        return false;
    }
    if range.max.is_some_and(|max| value > max) {
        return false;
    }
    true
}

fn primary_type_text(value: &crate::domain::card::model::PrimaryType) -> &'static str {
    match value {
        crate::domain::card::model::PrimaryType::Monster => "monster",
        crate::domain::card::model::PrimaryType::Spell => "spell",
        crate::domain::card::model::PrimaryType::Trap => "trap",
    }
}

fn normalize_contains(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
}

fn normalize_range(value: Option<NumericRangeFilterDto>) -> Option<NumericRangeFilterDto> {
    value.filter(|range| range.min.is_some() || range.max.is_some())
}

fn unique_u32(values: Option<Vec<u32>>) -> Vec<u32> {
    values
        .unwrap_or_default()
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn unique_values<T: PartialEq>(values: Option<Vec<T>>) -> Vec<T> {
    let mut unique = Vec::new();
    for value in values.unwrap_or_default() {
        if !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}
