use std::path::PathBuf;

use crate::bootstrap::AppState;
use crate::application::dto::card::{
    CardDetailDto, CardListPageDto, CreateCardInput, GetCardInput, ListCardsInput,
    SuggestCodeInput, UpdateCardInput,
};
use crate::application::dto::common::WriteResultDto;
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
    crate::application::workspace::service::WorkspaceService::new(state)
        .create_workspace(&path, name, description)
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
    crate::application::workspace::service::WorkspaceService::new(state)
        .delete_workspace(workspace_id, &path, delete_directory)
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
        .map(|workspace| workspace.pack_overviews.values().cloned().collect::<Vec<_>>())
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
    let query = crate::application::card::service::CardService::new(state);
    let code_context = query.build_code_context(&input.pack_id, Some(&input.card_id))?;
    let (_session, updated, warnings) = crate::application::pack::write_service::PackWriteService::new(state)
        .update_card(
            &input.workspace_id,
            &input.pack_id,
            &input.card_id,
            input.card,
            code_context,
        )?;
    let detail = query.get_card(GetCardInput {
        workspace_id: input.workspace_id,
        pack_id: input.pack_id,
        card_id: updated.id.clone(),
    })?;
    Ok(WriteResultDto::Ok { data: detail, warnings })
}

pub fn create_card(
    state: &AppState,
    input: CreateCardInput,
) -> AppResult<WriteResultDto<CardDetailDto>> {
    let query = crate::application::card::service::CardService::new(state);
    let code_context = query.build_code_context(&input.pack_id, None)?;
    let (_session, created, warnings) = crate::application::pack::write_service::PackWriteService::new(state)
        .create_card(&input.workspace_id, &input.pack_id, input.card, code_context)?;
    let detail = query.get_card(GetCardInput {
        workspace_id: input.workspace_id,
        pack_id: input.pack_id,
        card_id: created.id.clone(),
    })?;
    Ok(WriteResultDto::Ok { data: detail, warnings })
}

pub fn delete_card(state: &AppState, pack_id: &str, card_id: &str) -> AppResult<()> {
    let workspace_id = crate::application::pack::service::current_workspace_id(state)?;
    crate::application::pack::write_service::PackWriteService::new(state)
        .delete_card(&workspace_id, pack_id, card_id)
        .map(|_| ())
}

pub fn suggest_card_code(
    state: &AppState,
    input: SuggestCodeInput,
) -> AppResult<crate::application::dto::card::CodeSuggestionDto> {
    crate::application::card::service::CardService::new(state).suggest_code(input)
}
