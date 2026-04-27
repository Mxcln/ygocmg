use crate::application::dto::strings::{
    GetPackStringInput, ListPackStringsInput, PackStringRecordDetailDto, PackStringsPageDto,
};
use crate::bootstrap::AppState;
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::strings::model::PackStringEntry;

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
            .filter(|entry| matches_filters(entry, input.kind_filter.as_ref(), input.key_filter, &keyword))
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
            .ok_or_else(|| AppError::new("pack_strings.not_found", "pack string record was not found"))?;

        Ok(PackStringRecordDetailDto {
            record: record.into(),
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
