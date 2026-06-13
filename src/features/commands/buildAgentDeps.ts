import { useShellStore } from "../../shared/stores/shellStore";
import { queryClient } from "../../app/providers";
import type { CommandDeps } from "./types";

/** Assemble CommandDeps outside React, for agent tools. */
export function buildAgentDeps(): CommandDeps {
  const shell = useShellStore.getState();
  return {
    shell: {
      setActivePack: shell.setActivePack,
      addOpenPack: shell.addOpenPack,
      removeOpenPack: shell.removeOpenPack,
      updatePackMetadata: shell.updatePackMetadata,
      setPackOverviews: shell.setPackOverviews,
    },
    queryClient,
  };
}
