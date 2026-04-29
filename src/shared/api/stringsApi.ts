import { invokeApi } from "./invoke";
import type {
  ConfirmPackStringsWriteInput,
  ConfirmPackStringRecordWriteInput,
  DeletePackStringsInput,
  DeletePackStringsResult,
  GetPackStringInput,
  ListPackStringsInput,
  PackStringRecordDetail,
  RemovePackStringTranslationInput,
  PackStringsPage,
  UpsertPackStringInput,
  UpsertPackStringRecordInput,
} from "../contracts/strings";
import type { WriteResult } from "../contracts/card";

export const stringsApi = {
  listPackStrings(input: ListPackStringsInput) {
    return invokeApi<PackStringsPage>("list_pack_strings", { input });
  },

  getPackString(input: GetPackStringInput) {
    return invokeApi<PackStringRecordDetail>("get_pack_string", { input });
  },

  upsertPackString(input: UpsertPackStringInput) {
    return invokeApi<WriteResult<PackStringsPage>>("upsert_pack_string", { input });
  },

  upsertPackStringRecord(input: UpsertPackStringRecordInput) {
    return invokeApi<WriteResult<PackStringRecordDetail>>("upsert_pack_string_record", { input });
  },

  removePackStringTranslation(input: RemovePackStringTranslationInput) {
    return invokeApi<WriteResult<DeletePackStringsResult>>("remove_pack_string_translation", { input });
  },

  deletePackStrings(input: DeletePackStringsInput) {
    return invokeApi<WriteResult<DeletePackStringsResult>>("delete_pack_strings", { input });
  },

  confirmPackStringsWrite(input: ConfirmPackStringsWriteInput) {
    return invokeApi<PackStringsPage>("confirm_pack_strings_write", { input });
  },

  confirmPackStringRecordWrite(input: ConfirmPackStringRecordWriteInput) {
    return invokeApi<PackStringRecordDetail>("confirm_pack_string_record_write", { input });
  },
};
