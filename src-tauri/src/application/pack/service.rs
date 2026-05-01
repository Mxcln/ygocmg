use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::bootstrap::AppState;
use crate::domain::card::derive::derive_card_list_row;
use crate::domain::card::model::{CardEntity, CardsFile};
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::ids::{CardId, PackId};
use crate::domain::common::issue::IssueLevel;
use crate::domain::common::time::now_utc;
use crate::domain::language::rules::{
    normalize_language_id, validate_catalog_membership, visible_catalog_ids,
};
use crate::domain::pack::model::{PackKind, PackMetadata, PackOverview};
use crate::domain::pack::summary::{touch_pack_metadata, validate_pack_metadata};
use crate::domain::resource::model::CardAssetState;
use crate::domain::resource::path_rules::detect_card_asset_state;
use crate::domain::strings::model::PackStringsFile;
use crate::domain::workspace::rules::touch_workspace;
use crate::infrastructure::json_store;
use crate::infrastructure::pack_locator::{self, WorkspacePackInventory};
use crate::runtime::sessions::PackSession;

pub struct PackService<'a> {
    state: &'a AppState,
}

impl<'a> PackService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub fn create_pack(
        &self,
        name: &str,
        pack_code: Option<String>,
        author: &str,
        version: &str,
        description: Option<String>,
        display_language_order: Vec<String>,
        default_export_language: Option<String>,
    ) -> AppResult<PackMetadata> {
        let workspace_path =
            crate::application::workspace::service::WorkspaceService::new(self.state)
                .current_workspace_path()?;
        let now = now_utc();
        let config = crate::application::config::service::ConfigService::new(self.state)
            .load()
            .unwrap_or_else(|_| crate::domain::config::rules::default_global_config());
        let display_language_order = normalize_language_list(display_language_order);
        let default_export_language = default_export_language
            .map(|value| normalize_language_id(&value))
            .filter(|value| !value.is_empty());
        let empty_existing_languages = BTreeSet::new();
        let pack_code = normalize_pack_code(pack_code);

        let metadata = PackMetadata {
            id: Uuid::now_v7().to_string(),
            kind: PackKind::Custom,
            name: name.trim().to_string(),
            pack_code,
            author: author.trim().to_string(),
            version: version.trim().to_string(),
            description,
            created_at: now,
            updated_at: now,
            display_language_order: display_language_order.clone(),
            default_export_language: default_export_language.clone(),
        };

        let mut issues = validate_pack_metadata(&metadata);
        issues.extend(validate_pack_language_ids(
            &display_language_order,
            default_export_language.as_deref(),
            &config.text_language_catalog,
            &empty_existing_languages,
        ));
        if issues
            .iter()
            .any(|issue| matches!(issue.level, IssueLevel::Error))
        {
            return Err(AppError::new(
                "pack.validation_failed",
                "pack metadata contains validation errors",
            ));
        }

        let storage_name =
            pack_locator::suggest_pack_storage_name(&workspace_path, &metadata.name, &metadata.id)?;
        let pack_path = json_store::packs_root_path(&workspace_path).join(storage_name);
        json_store::ensure_pack_layout(&pack_path)?;
        json_store::save_pack_metadata(&pack_path, &metadata)?;
        json_store::save_cards(&pack_path, &[])?;
        json_store::save_pack_strings(&pack_path, &PackStringsFile::default())?;
        self.update_workspace_meta(&workspace_path, |meta| {
            meta.pack_order.push(metadata.id.clone());
            meta.last_opened_pack_id = Some(metadata.id.clone());
        })?;
        self.refresh_current_workspace_summary()?;
        Ok(metadata)
    }

    pub fn open_pack(&self, pack_id: &str) -> AppResult<PackMetadata> {
        let workspace_path =
            crate::application::workspace::service::WorkspaceService::new(self.state)
                .current_workspace_path()?;
        let workspace_id = self.current_workspace_id()?;

        {
            let sessions = self.state.sessions.read().map_err(|_| {
                AppError::new("pack.session_lock_poisoned", "pack session lock poisoned")
            })?;
            if let Some(existing) = sessions.open_packs.get(pack_id) {
                return Ok(existing.metadata.clone());
            }
        }

        let pack_path = self.resolve_pack_path(&workspace_path, pack_id)?;
        let metadata = json_store::load_pack_metadata(&pack_path)?;
        let cards = json_store::load_cards(&pack_path)?;
        let strings = json_store::load_pack_strings(&pack_path)?;
        let session = build_pack_session(pack_path, metadata, cards, strings, 0)?;
        let response = session.metadata.clone();

        {
            let mut sessions = self.state.sessions.write().map_err(|_| {
                AppError::new("pack.session_lock_poisoned", "pack session lock poisoned")
            })?;
            ensure_workspace_matches_locked(&sessions, &workspace_id)?;
            sessions.put_pack(session.clone());
        }

        self.refresh_current_workspace_summary()?;
        self.persist_session_state(&workspace_path)?;
        self.state
            .confirmation_cache
            .write()
            .map_err(|_| {
                AppError::new(
                    "confirmation.cache_lock_poisoned",
                    "confirmation cache lock poisoned",
                )
            })?
            .invalidate_pack(&workspace_id, pack_id);
        Ok(response)
    }

    pub fn close_pack(&self, pack_id: &str) -> AppResult<()> {
        let workspace_path =
            crate::application::workspace::service::WorkspaceService::new(self.state)
                .current_workspace_path()?;

        {
            let mut sessions = self.state.sessions.write().map_err(|_| {
                AppError::new("pack.session_lock_poisoned", "pack session lock poisoned")
            })?;
            sessions.remove_pack(pack_id);
        }

        let workspace_id = self.current_workspace_id()?;
        self.state
            .confirmation_cache
            .write()
            .map_err(|_| {
                AppError::new(
                    "confirmation.cache_lock_poisoned",
                    "confirmation cache lock poisoned",
                )
            })?
            .invalidate_pack(&workspace_id, pack_id);

        self.persist_session_state(&workspace_path)
    }

    pub fn set_active_pack(&self, pack_id: &str) -> AppResult<()> {
        let workspace_path =
            crate::application::workspace::service::WorkspaceService::new(self.state)
                .current_workspace_path()?;

        {
            let mut sessions = self.state.sessions.write().map_err(|_| {
                AppError::new("pack.session_lock_poisoned", "pack session lock poisoned")
            })?;
            let workspace = sessions.current_workspace.as_mut().ok_or_else(|| {
                AppError::new("workspace.not_open", "no workspace is currently open")
            })?;

            if !workspace
                .open_pack_ids
                .iter()
                .any(|current| current == pack_id)
            {
                return Err(AppError::new("pack.not_open", "pack is not currently open"));
            }

            workspace.active_pack_id = Some(pack_id.to_string());
        }

        self.persist_session_state(&workspace_path)
    }

    pub fn update_pack_metadata(
        &self,
        pack_id: &str,
        name: &str,
        pack_code: Option<String>,
        author: &str,
        version: &str,
        description: Option<String>,
        display_language_order: Vec<String>,
        default_export_language: Option<String>,
    ) -> AppResult<PackMetadata> {
        let workspace_path =
            crate::application::workspace::service::WorkspaceService::new(self.state)
                .current_workspace_path()?;
        let pack_path = self.resolve_pack_path(&workspace_path, pack_id)?;

        let mut metadata = json_store::load_pack_metadata(&pack_path)?;
        metadata.name = name.trim().to_string();
        metadata.pack_code = normalize_pack_code(pack_code);
        metadata.author = author.trim().to_string();
        metadata.version = version.trim().to_string();
        metadata.description = description;
        let config = crate::application::config::service::ConfigService::new(self.state)
            .load()
            .unwrap_or_else(|_| crate::domain::config::rules::default_global_config());
        let existing_languages = pack_existing_languages(
            &metadata,
            &json_store::load_cards(&pack_path).unwrap_or_default(),
            &json_store::load_pack_strings(&pack_path).unwrap_or_default(),
        );
        let display_language_order = normalize_language_list(display_language_order);
        let default_export_language = default_export_language
            .map(|value| normalize_language_id(&value))
            .filter(|value| !value.is_empty());
        metadata.display_language_order = display_language_order.clone();
        metadata.default_export_language = default_export_language.clone();

        let mut issues = validate_pack_metadata(&metadata);
        issues.extend(validate_pack_language_ids(
            &display_language_order,
            default_export_language.as_deref(),
            &config.text_language_catalog,
            &existing_languages,
        ));
        if issues
            .iter()
            .any(|issue| matches!(issue.level, IssueLevel::Error))
        {
            return Err(AppError::new(
                "pack.validation_failed",
                "pack metadata contains validation errors",
            ));
        }

        metadata = touch_pack_metadata(&metadata, now_utc());
        json_store::save_pack_metadata(&pack_path, &metadata)?;

        let maybe_next_session = {
            let snapshot = try_get_open_pack_snapshot(self.state, pack_id)?;
            if let Some(snapshot) = snapshot {
                Some(build_pack_session(
                    snapshot.pack_path.clone(),
                    metadata.clone(),
                    snapshot.cards.clone(),
                    snapshot.strings.clone(),
                    snapshot.revision + 1,
                )?)
            } else {
                None
            }
        };

        if let Some(next_session) = maybe_next_session {
            replace_open_pack_session(
                self.state,
                &self.current_workspace_id()?,
                pack_id,
                next_session,
            )?;
        }

        self.refresh_current_workspace_summary()?;
        Ok(metadata)
    }

    pub fn delete_pack(&self, pack_id: &str) -> AppResult<()> {
        let workspace_path =
            crate::application::workspace::service::WorkspaceService::new(self.state)
                .current_workspace_path()?;
        let pack_path = self.resolve_pack_path(&workspace_path, pack_id)?;
        if pack_path.exists() {
            fs::remove_dir_all(&pack_path).map_err(|source| {
                AppError::from_io("pack.delete_failed", source)
                    .with_detail("path", pack_path.display().to_string())
            })?;
        }

        self.update_workspace_meta(&workspace_path, |meta| {
            meta.pack_order.retain(|current| current != pack_id);
            if meta.last_opened_pack_id.as_deref() == Some(pack_id) {
                meta.last_opened_pack_id = meta.pack_order.last().cloned();
            }
            meta.open_pack_ids.retain(|current| current != pack_id);
        })?;

        {
            let mut sessions = self.state.sessions.write().map_err(|_| {
                AppError::new("pack.session_lock_poisoned", "pack session lock poisoned")
            })?;
            sessions.remove_pack(pack_id);
        }

        let workspace_id = self.current_workspace_id()?;
        self.state
            .confirmation_cache
            .write()
            .map_err(|_| {
                AppError::new(
                    "confirmation.cache_lock_poisoned",
                    "confirmation cache lock poisoned",
                )
            })?
            .invalidate_pack(&workspace_id, pack_id);

        self.refresh_current_workspace_summary()
    }

    pub fn refresh_current_workspace_summary(&self) -> AppResult<()> {
        let workspace_path =
            crate::application::workspace::service::WorkspaceService::new(self.state)
                .current_workspace_path()?;
        let meta = json_store::load_workspace_meta(&workspace_path)?;
        let inventory = load_pack_inventory(&workspace_path)?;

        let mut sessions = self.state.sessions.write().map_err(|_| {
            AppError::new("pack.session_lock_poisoned", "pack session lock poisoned")
        })?;
        if let Some(current) = &mut sessions.current_workspace {
            current.meta = meta;
            current.pack_paths = inventory.pack_paths;
            current.pack_overviews = inventory.pack_overviews;
        }

        Ok(())
    }

    fn persist_session_state(&self, workspace_path: &Path) -> AppResult<()> {
        let (open_ids, active_id) = {
            let sessions = self.state.sessions.read().map_err(|_| {
                AppError::new("pack.session_lock_poisoned", "pack session lock poisoned")
            })?;
            let ws = sessions.current_workspace.as_ref().ok_or_else(|| {
                AppError::new("workspace.not_open", "no workspace is currently open")
            })?;
            (ws.open_pack_ids.clone(), ws.active_pack_id.clone())
        };

        self.update_workspace_meta(workspace_path, |meta| {
            meta.open_pack_ids = open_ids;
            meta.last_opened_pack_id = active_id;
        })
    }

    fn update_workspace_meta<F>(&self, workspace_path: &Path, mutator: F) -> AppResult<()>
    where
        F: FnOnce(&mut crate::domain::workspace::model::WorkspaceMeta),
    {
        let mut meta = json_store::load_workspace_meta(workspace_path)?;
        mutator(&mut meta);
        meta = touch_workspace(&meta, now_utc());
        json_store::save_workspace_meta(workspace_path, &meta)
    }

    fn resolve_pack_path(
        &self,
        workspace_path: &Path,
        pack_id: &str,
    ) -> AppResult<std::path::PathBuf> {
        let sessions = self.state.sessions.read().map_err(|_| {
            AppError::new("pack.session_lock_poisoned", "pack session lock poisoned")
        })?;
        if let Some(workspace) = &sessions.current_workspace {
            if workspace.workspace_path == workspace_path {
                return workspace
                    .pack_paths
                    .get(pack_id)
                    .cloned()
                    .ok_or_else(|| AppError::new("pack.not_found", "pack was not found"));
            }
        }

        let inventory = load_pack_inventory(workspace_path)?;
        pack_locator::resolve_pack_path(&inventory, pack_id)
    }
}

