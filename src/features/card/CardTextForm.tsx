import { useState } from "react";
import type { CardEntity, CardTexts } from "../../shared/contracts/card";
import type { LanguageCode } from "../../shared/contracts/common";

interface CardTextFormProps {
  draft: CardEntity;
  availableLanguages: LanguageCode[];
  displayLanguageOrder: LanguageCode[];
  onChange?: (patch: Partial<CardEntity>) => void;
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

export function CardTextForm({
  draft,
  availableLanguages,
  displayLanguageOrder,
  onChange,
  readonly = false,
}: CardTextFormProps) {
  const allLangs = availableLanguages.length > 0
    ? availableLanguages
    : Object.keys(draft.texts);

  const defaultLang = displayLanguageOrder.find((l) => allLangs.includes(l))
    ?? allLangs[0]
    ?? "";

  const [currentLang, setCurrentLang] = useState(defaultLang);
  const [stringsExpanded, setStringsExpanded] = useState(false);

  const activeLang = allLangs.includes(currentLang) ? currentLang : defaultLang;
  const currentTexts = ensureCardTexts(draft.texts[activeLang]);

  function updateTexts(patch: Partial<CardTexts>) {
    if (readonly || !onChange) return;
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

  if (allLangs.length === 0) {
    return (
      <div className="card-list-empty">
        <p>No languages available.</p>
      </div>
    );
  }

  return (
    <div>
      <div className="card-text-lang-bar">
        {allLangs.map((lang) => (
          <button
            key={lang}
            type="button"
            className={`card-text-lang-btn ${activeLang === lang ? "active" : ""}`}
            onClick={() => setCurrentLang(lang)}
          >
            {lang}
          </button>
        ))}
      </div>

      <div className="card-text-field">
        <label className="card-text-label">Name</label>
        <input
          className="card-text-input"
          type="text"
          value={currentTexts.name}
          onChange={(e) => updateTexts({ name: e.target.value })}
          readOnly={readonly}
        />
      </div>

      <div className="card-text-field">
        <label className="card-text-label">Effect</label>
        <textarea
          className="card-text-input"
          rows={6}
          value={currentTexts.desc}
          onChange={(e) => updateTexts({ desc: e.target.value })}
          readOnly={readonly}
        />
      </div>

      <button
        type="button"
        className="strings-toggle"
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
        Strings (16)
      </button>

      {stringsExpanded && (
        <div className="strings-list">
          {currentTexts.strings.map((s, i) => (
            <div key={i} className="string-row">
              <span className="string-row-label">{i}</span>
              <input
                className="string-row-input"
                type="text"
                value={s}
                onChange={(e) => updateString(i, e.target.value)}
                readOnly={readonly}
              />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
