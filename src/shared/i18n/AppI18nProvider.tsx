import { createContext, useContext, useMemo } from "react";
import type { ReactNode } from "react";
import { IntlProvider, createIntl, createIntlCache } from "react-intl";
import type { IntlShape, MessageDescriptor, PrimitiveType } from "react-intl";
import { APP_MESSAGES } from "./messages";
import type { AppMessageId } from "./messages";
import { DEFAULT_APP_LOCALE, normalizeAppLocale } from "./locales";
import type { AppLocale } from "./locales";

type MessageValues = Record<string, PrimitiveType>;

interface AppI18nContextValue {
  locale: AppLocale;
  intl: IntlShape;
  t: (id: AppMessageId, values?: MessageValues) => string;
  formatDescriptor: (descriptor: MessageDescriptor, values?: MessageValues) => string;
}

const intlCache = createIntlCache();
const fallbackIntl = createIntl(
  {
    locale: DEFAULT_APP_LOCALE,
    messages: APP_MESSAGES[DEFAULT_APP_LOCALE],
  },
  intlCache,
);

let activeLocale: AppLocale = DEFAULT_APP_LOCALE;
let activeIntl: IntlShape = fallbackIntl;

function formatWithIntl(
  intl: IntlShape,
  locale: AppLocale,
  descriptor: MessageDescriptor,
  values?: MessageValues,
): string {
  if (locale !== DEFAULT_APP_LOCALE && !Object.hasOwn(APP_MESSAGES[locale], descriptor.id ?? "")) {
    const fallback = fallbackIntl.formatMessage(descriptor, values);
    return `[未翻訳] ${fallback}`;
  }
  return intl.formatMessage(descriptor, values);
}

export function formatAppMessage(
  descriptor: MessageDescriptor,
  values?: MessageValues,
): string {
  return formatWithIntl(activeIntl, activeLocale, descriptor, values);
}

export function formatAppMessageById(
  id: AppMessageId,
  values?: MessageValues,
): string {
  return formatAppMessage({ id, defaultMessage: APP_MESSAGES[DEFAULT_APP_LOCALE][id] }, values);
}

export function getActiveAppLocale(): AppLocale {
  return activeLocale;
}

const AppI18nContext = createContext<AppI18nContextValue>({
  locale: DEFAULT_APP_LOCALE,
  intl: fallbackIntl,
  t: (id, values) => formatWithIntl(fallbackIntl, DEFAULT_APP_LOCALE, { id, defaultMessage: APP_MESSAGES[DEFAULT_APP_LOCALE][id] }, values),
  formatDescriptor: (descriptor, values) => formatWithIntl(fallbackIntl, DEFAULT_APP_LOCALE, descriptor, values),
});

export function AppI18nProvider({
  locale,
  children,
}: {
  locale: string | null | undefined;
  children: ReactNode;
}) {
  const appLocale = normalizeAppLocale(locale);
  const messages = APP_MESSAGES[appLocale];
  const intl = useMemo(
    () =>
      createIntl(
        {
          locale: appLocale,
          messages,
        },
        intlCache,
      ),
    [appLocale, messages],
  );

  const value = useMemo<AppI18nContextValue>(
    () => ({
      locale: appLocale,
      intl,
      t: (id, values) => formatWithIntl(intl, appLocale, { id, defaultMessage: APP_MESSAGES[DEFAULT_APP_LOCALE][id] }, values),
      formatDescriptor: (descriptor, values) => formatWithIntl(intl, appLocale, descriptor, values),
    }),
    [appLocale, intl],
  );

  activeLocale = appLocale;
  activeIntl = intl;

  return (
    <IntlProvider locale={appLocale} messages={messages} defaultLocale={DEFAULT_APP_LOCALE}>
      <AppI18nContext.Provider value={value}>{children}</AppI18nContext.Provider>
    </IntlProvider>
  );
}

export function useAppI18n(): AppI18nContextValue {
  return useContext(AppI18nContext);
}
