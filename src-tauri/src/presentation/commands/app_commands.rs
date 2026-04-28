use std::path::PathBuf;

use crate::application::dto::card::{
    CardDetailDto, CardListPageDto, ConfirmCardWriteInput, CreateCardInput, DeleteCardInput,
    DeleteCardResultDto, GetCardInput, ListCardsInput, SuggestCodeInput, UpdateCardInput,
};
use crate::application::dto::common::{PreviewResultDto, WriteResultDto};
use crate::application::dto::export::{ExportPreviewDto, PreviewExportBundleInput};
use crate::application::dto::job::{GetJobStatusInput, JobSnapshotDto};
use crate::application::dto::resource::{
    CardAssetStateDto, CreateEmptyScriptInput, DeleteFieldImageInput, DeleteMainImageInput,
    DeleteScriptInput, ImportFieldImageInput, ImportMainImageInput, ImportScriptInput,
    OpenScriptExternalInput,
};
use crate::application::dto::standard_pack::{
    GetStandardCardInput, SearchStandardCardsInput, StandardCardDetailDto, StandardCardPageDto,
    StandardPackStatusDto,
};
use crate::application::dto::strings::{
    ConfirmPackStringsWriteInput, DeletePackStringsInput, DeletePackStringsResultDto,
    GetPackStringInput, ListPackStringsInput, PackStringRecordDetailDto, PackStringsPageDto,
    RemovePackStringTranslationInput, UpsertPackStringInput, UpsertPackStringRecordInput,
};
use crate::bootstrap::AppState;
use crate::domain::common::error::AppResult;
use crate::domain::config::model::GlobalConfig;
use crate::domain::pack::model::{PackMetadata, PackOverview};
use crate::domain::workspace::model::{WorkspaceMeta, WorkspaceRegistryFile};

pub fn initialize(state: &AppState) -> AppResult<GlobalConfig> {
    crate::application::config::service::ConfigService::new(state).ensure_initialized()
}

pub fn load_config(state: &AppState) -> AppResult<GlobalConfig> {
    crate::application::config::service::ConfigService::new(state).load()
}

pub fn save_config(state: &AppState, config: &GlobalConfig) -> AppResult<GlobalConfig> {
    crate::application::config::service::ConfigService::new(state).save(config)?;
    Ok(config.clone())
}

pub fn list_recent_workspaces(state: &AppState) -> AppResult<WorkspaceRegistryFile> {
    crate::application::workspace::service::WorkspaceService::new(state).list_recent()
}

pub fn create_workspace(
    state: &AppState,
    path: PathBuf,
    name: &str,
    description: Option<String>,
) -> AppResult<WorkspaceMeta> {
    crate::application::workspace::service::WorkspaceService::new(state).create_workspace(
        &path,
        name,
        description,
    )
}

pub fn open_workspace(state: &AppState, path: PathBuf) -> AppResult<WorkspaceMeta> {
    Ok(
        crate::application::workspace::service::WorkspaceService::new(state)
            .open_workspace(&path)?
            .meta,
    )
}

pub fn delete_workspace(
    state: &AppState,
    workspace_id: &str,
    path: PathBuf,
    delete_directory: bool,
) -> AppResult<()> {
    crate::application::workspace::service::WorkspaceService::new(state).delete_workspace(
        workspace_id,
        &path,
        delete_directory,
    )
}

pub fn create_pack(
    state: &AppState,
    name: &str,
    author: &str,
    version: &str,
    description: Option<String>,
    display_language_order: Vec<String>,
    default_export_language: Option<String>,
) -> AppResult<PackMetadata> {
    crate::application::pack::service::PackService::new(state).create_pack(
        name,
        author,
        version,
        description,
        display_language_order,
        default_export_language,
    )
}

pub fn open_pack(state: &AppState, pack_id: &str) -> AppResult<PackMetadata> {
    crate::application::pack::service::PackService::new(state).open_pack(pack_id)
}

pub fn close_pack(state: &AppState, pack_id: &str) -> AppResult<()> {
    crate::application::pack::service::PackService::new(state).close_pack(pack_id)
}

pub fn set_active_pack(state: &AppState, pack_id: &str) -> AppResult<()> {
    crate::application::pack::service::PackService::new(state).set_active_pack(pack_id)
}

