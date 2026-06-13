use crate::application::dto::strings::{
    GetPackStringInput, ListPackStringsInput, PackStringRecordDetailDto, PackStringsPageDto,
    SetnameKeySuggestionDto, SuggestSetnameKeyInput,
};
use crate::application::standard_pack::repository::{
    SqliteStandardPackRepository, StandardPackRepository,
};
use crate::bootstrap::AppState;
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::issue::{ValidationIssue, ValidationTarget};
use crate::domain::namespace::model::{
    PackStringNamespaceIndex, build_pack_strings_namespace_index, suggest_next_setname_base,
};
use crate::domain::strings::model::PackStringEntry;
use crate::infrastructure::json_store;

pub struct PackStringsService<'a> {
    state: &'a AppState,
}

impl<'a> PackStringsService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub fn list_pack_strings(&self, input: ListPackStringsInput) -> AppResult<PackStringsPageDto> {
        let pack = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            &input.workspace_id,
            &input.pack_id,
        )?;

        let keyword = input.keyword.unwrap_or_default().trim().to_lowercase();
        let mut items = pack
            .strings
            .project_language_entries(&input.language)
            .into_iter()
            .filter(|entry| {
                matches_filters(
                    entry,
                    input.kind_filter.as_ref(),
                    input.key_filter,
                    &keyword,
                )
            })
            .collect::<Vec<_>>();

        items.sort_by(|left, right| left.kind.cmp(&right.kind).then(left.key.cmp(&right.key)));

        let page_size = input.page_size.max(1);
        let page = input.page.max(1);
        let total = items.len() as u64;
        let start = ((page - 1) as usize).saturating_mul(page_size as usize);
        let paged = if start >= items.len() {
            Vec::new()
        } else {
            items
                .into_iter()
                .skip(start)
                .take(page_size as usize)
                .map(Into::into)
                .collect()
        };

        Ok(PackStringsPageDto {
            language: input.language,
            items: paged,
            page,
            page_size,
            total,
        })
    }

    pub fn get_pack_string(
        &self,
        input: GetPackStringInput,
    ) -> AppResult<PackStringRecordDetailDto> {
        let pack = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            &input.workspace_id,
            &input.pack_id,
        )?;

        let record = pack
            .strings
            .get_record(&input.kind, input.key)
            .cloned()
            .ok_or_else(|| {
                AppError::new("pack_strings.not_found", "pack string record was not found")
            })?;

        Ok(PackStringRecordDetailDto {
            record: record.into(),
        })
    }

    /// Suggest the next free top-level setname base (child = 0) for a pack,
    /// avoiding bases already used by this pack, other custom packs in the
    /// workspace, and the standard reference. The recommended base range comes
    /// from global config. Returns `suggested_key = None` when the range is full.
    pub fn suggest_setname_key(
        &self,
        input: SuggestSetnameKeyInput,
    ) -> AppResult<SetnameKeySuggestionDto> {
        let pack = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            &input.workspace_id,
            &input.pack_id,
        )?;

        // Other custom packs in the workspace (current pack excluded).
        let workspace_index = crate::application::card::service::CardService::new(self.state)
            .build_workspace_namespace_index(Some(&input.pack_id))
            .unwrap_or_default();
        let mut used = workspace_index.strings_by_pack.values().fold(
            PackStringNamespaceIndex::default(),
            |mut acc, item| {
                acc.extend(item);
                acc
            },
        );

        // This pack's own setname bases.
        let current_index = build_pack_strings_namespace_index(&pack.strings);
        used.extend(&current_index);

        // Standard reference bases (includes official setnames).
        let standard = SqliteStandardPackRepository::new(self.state)
            .strings_baseline()
            .unwrap_or_else(|_| self.state.standard_baseline.strings.clone());

        let mut all_bases = used.setname_bases;
        all_bases.extend(standard.setname_bases.iter().copied());

        let config = json_store::load_global_config(self.state.app_data_dir())
            .unwrap_or_else(|_| crate::domain::config::rules::default_global_config());

        let suggested = suggest_next_setname_base(
            &all_bases,
            config.setname_base_recommended_min,
            config.setname_base_recommended_max,
        );

        let warnings = if suggested.is_none() {
            vec![
                ValidationIssue::warning(
                    "pack_strings.setname_base_range_exhausted",
                    ValidationTarget::new("pack_strings").with_field("key"),
                )
                .with_param("recommended_base_min", config.setname_base_recommended_min)
                .with_param("recommended_base_max", config.setname_base_recommended_max),
            ]
        } else {
            Vec::new()
        };

        Ok(SetnameKeySuggestionDto {
            suggested_key: suggested.map(u32::from),
            warnings,
        })
    }
}

fn matches_filters(
    entry: &PackStringEntry,
    kind_filter: Option<&crate::domain::strings::model::PackStringKind>,
    key_filter: Option<u32>,
    keyword: &str,
) -> bool {
    if let Some(kind) = kind_filter {
        if &entry.kind != kind {
            return false;
        }
    }
    if let Some(key) = key_filter {
        if entry.key != key {
            return false;
        }
    }
    if keyword.is_empty() {
        return true;
    }
    entry.value.to_lowercase().contains(keyword)
}
