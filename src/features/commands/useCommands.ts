import { useQueryClient } from "@tanstack/react-query";
import { useShellStore } from "../../shared/stores/shellStore";
import type { CreatePackInput, UpdatePackMetadataInput } from "../../shared/api/packApi";
import type { CommandDeps } from "./types";
import * as packCommands from "./packCommands";

/** Build CommandDeps from React/store context. */
function useCommandDeps(): CommandDeps {
  const queryClient = useQueryClient();
  const setActivePack = useShellStore((s) => s.setActivePack);
  const addOpenPack = useShellStore((s) => s.addOpenPack);
  const removeOpenPack = useShellStore((s) => s.removeOpenPack);
  const updatePackMetadata = useShellStore((s) => s.updatePackMetadata);
  const setPackOverviews = useShellStore((s) => s.setPackOverviews);

  return {
    shell: { setActivePack, addOpenPack, removeOpenPack, updatePackMetadata, setPackOverviews },
    queryClient,
  };
}

export function useCommands() {
  const deps = useCommandDeps();
  const openDialog = useShellStore((s) => s.openDialog);
  const closeDialog = useShellStore((s) => s.closeDialog);

  return {
    switchPack: (packId: string) => packCommands.switchPack(packId, deps),
    openPack: (packId: string) => packCommands.openPack(packId, deps),
    createPack: (input: CreatePackInput) => packCommands.createPack(input, deps),
    closePack: (packId: string) => packCommands.closePack(packId, deps),
    updatePackMeta: (input: UpdatePackMetadataInput) => packCommands.updatePackMeta(input, deps),

    /** Delete a pack via the UI confirmation dialog. */
    deletePackWithDialog: async (packId: string, packName: string) => {
      const result = await packCommands.deletePack(packId, packName, deps);
      if (result.status !== "needs_confirmation") return;
      openDialog({
        kind: "confirm",
        danger: true,
        title: "Delete pack",
        message: result.confirmation.summary,
        confirmLabel: "Delete",
        cancelLabel: "Cancel",
        onConfirm: async () => {
          await result.confirmation.commit();
          closeDialog();
        },
      });
    },
  };
}