pub fn update_pack_metadata(
    state: &AppState,
    pack_id: &str,
    name: &str,
    author: &str,
    version: &str,
    description: Option<String>,
    display_language_order: Vec<String>,
    default_export_language: Option<String>,
) -> AppResult<PackMetadata> {
    crate::application::pack::service::PackService::new(state).update_pack_metadata(
        pack_id,
        name,
        author,
        version,
        description,
        display_language_order,
        default_export_language,
    )
}

pub fn delete_pack(state: &AppState, pack_id: &str) -> AppResult<()> {
    crate::application::pack::service::PackService::new(state).delete_pack(pack_id)
}

pub fn list_pack_overviews(state: &AppState) -> AppResult<Vec<PackOverview>> {
    let sessions = state.sessions.read().map_err(|_| {
        crate::domain::common::error::AppError::new(
            "presentation.session_lock_poisoned",
            "session lock poisoned",
        )
    })?;
    let mut values = sessions
        .current_workspace
        .as_ref()
        .map(|workspace| {
            workspace
                .pack_overviews
                .values()
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    values.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(values)
}

pub fn list_cards(state: &AppState, input: ListCardsInput) -> AppResult<CardListPageDto> {
    crate::application::card::service::CardService::new(state).list_cards(input)
}

pub fn get_card(state: &AppState, input: GetCardInput) -> AppResult<CardDetailDto> {
    crate::application::card::service::CardService::new(state).get_card(input)
}

pub fn update_card(
    state: &AppState,
    input: UpdateCardInput,
) -> AppResult<WriteResultDto<CardDetailDto>> {
    crate::application::card::confirmation_service::CardWriteConfirmationService::new(state)
        .update_card(input)
}

pub fn create_card(
    state: &AppState,
    input: CreateCardInput,
) -> AppResult<WriteResultDto<CardDetailDto>> {
    crate::application::card::confirmation_service::CardWriteConfirmationService::new(state)
        .create_card(input)
}

pub fn delete_card(
    state: &AppState,
    input: DeleteCardInput,
) -> AppResult<WriteResultDto<DeleteCardResultDto>> {
    crate::application::pack::write_service::PackWriteService::new(state).delete_card(
        &input.workspace_id,
        &input.pack_id,
        &input.card_id,
    )?;
    Ok(WriteResultDto::Ok {
        data: DeleteCardResultDto {
            deleted_card_id: input.card_id,
        },
        warnings: Vec::new(),
    })
}

pub fn confirm_card_write(
    state: &AppState,
    input: ConfirmCardWriteInput,
) -> AppResult<CardDetailDto> {
    crate::application::card::confirmation_service::CardWriteConfirmationService::new(state)
        .confirm_card_write(input)
}

pub fn suggest_card_code(
    state: &AppState,
    input: SuggestCodeInput,
) -> AppResult<crate::application::dto::card::CodeSuggestionDto> {
    crate::application::card::service::CardService::new(state).suggest_code(input)
}

pub fn list_pack_strings(
    state: &AppState,
    input: ListPackStringsInput,
) -> AppResult<PackStringsPageDto> {
    crate::application::strings::service::PackStringsService::new(state).list_pack_strings(input)
}

pub fn get_pack_string(
    state: &AppState,
    input: GetPackStringInput,
) -> AppResult<PackStringRecordDetailDto> {
    crate::application::strings::service::PackStringsService::new(state).get_pack_string(input)
}

pub fn upsert_pack_string(
    state: &AppState,
    input: UpsertPackStringInput,
) -> AppResult<WriteResultDto<PackStringsPageDto>> {
    crate::application::strings::confirmation_service::PackStringsConfirmationService::new(state)
        .upsert_pack_string(input)
}

pub fn upsert_pack_string_record(
    state: &AppState,
    input: UpsertPackStringRecordInput,
) -> AppResult<WriteResultDto<PackStringRecordDetailDto>> {
    crate::application::strings::confirmation_service::PackStringsConfirmationService::new(state)
        .upsert_pack_string_record(input)
}

pub fn delete_pack_strings(
    state: &AppState,
    input: DeletePackStringsInput,
) -> AppResult<WriteResultDto<DeletePackStringsResultDto>> {
    let (_, deleted_count) = crate::application::pack::write_service::PackWriteService::new(state)
        .delete_pack_strings(&input.workspace_id, &input.pack_id, &input.entries)?;
    Ok(WriteResultDto::Ok {
        data: DeletePackStringsResultDto { deleted_count },
        warnings: Vec::new(),
    })
}

pub fn remove_pack_string_translation(
    state: &AppState,
    input: RemovePackStringTranslationInput,
) -> AppResult<WriteResultDto<DeletePackStringsResultDto>> {
    let (_, changed) = crate::application::pack::write_service::PackWriteService::new(state)
        .remove_pack_string_translation(
            &input.workspace_id,
            &input.pack_id,
            &input.kind,
            input.key,
            &input.language,
        )?;
    Ok(WriteResultDto::Ok {
        data: DeletePackStringsResultDto {
            deleted_count: usize::from(changed),
        },
        warnings: Vec::new(),
    })
}

pub fn confirm_pack_strings_write(
    state: &AppState,
    input: ConfirmPackStringsWriteInput,
) -> AppResult<PackStringsPageDto> {
    crate::application::strings::confirmation_service::PackStringsConfirmationService::new(state)
        .confirm_pack_strings_write(input)
}

pub fn import_main_image(
    state: &AppState,
    input: ImportMainImageInput,
) -> AppResult<WriteResultDto<CardAssetStateDto>> {
    crate::application::resource::service::ResourceService::new(state).import_main_image(input)
}

pub fn delete_main_image(
    state: &AppState,
    input: DeleteMainImageInput,
) -> AppResult<WriteResultDto<CardAssetStateDto>> {
    crate::application::resource::service::ResourceService::new(state).delete_main_image(input)
}

pub fn import_field_image(
    state: &AppState,
    input: ImportFieldImageInput,
) -> AppResult<WriteResultDto<CardAssetStateDto>> {
    crate::application::resource::service::ResourceService::new(state).import_field_image(input)
}

pub fn delete_field_image(
    state: &AppState,
    input: DeleteFieldImageInput,
) -> AppResult<WriteResultDto<CardAssetStateDto>> {
    crate::application::resource::service::ResourceService::new(state).delete_field_image(input)
}

pub fn create_empty_script(
    state: &AppState,
    input: CreateEmptyScriptInput,
) -> AppResult<WriteResultDto<CardAssetStateDto>> {
    crate::application::resource::service::ResourceService::new(state).create_empty_script(input)
}

pub fn import_script(
    state: &AppState,
    input: ImportScriptInput,
) -> AppResult<WriteResultDto<CardAssetStateDto>> {
    crate::application::resource::service::ResourceService::new(state).import_script(input)
}

pub fn delete_script(
    state: &AppState,
    input: DeleteScriptInput,
) -> AppResult<WriteResultDto<CardAssetStateDto>> {
    crate::application::resource::service::ResourceService::new(state).delete_script(input)
}

pub fn open_script_external(state: &AppState, input: OpenScriptExternalInput) -> AppResult<()> {
    crate::application::resource::service::ResourceService::new(state).open_script_external(input)
}

pub fn preview_export_bundle(
    state: &AppState,
    input: PreviewExportBundleInput,
) -> AppResult<PreviewResultDto<ExportPreviewDto>> {
    crate::application::export::service::ExportService::new(state).preview_export_bundle(input)
}

pub fn get_standard_pack_status(state: &AppState) -> StandardPackStatusDto {
    crate::application::standard_pack::service::StandardPackService::new(state).get_status()
}

pub fn rebuild_standard_pack_index(
    state: &AppState,
) -> AppResult<crate::application::dto::job::JobAcceptedDto> {
    crate::application::standard_pack::service::StandardPackService::new(state).rebuild_index()
}

pub fn search_standard_cards(
    state: &AppState,
    input: SearchStandardCardsInput,
) -> AppResult<StandardCardPageDto> {
    crate::application::standard_pack::service::StandardPackService::new(state).search_cards(input)
}

pub fn search_standard_strings(
    state: &AppState,
    input: crate::application::dto::standard_pack::SearchStandardStringsInput,
) -> AppResult<crate::application::dto::standard_pack::StandardStringsPageDto> {
    crate::application::standard_pack::service::StandardPackService::new(state)
        .search_strings(input)
}

pub fn get_standard_card(
    state: &AppState,
    input: GetStandardCardInput,
) -> AppResult<StandardCardDetailDto> {
    crate::application::standard_pack::service::StandardPackService::new(state).get_card(input)
}

pub fn get_job_status(state: &AppState, input: GetJobStatusInput) -> AppResult<JobSnapshotDto> {
    crate::application::jobs::service::JobService::new(state).get_job_status(input)
}

pub fn list_active_jobs(state: &AppState) -> AppResult<Vec<JobSnapshotDto>> {
    crate::application::jobs::service::JobService::new(state).list_active_jobs()
}
