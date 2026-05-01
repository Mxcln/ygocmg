use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::application::dto::common::PreviewResultDto;
use crate::application::dto::import::{
    ExecuteImportPackInput, ImportPreviewDto, PreviewImportPackInput,
};
use crate::application::dto::job::{JobAcceptedDto, JobKindDto};
use crate::application::standard_pack::repository::{
    SqliteStandardPackRepository, StandardPackRepository,
};
use crate::bootstrap::AppState;
use crate::domain::card::code::{
    CodePolicy, CodeValidationContext, STANDARD_RESERVED_CODE_MAX, validate_card_code,
};
use crate::domain::card::model::{CardEntity, PrimaryType, SpellSubtype};
use crate::domain::card::validate::validate_card_structure;
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::ids::LanguageCode;
use crate::domain::common::issue::{IssueLevel, ValidationIssue, ValidationTarget};
use crate::domain::common::time::{AppTimestamp, now_utc};
use crate::domain::language::rules::{normalize_language_id, validate_catalog_membership};
use crate::domain::pack::model::{PackKind, PackMetadata};
use crate::domain::pack::summary::{touch_pack_metadata, validate_pack_metadata};
use crate::domain::resource::path_rules::{
    card_image_path, field_image_path, pack_field_pics_dir, pack_pics_dir, pack_scripts_dir,
    script_path,
};
use crate::domain::strings::model::{PackStringRecord, PackStringsFile};
use crate::domain::workspace::rules::touch_workspace;
use crate::infrastructure::fs::transaction::{FsOperation, execute_plan};
use crate::infrastructure::json_store;
use crate::infrastructure::pack_locator;
use crate::runtime::preview_token_cache::{ImportPreviewEntry, write_cache};

const PREVIEW_TTL_MINUTES: i64 = 10;
const DEFAULT_SOURCE_LANGUAGE_KEY: &str = "default";

#[derive(Debug, Clone)]
struct PreparedImport {
    metadata: PackMetadata,
    pack_path: PathBuf,
    cards: Vec<CardEntity>,
    strings: PackStringsFile,
    issues: Vec<ValidationIssue>,
    missing_main_image_count: usize,
    missing_script_count: usize,
    missing_field_image_count: usize,
    resource_plan: ImportResourcePlan,
}

#[derive(Debug, Clone, Default)]
struct ImportResourcePlan {
    main_images: Vec<(PathBuf, u32)>,
    field_images: Vec<(PathBuf, u32)>,
    scripts: Vec<(PathBuf, u32)>,
}

pub struct ImportService<'a> {
    state: &'a AppState,
}

