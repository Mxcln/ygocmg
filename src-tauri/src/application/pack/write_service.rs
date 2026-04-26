use uuid::Uuid;

use crate::bootstrap::AppState;
use crate::domain::card::code::{validate_card_code, CodeValidationContext};
use crate::domain::card::model::{CardEntity, CardUpdateInput};
use crate::domain::card::normalize::{apply_card_update, create_card_entity, normalize_card_input};
use crate::domain::card::validate::{
    collect_card_warnings, validate_card_structure, validate_card_update_input,
};
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::ids::{CardId, PackId, WorkspaceId};
use crate::domain::common::issue::{IssueLevel, ValidationIssue};
use crate::domain::common::time::now_utc;
use crate::infrastructure::fs::transaction::{execute_plan, FsOperation};
use crate::infrastructure::json_store;
use crate::runtime::sessions::PackSession;

#[derive(Debug, Clone)]
pub struct PreparedCreateCardWrite {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub snapshot: PackSession,
    pub normalized_input: CardUpdateInput,
    pub card: CardEntity,
    pub warnings: Vec<ValidationIssue>,
}

#[derive(Debug, Clone)]
pub struct PreparedUpdateCardWrite {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub snapshot: PackSession,
    pub normalized_input: CardUpdateInput,
    pub existing: CardEntity,
    pub updated: CardEntity,
    pub warnings: Vec<ValidationIssue>,
}

pub struct PackWriteService<'a> {
    state: &'a AppState,
}

