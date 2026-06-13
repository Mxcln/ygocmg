import type { LanguageCode } from "./common";

export type TextLanguageKind = "builtin" | "custom";

export type ThemeMode = "system" | "light" | "dark";

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
  /** Recommended custom setname base range (12-bit base, hex 0x000–0xFFF). Drives agent key suggestion and the out-of-range warning. */
  setname_base_recommended_min: number;
  setname_base_recommended_max: number;
  shell_sidebar_width: number;
  shell_sidebar_collapsed: boolean;
  shell_right_sidebar_width: number;
  shell_right_sidebar_collapsed: boolean;
  shell_window_width: number;
  shell_window_height: number;
  shell_window_is_maximized: boolean;
  text_language_catalog: TextLanguageProfile[];
  standard_pack_source_language: LanguageCode | null;
  theme_mode: ThemeMode;
  high_contrast: boolean;
  custom_brand_color: string | null;
  deepseek_api_key: string | null;
  /** Agent reply language: "auto" follows the app UI language, otherwise an explicit locale. */
  agent_language: AgentLanguage;
}

export type AgentLanguage = "auto" | LanguageCode;