fn normalize_pack_code(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .map(|v| v.to_ascii_uppercase())
}

pub fn load_pack_inventory(workspace_path: &Path) -> AppResult<WorkspacePackInventory> {
    pack_locator::load_workspace_pack_inventory(workspace_path)
}

pub fn load_pack_overviews(workspace_path: &Path) -> AppResult<BTreeMap<PackId, PackOverview>> {
    Ok(load_pack_inventory(workspace_path)?.pack_overviews)
}

pub fn current_workspace_id(state: &AppState) -> AppResult<String> {
    let sessions = state
        .sessions
        .read()
        .map_err(|_| AppError::new("pack.session_lock_poisoned", "pack session lock poisoned"))?;
    sessions
        .current_workspace_id()
        .cloned()
        .ok_or_else(|| AppError::new("workspace.not_open", "no workspace is currently open"))
}

pub fn ensure_workspace_matches(state: &AppState, workspace_id: &str) -> AppResult<()> {
    let sessions = state
        .sessions
        .read()
        .map_err(|_| AppError::new("pack.session_lock_poisoned", "pack session lock poisoned"))?;
    ensure_workspace_matches_locked(&sessions, workspace_id)
}

pub fn require_open_pack_snapshot(
    state: &AppState,
    workspace_id: &str,
    pack_id: &str,
) -> AppResult<PackSession> {
    let sessions = state
        .sessions
        .read()
        .map_err(|_| AppError::new("pack.session_lock_poisoned", "pack session lock poisoned"))?;
    ensure_workspace_matches_locked(&sessions, workspace_id)?;
    sessions
        .open_packs
        .get(pack_id)
        .cloned()
        .ok_or_else(|| AppError::new("pack.not_open", "pack is not currently open"))
}

