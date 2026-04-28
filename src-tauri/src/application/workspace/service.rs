use std::fs;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::bootstrap::AppState;
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::time::now_utc;
use crate::domain::workspace::model::{
    WorkspaceMeta, WorkspaceRegistryEntry, WorkspaceRegistryFile,
};
use crate::domain::workspace::rules::validate_workspace_meta;
use crate::infrastructure::json_store;
use crate::infrastructure::pack_locator;
use crate::runtime::sessions::WorkspaceSession;

pub struct WorkspaceService<'a> {
    state: &'a AppState,
}

impl<'a> WorkspaceService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub fn list_recent(&self) -> AppResult<WorkspaceRegistryFile> {
        json_store::load_workspace_registry(self.state.app_data_dir())
    }

    pub fn create_workspace(
        &self,
        root: &Path,
        name: &str,
        description: Option<String>,
    ) -> AppResult<WorkspaceMeta> {
        let workspace_path = root.to_path_buf();
        if workspace_path.exists()
            && workspace_path
                .read_dir()
                .map(|mut it| it.next().is_some())
                .unwrap_or(false)
        {
            return Err(AppError::new(
                "workspace.path_not_empty",
                "workspace target path must be empty or missing",
            )
            .with_detail("path", workspace_path.display().to_string()));
        }

        fs::create_dir_all(&workspace_path).map_err(|source| {
            AppError::from_io("workspace.root_create_failed", source)
                .with_detail("path", workspace_path.display().to_string())
        })?;
        json_store::ensure_workspace_layout(&workspace_path)?;

        let now = now_utc();
        let meta = WorkspaceMeta {
            id: Uuid::now_v7().to_string(),
            name: name.trim().to_string(),
            description,
            created_at: now,
            updated_at: now,
            pack_order: Vec::new(),
            last_opened_pack_id: None,
            open_pack_ids: Vec::new(),
        };

        let issues = validate_workspace_meta(&meta);
        if issues
            .iter()
            .any(|issue| matches!(issue.level, crate::domain::common::issue::IssueLevel::Error))
        {
            return Err(AppError::new(
                "workspace.validation_failed",
                "workspace contains validation errors",
            ));
        }

        json_store::save_workspace_meta(&workspace_path, &meta)?;
        self.upsert_registry_entry(WorkspaceRegistryEntry {
            workspace_id: meta.id.clone(),
            path: workspace_path,
            name_cache: Some(meta.name.clone()),
            last_opened_at: Some(now),
        })?;

        Ok(meta)
    }

    pub fn open_workspace(&self, workspace_path: &Path) -> AppResult<WorkspaceSession> {
        let meta = json_store::load_workspace_meta(workspace_path)?;
        let inventory = pack_locator::load_workspace_pack_inventory(workspace_path)?;

        let session = WorkspaceSession {
            workspace_path: workspace_path.to_path_buf(),
            meta: meta.clone(),
            pack_paths: inventory.pack_paths,
            pack_overviews: inventory.pack_overviews,
            open_pack_ids: Vec::new(),
            active_pack_id: None,
        };

        {
            let mut sessions = self.state.sessions.write().map_err(|_| {
                AppError::new(
                    "workspace.session_lock_poisoned",
                    "workspace session lock poisoned",
                )
            })?;
            sessions.set_workspace(session.clone());
        }
        self.state
            .confirmation_cache
            .write()
            .map_err(|_| {
                AppError::new(
                    "confirmation.cache_lock_poisoned",
                    "confirmation cache lock poisoned",
                )
            })?
            .clear();

        self.upsert_registry_entry(WorkspaceRegistryEntry {
            workspace_id: meta.id,
            path: workspace_path.to_path_buf(),
            name_cache: Some(meta.name),
            last_opened_at: Some(now_utc()),
        })?;

        Ok(session)
    }

    pub fn delete_workspace(
        &self,
        workspace_id: &str,
        workspace_path: &Path,
        delete_directory: bool,
    ) -> AppResult<()> {
        if delete_directory {
            self.delete_workspace_directory(workspace_path)?;
            self.remove_workspace_record(workspace_id)?;
            self.clear_current_workspace_if_matches(workspace_id, workspace_path)?;
            return Ok(());
        }

        self.remove_workspace_record(workspace_id)
    }

    pub fn remove_workspace_record(&self, workspace_id: &str) -> AppResult<()> {
        let mut registry = json_store::load_workspace_registry(self.state.app_data_dir())?;
        registry
            .workspaces
            .retain(|entry| entry.workspace_id != workspace_id);
        json_store::save_workspace_registry(self.state.app_data_dir(), &registry)
    }

    pub fn delete_workspace_directory(&self, workspace_path: &Path) -> AppResult<()> {
        if workspace_path.exists() {
            fs::remove_dir_all(workspace_path).map_err(|source| {
                AppError::from_io("workspace.delete_directory_failed", source)
                    .with_detail("path", workspace_path.display().to_string())
            })?;
        }
        Ok(())
    }

    fn upsert_registry_entry(&self, next: WorkspaceRegistryEntry) -> AppResult<()> {
        let mut registry = json_store::load_workspace_registry(self.state.app_data_dir())?;
        registry
            .workspaces
            .retain(|entry| entry.workspace_id != next.workspace_id);
        registry.workspaces.push(next);
        registry
            .workspaces
            .sort_by(|left, right| right.last_opened_at.cmp(&left.last_opened_at));
        json_store::save_workspace_registry(self.state.app_data_dir(), &registry)
    }

    pub fn current_workspace_path(&self) -> AppResult<PathBuf> {
        let sessions = self.state.sessions.read().map_err(|_| {
            AppError::new(
                "workspace.session_lock_poisoned",
                "workspace session lock poisoned",
            )
        })?;
        sessions
            .current_workspace
            .as_ref()
            .map(|session| session.workspace_path.clone())
            .ok_or_else(|| AppError::new("workspace.not_open", "no workspace is currently open"))
    }

    fn clear_current_workspace_if_matches(
        &self,
        workspace_id: &str,
        workspace_path: &Path,
    ) -> AppResult<()> {
        let mut sessions = self.state.sessions.write().map_err(|_| {
            AppError::new(
                "workspace.session_lock_poisoned",
                "workspace session lock poisoned",
            )
        })?;

        let should_clear = sessions
            .current_workspace
            .as_ref()
            .map(|session| {
                session.meta.id == workspace_id
                    || session.workspace_path.as_path() == workspace_path
            })
            .unwrap_or(false);

        if should_clear {
            sessions.clear_workspace();
        }

        drop(sessions);
        if should_clear {
            self.state
                .confirmation_cache
                .write()
                .map_err(|_| {
                    AppError::new(
                        "confirmation.cache_lock_poisoned",
                        "confirmation cache lock poisoned",
                    )
                })?
                .invalidate_workspace(workspace_id);
        }

        Ok(())
    }
}
