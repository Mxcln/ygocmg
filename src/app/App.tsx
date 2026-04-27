import { useEffect, useRef, useState, useCallback } from "react";
import type { CSSProperties, PointerEvent as ReactPointerEvent } from "react";
import { LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { useShellStore } from "../shared/stores/shellStore";
import { configApi } from "../shared/api/configApi";
import { workspaceApi } from "../shared/api/workspaceApi";
import { packApi } from "../shared/api/packApi";
import type { GlobalConfig } from "../shared/contracts/config";
import type { PackMetadata } from "../shared/contracts/pack";
import type { WorkspaceMeta, WorkspaceRegistryFile } from "../shared/contracts/workspace";
import { formatError, formatTimestamp } from "../shared/utils/format";
import { WorkspaceModal } from "../features/workspace/WorkspaceModal";
import { SettingsModal } from "../features/settings/SettingsModal";
import { AddPackModal } from "../features/pack/AddPackModal";
import { CardListPanel } from "../features/card/CardListPanel";
import { useQueryClient } from "@tanstack/react-query";
import { CardEditDrawer } from "../features/card/CardEditDrawer";
import { AppDialog } from "../features/dialogs/AppDialog";
import { StringsListPanel } from "../features/strings/StringsListPanel";

type NoticeTone = "success" | "warning" | "error";

interface Notice {
  tone: NoticeTone;
  title: string;
  detail: string;
}

interface CurrentWorkspaceRef {
  meta: WorkspaceMeta;
  path: string;
}

type PackTab = "cards" | "strings";

const SIDEBAR_MIN_WIDTH = 140;
const SIDEBAR_MAX_WIDTH = 280;
const SIDEBAR_DEFAULT_WIDTH = 150;
const WINDOW_MIN_WIDTH = 960;
const WINDOW_MIN_HEIGHT = 640;

export function App() {
  const [config, setConfig] = useState<GlobalConfig | null>(null);
  const [recentWorkspaces, setRecentWorkspaces] = useState<WorkspaceRegistryFile | null>(null);
  const [currentWorkspace, setCurrentWorkspace] = useState<CurrentWorkspaceRef | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [maximized, setMaximized] = useState(false);
  const [shellReady, setShellReady] = useState(false);
  const [metaExpanded, setMetaExpanded] = useState(false);
  const [metaEditing, setMetaEditing] = useState(false);
  const [metaDraft, setMetaDraft] = useState<{
    name: string;
    author: string;
    version: string;
    description: string;
    displayLanguageOrder: string;
    defaultExportLanguage: string;
  } | null>(null);
  const [metaSaving, setMetaSaving] = useState(false);
  const [activeTab, setActiveTab] = useState<PackTab>("cards");
  const [editingCardId, setEditingCardId] = useState<string | null>(null);
  const [isCreatingCard, setIsCreatingCard] = useState(false);
  const [sidebarWidth, setSidebarWidth] = useState(SIDEBAR_DEFAULT_WIDTH);
  const configRef = useRef<GlobalConfig | null>(null);
  const queryClient = useQueryClient();

  const cardDrawerOpen = editingCardId !== null || isCreatingCard;

  const handleEditCard = useCallback((cardId: string) => {
    setEditingCardId(cardId);
    setIsCreatingCard(false);
  }, []);

  const handleNewCard = useCallback(() => {
    setEditingCardId(null);
    setIsCreatingCard(true);
  }, []);

  const handleDrawerClose = useCallback(() => {
    setEditingCardId(null);
    setIsCreatingCard(false);
  }, []);

  const handleDrawerSaved = useCallback(() => {
    void queryClient.invalidateQueries({ queryKey: ["cards"] });
    handleDrawerClose();
  }, [queryClient, handleDrawerClose]);

  const modal = useShellStore((s) => s.modal);
  const dialog = useShellStore((s) => s.dialog);
  const openModal = useShellStore((s) => s.openModal);
  const closeModal = useShellStore((s) => s.closeModal);
  const openDialog = useShellStore((s) => s.openDialog);
  const closeDialog = useShellStore((s) => s.closeDialog);
  const openPackIds = useShellStore((s) => s.openPackIds);
  const activePackId = useShellStore((s) => s.activePackId);
  const packMetadataMap = useShellStore((s) => s.packMetadataMap);
  const setActivePack = useShellStore((s) => s.setActivePack);
  const addOpenPack = useShellStore((s) => s.addOpenPack);
  const removeOpenPack = useShellStore((s) => s.removeOpenPack);
  const updatePackMetadataInStore = useShellStore((s) => s.updatePackMetadata);
  const setPackOverviews = useShellStore((s) => s.setPackOverviews);
  const setWorkspace = useShellStore((s) => s.setWorkspace);

  useEffect(() => {
    configRef.current = config;
  }, [config]);

  function handleNotice(tone: NoticeTone, title: string, detail: string) {
    setNotice({ tone, title, detail });
  }

  async function persistActivePack(packId: string) {
    setActivePack(packId);
    setMetaExpanded(false);
    setMetaEditing(false);
    setMetaDraft(null);
    setEditingCardId(null);
    setIsCreatingCard(false);
    try {
      await packApi.setActivePack({ packId });
    } catch (err) {
      handleNotice("error", "Failed to switch pack", formatError(err));
    }
  }

  async function persistSidebarWidth(nextWidth: number) {
    const currentConfig = configRef.current;
    if (!currentConfig || currentConfig.shell_sidebar_width === nextWidth) return;

    try {
      const nextConfig = await configApi.saveConfig({
        ...currentConfig,
        shell_sidebar_width: nextWidth,
      });
      configRef.current = nextConfig;
      setConfig(nextConfig);
    } catch {
      // keep resizing usable even if config persistence fails
    }
  }

  async function persistWindowState(partial: Partial<GlobalConfig>) {
    const currentConfig = configRef.current;
    if (!currentConfig) return;

    const nextConfig: GlobalConfig = {
      ...currentConfig,
      ...partial,
    };
    const unchanged =
      nextConfig.shell_window_width === currentConfig.shell_window_width &&
      nextConfig.shell_window_height === currentConfig.shell_window_height &&
      nextConfig.shell_window_is_maximized === currentConfig.shell_window_is_maximized;

    if (unchanged) return;

    try {
      const savedConfig = await configApi.saveConfig(nextConfig);
      configRef.current = savedConfig;
      setConfig(savedConfig);
    } catch (err) {
      console.error("Failed to persist window state", err);
    }
  }

  async function syncWindowState(persist: boolean) {
    const appWindow = getCurrentWindow();
    const isMax = await appWindow.isMaximized().catch(() => false);
    setMaximized(isMax);

    if (!persist) return;

    if (isMax) {
      await persistWindowState({ shell_window_is_maximized: true });
      return;
    }

    await persistWindowState({
      shell_window_width: Math.max(WINDOW_MIN_WIDTH, Math.round(window.innerWidth)),
      shell_window_height: Math.max(WINDOW_MIN_HEIGHT, Math.round(window.innerHeight)),
      shell_window_is_maximized: false,
    });
  }

  function beginSidebarResize(event: ReactPointerEvent<HTMLDivElement>) {
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
      void persistSidebarWidth(latestWidth);
    };

    window.addEventListener("pointermove", handleMove);
    window.addEventListener("pointerup", handleUp);
    window.addEventListener("pointercancel", handleUp);
  }

  useEffect(() => {
    let active = true;

    async function bootstrap() {
      try {
        const nextConfig = await configApi.initialize();
        const nextRecent = await workspaceApi.listRecentWorkspaces();
        if (!active) return;
        setConfig(nextConfig);
        setSidebarWidth(nextConfig.shell_sidebar_width);
        setRecentWorkspaces(nextRecent);

        await tryRestoreLastSession(nextRecent);
      } catch (err) {
        if (!active) return;
        setError(formatError(err));
      } finally {
        if (active) setLoading(false);
      }
    }

    async function tryRestoreLastSession(registry: WorkspaceRegistryFile) {
      const sorted = [...registry.workspaces]
        .filter((ws) => ws.last_opened_at)
        .sort((a, b) => (b.last_opened_at ?? "").localeCompare(a.last_opened_at ?? ""));
      const lastEntry = sorted[0];
      if (!lastEntry) return;

      try {
        const meta = await workspaceApi.openWorkspace({ path: lastEntry.path });
        if (!active) return;
        setCurrentWorkspace({ meta, path: lastEntry.path });
        setWorkspace(meta.id, meta.name, lastEntry.path);

        const overviews = await packApi.listPackOverviews();
        if (!active) return;
        setPackOverviews(overviews);

        await restorePackSession(meta.open_pack_ids, meta.last_opened_pack_id);
      } catch {
        // workspace may have been deleted or moved; silently skip
      }
    }

    async function restorePackSession(savedPackIds: string[], lastActiveId: string | null) {
      if (savedPackIds.length === 0) return;

      for (const packId of savedPackIds) {
        if (!active) return;
        try {
          const meta = await packApi.openPack({ packId });
          addOpenPack(packId, meta);
        } catch {
          // pack may have been deleted; skip it
        }
      }

      if (!active) return;
      if (lastActiveId && savedPackIds.includes(lastActiveId)) {
        await persistActivePack(lastActiveId);
      }
    }

    void bootstrap();
    return () => {
      active = false;
    };
  }, []);

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

  async function handleWorkspaceOpened(meta: WorkspaceMeta, path: string) {
    setCurrentWorkspace({ meta, path });
    setWorkspace(meta.id, meta.name, path);
    try {
      const overviews = await packApi.listPackOverviews();
      setPackOverviews(overviews);
    } catch {
      // overviews will remain empty
    }

    for (const packId of meta.open_pack_ids) {
      try {
        const packMeta = await packApi.openPack({ packId });
        addOpenPack(packId, packMeta);
      } catch {
        // pack may no longer exist; skip
      }
    }

    if (meta.last_opened_pack_id && meta.open_pack_ids.includes(meta.last_opened_pack_id)) {
      await persistActivePack(meta.last_opened_pack_id);
    }
  }

  async function handlePackOpened(packId: string, metadata: PackMetadata) {
    addOpenPack(packId, metadata);
    try {
      const overviews = await packApi.listPackOverviews();
      setPackOverviews(overviews);
    } catch {
      // overviews refresh is best-effort
    }
  }

  async function handlePackCreated(packId: string, metadata: PackMetadata) {
    addOpenPack(packId, metadata);
    try {
      const overviews = await packApi.listPackOverviews();
      setPackOverviews(overviews);
    } catch {
      // overviews refresh is best-effort
    }
  }

  async function handleClosePack(packId: string) {
    try {
      await packApi.closePack({ packId });
      removeOpenPack(packId);
    } catch (err) {
      handleNotice("error", "Failed to close pack", formatError(err));
    }
  }

  function handleStartEditMeta() {
    if (!activeMeta) return;
    setMetaDraft({
      name: activeMeta.name,
      author: activeMeta.author,
      version: activeMeta.version,
      description: activeMeta.description || "",
      displayLanguageOrder: activeMeta.display_language_order.join(", "),
      defaultExportLanguage: activeMeta.default_export_language || "",
    });
    setMetaEditing(true);
  }

  function handleCancelEditMeta() {
    setMetaEditing(false);
    setMetaDraft(null);
  }

  async function handleSavePackMetadata() {
    if (!activePackId || !metaDraft) return;
    const trimmedName = metaDraft.name.trim();
    if (!trimmedName) {
      handleNotice("error", "Validation Error", "Pack name cannot be empty.");
      return;
    }

    setMetaSaving(true);
    try {
      const langList = metaDraft.displayLanguageOrder
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean);

      const updated = await packApi.updatePackMetadata({
        packId: activePackId,
        name: trimmedName,
        author: metaDraft.author.trim(),
        version: metaDraft.version.trim(),
        description: metaDraft.description.trim() || null,
        displayLanguageOrder: langList,
        defaultExportLanguage: metaDraft.defaultExportLanguage.trim() || null,
      });

      updatePackMetadataInStore(activePackId, updated);
      const overviews = await packApi.listPackOverviews();
      setPackOverviews(overviews);
      setMetaEditing(false);
      setMetaDraft(null);
      handleNotice("success", "Metadata Saved", "Pack metadata has been updated.");
    } catch (err) {
      handleNotice("error", "Failed to save metadata", formatError(err));
    } finally {
      setMetaSaving(false);
    }
  }

  async function handleDeletePack(packId: string) {
    try {
      await packApi.deletePack({ packId });
      closeDialog();
      removeOpenPack(packId);
      const overviews = await packApi.listPackOverviews();
      setPackOverviews(overviews);
      handleNotice("success", "Pack deleted", "The pack has been removed from the workspace.");
    } catch (err) {
      handleNotice("error", "Failed to delete pack", formatError(err));
    }
  }

  if (loading) {
    return (
      <main className="launch-shell">
        <p className="launch-text">Loading...</p>
      </main>
    );
  }

  if (error || !config || !recentWorkspaces) {
    return (
      <main className="launch-shell">
        <div className="launch-error">
          <p>{error ?? "Initialization failed."}</p>
          <button className="btn btn-primary" type="button" onClick={() => window.location.reload()}>
            Reload
          </button>
        </div>
      </main>
    );
  }

  const workspaceName = currentWorkspace?.meta.name ?? "No Workspace Open";
  const activeMeta = activePackId ? packMetadataMap[activePackId] : null;
  const shellStyle = { "--sidebar-width": `${sidebarWidth}px` } as CSSProperties;
  const preferredTextLanguages = activeMeta?.display_language_order.join(", ") || "—";
  const summaryDetail = activeMeta
    ? `${activeMeta.author} · v${activeMeta.version} · ${preferredTextLanguages}`
    : "Loading metadata...";

  return (
    <div className={`app-shell ${shellReady ? "ready" : ""}`} style={shellStyle}>
      <header
        className="titlebar"
        data-tauri-drag-region
        onDoubleClick={() => void handleWindowAction("toggle-maximize")}
      >
        <div className="titlebar-left" data-tauri-drag-region>
          <span className="app-icon">Y</span>
          <span className="titlebar-name">{workspaceName}</span>
        </div>
        <div className="titlebar-spacer" data-tauri-drag-region />
        <div className="window-controls">
          <button type="button" className="win-btn" aria-label="Minimize" onClick={() => void handleWindowAction("minimize")}>
            <svg width="10" height="1" viewBox="0 0 10 1">
              <rect width="10" height="1" fill="currentColor" />
            </svg>
          </button>
          <button type="button" className="win-btn" aria-label={maximized ? "Restore" : "Maximize"} onClick={() => void handleWindowAction("toggle-maximize")}>
            {maximized ? (
              <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1">
                <rect x="2" y="0" width="8" height="8" rx="0.5" />
                <rect x="0" y="2" width="8" height="8" rx="0.5" fill="var(--panel)" />
              </svg>
            ) : (
              <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1">
                <rect x="0.5" y="0.5" width="9" height="9" rx="0.5" />
              </svg>
            )}
          </button>
          <button type="button" className="win-btn win-close" aria-label="Close" onClick={() => void handleWindowAction("close")}>
            <svg width="10" height="10" viewBox="0 0 10 10" stroke="currentColor" strokeWidth="1.2">
              <line x1="0" y1="0" x2="10" y2="10" />
              <line x1="10" y1="0" x2="0" y2="10" />
            </svg>
          </button>
        </div>
      </header>

      <div className="shell-body">
        <aside className="sidebar">
          <div className="sidebar-actions">
            <button
              type="button"
              className={`action-btn ${modal?.type === "workspace" ? "active" : ""}`}
              title="Workspace"
              onClick={() => openModal("workspace")}
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3">
                <rect x="1" y="2" width="14" height="12" rx="1.5" />
                <line x1="1" y1="6" x2="15" y2="6" />
              </svg>
            </button>
            <button type="button" className="action-btn" title="Export Expansions" disabled>
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3">
                <path d="M2 11v3h12v-3" />
                <path d="M8 2v8" />
                <path d="M5 7l3 3 3-3" />
              </svg>
            </button>
            <button
              type="button"
              className={`action-btn ${modal?.type === "settings" ? "active" : ""}`}
              title="Global Settings"
              onClick={() => openModal("settings")}
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3">
                <circle cx="8" cy="8" r="2.5" />
                <path d="M8 1v2m0 10v2M1 8h2m10 0h2M2.9 2.9l1.5 1.5m7.2 7.2l1.5 1.5M13.1 2.9l-1.5 1.5M4.4 11.6l-1.5 1.5" />
              </svg>
            </button>
          </div>

          <div className="pack-list">
            {openPackIds.map((packId) => {
              const meta = packMetadataMap[packId];
              return (
                <div key={packId} className={`pack-item-row ${activePackId === packId ? "active" : ""}`}>
                  <button
                    type="button"
                    className="pack-item-name"
                    onClick={() => void persistActivePack(packId)}
                    title={meta?.name ?? packId}
                  >
                    {meta?.name ?? packId}
                  </button>
                  <button
                    type="button"
                    className="pack-close-btn"
                    title="Close pack"
                    onClick={(event) => {
                      event.stopPropagation();
                      void handleClosePack(packId);
                    }}
                  >
                    <svg width="8" height="8" viewBox="0 0 8 8" stroke="currentColor" strokeWidth="1.2">
                      <line x1="0" y1="0" x2="8" y2="8" />
                      <line x1="8" y1="0" x2="0" y2="8" />
                    </svg>
                  </button>
                </div>
              );
            })}

            <button
              type="button"
              className="pack-item pack-add"
              onClick={() => openModal("addPack")}
              disabled={!currentWorkspace}
              title={currentWorkspace ? "Open or create a pack" : "Open a workspace first"}
            >
              +
            </button>
          </div>

          <div className="sidebar-bottom">
            <button type="button" className="pack-item pack-standard" disabled>
              Standard Pack
            </button>
          </div>

          <div
            className="sidebar-resize-handle"
            role="separator"
            aria-orientation="vertical"
            aria-label="Resize sidebar"
            onPointerDown={beginSidebarResize}
          />
        </aside>

        <section className="work-area">
          {notice && (
            <div className={`notice ${notice.tone}`}>
              <strong>{notice.title}</strong>
              <span>{notice.detail}</span>
              <button type="button" className="notice-close" onClick={() => setNotice(null)}>
                ×
              </button>
            </div>
          )}

          {!activePackId ? (
            <div className="empty-state">
              <p className="empty-label">No Pack Open</p>
              <p className="empty-hint">
                {currentWorkspace
                  ? "Use the + button in the sidebar to open or create a pack."
                  : "Open a workspace first, then add packs from the sidebar."}
              </p>
            </div>
          ) : (
            <>
              <div className="meta-bar">
                <div className="meta-summary">
                  <strong className="meta-pack-name" title={activeMeta?.name ?? activePackId}>
                    {activeMeta?.name ?? activePackId}
                  </strong>
                  <span className="meta-detail" title={summaryDetail}>
                    {summaryDetail}
                  </span>
                </div>
                <button
                  type="button"
                  className="meta-toggle"
                  onClick={() => setMetaExpanded(!metaExpanded)}
                  aria-label={metaExpanded ? "Collapse metadata" : "Expand metadata"}
                >
                  <svg
                    width="12"
                    height="12"
                    viewBox="0 0 12 12"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1.5"
                    style={{ transform: metaExpanded ? "rotate(180deg)" : "none", transition: "transform 150ms" }}
                  >
                    <path d="M2 4l4 4 4-4" />
                  </svg>
                </button>
              </div>

              <div className="work-area-content">
                {metaExpanded && activeMeta && (
                  <>
                    <div
                      className="meta-drawer-backdrop"
                      onClick={() => {
                        if (!metaEditing) {
                          setMetaExpanded(false);
                        }
                      }}
                    />
                    <div className="meta-expanded">
                      {metaEditing && metaDraft ? (
                        <>
                          <div className="meta-grid">
                            <div className="meta-field">
                              <span className="meta-field-label">Name</span>
                              <input
                                className="meta-edit-input"
                                value={metaDraft.name}
                                onChange={(e) => setMetaDraft({ ...metaDraft, name: e.target.value })}
                              />
                            </div>
                            <div className="meta-field">
                              <span className="meta-field-label">Author</span>
                              <input
                                className="meta-edit-input"
                                value={metaDraft.author}
                                onChange={(e) => setMetaDraft({ ...metaDraft, author: e.target.value })}
                              />
                            </div>
                            <div className="meta-field">
                              <span className="meta-field-label">Version</span>
                              <input
                                className="meta-edit-input"
                                value={metaDraft.version}
                                onChange={(e) => setMetaDraft({ ...metaDraft, version: e.target.value })}
                              />
                            </div>
                            <div className="meta-field">
                              <span className="meta-field-label">Preferred Text Languages</span>
                              <input
                                className="meta-edit-input"
                                value={metaDraft.displayLanguageOrder}
                                onChange={(e) => setMetaDraft({ ...metaDraft, displayLanguageOrder: e.target.value })}
                                placeholder="e.g. zh-CN, en-US"
                              />
                            </div>
                            <div className="meta-field">
                              <span className="meta-field-label">Default Export Language</span>
                              <input
                                className="meta-edit-input"
                                value={metaDraft.defaultExportLanguage}
                                onChange={(e) => setMetaDraft({ ...metaDraft, defaultExportLanguage: e.target.value })}
                                placeholder="e.g. zh-CN"
                              />
                            </div>
                            <div className="meta-field">
                              <span className="meta-field-label">Created</span>
                              <span className="meta-field-value">{formatTimestamp(activeMeta.created_at)}</span>
                            </div>
                            <div className="meta-field">
                              <span className="meta-field-label">Updated</span>
                              <span className="meta-field-value">{formatTimestamp(activeMeta.updated_at)}</span>
                            </div>
                            <div className="meta-field meta-field-wide">
                              <span className="meta-field-label">Description</span>
                              <textarea
                                className="meta-edit-input"
                                value={metaDraft.description}
                                onChange={(e) => setMetaDraft({ ...metaDraft, description: e.target.value })}
                                rows={3}
                              />
                            </div>
                          </div>
                          <div className="meta-actions">
                            <button
                              type="button"
                              className="primary-button"
                              onClick={() => void handleSavePackMetadata()}
                              disabled={metaSaving}
                            >
                              {metaSaving ? "Saving..." : "Save"}
                            </button>
                            <button
                              type="button"
                              className="ghost-button"
                              onClick={handleCancelEditMeta}
                              disabled={metaSaving}
                            >
                              Cancel
                            </button>
                          </div>
                        </>
                      ) : (
                        <>
                          <div className="meta-grid">
                            <div className="meta-field">
                              <span className="meta-field-label">Name</span>
                              <span className="meta-field-value meta-field-value-inline" title={activeMeta.name}>
                                {activeMeta.name}
                              </span>
                            </div>
                            <div className="meta-field">
                              <span className="meta-field-label">Author</span>
                              <span className="meta-field-value meta-field-value-inline" title={activeMeta.author}>
                                {activeMeta.author}
                              </span>
                            </div>
                            <div className="meta-field">
                              <span className="meta-field-label">Version</span>
                              <span className="meta-field-value meta-field-value-inline" title={activeMeta.version}>
                                {activeMeta.version}
                              </span>
                            </div>
                            <div className="meta-field">
                              <span className="meta-field-label">Preferred Text Languages</span>
                              <span className="meta-field-value" title={preferredTextLanguages}>
                                {preferredTextLanguages}
                              </span>
                            </div>
                            <div className="meta-field">
                              <span className="meta-field-label">Default Export Language</span>
                              <span
                                className="meta-field-value meta-field-value-inline"
                                title={activeMeta.default_export_language || "—"}
                              >
                                {activeMeta.default_export_language || "—"}
                              </span>
                            </div>
                            <div className="meta-field">
                              <span className="meta-field-label">Created</span>
                              <span className="meta-field-value">{formatTimestamp(activeMeta.created_at)}</span>
                            </div>
                            <div className="meta-field">
                              <span className="meta-field-label">Updated</span>
                              <span className="meta-field-value">{formatTimestamp(activeMeta.updated_at)}</span>
                            </div>
                            <div className="meta-field meta-field-wide">
                              <span className="meta-field-label">Description</span>
                              <span
                                className="meta-field-value meta-field-value-description"
                                title={activeMeta.description || "—"}
                              >
                                {activeMeta.description || "—"}
                              </span>
                            </div>
                          </div>
                          <div className="meta-actions">
                            <button
                              type="button"
                              className="ghost-button"
                              onClick={handleStartEditMeta}
                            >
                              Edit
                            </button>
                            <button
                              type="button"
                              className="ghost-button danger-ghost"
                              onClick={() => {
                                const packId = activePackId!;
                                openDialog({
                                  kind: "confirm",
                                  title: "Delete pack",
                                  message: `Delete pack "${activeMeta.name}"? This cannot be undone.`,
                                  confirmLabel: "Delete Pack",
                                  cancelLabel: "Cancel",
                                  danger: true,
                                  onConfirm: async () => {
                                    await handleDeletePack(packId);
                                  },
                                });
                              }}
                            >
                              Delete Pack
                            </button>
                          </div>
                        </>
                      )}
                    </div>
                  </>
                )}

                <div className="tab-strip">
                  <button
                    type="button"
                    className={`tab-btn ${activeTab === "cards" ? "active" : ""}`}
                    onClick={() => setActiveTab("cards")}
                  >
                    Cards
                  </button>
                  <button
                    type="button"
                    className={`tab-btn ${activeTab === "strings" ? "active" : ""}`}
                    onClick={() => setActiveTab("strings")}
                  >
                    Strings
                  </button>
                </div>

                <div className="tab-content">
                  {activeTab === "cards" ? (
                    <CardListPanel onEditCard={handleEditCard} onNewCard={handleNewCard} />
                  ) : (
                    <StringsListPanel />
                  )}
                </div>
              </div>

              {cardDrawerOpen && activePackId && (
                <CardEditDrawer
                  packId={activePackId}
                  workspaceId={useShellStore.getState().workspaceId!}
                  cardId={isCreatingCard ? null : editingCardId}
                  onClose={handleDrawerClose}
                  onSaved={handleDrawerSaved}
                />
              )}
            </>
          )}
        </section>
      </div>

      {modal && (
        <div className="modal-layer">
          <div className="modal-backdrop" onClick={closeModal} />
          <section className="modal-box" role="dialog" aria-modal="true">
            {modal.type === "workspace" && (
              <WorkspaceModal
                config={config}
                recentWorkspaces={recentWorkspaces}
                currentWorkspace={currentWorkspace}
                onWorkspaceOpened={(meta, path) => void handleWorkspaceOpened(meta, path)}
                onRecentRefreshed={setRecentWorkspaces}
                onNotice={handleNotice}
              />
            )}
            {modal.type === "settings" && (
              <SettingsModal config={config} onConfigSaved={setConfig} onNotice={handleNotice} />
            )}
            {modal.type === "addPack" && (
              <AddPackModal
                hasWorkspace={currentWorkspace !== null}
                onPackOpened={(id, meta) => void handlePackOpened(id, meta)}
                onPackCreated={(id, meta) => void handlePackCreated(id, meta)}
                onOverviewsRefreshed={setPackOverviews}
                onNotice={handleNotice}
              />
            )}
          </section>
        </div>
      )}
      {dialog && (
        <AppDialog />
      )}
    </div>
  );
}
