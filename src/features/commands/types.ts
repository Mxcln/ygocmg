import type { QueryClient } from "@tanstack/react-query";
import type { PackMetadata, PackOverview } from "../../shared/contracts/pack";

/** The subset of shellStore actions the pack command layer drives. */
export interface ShellCommandActions {
  setActivePack: (id: string | null) => void;
  addOpenPack: (id: string, metadata: PackMetadata) => void;
  removeOpenPack: (id: string) => void;
  updatePackMetadata: (id: string, metadata: PackMetadata) => void;
  setPackOverviews: (overviews: PackOverview[]) => void;
}

/** Side-effect handles injected into every command. No React state lives here. */
export interface CommandDeps {
  shell: ShellCommandActions;
  queryClient: QueryClient;
}

/** A destructive operation paused for confirmation. */
export interface ConfirmationRequest {
  /** Human-readable summary of the operation. */
  summary: string;
  /** The closure that actually performs the operation once confirmed. */
  commit: () => Promise<unknown>;
}

export type CommandResult<T> =
  | { status: "ok"; data: T }
  | { status: "needs_confirmation"; confirmation: ConfirmationRequest };
