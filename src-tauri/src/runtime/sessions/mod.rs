use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::domain::card::model::{CardEntity, CardListRow};
use crate::domain::common::ids::{PackId, WorkspaceId};
use crate::domain::pack::model::{PackMetadata, PackOverview};
use crate::domain::resource::model::CardAssetState;
use crate::domain::strings::model::PackStringsFile;
use crate::domain::workspace::model::WorkspaceMeta;

#[derive(Debug, Clone)]
pub struct WorkspaceSession {
    pub workspace_path: PathBuf,
    pub meta: WorkspaceMeta,
    pub pack_paths: BTreeMap<PackId, PathBuf>,
    pub pack_overviews: BTreeMap<PackId, PackOverview>,
    pub open_pack_ids: Vec<PackId>,
    pub active_pack_id: Option<PackId>,
}

#[derive(Debug, Clone)]
pub struct PackSession {
    pub pack_id: PackId,
    pub pack_path: PathBuf,
    pub revision: u64,
    pub source_stamp: String,
    pub metadata: PackMetadata,
    pub cards: Vec<CardEntity>,
    pub strings: PackStringsFile,
    pub asset_index: BTreeMap<crate::domain::common::ids::CardId, CardAssetState>,
    pub card_list_cache: Vec<CardListRow>,
}

#[derive(Debug, Default)]
pub struct SessionManager {
    pub current_workspace: Option<WorkspaceSession>,
    pub open_packs: BTreeMap<PackId, PackSession>,
}

impl SessionManager {
    pub fn current_workspace_id(&self) -> Option<&WorkspaceId> {
        self.current_workspace.as_ref().map(|workspace| &workspace.meta.id)
    }

    pub fn set_workspace(&mut self, session: WorkspaceSession) {
        let previous_workspace_id = self.current_workspace_id().cloned();
        let next_workspace_id = session.meta.id.clone();

        if previous_workspace_id.as_deref() != Some(next_workspace_id.as_str()) {
            self.open_packs.clear();
        } else {
            self.open_packs
                .retain(|pack_id, _| session.pack_overviews.contains_key(pack_id));
        }

        self.current_workspace = Some(session);
    }

    pub fn put_pack(&mut self, session: PackSession) {
        let pack_id = session.metadata.id.clone();
        self.open_packs.insert(pack_id.clone(), session);
        if let Some(workspace) = &mut self.current_workspace {
            if !workspace.open_pack_ids.contains(&pack_id) {
                workspace.open_pack_ids.push(pack_id.clone());
            }
            workspace.active_pack_id = Some(pack_id.clone());
            workspace.meta.last_opened_pack_id = Some(pack_id);
        }
    }

    pub fn remove_pack(&mut self, pack_id: &str) {
        self.open_packs.remove(pack_id);
        if let Some(workspace) = &mut self.current_workspace {
            workspace.open_pack_ids.retain(|value| value != pack_id);
            if workspace.active_pack_id.as_deref() == Some(pack_id) {
                workspace.active_pack_id = workspace.open_pack_ids.last().cloned();
            }
        }
    }

    pub fn is_pack_open(&self, pack_id: &str) -> bool {
        self.open_packs.contains_key(pack_id)
    }

    pub fn clear_workspace(&mut self) {
        self.current_workspace = None;
        self.open_packs.clear();
    }
}
