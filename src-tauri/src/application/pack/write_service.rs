use std::collections::BTreeSet;
use std::path::Path;

use uuid::Uuid;

use crate::application::dto::strings::PackStringKeyDto;
use crate::bootstrap::AppState;
use crate::domain::card::code::{CodeValidationContext, validate_card_code};
use crate::domain::card::model::{CardEntity, CardUpdateInput, PrimaryType, SpellSubtype};
use crate::domain::card::normalize::{apply_card_update, create_card_entity, normalize_card_input};
use crate::domain::card::validate::{
    collect_card_warnings, validate_card_structure, validate_card_update_input,
};
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::ids::{CardId, PackId, WorkspaceId};
use crate::domain::common::issue::{IssueLevel, ValidationIssue};
use crate::domain::common::time::now_utc;
use crate::domain::language::rules::{normalize_language_id, validate_catalog_membership};
use crate::domain::namespace::model::{
    PackStringsNamespaceContext, build_pack_strings_namespace_index,
};
use crate::domain::namespace::validate::validate_pack_string_record_namespace;
use crate::domain::resource::model::CardAssetState;
use crate::domain::resource::path_rules::{card_image_path, field_image_path, script_path};
use crate::domain::strings::model::{
    PackStringEntry, PackStringRecord, PackStringsFile, RemovePackStringTranslationOutcome,
    UpsertPackStringRecordOutcome, UpsertPackStringTranslationOutcome,
};
use crate::domain::strings::validate::validate_pack_strings;
use crate::infrastructure::fs::transaction::{FsOperation, execute_plan};
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

#[derive(Debug, Clone)]
pub struct PreparedUpsertPackStringWrite {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub snapshot: PackSession,
    pub language: String,
    pub entry: PackStringEntry,
    pub next_strings: PackStringsFile,
    pub warnings: Vec<ValidationIssue>,
}

#[derive(Debug, Clone)]
pub struct PreparedUpsertPackStringRecordWrite {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub snapshot: PackSession,
    pub record: PackStringRecord,
    pub next_strings: PackStringsFile,
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
        let empty_existing_languages = BTreeSet::new();
        validate_card_languages_or_err(self.state, &normalized, &empty_existing_languages)?;
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
            create_card_entity(
                existing_seed.id,
                normalized.clone(),
                existing_seed.created_at,
            )
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
        let existing_languages = existing.texts.keys().cloned().collect::<BTreeSet<_>>();
        validate_card_languages_or_err(self.state, &normalized, &existing_languages)?;

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
        next_metadata =
            crate::domain::pack::summary::touch_pack_metadata(&next_metadata, now_utc());
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

    pub fn prepare_upsert_pack_string(
        &self,
        workspace_id: &str,
        pack_id: &str,
        language: &str,
        entry: PackStringEntry,
    ) -> AppResult<PreparedUpsertPackStringWrite> {
        reject_authoring_system_string(&entry.kind, entry.key)?;
        let snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            workspace_id,
            pack_id,
        )?;
        let normalized_language = normalize_language_id(language);
        let existing_languages = pack_existing_languages(&snapshot);
        validate_language_or_err(
            self.state,
            &normalized_language,
            &existing_languages,
            "pack_strings",
            "language",
            "pack_strings.language",
        )?;
        let mut next_strings = snapshot.strings.clone();

        let mut warnings = Vec::new();
        match next_strings.upsert_translation(&normalized_language, &entry) {
            UpsertPackStringTranslationOutcome::NoChange => {
                return Ok(PreparedUpsertPackStringWrite {
                    workspace_id: workspace_id.to_string(),
                    pack_id: pack_id.to_string(),
                    snapshot,
                    language: normalized_language,
                    entry,
                    next_strings,
                    warnings,
                });
            }
            UpsertPackStringTranslationOutcome::Updated { .. } => {
                warnings.push(
                    crate::application::strings::confirmation_service::overwrite_warning(
                        &normalized_language,
                        &entry,
                    ),
                );
            }
            UpsertPackStringTranslationOutcome::Inserted => {}
        }

        if let Some(record) = next_strings.get_record(&entry.kind, entry.key) {
            warnings.extend(self.pack_string_namespace_warnings(&snapshot, record, None));
        }
        validate_pack_strings_or_err(&next_strings)?;

