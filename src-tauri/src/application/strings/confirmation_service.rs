use uuid::Uuid;

use crate::application::dto::common::WriteResultDto;
use crate::application::dto::strings::{
    ConfirmPackStringRecordWriteInput, ConfirmPackStringsWriteInput, GetPackStringInput,
    ListPackStringsInput, PackStringRecordDetailDto, PackStringsPageDto, UpsertPackStringInput,
    UpsertPackStringRecordInput,
};
use crate::bootstrap::AppState;
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::ids::ConfirmationToken;
use crate::domain::common::issue::ValidationIssue;
use crate::runtime::confirmation_cache::PackStringRecordConfirmationEntry;
use crate::runtime::confirmation_cache::{
    PackStringsConfirmationEntry, PackStringsConfirmationInputSnapshot,
    PackStringsConfirmationOperationKind,
};

pub struct PackStringsConfirmationService<'a> {
    state: &'a AppState,
}

impl<'a> PackStringsConfirmationService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub fn upsert_pack_string(
        &self,
        input: UpsertPackStringInput,
    ) -> AppResult<WriteResultDto<PackStringsPageDto>> {
        crate::application::pack::service::ensure_workspace_matches(
            self.state,
            &input.workspace_id,
        )?;
        let write_service =
            crate::application::pack::write_service::PackWriteService::new(self.state);
        let prepared = write_service.prepare_upsert_pack_string(
            &input.workspace_id,
            &input.pack_id,
            &input.language,
            input.entry.clone().into(),
        )?;

        if prepared.warnings.is_empty() {
            let next_session = write_service.commit_prepared_upsert_pack_string(&prepared)?;
            let page = crate::application::strings::service::PackStringsService::new(self.state)
                .list_pack_strings(ListPackStringsInput {
                    workspace_id: input.workspace_id,
                    pack_id: input.pack_id,
                    language: input.language,
                    kind_filter: None,
                    key_filter: None,
                    keyword: None,
                    page: 1,
                    page_size: next_session
                        .strings
                        .language_entry_count(&prepared.language)
                        .max(1) as u32,
                })?;
            return Ok(WriteResultDto::Ok {
                data: page,
                warnings: prepared.warnings,
            });
        }

        let token: ConfirmationToken = Uuid::now_v7().to_string();
        self.state
            .confirmation_cache
            .write()
            .map_err(|_| {
                AppError::new(
                    "confirmation.cache_lock_poisoned",
                    "confirmation cache lock poisoned",
                )
            })?
            .insert_pack_strings_entry(PackStringsConfirmationEntry {
                confirmation_token: token.clone(),
                workspace_id: input.workspace_id,
                pack_id: input.pack_id,
                pack_revision: prepared.snapshot.revision,
                source_stamp: prepared.snapshot.source_stamp.clone(),
                operation_kind: PackStringsConfirmationOperationKind::UpsertPackString,
                input_snapshot: PackStringsConfirmationInputSnapshot {
                    language: prepared.language.clone(),
                    entry: prepared.entry.clone(),
                },
                warnings: prepared.warnings.clone(),
            });

        Ok(WriteResultDto::NeedsConfirmation {
            confirmation_token: token,
            warnings: prepared.warnings,
            preview: None,
        })
    }

    pub fn confirm_pack_strings_write(
        &self,
        input: ConfirmPackStringsWriteInput,
    ) -> AppResult<PackStringsPageDto> {
        let entry = {
            let mut cache = self.state.confirmation_cache.write().map_err(|_| {
                AppError::new(
                    "confirmation.cache_lock_poisoned",
                    "confirmation cache lock poisoned",
                )
            })?;
            cache
                .remove_pack_strings_entry(&input.confirmation_token)
                .ok_or_else(|| {
                    AppError::new(
                        "confirmation.invalid_token",
                        "confirmation token is missing or already consumed",
                    )
                    .with_detail("confirmation_token", input.confirmation_token.clone())
                })?
        };

        let current_snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            &entry.workspace_id,
            &entry.pack_id,
        )?;
        if current_snapshot.revision != entry.pack_revision {
            return Err(AppError::new(
                "confirmation.stale_revision",
                "confirmation token no longer matches the pack revision",
            )
            .with_detail("expected_revision", entry.pack_revision)
            .with_detail("actual_revision", current_snapshot.revision));
        }
        if current_snapshot.source_stamp != entry.source_stamp {
            return Err(AppError::new(
                "confirmation.stale_source_stamp",
                "confirmation token no longer matches current disk state",
            )
            .with_detail("expected_source_stamp", entry.source_stamp)
            .with_detail("actual_source_stamp", current_snapshot.source_stamp));
        }

        let write_service =
            crate::application::pack::write_service::PackWriteService::new(self.state);
        let prepared = write_service.prepare_upsert_pack_string(
            &entry.workspace_id,
            &entry.pack_id,
            &entry.input_snapshot.language,
            entry.input_snapshot.entry,
        )?;
        let next_session = write_service.commit_prepared_upsert_pack_string(&prepared)?;
        let language = prepared.language.clone();
        crate::application::strings::service::PackStringsService::new(self.state).list_pack_strings(
            ListPackStringsInput {
                workspace_id: entry.workspace_id,
                pack_id: entry.pack_id,
                language: language.clone(),
                kind_filter: None,
                key_filter: None,
                keyword: None,
                page: 1,
                page_size: next_session.strings.language_entry_count(&language).max(1) as u32,
            },
        )
    }

    pub fn upsert_pack_string_record(
        &self,
        input: UpsertPackStringRecordInput,
    ) -> AppResult<WriteResultDto<PackStringRecordDetailDto>> {
        crate::application::pack::service::ensure_workspace_matches(
            self.state,
            &input.workspace_id,
        )?;
        let write_service =
            crate::application::pack::write_service::PackWriteService::new(self.state);
        let prepared = write_service.prepare_upsert_pack_string_record(
            &input.workspace_id,
            &input.pack_id,
            input.record.clone().into(),
        )?;

        if prepared.warnings.is_empty() {
            write_service.commit_prepared_upsert_pack_string_record(&prepared)?;
            let detail = crate::application::strings::service::PackStringsService::new(self.state)
                .get_pack_string(GetPackStringInput {
                    workspace_id: input.workspace_id,
                    pack_id: input.pack_id,
                    kind: prepared.record.kind.clone(),
                    key: prepared.record.key,
                })?;
            return Ok(WriteResultDto::Ok {
                data: detail,
                warnings: prepared.warnings,
            });
        }

        let token: ConfirmationToken = Uuid::now_v7().to_string();
        self.state
            .confirmation_cache
            .write()
            .map_err(|_| {
                AppError::new(
                    "confirmation.cache_lock_poisoned",
                    "confirmation cache lock poisoned",
                )
            })?
            .insert_pack_string_record_entry(PackStringRecordConfirmationEntry {
                confirmation_token: token.clone(),
                workspace_id: input.workspace_id,
                pack_id: input.pack_id,
                pack_revision: prepared.snapshot.revision,
                source_stamp: prepared.snapshot.source_stamp.clone(),
                input_snapshot:
                    crate::runtime::confirmation_cache::PackStringRecordConfirmationInputSnapshot {
                        record: prepared.record.clone(),
                    },
                warnings: prepared.warnings.clone(),
            });

        Ok(WriteResultDto::NeedsConfirmation {
            confirmation_token: token,
            warnings: prepared.warnings,
            preview: None,
        })
    }

    pub fn confirm_pack_string_record_write(
        &self,
        input: ConfirmPackStringRecordWriteInput,
    ) -> AppResult<PackStringRecordDetailDto> {
        let entry = {
            let mut cache = self.state.confirmation_cache.write().map_err(|_| {
                AppError::new(
                    "confirmation.cache_lock_poisoned",
                    "confirmation cache lock poisoned",
                )
            })?;
            cache
                .remove_pack_string_record_entry(&input.confirmation_token)
                .ok_or_else(|| {
                    AppError::new(
                        "confirmation.invalid_token",
                        "confirmation token is missing or already consumed",
                    )
                    .with_detail("confirmation_token", input.confirmation_token.clone())
                })?
        };

        let current_snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            &entry.workspace_id,
            &entry.pack_id,
        )?;
        if current_snapshot.revision != entry.pack_revision {
            return Err(AppError::new(
                "confirmation.stale_revision",
                "confirmation token no longer matches the pack revision",
            )
            .with_detail("expected_revision", entry.pack_revision)
            .with_detail("actual_revision", current_snapshot.revision));
        }
        if current_snapshot.source_stamp != entry.source_stamp {
            return Err(AppError::new(
                "confirmation.stale_source_stamp",
                "confirmation token no longer matches current disk state",
            )
            .with_detail("expected_source_stamp", entry.source_stamp)
            .with_detail("actual_source_stamp", current_snapshot.source_stamp));
        }

        let write_service =
            crate::application::pack::write_service::PackWriteService::new(self.state);
        let prepared = write_service.prepare_upsert_pack_string_record(
            &entry.workspace_id,
            &entry.pack_id,
            entry.input_snapshot.record,
        )?;
        write_service.commit_prepared_upsert_pack_string_record(&prepared)?;
        crate::application::strings::service::PackStringsService::new(self.state).get_pack_string(
            GetPackStringInput {
                workspace_id: entry.workspace_id,
                pack_id: entry.pack_id,
                kind: prepared.record.kind,
                key: prepared.record.key,
            },
        )
    }
}

pub fn overwrite_warning(
    language: &str,
    entry: &crate::domain::strings::model::PackStringEntry,
) -> ValidationIssue {
    crate::domain::common::issue::ValidationIssue::warning(
        "pack_strings.overwrite_existing_value",
        crate::domain::common::issue::ValidationTarget::new("pack_strings")
            .with_entity_id(language.to_string())
            .with_field("entry"),
    )
    .with_param("language", language)
    .with_param("kind", &entry.kind)
    .with_param("key", entry.key)
}

pub fn overwrite_record_warning(
    previous: &crate::domain::strings::model::PackStringRecord,
    next: &crate::domain::strings::model::PackStringRecord,
) -> ValidationIssue {
    crate::domain::common::issue::ValidationIssue::warning(
        "pack_strings.overwrite_existing_record",
        crate::domain::common::issue::ValidationTarget::new("pack_strings").with_field("record"),
    )
    .with_param("kind", &next.kind)
    .with_param("key", next.key)
    .with_param(
        "previous_languages",
        previous.values.keys().collect::<Vec<_>>(),
    )
    .with_param("next_languages", next.values.keys().collect::<Vec<_>>())
}
