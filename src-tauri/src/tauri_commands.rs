use std::path::PathBuf;

use serde::Deserialize;
use tauri::State;

use crate::bootstrap::AppState;
use crate::domain::card::model::CardUpdateInput;
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
pub struct ListCardsInput {
    pack_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCardInput {
    pack_id: String,
    card: CardUpdateInput,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCardInput {
    pack_id: String,
    card_id: String,
    card: CardUpdateInput,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteCardInput {
    pack_id: String,
    card_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestCardCodeInput {
    pack_id: String,
    preferred_start: Option<u32>,
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
pub fn list_pack_overviews(
    state: State<'_, AppState>,
) -> CommandResult<Vec<crate::domain::pack::model::PackOverview>> {
    crate::presentation::commands::app_commands::list_pack_overviews(&state)
}

#[tauri::command]
pub fn list_cards(
    state: State<'_, AppState>,
    input: ListCardsInput,
) -> CommandResult<Vec<crate::domain::card::model::CardListRow>> {
    crate::presentation::commands::app_commands::list_cards(&state, &input.pack_id)
}

#[tauri::command]
pub fn create_card(
    state: State<'_, AppState>,
    input: CreateCardInput,
) -> CommandResult<crate::domain::card::model::CardEntity> {
    crate::presentation::commands::app_commands::create_card(&state, &input.pack_id, input.card)
}

#[tauri::command]
pub fn update_card(
    state: State<'_, AppState>,
    input: UpdateCardInput,
) -> CommandResult<crate::domain::card::model::CardEntity> {
    crate::presentation::commands::app_commands::update_card(
        &state,
        &input.pack_id,
        &input.card_id,
        input.card,
    )
}

#[tauri::command]
pub fn delete_card(
    state: State<'_, AppState>,
    input: DeleteCardInput,
) -> CommandResult<()> {
    crate::presentation::commands::app_commands::delete_card(
        &state,
        &input.pack_id,
        &input.card_id,
    )
}

#[tauri::command]
pub fn suggest_card_code(
    state: State<'_, AppState>,
    input: SuggestCardCodeInput,
) -> CommandResult<Option<u32>> {
    crate::presentation::commands::app_commands::suggest_card_code(
        &state,
        &input.pack_id,
        input.preferred_start,
    )
}
