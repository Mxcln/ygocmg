import { invokeApi } from "./invoke";
import type {
  ExecuteExportBundleInput,
  ExecuteExportBundleResult,
  ExportPreviewResult,
  PreviewExportBundleInput,
} from "../contracts/export";

export const exportApi = {
  previewExportBundle(input: PreviewExportBundleInput) {
    return invokeApi<ExportPreviewResult>("preview_export_bundle", { input });
  },

  executeExportBundle(input: ExecuteExportBundleInput) {
    return invokeApi<ExecuteExportBundleResult>("execute_export_bundle", { input });
  },
};
