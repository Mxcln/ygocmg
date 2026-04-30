use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::application::dto::common::PreviewResultDto;
use crate::application::dto::export::{
    ExecuteExportBundleInput, ExportPreviewDto, PreviewExportBundleInput,
};
use crate::application::dto::job::{JobAcceptedDto, JobKindDto};
use crate::application::standard_pack::repository::{
    JsonStandardPackRepository, StandardPackRepository,
};
use crate::bootstrap::AppState;
use crate::domain::card::code::STANDARD_RESERVED_CODE_MAX;
use crate::domain::card::validate::validate_card_structure;
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::issue::{IssueLevel, ValidationIssue, ValidationTarget};
use crate::domain::common::time::{AppTimestamp, now_utc};
use crate::domain::language::rules::{normalize_language_id, validate_catalog_membership};
use crate::domain::namespace::model::setname_base;
use crate::domain::pack::model::PackKind;
use crate::domain::resource::path_rules::{card_image_path, field_image_path, script_path};
use crate::domain::strings::model::PackStringKind;
use crate::runtime::preview_token_cache::{ExportPreviewEntry, write_cache};
use crate::runtime::sessions::PackSession;

pub struct ExportService<'a> {
    state: &'a AppState,
}

impl<'a> ExportService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub fn preview_export_bundle(
        &self,
        input: PreviewExportBundleInput,
    ) -> AppResult<PreviewResultDto<ExportPreviewDto>> {
        let prepared = self.prepare_export(&input)?;
        let preview_token = Uuid::now_v7().to_string();
        let expires_at = preview_expires_at(now_utc());
        let snapshot_hash = prepared.snapshot_hash.clone();

        write_cache(&self.state.preview_token_cache)?.insert_export_entry(ExportPreviewEntry {
            preview_token: preview_token.clone(),
            workspace_id: input.workspace_id.clone(),
            pack_ids: input.pack_ids.clone(),
            snapshot_hash: snapshot_hash.clone(),
            expires_at,
            input_snapshot: input,
        });

        Ok(PreviewResultDto {
            preview_token,
            snapshot_hash,
            expires_at,
            data: prepared.preview,
        })
    }

    pub fn execute_export_bundle(
        &self,
        input: ExecuteExportBundleInput,
    ) -> AppResult<JobAcceptedDto> {
        let entry = {
            let mut cache = write_cache(&self.state.preview_token_cache)?;
            cache
                .remove_export_entry(&input.preview_token)
                .ok_or_else(|| {
                    AppError::new(
                        "export.preview_token_invalid",
                        "export preview token is missing or already consumed",
                    )
                    .with_detail("preview_token", input.preview_token.clone())
                })?
        };

        if entry.expires_at <= now_utc() {
            return Err(AppError::new(
                "export.preview_token_expired",
                "export preview token has expired",
            )
            .with_detail("preview_token", input.preview_token));
        }

        crate::application::pack::service::ensure_workspace_matches(
            self.state,
            &entry.workspace_id,
        )?;
        let state = self.state.clone();
        let workspace_id = entry.workspace_id.clone();
        let pack_ids = entry.pack_ids.clone();
        let input_snapshot = entry.input_snapshot.clone();
        let expected_snapshot_hash = entry.snapshot_hash.clone();

        self.state
            .jobs
            .submit(JobKindDto::ExportBundle, move |context| {
                context.progress(
                    "validating_preview",
                    Some(5),
                    Some("Validating export preview".to_string()),
                )?;
                crate::application::pack::service::ensure_workspace_matches(&state, &workspace_id)?;
                if input_snapshot.pack_ids != pack_ids {
                    return Err(AppError::new(
                        "export.preview_pack_ids_changed",
                        "export preview pack selection changed",
                    ));
                }
                let service = ExportService::new(&state);
                let prepared = service.prepare_export(&input_snapshot)?;
                if prepared.preview.error_count > 0 {
                    return Err(AppError::new(
                        "export.preview_has_errors",
                        "export preview contains blocking errors",
                    )
                    .with_detail("error_count", prepared.preview.error_count));
                }
                if prepared.snapshot_hash != expected_snapshot_hash {
                    return Err(AppError::new(
                        "export.preview_stale",
                        "export preview no longer matches selected packs",
                    )
                    .with_detail("expected_snapshot_hash", expected_snapshot_hash)
                    .with_detail("actual_snapshot_hash", prepared.snapshot_hash));
                }

                context.progress(
                    "writing_export",
                    Some(35),
                    Some("Writing export bundle".to_string()),
                )?;
                write_export_bundle(&input_snapshot, &prepared.packs)?;

                context.progress(
                    "export_ready",
                    Some(100),
                    Some("Export completed".to_string()),
                )
            })
    }

    fn prepare_export(&self, input: &PreviewExportBundleInput) -> AppResult<PreparedExport> {
        crate::application::pack::service::ensure_workspace_matches(
            self.state,
            &input.workspace_id,
        )?;
        if input.pack_ids.is_empty() {
            return Err(AppError::new(
                "export.pack_ids_required",
                "at least one pack must be selected for export",
            ));
        }
        if let Some(pack_id) = first_duplicate_pack_id(&input.pack_ids) {
            return Err(AppError::new(
                "export.pack_ids_duplicate",
                "export pack selection contains duplicate pack ids",
            )
            .with_detail("pack_id", pack_id));
        }
        validate_output_name(&input.output_name)?;

        let export_language = normalize_language_id(&input.export_language);
        let mut packs = Vec::new();
        for pack_id in &input.pack_ids {
            packs.push(
                crate::application::pack::service::require_open_pack_snapshot(
                    self.state,
                    &input.workspace_id,
                    pack_id,
                )?,
            );
        }

        let issues = self.collect_export_issues(&packs, input, &export_language);
        let warning_count = issues
            .iter()
            .filter(|issue| matches!(issue.level, IssueLevel::Warning))
            .count();
        let error_count = issues
            .iter()
            .filter(|issue| matches!(issue.level, IssueLevel::Error))
            .count();

        Ok(PreparedExport {
            snapshot_hash: self.snapshot_hash(&packs),
            preview: ExportPreviewDto {
                pack_count: packs.len(),
                card_count: packs.iter().map(|pack| pack.cards.len()).sum(),
                main_image_count: packs
                    .iter()
                    .map(|pack| {
                        pack.asset_index
                            .values()
                            .filter(|state| state.has_image)
                            .count()
                    })
                    .sum(),
                field_image_count: packs
                    .iter()
                    .map(|pack| {
                        pack.asset_index
                            .values()
                            .filter(|state| state.has_field_image)
                            .count()
                    })
                    .sum(),
                script_count: packs
                    .iter()
                    .map(|pack| {
                        pack.asset_index
                            .values()
                            .filter(|state| state.has_script)
                            .count()
                    })
                    .sum(),
                warning_count,
                error_count,
                issues,
            },
            packs,
        })
    }

    fn snapshot_hash(&self, packs: &[PackSession]) -> String {
        packs
            .iter()
            .map(|pack| format!("{}:{}:{}", pack.pack_id, pack.revision, pack.source_stamp))
            .collect::<Vec<_>>()
            .join("|")
    }

    fn collect_export_issues(
        &self,
        packs: &[PackSession],
        input: &PreviewExportBundleInput,
        export_language: &str,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let config = crate::application::config::service::ConfigService::new(self.state)
            .load()
            .unwrap_or_else(|_| crate::domain::config::rules::default_global_config());
        let existing_languages = packs
            .iter()
            .flat_map(pack_existing_languages)
            .collect::<BTreeSet<_>>();
        issues.extend(validate_catalog_membership(
            export_language,
            &config.text_language_catalog,
            &existing_languages,
            "export",
            "export_language",
            "export.language",
        ));

        let mut code_owners = BTreeMap::<u32, Vec<String>>::new();
        let mut setname_key_owners = BTreeMap::<u32, Vec<String>>::new();
        let mut setname_base_owners = BTreeMap::<u16, Vec<String>>::new();
        let mut counter_owners = BTreeMap::<u32, Vec<String>>::new();
        let mut victory_owners = BTreeMap::<u32, Vec<String>>::new();
        let standard_baseline = JsonStandardPackRepository::new(self.state)
            .namespace_baseline()
            .unwrap_or_else(|_| self.state.standard_baseline.clone());

        let output_path = export_output_path(input);
        if export_output_path_is_blocked(&output_path) {
            issues.push(
                ValidationIssue::error(
                    "export.output_dir_not_empty",
                    ValidationTarget::new("export").with_field("output_dir"),
                )
                .with_param("path", output_path.display().to_string()),
            );
        }

        for pack in packs {
            if !matches!(pack.metadata.kind, PackKind::Custom) {
                issues.push(
                    ValidationIssue::error(
                        "export.pack_kind_not_supported",
                        ValidationTarget::new("export")
                            .with_entity_id(pack.pack_id.clone())
                            .with_field("pack_ids"),
                    )
                    .with_param("pack_id", &pack.pack_id),
                );
            }
            for card in &pack.cards {
                code_owners
                    .entry(card.code)
                    .or_default()
                    .push(pack.pack_id.clone());
                for issue in validate_card_structure(card) {
                    issues.push(
                        issue
                            .with_param("pack_id", &pack.pack_id)
                            .with_param("card_id", &card.id)
                            .with_param("code", card.code),
                    );
                }
                if !card.texts.contains_key(export_language) {
                    issues.push(
                        ValidationIssue::error(
                            "export.card_text_missing_target_language",
                            ValidationTarget::new("export")
                                .with_entity_id(card.id.clone())
                                .with_field("texts"),
                        )
                        .with_param("pack_id", &pack.pack_id)
                        .with_param("card_id", &card.id)
                        .with_param("code", card.code)
                        .with_param("language", export_language),
                    );
                }
            }
            for record in &pack.strings.entries {
                if !record.values.contains_key(export_language) {
                    issues.push(
                        ValidationIssue::error(
                            "export.pack_string_missing_target_language",
                            ValidationTarget::new("export")
                                .with_entity_id(pack.pack_id.clone())
                                .with_field("strings"),
                        )
                        .with_param("pack_id", &pack.pack_id)
                        .with_param("kind", &record.kind)
                        .with_param("key", record.key)
                        .with_param("language", export_language),
                    );
                }

                match record.kind {
                    PackStringKind::System => {
                        if standard_baseline.strings.system_keys.contains(&record.key) {
                            issues.push(
                                ValidationIssue::error(
                                    "export.system_key_conflicts_with_standard_pack",
                                    ValidationTarget::new("export")
                                        .with_entity_id(pack.pack_id.clone())
                                        .with_field("strings"),
                                )
                                .with_param("pack_id", &pack.pack_id)
                                .with_param("key", record.key),
                            );
                        }
                    }
                    PackStringKind::Setname => {
                        let base = setname_base(record.key);
                        setname_key_owners
                            .entry(record.key)
                            .or_default()
                            .push(pack.pack_id.clone());
                        setname_base_owners
                            .entry(base)
                            .or_default()
                            .push(pack.pack_id.clone());
                        if standard_baseline.strings.setname_keys.contains(&record.key) {
                            issues.push(
                                ValidationIssue::error(
                                    "export.setname_key_conflicts_with_standard_pack",
                                    ValidationTarget::new("export")
                                        .with_entity_id(pack.pack_id.clone())
                                        .with_field("strings"),
                                )
                                .with_param("pack_id", &pack.pack_id)
                                .with_param("key", record.key),
                            );
                        } else if standard_baseline.strings.setname_bases.contains(&base) {
                            issues.push(
                                ValidationIssue::warning(
                                    "export.setname_base_overlaps_standard_pack",
                                    ValidationTarget::new("export")
                                        .with_entity_id(pack.pack_id.clone())
                                        .with_field("strings"),
                                )
                                .with_param("pack_id", &pack.pack_id)
                                .with_param("base", base)
                                .with_param("key", record.key),
                            );
                        }
                    }
                    PackStringKind::Counter => {
                        counter_owners
                            .entry(record.key)
                            .or_default()
                            .push(pack.pack_id.clone());
                        if standard_baseline.strings.counter_keys.contains(&record.key) {
                            issues.push(
                                ValidationIssue::error(
                                    "export.counter_key_conflicts_with_standard_pack",
                                    ValidationTarget::new("export")
                                        .with_entity_id(pack.pack_id.clone())
                                        .with_field("strings"),
                                )
                                .with_param("pack_id", &pack.pack_id)
                                .with_param("key", record.key),
                            );
                        }
                    }
                    PackStringKind::Victory => {
                        victory_owners
                            .entry(record.key)
                            .or_default()
                            .push(pack.pack_id.clone());
                        if standard_baseline.strings.victory_keys.contains(&record.key) {
                            issues.push(
                                ValidationIssue::error(
                                    "export.victory_key_conflicts_with_standard_pack",
                                    ValidationTarget::new("export")
                                        .with_entity_id(pack.pack_id.clone())
                                        .with_field("strings"),
                                )
                                .with_param("pack_id", &pack.pack_id)
                                .with_param("key", record.key),
                            );
                        }
                    }
                }
            }
        }

        for (code, owners) in code_owners {
            if owners.len() > 1 {
                issues.push(
                    ValidationIssue::error(
                        "export.code_conflicts_between_selected_packs",
                        ValidationTarget::new("export").with_field("code"),
                    )
                    .with_param("code", code)
                    .with_param("pack_ids", owners),
                );
            }
            if standard_baseline.standard_codes.contains(&code) {
                issues.push(
                    ValidationIssue::error(
                        "export.code_conflicts_with_standard_pack",
                        ValidationTarget::new("export").with_field("code"),
                    )
                    .with_param("code", code),
                );
            } else if code <= STANDARD_RESERVED_CODE_MAX {
                issues.push(
                    ValidationIssue::warning(
                        "export.code_in_standard_reserved_range",
                        ValidationTarget::new("export").with_field("code"),
                    )
                    .with_param("code", code)
                    .with_param("reserved_max", STANDARD_RESERVED_CODE_MAX),
                );
            }
        }

        push_duplicate_owner_issues(
            &mut issues,
            "export.setname_key_conflicts_between_selected_packs",
            "key",
            setname_key_owners,
            IssueLevel::Error,
        );
        push_duplicate_owner_issues(
            &mut issues,
            "export.setname_base_overlaps_between_selected_packs",
            "base",
            setname_base_owners,
            IssueLevel::Warning,
        );
        push_duplicate_owner_issues(
            &mut issues,
            "export.counter_key_conflicts_between_selected_packs",
            "key",
            counter_owners,
            IssueLevel::Error,
        );
        push_duplicate_owner_issues(
            &mut issues,
            "export.victory_key_conflicts_between_selected_packs",
            "key",
            victory_owners,
            IssueLevel::Error,
        );

        issues
    }
}

