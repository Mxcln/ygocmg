use std::path::PathBuf;

use crate::bootstrap::AppState;
use crate::domain::card::model::{CardEntity, CardListRow, CardUpdateInput};
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
    Ok(
        crate::application::pack::service::PackService::new(state)
            .open_pack(pack_id)?
            .metadata,
    )
}

pub fn close_pack(state: &AppState, pack_id: &str) -> AppResult<()> {
    crate::application::pack::service::PackService::new(state).close_pack(pack_id)
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

pub fn list_cards(state: &AppState, pack_id: &str) -> AppResult<Vec<CardListRow>> {
    crate::application::card::service::CardService::new(state).list_cards(pack_id)
}

pub fn create_card(
    state: &AppState,
    pack_id: &str,
    input: CardUpdateInput,
) -> AppResult<CardEntity> {
    Ok(crate::application::card::service::CardService::new(state)
        .create_card(pack_id, input)?
        .0)
}

pub fn update_card(
    state: &AppState,
    pack_id: &str,
    card_id: &str,
    input: CardUpdateInput,
) -> AppResult<CardEntity> {
    Ok(crate::application::card::service::CardService::new(state)
        .update_card(pack_id, card_id, input)?
        .0)
}

pub fn delete_card(state: &AppState, pack_id: &str, card_id: &str) -> AppResult<()> {
    crate::application::card::service::CardService::new(state).delete_card(pack_id, card_id)
}

pub fn suggest_card_code(
    state: &AppState,
    pack_id: &str,
    preferred_start: Option<u32>,
) -> AppResult<Option<u32>> {
    crate::application::card::service::CardService::new(state).suggest_code(pack_id, preferred_start)
}