impl<'a> ImportService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub fn preview_import_pack(
        &self,
        input: PreviewImportPackInput,
    ) -> AppResult<PreviewResultDto<ImportPreviewDto>> {
        crate::application::pack::service::ensure_workspace_matches(
            self.state,
            &input.workspace_id,
        )?;

        let target_pack_id = Uuid::now_v7().to_string();
        let prepared = self.prepare_import(&input, &target_pack_id)?;
        let (warning_count, error_count) = issue_counts(&prepared.issues);
        let preview_token = Uuid::now_v7().to_string();
        let snapshot_hash = source_snapshot_hash(&input, &prepared.pack_path)?;
        let expires_at = preview_expires_at(now_utc());

        write_cache(&self.state.preview_token_cache)?.insert_import_entry(ImportPreviewEntry {
            preview_token: preview_token.clone(),
            workspace_id: input.workspace_id.clone(),
            target_pack_id,
            snapshot_hash: snapshot_hash.clone(),
            expires_at,
            input_snapshot: input,
        });

        Ok(PreviewResultDto {
            preview_token,
            snapshot_hash,
            expires_at,
            data: ImportPreviewDto {
                target_pack_id: prepared.metadata.id,
                target_pack_name: prepared.metadata.name,
                card_count: prepared.cards.len(),
                warning_count,
                error_count,
                missing_main_image_count: prepared.missing_main_image_count,
                missing_script_count: prepared.missing_script_count,
                missing_field_image_count: prepared.missing_field_image_count,
                issues: prepared.issues,
            },
        })
    }

    pub fn execute_import_pack(&self, input: ExecuteImportPackInput) -> AppResult<JobAcceptedDto> {
        let entry = {
            let mut cache = write_cache(&self.state.preview_token_cache)?;
            cache
                .remove_import_entry(&input.preview_token)
                .ok_or_else(|| {
                    AppError::new(
                        "import.preview_token_invalid",
                        "import preview token is missing or already consumed",
                    )
                    .with_detail("preview_token", input.preview_token.clone())
                })?
        };

        if entry.expires_at <= now_utc() {
            return Err(AppError::new(
                "import.preview_token_expired",
                "import preview token has expired",
            )
            .with_detail("preview_token", input.preview_token));
        }

        crate::application::pack::service::ensure_workspace_matches(
            self.state,
            &entry.workspace_id,
        )?;
        let state = self.state.clone();
        let workspace_id = entry.workspace_id.clone();
        let input_snapshot = entry.input_snapshot.clone();
        let target_pack_id = entry.target_pack_id.clone();
        let expected_snapshot_hash = entry.snapshot_hash.clone();

        self.state
            .jobs
            .submit(JobKindDto::ImportPack, move |context| {
                context.progress(
                    "validating_preview",
                    Some(5),
                    Some("Validating import preview".to_string()),
                )?;
                crate::application::pack::service::ensure_workspace_matches(&state, &workspace_id)?;
                let service = ImportService::new(&state);
                let prepared = service.prepare_import(&input_snapshot, &target_pack_id)?;
                let (_, error_count) = issue_counts(&prepared.issues);
                if error_count > 0 {
                    return Err(AppError::new(
                        "import.preview_has_errors",
                        "import preview contains blocking errors",
                    )
                    .with_detail("error_count", error_count));
                }
                let current_snapshot_hash =
                    source_snapshot_hash(&input_snapshot, &prepared.pack_path)?;
                if current_snapshot_hash != expected_snapshot_hash {
                    return Err(AppError::new(
                        "import.preview_stale",
                        "import preview no longer matches source files",
                    )
                    .with_detail("expected_snapshot_hash", expected_snapshot_hash)
                    .with_detail("actual_snapshot_hash", current_snapshot_hash));
                }

                context.progress(
                    "writing_pack",
                    Some(35),
                    Some("Writing imported pack".to_string()),
                )?;
                service.write_imported_pack(&workspace_id, prepared)?;

                context.progress(
                    "refreshing_workspace",
                    Some(95),
                    Some("Refreshing workspace".to_string()),
                )?;
                crate::application::pack::service::PackService::new(&state)
                    .refresh_current_workspace_summary()?;

                context.progress(
                    "import_ready",
                    Some(100),
                    Some("Import completed".to_string()),
                )
            })
    }

    fn prepare_import(
        &self,
        input: &PreviewImportPackInput,
        target_pack_id: &str,
    ) -> AppResult<PreparedImport> {
        let workspace_path =
            crate::application::workspace::service::WorkspaceService::new(self.state)
                .current_workspace_path()?;
        let config = crate::application::config::service::ConfigService::new(self.state)
            .load()
            .unwrap_or_else(|_| crate::domain::config::rules::default_global_config());
        let source_language = normalize_language_id(&input.source_language);
        let display_language_order =
            normalize_display_language_order(&input.display_language_order, &source_language);
        let empty_existing_languages = BTreeSet::new();
        let mut issues = Vec::new();
        issues.extend(validate_catalog_membership(
            &source_language,
            &config.text_language_catalog,
            &empty_existing_languages,
            "import",
            "source_language",
            "import.source_language",
        ));
        for language in &display_language_order {
            issues.extend(validate_catalog_membership(
                language,
                &config.text_language_catalog,
                &empty_existing_languages,
                "import",
                "display_language_order",
                "import.display_language",
            ));
        }
        if let Some(language) = &input.default_export_language {
            let normalized = normalize_language_id(language);
            issues.extend(validate_catalog_membership(
                &normalized,
                &config.text_language_catalog,
                &empty_existing_languages,
                "import",
                "default_export_language",
                "import.default_export_language",
            ));
        }
        if !input
            .display_language_order
            .iter()
            .any(|language| normalize_language_id(language) == source_language)
        {
            issues.push(
                ValidationIssue::warning(
                    "import.source_language_not_in_display_order",
                    ValidationTarget::new("import").with_field("display_language_order"),
                )
                .with_param("source_language", &source_language),
            );
        }

        let now = now_utc();
        let metadata = PackMetadata {
            id: target_pack_id.to_string(),
            kind: PackKind::Custom,
            name: input.new_pack_name.trim().to_string(),
            pack_code: input
                .new_pack_code
                .as_ref()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .map(|value| value.to_ascii_uppercase()),
            author: input.new_pack_author.trim().to_string(),
            version: input.new_pack_version.trim().to_string(),
            description: input.new_pack_description.clone(),
            created_at: now,
            updated_at: now,
            display_language_order,
            default_export_language: input
                .default_export_language
                .as_deref()
                .map(normalize_language_id)
                .filter(|value| !value.is_empty()),
        };

        issues.extend(
            validate_pack_metadata(&metadata)
                .into_iter()
                .map(prefix_pack_issue),
        );

        let storage_name =
            pack_locator::suggest_pack_storage_name(&workspace_path, &metadata.name, &metadata.id)?;
        let pack_path = json_store::packs_root_path(&workspace_path).join(storage_name);
        if pack_path.exists() {
            issues.push(
                ValidationIssue::error(
                    "import.target_pack_exists",
                    ValidationTarget::new("import").with_field("target_pack_id"),
                )
                .with_param("pack_id", &metadata.id)
                .with_param("path", pack_path.display().to_string()),
            );
        }

        let raw_records = crate::infrastructure::ygopro_cdb::load_cards_from_cdb(&input.cdb_path)?;
        let mut cards = Vec::with_capacity(raw_records.len());
        for record in raw_records {
            let mut card = record.card;
            card.id = Uuid::now_v7().to_string();
            remap_card_language(&mut card, &source_language);
            cards.push(card);
        }

        let mut code_counts = BTreeMap::<u32, usize>::new();
        for card in &cards {
            *code_counts.entry(card.code).or_default() += 1;
        }
        for (code, count) in code_counts {
            if count > 1 {
                issues.push(
                    ValidationIssue::error(
                        "import.cdb_duplicate_code",
                        ValidationTarget::new("import").with_field("code"),
                    )
                    .with_param("code", code)
                    .with_param("count", count),
                );
            }
        }

        let code_context = self.build_import_code_context()?;
        let imported_codes = cards.iter().map(|card| card.code).collect::<BTreeSet<_>>();
        for card in &cards {
            issues.extend(validate_card_structure(card).into_iter().map(|issue| {
                issue
                    .with_param("code", card.code)
                    .with_param("card_id", &card.id)
            }));
            let mut card_code_context = code_context.clone();
            card_code_context.current_pack_codes = imported_codes
                .iter()
                .copied()
                .filter(|code| *code != card.code)
                .collect();
            issues.extend(
                validate_card_code(card.code, &card_code_context)
                    .into_iter()
                    .map(|issue| {
                        issue
                            .with_param("code", card.code)
                            .with_param("card_id", &card.id)
                    }),
            );
        }

        let strings = if let Some(path) = &input.strings_conf_path {
            import_strings_file(path, &source_language)?
        } else {
            PackStringsFile::default()
        };

        let resource_scan = scan_resources(input, &cards);
        issues.extend(resource_scan.issues);

        Ok(PreparedImport {
            metadata,
            pack_path,
            cards,
            strings,
            issues,
            missing_main_image_count: resource_scan.missing_main_image_count,
            missing_script_count: resource_scan.missing_script_count,
            missing_field_image_count: resource_scan.missing_field_image_count,
            resource_plan: resource_scan.plan,
        })
    }

    fn build_import_code_context(&self) -> AppResult<CodeValidationContext> {
        let config = crate::application::config::service::ConfigService::new(self.state)
            .load()
            .unwrap_or_else(|_| crate::domain::config::rules::default_global_config());
        let sessions = self.state.sessions.read().map_err(|_| {
            AppError::new("pack.session_lock_poisoned", "pack session lock poisoned")
        })?;
        let workspace = sessions
            .current_workspace
            .as_ref()
            .ok_or_else(|| AppError::new("workspace.not_open", "no workspace is currently open"))?;
        let mut other_custom_codes = BTreeSet::new();
        for pack_path in workspace.pack_paths.values() {
            for card in json_store::load_cards(pack_path)? {
                other_custom_codes.insert(card.code);
            }
        }
        let standard_baseline = SqliteStandardPackRepository::new(self.state)
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
            current_pack_codes: BTreeSet::new(),
            other_custom_codes,
            standard_codes: standard_baseline.standard_codes,
        })
    }

    fn write_imported_pack(&self, workspace_id: &str, prepared: PreparedImport) -> AppResult<()> {
        let workspace_path =
            crate::application::workspace::service::WorkspaceService::new(self.state)
                .current_workspace_path()?;
        if prepared.pack_path.exists() {
            return Err(
                AppError::new("import.target_pack_exists", "target pack already exists")
                    .with_detail("path", prepared.pack_path.display().to_string()),
            );
        }

        let metadata = touch_pack_metadata(&prepared.metadata, now_utc());
        let mut operations = vec![
            FsOperation::CreateDir {
                path: prepared.pack_path.clone(),
            },
            FsOperation::CreateDir {
                path: pack_pics_dir(&prepared.pack_path),
            },
            FsOperation::CreateDir {
                path: pack_field_pics_dir(&prepared.pack_path),
            },
            FsOperation::CreateDir {
                path: pack_scripts_dir(&prepared.pack_path),
            },
            FsOperation::WriteFile {
                path: json_store::pack_metadata_path(&prepared.pack_path),
                contents: encode_pack_metadata(&metadata)?,
            },
            FsOperation::WriteFile {
                path: json_store::cards_path(&prepared.pack_path),
                contents: encode_cards(&prepared.cards)?,
            },
            FsOperation::WriteFile {
                path: json_store::pack_strings_path(&prepared.pack_path),
                contents: encode_pack_strings(&prepared.strings)?,
            },
        ];

        for (source, code) in prepared.resource_plan.main_images {
            operations.push(FsOperation::WriteFile {
                path: card_image_path(&prepared.pack_path, code),
                contents: crate::infrastructure::assets::import_main_image(&source)?,
            });
        }
        for (source, code) in prepared.resource_plan.field_images {
            operations.push(FsOperation::WriteFile {
                path: field_image_path(&prepared.pack_path, code),
                contents: crate::infrastructure::assets::import_field_image(&source)?,
            });
        }
        for (source, code) in prepared.resource_plan.scripts {
            let contents = fs::read(&source).map_err(|source_error| {
                AppError::from_io("import.script_read_failed", source_error)
                    .with_detail("path", source.display().to_string())
            })?;
            operations.push(FsOperation::WriteFile {
                path: script_path(&prepared.pack_path, code),
                contents,
            });
        }

        execute_plan(operations)?;
        self.update_workspace_for_import(&workspace_path, &metadata.id)?;
        crate::application::pack::service::ensure_workspace_matches(self.state, workspace_id)?;
        Ok(())
    }

    fn update_workspace_for_import(&self, workspace_path: &Path, pack_id: &str) -> AppResult<()> {
        let mut meta = json_store::load_workspace_meta(workspace_path)?;
        if !meta.pack_order.iter().any(|current| current == pack_id) {
            meta.pack_order.push(pack_id.to_string());
        }
        meta = touch_workspace(&meta, now_utc());
        json_store::save_workspace_meta(workspace_path, &meta)
    }
}

