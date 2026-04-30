import type { TextLanguageProfile } from "../../shared/contracts/config";
import styles from "./TextLanguagePicker.module.css";
import { useAppI18n } from "../../shared/i18n";
import { languageLabel, normalizeLanguageId, visibleTextLanguages } from "../../shared/utils/language";

interface TextLanguagePickerProps {
  catalog: TextLanguageProfile[];
  value: string;
  onChange: (language: string) => void;
  disabled?: boolean;
  placeholder?: string;
  allowEmpty?: boolean;
  existingLanguages?: string[];
  excludeLanguages?: string[];
  className?: string;
}

export function TextLanguagePicker({
  catalog,
  value,
  onChange,
  disabled = false,
  placeholder,
  allowEmpty = false,
  existingLanguages = [],
  excludeLanguages = [],
  className,
}: TextLanguagePickerProps) {
  const { t } = useAppI18n();
  const excluded = new Set(excludeLanguages.map(normalizeLanguageId).filter(Boolean));
  const visible = visibleTextLanguages(catalog).filter((language) => !excluded.has(language.id));
  const normalized = normalizeLanguageId(value);
  const optionIds = new Set(visible.map((language) => language.id));
  for (const language of existingLanguages) {
    const id = normalizeLanguageId(language);
    if (id && !optionIds.has(id)) optionIds.add(id);
  }
  if (normalized && !optionIds.has(normalized)) optionIds.add(normalized);

  return (
    <select
      className={className ?? styles.textLanguagePicker}
      value={normalized}
      disabled={disabled}
      onChange={(event) => onChange(event.target.value)}
    >
      {allowEmpty && <option value="">{placeholder ?? t("language.select")}</option>}
      {[...optionIds].map((id) => (
        <option key={id} value={id}>
          {languageLabel(catalog, id)}
        </option>
      ))}
    </select>
  );
}
