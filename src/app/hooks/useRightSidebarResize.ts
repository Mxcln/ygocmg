import { useState } from "react";
import type { MutableRefObject, PointerEvent as ReactPointerEvent } from "react";
import type { GlobalConfig } from "../../shared/contracts/config";
import { persistConfig } from "./persistConfig";

const RIGHT_SIDEBAR_MIN_WIDTH = 280;
const RIGHT_SIDEBAR_MAX_WIDTH = 560;
const RIGHT_SIDEBAR_DEFAULT_WIDTH = 320;

export function useRightSidebarResize(
  configRef: MutableRefObject<GlobalConfig | null>,
  setConfig: (config: GlobalConfig) => void,
) {
  const [rightSidebarWidth, setRightSidebarWidth] = useState(RIGHT_SIDEBAR_DEFAULT_WIDTH);
  const [rightSidebarCollapsed, setRightSidebarCollapsed] = useState(true);

  function beginRightSidebarResize(event: ReactPointerEvent<HTMLDivElement>) {
    if (rightSidebarCollapsed) return;
    const startX = event.clientX;
    const startWidth = rightSidebarWidth;
    let latestWidth = startWidth;
    document.body.classList.add("is-resizing-sidebar");

    const handleMove = (moveEvent: PointerEvent) => {
      // Right sidebar grows when dragging left, so subtract the delta.
      const nextWidth = Math.min(
        RIGHT_SIDEBAR_MAX_WIDTH,
        Math.max(RIGHT_SIDEBAR_MIN_WIDTH, startWidth - (moveEvent.clientX - startX)),
      );
      latestWidth = nextWidth;
      setRightSidebarWidth(nextWidth);
    };

    const handleUp = () => {
      window.removeEventListener("pointermove", handleMove);
      window.removeEventListener("pointerup", handleUp);
      window.removeEventListener("pointercancel", handleUp);
      document.body.classList.remove("is-resizing-sidebar");
      void persistConfig(configRef, { shell_right_sidebar_width: latestWidth }, setConfig);
    };

    window.addEventListener("pointermove", handleMove);
    window.addEventListener("pointerup", handleUp);
    window.addEventListener("pointercancel", handleUp);
  }

  function updateRightSidebarCollapsed(collapsed: boolean) {
    setRightSidebarCollapsed(collapsed);
    void persistConfig(configRef, { shell_right_sidebar_collapsed: collapsed }, setConfig);
  }

  function toggleRightSidebarCollapsed() {
    updateRightSidebarCollapsed(!rightSidebarCollapsed);
  }

  return {
    rightSidebarWidth,
    setRightSidebarWidth,
    rightSidebarCollapsed,
    setRightSidebarCollapsed,
    toggleRightSidebarCollapsed,
    beginRightSidebarResize,
  };
}
