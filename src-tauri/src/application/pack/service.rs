use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use uuid::Uuid;

use crate::bootstrap::AppState;
use crate::domain::card::model::CardsFile;
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::ids::PackId;
use crate::domain::common::time::now_utc;
use crate::domain::pack::model::{PackKind, PackMetadata, PackOverview};
use crate::domain::pack::summary::{derive_pack_overview, touch_pack_metadata, validate_pack_metadata};
use crate::domain::strings::model::PackStringsFile;
use crate::domain::workspace::rules::touch_workspace;
use crate::infrastructure::json_store;
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
        author: &str,
        version: &str,
        description: Option<String>,
        display_language_order: Vec<String>,
        default_export_language: Option<String>,
    ) -> AppResult<PackMetadata> {
        let workspace_path = crate::application::workspace::service::WorkspaceService::new(self.state)
            .current_workspace_path()?;
        let now = now_utc();
        let metadata = PackMetadata {
            id: Uuid::now_v7().to_string(),
            kind: PackKind::Custom,
            name: name.trim().to_string(),
            author: author.trim().to_string(),
            version: version.trim().to_string(),
            description,
            created_at: now,
            updated_at: now,
            display_language_order,
            default_export_language,
        };

        let issues = validate_pack_metadata(&metadata);
        if issues
            .iter()
            .any(|issue| matches!(issue.level, crate::domain::common::issue::IssueLevel::Error))
        {
            return Err(AppError::new(
                "pack.validation_failed",
                "pack metadata contains validation errors",
            ));
        }

        let pack_path = json_store::pack_path(&workspace_path, &metadata.id);
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

    pub fn open_pack(&self, pack_id: &str) -> AppResult<PackSession> {
        let workspace_path = crate::application::workspace::service::WorkspaceService::new(self.state)
            .current_workspace_path()?;
        let pack_path = json_store::pack_path(&workspace_path, pack_id);
        let metadata = json_store::load_pack_metadata(&pack_path)?;
        let cards = json_store::load_cards(&pack_path)?;
        let strings = json_store::load_pack_strings(&pack_path)?;

        let session = PackSession {
            pack_path,
            metadata,
            cards,
            strings,
        };

        {
            let mut sessions = self.state.sessions.write().map_err(|_| {
                AppError::new("pack.session_lock_poisoned", "pack session lock poisoned")
            })?;
            sessions.put_pack(session.clone());
        }

        self.refresh_current_workspace_summary()?;
        self.persist_session_state(&workspace_path)?;
        Ok(session)
    }

    pub fn close_pack(&self, pack_id: &str) -> AppResult<()> {
        let workspace_path = crate::application::workspace::service::WorkspaceService::new(self.state)
            .current_workspace_path()?;

        {
            let mut sessions = self.state.sessions.write().map_err(|_| {
                AppError::new("pack.session_lock_poisoned", "pack session lock poisoned")
            })?;
            sessions.remove_pack(pack_id);
        }

        self.persist_session_state(&workspace_path)
    }

    pub fn delete_pack(&self, pack_id: &str) -> AppResult<()> {
        let workspace_path = crate::application::workspace::service::WorkspaceService::new(self.state)
            .current_workspace_path()?;
        let pack_path = json_store::pack_path(&workspace_path, pack_id);
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

        self.refresh_current_workspace_summary()
    }

    pub fn refresh_current_workspace_summary(&self) -> AppResult<()> {
        let workspace_path = crate::application::workspace::service::WorkspaceService::new(self.state)
            .current_workspace_path()?;
        let meta = json_store::load_workspace_meta(&workspace_path)?;
        let pack_overviews = load_pack_overviews(&workspace_path)?;

        let mut sessions = self.state.sessions.write().map_err(|_| {
            AppError::new("pack.session_lock_poisoned", "pack session lock poisoned")
        })?;
        if let Some(current) = &mut sessions.current_workspace {
            current.meta = meta;
            current.pack_overviews = pack_overviews;
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
}

pub fn load_pack_overviews(workspace_path: &Path) -> AppResult<BTreeMap<PackId, PackOverview>> {
    let packs_root = json_store::packs_root_path(workspace_path);
    if !packs_root.exists() {
        return Ok(BTreeMap::new());
    }

    let mut overviews = BTreeMap::new();
    for entry in fs::read_dir(&packs_root).map_err(|source| {
        AppError::from_io("pack.read_dir_failed", source)
            .with_detail("path", packs_root.display().to_string())
    })? {
        let entry = entry.map_err(|source| {
            AppError::from_io("pack.read_dir_entry_failed", source)
                .with_detail("path", packs_root.display().to_string())
        })?;
        let pack_path = entry.path();
        if !pack_path.is_dir() {
            continue;
        }
        let metadata = json_store::load_pack_metadata(&pack_path)?;
        let cards = json_store::load_cards(&pack_path).unwrap_or_default();
        overviews.insert(metadata.id.clone(), derive_pack_overview(&metadata, cards.len()));
    }

    Ok(overviews)
}

pub fn persist_open_pack(
    state: &AppState,
    pack_id: &str,
    mutator: impl FnOnce(&mut PackSession) -> AppResult<()>,
) -> AppResult<PackSession> {
    let mut sessions = state.sessions.write().map_err(|_| {
        AppError::new("pack.session_lock_poisoned", "pack session lock poisoned")
    })?;
    let session = sessions
        .open_packs
        .get_mut(pack_id)
        .ok_or_else(|| AppError::new("pack.not_open", "pack is not currently open"))?;
    mutator(session)?;
    Ok(session.clone())
}

pub fn touch_pack_and_persist(
    pack_path: &Path,
    metadata: &mut PackMetadata,
) -> AppResult<()> {
    *metadata = touch_pack_metadata(metadata, now_utc());
    json_store::save_pack_metadata(pack_path, metadata)
}

pub fn pack_file_card_count(pack_path: &Path) -> AppResult<usize> {
    let path = json_store::cards_path(pack_path);
    if !path.exists() {
        return Ok(0);
    }
    let file: CardsFile = json_store::read_json(&path)?;
    Ok(file.cards.len())
}
