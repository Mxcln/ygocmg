import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
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
  const [activeTab, setActiveTab] = useState<PackTab>("cards");

  const modal = useShellStore((s) => s.modal);
  const openModal = useShellStore((s) => s.openModal);
  const closeModal = useShellStore((s) => s.closeModal);
  const openPackIds = useShellStore((s) => s.openPackIds);
  const activePackId = useShellStore((s) => s.activePackId);
  const packMetadataMap = useShellStore((s) => s.packMetadataMap);
  const setActivePack = useShellStore((s) => s.setActivePack);
  const addOpenPack = useShellStore((s) => s.addOpenPack);
  const removeOpenPack = useShellStore((s) => s.removeOpenPack);
  const setPackOverviews = useShellStore((s) => s.setPackOverviews);
  const setWorkspace = useShellStore((s) => s.setWorkspace);

  useEffect(() => {
    let active = true;

    async function bootstrap() {
      try {
        const nextConfig = await configApi.initialize();
        const nextRecent = await workspaceApi.listRecentWorkspaces();
        if (!active) return;
        setConfig(nextConfig);
        setRecentWorkspaces(nextRecent);
      } catch (err) {
        if (!active) return;
        setError(formatError(err));
      } finally {
        if (active) setLoading(false);
      }
    }

    void bootstrap();
    return () => { active = false; };
  }, []);

  useEffect(() => {
    let cancelled = false;
    let unlistenFocus: UnlistenFn | null = null;

    async function bindWindowState() {
      try {
        const appWindow = getCurrentWindow();
        const isMax = await appWindow.isMaximized();
        if (!cancelled) {
          setMaximized(isMax);
          setShellReady(true);
        }
        unlistenFocus = await appWindow.onFocusChanged(() => {});
      } catch {
        if (!cancelled) setShellReady(true);
      }
    }

    void bindWindowState();
    return () => {
      cancelled = true;
      if (unlistenFocus) void unlistenFocus();
    };
  }, []);

  function handleNotice(tone: NoticeTone, title: string, detail: string) {
    setNotice({ tone, title, detail });
  }

  async function handleWindowAction(action: "minimize" | "toggle-maximize" | "close") {
    const appWindow = getCurrentWindow();
    if (action === "minimize") { await appWindow.minimize(); return; }
    if (action === "toggle-maximize") {
      await appWindow.toggleMaximize();
      const isMax = await appWindow.isMaximized();
      setMaximized(isMax);
      return;
    }
    await appWindow.close();
  }

  async function handleWorkspaceOpened(meta: WorkspaceMeta, path: string) {
    setCurrentWorkspace({ meta, path });
    setWorkspace(meta.id, meta.name);
    try {
      const overviews = await packApi.listPackOverviews();
      setPackOverviews(overviews);
    } catch {
      // overviews will remain empty
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

  async function handleDeletePack(packId: string) {
    try {
      await packApi.deletePack({ packId });
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

  return (
    <div className={`app-shell ${shellReady ? "ready" : ""}`}>
      {/* ── Titlebar ── */}
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
            <svg width="10" height="1" viewBox="0 0 10 1"><rect width="10" height="1" fill="currentColor"/></svg>
          </button>
          <button type="button" className="win-btn" aria-label={maximized ? "Restore" : "Maximize"} onClick={() => void handleWindowAction("toggle-maximize")}>
            {maximized ? (
              <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1">
                <rect x="2" y="0" width="8" height="8" rx="0.5"/>
                <rect x="0" y="2" width="8" height="8" rx="0.5" fill="var(--panel)"/>
              </svg>
            ) : (
              <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1">
                <rect x="0.5" y="0.5" width="9" height="9" rx="0.5"/>
              </svg>
            )}
          </button>
          <button type="button" className="win-btn win-close" aria-label="Close" onClick={() => void handleWindowAction("close")}>
            <svg width="10" height="10" viewBox="0 0 10 10" stroke="currentColor" strokeWidth="1.2">
              <line x1="0" y1="0" x2="10" y2="10"/><line x1="10" y1="0" x2="0" y2="10"/>
            </svg>
          </button>
        </div>
      </header>

      <div className="shell-body">
        {/* ── Sidebar ── */}
        <aside className="sidebar">
          <div className="sidebar-actions">
            <button
              type="button"
              className={`action-btn ${modal?.type === "workspace" ? "active" : ""}`}
              title="Workspace"
              onClick={() => openModal("workspace")}
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3">
                <rect x="1" y="2" width="14" height="12" rx="1.5"/><line x1="1" y1="6" x2="15" y2="6"/>
              </svg>
            </button>
            <button
              type="button"
              className="action-btn"
              title="Export Expansions"
              disabled
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3">
                <path d="M2 11v3h12v-3"/><path d="M8 2v8"/><path d="M5 7l3 3 3-3"/>
              </svg>
            </button>
            <button
              type="button"
              className={`action-btn ${modal?.type === "settings" ? "active" : ""}`}
              title="Global Settings"
              onClick={() => openModal("settings")}
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3">
                <circle cx="8" cy="8" r="2.5"/><path d="M8 1v2m0 10v2M1 8h2m10 0h2M2.9 2.9l1.5 1.5m7.2 7.2l1.5 1.5M13.1 2.9l-1.5 1.5M4.4 11.6l-1.5 1.5"/>
              </svg>
            </button>
          </div>

          <div className="pack-list">
            {openPackIds.map((packId) => {
              const meta = packMetadataMap[packId];
              return (
                <div
                  key={packId}
                  className={`pack-item-row ${activePackId === packId ? "active" : ""}`}
                >
                  <button
                    type="button"
                    className="pack-item-name"
                    onClick={() => setActivePack(packId)}
                    title={meta?.name ?? packId}
                  >
                    {meta?.name ?? packId}
                  </button>
                  <button
                    type="button"
                    className="pack-close-btn"
                    title="Close pack"
                    onClick={(e) => {
                      e.stopPropagation();
                      void handleClosePack(packId);
                    }}
                  >
                    <svg width="8" height="8" viewBox="0 0 8 8" stroke="currentColor" strokeWidth="1.2">
                      <line x1="0" y1="0" x2="8" y2="8"/><line x1="8" y1="0" x2="0" y2="8"/>
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
        </aside>

        {/* ── Work Area ── */}
        <section className="work-area">
          {notice && (
            <div className={`notice ${notice.tone}`}>
              <strong>{notice.title}</strong>
              <span>{notice.detail}</span>
              <button type="button" className="notice-close" onClick={() => setNotice(null)}>×</button>
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
              {/* Pack Metadata Bar */}
              <div className="meta-bar">
                <div className="meta-summary">
                  <strong className="meta-pack-name">{activeMeta?.name ?? activePackId}</strong>
                  <span className="meta-detail">
                    {activeMeta
                      ? `${activeMeta.author} · v${activeMeta.version} · ${activeMeta.display_language_order.join(", ") || "no languages"}`
                      : "Loading metadata..."}
                  </span>
                </div>
                <button
                  type="button"
                  className="meta-toggle"
                  onClick={() => setMetaExpanded(!metaExpanded)}
                  aria-label={metaExpanded ? "Collapse metadata" : "Expand metadata"}
                >
                  <svg
                    width="12" height="12" viewBox="0 0 12 12"
                    fill="none" stroke="currentColor" strokeWidth="1.5"
                    style={{ transform: metaExpanded ? "rotate(180deg)" : "none", transition: "transform 150ms" }}
                  >
                    <path d="M2 4l4 4 4-4"/>
                  </svg>
                </button>
              </div>

              {metaExpanded && activeMeta && (
                <div className="meta-expanded">
                  <div className="meta-grid">
                    <div className="meta-field">
                      <span className="meta-field-label">Name</span>
                      <span className="meta-field-value">{activeMeta.name}</span>
                    </div>
                    <div className="meta-field">
                      <span className="meta-field-label">Author</span>
                      <span className="meta-field-value">{activeMeta.author}</span>
                    </div>
                    <div className="meta-field">
                      <span className="meta-field-label">Version</span>
                      <span className="meta-field-value">{activeMeta.version}</span>
                    </div>
                    <div className="meta-field">
                      <span className="meta-field-label">Description</span>
                      <span className="meta-field-value">{activeMeta.description || "—"}</span>
                    </div>
                    <div className="meta-field">
                      <span className="meta-field-label">Languages</span>
                      <span className="meta-field-value">
                        {activeMeta.display_language_order.join(", ") || "—"}
                      </span>
                    </div>
                    <div className="meta-field">
                      <span className="meta-field-label">Default Export Language</span>
                      <span className="meta-field-value">{activeMeta.default_export_language || "—"}</span>
                    </div>
                    <div className="meta-field">
                      <span className="meta-field-label">Created</span>
                      <span className="meta-field-value">{formatTimestamp(activeMeta.created_at)}</span>
                    </div>
                    <div className="meta-field">
                      <span className="meta-field-label">Updated</span>
                      <span className="meta-field-value">{formatTimestamp(activeMeta.updated_at)}</span>
                    </div>
                  </div>

                  <div className="meta-actions">
                    <button
                      type="button"
                      className="ghost-button danger-ghost"
                      onClick={() => {
                        if (window.confirm(`Delete pack "${activeMeta.name}"? This cannot be undone.`)) {
                          void handleDeletePack(activePackId);
                        }
                      }}
                    >
                      Delete Pack
                    </button>
                  </div>
                </div>
              )}

              {/* Tab Strip */}
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

              {/* Tab Content */}
              <div className="tab-content">
                {activeTab === "cards" ? (
                  <p className="content-placeholder">Card list will appear here (P3)</p>
                ) : (
                  <p className="content-placeholder">String entries will appear here (P4)</p>
                )}
              </div>
            </>
          )}
        </section>
      </div>

      {/* ── Modal Layer ── */}
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
              <SettingsModal
                config={config}
                onConfigSaved={setConfig}
                onNotice={handleNotice}
              />
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
    </div>
  );
}
