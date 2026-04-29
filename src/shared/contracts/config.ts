import type { LanguageCode } from "./common";

export type TextLanguageKind = "builtin" | "custom";

export interface TextLanguageProfile {
  id: LanguageCode;
  label: string;
  kind: TextLanguageKind;
  hidden: boolean;
  last_used_at: string | null;
}

export interface GlobalConfig {
  app_language: LanguageCode;
  ygopro_path: string | null;
  external_text_editor_path: string | null;
  custom_code_recommended_min: number;
  custom_code_recommended_max: number;
  custom_code_min_gap: number;
  shell_sidebar_width: number;
  shell_window_width: number;
  shell_window_height: number;
  shell_window_is_maximized: boolean;
  text_language_catalog: TextLanguageProfile[];
  standard_pack_source_language: LanguageCode | null;
}
