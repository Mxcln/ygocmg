import { packApi } from "../../shared/api/packApi";
import type { CreatePackInput, UpdatePackMetadataInput } from "../../shared/api/packApi";
import type { PackMetadata } from "../../shared/contracts/pack";
import type { CommandDeps, CommandResult } from "./types";

/** Best-effort overview refresh; failures are swallowed (mirrors App.tsx). */
async function refreshOverviews(deps: CommandDeps): Promise<void> {
  try {
    const overviews = await packApi.listPackOverviews();
    deps.shell.setPackOverviews(overviews);
  } catch {
    // overview refresh is best-effort
  }
}

export async function switchPack(packId: string, deps: CommandDeps): Promise<CommandResult<void>> {
  deps.shell.setActivePack(packId);
  await packApi.setActivePack({ packId });
  void deps.queryClient.invalidateQueries({ queryKey: ["cards"] });
  return { status: "ok", data: undefined };
}

export async function openPack(packId: string, deps: CommandDeps): Promise<CommandResult<PackMetadata>> {
  const metadata = await packApi.openPack({ packId });
  deps.shell.addOpenPack(packId, metadata);
  await refreshOverviews(deps);
  return { status: "ok", data: metadata };
}

export async function createPack(input: CreatePackInput, deps: CommandDeps): Promise<CommandResult<PackMetadata>> {
  const created = await packApi.createPack(input);
  const opened = await packApi.openPack({ packId: created.id });
  deps.shell.addOpenPack(created.id, opened);
  await refreshOverviews(deps);
  return { status: "ok", data: opened };
}

export async function closePack(packId: string, deps: CommandDeps): Promise<CommandResult<void>> {
  await packApi.closePack({ packId });
  deps.shell.removeOpenPack(packId);
  return { status: "ok", data: undefined };
}

export async function updatePackMeta(input: UpdatePackMetadataInput, deps: CommandDeps): Promise<CommandResult<PackMetadata>> {
  const updated = await packApi.updatePackMetadata(input);
  deps.shell.updatePackMetadata(input.packId, updated);
  void deps.queryClient.invalidateQueries({ queryKey: ["cards", input.packId] });
  await refreshOverviews(deps);
  return { status: "ok", data: updated };
}

export async function deletePack(
  packId: string,
  packName: string,
  deps: CommandDeps,
): Promise<CommandResult<void>> {
  return {
    status: "needs_confirmation",
    confirmation: {
      summary: `Delete pack "${packName}" (${packId}). This cannot be undone.`,
      commit: async () => {
        await packApi.deletePack({ packId });
        deps.shell.removeOpenPack(packId);
        await refreshOverviews(deps);
      },
    },
  };
}
