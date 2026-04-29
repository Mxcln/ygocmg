use std::path::PathBuf;

use serde::Deserialize;
use tauri::State;

use crate::application::dto::card::{
    ConfirmCardWriteInput, CreateCardInput, DeleteCardInput, DeleteCardResultDto, GetCardInput,
    ListCardsInput, SuggestCodeInput, UpdateCardInput,
};
use crate::application::dto::common::{PreviewResultDto, WriteResultDto};
use crate::application::dto::export::{ExecuteExportBundleInput, PreviewExportBundleInput};
use crate::application::dto::import::{ExecuteImportPackInput, PreviewImportPackInput};
use crate::application::dto::job::GetJobStatusInput;
use crate::application::dto::resource::{
    CreateEmptyScriptInput, DeleteFieldImageInput, DeleteMainImageInput, DeleteScriptInput,
    ImportFieldImageInput, ImportMainImageInput, ImportScriptInput, OpenScriptExternalInput,
};
use crate::application::dto::standard_pack::{
    GetStandardCardInput, SearchStandardCardsInput, SearchStandardStringsInput,
};
use crate::application::dto::strings::{
    ConfirmPackStringRecordWriteInput, ConfirmPackStringsWriteInput, DeletePackStringsInput,
    DeletePackStringsResultDto, GetPackStringInput, ListPackStringsInput,
    RemovePackStringTranslationInput, UpsertPackStringInput, UpsertPackStringRecordInput,
};
use crate::bootstrap::AppState;
use crate::domain::common::error::AppError;
use crate::domain::config::model::GlobalConfig;

