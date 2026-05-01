import type { LanguageCode, PackId, PreviewResult, PreviewToken, ValidationIssue, WorkspaceId } from "./common";
import type { JobAccepted } from "./job";

export interface PreviewImportPackInput {
  workspaceId: WorkspaceId;
  newPackName: string;
  newPackCode: string | null;
  newPackAuthor: string;
  newPackVersion: string;
  newPackDescription: string | null;
  displayLanguageOrder: LanguageCode[];
  defaultExportLanguage: LanguageCode | null;
  cdbPath: string;
  picsDir: string | null;
  fieldPicsDir: string | null;
  scriptDir: string | null;
  stringsConfPath: string | null;
  sourceLanguage: LanguageCode;
}

export interface ImportPreview {
  target_pack_id: PackId;
  target_pack_name: string;
  card_count: number;
  warning_count: number;
  error_count: number;
  missing_main_image_count: number;
  missing_script_count: number;
  missing_field_image_count: number;
  issues: ValidationIssue[];
}

export interface ExecuteImportPackInput {
  previewToken: PreviewToken;
}

export type ImportPreviewResult = PreviewResult<ImportPreview>;
export type ExecuteImportPackResult = JobAccepted;
