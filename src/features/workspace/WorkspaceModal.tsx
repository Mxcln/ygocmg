import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useShellStore } from "../../shared/stores/shellStore";
import { workspaceApi } from "../../shared/api/workspaceApi";
import type { GlobalConfig } from "../../shared/contracts/config";
import type {
  WorkspaceMeta,
  WorkspaceRegistryEntry,
  WorkspaceRegistryFile,
} from "../../shared/contracts/workspace";
import {
  formatTimestamp,
  formatError,
  normalizeOptionalText,
} from "../../shared/utils/format";
import { useAppI18n } from "../../shared/i18n";
import shared from "../../shared/styles/shared.module.css";
import styles from "./WorkspaceModal.module.css";

type WorkspaceView = "recent" | "create";

interface CreateWorkspaceForm {
  name: string;
  description: string;
  path: string;
}

export interface WorkspaceModalProps {
  config: GlobalConfig;
  recentWorkspaces: WorkspaceRegistryFile;
  currentWorkspace: { meta: WorkspaceMeta; path: string } | null;
  onWorkspaceOpened: (meta: WorkspaceMeta, path: string) => void;
  onRecentRefreshed: (registry: WorkspaceRegistryFile) => void;
  onNotice: (tone: "success" | "warning" | "error", title: string, detail: string) => void;
}

export function WorkspaceModal({
  config,
  recentWorkspaces,
  currentWorkspace,
  onWorkspaceOpened,
  onRecentRefreshed,
  onNotice,
}: WorkspaceModalProps) {
  const { t } = useAppI18n();
  const closeModal = useShellStore((s) => s.closeModal);

  const [view, setView] = useState<WorkspaceView>(currentWorkspace ? "recent" : "create");
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [openPath, setOpenPath] = useState("");
  const [createForm, setCreateForm] = useState<CreateWorkspaceForm>({
    name: "",
    description: "",
    path: "",
  });

  async function refreshRecent() {
    const next = await workspaceApi.listRecentWorkspaces();
    onRecentRefreshed(next);
    return next;
  }

  async function handleOpenWorkspace(path: string) {
    const trimmedPath = path.trim();
    if (!trimmedPath) {
      onNotice("warning", t("workspace.notice.pathRequired.title"), t("workspace.notice.pathRequired.detail"));
      return;
    }

    setBusyAction(`open:${trimmedPath}`);
    try {
      const workspace = await workspaceApi.openWorkspace({ path: trimmedPath });
      await refreshRecent();
      onWorkspaceOpened(workspace, trimmedPath);
      closeModal();
      onNotice("success", t("workspace.notice.opened.title"), t("workspace.notice.opened.detail", { name: workspace.name }));
    } catch (err) {
      onNotice("error", t("workspace.notice.openFailed"), formatError(err));
    } finally {
      setBusyAction(null);
    }
  }

  async function handleCreateWorkspace(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusyAction("create");

    try {
      const name = createForm.name.trim();
      const description = normalizeOptionalText(createForm.description);
      const path = createForm.path.trim();

      if (!name || !path) {
        throw new Error(t("workspace.error.nameAndPathRequired"));
      }

      await workspaceApi.createWorkspace({ name, description, path });
      const opened = await workspaceApi.openWorkspace({ path });
      await refreshRecent();
      onWorkspaceOpened(opened, path);
      setCreateForm({ name: "", description: "", path: "" });
      closeModal();
      onNotice("success", t("workspace.notice.created.title"), t("workspace.notice.created.detail", { name }));
    } catch (err) {
      onNotice("error", t("workspace.notice.createFailed"), formatError(err));
      setView("create");
    } finally {
      setBusyAction(null);
    }
  }

  return (
    <>
      <header className={shared.modalHeader}>
        <h2>{t("workspace.title")}</h2>
        <button className={shared.modalCloseButton} type="button" onClick={closeModal}>
          {t("action.close")}
        </button>
      </header>

      <div className={`${shared.modalBody} ${shared.workspaceModalBody}`}>
        <aside className={shared.modalTabs}>
          <button type="button" className={view === "recent" ? "active" : ""} onClick={() => setView("recent")}>
            {t("workspace.tab.open")}
          </button>
          <button type="button" className={view === "create" ? "active" : ""} onClick={() => setView("create")}>
            {t("workspace.tab.create")}
          </button>
        </aside>

        <div className={shared.modalPanel}>
          {view === "recent" && (
            <RecentPanel
              recentWorkspaces={recentWorkspaces.workspaces}
              currentWorkspace={currentWorkspace}
              openPath={openPath}
              busyAction={busyAction}
              onOpenPathChange={setOpenPath}
              onOpen={handleOpenWorkspace}
              labels={{
                workspacePath: t("workspace.path"),
                browse: t("action.browse"),
                open: t("action.open"),
                opening: t("workspace.opening"),
                noRecent: t("workspace.noRecent"),
                unnamed: t("workspace.unnamed"),
                current: t("workspace.current"),
                selectDirectory: t("workspace.selectDirectory"),
              }}
            />
          )}

          {view === "create" && (
            <CreatePanel
              createForm={createForm}
              busyAction={busyAction}
              onFormChange={setCreateForm}
              onReset={() => setCreateForm({ name: "", description: "", path: "" })}
              onSubmit={handleCreateWorkspace}
              labels={{
                workspaceName: t("workspace.name"),
                description: t("workspace.description"),
                workspacePath: t("workspace.path"),
                descriptionPlaceholder: t("workspace.descriptionPlaceholder"),
                browse: t("action.browse"),
                createAndOpen: t("workspace.createAndOpen"),
                creating: t("workspace.creating"),
                reset: t("workspace.reset"),
                selectDirectory: t("workspace.selectDirectory"),
              }}
            />
          )}
        </div>
      </div>
    </>
  );
}

