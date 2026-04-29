use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use crate::application::dto::export::PreviewExportBundleInput;
use crate::application::dto::import::PreviewImportPackInput;
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::ids::{PackId, PreviewToken, WorkspaceId};
use crate::domain::common::time::AppTimestamp;

#[derive(Debug, Clone)]
pub struct ImportPreviewEntry {
    pub preview_token: PreviewToken,
    pub workspace_id: WorkspaceId,
    pub target_pack_id: PackId,
    pub snapshot_hash: String,
    pub expires_at: AppTimestamp,
    pub input_snapshot: PreviewImportPackInput,
}

#[derive(Debug, Clone)]
pub struct ExportPreviewEntry {
    pub preview_token: PreviewToken,
    pub workspace_id: WorkspaceId,
    pub pack_ids: Vec<PackId>,
    pub snapshot_hash: String,
    pub expires_at: AppTimestamp,
    pub input_snapshot: PreviewExportBundleInput,
}

#[derive(Debug, Default)]
pub struct PreviewTokenCache {
    import_entries: BTreeMap<PreviewToken, ImportPreviewEntry>,
    export_entries: BTreeMap<PreviewToken, ExportPreviewEntry>,
}

impl PreviewTokenCache {
    pub fn insert_import_entry(&mut self, entry: ImportPreviewEntry) {
        self.import_entries
            .insert(entry.preview_token.clone(), entry);
    }

    pub fn insert_export_entry(&mut self, entry: ExportPreviewEntry) {
        self.export_entries
            .insert(entry.preview_token.clone(), entry);
    }

    pub fn remove_import_entry(&mut self, token: &PreviewToken) -> Option<ImportPreviewEntry> {
        self.import_entries.remove(token)
    }

    pub fn remove_export_entry(&mut self, token: &PreviewToken) -> Option<ExportPreviewEntry> {
        self.export_entries.remove(token)
    }

    pub fn invalidate_workspace(&mut self, workspace_id: &str) {
        self.import_entries
            .retain(|_, entry| entry.workspace_id != workspace_id);
        self.export_entries
            .retain(|_, entry| entry.workspace_id != workspace_id);
    }

    pub fn clear(&mut self) {
        self.import_entries.clear();
        self.export_entries.clear();
    }

    pub fn debug_get_import_entry(&self, token: &PreviewToken) -> Option<&ImportPreviewEntry> {
        self.import_entries.get(token)
    }

    pub fn debug_get_export_entry(&self, token: &PreviewToken) -> Option<&ExportPreviewEntry> {
        self.export_entries.get(token)
    }
}

pub type SharedPreviewTokenCache = Arc<RwLock<PreviewTokenCache>>;

pub fn read_cache(
    cache: &SharedPreviewTokenCache,
) -> AppResult<std::sync::RwLockReadGuard<'_, PreviewTokenCache>> {
    cache
        .read()
        .map_err(|_| AppError::new("preview.cache_lock_poisoned", "preview cache lock poisoned"))
}

pub fn write_cache(
    cache: &SharedPreviewTokenCache,
) -> AppResult<std::sync::RwLockWriteGuard<'_, PreviewTokenCache>> {
    cache
        .write()
        .map_err(|_| AppError::new("preview.cache_lock_poisoned", "preview cache lock poisoned"))
}
