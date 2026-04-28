import { useState, useEffect } from "react";
import { useShellStore } from "../../shared/stores/shellStore";
import { packApi } from "../../shared/api/packApi";
import type { PackMetadata, PackOverview } from "../../shared/contracts/pack";
import { formatTimestamp, formatError } from "../../shared/utils/format";
import { ImportPackPanel } from "./ImportPackPanel";

type AddPackTab = "openPack" | "createPack" | "importPack";

interface CreatePackForm {
  name: string;
  author: string;
  version: string;
  description: string;
  displayLanguageOrder: string;
  defaultExportLanguage: string;
}

const EMPTY_CREATE_FORM: CreatePackForm = {
  name: "",
  author: "",
  version: "1.0.0",
  description: "",
  displayLanguageOrder: "en-US",
  defaultExportLanguage: "",
};

export interface AddPackModalProps {
  hasWorkspace: boolean;
  onPackOpened: (packId: string, metadata: PackMetadata) => void;
  onPackCreated: (packId: string, metadata: PackMetadata) => void;
  onOverviewsRefreshed: (overviews: PackOverview[]) => void;
  onNotice: (tone: "success" | "warning" | "error", title: string, detail: string) => void;
}

export function AddPackModal({
  hasWorkspace,
  onPackOpened,
  onPackCreated,
  onOverviewsRefreshed,
  onNotice,
}: AddPackModalProps) {
  const closeModal = useShellStore((s) => s.closeModal);
  const addPackTab = useShellStore((s) => s.modal?.addPackTab ?? "openPack");
  const setAddPackTab = useShellStore((s) => s.setAddPackTab);
  const openPackIds = useShellStore((s) => s.openPackIds);
  const workspaceId = useShellStore((s) => s.workspaceId);

  const [overviews, setOverviews] = useState<PackOverview[]>([]);
  const [loadingOverviews, setLoadingOverviews] = useState(false);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [createForm, setCreateForm] = useState<CreatePackForm>(EMPTY_CREATE_FORM);

  useEffect(() => {
    if (!hasWorkspace) return;
    let active = true;
    setLoadingOverviews(true);
    packApi
      .listPackOverviews()
      .then((list) => {
        if (!active) return;
        setOverviews(list);
        onOverviewsRefreshed(list);
      })
      .catch(() => {})
      .finally(() => {
        if (active) setLoadingOverviews(false);
      });
    return () => { active = false; };
  }, [hasWorkspace]);

  const unopenedPacks = overviews.filter((p) => !openPackIds.includes(p.id));

  async function handleOpenPack(packId: string) {
    setBusyAction(`open:${packId}`);
    try {
      const metadata = await packApi.openPack({ packId });
      onPackOpened(packId, metadata);
      onNotice("success", "Pack opened", `Pack is now active in the sidebar.`);
      closeModal();
    } catch (err) {
      onNotice("error", "Failed to open pack", formatError(err));
    } finally {
      setBusyAction(null);
    }
  }

  async function handleCreatePack(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const name = createForm.name.trim();
    const author = createForm.author.trim();
    const version = createForm.version.trim();
    if (!name || !author || !version) {
      onNotice("warning", "Missing fields", "Name, author, and version are required.");
      return;
    }

    setBusyAction("create");
    try {
      const langOrder = createForm.displayLanguageOrder
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean);
      const defExport = createForm.defaultExportLanguage.trim() || null;
      const desc = createForm.description.trim() || null;

      const createdMeta = await packApi.createPack({
        name,
        author,
        version,
        description: desc,
        displayLanguageOrder: langOrder,
        defaultExportLanguage: defExport,
      });

      const openedMeta = await packApi.openPack({ packId: createdMeta.id });
      onPackCreated(createdMeta.id, openedMeta);
      setCreateForm(EMPTY_CREATE_FORM);
      onNotice("success", "Pack created", `${createdMeta.name} has been created.`);
      closeModal();
    } catch (err) {
      onNotice("error", "Failed to create pack", formatError(err));
    } finally {
      setBusyAction(null);
    }
  }

  return (
    <>
      <header className="modal-header">
        <div>
          <p className="eyebrow">Pack</p>
          <h2>Add Pack</h2>
        </div>
        <button className="modal-close-button" type="button" onClick={closeModal}>
          Close
        </button>
      </header>

      <div className="modal-body workspace-modal-body">
        <aside className="modal-tabs">
          <button
            type="button"
            className={addPackTab === "openPack" ? "active" : ""}
            onClick={() => setAddPackTab("openPack")}
          >
            Open Pack
          </button>
          <button
            type="button"
            className={addPackTab === "createPack" ? "active" : ""}
            onClick={() => setAddPackTab("createPack")}
          >
            Create Pack
          </button>
          <button
            type="button"
            className={addPackTab === "importPack" ? "active" : ""}
            onClick={() => setAddPackTab("importPack")}
          >
            Import Pack
          </button>
        </aside>

        <div className="modal-panel">
          {!hasWorkspace ? (
            <p className="empty-state-text">
              Open a workspace first before managing packs.
            </p>
          ) : addPackTab === "openPack" ? (
            <OpenPackPanel
              overviews={unopenedPacks}
              loading={loadingOverviews}
              busyAction={busyAction}
              onOpen={handleOpenPack}
            />
          ) : addPackTab === "createPack" ? (
            <CreatePackPanel
              form={createForm}
              busyAction={busyAction}
              onFormChange={setCreateForm}
              onReset={() => setCreateForm(EMPTY_CREATE_FORM)}
              onSubmit={handleCreatePack}
            />
          ) : workspaceId ? (
            <ImportPackPanel
              workspaceId={workspaceId}
              onPackOpened={onPackOpened}
              onNotice={onNotice}
              closeModal={closeModal}
            />
          ) : null}
        </div>
      </div>
    </>
  );
}