#[derive(Debug)]
struct ResourceScan {
    plan: ImportResourcePlan,
    issues: Vec<ValidationIssue>,
    missing_main_image_count: usize,
    missing_script_count: usize,
    missing_field_image_count: usize,
}

fn scan_resources(input: &PreviewImportPackInput, cards: &[CardEntity]) -> ResourceScan {
    let mut plan = ImportResourcePlan::default();
    let mut issues = Vec::new();
    let mut missing_main_image_count = 0;
    let mut missing_script_count = 0;
    let mut missing_field_image_count = 0;

    for card in cards {
        match find_main_image(input.pics_dir.as_deref(), card.code) {
            Some(path) => plan.main_images.push((path, card.code)),
            None => {
                missing_main_image_count += 1;
                issues.push(missing_resource_issue(
                    "import.missing_main_image",
                    "pics",
                    card.code,
                ));
            }
        }

        match find_script(input.script_dir.as_deref(), card.code) {
            Some(path) => plan.scripts.push((path, card.code)),
            None => {
                missing_script_count += 1;
                issues.push(missing_resource_issue(
                    "import.missing_script",
                    "script",
                    card.code,
                ));
            }
        }

        if is_field_spell(card) {
            match find_field_image(input.field_pics_dir.as_deref(), card.code) {
                Some(path) => plan.field_images.push((path, card.code)),
                None => {
                    missing_field_image_count += 1;
                    issues.push(missing_resource_issue(
                        "import.missing_field_image",
                        "pics/field",
                        card.code,
                    ));
                }
            }
        } else if let Some(path) = find_field_image(input.field_pics_dir.as_deref(), card.code) {
            plan.field_images.push((path, card.code));
        }
    }

    ResourceScan {
        plan,
        issues,
        missing_main_image_count,
        missing_script_count,
        missing_field_image_count,
    }
}