impl<'a> PackWriteService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub fn create_card(
        &self,
        workspace_id: &str,
        pack_id: &str,
        input: CardUpdateInput,
        code_context: CodeValidationContext,
    ) -> AppResult<(PackSession, CardEntity, Vec<ValidationIssue>)> {
        let prepared = self.prepare_create_card(workspace_id, pack_id, input, code_context)?;
        let next_session = self.commit_prepared_create_card_session(&prepared)?;
        Ok((next_session, prepared.card, prepared.warnings))
    }

    pub fn update_card(
        &self,
        workspace_id: &str,
        pack_id: &PackId,
        card_id: &str,
        input: CardUpdateInput,
        code_context: CodeValidationContext,
    ) -> AppResult<(PackSession, CardEntity, Vec<ValidationIssue>)> {
        let prepared =
            self.prepare_update_card(workspace_id, pack_id, card_id, input, code_context)?;
        let next_session = self.commit_prepared_update_card_session(&prepared)?;
        Ok((next_session, prepared.updated, prepared.warnings))
    }

    pub fn prepare_create_card(
        &self,
        workspace_id: &str,
        pack_id: &str,
        input: CardUpdateInput,
        code_context: CodeValidationContext,
    ) -> AppResult<PreparedCreateCardWrite> {
        self.prepare_create_card_with_seed(workspace_id, pack_id, input, code_context, None)
    }

    pub fn prepare_create_card_with_seed(
        &self,
        workspace_id: &str,
        pack_id: &str,
        input: CardUpdateInput,
        code_context: CodeValidationContext,
        seeded_card: Option<CardEntity>,
    ) -> AppResult<PreparedCreateCardWrite> {
        let snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            workspace_id,
            pack_id,
        )?;
        let now = now_utc();
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

        let code_issues = validate_card_code(normalized.code, &code_context);
        if code_issues
            .iter()
            .any(|issue| matches!(issue.level, IssueLevel::Error))
        {
            return Err(AppError::new(
                "card.code_validation_failed",
                "card code contains validation errors",
            ));
        }

        let card = if let Some(existing_seed) = seeded_card {
            create_card_entity(existing_seed.id, normalized.clone(), existing_seed.created_at)
        } else {
            create_card_entity(Uuid::now_v7().to_string(), normalized.clone(), now)
        };
        let warnings = collect_card_warnings(&card, &code_context);

        Ok(PreparedCreateCardWrite {
            workspace_id: workspace_id.to_string(),
            pack_id: pack_id.to_string(),
            snapshot,
            normalized_input: normalized,
            card,
            warnings,
        })
    }

    pub fn prepare_update_card(
        &self,
        workspace_id: &str,
        pack_id: &PackId,
        card_id: &str,
        input: CardUpdateInput,
        code_context: CodeValidationContext,
    ) -> AppResult<PreparedUpdateCardWrite> {
        let snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            workspace_id,
            pack_id,
        )?;
        let now = now_utc();
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

        let existing = snapshot
            .cards
            .iter()
            .find(|card| card.id == card_id)
            .cloned()
            .ok_or_else(|| AppError::new("card.not_found", "card was not found"))?;

        let code_issues = validate_card_code(normalized.code, &code_context);
        if code_issues
            .iter()
            .any(|issue| matches!(issue.level, IssueLevel::Error))
        {
            return Err(AppError::new(
                "card.code_validation_failed",
                "card code contains validation errors",
            ));
        }

        let updated = apply_card_update(&existing, normalized.clone(), now);
        let warnings = collect_card_warnings(&updated, &code_context);

        let structure_post = validate_card_structure(&updated);
        if structure_post
            .iter()
            .any(|issue| matches!(issue.level, IssueLevel::Error))
        {
            return Err(AppError::new(
                "card.post_validation_failed",
                "updated card failed post validation",
            ));
        }

        Ok(PreparedUpdateCardWrite {
            workspace_id: workspace_id.to_string(),
            pack_id: pack_id.clone(),
            snapshot,
            normalized_input: normalized,
            existing,
            updated,
            warnings,
        })
    }

    pub fn commit_prepared_create_card(
        &self,
        prepared: &PreparedCreateCardWrite,
    ) -> AppResult<CardId> {
        self.commit_prepared_create_card_session(prepared)
            .map(|_| prepared.card.id.clone())
    }

    pub fn commit_prepared_update_card(
        &self,
        prepared: &PreparedUpdateCardWrite,
    ) -> AppResult<CardId> {
        self.commit_prepared_update_card_session(prepared)
            .map(|_| prepared.updated.id.clone())
    }

    pub fn commit_prepared_create_card_session(
        &self,
        prepared: &PreparedCreateCardWrite,
    ) -> AppResult<PackSession> {
        let now = now_utc();
        let mut next_cards = prepared.snapshot.cards.clone();
        next_cards.push(prepared.card.clone());
        next_cards.sort_by_key(|value| value.code);

        let mut next_metadata = prepared.snapshot.metadata.clone();
        next_metadata = crate::domain::pack::summary::touch_pack_metadata(&next_metadata, now);

        execute_plan(vec![
            FsOperation::WriteFile {
                path: json_store::cards_path(&prepared.snapshot.pack_path),
                contents: encode_cards(&next_cards)?,
            },
            FsOperation::WriteFile {
                path: json_store::pack_metadata_path(&prepared.snapshot.pack_path),
                contents: encode_pack_metadata(&next_metadata)?,
            },
        ])?;

        let next_session = crate::application::pack::service::build_pack_session(
            prepared.snapshot.pack_path.clone(),
            next_metadata,
            next_cards,
            prepared.snapshot.strings.clone(),
            prepared.snapshot.revision + 1,
        )?;

        self.replace_and_refresh(&prepared.workspace_id, &prepared.pack_id, next_session)
    }

    pub fn commit_prepared_update_card_session(
        &self,
        prepared: &PreparedUpdateCardWrite,
    ) -> AppResult<PackSession> {
        let mut next_cards = prepared.snapshot.cards.clone();
        let card = next_cards
            .iter_mut()
            .find(|card| card.id == prepared.updated.id)
            .ok_or_else(|| AppError::new("card.not_found", "card was not found"))?;
        *card = prepared.updated.clone();

        next_cards.sort_by_key(|value| value.code);

        let mut operations = Vec::new();
        let old_code = prepared.existing.code;
        let new_code = prepared.updated.code;
        if old_code != new_code {
            for (from, to) in crate::domain::resource::path_rules::planned_asset_renames(
                &prepared.snapshot.pack_path,
                old_code,
                new_code,
            ) {
                operations.push(FsOperation::Rename { from, to });
            }
        }

        let mut next_metadata = prepared.snapshot.metadata.clone();
        next_metadata = crate::domain::pack::summary::touch_pack_metadata(&next_metadata, now_utc());
        operations.push(FsOperation::WriteFile {
            path: json_store::cards_path(&prepared.snapshot.pack_path),
            contents: encode_cards(&next_cards)?,
        });
        operations.push(FsOperation::WriteFile {
            path: json_store::pack_metadata_path(&prepared.snapshot.pack_path),
            contents: encode_pack_metadata(&next_metadata)?,
        });
        execute_plan(operations)?;

        let next_session = crate::application::pack::service::build_pack_session(
            prepared.snapshot.pack_path.clone(),
            next_metadata,
            next_cards,
            prepared.snapshot.strings.clone(),
            prepared.snapshot.revision + 1,
        )?;

        self.replace_and_refresh(&prepared.workspace_id, &prepared.pack_id, next_session)
    }

    pub fn delete_card(
        &self,
        workspace_id: &str,
        pack_id: &str,
        card_id: &str,
    ) -> AppResult<PackSession> {
        let snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            workspace_id,
            pack_id,
        )?;
        let now = now_utc();
        let mut next_cards = snapshot.cards.clone();
        let original_len = next_cards.len();
        next_cards.retain(|card| card.id != card_id);
        if next_cards.len() == original_len {
            return Err(AppError::new("card.not_found", "card was not found"));
        }

        let mut next_metadata = snapshot.metadata.clone();
        next_metadata = crate::domain::pack::summary::touch_pack_metadata(&next_metadata, now);

        execute_plan(vec![
            FsOperation::WriteFile {
                path: json_store::cards_path(&snapshot.pack_path),
                contents: encode_cards(&next_cards)?,
            },
            FsOperation::WriteFile {
                path: json_store::pack_metadata_path(&snapshot.pack_path),
                contents: encode_pack_metadata(&next_metadata)?,
            },
        ])?;

        let next_session = crate::application::pack::service::build_pack_session(
            snapshot.pack_path.clone(),
            next_metadata,
            next_cards,
            snapshot.strings.clone(),
            snapshot.revision + 1,
        )?;

        self.replace_and_refresh(workspace_id, pack_id, next_session)
    }

    fn replace_and_refresh(
        &self,
        workspace_id: &str,
        pack_id: &str,
        next_session: PackSession,
    ) -> AppResult<PackSession> {
        crate::application::pack::service::replace_open_pack_session(
            self.state,
            workspace_id,
            pack_id,
            next_session.clone(),
        )?;
        crate::application::pack::service::PackService::new(self.state)
            .refresh_current_workspace_summary()?;
        self.state
            .confirmation_cache
            .write()
            .map_err(|_| {
                AppError::new(
                    "confirmation.cache_lock_poisoned",
                    "confirmation cache lock poisoned",
                )
            })?
            .invalidate_pack(workspace_id, pack_id);
        Ok(next_session)
    }
}

fn encode_cards(cards: &[CardEntity]) -> AppResult<Vec<u8>> {
    serde_json::to_vec_pretty(&crate::domain::card::model::CardsFile {
        schema_version: crate::infrastructure::json_store::SCHEMA_VERSION,
        cards: cards.to_vec(),
    })
    .map_err(|source| AppError::new("json_store.serialize_failed", source.to_string()))
}

fn encode_pack_metadata(metadata: &crate::domain::pack::model::PackMetadata) -> AppResult<Vec<u8>> {
    serde_json::to_vec_pretty(&crate::domain::pack::model::PackMetadataFile {
        schema_version: crate::infrastructure::json_store::SCHEMA_VERSION,
        data: metadata.clone(),
    })
    .map_err(|source| AppError::new("json_store.serialize_failed", source.to_string()))
}
