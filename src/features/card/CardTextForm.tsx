import { useEffect, useMemo, useState } from "react";
import type { CardEntity, CardTexts } from "../../shared/contracts/card";
import type { LanguageCode } from "../../shared/contracts/common";
import type { TextLanguageProfile } from "../../shared/contracts/config";
import {
  compactLanguageLabel,
  languageLabel,
  uniqueLanguageOrder,
  visibleTextLanguages,
} from "../../shared/utils/language";
import { useAppI18n } from "../../shared/i18n";

import shared from "../../shared/styles/shared.module.css";
import styles from "./CardTextForm.module.css";

interface CardTextFormProps {
  draft: CardEntity;
  catalog: TextLanguageProfile[];
  displayLanguageOrder: LanguageCode[];
  onChange?: (patch: Partial<CardEntity>) => void;
  onConfirmDeleteLanguage?: (language: LanguageCode, onConfirm: () => void) => void;
  readonly?: boolean;
}

const EMPTY_STRINGS = Array.from({ length: 16 }, () => "");

function ensureCardTexts(texts: CardTexts | undefined): CardTexts {
  return {
    name: texts?.name ?? "",
    desc: texts?.desc ?? "",
    strings: texts?.strings?.length === 16
      ? texts.strings
      : [...(texts?.strings ?? []), ...EMPTY_STRINGS].slice(0, 16),
  };
}

function hasTextValue(texts: CardTexts | undefined): boolean {
  if (!texts) return false;
  return Boolean(texts.name.trim() || texts.desc.trim() || texts.strings.some((value) => value.trim()));
}