pub fn try_get_open_pack_snapshot(
    state: &AppState,
    pack_id: &str,
) -> AppResult<Option<PackSession>> {
    let sessions = state
        .sessions
        .read()
        .map_err(|_| AppError::new("pack.session_lock_poisoned", "pack session lock poisoned"))?;
    Ok(sessions.open_packs.get(pack_id).cloned())
}

pub fn replace_open_pack_session(
    state: &AppState,
    workspace_id: &str,
    pack_id: &str,
    session: PackSession,
) -> AppResult<()> {
    let mut sessions = state
        .sessions
        .write()
        .map_err(|_| AppError::new("pack.session_lock_poisoned", "pack session lock poisoned"))?;
    ensure_workspace_matches_locked(&sessions, workspace_id)?;
    if !sessions.open_packs.contains_key(pack_id) {
        return Err(AppError::new("pack.not_open", "pack is not currently open"));
    }
    sessions.open_packs.insert(pack_id.to_string(), session);
    Ok(())
}

pub fn pack_file_card_count(pack_path: &Path) -> AppResult<usize> {
    let path = json_store::cards_path(pack_path);
    if !path.exists() {
        return Ok(0);
    }
    let file: CardsFile = json_store::read_json(&path)?;
    Ok(file.cards.len())
}

pub fn build_pack_session(
    pack_path: PathBuf,
    metadata: PackMetadata,
    cards: Vec<CardEntity>,
    strings: PackStringsFile,
    revision: u64,
) -> AppResult<PackSession> {
    let asset_index = build_asset_index(&pack_path, &cards);
    let card_list_cache =
        build_card_list_cache(&cards, &asset_index, &metadata.display_language_order);
    let source_stamp = build_source_stamp(&pack_path, &metadata)?;

    Ok(PackSession {
        pack_id: metadata.id.clone(),
        pack_path,
        revision,
        source_stamp,
        metadata,
        cards,
        strings,
        asset_index,
        card_list_cache,
    })
}

