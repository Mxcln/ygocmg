import { useState } from "react";
import type { MutableRefObject, PointerEvent as ReactPointerEvent } from "react";
import type { GlobalConfig } from "../../shared/contracts/config";
import { persistConfig } from "./persistConfig";

const SIDEBAR_MIN_WIDTH = 140;
const SIDEBAR_MAX_WIDTH = 280;
const SIDEBAR_DEFAULT_WIDTH = 150;
export const SIDEBAR_COLLAPSED_WIDTH = 56;

export function useSidebarResize(
  configRef: MutableRefObject<GlobalConfig | null>,
  setConfig: (config: GlobalConfig) => void,
) {
  const [sidebarWidth, setSidebarWidth] = useState(SIDEBAR_DEFAULT_WIDTH);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

  function beginSidebarResize(event: ReactPointerEvent<HTMLDivElement>) {
    if (sidebarCollapsed) return;
    const startX = event.clientX;
    const startWidth = sidebarWidth;
    let latestWidth = startWidth;
    document.body.classList.add("is-resizing-sidebar");

    const handleMove = (moveEvent: PointerEvent) => {
      const nextWidth = Math.min(
        SIDEBAR_MAX_WIDTH,
        Math.max(SIDEBAR_MIN_WIDTH, startWidth + moveEvent.clientX - startX),
      );
      latestWidth = nextWidth;
      setSidebarWidth(nextWidth);
    };

    const handleUp = () => {
      window.removeEventListener("pointermove", handleMove);
      window.removeEventListener("pointerup", handleUp);
      window.removeEventListener("pointercancel", handleUp);
      document.body.classList.remove("is-resizing-sidebar");
      void persistConfig(configRef, { shell_sidebar_width: latestWidth }, setConfig);
    };

    window.addEventListener("pointermove", handleMove);
    window.addEventListener("pointerup", handleUp);
    window.addEventListener("pointercancel", handleUp);
  }

  function updateSidebarCollapsed(collapsed: boolean) {
    setSidebarCollapsed(collapsed);
    void persistConfig(configRef, { shell_sidebar_collapsed: collapsed }, setConfig);
  }

  function toggleSidebarCollapsed() {
    updateSidebarCollapsed(!sidebarCollapsed);
  }

  return {
    sidebarWidth,
    setSidebarWidth,
    sidebarCollapsed,
    setSidebarCollapsed,
    toggleSidebarCollapsed,
    beginSidebarResize,
    effectiveSidebarWidth: sidebarCollapsed ? SIDEBAR_COLLAPSED_WIDTH : sidebarWidth,
  };
}