export function CardTextForm({
  draft,
  catalog,
  displayLanguageOrder,
  onChange,
  onConfirmDeleteLanguage,
  readonly = false,
}: CardTextFormProps) {
  const { td, t } = useAppI18n();
  const expectedLanguages = useMemo(
    () => uniqueLanguageOrder(displayLanguageOrder),
    [displayLanguageOrder],
  );
  const actualLanguages = Object.keys(draft.texts);
  const allLangs = useMemo(
    () => uniqueLanguageOrder([...expectedLanguages, ...actualLanguages]),
    [expectedLanguages, actualLanguages],
  );
  const defaultLang = displayLanguageOrder.find((language) => allLangs.includes(language))
    ?? allLangs[0]
    ?? "";

  const [currentLang, setCurrentLang] = useState(defaultLang);
  const [stringsExpanded, setStringsExpanded] = useState(false);
  const [addingLanguage, setAddingLanguage] = useState(false);

  useEffect(() => {
    if (currentLang && allLangs.includes(currentLang)) return;
    setCurrentLang(defaultLang);
  }, [allLangs, currentLang, defaultLang]);

  const activeLang = allLangs.includes(currentLang) ? currentLang : defaultLang;
  const currentTexts = ensureCardTexts(draft.texts[activeLang]);
  const visibleCatalog = visibleTextLanguages(catalog);
  const addableLanguages = visibleCatalog.filter((language) => !actualLanguages.includes(language.id));

  function updateTexts(patch: Partial<CardTexts>) {
    if (readonly || !onChange || !activeLang) return;
    const updated: CardTexts = { ...currentTexts, ...patch };
    onChange({
      texts: { ...draft.texts, [activeLang]: updated },
    });
  }

  function updateString(index: number, value: string) {
    const nextStrings = [...currentTexts.strings];
    nextStrings[index] = value;
    updateTexts({ strings: nextStrings });
  }

  function addLanguage(language: string) {
    if (readonly || !onChange || !language || actualLanguages.includes(language)) return;
    onChange({
      texts: {
        ...draft.texts,
        [language]: ensureCardTexts(undefined),
      },
    });
    setCurrentLang(language);
    setAddingLanguage(false);
  }

  function deleteLanguage(language: string) {
    if (readonly || !onChange) return;
    const applyDelete = () => {
      const nextTexts = { ...draft.texts };
      delete nextTexts[language];
      onChange({ texts: nextTexts });
      const nextLanguage = allLangs.find((current) => current !== language) ?? "";
      setCurrentLang(nextLanguage);
    };
    if (hasTextValue(draft.texts[language]) && onConfirmDeleteLanguage) {
      onConfirmDeleteLanguage(language, applyDelete);
    } else {
      applyDelete();
    }
  }

  if (allLangs.length === 0 && readonly) {
    return (
      <div className={shared.cardListEmpty}>
        <p>{td("card.text.noLanguages", "No languages available.")}</p>
      </div>
    );
  }

  return (
    <div>
      {!activeLang ? (
        <div className={shared.cardListEmpty}>
          <p>{td("card.text.noLanguageSelected", "No language selected.")}</p>
        </div>
      ) : (
        <>
          <div className={styles.cardTextActiveRow}>
            <div className={styles.cardTextLangBar}>
              {allLangs.map((lang) => {
                const missing = !draft.texts[lang];
                return (
                  <button
                    key={lang}
                    type="button"
                    className={`${styles.cardTextLangBtn} ${activeLang === lang ? "active" : ""} ${missing ? "missing" : ""}`}
                    onClick={() => {
                      if (missing && !readonly) {
                        addLanguage(lang);
                      } else {
                        setCurrentLang(lang);
                      }
                    }}
                    title={missing ? td("card.text.createEmptyLanguage", "Create empty text for this language") : languageLabel(catalog, lang)}
                  >
                    {compactLanguageLabel(catalog, lang)}
                  </button>
                );
              })}
              {!readonly && (
                <>
                  {addingLanguage ? (
                    <select
                      className={styles.cardTextAddSelect}
                      autoFocus
                      value=""
                      onBlur={() => setAddingLanguage(false)}
                      onChange={(event) => addLanguage(event.target.value)}
                    >
                      <option value="">{addableLanguages.length === 0 ? td("language.noMoreLanguages", "No more languages") : t("language.addLanguage")}</option>
                      {addableLanguages.map((language) => (
                        <option key={language.id} value={language.id}>
                          {languageLabel(catalog, language.id)}
                        </option>
                      ))}
                    </select>
                  ) : (
                    <button
                      type="button"
                      className={styles.cardTextLangAdd}
                      disabled={addableLanguages.length === 0}
                      onClick={() => setAddingLanguage(true)}
                      title={t("language.addLanguage")}
                    >
                      +
                    </button>
                  )}
                </>
              )}
            </div>
            {!readonly && draft.texts[activeLang] && (
              <button type="button" className={styles.cardTextLangDelete} onClick={() => deleteLanguage(activeLang)}>
                {t("action.delete")}
              </button>
            )}
          </div>

          <div className={styles.cardTextField}>
            <label className={styles.cardTextLabel}>{td("card.text.name", "Name")}</label>
            <input
              className={styles.cardTextInput}
              type="text"
              value={currentTexts.name}
              onChange={(e) => updateTexts({ name: e.target.value })}
              readOnly={readonly}
            />
          </div>

          <div className={styles.cardTextField}>
            <label className={styles.cardTextLabel}>{td("card.text.effect", "Effect")}</label>
            <textarea
              className={styles.cardTextInput}
              rows={6}
              value={currentTexts.desc}
              onChange={(e) => updateTexts({ desc: e.target.value })}
              readOnly={readonly}
            />
          </div>

          <button
            type="button"
            className={styles.stringsToggle}
            onClick={() => setStringsExpanded(!stringsExpanded)}
          >
            <svg
              width="10"
              height="10"
              viewBox="0 0 10 10"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              style={{
                transform: stringsExpanded ? "rotate(90deg)" : "none",
              }}
            >
              <path d="M3 1l4 4-4 4" />
            </svg>
            {td("card.text.stringsCount", "Strings (16)")}
          </button>

          {stringsExpanded && (
            <div className={styles.stringsList}>
              {currentTexts.strings.map((s, i) => (
                <div key={i} className={styles.stringRow}>
                  <span className={styles.stringRowLabel}>{i}</span>
                  <input
                    className={styles.stringRowInput}
                    type="text"
                    value={s}
                    onChange={(e) => updateString(i, e.target.value)}
                    readOnly={readonly}
                  />
                </div>
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}
