import { useMemo, useState } from "react";
import type { TextLanguageProfile } from "../../shared/contracts/config";
import { useAppI18n } from "../../shared/i18n";
import {
  compactLanguageLabel,
  languageLabel,
  normalizeLanguageId,
  uniqueLanguageOrder,
  visibleTextLanguages,
} from "../../shared/utils/language";
import styles from "./LanguageOrderEditor.module.css";
import pickerStyles from "./TextLanguagePicker.module.css";

interface LanguageOrderEditorProps {
  catalog: TextLanguageProfile[];
  value: string[];
  onChange: (languages: string[]) => void;
  disabled?: boolean;
  existingLanguages?: string[];
}

export function LanguageOrderEditor({
  catalog,
  value,
  onChange,
  disabled = false,
  existingLanguages = [],
}: LanguageOrderEditorProps) {
  const { t } = useAppI18n();
  const languages = useMemo(() => uniqueLanguageOrder(value), [value]);
  const visible = visibleTextLanguages(catalog);
  const [pendingLanguage, setPendingLanguage] = useState("");
  const existing = new Set(existingLanguages.map(normalizeLanguageId).filter(Boolean));

  const addable = visible.filter((language) => !languages.includes(language.id));

  function move(from: number, to: number) {
    if (disabled || to < 0 || to >= languages.length) return;
    const next = [...languages];
    const [item] = next.splice(from, 1);
    next.splice(to, 0, item);
    onChange(next);
  }

  function remove(language: string) {
    if (disabled) return;
    onChange(languages.filter((current) => current !== language));
  }

  function add(language: string) {
    const normalized = normalizeLanguageId(language);
    if (!normalized || languages.includes(normalized)) return;
    onChange([...languages, normalized]);
    setPendingLanguage("");
  }

  return (
    <div className={styles.languageOrderEditor}>
      <div className={styles.languageOrderList}>
        {languages.map((language, index) => (
          <span
            key={language}
            className={`${styles.languageOrderChip}${existing.has(language) ? ` ${styles.languageOrderChipLegacy}` : ""}`}
          >
            <span title={languageLabel(catalog, language)}>{compactLanguageLabel(catalog, language)}</span>
            <button
              type="button"
              className={styles.languageChipButton}
              disabled={disabled || index === 0}
              onClick={() => move(index, index - 1)}
              title={t("language.moveEarlier")}
            >
              ^
            </button>
            <button
              type="button"
              className={styles.languageChipButton}
              disabled={disabled || index === languages.length - 1}
              onClick={() => move(index, index + 1)}
              title={t("language.moveLater")}
            >
              v
            </button>
            <button
              type="button"
              className={styles.languageChipButton}
              disabled={disabled}
              onClick={() => remove(language)}
              title={t("language.remove")}
            >
              x
            </button>
          </span>
        ))}
        {languages.length === 0 && <span className={styles.languageOrderEmpty}>{t("language.noLanguagesSelected")}</span>}
      </div>
      <select
        className={pickerStyles.textLanguagePicker}
        value={pendingLanguage}
        disabled={disabled || addable.length === 0}
        onChange={(event) => add(event.target.value)}
      >
        <option value="">{addable.length === 0 ? t("language.noMoreLanguages") : t("language.addLanguage")}</option>
        {addable.map((language) => (
          <option key={language.id} value={language.id}>
            {languageLabel(catalog, language.id)}
          </option>
        ))}
      </select>
    </div>
  );
}
