import type { LanguageCode } from "../contracts/common";
import type { GlobalConfig, TextLanguageProfile } from "../contracts/config";

export const LEGACY_DEFAULT_LANGUAGE = "default";

export function visibleTextLanguages(catalog: TextLanguageProfile[]): TextLanguageProfile[] {
  return catalog.filter((language) => !language.hidden);
}

export function normalizeLanguageId(id: string): LanguageCode {
  return id.trim();
}

export function isLegacyDefaultLanguage(id: string): boolean {
  return normalizeLanguageId(id) === LEGACY_DEFAULT_LANGUAGE;
}

export function languageExists(catalog: TextLanguageProfile[], id: string): boolean {
  const normalized = normalizeLanguageId(id);
  return visibleTextLanguages(catalog).some((language) => language.id === normalized);
}

export function languageLabel(catalog: TextLanguageProfile[], id: string): string {
  const normalized = normalizeLanguageId(id);
  const profile = catalog.find((language) => language.id === normalized);
  if (profile) return `${profile.label} (${profile.id})`;
  if (isLegacyDefaultLanguage(normalized)) return "Legacy default";
  return normalized || "Unselected";
}

export function compactLanguageLabel(catalog: TextLanguageProfile[], id: string): string {
  const normalized = normalizeLanguageId(id);
  const profile = catalog.find((language) => language.id === normalized);
  return profile?.label || normalized || "Unselected";
}

export function uniqueLanguageOrder(languages: string[]): LanguageCode[] {
  const next: LanguageCode[] = [];
  for (const language of languages) {
    const normalized = normalizeLanguageId(language);
    if (normalized && !next.includes(normalized)) next.push(normalized);
  }
  return next;
}

export function preferredAuthoringLanguage(config: GlobalConfig): LanguageCode {
  const visible = visibleTextLanguages(config.text_language_catalog);
  return (
    visible.find((language) => language.id === "en-US")?.id ??
    visible[0]?.id ??
    "en-US"
  );
}

export function preferredImportSourceLanguage(config: GlobalConfig): LanguageCode {
  const visible = visibleTextLanguages(config.text_language_catalog);
  return (
    (config.standard_pack_source_language &&
    languageExists(config.text_language_catalog, config.standard_pack_source_language)
      ? config.standard_pack_source_language
      : null) ??
    (languageExists(config.text_language_catalog, config.app_language) ? config.app_language : null) ??
    visible.find((language) => language.id === "zh-CN")?.id ??
    visible[0]?.id ??
    "zh-CN"
  );
}

export function validateCustomLanguageId(id: string): string | null {
  const normalized = normalizeLanguageId(id);
  if (!normalized) return "Language id is required.";
  if (isLegacyDefaultLanguage(normalized)) return "default is reserved for legacy data.";
  if (!normalized.startsWith("x-") && !/^[a-z]{2,3}(-[A-Za-z0-9]{2,8})*$/.test(normalized)) {
    return "Use a BCP-style id such as fr-FR, or a custom id beginning with x-.";
  }
  if (normalized.startsWith("x-") && !/^x-[A-Za-z0-9]+(-[A-Za-z0-9]+)*$/.test(normalized)) {
    return "Custom ids must begin with x- and use letters, numbers, and hyphens.";
  }
  return null;
}