function RecentPanel({
  recentWorkspaces,
  currentWorkspace,
  openPath,
  busyAction,
  onOpenPathChange,
  onOpen,
  labels,
}: {
  recentWorkspaces: WorkspaceRegistryEntry[];
  currentWorkspace: { meta: WorkspaceMeta; path: string } | null;
  openPath: string;
  busyAction: string | null;
  onOpenPathChange: (value: string) => void;
  onOpen: (path: string) => Promise<void>;
  labels: {
    workspacePath: string;
    browse: string;
    open: string;
    opening: string;
    noRecent: string;
    unnamed: string;
    current: string;
    selectDirectory: string;
  };
}) {
  return (
    <section className={shared.workspaceRecentPanel}>
      <div className={shared.field}>
        <span>{labels.workspacePath}</span>
        <div className={shared.filePickerRow}>
          <input
            value={openPath}
            onChange={(e) => onOpenPathChange(e.target.value)}
            placeholder="D:\\YGOCMG\\workspace-demo"
          />
          <button
            type="button"
            className={shared.ghostButton}
            onClick={async () => {
              const selected = await open({ directory: true, title: labels.selectDirectory });
              if (typeof selected === "string") onOpenPathChange(selected);
            }}
          >
            {labels.browse}
          </button>
          <button
            className={shared.primaryButton}
            type="button"
            disabled={busyAction !== null || !openPath.trim()}
            onClick={() => void onOpen(openPath)}
          >
            {busyAction?.startsWith("open:") ? labels.opening : labels.open}
          </button>
        </div>
      </div>

      {recentWorkspaces.length === 0 ? (
        <p className={shared.emptyStateText}>{labels.noRecent}</p>
      ) : (
        <ul className={styles.workspaceList}>
          {recentWorkspaces.map((ws) => {
            const isCurrent = currentWorkspace?.meta.id === ws.workspace_id;
            return (
              <li
                key={ws.workspace_id}
                className={`${styles.workspaceListItem} ${isCurrent ? "current-workspace" : ""}`}
              >
                <strong className={styles.wsName}>{ws.name_cache ?? labels.unnamed}</strong>
                <code className={styles.wsPath}>{ws.path}</code>
                <span className={styles.wsTime}>{formatTimestamp(ws.last_opened_at)}</span>
                {isCurrent && <span className={shared.livePill}>{labels.current}</span>}
                <button
                  className={`${shared.ghostButton} ${styles.wsOpenBtn}`}
                  type="button"
                  disabled={busyAction !== null}
                  onClick={() => void onOpen(ws.path)}
                >
                  {labels.open}
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}

function CreatePanel({
  createForm,
  busyAction,
  onFormChange,
  onReset,
  onSubmit,
  labels,
}: {
  createForm: CreateWorkspaceForm;
  busyAction: string | null;
  onFormChange: (updater: (prev: CreateWorkspaceForm) => CreateWorkspaceForm) => void;
  onReset: () => void;
  onSubmit: (event: React.FormEvent<HTMLFormElement>) => void;
  labels: {
    workspaceName: string;
    description: string;
    workspacePath: string;
    descriptionPlaceholder: string;
    browse: string;
    createAndOpen: string;
    creating: string;
    reset: string;
    selectDirectory: string;
  };
}) {
  return (
    <section className={shared.workspaceCreatePanel}>
      <form className={shared.formStack} onSubmit={onSubmit}>
        <label className={shared.field}>
          <span>{labels.workspaceName}</span>
          <input
            value={createForm.name}
            onChange={(e) => onFormChange((c) => ({ ...c, name: e.target.value }))}
            placeholder="OCG Custom Lab"
          />
        </label>

        <label className={shared.field}>
          <span>{labels.description}</span>
          <textarea
            rows={3}
            value={createForm.description}
            onChange={(e) => onFormChange((c) => ({ ...c, description: e.target.value }))}
            placeholder={labels.descriptionPlaceholder}
          />
        </label>

        <div className={shared.field}>
          <span>{labels.workspacePath}</span>
          <div className={shared.filePickerRow}>
            <input
              value={createForm.path}
              onChange={(e) => onFormChange((c) => ({ ...c, path: e.target.value }))}
              placeholder="D:\\YGOCMG\\workspaces\\ocg-custom-lab"
            />
            <button
              type="button"
              className={shared.ghostButton}
              onClick={async () => {
                const selected = await open({ directory: true, title: labels.selectDirectory });
                if (typeof selected === "string") onFormChange((c) => ({ ...c, path: selected }));
              }}
            >
              {labels.browse}
            </button>
          </div>
        </div>

        <div className={shared.formActions}>
          <button className={shared.primaryButton} type="submit" disabled={busyAction !== null}>
            {busyAction === "create" ? labels.creating : labels.createAndOpen}
          </button>
          <button className={shared.ghostButton} type="button" disabled={busyAction !== null} onClick={onReset}>
            {labels.reset}
          </button>
        </div>
      </form>
    </section>
  );
}
