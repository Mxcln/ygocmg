import { useState, useEffect } from "react";
import { useShellStore } from "../../shared/stores/shellStore";
import { configApi } from "../../shared/api/configApi";
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
  buildSuggestedWorkspacePath,
} from "../../shared/utils/format";

type WorkspaceView = "overview" | "recent" | "create";

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
  const closeModal = useShellStore((s) => s.closeModal);

  const [view, setView] = useState<WorkspaceView>(currentWorkspace ? "recent" : "create");
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [openPath, setOpenPath] = useState("");
  const [createForm, setCreateForm] = useState<CreateWorkspaceForm>({
    name: "",
    description: "",
    path: buildSuggestedWorkspacePath(config.default_workspace_root ?? "", ""),
  });
  const [createPathTouched, setCreatePathTouched] = useState(false);

  useEffect(() => {
    if (createPathTouched) return;

    setCreateForm((current) => {
      const nextPath = buildSuggestedWorkspacePath(
        config.default_workspace_root ?? "",
        current.name,
      );
      if (current.path === nextPath) return current;
      return { ...current, path: nextPath };
    });
  }, [config.default_workspace_root, createForm.name, createPathTouched]);

  const recentCount = recentWorkspaces.workspaces.length;
  const hasYgoProPath = Boolean(config.ygopro_path);
  const hasGlobalRoot = Boolean(config.default_workspace_root);

  async function refreshRecent() {
    const next = await workspaceApi.listRecentWorkspaces();
    onRecentRefreshed(next);
    return next;
  }

  async function handleOpenWorkspace(path: string) {
    const trimmedPath = path.trim();
    if (!trimmedPath) {
      onNotice("warning", "Workspace path required", "Enter a workspace path or choose from recent workspaces.");
      return;
    }

    setBusyAction(`open:${trimmedPath}`);
    try {
      const workspace = await workspaceApi.openWorkspace({ path: trimmedPath });
      await refreshRecent();
      onWorkspaceOpened(workspace, trimmedPath);
      closeModal();
      onNotice("success", "Workspace opened", `Current workspace switched to ${workspace.name}.`);
    } catch (err) {
      onNotice("error", "Failed to open workspace", formatError(err));
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
        throw new Error("Workspace name and path are both required.");
      }

      await workspaceApi.createWorkspace({ name, description, path });
      const opened = await workspaceApi.openWorkspace({ path });
      await refreshRecent();
      onWorkspaceOpened(opened, path);
      setCreatePathTouched(false);
      setCreateForm({
        name: "",
        description: "",
        path: buildSuggestedWorkspacePath(config.default_workspace_root ?? "", ""),
      });
      closeModal();
      onNotice("success", "Workspace created", `${name} is now the current workspace.`);
    } catch (err) {
      onNotice("error", "Failed to create workspace", formatError(err));
      setView("create");
    } finally {
      setBusyAction(null);
    }
  }

  return (
    <>
      <header className="modal-header">
        <div>
          <p className="eyebrow">Workspace</p>
          <h2>Workspace</h2>
        </div>
        <button className="modal-close-button" type="button" onClick={closeModal}>
          Close
        </button>
      </header>

      <div className="modal-body workspace-modal-body">
        <aside className="modal-tabs">
          <button type="button" className={view === "overview" ? "active" : ""} onClick={() => setView("overview")}>
            Overview
          </button>
          <button type="button" className={view === "recent" ? "active" : ""} onClick={() => setView("recent")}>
            Recent Workspaces
          </button>
          <button type="button" className={view === "create" ? "active" : ""} onClick={() => setView("create")}>
            Create Workspace
          </button>
        </aside>

        <div className="modal-panel">
          {view === "overview" && (
            <OverviewPanel
              currentWorkspace={currentWorkspace}
              recentCount={recentCount}
              hasYgoProPath={hasYgoProPath}
              hasGlobalRoot={hasGlobalRoot}
              onSwitchView={setView}
            />
          )}

          {view === "recent" && (
            <RecentPanel
              recentWorkspaces={recentWorkspaces.workspaces}
              currentWorkspace={currentWorkspace}
              openPath={openPath}
              busyAction={busyAction}
              recentCount={recentCount}
              onOpenPathChange={setOpenPath}
              onOpen={handleOpenWorkspace}
            />
          )}

          {view === "create" && (
            <CreatePanel
              createForm={createForm}
              hasGlobalRoot={hasGlobalRoot}
              busyAction={busyAction}
              defaultRoot={config.default_workspace_root ?? ""}
              onFormChange={setCreateForm}
              onPathTouched={() => setCreatePathTouched(true)}
              onReset={() => {
                setCreatePathTouched(false);
                setCreateForm({
                  name: "",
                  description: "",
                  path: buildSuggestedWorkspacePath(config.default_workspace_root ?? "", ""),
                });
              }}
              onSubmit={handleCreateWorkspace}
            />
          )}
        </div>
      </div>
    </>
  );
}