fn preview_expires_at(now: AppTimestamp) -> AppTimestamp {
    now + chrono::Duration::minutes(10)
}

fn push_duplicate_owner_issues<T>(
    issues: &mut Vec<ValidationIssue>,
    code: &str,
    field_name: &str,
    owners: BTreeMap<T, Vec<String>>,
    level: IssueLevel,
) where
    T: serde::Serialize + Ord + Copy,
{
    for (value, pack_ids) in owners {
        let unique = pack_ids.iter().collect::<BTreeSet<_>>();
        if unique.len() > 1 {
            let issue = match level {
                IssueLevel::Error => ValidationIssue::error(
                    code,
                    ValidationTarget::new("export").with_field(field_name),
                ),
                IssueLevel::Warning => ValidationIssue::warning(
                    code,
                    ValidationTarget::new("export").with_field(field_name),
                ),
            };
            issues.push(
                issue
                    .with_param(field_name, value)
                    .with_param("pack_ids", pack_ids),
            );
        }
    }
}

fn first_duplicate_pack_id(pack_ids: &[String]) -> Option<&str> {
    let mut seen = BTreeSet::new();
    for pack_id in pack_ids {
        if !seen.insert(pack_id.as_str()) {
            return Some(pack_id.as_str());
        }
    }
    None
}