        Ok(PreparedUpsertPackStringWrite {
            workspace_id: workspace_id.to_string(),
            pack_id: pack_id.to_string(),
            snapshot,
            language: normalized_language,
            entry,
            next_strings,
            warnings,
        })
    }

    pub fn prepare_upsert_pack_string_record(
        &self,
        workspace_id: &str,
        pack_id: &str,
        record: PackStringRecord,
    ) -> AppResult<PreparedUpsertPackStringRecordWrite> {
        reject_authoring_system_string(&record.kind, record.key)?;
        let snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            workspace_id,
            pack_id,
        )?;
        let existing_languages = pack_existing_languages(&snapshot);
        for language in record.values.keys() {
            validate_language_or_err(
                self.state,
                language,
                &existing_languages,
                "pack_strings",
                "values",
                "pack_strings.language",
            )?;
        }
        let mut next_strings = snapshot.strings.clone();
        let previous = snapshot
            .strings
            .get_record(&record.kind, record.key)
            .cloned();

        let mut warnings = Vec::new();
        match next_strings.upsert_record(record.clone()) {
            UpsertPackStringRecordOutcome::NoChange => {
                return Ok(PreparedUpsertPackStringRecordWrite {
                    workspace_id: workspace_id.to_string(),
                    pack_id: pack_id.to_string(),
                    snapshot,
                    record,
                    next_strings,
                    warnings,
                });
            }
            UpsertPackStringRecordOutcome::Inserted => {}
            UpsertPackStringRecordOutcome::Replaced { previous } => {
                warnings.push(
                    crate::application::strings::confirmation_service::overwrite_record_warning(
                        &previous, &record,
                    ),
                );
            }
        }

        warnings.extend(self.pack_string_namespace_warnings(&snapshot, &record, previous.as_ref()));
        validate_pack_strings_or_err(&next_strings)?;

        Ok(PreparedUpsertPackStringRecordWrite {
            workspace_id: workspace_id.to_string(),
            pack_id: pack_id.to_string(),
            snapshot,
            record,
            next_strings,
            warnings,
        })
    }

    pub fn commit_prepared_upsert_pack_string(
        &self,
        prepared: &PreparedUpsertPackStringWrite,
    ) -> AppResult<PackSession> {
        self.commit_pack_strings(
            &prepared.workspace_id,
            &prepared.pack_id,
            &prepared.snapshot,
            prepared.next_strings.clone(),
        )
    }

    pub fn commit_prepared_upsert_pack_string_record(
        &self,
        prepared: &PreparedUpsertPackStringRecordWrite,
    ) -> AppResult<PackSession> {
        self.commit_pack_strings(
            &prepared.workspace_id,
            &prepared.pack_id,
            &prepared.snapshot,
            prepared.next_strings.clone(),
        )
    }

    pub fn delete_pack_strings(
        &self,
        workspace_id: &str,
        pack_id: &str,
        keys: &[PackStringKeyDto],
    ) -> AppResult<(PackSession, usize)> {
        let snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            workspace_id,
            pack_id,
        )?;
        let key_set = keys
            .iter()
            .map(|item| (item.kind.clone(), item.key))
            .collect::<BTreeSet<_>>();
        let mut next_strings = snapshot.strings.clone();
        let deleted_count = next_strings.delete_records(&key_set.into_iter().collect::<Vec<_>>());
        if deleted_count == 0 {
            return Ok((snapshot, 0));
        }

        validate_pack_strings_or_err(&next_strings)?;
        let next_metadata =
            crate::domain::pack::summary::touch_pack_metadata(&snapshot.metadata, now_utc());
        execute_plan(vec![
            FsOperation::WriteFile {
                path: json_store::pack_strings_path(&snapshot.pack_path),
                contents: encode_pack_strings(&next_strings)?,
            },
            FsOperation::WriteFile {
                path: json_store::pack_metadata_path(&snapshot.pack_path),
                contents: encode_pack_metadata(&next_metadata)?,
            },
        ])?;

        let next_session = crate::application::pack::service::build_pack_session(
            snapshot.pack_path.clone(),
            next_metadata,
            snapshot.cards.clone(),
            next_strings,
            snapshot.revision + 1,
        )?;
        Ok((
            self.replace_and_refresh(workspace_id, pack_id, next_session)?,
            deleted_count,
        ))
    }

    pub fn remove_pack_string_translation(
        &self,
        workspace_id: &str,
        pack_id: &str,
        kind: &crate::domain::strings::model::PackStringKind,
        key: u32,
        language: &str,
    ) -> AppResult<(PackSession, bool)> {
        let snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            workspace_id,
            pack_id,
        )?;
        let mut next_strings = snapshot.strings.clone();
        let changed = match next_strings.remove_translation(kind, key, language) {
            RemovePackStringTranslationOutcome::NoChange => false,
            RemovePackStringTranslationOutcome::Updated(_) => true,
            RemovePackStringTranslationOutcome::DeletedRecord => true,
        };
        if !changed {
            return Ok((snapshot, false));
        }

        validate_pack_strings_or_err(&next_strings)?;
        let next_metadata =
            crate::domain::pack::summary::touch_pack_metadata(&snapshot.metadata, now_utc());
        execute_plan(vec![
            FsOperation::WriteFile {
                path: json_store::pack_strings_path(&snapshot.pack_path),
                contents: encode_pack_strings(&next_strings)?,
            },
            FsOperation::WriteFile {
                path: json_store::pack_metadata_path(&snapshot.pack_path),
                contents: encode_pack_metadata(&next_metadata)?,
            },
        ])?;

        let next_session = crate::application::pack::service::build_pack_session(
            snapshot.pack_path.clone(),
            next_metadata,
            snapshot.cards.clone(),
            next_strings,
            snapshot.revision + 1,
        )?;
        Ok((
            self.replace_and_refresh(workspace_id, pack_id, next_session)?,
            true,
        ))
    }

    pub fn import_main_image(
        &self,
        workspace_id: &str,
        pack_id: &str,
        card_id: &str,
        source_path: &Path,
    ) -> AppResult<CardAssetState> {
        let snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            workspace_id,
            pack_id,
        )?;
        let card = require_card(&snapshot, card_id)?;
        let target_path = card_image_path(&snapshot.pack_path, card.code);
        let contents = crate::infrastructure::assets::import_main_image(source_path)?;
        self.apply_asset_operation(
            workspace_id,
            pack_id,
            snapshot,
            card_id,
            vec![FsOperation::WriteFile {
                path: target_path,
                contents,
            }],
        )
    }

    pub fn delete_main_image(
        &self,
        workspace_id: &str,
        pack_id: &str,
        card_id: &str,
    ) -> AppResult<CardAssetState> {
        let snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            workspace_id,
            pack_id,
        )?;
        let card = require_card(&snapshot, card_id)?;
        let target_path = card_image_path(&snapshot.pack_path, card.code);
        self.apply_asset_delete(workspace_id, pack_id, snapshot, card_id, target_path)
    }

    pub fn import_field_image(
        &self,
        workspace_id: &str,
        pack_id: &str,
        card_id: &str,
        source_path: &Path,
    ) -> AppResult<CardAssetState> {
        let snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            workspace_id,
            pack_id,
        )?;
        let card = require_card(&snapshot, card_id)?;
        require_field_spell(card)?;
        let target_path = field_image_path(&snapshot.pack_path, card.code);
        let contents = crate::infrastructure::assets::import_field_image(source_path)?;
        self.apply_asset_operation(
            workspace_id,
            pack_id,
            snapshot,
            card_id,
            vec![FsOperation::WriteFile {
                path: target_path,
                contents,
            }],
        )
    }

    pub fn delete_field_image(
        &self,
        workspace_id: &str,
        pack_id: &str,
        card_id: &str,
    ) -> AppResult<CardAssetState> {
        let snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            workspace_id,
            pack_id,
        )?;
        let card = require_card(&snapshot, card_id)?;
        let target_path = field_image_path(&snapshot.pack_path, card.code);
        self.apply_asset_delete(workspace_id, pack_id, snapshot, card_id, target_path)
    }

    pub fn create_empty_script(
        &self,
        workspace_id: &str,
        pack_id: &str,
        card_id: &str,
    ) -> AppResult<CardAssetState> {
        let snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            workspace_id,
            pack_id,
        )?;
        let card = require_card(&snapshot, card_id)?;
        let target_path = script_path(&snapshot.pack_path, card.code);
        if target_path.exists() {
            return Err(
                AppError::new("resource.script_exists", "script file already exists")
                    .with_detail("path", target_path.display().to_string()),
            );
        }
        self.apply_asset_operation(
            workspace_id,
            pack_id,
            snapshot,
            card_id,
            vec![FsOperation::WriteFile {
                path: target_path,
                contents: Vec::new(),
            }],
        )
    }

    pub fn import_script(
        &self,
        workspace_id: &str,
        pack_id: &str,
        card_id: &str,
        source_path: &Path,
    ) -> AppResult<CardAssetState> {
        let snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            workspace_id,
            pack_id,
        )?;
        let card = require_card(&snapshot, card_id)?;
        let contents = std::fs::read(source_path).map_err(|source| {
            AppError::from_io("resource.script_read_failed", source)
                .with_detail("path", source_path.display().to_string())
        })?;
        let target_path = script_path(&snapshot.pack_path, card.code);
        self.apply_asset_operation(
            workspace_id,
            pack_id,
            snapshot,
            card_id,
            vec![FsOperation::WriteFile {
                path: target_path,
                contents,
            }],
        )
    }

    pub fn delete_script(
        &self,
        workspace_id: &str,
        pack_id: &str,
        card_id: &str,
    ) -> AppResult<CardAssetState> {
        let snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            workspace_id,
            pack_id,
        )?;
        let card = require_card(&snapshot, card_id)?;
        let target_path = script_path(&snapshot.pack_path, card.code);
        self.apply_asset_delete(workspace_id, pack_id, snapshot, card_id, target_path)
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

    fn apply_asset_operation(
        &self,
        workspace_id: &str,
        pack_id: &str,
        snapshot: PackSession,
        card_id: &str,
        mut operations: Vec<FsOperation>,
    ) -> AppResult<CardAssetState> {
        let next_metadata =
            crate::domain::pack::summary::touch_pack_metadata(&snapshot.metadata, now_utc());
        operations.push(FsOperation::WriteFile {
            path: json_store::pack_metadata_path(&snapshot.pack_path),
            contents: encode_pack_metadata(&next_metadata)?,
        });
        execute_plan(operations)?;
        let next_session = crate::application::pack::service::build_pack_session(
            snapshot.pack_path.clone(),
            next_metadata,
            snapshot.cards.clone(),
            snapshot.strings.clone(),
            snapshot.revision + 1,
        )?;
        let next_session = self.replace_and_refresh(workspace_id, pack_id, next_session)?;
        next_session
            .asset_index
            .get(card_id)
            .cloned()
            .ok_or_else(|| AppError::new("card.not_found", "card was not found"))
    }

    fn apply_asset_delete(
        &self,
        workspace_id: &str,
        pack_id: &str,
        snapshot: PackSession,
        card_id: &str,
        target_path: std::path::PathBuf,
    ) -> AppResult<CardAssetState> {
        self.apply_asset_operation(
            workspace_id,
            pack_id,
            snapshot,
            card_id,
            vec![FsOperation::DeleteFile { path: target_path }],
        )
    }

    fn commit_pack_strings(
        &self,
        workspace_id: &str,
        pack_id: &str,
        snapshot: &PackSession,
        next_strings: PackStringsFile,
    ) -> AppResult<PackSession> {
        if snapshot.strings == next_strings {
            return Ok(snapshot.clone());
        }
        let next_metadata =
            crate::domain::pack::summary::touch_pack_metadata(&snapshot.metadata, now_utc());

        execute_plan(vec![
            FsOperation::WriteFile {
                path: json_store::pack_strings_path(&snapshot.pack_path),
                contents: encode_pack_strings(&next_strings)?,
            },
            FsOperation::WriteFile {
                path: json_store::pack_metadata_path(&snapshot.pack_path),
                contents: encode_pack_metadata(&next_metadata)?,
            },
        ])?;

        let next_session = crate::application::pack::service::build_pack_session(
            snapshot.pack_path.clone(),
            next_metadata,
            snapshot.cards.clone(),
            next_strings,
            snapshot.revision + 1,
        )?;
        self.replace_and_refresh(workspace_id, pack_id, next_session)
    }

    fn pack_string_namespace_warnings(
        &self,
        snapshot: &PackSession,
        record: &PackStringRecord,
        previous: Option<&PackStringRecord>,
    ) -> Vec<ValidationIssue> {
        let workspace_index = crate::application::card::service::CardService::new(self.state)
            .build_workspace_namespace_index(Some(&snapshot.pack_id))
            .unwrap_or_default();
        let mut other_custom = workspace_index.strings_by_pack.values().fold(
            crate::domain::namespace::model::PackStringNamespaceIndex::default(),
            |mut acc, item| {
                acc.extend(item);
                acc
            },
        );

        if let Some(previous) = previous {
            let previous_index = build_pack_strings_namespace_index(&PackStringsFile {
                schema_version: snapshot.strings.schema_version,
                entries: vec![previous.clone()],
            });
            for key in previous_index.system_keys {
                other_custom.system_keys.remove(&key);
            }
            for key in previous_index.victory_keys {
                other_custom.victory_keys.remove(&key);
            }
            for key in previous_index.counter_keys {
                other_custom.counter_keys.remove(&key);
            }
            for base in previous_index.setname_bases {
                other_custom.setname_bases.remove(&base);
            }
        }

        let standard =
            crate::infrastructure::standard_pack::standard_strings(self.state.app_data_dir())
                .unwrap_or_else(|| self.state.standard_baseline.strings.clone());

        validate_pack_string_record_namespace(
            record,
            &PackStringsNamespaceContext {
                other_custom,
                standard,
            },
        )
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

fn encode_pack_strings(strings: &PackStringsFile) -> AppResult<Vec<u8>> {
    serde_json::to_vec_pretty(strings)
        .map_err(|source| AppError::new("json_store.serialize_failed", source.to_string()))
}

fn validate_pack_strings_or_err(strings: &PackStringsFile) -> AppResult<()> {
    let issues = validate_pack_strings(strings);
    if issues
        .iter()
        .any(|issue| matches!(issue.level, IssueLevel::Error))
    {
        let error_issues = issues
            .into_iter()
            .filter(|issue| matches!(issue.level, IssueLevel::Error))
            .collect::<Vec<_>>();
        return Err(AppError::new(
            "pack_strings.validation_failed",
            "pack strings contain validation errors",
        )
        .with_detail("issues", &error_issues));
    }
    Ok(())
}

fn validate_card_languages_or_err(
    state: &AppState,
    input: &CardUpdateInput,
    existing_languages: &BTreeSet<String>,
) -> AppResult<()> {
    for language in input.texts.keys() {
        validate_language_or_err(
            state,
            language,
            existing_languages,
            "card",
            "texts",
            "card.text_language",
        )?;
    }
    Ok(())
}

fn validate_language_or_err(
    state: &AppState,
    language: &str,
    existing_languages: &BTreeSet<String>,
    scope: &str,
    field: &str,
    code: &str,
) -> AppResult<()> {
    let config = crate::application::config::service::ConfigService::new(state)
        .load()
        .unwrap_or_else(|_| crate::domain::config::rules::default_global_config());
    let issues = validate_catalog_membership(
        language,
        &config.text_language_catalog,
        existing_languages,
        scope,
        field,
        code,
    );
    if issues
        .iter()
        .any(|issue| matches!(issue.level, IssueLevel::Error))
    {
        return Err(AppError::new(
            format!("{scope}.language_validation_failed"),
            "language contains validation errors",
        )
        .with_detail("issues", &issues));
    }
    Ok(())
}

fn pack_existing_languages(snapshot: &PackSession) -> BTreeSet<String> {
    let mut languages = BTreeSet::new();
    languages.extend(snapshot.metadata.display_language_order.iter().cloned());
    if let Some(language) = &snapshot.metadata.default_export_language {
        languages.insert(language.clone());
    }
    for card in &snapshot.cards {
        languages.extend(card.texts.keys().cloned());
    }
    for record in &snapshot.strings.entries {
        languages.extend(record.values.keys().cloned());
    }
    languages
}

fn require_card<'a>(snapshot: &'a PackSession, card_id: &str) -> AppResult<&'a CardEntity> {
    snapshot
        .cards
        .iter()
        .find(|card| card.id == card_id)
        .ok_or_else(|| AppError::new("card.not_found", "card was not found"))
}

fn require_field_spell(card: &CardEntity) -> AppResult<()> {
    if matches!(card.primary_type, PrimaryType::Spell)
        && matches!(card.spell_subtype, Some(SpellSubtype::Field))
    {
        return Ok(());
    }
    Err(AppError::new(
        "resource.field_image_requires_field_spell",
        "field image can only be attached to field spell cards",
    ))
}

fn reject_authoring_system_string(
    kind: &crate::domain::strings::model::PackStringKind,
    key: u32,
) -> AppResult<()> {
    if matches!(kind, crate::domain::strings::model::PackStringKind::System) {
        return Err(AppError::new(
            "pack_strings.system_not_supported_for_custom_packs",
            "system strings are reserved and cannot be authored in custom packs",
        )
        .with_detail("key", key));
    }
    Ok(())
}
