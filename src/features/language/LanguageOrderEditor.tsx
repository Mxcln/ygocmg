import { useMemo, useState } from "react";
import type { TextLanguageProfile } from "../../shared/contracts/config";
import {
  compactLanguageLabel,
  languageLabel,
  normalizeLanguageId,
  uniqueLanguageOrder,
  visibleTextLanguages,
} from "../../shared/utils/language";

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
    <div className="language-order-editor">
      <div className="language-order-list">
        {languages.map((language, index) => (
          <span key={language} className={`language-order-chip ${existing.has(language) ? "legacy" : ""}`}>
            <span title={languageLabel(catalog, language)}>{compactLanguageLabel(catalog, language)}</span>
            <button
              type="button"
              className="language-chip-button"
              disabled={disabled || index === 0}
              onClick={() => move(index, index - 1)}
              title="Move earlier"
            >
              ^
            </button>
            <button
              type="button"
              className="language-chip-button"
              disabled={disabled || index === languages.length - 1}
              onClick={() => move(index, index + 1)}
              title="Move later"
            >
              v
            </button>
            <button
              type="button"
              className="language-chip-button"
              disabled={disabled}
              onClick={() => remove(language)}
              title="Remove"
            >
              x
            </button>
          </span>
        ))}
        {languages.length === 0 && <span className="language-order-empty">No languages selected</span>}
      </div>
      <select
        className="text-language-picker"
        value={pendingLanguage}
        disabled={disabled || addable.length === 0}
        onChange={(event) => add(event.target.value)}
      >
        <option value="">{addable.length === 0 ? "No more languages" : "Add language"}</option>
        {addable.map((language) => (
          <option key={language.id} value={language.id}>
            {languageLabel(catalog, language.id)}
          </option>
        ))}
      </select>
    </div>
  );
}