function OverviewPanel({
  currentWorkspace,
  recentCount,
  hasYgoProPath,
  hasGlobalRoot,
  onSwitchView,
}: {
  currentWorkspace: { meta: WorkspaceMeta; path: string } | null;
  recentCount: number;
  hasYgoProPath: boolean;
  hasGlobalRoot: boolean;
  onSwitchView: (view: WorkspaceView) => void;
}) {
  return (
    <section className="workspace-overview-panel">
      <article className="overview-slab">
        <p className="section-kicker">Current</p>
        <h3>{currentWorkspace?.meta.name ?? "No Workspace Open"}</h3>
        <p>{currentWorkspace?.path ?? "Nothing is open yet. Choose a recent item or create a fresh workspace."}</p>
      </article>

      <div className="overview-grid">
        <article className="overview-tile">
          <span>Recent entries</span>
          <strong>{recentCount}</strong>
        </article>
        <article className="overview-tile">
          <span>YGOPro path</span>
          <strong>{hasYgoProPath ? "Ready" : "Missing"}</strong>
        </article>
        <article className="overview-tile">
          <span>Suggested root</span>
          <strong>{hasGlobalRoot ? "Ready" : "Missing"}</strong>
        </article>
      </div>

      <div className="overview-actions">
        <button className="primary-button" type="button" onClick={() => onSwitchView("recent")}>
          Browse Recent Workspaces
        </button>
        <button className="ghost-button" type="button" onClick={() => onSwitchView("create")}>
          Create New Workspace
        </button>
      </div>
    </section>
  );
}

function RecentPanel({
  recentWorkspaces,
  currentWorkspace,
  openPath,
  busyAction,
  recentCount,
  onOpenPathChange,
  onOpen,
}: {
  recentWorkspaces: WorkspaceRegistryEntry[];
  currentWorkspace: { meta: WorkspaceMeta; path: string } | null;
  openPath: string;
  busyAction: string | null;
  recentCount: number;
  onOpenPathChange: (value: string) => void;
  onOpen: (path: string) => Promise<void>;
}) {
  return (
    <section className="workspace-recent-panel">
      <div className="panel-header">
        <div>
          <p className="section-kicker">Open</p>
          <h3>Open Recent or By Path</h3>
        </div>
        <span className="hint-chip">{recentCount} tracked items</span>
      </div>

      <div className="inline-form">
        <label className="field">
          <span>Workspace path</span>
          <input
            value={openPath}
            onChange={(e) => onOpenPathChange(e.target.value)}
            placeholder="D:\\YGOCMG\\workspace-demo"
          />
        </label>
        <button
          className="primary-button"
          type="button"
          disabled={busyAction !== null}
          onClick={() => void onOpen(openPath)}
        >
          {busyAction?.startsWith("open:") ? "Opening..." : "Open Workspace"}
        </button>
      </div>

      {recentWorkspaces.length === 0 ? (
        <p className="empty-state-text">No recent workspaces have been recorded yet.</p>
      ) : (
        <ul className="workspace-list">
          {recentWorkspaces.map((ws) => {
            const isCurrent = currentWorkspace?.meta.id === ws.workspace_id;
            return (
              <li key={ws.workspace_id} className={isCurrent ? "current-workspace" : ""}>
                <div className="workspace-row">
                  <div>
                    <strong>{ws.name_cache ?? "Unnamed Workspace"}</strong>
                    <p>{formatTimestamp(ws.last_opened_at)}</p>
                  </div>
                  {isCurrent && <span className="live-pill">CURRENT</span>}
                </div>
                <code>{ws.path}</code>
                <div className="list-actions">
                  <button
                    className="ghost-button"
                    type="button"
                    disabled={busyAction !== null}
                    onClick={() => void onOpen(ws.path)}
                  >
                    Open
                  </button>
                </div>
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
  hasGlobalRoot,
  busyAction,
  defaultRoot,
  onFormChange,
  onPathTouched,
  onReset,
  onSubmit,
}: {
  createForm: CreateWorkspaceForm;
  hasGlobalRoot: boolean;
  busyAction: string | null;
  defaultRoot: string;
  onFormChange: (updater: (prev: CreateWorkspaceForm) => CreateWorkspaceForm) => void;
  onPathTouched: () => void;
  onReset: () => void;
  onSubmit: (event: React.FormEvent<HTMLFormElement>) => void;
}) {
  return (
    <section className="workspace-create-panel">
      <div className="panel-header">
        <div>
          <p className="section-kicker">Create</p>
          <h3>Create and Open a Workspace</h3>
        </div>
        <span className="hint-chip">
          {hasGlobalRoot ? "Suggested from default root" : "Manual full path required"}
        </span>
      </div>

      <form className="form-stack" onSubmit={onSubmit}>
        <label className="field">
          <span>Workspace name</span>
          <input
            value={createForm.name}
            onChange={(e) => onFormChange((c) => ({ ...c, name: e.target.value }))}
            placeholder="OCG Custom Lab"
          />
        </label>

        <label className="field">
          <span>Description</span>
          <textarea
            rows={3}
            value={createForm.description}
            onChange={(e) => onFormChange((c) => ({ ...c, description: e.target.value }))}
            placeholder="Optional notes for what this workspace is for."
          />
        </label>

        <label className="field">
          <span>Workspace path</span>
          <input
            value={createForm.path}
            onChange={(e) => {
              onPathTouched();
              onFormChange((c) => ({ ...c, path: e.target.value }));
            }}
            placeholder="D:\\YGOCMG\\workspaces\\ocg-custom-lab"
          />
        </label>

        <div className="form-actions">
          <button className="primary-button" type="submit" disabled={busyAction !== null}>
            {busyAction === "create" ? "Creating..." : "Create and Open"}
          </button>
          <button className="ghost-button" type="button" disabled={busyAction !== null} onClick={onReset}>
            Reset
          </button>
        </div>
      </form>
    </section>
  );
}
