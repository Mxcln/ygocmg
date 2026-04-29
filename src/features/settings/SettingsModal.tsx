import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
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
import shared from "../../shared/styles/shared.module.css";
import { TextLanguagePicker } from "../language/TextLanguagePicker";
import styles from "./SettingsModal.module.css";

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
      <header className={shared.modalHeader}>
        <h2>Global Settings</h2>
        <div className={styles.settingsHeaderRight}>
          <span className={`${shared.hintChip} ${dirty ? "dirty" : ""}`}>
            {dirty ? "Unsaved changes" : "Synced"}
          </span>
          <button
            className={shared.primaryButton}
            type="button"
            disabled={busyAction !== null || !dirty}
            onClick={() => void handleSave()}
          >
            {busyAction === "save" ? "Saving..." : "Save Settings"}
          </button>
          <button className={shared.modalCloseButton} type="button" onClick={closeModal}>
            Close
          </button>
        </div>
      </header>

      <div className={`${shared.modalBody} ${shared.workspaceModalBody}`}>
        <aside className={shared.modalTabs}>
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

        <div className={shared.modalPanel}>
          {activeTab === "general" && (
            <div className={styles.settingsTabContent}>
              <section className={styles.settingsGroup}>
                <h4 className={styles.groupTitle}>Language</h4>

                <label className={shared.field}>
                  <span>App language</span>
                  <TextLanguagePicker
                    catalog={draft.text_language_catalog}
                    value={draft.app_language}
                    onChange={(appLanguage) => setDraft({ ...draft, app_language: appLanguage })}
                  />
                </label>
              </section>

              <section className={styles.settingsGroup}>
                <h4 className={styles.groupTitle}>External Paths</h4>

                <div className={shared.field}>
                  <span>YGOPro path</span>
                  <div className={shared.filePickerRow}>
                    <input
                      value={draft.ygopro_path ?? ""}
                      onChange={(e) => setDraft({ ...draft, ygopro_path: normalizeNullablePath(e.target.value) })}
                      placeholder="D:\\Games\\YGOPro"
                    />
                    <button
                      type="button"
                      className={shared.ghostButton}
                      onClick={async () => {
                        const selected = await open({ directory: true, title: "Select YGOPro Directory" });
                        if (typeof selected === "string") setDraft({ ...draft, ygopro_path: selected || null });
                      }}
                    >
                      Browse
                    </button>
                    {draft.ygopro_path && (
                      <button
                        type="button"
                        className={shared.ghostButton}
                        onClick={() => setDraft({ ...draft, ygopro_path: null })}
                      >
                        Clear
                      </button>
                    )}
                  </div>
                </div>

                <div className={shared.field}>
                  <span>External text editor path</span>
                  <div className={shared.filePickerRow}>
                    <input
                      value={draft.external_text_editor_path ?? ""}
                      onChange={(e) =>
                        setDraft({ ...draft, external_text_editor_path: normalizeNullablePath(e.target.value) })
                      }
                      placeholder="C:\\Program Files\\VS Code\\Code.exe"
                    />
                    <button
                      type="button"
                      className={shared.ghostButton}
                      onClick={async () => {
                        const selected = await open({
                          title: "Select Text Editor",
                          filters: [{ name: "Executable", extensions: ["exe"] }],
                        });
                        if (typeof selected === "string")
                          setDraft({ ...draft, external_text_editor_path: selected || null });
                      }}
                    >
                      Browse
                    </button>
                    {draft.external_text_editor_path && (
                      <button
                        type="button"
                        className={shared.ghostButton}
                        onClick={() => setDraft({ ...draft, external_text_editor_path: null })}
                      >
                        Clear
                      </button>
                    )}
                  </div>
                </div>
              </section>
            </div>
          )}

          {activeTab === "languages" && (
            <div className={styles.settingsTabContent}>
              <section className={styles.settingsGroup}>

                <div className={styles.languageCatalogList}>
                  {draft.text_language_catalog.map((language) => (
                    <div key={language.id} className={`${styles.languageCatalogRow} ${language.hidden ? "hidden" : ""}`}>
                      <span className={styles.languageKindBadge}>{language.kind}</span>
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
                          className={shared.ghostButton}
                          onClick={() => toggleCustomLanguageHidden(language.id)}
                        >
                          {language.hidden ? "Show" : "Hide"}
                        </button>
                      )}
                    </div>
                  ))}
                </div>

                <div className={styles.customLanguageAdd}>
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
                  <button type="button" className={shared.ghostButton} onClick={handleAddCustomLanguage}>
                    Add
                  </button>
                </div>
                {customLanguage.error && <div className={styles.settingsInlineError}>{customLanguage.error}</div>}
              </section>
            </div>
          )}

          {activeTab === "standardPack" && (
            <div className={styles.settingsTabContent}>
              <section className={styles.settingsGroup}>
                <h4 className={styles.groupTitle}>Source Language</h4>

                <label className={shared.field}>
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
                <span className={shared.fieldHint}>
                  {visibleLanguages.length} visible language{visibleLanguages.length === 1 ? "" : "s"}
                </span>
              </section>
            </div>
          )}

          {activeTab === "codePolicy" && (
            <div className={styles.settingsTabContent}>
              <section className={styles.settingsGroup}>
                <h4 className={styles.groupTitle}>Custom Card Numbering</h4>

                <label className={shared.field}>
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

                <label className={shared.field}>
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

                <label className={shared.field}>
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
