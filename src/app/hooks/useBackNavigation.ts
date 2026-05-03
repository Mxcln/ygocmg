import { useEffect, useRef } from "react";

export type BackNavigationSource = "escape" | "contextmenu";

interface BackNavigationHandler {
  id: number;
  order: number;
  priority: number;
  enabled: boolean;
  sources: readonly BackNavigationSource[];
  onBack: (source: BackNavigationSource) => void;
}

interface UseBackNavigationOptions {
  enabled?: boolean;
  priority?: number;
  sources?: readonly BackNavigationSource[];
  onBack: (source: BackNavigationSource) => void;
}

interface CloseRequestHandler {
  id: number;
  order: number;
  priority: number;
  enabled: boolean;
  onRequestClose: () => void;
}

interface UseCloseRequestOptions {
  enabled?: boolean;
  priority?: number;
  onRequestClose: () => void;
}

const handlers = new Map<number, BackNavigationHandler>();
const closeRequestHandlers = new Map<number, CloseRequestHandler>();
let nextHandlerId = 1;
let nextHandlerOrder = 1;

function hasModifierKey(event: KeyboardEvent | MouseEvent): boolean {
  return event.altKey || event.ctrlKey || event.metaKey || event.shiftKey;
}

function resolveElement(target: EventTarget | null): HTMLElement | null {
  return target instanceof HTMLElement ? target : null;
}

function allowsNativeContextMenu(target: EventTarget | null): boolean {
  const element = resolveElement(target);
  if (!element) return false;
  if (element.closest("[data-allow-native-context-menu='true']")) return true;
  if (element.closest("textarea")) return true;
  if (element.closest("select")) return true;
  const editable = element.closest("input, [contenteditable='true']");
  if (!(editable instanceof HTMLElement)) return false;
  if (editable.isContentEditable) return true;
  if (!(editable instanceof HTMLInputElement)) return false;
  return !["button", "checkbox", "color", "file", "hidden", "image", "radio", "range", "reset", "submit"].includes(editable.type);
}

function pickHandler(source: BackNavigationSource): BackNavigationHandler | null {
  let winner: BackNavigationHandler | null = null;
  for (const handler of handlers.values()) {
    if (!handler.enabled || !handler.sources.includes(source)) continue;
    if (
      !winner ||
      handler.priority > winner.priority ||
      (handler.priority === winner.priority && handler.order > winner.order)
    ) {
      winner = handler;
    }
  }
  return winner;
}

function pickCloseRequestHandler(): CloseRequestHandler | null {
  let winner: CloseRequestHandler | null = null;
  for (const handler of closeRequestHandlers.values()) {
    if (!handler.enabled) continue;
    if (
      !winner ||
      handler.priority > winner.priority ||
      (handler.priority === winner.priority && handler.order > winner.order)
    ) {
      winner = handler;
    }
  }
  return winner;
}

export function useBackNavigation({
  enabled = true,
  priority = 0,
  sources = ["escape", "contextmenu"],
  onBack,
}: UseBackNavigationOptions) {
  const idRef = useRef<number | null>(null);
  const orderRef = useRef<number | null>(null);
  const sourcesKey = sources.join("|");

  if (idRef.current === null) idRef.current = nextHandlerId++;
  if (orderRef.current === null) orderRef.current = nextHandlerOrder++;

  useEffect(() => {
    const id = idRef.current!;
    handlers.set(id, {
      id,
      order: orderRef.current!,
      priority,
      enabled,
      sources: [...sources],
      onBack,
    });
    return () => {
      handlers.delete(id);
    };
  }, [enabled, onBack, priority, sourcesKey]);
}

export function useCloseRequest({
  enabled = true,
  priority = 0,
  onRequestClose,
}: UseCloseRequestOptions) {
  const idRef = useRef<number | null>(null);
  const orderRef = useRef<number | null>(null);

  if (idRef.current === null) idRef.current = nextHandlerId++;
  if (orderRef.current === null) orderRef.current = nextHandlerOrder++;

  useEffect(() => {
    const id = idRef.current!;
    closeRequestHandlers.set(id, {
      id,
      order: orderRef.current!,
      priority,
      enabled,
      onRequestClose,
    });
    return () => {
      closeRequestHandlers.delete(id);
    };
  }, [enabled, onRequestClose, priority]);
}

export function requestClose(): boolean {
  const handler = pickCloseRequestHandler();
  if (!handler) return false;
  handler.onRequestClose();
  return true;
}

export function useGlobalBackNavigation() {
  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key !== "Escape" || event.defaultPrevented || event.repeat || hasModifierKey(event)) {
        return;
      }
      const handler = pickHandler("escape");
      if (!handler) return;
      event.preventDefault();
      handler.onBack("escape");
    }

    function handleContextMenu(event: MouseEvent) {
      if (event.defaultPrevented || event.button !== 2 || hasModifierKey(event)) {
        return;
      }
      if (allowsNativeContextMenu(event.target)) return;
      const handler = pickHandler("contextmenu");
      if (!handler) return;
      event.preventDefault();
      handler.onBack("contextmenu");
    }

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("contextmenu", handleContextMenu);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("contextmenu", handleContextMenu);
    };
  }, []);
}