fn build_asset_index(pack_path: &Path, cards: &[CardEntity]) -> BTreeMap<CardId, CardAssetState> {
    cards
        .iter()
        .map(|card| {
            (
                card.id.clone(),
                detect_card_asset_state(pack_path, card.code),
            )
        })
        .collect()
}

fn build_card_list_cache(
    cards: &[CardEntity],
    asset_index: &BTreeMap<CardId, CardAssetState>,
    display_language_order: &[String],
) -> Vec<crate::domain::card::model::CardListRow> {
    let mut rows = cards
        .iter()
        .map(|card| {
            let assets = asset_index.get(&card.id).cloned().unwrap_or_default();
            derive_card_list_row(card, &assets, display_language_order)
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| row.code);
    rows
}

fn build_source_stamp(pack_path: &Path, metadata: &PackMetadata) -> AppResult<String> {
    let cards_meta = file_stamp(&json_store::cards_path(pack_path))?;
    let strings_meta = file_stamp(&json_store::pack_strings_path(pack_path))?;
    Ok(format!(
        "updated_at={};cards={};strings={}",
        metadata.updated_at.to_rfc3339(),
        cards_meta,
        strings_meta
    ))
}

fn normalize_language_list(languages: Vec<String>) -> Vec<String> {
    let mut next = Vec::new();
    for language in languages {
        let normalized = normalize_language_id(&language);
        if !normalized.is_empty() && !next.iter().any(|current| current == &normalized) {
            next.push(normalized);
        }
    }
    next
}

fn validate_pack_language_ids(
    display_language_order: &[String],
    default_export_language: Option<&str>,
    catalog: &[crate::domain::language::model::TextLanguageProfile],
    existing_languages: &std::collections::BTreeSet<String>,
) -> Vec<crate::domain::common::issue::ValidationIssue> {
    let mut issues = Vec::new();
    let visible = visible_catalog_ids(catalog);
    for language in display_language_order {
        issues.extend(validate_catalog_membership(
            language,
            catalog,
            existing_languages,
            "pack",
            "display_language_order",
            "pack.display_language",
        ));
    }
    if let Some(language) = default_export_language {
        issues.extend(validate_catalog_membership(
            language,
            catalog,
            existing_languages,
            "pack",
            "default_export_language",
            "pack.default_export_language",
        ));
        if !display_language_order
            .iter()
            .any(|current| current == language)
            && visible.contains(language)
        {
            issues.push(
                crate::domain::common::issue::ValidationIssue::warning(
                    "pack.default_export_language_not_in_display_order",
                    crate::domain::common::issue::ValidationTarget::new("pack")
                        .with_field("default_export_language"),
                )
                .with_param("language", language),
            );
        }
    }
    issues
}

fn pack_existing_languages(
    metadata: &PackMetadata,
    cards: &[CardEntity],
    strings: &PackStringsFile,
) -> std::collections::BTreeSet<String> {
    let mut languages = std::collections::BTreeSet::new();
    languages.extend(metadata.display_language_order.iter().cloned());
    if let Some(language) = &metadata.default_export_language {
        languages.insert(language.clone());
    }
    for card in cards {
        languages.extend(card.texts.keys().cloned());
    }
    for record in &strings.entries {
        languages.extend(record.values.keys().cloned());
    }
    languages
}

fn file_stamp(path: &Path) -> AppResult<String> {
    if !path.exists() {
        return Ok("missing".to_string());
    }

    let metadata = fs::metadata(path).map_err(|source| {
        AppError::from_io("pack.file_metadata_failed", source)
            .with_detail("path", path.display().to_string())
    })?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    Ok(format!("{}:{}", modified, metadata.len()))
}

fn ensure_workspace_matches_locked(
    sessions: &crate::runtime::sessions::SessionManager,
    workspace_id: &str,
) -> AppResult<()> {
    let current = sessions
        .current_workspace_id()
        .ok_or_else(|| AppError::new("workspace.not_open", "no workspace is currently open"))?;
    if current != workspace_id {
        return Err(AppError::new(
            "workspace.mismatch",
            "workspace id does not match current session",
        )
        .with_detail("expected_workspace_id", current)
        .with_detail("actual_workspace_id", workspace_id));
    }
    Ok(())
}

impl<'a> PackService<'a> {
    fn current_workspace_id(&self) -> AppResult<String> {
        current_workspace_id(self.state)
    }
}