fn validate_output_name(output_name: &str) -> AppResult<()> {
    let trimmed = output_name.trim();
    if trimmed.is_empty() {
        return Err(AppError::new(
            "export.output_name_required",
            "export output name is required",
        ));
    }

    if !is_safe_output_name(trimmed) {
        return Err(AppError::new(
            "export.output_name_invalid",
            "export output name must be a single safe file name",
        )
        .with_detail("output_name", output_name.to_string()));
    }

    Ok(())
}

fn is_safe_output_name(value: &str) -> bool {
    if value == "." || value == ".." || Path::new(value).is_absolute() {
        return false;
    }
    if value.ends_with(' ') || value.ends_with('.') {
        return false;
    }
    if value.chars().any(|ch| {
        ch == '/'
            || ch == '\\'
            || ch.is_control()
            || matches!(ch, '<' | '>' | ':' | '"' | '|' | '?' | '*')
    }) {
        return false;
    }
    !is_reserved_windows_device_name(value)
}

fn is_reserved_windows_device_name(value: &str) -> bool {
    let stem = value
        .split('.')
        .next()
        .unwrap_or(value)
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || matches!(
            stem.as_bytes(),
            [b'C', b'O', b'M', b'1'..=b'9'] | [b'L', b'P', b'T', b'1'..=b'9']
        )
}