fn find_main_image(dir: Option<&Path>, code: u32) -> Option<PathBuf> {
    dir.and_then(|dir| find_existing(&[dir.join(format!("{code}.jpg"))]))
}

fn find_field_image(dir: Option<&Path>, code: u32) -> Option<PathBuf> {
    dir.and_then(|dir| find_existing(&[dir.join(format!("{code}.jpg"))]))
}

fn find_script(dir: Option<&Path>, code: u32) -> Option<PathBuf> {
    dir.and_then(|dir| find_existing(&[dir.join(format!("c{code}.lua"))]))
}

fn find_existing(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|path| path.exists()).cloned()
}

fn missing_resource_issue(code: &str, field: &str, card_code: u32) -> ValidationIssue {
    ValidationIssue::warning(code, ValidationTarget::new("import").with_field(field))
        .with_param("code", card_code)
}

fn is_field_spell(card: &CardEntity) -> bool {
    matches!(card.primary_type, PrimaryType::Spell)
        && matches!(card.spell_subtype, Some(SpellSubtype::Field))
}

fn import_strings_file(path: &Path, source_language: &str) -> AppResult<PackStringsFile> {
    let mut records = crate::infrastructure::strings_conf::load_records(path)?;
    for record in &mut records {
        remap_string_language(record, source_language);
    }
    let mut file = PackStringsFile {
        schema_version: crate::domain::strings::model::PACK_STRINGS_SCHEMA_VERSION,
        entries: records,
    };
    file.sort_entries();
    Ok(file)
}

