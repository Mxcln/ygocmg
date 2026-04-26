use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::common::ids::{PackId, WorkspaceId};
use crate::domain::common::time::AppTimestamp;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceMeta {
    pub id: WorkspaceId,
    pub name: String,
    pub description: Option<String>,
    pub created_at: AppTimestamp,
    pub updated_at: AppTimestamp,
    pub pack_order: Vec<PackId>,
    pub last_opened_pack_id: Option<PackId>,
    #[serde(default)]
    pub open_pack_ids: Vec<PackId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceFile {
    pub schema_version: u32,
    pub data: WorkspaceMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceRegistryEntry {
    pub workspace_id: WorkspaceId,
    pub path: PathBuf,
    pub name_cache: Option<String>,
    pub last_opened_at: Option<AppTimestamp>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceRegistryFile {
    pub schema_version: u32,
    pub workspaces: Vec<WorkspaceRegistryEntry>,
}

impl Default for WorkspaceRegistryFile {
    fn default() -> Self {
        Self {
            schema_version: 1,
            workspaces: Vec::new(),
        }
    }
}
