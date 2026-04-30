import type { AppMessages } from "./index";
import { EN_MESSAGES } from "./en-US";

export const ZH_MESSAGES: AppMessages = Object.fromEntries(
  Object.entries(EN_MESSAGES).map(([id, value]) => [id, `[待译] ${value}`]),
) as AppMessages;
