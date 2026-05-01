import { useCallback, useEffect, useRef, useState } from "react";
import type { CSSProperties, PointerEvent as ReactPointerEvent } from "react";
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
import { AppI18nProvider, formatAppMessageById, useAppI18n } from "../shared/i18n";
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
  const [notices, setNotices] = useState<Notice[]>([]);
  const nextNoticeId = useRef(1);
  const configRef = useRef<GlobalConfig | null>(null);
  const queryClient = useQueryClient();

  const { maximized, shellReady, handleWindowAction } = useAppWindow(configRef, setConfig);
  const { sidebarWidth, setSidebarWidth, beginSidebarResize } = useSidebarResize(configRef, setConfig);

  const setActivePack = useShellStore((s) => s.setActivePack);
  const addOpenPack = useShellStore((s) => s.addOpenPack);
  const setPackOverviews = useShellStore((s) => s.setPackOverviews);
  const setWorkspace = useShellStore((s) => s.setWorkspace);

  useEffect(() => {
    configRef.current = config;
  }, [config]);

  function handleNotice(tone: NoticeTone, title: string, detail: string) {
    const id = nextNoticeId.current;
    nextNoticeId.current += 1;
    setNotices((current) => [...current.slice(-3), { id, tone, title, detail }]);
  }

  const dismissNotice = useCallback((id: number) => {
    setNotices((current) => current.filter((notice) => notice.id !== id));
  }, []);

  async function persistActivePack(packId: string) {
    setActivePack(packId);
    try {
      await packApi.setActivePack({ packId });
    } catch (err) {
      handleNotice(
        "error",
        formatAppMessageById("app.notice.switchPackFailed"),
        formatError(err),
      );
    }
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
        setLoading(false);

        void tryRestoreLastSession(nextRecent);
      } catch (err) {
        if (!active) return;
        setError(formatError(err));
        setLoading(false);
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

  if (loading) {
    return (
      <AppI18nProvider locale={null}>
        <main className={styles.launchShell}>
          <p className={styles.launchText}>{formatAppMessageById("app.loading")}</p>
        </main>
      </AppI18nProvider>
    );
  }

  if (error || !config || !recentWorkspaces) {
    return (
      <AppI18nProvider locale={config?.app_language ?? null}>
        <main className={styles.launchShell}>
          <div className={styles.launchError}>
            <p>{error ?? formatAppMessageById("app.initializationFailed")}</p>
            <button className={shared.primaryButton} type="button" onClick={() => window.location.reload()}>
              {formatAppMessageById("app.reload")}
            </button>
          </div>
        </main>
      </AppI18nProvider>
    );
  }

  return (
    <AppI18nProvider locale={config.app_language}>
      <AppShell
        config={config}
        configRef={configRef}
        recentWorkspaces={recentWorkspaces}
        currentWorkspace={currentWorkspace}
        setConfig={setConfig}
        setRecentWorkspaces={setRecentWorkspaces}
        setCurrentWorkspace={setCurrentWorkspace}
        notices={notices}
        setNotices={setNotices}
        nextNoticeId={nextNoticeId}
        dismissNotice={dismissNotice}
        queryClient={queryClient}
        maximized={maximized}
        shellReady={shellReady}
        handleWindowAction={handleWindowAction}
        sidebarWidth={sidebarWidth}
        beginSidebarResize={beginSidebarResize}
      />
    </AppI18nProvider>
  );
}

function AppShell({
  config,
  configRef,
  recentWorkspaces,
  currentWorkspace,
  setConfig,
  setRecentWorkspaces,
  setCurrentWorkspace,
  notices,
  setNotices,
  nextNoticeId,
  dismissNotice,
  queryClient,
  maximized,
  shellReady,
  handleWindowAction,
  sidebarWidth,
  beginSidebarResize,
}: {
  config: GlobalConfig;
  configRef: React.MutableRefObject<GlobalConfig | null>;
  recentWorkspaces: WorkspaceRegistryFile;
  currentWorkspace: CurrentWorkspaceRef | null;
  setConfig: React.Dispatch<React.SetStateAction<GlobalConfig | null>>;
  setRecentWorkspaces: React.Dispatch<React.SetStateAction<WorkspaceRegistryFile | null>>;
  setCurrentWorkspace: React.Dispatch<React.SetStateAction<CurrentWorkspaceRef | null>>;
  notices: Notice[];
  setNotices: React.Dispatch<React.SetStateAction<Notice[]>>;
  nextNoticeId: React.MutableRefObject<number>;
  dismissNotice: (id: number) => void;
  queryClient: ReturnType<typeof useQueryClient>;
  maximized: boolean;
  shellReady: boolean;
  handleWindowAction: (action: "minimize" | "toggle-maximize" | "close") => void;
  sidebarWidth: number;
  beginSidebarResize: (event: ReactPointerEvent<HTMLDivElement>) => void;
}) {
  const { t } = useAppI18n();
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

  function handleNotice(tone: NoticeTone, title: string, detail: string) {
    const id = nextNoticeId.current;
    nextNoticeId.current += 1;
    setNotices((current) => [...current.slice(-3), { id, tone, title, detail }]);
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
      handleNotice("error", t("app.notice.switchPackFailed"), formatError(err));
    }
  }

  function handleOpenStandardPack() {
    setActiveStandardPack();
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
      handleNotice("error", t("app.notice.closePackFailed"), formatError(err));
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
    handleNotice("success", t("app.notice.packDeleted.title"), t("app.notice.packDeleted.detail"));
  }

  const workspaceName = currentWorkspace?.meta.name ?? t("app.noWorkspaceOpen");
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
          {isStandardView ? (
            <StandardPackView config={config} />
          ) : !activeCustomPackId ? (
            <div className={styles.emptyState}>
              <p className={styles.emptyLabel}>{t("app.noPackOpen")}</p>
              <p className={styles.emptyHint}>
                {currentWorkspace
                  ? t("app.empty.openOrCreatePack")
                  : t("app.empty.openWorkspaceFirst")}
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
      <NoticeBanner notices={notices} onDismiss={dismissNotice} />
    </div>
  );
}
