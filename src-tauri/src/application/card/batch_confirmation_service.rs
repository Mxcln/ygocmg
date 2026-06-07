use uuid::Uuid;

use crate::application::dto::card::{
    BulkDeleteCardsInput, BulkDeleteCardsResultDto, CardBatchWriteResultDto,
    ConfirmCardBatchWriteInput, MoveCardsInput, MoveCardsResultDto,
};
use crate::application::dto::common::WriteResultDto;
use crate::bootstrap::AppState;
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::ids::ConfirmationToken;
use crate::runtime::confirmation_cache::{
    CardBatchConfirmationEntry, CardBatchConfirmationInputSnapshot,
    CardBatchConfirmationOperationKind, PackConfirmationStamp,
};

pub struct CardBatchWriteConfirmationService<'a> {
    state: &'a AppState,
}

impl<'a> CardBatchWriteConfirmationService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub fn bulk_delete_cards(
        &self,
        input: BulkDeleteCardsInput,
    ) -> AppResult<WriteResultDto<BulkDeleteCardsResultDto>> {
        crate::application::pack::service::ensure_workspace_matches(
            self.state,
            &input.workspace_id,
        )?;
        let write_service =
            crate::application::pack::write_service::PackWriteService::new(self.state);
        let prepared = write_service.prepare_bulk_delete_cards(
            &input.workspace_id,
            &input.pack_id,
            input.card_ids.clone(),
            input.delete_assets,
        )?;

        if prepared.warnings.is_empty() {
            let data = write_service.commit_prepared_bulk_delete_cards(&prepared)?;
            return Ok(WriteResultDto::Ok {
                data,
                warnings: prepared.warnings,
            });
        }

        let token = self.insert_entry(CardBatchConfirmationEntry {
            confirmation_token: Uuid::now_v7().to_string(),
            workspace_id: input.workspace_id,
            operation_kind: CardBatchConfirmationOperationKind::BulkDeleteCards,
            input_snapshot: CardBatchConfirmationInputSnapshot::BulkDelete {
                pack_id: input.pack_id,
                card_ids: input.card_ids,
                delete_assets: input.delete_assets,
            },
            pack_stamps: vec![pack_stamp(&prepared.snapshot)],
            warnings: prepared.warnings.clone(),
        })?;

        Ok(WriteResultDto::NeedsConfirmation {
            confirmation_token: token,
            warnings: prepared.warnings,
            preview: None,
        })
    }

    pub fn move_cards(
        &self,
        input: MoveCardsInput,
    ) -> AppResult<WriteResultDto<MoveCardsResultDto>> {
        crate::application::pack::service::ensure_workspace_matches(
            self.state,
            &input.workspace_id,
        )?;
        let write_service =
            crate::application::pack::write_service::PackWriteService::new(self.state);
        let prepared = write_service.prepare_move_cards(
            &input.workspace_id,
            &input.source_pack_id,
            &input.target_pack_id,
            input.card_ids.clone(),
            input.move_assets,
        )?;

        if prepared.warnings.is_empty() {
            let data = write_service.commit_prepared_move_cards(&prepared)?;
            return Ok(WriteResultDto::Ok {
                data,
                warnings: prepared.warnings,
            });
        }

        let token = self.insert_entry(CardBatchConfirmationEntry {
            confirmation_token: Uuid::now_v7().to_string(),
            workspace_id: input.workspace_id,
            operation_kind: CardBatchConfirmationOperationKind::MoveCards,
            input_snapshot: CardBatchConfirmationInputSnapshot::Move {
                source_pack_id: input.source_pack_id,
                target_pack_id: input.target_pack_id,
                card_ids: input.card_ids,
                move_assets: input.move_assets,
            },
            pack_stamps: vec![
                pack_stamp(&prepared.source_snapshot),
                pack_stamp(&prepared.target_snapshot),
            ],
            warnings: prepared.warnings.clone(),
        })?;

        Ok(WriteResultDto::NeedsConfirmation {
            confirmation_token: token,
            warnings: prepared.warnings,
            preview: None,
        })
    }

    pub fn confirm_card_batch_write(
        &self,
        input: ConfirmCardBatchWriteInput,
    ) -> AppResult<CardBatchWriteResultDto> {
        let entry = {
            let mut cache = self.state.confirmation_cache.write().map_err(|_| {
                AppError::new(
                    "confirmation.cache_lock_poisoned",
                    "confirmation cache lock poisoned",
                )
            })?;
            cache
                .remove_card_batch_entry(&input.confirmation_token)
                .ok_or_else(|| {
                    AppError::new(
                        "confirmation.invalid_token",
                        "confirmation token is missing or already consumed",
                    )
                    .with_detail("confirmation_token", input.confirmation_token.clone())
                })?
        };

        for stamp in &entry.pack_stamps {
            let current_snapshot = crate::application::pack::service::require_open_pack_snapshot(
                self.state,
                &entry.workspace_id,
                &stamp.pack_id,
            )?;
            if current_snapshot.revision != stamp.pack_revision {
                return Err(AppError::new(
                    "confirmation.stale_revision",
                    "confirmation token no longer matches the pack revision",
                )
                .with_detail("pack_id", &stamp.pack_id)
                .with_detail("expected_revision", stamp.pack_revision)
                .with_detail("actual_revision", current_snapshot.revision));
            }
            if current_snapshot.source_stamp != stamp.source_stamp {
                return Err(AppError::new(
                    "confirmation.stale_source_stamp",
                    "confirmation token no longer matches current disk state",
                )
                .with_detail("pack_id", &stamp.pack_id)
                .with_detail("expected_source_stamp", &stamp.source_stamp)
                .with_detail("actual_source_stamp", current_snapshot.source_stamp));
            }
        }

        let write_service =
            crate::application::pack::write_service::PackWriteService::new(self.state);
        match entry.input_snapshot {
            CardBatchConfirmationInputSnapshot::BulkDelete {
                pack_id,
                card_ids,
                delete_assets,
            } => {
                match entry.operation_kind {
                    CardBatchConfirmationOperationKind::BulkDeleteCards => {}
                    _ => {
                        return Err(AppError::new(
                            "confirmation.invalid_entry",
                            "confirmation operation does not match input snapshot",
                        ));
                    }
                }
                let prepared = write_service.prepare_bulk_delete_cards(
                    &entry.workspace_id,
                    &pack_id,
                    card_ids,
                    delete_assets,
                )?;
                let data = write_service.commit_prepared_bulk_delete_cards(&prepared)?;
                Ok(CardBatchWriteResultDto::BulkDelete { data })
            }
            CardBatchConfirmationInputSnapshot::Move {
                source_pack_id,
                target_pack_id,
                card_ids,
                move_assets,
            } => {
                match entry.operation_kind {
                    CardBatchConfirmationOperationKind::MoveCards => {}
                    _ => {
                        return Err(AppError::new(
                            "confirmation.invalid_entry",
                            "confirmation operation does not match input snapshot",
                        ));
                    }
                }
                let prepared = write_service.prepare_move_cards(
                    &entry.workspace_id,
                    &source_pack_id,
                    &target_pack_id,
                    card_ids,
                    move_assets,
                )?;
                let data = write_service.commit_prepared_move_cards(&prepared)?;
                Ok(CardBatchWriteResultDto::Move { data })
            }
        }
    }

    fn insert_entry(&self, entry: CardBatchConfirmationEntry) -> AppResult<ConfirmationToken> {
        let token = entry.confirmation_token.clone();
        self.state
            .confirmation_cache
            .write()
            .map_err(|_| {
                AppError::new(
                    "confirmation.cache_lock_poisoned",
                    "confirmation cache lock poisoned",
                )
            })?
            .insert_card_batch_entry(entry);
        Ok(token)
    }
}

fn pack_stamp(session: &crate::runtime::sessions::PackSession) -> PackConfirmationStamp {
    PackConfirmationStamp {
        pack_id: session.pack_id.clone(),
        pack_revision: session.revision,
        source_stamp: session.source_stamp.clone(),
    }
}
