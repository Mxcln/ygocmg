use crate::application::dto::card::{CardListRowDto, SortDirectionDto};
use crate::application::dto::job::{JobAcceptedDto, JobKindDto};
use crate::application::dto::standard_pack::{
    GetStandardCardInput, SearchStandardCardsInput, SearchStandardStringsInput,
    StandardCardDetailDto, StandardCardPageDto, StandardCardSortFieldDto,
    StandardPackIndexStateDto, StandardPackStatusDto, StandardStringSortFieldDto,
    StandardStringsPageDto,
};
use crate::bootstrap::AppState;
use crate::domain::card::model::PrimaryType;
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::strings::model::PackStringEntry;
use crate::infrastructure::json_store;

pub struct StandardPackService<'a> {
    state: &'a AppState,
}

impl<'a> StandardPackService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub fn get_status(&self) -> StandardPackStatusDto {
        let config = json_store::load_global_config(self.state.app_data_dir()).ok();
        let status = crate::infrastructure::standard_pack::status(
            self.state.app_data_dir(),
            config
                .as_ref()
                .and_then(|config| config.ygopro_path.as_deref()),
            config
                .as_ref()
                .and_then(|config| config.standard_pack_source_language.as_deref()),
        );
        StandardPackStatusDto::from(status)
    }

    pub fn rebuild_index(&self) -> AppResult<JobAcceptedDto> {
        let config = json_store::load_global_config(self.state.app_data_dir())?;
        let ygopro_path = config.ygopro_path.ok_or_else(|| {
            AppError::new(
                "standard_pack.ygopro_path_not_configured",
                "YGOPro path is not configured",
            )
        })?;
        let source_language = config.standard_pack_source_language.ok_or_else(|| {
            AppError::new(
                "standard_pack.source_language_required",
                "standard pack source language is not configured",
            )
        })?;
        let app_data_dir = self.state.app_data_dir().to_path_buf();
        let index_cache = self.state.standard_pack_index_cache.clone();

        self.state
            .jobs
            .submit(JobKindDto::StandardPackIndexRebuild, move |context| {
                context.progress(
                    "discover_source",
                    Some(5),
                    Some("Locating standard CDB".to_string()),
                )?;
                let source = crate::infrastructure::standard_pack::discover_source(&ygopro_path)?;

                context.progress(
                    "build_index",
                    Some(20),
                    Some("Reading standard CDB".to_string()),
                )?;
                let index = crate::infrastructure::standard_pack::rebuild_index(
                    &source.ygopro_path,
                    &source_language,
                )?;

                context.progress(
                    "write_index",
                    Some(90),
                    Some("Writing standard index cache".to_string()),
                )?;
                crate::infrastructure::standard_pack::save_index(&app_data_dir, &index)?;
                context.progress(
                    "refresh_cache",
                    Some(95),
                    Some("Refreshing standard index cache".to_string()),
                )?;
                let _ = index_cache.replace(&app_data_dir, index)?;

                context.progress(
                    "index_ready",
                    Some(100),
                    Some("Standard index rebuilt".to_string()),
                )
            })
    }

    pub fn search_cards(&self, input: SearchStandardCardsInput) -> AppResult<StandardCardPageDto> {
        let snapshot = self
            .state
            .standard_pack_index_cache
            .get_or_load(self.state.app_data_dir())?;
        let index = snapshot.index();
        let keyword = input.keyword.unwrap_or_default().trim().to_lowercase();
        let mut rows = index
            .cards
            .iter()
            .filter(|record| {
                if keyword.is_empty() {
                    return true;
                }
                record.row.name.to_lowercase().contains(&keyword)
                    || record.row.desc.to_lowercase().contains(&keyword)
                    || record.row.code.to_string().contains(&keyword)
                    || record.row.subtype_display.to_lowercase().contains(&keyword)
                    || primary_type_label(&record.row.primary_type)
                        .to_lowercase()
                        .contains(&keyword)
            })
            .map(|record| CardListRowDto::from(record.row.clone()))
            .collect::<Vec<_>>();

        match input.sort_by {
            StandardCardSortFieldDto::Code => {
                rows.sort_by(|left, right| left.code.cmp(&right.code))
            }
            StandardCardSortFieldDto::Name => {
                rows.sort_by(|left, right| left.name.cmp(&right.name))
            }
            StandardCardSortFieldDto::Type => rows.sort_by(|left, right| {
                primary_type_label(&left.primary_type)
                    .cmp(primary_type_label(&right.primary_type))
                    .then(left.subtype_display.cmp(&right.subtype_display))
                    .then(left.code.cmp(&right.code))
            }),
        }

        if matches!(input.sort_direction, SortDirectionDto::Desc) {
            rows.reverse();
        }

        let page_size = input.page_size.max(1);
        let page = input.page.max(1);
        let total = rows.len() as u64;
        let start = ((page - 1) as usize).saturating_mul(page_size as usize);
        let items = if start >= rows.len() {
            Vec::new()
        } else {
            rows.into_iter()
                .skip(start)
                .take(page_size as usize)
                .collect()
        };

        Ok(StandardCardPageDto {
            items,
            page,
            page_size,
            total,
            ygopro_path: Some(index.source.ygopro_path.clone()),
            revision: index
                .indexed_at
                .timestamp_millis()
                .try_into()
                .unwrap_or_default(),
        })
    }

    pub fn get_card(&self, input: GetStandardCardInput) -> AppResult<StandardCardDetailDto> {
        let snapshot = self
            .state
            .standard_pack_index_cache
            .get_or_load(self.state.app_data_dir())?;
        let index = snapshot.index();
        let record = snapshot.card_by_code(input.code).ok_or_else(|| {
            AppError::new(
                "standard_pack.card_not_found",
                "standard card was not found",
            )
        })?;

        Ok(StandardCardDetailDto {
            card: record.card.clone().into(),
            asset_state: record.asset_state.clone(),
            available_languages: vec![index.source_language.clone()],
            ygopro_path: index.source.ygopro_path.clone(),
        })
    }

    pub fn search_strings(
        &self,
        input: SearchStandardStringsInput,
    ) -> AppResult<StandardStringsPageDto> {
        let snapshot = self
            .state
            .standard_pack_index_cache
            .get_or_load(self.state.app_data_dir())?;
        let index = snapshot.index();
        let keyword = input.keyword.unwrap_or_default().trim().to_lowercase();
        let mut rows = index
            .strings
            .records
            .iter()
            .filter(|record| {
                input
                    .kind_filter
                    .as_ref()
                    .map_or(true, |kind| &record.kind == kind)
            })
            .filter(|record| input.key_filter.map_or(true, |key| record.key == key))
            .filter_map(|record| {
                let value = record
                    .values
                    .get(&index.source_language)
                    .cloned()
                    .unwrap_or_default();
                if keyword.is_empty()
                    || value.to_lowercase().contains(&keyword)
                    || record.key.to_string().contains(&keyword)
                    || format_string_key_hex(record.key)
                        .to_lowercase()
                        .contains(&keyword)
                    || format!("{:?}", record.kind)
                        .to_lowercase()
                        .contains(&keyword)
                {
                    Some(PackStringEntry {
                        kind: record.kind.clone(),
                        key: record.key,
                        value,
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        match input.sort_by {
            StandardStringSortFieldDto::Kind => rows.sort_by(|left, right| {
                left.kind
                    .cmp(&right.kind)
                    .then(left.key.cmp(&right.key))
                    .then(left.value.cmp(&right.value))
            }),
            StandardStringSortFieldDto::Key => rows.sort_by(|left, right| {
                left.key
                    .cmp(&right.key)
                    .then(left.kind.cmp(&right.kind))
                    .then(left.value.cmp(&right.value))
            }),
            StandardStringSortFieldDto::Value => rows.sort_by(|left, right| {
                left.value
                    .cmp(&right.value)
                    .then(left.kind.cmp(&right.kind))
                    .then(left.key.cmp(&right.key))
            }),
        }

        if matches!(input.sort_direction, SortDirectionDto::Desc) {
            rows.reverse();
        }

        let page_size = input.page_size.max(1);
        let page = input.page.max(1);
        let total = rows.len() as u64;
        let start = ((page - 1) as usize).saturating_mul(page_size as usize);
        let items = if start >= rows.len() {
            Vec::new()
        } else {
            rows.into_iter()
                .skip(start)
                .take(page_size as usize)
                .map(Into::into)
                .collect()
        };

        Ok(StandardStringsPageDto {
            language: index.source_language.clone(),
            items,
            page,
            page_size,
            total,
            revision: index
                .indexed_at
                .timestamp_millis()
                .try_into()
                .unwrap_or_default(),
        })
    }
}

fn format_string_key_hex(value: u32) -> String {
    format!("{value:X}")
}

fn primary_type_label(value: &PrimaryType) -> &'static str {
    match value {
        PrimaryType::Monster => "monster",
        PrimaryType::Spell => "spell",
        PrimaryType::Trap => "trap",
    }
}

impl From<crate::infrastructure::standard_pack::StandardPackStatus> for StandardPackStatusDto {
    fn from(value: crate::infrastructure::standard_pack::StandardPackStatus) -> Self {
        let state = if !value.configured {
            StandardPackIndexStateDto::NotConfigured
        } else if !value.source_language_configured {
            StandardPackIndexStateDto::MissingLanguage
        } else if value.schema_mismatch {
            StandardPackIndexStateDto::MissingIndex
        } else if value.index_exists && value.message.is_some() {
            StandardPackIndexStateDto::MissingSource
        } else if value.message.is_some() {
            StandardPackIndexStateDto::MissingSource
        } else if !value.index_exists {
            StandardPackIndexStateDto::MissingIndex
        } else if value.stale {
            StandardPackIndexStateDto::Stale
        } else {
            StandardPackIndexStateDto::Ready
        };

        Self {
            configured: value.configured,
            ygopro_path: value
                .ygopro_path
                .map(|path| path.to_string_lossy().to_string()),
            cdb_path: value
                .cdb_path
                .map(|path| path.to_string_lossy().to_string()),
            index_exists: value.index_exists,
            schema_mismatch: value.schema_mismatch,
            stale: value.stale,
            source_language: value.source_language,
            indexed_at: value.indexed_at,
            card_count: value.card_count,
            state,
            message: value.message,
        }
    }
}