#[derive(Debug)]
struct PreparedExport {
    packs: Vec<PackSession>,
    snapshot_hash: String,
    preview: ExportPreviewDto,
}

fn export_output_path(input: &PreviewExportBundleInput) -> PathBuf {
    input.output_dir.join(input.output_name.trim())
}

fn export_output_path_is_blocked(path: &Path) -> bool {
    path.exists() && !is_empty_dir(path)
}

fn is_empty_dir(path: &Path) -> bool {
    path.is_dir()
        && fs::read_dir(path)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false)
}

fn write_export_bundle(input: &PreviewExportBundleInput, packs: &[PackSession]) -> AppResult<()> {
    let output_path = export_output_path(input);
    if export_output_path_is_blocked(&output_path) {
        return Err(AppError::new(
            "export.output_dir_not_empty",
            "export output directory already exists and is not empty",
        )
        .with_detail("path", output_path.display().to_string()));
    }

    let pics_dir = output_path.join("pics");
    let field_dir = pics_dir.join("field");
    let script_dir = output_path.join("script");
    fs::create_dir_all(&field_dir).map_err(|source| {
        AppError::from_io("export.create_dir_failed", source)
            .with_detail("path", field_dir.display().to_string())
    })?;
    fs::create_dir_all(&script_dir).map_err(|source| {
        AppError::from_io("export.create_dir_failed", source)
            .with_detail("path", script_dir.display().to_string())
    })?;

    let cards = packs
        .iter()
        .flat_map(|pack| pack.cards.iter().cloned())
        .collect::<Vec<_>>();
    let strings = packs
        .iter()
        .flat_map(|pack| pack.strings.entries.iter().cloned())
        .collect::<Vec<_>>();

    let export_language = normalize_language_id(&input.export_language);
    crate::infrastructure::ygopro_cdb::write_cards_to_cdb(
        &output_path.join(format!("{}.cdb", input.output_name.trim())),
        &cards,
        &export_language,
    )?;
    crate::infrastructure::strings_conf::write_records(
        &output_path.join("strings.conf"),
        &strings,
        &export_language,
    )?;
    copy_export_assets(&output_path, packs)
}

