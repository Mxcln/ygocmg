import { invokeApi } from "./invoke";
import type {
  CardDetail,
  CardListPage,
  ConfirmCardWriteInput,
  CreateCardInput,
  DeleteCardInput,
  DeleteCardResult,
  GetCardInput,
  ListCardsInput,
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

  confirmCardWrite(input: ConfirmCardWriteInput) {
    return invokeApi<CardDetail>("confirm_card_write", { input });
  },
};
