use std::collections::{BTreeMap, BTreeSet};

use uuid::Uuid;

use crate::application::dto::common::PreviewResultDto;
use crate::application::dto::export::{ExportPreviewDto, PreviewExportBundleInput};
use crate::bootstrap::AppState;
use crate::domain::card::code::STANDARD_RESERVED_CODE_MAX;
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::issue::{IssueLevel, ValidationIssue, ValidationTarget};
use crate::domain::common::time::{AppTimestamp, now_utc};
use crate::domain::namespace::model::{counter_low12, setname_base};
use crate::domain::strings::model::PackStringKind;

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

        let issues = self.collect_export_issues(&packs, &input.export_language);
        let warning_count = issues
            .iter()
            .filter(|issue| matches!(issue.level, IssueLevel::Warning))
            .count();
        let error_count = issues
            .iter()
            .filter(|issue| matches!(issue.level, IssueLevel::Error))
            .count();

        Ok(PreviewResultDto {
            preview_token: Uuid::now_v7().to_string(),
            snapshot_hash: self.snapshot_hash(&packs),
            expires_at: preview_expires_at(now_utc()),
            data: ExportPreviewDto {
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
        })
    }

    fn snapshot_hash(&self, packs: &[crate::runtime::sessions::PackSession]) -> String {
        packs
            .iter()
            .map(|pack| format!("{}:{}:{}", pack.pack_id, pack.revision, pack.source_stamp))
            .collect::<Vec<_>>()
            .join("|")
    }

    fn collect_export_issues(
        &self,
        packs: &[crate::runtime::sessions::PackSession],
        export_language: &str,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        let mut code_owners = BTreeMap::<u32, Vec<String>>::new();
        let mut setname_base_owners = BTreeMap::<u16, Vec<String>>::new();
        let mut counter_owners = BTreeMap::<u32, Vec<String>>::new();
        let mut victory_owners = BTreeMap::<u32, Vec<String>>::new();
        let standard_baseline = crate::infrastructure::standard_pack::standard_baseline_from_index(
            self.state.app_data_dir(),
        )
        .unwrap_or_else(|| self.state.standard_baseline.clone());

        for pack in packs {
            for card in &pack.cards {
                code_owners
                    .entry(card.code)
                    .or_default()
                    .push(pack.pack_id.clone());
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
                        setname_base_owners
                            .entry(base)
                            .or_default()
                            .push(pack.pack_id.clone());
                        if standard_baseline.strings.setname_bases.contains(&base) {
                            issues.push(
                                ValidationIssue::error(
                                    "export.setname_base_conflicts_with_standard_pack",
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
                        let _low12 = counter_low12(record.key);
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
            "export.setname_base_conflicts_between_selected_packs",
            "base",
            setname_base_owners,
        );
        push_duplicate_owner_issues(
            &mut issues,
            "export.counter_key_conflicts_between_selected_packs",
            "key",
            counter_owners,
        );
        push_duplicate_owner_issues(
            &mut issues,
            "export.victory_key_conflicts_between_selected_packs",
            "key",
            victory_owners,
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
) where
    T: serde::Serialize + Ord + Copy,
{
    for (value, pack_ids) in owners {
        let unique = pack_ids.iter().collect::<BTreeSet<_>>();
        if unique.len() > 1 {
            issues.push(
                ValidationIssue::error(
                    code,
                    ValidationTarget::new("export").with_field(field_name),
                )
                .with_param(field_name, value)
                .with_param("pack_ids", pack_ids),
            );
        }
    }
}
