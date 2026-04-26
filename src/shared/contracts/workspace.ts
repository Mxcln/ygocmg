import type { WorkspaceId, PackId } from "./common";

export interface CreateWorkspaceInput {
  path: string;
  name: string;
  description: string | null;
}

export interface OpenWorkspaceInput {
  path: string;
}

export interface WorkspaceMeta {
  id: WorkspaceId;
  name: string;
  description: string | null;
  created_at: string;
  updated_at: string;
  pack_order: PackId[];
  last_opened_pack_id: PackId | null;
}

export interface WorkspaceRegistryEntry {
  workspace_id: WorkspaceId;
  path: string;
  name_cache: string | null;
  last_opened_at: string | null;
}

export interface WorkspaceRegistryFile {
  schema_version: 1;
  workspaces: WorkspaceRegistryEntry[];
}
