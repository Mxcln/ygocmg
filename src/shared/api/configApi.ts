import { invokeApi } from "./invoke";
import type { GlobalConfig } from "../contracts/config";

export const configApi = {
  initialize() {
    return invokeApi<GlobalConfig>("initialize");
  },

  loadConfig() {
    return invokeApi<GlobalConfig>("load_config");
  },

  saveConfig(config: GlobalConfig) {
    return invokeApi<GlobalConfig>("save_config", { config });
  },
};
