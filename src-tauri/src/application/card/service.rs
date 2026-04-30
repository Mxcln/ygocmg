use std::collections::BTreeSet;

use crate::application::dto::card::{
    CardDetailDto, CardListPageDto, CardListRowDto, CardSortFieldDto, CodeSuggestionDto,
    GetCardInput, ListCardsInput, SortDirectionDto, SuggestCodeInput,
};
use crate::application::standard_pack::repository::{
    JsonStandardPackRepository, StandardPackRepository,
};
use crate::bootstrap::AppState;
use crate::domain::card::code::{
    CodePolicy, CodeValidationContext, STANDARD_RESERVED_CODE_MAX, suggest_next_code,
};
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
        let mut rows = pack
            .card_list_cache
            .iter()
            .filter(|row| {
                if keyword.is_empty() {
                    return true;
                }
                row.name.to_lowercase().contains(&keyword)
                    || row.desc.to_lowercase().contains(&keyword)
                    || row.code.to_string().contains(&keyword)
            })
            .cloned()
            .map(CardListRowDto::from)
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

        let baseline = JsonStandardPackRepository::new(self.state)
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
