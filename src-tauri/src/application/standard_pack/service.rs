use crate::application::dto::job::{JobAcceptedDto, JobKindDto};
use crate::application::dto::standard_pack::{
    GetStandardCardInput, ListStandardSetnamesInput, SearchStandardCardsInput,
    SearchStandardStringsInput, StandardCardDetailDto, StandardCardPageDto, StandardPackStatusDto,
    StandardSetnameEntryDto, StandardStringsPageDto,
};
use crate::application::standard_pack::repository::{
    SqliteStandardPackRepository, StandardPackRepository,
};
use crate::bootstrap::AppState;
use crate::domain::common::error::{AppError, AppResult};
use crate::infrastructure::json_store;

pub struct StandardPackService<'a> {
    state: &'a AppState,
}

impl<'a> StandardPackService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub fn get_status(&self) -> StandardPackStatusDto {
        self.repository().status()
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
        let runtime_cache = self.state.standard_pack_index_cache.clone();

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
                runtime_cache.clear()?;

                context.progress(
                    "index_ready",
                    Some(100),
                    Some("Standard index rebuilt".to_string()),
                )
            })
    }

    pub fn search_cards(&self, input: SearchStandardCardsInput) -> AppResult<StandardCardPageDto> {
        self.repository().search_cards(input)
    }

    pub fn get_card(&self, input: GetStandardCardInput) -> AppResult<StandardCardDetailDto> {
        self.repository().get_card(input)
    }

    pub fn search_strings(
        &self,
        input: SearchStandardStringsInput,
    ) -> AppResult<StandardStringsPageDto> {
        self.repository().search_strings(input)
    }

    pub fn list_setnames(
        &self,
        input: ListStandardSetnamesInput,
    ) -> AppResult<Vec<StandardSetnameEntryDto>> {
        self.repository().list_setnames(input)
    }

    fn repository(&self) -> SqliteStandardPackRepository<'_> {
        SqliteStandardPackRepository::new(self.state)
    }
}
