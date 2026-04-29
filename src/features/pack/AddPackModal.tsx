import { useState, useEffect } from "react";
import { useShellStore } from "../../shared/stores/shellStore";
import { packApi } from "../../shared/api/packApi";
import type { GlobalConfig } from "../../shared/contracts/config";
import type { PackMetadata, PackOverview } from "../../shared/contracts/pack";
import { formatTimestamp, formatError } from "../../shared/utils/format";
import { preferredAuthoringLanguage } from "../../shared/utils/language";
import { LanguageOrderEditor } from "../language/LanguageOrderEditor";
import shared from "../../shared/styles/shared.module.css";
import addPackStyles from "./AddPackModal.module.css";
import { TextLanguagePicker } from "../language/TextLanguagePicker";
import { ImportPackPanel } from "./ImportPackPanel";

type AddPackTab = "openPack" | "createPack" | "importPack";

interface CreatePackForm {
  name: string;
  author: string;
  version: string;
  description: string;
  displayLanguageOrder: string[];
  defaultExportLanguage: string;
}

function emptyCreateForm(config: GlobalConfig): CreatePackForm {
  const language = preferredAuthoringLanguage(config);
  return {
    name: "",
    author: "",
    version: "1.0.0",
    description: "",
    displayLanguageOrder: [language],
    defaultExportLanguage: language,
  };
}

export interface AddPackModalProps {
  config: GlobalConfig;
  hasWorkspace: boolean;
  onPackOpened: (packId: string, metadata: PackMetadata) => void;
  onPackCreated: (packId: string, metadata: PackMetadata) => void;
  onOverviewsRefreshed: (overviews: PackOverview[]) => void;
  onNotice: (tone: "success" | "warning" | "error", title: string, detail: string) => void;
}

export function AddPackModal({
  config,
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
  const [createForm, setCreateForm] = useState<CreatePackForm>(() => emptyCreateForm(config));

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
      const langOrder = createForm.displayLanguageOrder;
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
      setCreateForm(emptyCreateForm(config));
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
      <header className={shared.modalHeader}>
        <h2>Add Pack</h2>
        <button className={shared.modalCloseButton} type="button" onClick={closeModal}>
          Close
        </button>
      </header>

      <div className={`${shared.modalBody} ${shared.workspaceModalBody}`}>
        <aside className={shared.modalTabs}>
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

        <div className={shared.modalPanel}>
          {!hasWorkspace ? (
            <p className={shared.emptyStateText}>
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
              config={config}
              busyAction={busyAction}
              onFormChange={setCreateForm}
              onReset={() => setCreateForm(emptyCreateForm(config))}
              onSubmit={handleCreatePack}
            />
          ) : workspaceId ? (
            <ImportPackPanel
              workspaceId={workspaceId}
              config={config}
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
    <section className={shared.workspaceRecentPanel}>
      {loading ? (
        <p className={shared.emptyStateText}>Loading packs...</p>
      ) : overviews.length === 0 ? (
        <p className={shared.emptyStateText}>
          All packs are already open, or no packs exist yet. Create one using the Create Pack tab.
        </p>
      ) : (
        <ul className={addPackStyles.packList}>
          {overviews.map((pack) => (
            <li key={pack.id}>
              <div className={addPackStyles.workspaceRow}>
                <div>
                  <strong>{pack.name}</strong>
                  <p>
                    {pack.author} &middot; v{pack.version} &middot;{" "}
                    {pack.card_count} card{pack.card_count !== 1 ? "s" : ""}
                  </p>
                </div>
                <div className={shared.listActions}>
                  <button
                    className={shared.ghostButton}
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
  config,
  busyAction,
  onFormChange,
  onReset,
  onSubmit,
}: {
  form: CreatePackForm;
  config: GlobalConfig;
  busyAction: string | null;
  onFormChange: (form: CreatePackForm) => void;
  onReset: () => void;
  onSubmit: (e: React.FormEvent<HTMLFormElement>) => void;
}) {
  const defaultExportValue = form.displayLanguageOrder.includes(form.defaultExportLanguage)
    ? form.defaultExportLanguage
    : form.displayLanguageOrder[0] ?? "";

  return (
    <section className={shared.workspaceCreatePanel}>
      <form className={shared.formStack} onSubmit={onSubmit}>
        <label className={shared.field}>
          <span>Pack name</span>
          <input
            value={form.name}
            onChange={(e) => onFormChange({ ...form, name: e.target.value })}
            placeholder="My Custom Pack"
          />
        </label>

        <div className={shared.packFormRow}>
          <label className={shared.field}>
            <span>Author</span>
            <input
              value={form.author}
              onChange={(e) => onFormChange({ ...form, author: e.target.value })}
              placeholder="Author Name"
            />
          </label>
          <label className={shared.field}>
            <span>Version</span>
            <input
              value={form.version}
              onChange={(e) => onFormChange({ ...form, version: e.target.value })}
              placeholder="1.0.0"
            />
          </label>
        </div>

        <label className={shared.field}>
          <span>Description</span>
          <textarea
            rows={2}
            value={form.description}
            onChange={(e) => onFormChange({ ...form, description: e.target.value })}
            placeholder="Optional description for the pack."
          />
        </label>

        <div className={`${shared.packFormRow} ${shared.packFormRowLanguage}`}>
          <div className={shared.field}>
            <span>Display languages</span>
            <LanguageOrderEditor
              catalog={config.text_language_catalog}
              value={form.displayLanguageOrder}
              onChange={(displayLanguageOrder) => {
                const defaultExportLanguage = displayLanguageOrder.includes(form.defaultExportLanguage)
                  ? form.defaultExportLanguage
                  : displayLanguageOrder[0] ?? "";
                onFormChange({ ...form, displayLanguageOrder, defaultExportLanguage });
              }}
            />
          </div>
          <label className={shared.field}>
            <span>Default export language</span>
            <TextLanguagePicker
              catalog={config.text_language_catalog}
              value={defaultExportValue}
              existingLanguages={form.displayLanguageOrder}
              onChange={(defaultExportLanguage) =>
                onFormChange({ ...form, defaultExportLanguage })
              }
            />
          </label>
        </div>

        <div className={shared.formActions}>
          <button
            className={shared.primaryButton}
            type="submit"
            disabled={busyAction !== null}
          >
            {busyAction === "create" ? "Creating..." : "Create Pack"}
          </button>
          <button
            className={shared.ghostButton}
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
