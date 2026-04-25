use std::collections::BTreeSet;

use uuid::Uuid;

use crate::bootstrap::AppState;
use crate::domain::card::code::{CodePolicy, CodeValidationContext, suggest_next_code, validate_card_code};
use crate::domain::card::derive::derive_card_list_row;
use crate::domain::card::model::{CardEntity, CardListRow, CardUpdateInput};
use crate::domain::card::normalize::{apply_card_update, create_card_entity, normalize_card_input};
use crate::domain::card::validate::{collect_card_warnings, validate_card_structure, validate_card_update_input};
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::issue::{IssueLevel, ValidationIssue};
use crate::domain::config::rules::default_global_config;
use crate::domain::resource::path_rules::{detect_card_asset_state, planned_asset_renames};
use crate::infrastructure::fs::transaction::{FsOperation, execute_plan};
use crate::infrastructure::json_store;

pub struct CardService<'a> {
    state: &'a AppState,
}

impl<'a> CardService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub fn list_cards(&self, pack_id: &str) -> AppResult<Vec<CardListRow>> {
        let sessions = self.state.sessions.read().map_err(|_| {
            AppError::new("card.session_lock_poisoned", "card session lock poisoned")
        })?;
        let pack = sessions
            .open_packs
            .get(pack_id)
            .ok_or_else(|| AppError::new("pack.not_open", "pack is not currently open"))?;

        let rows = pack
            .cards
            .iter()
            .map(|card| {
                let assets = detect_card_asset_state(&pack.pack_path, card.code);
                derive_card_list_row(card, &assets, &pack.metadata.display_language_order)
            })
            .collect();

        Ok(rows)
    }

    pub fn create_card(
        &self,
        pack_id: &str,
        input: CardUpdateInput,
    ) -> AppResult<(CardEntity, Vec<ValidationIssue>)> {
        let normalized = normalize_card_input(input);
        let structure_issues = validate_card_update_input(&normalized);
        if structure_issues
            .iter()
            .any(|issue| matches!(issue.level, IssueLevel::Error))
        {
            return Err(AppError::new(
                "card.validation_failed",
                "card input contains validation errors",
            ));
        }

        let context = self.build_code_context(pack_id, None)?;
        let code_issues = validate_card_code(normalized.code, &context);
        if code_issues
            .iter()
            .any(|issue| matches!(issue.level, IssueLevel::Error))
        {
            return Err(AppError::new(
                "card.code_validation_failed",
                "card code contains validation errors",
            ));
        }

        let now = crate::domain::common::time::now_utc();
        let card = create_card_entity(Uuid::now_v7().to_string(), normalized, now);

        let warnings = collect_card_warnings(&card, &context);
        crate::application::pack::service::persist_open_pack(self.state, pack_id, |pack| {
            pack.cards.push(card.clone());
            pack.cards.sort_by_key(|value| value.code);
            json_store::save_cards(&pack.pack_path, &pack.cards)?;
            crate::application::pack::service::touch_pack_and_persist(&pack.pack_path, &mut pack.metadata)
        })?;
        crate::application::pack::service::PackService::new(self.state).refresh_current_workspace_summary()?;
        Ok((card, warnings))
    }

    pub fn update_card(
        &self,
        pack_id: &str,
        card_id: &str,
        input: CardUpdateInput,
    ) -> AppResult<(CardEntity, Vec<ValidationIssue>)> {
        let normalized = normalize_card_input(input);
        let structure_issues = validate_card_update_input(&normalized);
        if structure_issues
            .iter()
            .any(|issue| matches!(issue.level, IssueLevel::Error))
        {
            return Err(AppError::new(
                "card.validation_failed",
                "card input contains validation errors",
            ));
        }

        let existing = {
            let sessions = self.state.sessions.read().map_err(|_| {
                AppError::new("card.session_lock_poisoned", "card session lock poisoned")
            })?;
            let pack = sessions
                .open_packs
                .get(pack_id)
                .ok_or_else(|| AppError::new("pack.not_open", "pack is not currently open"))?;
            pack.cards
                .iter()
                .find(|card| card.id == card_id)
                .cloned()
                .ok_or_else(|| AppError::new("card.not_found", "card was not found"))?
        };

        let context = self.build_code_context(pack_id, Some(card_id))?;
        let code_issues = validate_card_code(normalized.code, &context);
        if code_issues
            .iter()
            .any(|issue| matches!(issue.level, IssueLevel::Error))
        {
            return Err(AppError::new(
                "card.code_validation_failed",
                "card code contains validation errors",
            ));
        }

        let updated = apply_card_update(&existing, normalized, crate::domain::common::time::now_utc());
        let warnings = collect_card_warnings(&updated, &context);
        let old_code = existing.code;
        let new_code = updated.code;

        crate::application::pack::service::persist_open_pack(self.state, pack_id, |pack| {
            if old_code != new_code {
                let mut operations = Vec::new();
                for (from, to) in planned_asset_renames(&pack.pack_path, old_code, new_code) {
                    operations.push(FsOperation::Rename { from, to });
                }
                execute_plan(operations)?;
            }

            let card = pack
                .cards
                .iter_mut()
                .find(|card| card.id == card_id)
                .ok_or_else(|| AppError::new("card.not_found", "card was not found"))?;
            *card = updated.clone();

            let structure_post = validate_card_structure(card);
            if structure_post
                .iter()
                .any(|issue| matches!(issue.level, IssueLevel::Error))
            {
                return Err(AppError::new(
                    "card.post_validation_failed",
                    "updated card failed post validation",
                ));
            }

            pack.cards.sort_by_key(|value| value.code);
            json_store::save_cards(&pack.pack_path, &pack.cards)?;
            crate::application::pack::service::touch_pack_and_persist(&pack.pack_path, &mut pack.metadata)
        })?;
        crate::application::pack::service::PackService::new(self.state).refresh_current_workspace_summary()?;
        Ok((updated, warnings))
    }

    pub fn delete_card(&self, pack_id: &str, card_id: &str) -> AppResult<()> {
        crate::application::pack::service::persist_open_pack(self.state, pack_id, |pack| {
            let original_len = pack.cards.len();
            pack.cards.retain(|card| card.id != card_id);
            if pack.cards.len() == original_len {
                return Err(AppError::new("card.not_found", "card was not found"));
            }
            json_store::save_cards(&pack.pack_path, &pack.cards)?;
            crate::application::pack::service::touch_pack_and_persist(&pack.pack_path, &mut pack.metadata)
        })?;
        crate::application::pack::service::PackService::new(self.state).refresh_current_workspace_summary()
    }

    pub fn suggest_code(&self, pack_id: &str, preferred_start: Option<u32>) -> AppResult<Option<u32>> {
        let context = self.build_code_context(pack_id, None)?;
        Ok(suggest_next_code(&context, preferred_start))
    }

    fn build_code_context(
        &self,
        pack_id: &str,
        exclude_card_id: Option<&str>,
    ) -> AppResult<CodeValidationContext> {
        let config = json_store::load_global_config(self.state.app_data_dir())
            .unwrap_or_else(|_| default_global_config());
        let workspace_path = crate::application::workspace::service::WorkspaceService::new(self.state)
            .current_workspace_path()?;
        let workspace_meta = json_store::load_workspace_meta(&workspace_path)?;

        let mut workspace_codes = BTreeSet::new();
        for current_pack_id in workspace_meta.pack_order {
            let pack_path = json_store::pack_path(&workspace_path, &current_pack_id);
            let cards = json_store::load_cards(&pack_path).unwrap_or_default();
            for card in cards {
                if current_pack_id == pack_id && exclude_card_id == Some(card.id.as_str()) {
                    continue;
                }
                workspace_codes.insert(card.code);
            }
        }

        Ok(CodeValidationContext {
            policy: CodePolicy {
                reserved_max: 99_999_999,
                recommended_min: config.custom_code_recommended_min,
                recommended_max: config.custom_code_recommended_max,
                hard_max: 268_435_455,
                min_gap: config.custom_code_min_gap,
            },
            workspace_custom_codes: workspace_codes,
            standard_codes: BTreeSet::new(),
        })
    }
}
