import { invokeApi } from "./invoke";
import type {
  CreateWorkspaceInput,
  OpenWorkspaceInput,
  WorkspaceMeta,
  WorkspaceRegistryFile,
} from "../contracts/workspace";

export const workspaceApi = {
  listRecentWorkspaces() {
    return invokeApi<WorkspaceRegistryFile>("list_recent_workspaces");
  },

  createWorkspace(input: CreateWorkspaceInput) {
    return invokeApi<WorkspaceMeta>("create_workspace", { input });
  },

  openWorkspace(input: OpenWorkspaceInput) {
    return invokeApi<WorkspaceMeta>("open_workspace", { input });
  },
};
