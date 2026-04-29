import { useState } from "react";
import { useShellStore } from "../../shared/stores/shellStore";
import { configApi } from "../../shared/api/configApi";
import type { GlobalConfig, TextLanguageProfile } from "../../shared/contracts/config";
import { normalizeNullablePath, parseNumberInput, formatError } from "../../shared/utils/format";
import {
  languageExists,
  languageLabel,
  normalizeLanguageId,
  validateCustomLanguageId,
  visibleTextLanguages,
} from "../../shared/utils/language";
import { TextLanguagePicker } from "../language/TextLanguagePicker";

type SettingsTab = "general" | "languages" | "standardPack" | "codePolicy";

interface CustomLanguageDraft {
  id: string;
  label: string;
  error: string | null;
}

export interface SettingsModalProps {
  config: GlobalConfig;
  onConfigSaved: (next: GlobalConfig) => void;
  onNotice: (tone: "success" | "warning" | "error", title: string, detail: string) => void;
}

export function SettingsModal({ config, onConfigSaved, onNotice }: SettingsModalProps) {
  const closeModal = useShellStore((s) => s.closeModal);

  const [activeTab, setActiveTab] = useState<SettingsTab>("general");
  const [draft, setDraft] = useState<GlobalConfig>(config);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [customLanguage, setCustomLanguage] = useState<CustomLanguageDraft>({
    id: "",
    label: "",
    error: null,
  });

  const dirty = JSON.stringify(config) !== JSON.stringify(draft);
  const visibleLanguages = visibleTextLanguages(draft.text_language_catalog);

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

  function handleAddCustomLanguage() {
    const id = normalizeLanguageId(customLanguage.id);
    const label = customLanguage.label.trim();
    const idError = validateCustomLanguageId(id);
    if (idError) {
      setCustomLanguage({ ...customLanguage, id, error: idError });
      return;
    }
    if (!label) {
      setCustomLanguage({ ...customLanguage, id, error: "Language label is required." });
      return;
    }
    if (languageExists(draft.text_language_catalog, id) || draft.text_language_catalog.some((language) => language.id === id)) {
      setCustomLanguage({ ...customLanguage, id, error: "Language id already exists." });
      return;
    }

    const profile: TextLanguageProfile = {
      id,
      label,
      kind: "custom",
      hidden: false,
      last_used_at: null,
    };
    setDraft({
      ...draft,
      text_language_catalog: [...draft.text_language_catalog, profile],
    });
    setCustomLanguage({ id: "", label: "", error: null });
  }

  function updateLanguageLabel(id: string, label: string) {
    setDraft({
      ...draft,
      text_language_catalog: draft.text_language_catalog.map((language) =>
        language.id === id ? { ...language, label } : language,
      ),
    });
  }

  function toggleCustomLanguageHidden(id: string) {
    setDraft({
      ...draft,
      text_language_catalog: draft.text_language_catalog.map((language) =>
        language.id === id ? { ...language, hidden: !language.hidden } : language,
      ),
      standard_pack_source_language:
        draft.standard_pack_source_language === id ? null : draft.standard_pack_source_language,
    });
  }

  return (
    <>
      <header className="modal-header">
        <div>
          <p className="eyebrow">Settings</p>
          <h2>Global Settings</h2>
        </div>
        <div className="settings-header-right">
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
          <button className="modal-close-button" type="button" onClick={closeModal}>
            Close
          </button>
        </div>
      </header>

      <div className="modal-body settings-modal-body">
        <aside className="modal-tabs">
          <button type="button" className={activeTab === "general" ? "active" : ""} onClick={() => setActiveTab("general")}>
            General
          </button>
          <button type="button" className={activeTab === "languages" ? "active" : ""} onClick={() => setActiveTab("languages")}>
            Text Languages
          </button>
          <button type="button" className={activeTab === "standardPack" ? "active" : ""} onClick={() => setActiveTab("standardPack")}>
            Standard Pack
          </button>
          <button type="button" className={activeTab === "codePolicy" ? "active" : ""} onClick={() => setActiveTab("codePolicy")}>
            Code Policy
          </button>
        </aside>

        <div className="modal-panel">
          {activeTab === "general" && (
            <div className="settings-tab-content">
              <section className="settings-group">
                <div className="group-heading">
                  <p className="section-kicker">Program</p>
                  <h4>Language</h4>
                </div>

                <label className="field">
                  <span>App language</span>
                  <TextLanguagePicker
                    catalog={draft.text_language_catalog}
                    value={draft.app_language}
                    onChange={(appLanguage) => setDraft({ ...draft, app_language: appLanguage })}
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
            </div>
          )}

          {activeTab === "languages" && (
            <div className="settings-tab-content">
              <section className="settings-group">
                <div className="group-heading">
                  <p className="section-kicker">Text Languages</p>
                  <h4>Catalog</h4>
                </div>

                <div className="language-catalog-list">
                  {draft.text_language_catalog.map((language) => (
                    <div key={language.id} className={`language-catalog-row ${language.hidden ? "hidden" : ""}`}>
                      <span className="language-kind-badge">{language.kind}</span>
                      <code>{language.id}</code>
                      <input
                        value={language.label}
                        disabled={language.kind === "builtin"}
                        onChange={(event) => updateLanguageLabel(language.id, event.target.value)}
                        title={languageLabel(draft.text_language_catalog, language.id)}
                      />
                      {language.kind === "custom" && (
                        <button
                          type="button"
                          className="ghost-button"
                          onClick={() => toggleCustomLanguageHidden(language.id)}
                        >
                          {language.hidden ? "Show" : "Hide"}
                        </button>
                      )}
                    </div>
                  ))}
                </div>

                <div className="custom-language-add">
                  <input
                    value={customLanguage.id}
                    onChange={(event) =>
                      setCustomLanguage({ ...customLanguage, id: event.target.value, error: null })
                    }
                    placeholder="x-custom or fr-FR"
                  />
                  <input
                    value={customLanguage.label}
                    onChange={(event) =>
                      setCustomLanguage({ ...customLanguage, label: event.target.value, error: null })
                    }
                    placeholder="Label"
                  />
                  <button type="button" className="ghost-button" onClick={handleAddCustomLanguage}>
                    Add
                  </button>
                </div>
                {customLanguage.error && <div className="settings-inline-error">{customLanguage.error}</div>}
              </section>
            </div>
          )}

          {activeTab === "standardPack" && (
            <div className="settings-tab-content">
              <section className="settings-group">
                <div className="group-heading">
                  <p className="section-kicker">Standard Pack</p>
                  <h4>Source Language</h4>
                </div>

                <label className="field">
                  <span>Imported text language</span>
                  <TextLanguagePicker
                    catalog={draft.text_language_catalog}
                    value={draft.standard_pack_source_language ?? ""}
                    allowEmpty
                    placeholder="Select source language"
                    onChange={(sourceLanguage) =>
                      setDraft({ ...draft, standard_pack_source_language: sourceLanguage || null })
                    }
                  />
                </label>
                <span className="field-hint">
                  {visibleLanguages.length} visible language{visibleLanguages.length === 1 ? "" : "s"}
                </span>
              </section>
            </div>
          )}

          {activeTab === "codePolicy" && (
            <div className="settings-tab-content">
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
          )}
        </div>
      </div>
    </>
  );
}
