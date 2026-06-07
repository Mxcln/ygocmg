import { invokeApi } from "./invoke";
import type {
  BulkDeleteCardsInput,
  BulkDeleteCardsResult,
  CardBatchWriteResult,
  CardDetail,
  CardListPage,
  ConfirmCardBatchWriteInput,
  ConfirmCardWriteInput,
  CreateCardInput,
  DeleteCardInput,
  DeleteCardResult,
  GetCardInput,
  ListCardsInput,
  MoveCardsInput,
  MoveCardsResult,
  SuggestCodeInput,
  SuggestCodeResult,
  UpdateCardInput,
  WriteResult,
} from "../contracts/card";

export const cardApi = {
  listCards(input: ListCardsInput) {
    return invokeApi<CardListPage>("list_cards", { input });
  },

  getCard(input: GetCardInput) {
    return invokeApi<CardDetail>("get_card", { input });
  },

  suggestCardCode(input: SuggestCodeInput) {
    return invokeApi<SuggestCodeResult>("suggest_card_code", { input });
  },

  createCard(input: CreateCardInput) {
    return invokeApi<WriteResult<CardDetail>>("create_card", { input });
  },

  updateCard(input: UpdateCardInput) {
    return invokeApi<WriteResult<CardDetail>>("update_card", { input });
  },

  deleteCard(input: DeleteCardInput) {
    return invokeApi<WriteResult<DeleteCardResult>>("delete_card", { input });
  },

  bulkDeleteCards(input: BulkDeleteCardsInput) {
    return invokeApi<WriteResult<BulkDeleteCardsResult>>("bulk_delete_cards", { input });
  },

  moveCards(input: MoveCardsInput) {
    return invokeApi<WriteResult<MoveCardsResult>>("move_cards", { input });
  },

  confirmCardWrite(input: ConfirmCardWriteInput) {
    return invokeApi<CardDetail>("confirm_card_write", { input });
  },

  confirmCardBatchWrite(input: ConfirmCardBatchWriteInput) {
    return invokeApi<CardBatchWriteResult>("confirm_card_batch_write", { input });
  },
};
