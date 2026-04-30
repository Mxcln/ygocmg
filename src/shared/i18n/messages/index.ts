import type { AppLocale } from "../locales";
import { EN_MESSAGES } from "./en-US";
import { ZH_MESSAGES } from "./zh-CN";

export type AppMessageId = keyof typeof EN_MESSAGES;
export type AppMessages = Record<AppMessageId, string>;

export { EN_MESSAGES } from "./en-US";
export { ZH_MESSAGES } from "./zh-CN";

export const APP_MESSAGES: Record<AppLocale, AppMessages> = {
  "en-US": EN_MESSAGES,
  "zh-CN": ZH_MESSAGES,
};

export function appMessageDescriptor(id: AppMessageId) {
  return {
    id,
    defaultMessage: EN_MESSAGES[id],
  };
}
