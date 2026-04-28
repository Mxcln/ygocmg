import { invokeApi } from "./invoke";
import type {
  ExecuteImportPackInput,
  ExecuteImportPackResult,
  ImportPreviewResult,
  PreviewImportPackInput,
} from "../contracts/import";

export const importApi = {
  previewImportPack(input: PreviewImportPackInput) {
    return invokeApi<ImportPreviewResult>("preview_import_pack", { input });
  },

  executeImportPack(input: ExecuteImportPackInput) {
    return invokeApi<ExecuteImportPackResult>("execute_import_pack", { input });
  },
};
