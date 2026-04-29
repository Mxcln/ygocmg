import type { MutableRefObject } from "react";
import { configApi } from "../../shared/api/configApi";
import type { GlobalConfig } from "../../shared/contracts/config";

let pending: Promise<GlobalConfig> | null = null;

/**
 * Serializes concurrent config saves so that sidebar-width and window-state
 * persistence never overwrite each other.  Each call waits for any in-flight
 * save to finish, then reads the *latest* configRef snapshot before merging.
 */
export async function persistConfig(
  configRef: MutableRefObject<GlobalConfig | null>,
  patch: Partial<GlobalConfig>,
  setConfig: (config: GlobalConfig) => void,
): Promise<void> {
  if (pending) await pending.catch(() => {});

  const current = configRef.current;
  if (!current) return;

  const merged: GlobalConfig = { ...current, ...patch };

  const unchanged = (Object.keys(patch) as (keyof GlobalConfig)[]).every(
    (k) => merged[k] === current[k],
  );
  if (unchanged) return;

  pending = configApi.saveConfig(merged);
  try {
    const saved = await pending;
    configRef.current = saved;
    setConfig(saved);
  } finally {
    pending = null;
  }
}
