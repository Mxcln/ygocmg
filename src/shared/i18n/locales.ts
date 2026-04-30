import type { MessageDescriptor } from "react-intl";

export type AppLocale = "en-US" | "zh-CN";

export interface AppLocaleOption {
  id: AppLocale;
  label: string;
}

export const DEFAULT_APP_LOCALE: AppLocale = "en-US";

export const APP_LOCALE_OPTIONS: AppLocaleOption[] = [
  { id: "en-US", label: "English" },
  { id: "zh-CN", label: "Simplified Chinese" },
];

export function isSupportedAppLocale(value: string): value is AppLocale {
  return APP_LOCALE_OPTIONS.some((option) => option.id === value);
}

export function normalizeAppLocale(value: string | null | undefined): AppLocale {
  const normalized = value?.trim();
  return normalized && isSupportedAppLocale(normalized) ? normalized : DEFAULT_APP_LOCALE;
}

export function makeDescriptor(
  id: string,
  defaultMessage: string,
  description?: string,
): MessageDescriptor {
  return { id, defaultMessage, description };
}
