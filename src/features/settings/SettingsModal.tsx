import { useState } from "react";
import { useShellStore } from "../../shared/stores/shellStore";
import { configApi } from "../../shared/api/configApi";
import type { GlobalConfig } from "../../shared/contracts/config";
import { normalizeNullablePath, parseNumberInput, formatError } from "../../shared/utils/format";

export interface SettingsModalProps {
  config: GlobalConfig;
  onConfigSaved: (next: GlobalConfig) => void;
  onNotice: (tone: "success" | "warning" | "error", title: string, detail: string) => void;
}

export function SettingsModal({ config, onConfigSaved, onNotice }: SettingsModalProps) {
  const closeModal = useShellStore((s) => s.closeModal);

  const [draft, setDraft] = useState<GlobalConfig>(config);
  const [busyAction, setBusyAction] = useState<string | null>(null);

  const dirty = JSON.stringify(config) !== JSON.stringify(draft);

  async function handleSave() {
    setBusyAction("save");
    try {
      const next = await configApi.saveConfig(draft);
      onConfigSaved(next);
      onNotice("success", "Settings saved", "Program-level configuration has been written successfully.");
    } catch (err) {
      onNotice("error", "Failed to save settings", formatError(err));
    } finally {
      setBusyAction(null);
    }
  }

  return (
    <>
      <header className="modal-header">
        <div>
          <p className="eyebrow">Settings</p>
          <h2>Global Settings</h2>
        </div>
        <button className="modal-close-button" type="button" onClick={closeModal}>
          Close
        </button>
      </header>

      <div className="modal-body settings-modal-body">
        <section className="settings-toolbar">
          <div>
            <p className="section-kicker">Program Settings</p>
            <h3>Desktop-level configuration</h3>
          </div>
          <div className="header-actions">
            <span className={`hint-chip ${dirty ? "dirty" : ""}`}>
              {dirty ? "Unsaved changes" : "Synced"}
            </span>
            <button
              className="primary-button"
              type="button"
              disabled={busyAction !== null || !dirty}
              onClick={() => void handleSave()}
            >
              {busyAction === "save" ? "Saving..." : "Save Settings"}
            </button>
          </div>
        </section>

        <div className="settings-grid">
          <section className="settings-group">
            <div className="group-heading">
              <p className="section-kicker">Program</p>
              <h4>Language and workspace root</h4>
            </div>

            <label className="field">
              <span>App language</span>
              <input
                list="language-suggestions"
                value={draft.app_language}
                onChange={(e) => setDraft({ ...draft, app_language: e.target.value })}
                placeholder="en-US"
              />
            </label>

            <label className="field">
              <span>Default workspace root</span>
              <input
                value={draft.default_workspace_root ?? ""}
                onChange={(e) =>
                  setDraft({ ...draft, default_workspace_root: normalizeNullablePath(e.target.value) })
                }
                placeholder="D:\\YGOCMG\\workspaces"
              />
            </label>
          </section>

          <section className="settings-group">
            <div className="group-heading">
              <p className="section-kicker">Tools</p>
              <h4>External paths</h4>
            </div>

            <label className="field">
              <span>YGOPro path</span>
              <input
                value={draft.ygopro_path ?? ""}
                onChange={(e) => setDraft({ ...draft, ygopro_path: normalizeNullablePath(e.target.value) })}
                placeholder="D:\\Games\\YGOPro"
              />
            </label>

            <label className="field">
              <span>External text editor path</span>
              <input
                value={draft.external_text_editor_path ?? ""}
                onChange={(e) =>
                  setDraft({ ...draft, external_text_editor_path: normalizeNullablePath(e.target.value) })
                }
                placeholder="C:\\Program Files\\VS Code\\Code.exe"
              />
            </label>
          </section>

          <section className="settings-group">
            <div className="group-heading">
              <p className="section-kicker">Code Policy</p>
              <h4>Custom card numbering</h4>
            </div>

            <label className="field">
              <span>Recommended code minimum</span>
              <input
                type="number"
                min={0}
                value={draft.custom_code_recommended_min}
                onChange={(e) =>
                  setDraft({
                    ...draft,
                    custom_code_recommended_min: parseNumberInput(e.target.value, draft.custom_code_recommended_min),
                  })
                }
              />
            </label>

            <label className="field">
              <span>Recommended code maximum</span>
              <input
                type="number"
                min={0}
                value={draft.custom_code_recommended_max}
                onChange={(e) =>
                  setDraft({
                    ...draft,
                    custom_code_recommended_max: parseNumberInput(e.target.value, draft.custom_code_recommended_max),
                  })
                }
              />
            </label>

            <label className="field">
              <span>Minimum gap</span>
              <input
                type="number"
                min={1}
                value={draft.custom_code_min_gap}
                onChange={(e) =>
                  setDraft({
                    ...draft,
                    custom_code_min_gap: parseNumberInput(e.target.value, draft.custom_code_min_gap),
                  })
                }
              />
            </label>
          </section>
        </div>

        <datalist id="language-suggestions">
          <option value="en-US" />
          <option value="zh-CN" />
          <option value="ja-JP" />
        </datalist>
      </div>
    </>
  );
}
