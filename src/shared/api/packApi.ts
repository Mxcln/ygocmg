import { invokeApi } from "./invoke";
import type { PackMetadata, PackOverview } from "../contracts/pack";

export interface CreatePackInput {
  name: string;
  author: string;
  version: string;
  description: string | null;
  displayLanguageOrder: string[];
  defaultExportLanguage: string | null;
}

export interface OpenPackInput {
  packId: string;
}

export interface ClosePackInput {
  packId: string;
}

export interface DeletePackInput {
  packId: string;
}

export const packApi = {
  listPackOverviews() {
    return invokeApi<PackOverview[]>("list_pack_overviews");
  },

  createPack(input: CreatePackInput) {
    return invokeApi<PackMetadata>("create_pack", { input });
  },

  openPack(input: OpenPackInput) {
    return invokeApi<PackMetadata>("open_pack", { input });
  },

  closePack(input: ClosePackInput) {
    return invokeApi<void>("close_pack", { input });
  },

  deletePack(input: DeletePackInput) {
    return invokeApi<void>("delete_pack", { input });
  },
};
