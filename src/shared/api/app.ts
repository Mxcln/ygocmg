import { invokeApi } from "./invoke";
import type { GlobalConfig, WorkspaceRegistryFile } from "../contracts/app";

export const appApi = {
  initialize() {
    return invokeApi<GlobalConfig>("initialize");
  },

  loadConfig() {
    return invokeApi<GlobalConfig>("load_config");
  },

  listRecentWorkspaces() {
    return invokeApi<WorkspaceRegistryFile>("list_recent_workspaces");
  },
};
