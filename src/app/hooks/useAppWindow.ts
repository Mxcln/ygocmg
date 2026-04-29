import { useEffect, useState } from "react";
import type { Dispatch, MutableRefObject, SetStateAction } from "react";
import { LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { configApi } from "../../shared/api/configApi";
import type { GlobalConfig } from "../../shared/contracts/config";
import { persistConfig } from "./persistConfig";

const WINDOW_MIN_WIDTH = 960;
const WINDOW_MIN_HEIGHT = 640;

export function useAppWindow(
  configRef: MutableRefObject<GlobalConfig | null>,
  setConfig: Dispatch<SetStateAction<GlobalConfig | null>>,
) {
  const [maximized, setMaximized] = useState(false);
  const [shellReady, setShellReady] = useState(false);

  async function syncWindowState(persist: boolean) {
    const appWindow = getCurrentWindow();
    const isMax = await appWindow.isMaximized().catch(() => false);
    setMaximized(isMax);

    if (!persist) return;

    if (isMax) {
      await persistConfig(configRef, { shell_window_is_maximized: true }, setConfig);
      return;
    }

    await persistConfig(
      configRef,
      {
        shell_window_width: Math.max(WINDOW_MIN_WIDTH, Math.round(window.innerWidth)),
        shell_window_height: Math.max(WINDOW_MIN_HEIGHT, Math.round(window.innerHeight)),
        shell_window_is_maximized: false,
      },
      setConfig,
    );
  }

  useEffect(() => {
    let cancelled = false;
    let unlistenResize: UnlistenFn | null = null;
    let saveTimer: number | null = null;

    async function bindWindowState() {
      try {
        const appWindow = getCurrentWindow();
        const currentConfig = configRef.current ?? (await configApi.loadConfig());

        if (!cancelled) {
          configRef.current = currentConfig;
          setConfig((prev) => prev ?? currentConfig);
        }

        if (currentConfig.shell_window_is_maximized) {
          await appWindow.maximize();
        } else {
          const width = Math.max(WINDOW_MIN_WIDTH, currentConfig.shell_window_width);
          const height = Math.max(WINDOW_MIN_HEIGHT, currentConfig.shell_window_height);
          await appWindow.unmaximize().catch(() => {});
          await appWindow.setSize(new LogicalSize(width, height));
        }

        if (!cancelled) {
          await syncWindowState(false);
          setShellReady(true);
        }
        unlistenResize = await appWindow.onResized(() => {
          handleViewportChanged();
        });
      } catch {
        if (!cancelled) setShellReady(true);
      }
    }

    const handleViewportChanged = () => {
      if (saveTimer !== null) {
        window.clearTimeout(saveTimer);
      }

      saveTimer = window.setTimeout(() => {
        if (!cancelled) {
          void syncWindowState(true);
        }
      }, 120);
    };

    void bindWindowState();
    window.addEventListener("resize", handleViewportChanged);
    return () => {
      cancelled = true;
      window.removeEventListener("resize", handleViewportChanged);
      if (saveTimer !== null) {
        window.clearTimeout(saveTimer);
      }
      if (unlistenResize) void unlistenResize();
    };
  }, []);

  async function handleWindowAction(action: "minimize" | "toggle-maximize" | "close") {
    const appWindow = getCurrentWindow();
    if (action === "minimize") {
      await appWindow.minimize();
      return;
    }
    if (action === "toggle-maximize") {
      if (maximized) {
        await appWindow.unmaximize();
      } else {
        await appWindow.maximize();
      }
      window.setTimeout(() => void syncWindowState(true), 120);
      return;
    }
    await syncWindowState(true);
    await appWindow.close();
  }

  return { maximized, shellReady, handleWindowAction };
}