fn remap_card_language(card: &mut CardEntity, source_language: &str) {
    if card.texts.contains_key(source_language) {
        return;
    }
    if let Some(texts) = card.texts.remove(DEFAULT_SOURCE_LANGUAGE_KEY) {
        card.texts.insert(source_language.to_string(), texts);
    }
}

fn remap_string_language(record: &mut PackStringRecord, source_language: &str) {
    if record.values.contains_key(source_language) {
        return;
    }
    if let Some(value) = record.values.remove(DEFAULT_SOURCE_LANGUAGE_KEY) {
        record.values.insert(source_language.to_string(), value);
    }
}

fn normalize_display_language_order(
    languages: &[LanguageCode],
    source_language: &str,
) -> Vec<LanguageCode> {
    let mut deduped = Vec::new();
    for language in languages {
        if !deduped.iter().any(|current| current == language) {
            deduped.push(language.clone());
        }
    }
    if !deduped.iter().any(|current| current == source_language) {
        deduped.insert(0, source_language.to_string());
    }
    deduped
}

fn prefix_pack_issue(issue: ValidationIssue) -> ValidationIssue {
    issue
        .with_param("source", "pack_metadata")
        .with_param("scope", "import")
}

fn issue_counts(issues: &[ValidationIssue]) -> (usize, usize) {
    let warning_count = issues
        .iter()
        .filter(|issue| matches!(issue.level, IssueLevel::Warning))
        .count();
    let error_count = issues
        .iter()
        .filter(|issue| matches!(issue.level, IssueLevel::Error))
        .count();
    (warning_count, error_count)
}