function OpenPackPanel({
  overviews,
  loading,
  busyAction,
  onOpen,
}: {
  overviews: PackOverview[];
  loading: boolean;
  busyAction: string | null;
  onOpen: (packId: string) => void;
}) {
  return (
    <section className="workspace-recent-panel">
      <div className="panel-header">
        <div>
          <p className="section-kicker">Open</p>
          <h3>Open an Existing Pack</h3>
        </div>
        <span className="hint-chip">{overviews.length} available</span>
      </div>

      {loading ? (
        <p className="empty-state-text">Loading packs...</p>
      ) : overviews.length === 0 ? (
        <p className="empty-state-text">
          All packs are already open, or no packs exist yet. Create one using the Create Pack tab.
        </p>
      ) : (
        <ul className="workspace-list">
          {overviews.map((pack) => (
            <li key={pack.id}>
              <div className="workspace-row">
                <div>
                  <strong>{pack.name}</strong>
                  <p>
                    {pack.author} &middot; v{pack.version} &middot;{" "}
                    {pack.card_count} card{pack.card_count !== 1 ? "s" : ""}
                  </p>
                </div>
                <div className="list-actions">
                  <button
                    className="ghost-button"
                    type="button"
                    disabled={busyAction !== null}
                    onClick={() => onOpen(pack.id)}
                  >
                    {busyAction === `open:${pack.id}` ? "Opening..." : "Open"}
                  </button>
                </div>
              </div>
              <code>{formatTimestamp(pack.updated_at)}</code>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function CreatePackPanel({
  form,
  busyAction,
  onFormChange,
  onReset,
  onSubmit,
}: {
  form: CreatePackForm;
  busyAction: string | null;
  onFormChange: (form: CreatePackForm) => void;
  onReset: () => void;
  onSubmit: (e: React.FormEvent<HTMLFormElement>) => void;
}) {
  return (
    <section className="workspace-create-panel">
      <div className="panel-header">
        <div>
          <p className="section-kicker">Create</p>
          <h3>Create a New Pack</h3>
        </div>
      </div>

      <form className="form-stack" onSubmit={onSubmit}>
        <label className="field">
          <span>Pack name</span>
          <input
            value={form.name}
            onChange={(e) => onFormChange({ ...form, name: e.target.value })}
            placeholder="My Custom Pack"
          />
        </label>

        <div className="pack-form-row">
          <label className="field">
            <span>Author</span>
            <input
              value={form.author}
              onChange={(e) => onFormChange({ ...form, author: e.target.value })}
              placeholder="Author Name"
            />
          </label>
          <label className="field">
            <span>Version</span>
            <input
              value={form.version}
              onChange={(e) => onFormChange({ ...form, version: e.target.value })}
              placeholder="1.0.0"
            />
          </label>
        </div>

        <label className="field">
          <span>Description</span>
          <textarea
            rows={2}
            value={form.description}
            onChange={(e) => onFormChange({ ...form, description: e.target.value })}
            placeholder="Optional description for the pack."
          />
        </label>

        <div className="pack-form-row">
          <label className="field">
            <span>Display languages</span>
            <input
              value={form.displayLanguageOrder}
              onChange={(e) =>
                onFormChange({ ...form, displayLanguageOrder: e.target.value })
              }
              placeholder="en-US, zh-CN, ja-JP"
            />
          </label>
          <label className="field">
            <span>Default export language</span>
            <input
              value={form.defaultExportLanguage}
              onChange={(e) =>
                onFormChange({ ...form, defaultExportLanguage: e.target.value })
              }
              placeholder="en-US"
            />
          </label>
        </div>

        <div className="form-actions">
          <button
            className="primary-button"
            type="submit"
            disabled={busyAction !== null}
          >
            {busyAction === "create" ? "Creating..." : "Create Pack"}
          </button>
          <button
            className="ghost-button"
            type="button"
            disabled={busyAction !== null}
            onClick={onReset}
          >
            Reset
          </button>
        </div>
      </form>
    </section>
  );
}
