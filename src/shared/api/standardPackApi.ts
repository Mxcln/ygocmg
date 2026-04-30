import { invokeApi } from "./invoke";
import type {
  GetStandardCardInput,
  ListStandardSetnamesInput,
  SearchStandardCardsInput,
  SearchStandardStringsInput,
  StandardCardDetail,
  StandardCardPage,
  StandardPackRebuildJob,
  StandardPackStatus,
  StandardSetnameEntry,
  StandardStringsPage,
} from "../contracts/standardPack";

export const standardPackApi = {
  getStatus() {
    return invokeApi<StandardPackStatus>("get_standard_pack_status");
  },

  rebuildIndex() {
    return invokeApi<StandardPackRebuildJob>("rebuild_standard_pack_index");
  },

  searchCards(input: SearchStandardCardsInput) {
    return invokeApi<StandardCardPage>("search_standard_cards", { input });
  },

  searchStrings(input: SearchStandardStringsInput) {
    return invokeApi<StandardStringsPage>("search_standard_strings", { input });
  },

  getCard(input: GetStandardCardInput) {
    return invokeApi<StandardCardDetail>("get_standard_card", { input });
  },

  listSetnames(input: ListStandardSetnamesInput) {
    return invokeApi<StandardSetnameEntry[]>("list_standard_setnames", { input });
  },
};
