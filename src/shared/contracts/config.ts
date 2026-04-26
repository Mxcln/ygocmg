import type { LanguageCode } from "./common";

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
}