fn preview_expires_at(now: AppTimestamp) -> AppTimestamp {
    now + chrono::Duration::minutes(PREVIEW_TTL_MINUTES)
}

fn source_snapshot_hash(input: &PreviewImportPackInput, pack_path: &Path) -> AppResult<String> {
    let mut parts = vec![
        format!("pack_path:{}", pack_path.display()),
        format!("workspace_id:{}", input.workspace_id),
        format!("source_language:{}", input.source_language),
        format!("cdb:{}", path_stamp(&input.cdb_path)?),
    ];

    for (label, path) in [
        ("pics_dir", input.pics_dir.as_ref()),
        ("field_pics_dir", input.field_pics_dir.as_ref()),
        ("script_dir", input.script_dir.as_ref()),
        ("strings_conf", input.strings_conf_path.as_ref()),
    ] {
        parts.push(match path {
            Some(path) if path.is_dir() => format!("{label}:{}", directory_stamp(path)?),
            Some(path) => format!("{label}:{}", path_stamp(path)?),
            None => format!("{label}:<none>"),
        });
    }

    Ok(parts.join("|"))
}

fn directory_stamp(path: &Path) -> AppResult<String> {
    let mut stamps = Vec::new();
    for entry in fs::read_dir(path).map_err(|source| {
        AppError::from_io("import.source_dir_read_failed", source)
            .with_detail("path", path.display().to_string())
    })? {
        let entry = entry.map_err(|source| {
            AppError::from_io("import.source_dir_entry_failed", source)
                .with_detail("path", path.display().to_string())
        })?;
        let entry_path = entry.path();
        if entry_path.is_file() {
            stamps.push(path_stamp(&entry_path)?);
        }
    }
    stamps.sort();
    Ok(format!("{}:[{}]", path.display(), stamps.join(",")))
}

fn path_stamp(path: &Path) -> AppResult<String> {
    let metadata = fs::metadata(path).map_err(|source| {
        AppError::from_io("import.source_metadata_failed", source)
            .with_detail("path", path.display().to_string())
    })?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    Ok(format!(
        "{}:{}:{}",
        path.display(),
        metadata.len(),
        modified
    ))
}

fn encode_pack_metadata(metadata: &PackMetadata) -> AppResult<Vec<u8>> {
    serde_json::to_vec_pretty(&crate::domain::pack::model::PackMetadataFile {
        schema_version: json_store::SCHEMA_VERSION,
        data: metadata.clone(),
    })
    .map_err(|source| AppError::new("import.serialize_metadata_failed", source.to_string()))
}

fn encode_cards(cards: &[CardEntity]) -> AppResult<Vec<u8>> {
    serde_json::to_vec_pretty(&crate::domain::card::model::CardsFile {
        schema_version: json_store::SCHEMA_VERSION,
        cards: cards.to_vec(),
    })
    .map_err(|source| AppError::new("import.serialize_cards_failed", source.to_string()))
}

fn encode_pack_strings(strings: &PackStringsFile) -> AppResult<Vec<u8>> {
    serde_json::to_vec_pretty(strings)
        .map_err(|source| AppError::new("import.serialize_strings_failed", source.to_string()))
}
