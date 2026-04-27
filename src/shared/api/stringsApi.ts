import { invokeApi } from "./invoke";
import type {
  ConfirmPackStringsWriteInput,
  DeletePackStringsInput,
  DeletePackStringsResult,
  ListPackStringsInput,
  PackStringsPage,
  UpsertPackStringInput,
} from "../contracts/strings";
import type { WriteResult } from "../contracts/card";

export const stringsApi = {
  listPackStrings(input: ListPackStringsInput) {
    return invokeApi<PackStringsPage>("list_pack_strings", { input });
  },

  upsertPackString(input: UpsertPackStringInput) {
    return invokeApi<WriteResult<PackStringsPage>>("upsert_pack_string", { input });
  },

  deletePackStrings(input: DeletePackStringsInput) {
    return invokeApi<WriteResult<DeletePackStringsResult>>("delete_pack_strings", { input });
  },

  confirmPackStringsWrite(input: ConfirmPackStringsWriteInput) {
    return invokeApi<PackStringsPage>("confirm_pack_strings_write", { input });
  },
};
