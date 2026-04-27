import { invokeApi } from "./invoke";
import type {
  CardAssetStateDto,
  CreateEmptyScriptInput,
  DeleteFieldImageInput,
  DeleteMainImageInput,
  DeleteScriptInput,
  ImportFieldImageInput,
  ImportMainImageInput,
  ImportScriptInput,
  OpenScriptExternalInput,
} from "../contracts/resource";
import type { WriteResult } from "../contracts/card";

export const resourceApi = {
  importMainImage(input: ImportMainImageInput) {
    return invokeApi<WriteResult<CardAssetStateDto>>("import_main_image", { input });
  },

  deleteMainImage(input: DeleteMainImageInput) {
    return invokeApi<WriteResult<CardAssetStateDto>>("delete_main_image", { input });
  },

  importFieldImage(input: ImportFieldImageInput) {
    return invokeApi<WriteResult<CardAssetStateDto>>("import_field_image", { input });
  },

  deleteFieldImage(input: DeleteFieldImageInput) {
    return invokeApi<WriteResult<CardAssetStateDto>>("delete_field_image", { input });
  },

  createEmptyScript(input: CreateEmptyScriptInput) {
    return invokeApi<WriteResult<CardAssetStateDto>>("create_empty_script", { input });
  },

  importScript(input: ImportScriptInput) {
    return invokeApi<WriteResult<CardAssetStateDto>>("import_script", { input });
  },

  deleteScript(input: DeleteScriptInput) {
    return invokeApi<WriteResult<CardAssetStateDto>>("delete_script", { input });
  },

  openScriptExternal(input: OpenScriptExternalInput) {
    return invokeApi<void>("open_script_external", { input });
  },
};