type CommandResult<T> = Result<T, AppError>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceInput {
    path: PathBuf,
    name: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenWorkspaceInput {
    path: PathBuf,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeleteWorkspaceMode {
    RemoveRecord,
    DeleteDirectory,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteWorkspaceInput {
    workspace_id: String,
    path: PathBuf,
    mode: DeleteWorkspaceMode,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePackInput {
    name: String,
    author: String,
    version: String,
    description: Option<String>,
    display_language_order: Vec<String>,
    default_export_language: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenPackInput {
    pack_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosePackInput {
    pack_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetActivePackInput {
    pack_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePackMetadataInput {
    pack_id: String,
    name: String,
    author: String,
    version: String,
    description: Option<String>,
    display_language_order: Vec<String>,
    default_export_language: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePackInput {
    pack_id: String,
}

#[tauri::command]
pub fn initialize(state: State<'_, AppState>) -> CommandResult<GlobalConfig> {
    crate::presentation::commands::app_commands::initialize(&state)
}

#[tauri::command]
pub fn load_config(state: State<'_, AppState>) -> CommandResult<GlobalConfig> {
    crate::presentation::commands::app_commands::load_config(&state)
}

#[tauri::command]
pub fn save_config(
    state: State<'_, AppState>,
    config: GlobalConfig,
) -> CommandResult<GlobalConfig> {
    crate::presentation::commands::app_commands::save_config(&state, &config)
}

#[tauri::command]
pub fn list_recent_workspaces(
    state: State<'_, AppState>,
) -> CommandResult<crate::domain::workspace::model::WorkspaceRegistryFile> {
    crate::presentation::commands::app_commands::list_recent_workspaces(&state)
}

#[tauri::command]
pub fn create_workspace(
    state: State<'_, AppState>,
    input: CreateWorkspaceInput,
) -> CommandResult<crate::domain::workspace::model::WorkspaceMeta> {
    crate::presentation::commands::app_commands::create_workspace(
        &state,
        input.path,
        &input.name,
        input.description,
    )
}

#[tauri::command]
pub fn open_workspace(
    state: State<'_, AppState>,
    input: OpenWorkspaceInput,
) -> CommandResult<crate::domain::workspace::model::WorkspaceMeta> {
    crate::presentation::commands::app_commands::open_workspace(&state, input.path)
}

#[tauri::command]
pub fn delete_workspace(
    state: State<'_, AppState>,
    input: DeleteWorkspaceInput,
) -> CommandResult<()> {
    crate::presentation::commands::app_commands::delete_workspace(
        &state,
        &input.workspace_id,
        input.path,
        matches!(input.mode, DeleteWorkspaceMode::DeleteDirectory),
    )
}

#[tauri::command]
pub fn create_pack(
    state: State<'_, AppState>,
    input: CreatePackInput,
) -> CommandResult<crate::domain::pack::model::PackMetadata> {
    crate::presentation::commands::app_commands::create_pack(
        &state,
        &input.name,
        &input.author,
        &input.version,
        input.description,
        input.display_language_order,
        input.default_export_language,
    )
}

#[tauri::command]
pub fn open_pack(
    state: State<'_, AppState>,
    input: OpenPackInput,
) -> CommandResult<crate::domain::pack::model::PackMetadata> {
    crate::presentation::commands::app_commands::open_pack(&state, &input.pack_id)
}

#[tauri::command]
pub fn close_pack(state: State<'_, AppState>, input: ClosePackInput) -> CommandResult<()> {
    crate::presentation::commands::app_commands::close_pack(&state, &input.pack_id)
}

#[tauri::command]
pub fn set_active_pack(state: State<'_, AppState>, input: SetActivePackInput) -> CommandResult<()> {
    crate::presentation::commands::app_commands::set_active_pack(&state, &input.pack_id)
}

#[tauri::command]
pub fn update_pack_metadata(
    state: State<'_, AppState>,
    input: UpdatePackMetadataInput,
) -> CommandResult<crate::domain::pack::model::PackMetadata> {
    crate::presentation::commands::app_commands::update_pack_metadata(
        &state,
        &input.pack_id,
        &input.name,
        &input.author,
        &input.version,
        input.description,
        input.display_language_order,
        input.default_export_language,
    )
}

#[tauri::command]
pub fn delete_pack(state: State<'_, AppState>, input: DeletePackInput) -> CommandResult<()> {
    crate::presentation::commands::app_commands::delete_pack(&state, &input.pack_id)
}

#[tauri::command]
pub fn list_pack_overviews(
    state: State<'_, AppState>,
) -> CommandResult<Vec<crate::domain::pack::model::PackOverview>> {
    crate::presentation::commands::app_commands::list_pack_overviews(&state)
}

#[tauri::command]
pub fn list_cards(
    state: State<'_, AppState>,
    input: ListCardsInput,
) -> CommandResult<crate::application::dto::card::CardListPageDto> {
    crate::presentation::commands::app_commands::list_cards(&state, input)
}

#[tauri::command]
pub fn get_card(
    state: State<'_, AppState>,
    input: GetCardInput,
) -> CommandResult<crate::application::dto::card::CardDetailDto> {
    crate::presentation::commands::app_commands::get_card(&state, input)
}

#[tauri::command]
pub fn create_card(
    state: State<'_, AppState>,
    input: CreateCardInput,
) -> CommandResult<WriteResultDto<crate::application::dto::card::CardDetailDto>> {
    crate::presentation::commands::app_commands::create_card(&state, input)
}

#[tauri::command]
pub fn update_card(
    state: State<'_, AppState>,
    input: UpdateCardInput,
) -> CommandResult<WriteResultDto<crate::application::dto::card::CardDetailDto>> {
    crate::presentation::commands::app_commands::update_card(&state, input)
}

#[tauri::command]
pub fn delete_card(
    state: State<'_, AppState>,
    input: DeleteCardInput,
) -> CommandResult<WriteResultDto<DeleteCardResultDto>> {
    crate::presentation::commands::app_commands::delete_card(&state, input)
}

#[tauri::command]
pub fn confirm_card_write(
    state: State<'_, AppState>,
    input: ConfirmCardWriteInput,
) -> CommandResult<crate::application::dto::card::CardDetailDto> {
    crate::presentation::commands::app_commands::confirm_card_write(&state, input)
}

#[tauri::command]
pub fn suggest_card_code(
    state: State<'_, AppState>,
    input: SuggestCodeInput,
) -> CommandResult<crate::application::dto::card::CodeSuggestionDto> {
    crate::presentation::commands::app_commands::suggest_card_code(&state, input)
}

#[tauri::command]
pub fn list_pack_strings(
    state: State<'_, AppState>,
    input: ListPackStringsInput,
) -> CommandResult<crate::application::dto::strings::PackStringsPageDto> {
    crate::presentation::commands::app_commands::list_pack_strings(&state, input)
}

#[tauri::command]
pub fn get_pack_string(
    state: State<'_, AppState>,
    input: GetPackStringInput,
) -> CommandResult<crate::application::dto::strings::PackStringRecordDetailDto> {
    crate::presentation::commands::app_commands::get_pack_string(&state, input)
}

#[tauri::command]
pub fn upsert_pack_string(
    state: State<'_, AppState>,
    input: UpsertPackStringInput,
) -> CommandResult<WriteResultDto<crate::application::dto::strings::PackStringsPageDto>> {
    crate::presentation::commands::app_commands::upsert_pack_string(&state, input)
}

#[tauri::command]
pub fn upsert_pack_string_record(
    state: State<'_, AppState>,
    input: UpsertPackStringRecordInput,
) -> CommandResult<WriteResultDto<crate::application::dto::strings::PackStringRecordDetailDto>> {
    crate::presentation::commands::app_commands::upsert_pack_string_record(&state, input)
}

#[tauri::command]
pub fn delete_pack_strings(
    state: State<'_, AppState>,
    input: DeletePackStringsInput,
) -> CommandResult<WriteResultDto<DeletePackStringsResultDto>> {
    crate::presentation::commands::app_commands::delete_pack_strings(&state, input)
}

#[tauri::command]
pub fn remove_pack_string_translation(
    state: State<'_, AppState>,
    input: RemovePackStringTranslationInput,
) -> CommandResult<WriteResultDto<DeletePackStringsResultDto>> {
    crate::presentation::commands::app_commands::remove_pack_string_translation(&state, input)
}

#[tauri::command]
pub fn confirm_pack_strings_write(
    state: State<'_, AppState>,
    input: ConfirmPackStringsWriteInput,
) -> CommandResult<crate::application::dto::strings::PackStringsPageDto> {
    crate::presentation::commands::app_commands::confirm_pack_strings_write(&state, input)
}

#[tauri::command]
pub fn confirm_pack_string_record_write(
    state: State<'_, AppState>,
    input: ConfirmPackStringRecordWriteInput,
) -> CommandResult<crate::application::dto::strings::PackStringRecordDetailDto> {
    crate::presentation::commands::app_commands::confirm_pack_string_record_write(&state, input)
}

#[tauri::command]
pub fn import_main_image(
    state: State<'_, AppState>,
    input: ImportMainImageInput,
) -> CommandResult<WriteResultDto<crate::application::dto::resource::CardAssetStateDto>> {
    crate::presentation::commands::app_commands::import_main_image(&state, input)
}

#[tauri::command]
pub fn delete_main_image(
    state: State<'_, AppState>,
    input: DeleteMainImageInput,
) -> CommandResult<WriteResultDto<crate::application::dto::resource::CardAssetStateDto>> {
    crate::presentation::commands::app_commands::delete_main_image(&state, input)
}

#[tauri::command]
pub fn import_field_image(
    state: State<'_, AppState>,
    input: ImportFieldImageInput,
) -> CommandResult<WriteResultDto<crate::application::dto::resource::CardAssetStateDto>> {
    crate::presentation::commands::app_commands::import_field_image(&state, input)
}

#[tauri::command]
pub fn delete_field_image(
    state: State<'_, AppState>,
    input: DeleteFieldImageInput,
) -> CommandResult<WriteResultDto<crate::application::dto::resource::CardAssetStateDto>> {
    crate::presentation::commands::app_commands::delete_field_image(&state, input)
}

#[tauri::command]
pub fn create_empty_script(
    state: State<'_, AppState>,
    input: CreateEmptyScriptInput,
) -> CommandResult<WriteResultDto<crate::application::dto::resource::CardAssetStateDto>> {
    crate::presentation::commands::app_commands::create_empty_script(&state, input)
}

#[tauri::command]
pub fn import_script(
    state: State<'_, AppState>,
    input: ImportScriptInput,
) -> CommandResult<WriteResultDto<crate::application::dto::resource::CardAssetStateDto>> {
    crate::presentation::commands::app_commands::import_script(&state, input)
}

#[tauri::command]
pub fn delete_script(
    state: State<'_, AppState>,
    input: DeleteScriptInput,
) -> CommandResult<WriteResultDto<crate::application::dto::resource::CardAssetStateDto>> {
    crate::presentation::commands::app_commands::delete_script(&state, input)
}

#[tauri::command]
pub fn open_script_external(
    state: State<'_, AppState>,
    input: OpenScriptExternalInput,
) -> CommandResult<()> {
    crate::presentation::commands::app_commands::open_script_external(&state, input)
}

#[tauri::command]
pub fn preview_export_bundle(
    state: State<'_, AppState>,
    input: PreviewExportBundleInput,
) -> CommandResult<PreviewResultDto<crate::application::dto::export::ExportPreviewDto>> {
    crate::presentation::commands::app_commands::preview_export_bundle(&state, input)
}

#[tauri::command]
pub fn execute_export_bundle(
    state: State<'_, AppState>,
    input: ExecuteExportBundleInput,
) -> CommandResult<crate::application::dto::job::JobAcceptedDto> {
    crate::presentation::commands::app_commands::execute_export_bundle(&state, input)
}

#[tauri::command]
pub fn preview_import_pack(
    state: State<'_, AppState>,
    input: PreviewImportPackInput,
) -> CommandResult<PreviewResultDto<crate::application::dto::import::ImportPreviewDto>> {
    crate::presentation::commands::app_commands::preview_import_pack(&state, input)
}

#[tauri::command]
pub fn execute_import_pack(
    state: State<'_, AppState>,
    input: ExecuteImportPackInput,
) -> CommandResult<crate::application::dto::job::JobAcceptedDto> {
    crate::presentation::commands::app_commands::execute_import_pack(&state, input)
}

#[tauri::command]
pub fn get_standard_pack_status(
    state: State<'_, AppState>,
) -> crate::application::dto::standard_pack::StandardPackStatusDto {
    crate::presentation::commands::app_commands::get_standard_pack_status(&state)
}

#[tauri::command]
pub fn rebuild_standard_pack_index(
    state: State<'_, AppState>,
) -> CommandResult<crate::application::dto::job::JobAcceptedDto> {
    crate::presentation::commands::app_commands::rebuild_standard_pack_index(&state)
}

#[tauri::command]
pub fn search_standard_cards(
    state: State<'_, AppState>,
    input: SearchStandardCardsInput,
) -> CommandResult<crate::application::dto::standard_pack::StandardCardPageDto> {
    crate::presentation::commands::app_commands::search_standard_cards(&state, input)
}

#[tauri::command]
pub fn search_standard_strings(
    state: State<'_, AppState>,
    input: SearchStandardStringsInput,
) -> CommandResult<crate::application::dto::standard_pack::StandardStringsPageDto> {
    crate::presentation::commands::app_commands::search_standard_strings(&state, input)
}

#[tauri::command]
pub fn get_standard_card(
    state: State<'_, AppState>,
    input: GetStandardCardInput,
) -> CommandResult<crate::application::dto::standard_pack::StandardCardDetailDto> {
    crate::presentation::commands::app_commands::get_standard_card(&state, input)
}

#[tauri::command]
pub fn get_job_status(
    state: State<'_, AppState>,
    input: GetJobStatusInput,
) -> CommandResult<crate::application::dto::job::JobSnapshotDto> {
    crate::presentation::commands::app_commands::get_job_status(&state, input)
}

#[tauri::command]
pub fn list_active_jobs(
    state: State<'_, AppState>,
) -> CommandResult<Vec<crate::application::dto::job::JobSnapshotDto>> {
    crate::presentation::commands::app_commands::list_active_jobs(&state)
}