fn pack_existing_languages(pack: &PackSession) -> BTreeSet<String> {
    let mut languages = BTreeSet::new();
    languages.extend(pack.metadata.display_language_order.iter().cloned());
    if let Some(language) = &pack.metadata.default_export_language {
        languages.insert(language.clone());
    }
    for card in &pack.cards {
        languages.extend(card.texts.keys().cloned());
    }
    for record in &pack.strings.entries {
        languages.extend(record.values.keys().cloned());
    }
    languages
}

fn copy_export_assets(output_path: &Path, packs: &[PackSession]) -> AppResult<()> {
    for pack in packs {
        for card in &pack.cards {
            copy_if_exists(
                &card_image_path(&pack.pack_path, card.code),
                &output_path.join("pics").join(format!("{}.jpg", card.code)),
            )?;
            copy_if_exists(
                &field_image_path(&pack.pack_path, card.code),
                &output_path
                    .join("pics")
                    .join("field")
                    .join(format!("{}.jpg", card.code)),
            )?;
            copy_if_exists(
                &script_path(&pack.pack_path, card.code),
                &output_path
                    .join("script")
                    .join(format!("c{}.lua", card.code)),
            )?;
        }
    }
    Ok(())
}

fn copy_if_exists(source: &Path, target: &Path) -> AppResult<()> {
    if !source.exists() {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|io_err| {
            AppError::from_io("export.create_dir_failed", io_err)
                .with_detail("path", parent.display().to_string())
        })?;
    }
    fs::copy(source, target)
        .map(|_| ())
        .map_err(|source_error| {
            AppError::from_io("export.copy_asset_failed", source_error)
                .with_detail("source", source.display().to_string())
                .with_detail("target", target.display().to_string())
        })
}
