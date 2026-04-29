import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { useShellStore } from "../shared/stores/shellStore";
import { configApi } from "../shared/api/configApi";
import { workspaceApi } from "../shared/api/workspaceApi";
import { packApi } from "../shared/api/packApi";
import type { GlobalConfig } from "../shared/contracts/config";
import type { PackMetadata } from "../shared/contracts/pack";
import type { WorkspaceMeta, WorkspaceRegistryFile } from "../shared/contracts/workspace";
import { formatError } from "../shared/utils/format";
import { WorkspaceModal } from "../features/workspace/WorkspaceModal";
import { SettingsModal } from "../features/settings/SettingsModal";
import { AddPackModal } from "../features/pack/AddPackModal";
import { useQueryClient } from "@tanstack/react-query";
import { AppDialog } from "../features/dialogs/AppDialog";
import { StandardPackView } from "../features/standardPack/StandardPackView";
import { ExportModal } from "../features/export/ExportModal";
import { useAppWindow } from "./hooks/useAppWindow";
import { useSidebarResize } from "./hooks/useSidebarResize";
import { TitleBar } from "./TitleBar";
import { AppSidebar } from "./AppSidebar";
import { NoticeBanner } from "./NoticeBanner";
import type { Notice, NoticeTone } from "./NoticeBanner";
import { PackWorkArea } from "./PackWorkArea";
import shared from "../shared/styles/shared.module.css";
import styles from "./App.module.css";

interface CurrentWorkspaceRef {
  meta: WorkspaceMeta;
  path: string;
}

export function App() {
  const [config, setConfig] = useState<GlobalConfig | null>(null);
  const [recentWorkspaces, setRecentWorkspaces] = useState<WorkspaceRegistryFile | null>(null);
  const [currentWorkspace, setCurrentWorkspace] = useState<CurrentWorkspaceRef | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const configRef = useRef<GlobalConfig | null>(null);
  const queryClient = useQueryClient();

  const { maximized, shellReady, handleWindowAction } = useAppWindow(configRef, setConfig);
  const { sidebarWidth, setSidebarWidth, beginSidebarResize } = useSidebarResize(configRef, setConfig);

  const modal = useShellStore((s) => s.modal);
  const dialog = useShellStore((s) => s.dialog);
  const closeModal = useShellStore((s) => s.closeModal);
  const activePackId = useShellStore((s) => s.activePackId);
  const activeView = useShellStore((s) => s.activeView);
  const setActivePack = useShellStore((s) => s.setActivePack);
  const setActiveStandardPack = useShellStore((s) => s.setActiveStandardPack);
  const addOpenPack = useShellStore((s) => s.addOpenPack);
  const removeOpenPack = useShellStore((s) => s.removeOpenPack);
  const setPackOverviews = useShellStore((s) => s.setPackOverviews);
  const setWorkspace = useShellStore((s) => s.setWorkspace);

  useEffect(() => {
    configRef.current = config;
  }, [config]);

  function handleNotice(tone: NoticeTone, title: string, detail: string) {
    setNotice({ tone, title, detail });
  }

  function handleConfigSaved(nextConfig: GlobalConfig) {
    configRef.current = nextConfig;
    setConfig(nextConfig);
    void queryClient.invalidateQueries({ queryKey: ["standard-pack-status"] });
    void queryClient.invalidateQueries({ queryKey: ["standard-cards"] });
  }

  async function persistActivePack(packId: string) {
    setActivePack(packId);
    try {
      await packApi.setActivePack({ packId });
    } catch (err) {
      handleNotice("error", "Failed to switch pack", formatError(err));
    }
  }

  function handleOpenStandardPack() {
    setActiveStandardPack();
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

  async function handlePackDeleted(packId: string) {
    removeOpenPack(packId);
    try {
      const overviews = await packApi.listPackOverviews();
      setPackOverviews(overviews);
    } catch {
      // best-effort overview refresh
    }
    handleNotice("success", "Pack deleted", "The pack has been removed from the workspace.");
  }

  if (loading) {
    return (
      <main className={styles.launchShell}>
        <p className={styles.launchText}>Loading...</p>
      </main>
    );
  }

  if (error || !config || !recentWorkspaces) {
    return (
      <main className={styles.launchShell}>
        <div className={styles.launchError}>
          <p>{error ?? "Initialization failed."}</p>
          <button className={shared.primaryButton} type="button" onClick={() => window.location.reload()}>
            Reload
          </button>
        </div>
      </main>
    );
  }

  const workspaceName = currentWorkspace?.meta.name ?? "No Workspace Open";
  const activeCustomPackId =
    activeView?.type === "custom_pack" ? activeView.packId : activePackId;
  const isStandardView = activeView?.type === "standard_pack";
  const shellStyle = { "--sidebar-width": `${sidebarWidth}px` } as CSSProperties;

  return (
    <div className={styles.appShell} data-ready={shellReady || undefined} style={shellStyle}>
      <TitleBar
        workspaceName={workspaceName}
        maximized={maximized}
        onWindowAction={handleWindowAction}
      />

      <div className={styles.shellBody}>
        <AppSidebar
          hasWorkspace={currentWorkspace !== null}
          isStandardView={isStandardView}
          onPackClick={(packId) => void persistActivePack(packId)}
          onClosePack={(packId) => void handleClosePack(packId)}
          onOpenStandardPack={handleOpenStandardPack}
          onBeginResize={beginSidebarResize}
        />

        <section className={styles.workArea}>
          {notice && <NoticeBanner notice={notice} onDismiss={() => setNotice(null)} />}

          {isStandardView ? (
            <StandardPackView config={config} />
          ) : !activeCustomPackId ? (
            <div className={styles.emptyState}>
              <p className={styles.emptyLabel}>No Pack Open</p>
              <p className={styles.emptyHint}>
                {currentWorkspace
                  ? "Use the + button in the sidebar to open or create a pack."
                  : "Open a workspace first, then add packs from the sidebar."}
              </p>
            </div>
          ) : (
            <PackWorkArea
              config={config}
              onNotice={handleNotice}
              onPackDeleted={(packId) => void handlePackDeleted(packId)}
            />
          )}
        </section>
      </div>

      {modal && (
        <div className={styles.modalLayer}>
          <div className={styles.modalBackdrop} onClick={closeModal} />
          <section className={styles.modalBox} role="dialog" aria-modal="true">
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
              <SettingsModal config={config} onConfigSaved={handleConfigSaved} onNotice={handleNotice} />
            )}
            {modal.type === "addPack" && (
              <AddPackModal
                config={config}
                hasWorkspace={currentWorkspace !== null}
                onPackOpened={(id, meta) => void handlePackOpened(id, meta)}
                onPackCreated={(id, meta) => void handlePackCreated(id, meta)}
                onOverviewsRefreshed={setPackOverviews}
                onNotice={handleNotice}
              />
            )}
            {modal.type === "export" && (
              <ExportModal config={config} onNotice={handleNotice} />
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
