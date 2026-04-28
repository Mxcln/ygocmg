use uuid::Uuid;

use crate::application::dto::card::{
    CardDetailDto, ConfirmCardWriteInput, CreateCardInput, GetCardInput, UpdateCardInput,
};
use crate::application::dto::common::WriteResultDto;
use crate::bootstrap::AppState;
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::ids::ConfirmationToken;
use crate::runtime::confirmation_cache::{
    CardConfirmationEntry, CardConfirmationInputSnapshot, CardConfirmationOperationKind,
};

pub struct CardWriteConfirmationService<'a> {
    state: &'a AppState,
}

impl<'a> CardWriteConfirmationService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub fn create_card(&self, input: CreateCardInput) -> AppResult<WriteResultDto<CardDetailDto>> {
        crate::application::pack::service::ensure_workspace_matches(
            self.state,
            &input.workspace_id,
        )?;
        let card_service = crate::application::card::service::CardService::new(self.state);
        let code_context = card_service.build_code_context(&input.pack_id, None)?;
        let write_service =
            crate::application::pack::write_service::PackWriteService::new(self.state);
        let prepared = write_service.prepare_create_card(
            &input.workspace_id,
            &input.pack_id,
            input.card.clone(),
            code_context,
        )?;

        if prepared.warnings.is_empty() {
            let card_id = write_service.commit_prepared_create_card(&prepared)?;
            let detail = card_service.get_card(GetCardInput {
                workspace_id: input.workspace_id,
                pack_id: input.pack_id,
                card_id,
            })?;
            return Ok(WriteResultDto::Ok {
                data: detail,
                warnings: prepared.warnings,
            });
        }

        let token: ConfirmationToken = Uuid::now_v7().to_string();
        let entry = CardConfirmationEntry {
            confirmation_token: token.clone(),
            workspace_id: input.workspace_id,
            pack_id: input.pack_id,
            pack_revision: prepared.snapshot.revision,
            source_stamp: prepared.snapshot.source_stamp.clone(),
            operation_kind: CardConfirmationOperationKind::CreateCard,
            input_snapshot: CardConfirmationInputSnapshot {
                card_id: None,
                card: prepared.normalized_input,
                create_card_seed: Some(prepared.card.clone()),
            },
            warnings: prepared.warnings.clone(),
        };
        self.state
            .confirmation_cache
            .write()
            .map_err(|_| {
                AppError::new(
                    "confirmation.cache_lock_poisoned",
                    "confirmation cache lock poisoned",
                )
            })?
            .insert_card_entry(entry);

        Ok(WriteResultDto::NeedsConfirmation {
            confirmation_token: token,
            warnings: prepared.warnings,
            preview: None,
        })
    }

    pub fn update_card(&self, input: UpdateCardInput) -> AppResult<WriteResultDto<CardDetailDto>> {
        crate::application::pack::service::ensure_workspace_matches(
            self.state,
            &input.workspace_id,
        )?;
        let card_service = crate::application::card::service::CardService::new(self.state);
        let code_context = card_service.build_code_context(&input.pack_id, Some(&input.card_id))?;
        let write_service =
            crate::application::pack::write_service::PackWriteService::new(self.state);
        let prepared = write_service.prepare_update_card(
            &input.workspace_id,
            &input.pack_id,
            &input.card_id,
            input.card.clone(),
            code_context,
        )?;

        if prepared.warnings.is_empty() {
            let card_id = write_service.commit_prepared_update_card(&prepared)?;
            let detail = card_service.get_card(GetCardInput {
                workspace_id: input.workspace_id,
                pack_id: input.pack_id,
                card_id,
            })?;
            return Ok(WriteResultDto::Ok {
                data: detail,
                warnings: prepared.warnings,
            });
        }

        let token: ConfirmationToken = Uuid::now_v7().to_string();
        let entry = CardConfirmationEntry {
            confirmation_token: token.clone(),
            workspace_id: input.workspace_id,
            pack_id: input.pack_id,
            pack_revision: prepared.snapshot.revision,
            source_stamp: prepared.snapshot.source_stamp.clone(),
            operation_kind: CardConfirmationOperationKind::UpdateCard,
            input_snapshot: CardConfirmationInputSnapshot {
                card_id: Some(input.card_id),
                card: prepared.normalized_input,
                create_card_seed: None,
            },
            warnings: prepared.warnings.clone(),
        };
        self.state
            .confirmation_cache
            .write()
            .map_err(|_| {
                AppError::new(
                    "confirmation.cache_lock_poisoned",
                    "confirmation cache lock poisoned",
                )
            })?
            .insert_card_entry(entry);

        Ok(WriteResultDto::NeedsConfirmation {
            confirmation_token: token,
            warnings: prepared.warnings,
            preview: None,
        })
    }

    pub fn confirm_card_write(&self, input: ConfirmCardWriteInput) -> AppResult<CardDetailDto> {
        let entry = {
            let mut cache = self.state.confirmation_cache.write().map_err(|_| {
                AppError::new(
                    "confirmation.cache_lock_poisoned",
                    "confirmation cache lock poisoned",
                )
            })?;
            cache
                .remove_card_entry(&input.confirmation_token)
                .ok_or_else(|| {
                    AppError::new(
                        "confirmation.invalid_token",
                        "confirmation token is missing or already consumed",
                    )
                    .with_detail("confirmation_token", input.confirmation_token.clone())
                })?
        };
        let CardConfirmationEntry {
            workspace_id,
            pack_id,
            pack_revision,
            source_stamp,
            operation_kind,
            input_snapshot,
            ..
        } = entry;

        let current_snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            &workspace_id,
            &pack_id,
        )?;
        if current_snapshot.revision != pack_revision {
            return Err(AppError::new(
                "confirmation.stale_revision",
                "confirmation token no longer matches the pack revision",
            )
            .with_detail("expected_revision", pack_revision)
            .with_detail("actual_revision", current_snapshot.revision));
        }
        if current_snapshot.source_stamp != source_stamp {
            return Err(AppError::new(
                "confirmation.stale_source_stamp",
                "confirmation token no longer matches current disk state",
            )
            .with_detail("expected_source_stamp", source_stamp)
            .with_detail("actual_source_stamp", current_snapshot.source_stamp));
        }

        let card_service = crate::application::card::service::CardService::new(self.state);
        let write_service =
            crate::application::pack::write_service::PackWriteService::new(self.state);

        let detail = match operation_kind {
            CardConfirmationOperationKind::CreateCard => {
                let seeded_card = input_snapshot.create_card_seed.ok_or_else(|| {
                    AppError::new(
                        "confirmation.invalid_entry",
                        "create confirmation entry is missing staged card seed",
                    )
                })?;
                let code_context = card_service.build_code_context(&pack_id, None)?;
                let prepared = write_service.prepare_create_card_with_seed(
                    &workspace_id,
                    &pack_id,
                    input_snapshot.card,
                    code_context,
                    Some(seeded_card),
                )?;
                let card_id = write_service.commit_prepared_create_card(&prepared)?;
                card_service.get_card(GetCardInput {
                    workspace_id,
                    pack_id,
                    card_id,
                })?
            }
            CardConfirmationOperationKind::UpdateCard => {
                let card_id = input_snapshot.card_id.clone().ok_or_else(|| {
                    AppError::new(
                        "confirmation.invalid_entry",
                        "update confirmation entry is missing card id",
                    )
                })?;
                let code_context = card_service.build_code_context(&pack_id, Some(&card_id))?;
                let prepared = write_service.prepare_update_card(
                    &workspace_id,
                    &pack_id,
                    &card_id,
                    input_snapshot.card,
                    code_context,
                )?;
                let committed_card_id = write_service.commit_prepared_update_card(&prepared)?;
                card_service.get_card(GetCardInput {
                    workspace_id,
                    pack_id,
                    card_id: committed_card_id,
                })?
            }
        };

        Ok(detail)
    }
}
