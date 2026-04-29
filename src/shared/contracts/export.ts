import type { LanguageCode, PackId, PreviewResult, PreviewToken, ValidationIssue, WorkspaceId } from "./common";
import type { JobAccepted } from "./job";

export interface PreviewExportBundleInput {
  workspaceId: WorkspaceId;
  packIds: PackId[];
  exportLanguage: LanguageCode;
  outputDir: string;
  outputName: string;
}

export interface ExportPreview {
  pack_count: number;
  card_count: number;
  main_image_count: number;
  field_image_count: number;
  script_count: number;
  warning_count: number;
  error_count: number;
  issues: ValidationIssue[];
}

export interface ExecuteExportBundleInput {
  previewToken: PreviewToken;
}

export type ExportPreviewResult = PreviewResult<ExportPreview>;
export type ExecuteExportBundleResult = JobAccepted;
