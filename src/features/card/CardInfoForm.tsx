import { useEffect, useState } from "react";

import type {
  CardEntity,
  PrimaryType,
  Ot,
  MonsterFlag,
  Race,
  Attribute,
  SpellSubtype,
  TrapSubtype,
  LinkMarker,
} from "../../shared/contracts/card";
import {
  CARD_CATEGORY_MAX_MASK,
  CARD_CATEGORY_OPTIONS,
  formatCardCategoryMask,
  hasCardCategoryMask,
  normalizeCardCategoryMask,
} from "../../shared/constants/cardCategories";

import styles from "./CardInfoForm.module.css";

interface CardInfoFormProps {
  draft: CardEntity;
  onChange?: (patch: Partial<CardEntity>) => void;
  readonly?: boolean;
}

const ALL_OT: Ot[] = ["ocg", "tcg", "custom"];

const ALL_MONSTER_FLAGS: MonsterFlag[] = [
  "normal", "effect", "fusion", "ritual", "synchro", "xyz",
  "pendulum", "link", "tuner", "token", "gemini", "spirit",
  "union", "flip", "toon",
];

const ALL_RACES: Race[] = [
  "warrior", "spellcaster", "fairy", "fiend", "zombie", "machine",
  "aqua", "pyro", "rock", "winged_beast", "plant", "insect",
  "thunder", "dragon", "beast", "beast_warrior", "dinosaur",
  "fish", "sea_serpent", "reptile", "psychic", "divine_beast",
  "creator_god", "wyrm", "cyberse", "illusion",
];

const ALL_ATTRIBUTES: Attribute[] = [
  "light", "dark", "earth", "water", "fire", "wind", "divine",
];

const ALL_SPELL_SUBTYPES: SpellSubtype[] = [
  "normal", "continuous", "quick_play", "ritual", "field", "equip",
];

const ALL_TRAP_SUBTYPES: TrapSubtype[] = [
  "normal", "continuous", "counter",
];

const LINK_MARKER_POSITIONS: (LinkMarker | null)[][] = [
  ["top_left", "top", "top_right"],
  ["left", null, "right"],
  ["bottom_left", "bottom", "bottom_right"],
];

const LINK_MARKER_ARROWS: Record<LinkMarker, string> = {
  top_left: "\u2196", top: "\u2191", top_right: "\u2197",
  left: "\u2190", right: "\u2192",
  bottom_left: "\u2199", bottom: "\u2193", bottom_right: "\u2198",
};

function displayLabel(value: string): string {
  return value.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

export function CardInfoForm({ draft, onChange, readonly = false }: CardInfoFormProps) {
  const [categoryRawInput, setCategoryRawInput] = useState("");
  const [categoryPickerOpen, setCategoryPickerOpen] = useState(false);
  const isMonster = draft.primary_type === "monster";
  const isSpell = draft.primary_type === "spell";
  const isTrap = draft.primary_type === "trap";
  const flags = draft.monster_flags ?? [];
  const isLink = isMonster && flags.includes("link");
  const isPendulum = isMonster && flags.includes("pendulum");
  const normalizedCategory = normalizeCardCategoryMask(draft.category);
  const categoryRawDisplay = formatCardCategoryMask(normalizedCategory);
  const selectedCategoryOptions = CARD_CATEGORY_OPTIONS.filter((option) =>
    hasCardCategoryMask(normalizedCategory, option.mask),
  );

  useEffect(() => {
    setCategoryRawInput(categoryRawDisplay);
  }, [categoryRawDisplay]);

  function emitChange(patch: Partial<CardEntity>) {
    if (readonly || !onChange) return;
    onChange(patch);
  }

  function handlePrimaryTypeChange(pt: PrimaryType) {
    if (pt === draft.primary_type) return;
    const patch: Partial<CardEntity> = { primary_type: pt };
    if (pt !== "monster") {
      patch.monster_flags = null;
      patch.atk = null;
      patch.def = null;
      patch.race = null;
      patch.attribute = null;
      patch.level = null;
      patch.pendulum = null;
      patch.link = null;
    }
    if (pt !== "spell") patch.spell_subtype = null;
    if (pt !== "trap") patch.trap_subtype = null;
    emitChange(patch);
  }

  function toggleMonsterFlag(flag: MonsterFlag) {
    const current = draft.monster_flags ?? [];
    const next = current.includes(flag)
      ? current.filter((f) => f !== flag)
      : [...current, flag];

    const patch: Partial<CardEntity> = { monster_flags: next };

    if (flag === "link" && !current.includes("link")) {
      patch.def = null;
      patch.level = null;
      patch.link = { markers: [] };
    }
    if (flag === "link" && current.includes("link")) {
      patch.link = null;
    }
    if (flag === "pendulum" && !current.includes("pendulum")) {
      patch.pendulum = { left_scale: 0, right_scale: 0 };
    }
    if (flag === "pendulum" && current.includes("pendulum")) {
      patch.pendulum = null;
    }
    emitChange(patch);
  }

  function toggleLinkMarker(marker: LinkMarker) {
    const current = draft.link?.markers ?? [];
    const next = current.includes(marker)
      ? current.filter((m) => m !== marker)
      : [...current, marker];
    emitChange({ link: { markers: next } });
  }

  function toggleCategoryMask(mask: number) {
    const normalized = normalizeCardCategoryMask(draft.category);
    const selected = hasCardCategoryMask(normalized, mask);
    const next = selected ? normalized - mask : normalized + mask;
    emitChange({ category: normalizeCardCategoryMask(next) });
  }

  function handleCategoryRawInput(value: string) {
    setCategoryRawInput(value);
    const raw = value.trim().replace(/^0x/i, "");
    if (raw === "") {
      emitChange({ category: 0 });
      return;
    }
    if (!/^[0-9a-fA-F]+$/.test(raw)) return;
    const parsed = Number.parseInt(raw, 16);
    if (!Number.isFinite(parsed)) return;
    const next = Math.min(parsed, CARD_CATEGORY_MAX_MASK);
    if (next !== parsed) {
      setCategoryRawInput(formatCardCategoryMask(next));
    }
    emitChange({ category: next });
  }

  function handleNumberInput(
    field: "code" | "alias" | "setcode" | "atk" | "def" | "level",
    value: string,
  ) {
    if (value === "" || value === "-") {
      emitChange({ [field]: 0 });
      return;
    }
    if (value === "?") {
      if (field === "atk" || field === "def") {
        emitChange({ [field]: -2 });
      }
      return;
    }
    const parsed = Number.parseInt(value, 10);
    if (Number.isFinite(parsed)) {
      emitChange({ [field]: parsed });
    }
  }

  function formatStatValue(val: number | null): string {
    if (val === null) return "";
    if (val === -2) return "?";
    return String(val);
  }

  return (
    <div className={styles.cardInfoGrid}>
      {/* Basic fields */}
      <div className={styles.cardInfoField}>
        <label className={styles.cardInfoLabel}>Code</label>
        <input
          className={styles.cardInfoInput}
          type="text"
          inputMode="numeric"
          value={draft.code}
          onChange={(e) => handleNumberInput("code", e.target.value)}
          readOnly={readonly}
        />
      </div>
      <div className={styles.cardInfoField}>
        <label className={styles.cardInfoLabel}>Alias</label>
        <input
          className={styles.cardInfoInput}
          type="text"
          inputMode="numeric"
          value={draft.alias}
          onChange={(e) => handleNumberInput("alias", e.target.value)}
          readOnly={readonly}
        />
      </div>

      <div className={styles.cardInfoField}>
        <label className={styles.cardInfoLabel}>Setcode</label>
        <input
          className={styles.cardInfoInput}
          type="text"
          value={`0x${draft.setcode.toString(16).toUpperCase()}`}
          onChange={(e) => {
            const raw = e.target.value.replace(/^0x/i, "");
            const parsed = Number.parseInt(raw, 16);
            if (Number.isFinite(parsed)) emitChange({ setcode: parsed });
            else if (raw === "") emitChange({ setcode: 0 });
          }}
          readOnly={readonly}
        />
      </div>
      <div className={styles.cardInfoField}>
        <label className={styles.cardInfoLabel}>OT</label>
        <select
          className={styles.cardInfoSelect}
          value={draft.ot}
          onChange={(e) => emitChange({ ot: e.target.value as Ot })}
          disabled={readonly}
        >
          {ALL_OT.map((o) => (
            <option key={o} value={o}>{displayLabel(o)}</option>
          ))}
        </select>
      </div>

      <div className={`${styles.cardInfoField} ${styles.cardInfoFieldFull}`}>
        <div className={styles.categoryHeader}>
          <label className={styles.cardInfoLabel}>Effect Categories</label>
          <span className={styles.categoryRawValue}>Raw: {categoryRawDisplay}</span>
        </div>
        <div className={styles.categoryTagRow}>
          <div className={styles.categoryTags}>
            {selectedCategoryOptions.length === 0 ? (
              <span className={styles.categoryEmptyTag}>No categories</span>
            ) : (
              selectedCategoryOptions.map((option) => (
                <button
                  key={option.bitIndex}
                  type="button"
                  className={styles.categoryTag}
                  onClick={() => toggleCategoryMask(option.mask)}
                  disabled={readonly}
                  title={`Remove ${option.label}`}
                >
                  {option.label}
                </button>
              ))
            )}
          </div>
          <button
            type="button"
            className={`${styles.categoryAddButton} ${categoryPickerOpen ? "active" : ""}`}
            onClick={() => setCategoryPickerOpen((open) => !open)}
            disabled={readonly}
            aria-expanded={categoryPickerOpen}
          >
            +
          </button>
        </div>
        {categoryPickerOpen && (
          <div className={styles.categoryPickerPanel}>
            <div className={styles.categoryCheckboxGrid}>
              {CARD_CATEGORY_OPTIONS.map((option) => {
                const selected = hasCardCategoryMask(normalizedCategory, option.mask);
                return (
                  <label key={option.bitIndex} className={styles.categoryCheckboxItem}>
                    <input
                      type="checkbox"
                      checked={selected}
                      onChange={() => toggleCategoryMask(option.mask)}
                      disabled={readonly}
                    />
                    <span>{option.label}</span>
                  </label>
                );
              })}
            </div>
            <details className={styles.categoryAdvanced}>
              <summary>Advanced raw mask</summary>
              <input
                className={styles.cardInfoInput}
                type="text"
                value={categoryRawInput}
                onChange={(e) => handleCategoryRawInput(e.target.value)}
                onBlur={() => setCategoryRawInput(categoryRawDisplay)}
                readOnly={readonly}
              />
            </details>
          </div>
        )}
      </div>
      <div className={styles.cardInfoField}>
        <label className={styles.cardInfoLabel}>Primary Type</label>
        <div className={styles.cardTypeRadioGroup}>
          {(["monster", "spell", "trap"] as PrimaryType[]).map((pt) => (
            <button
              key={pt}
              type="button"
              className={`${styles.cardTypeRadio} ${draft.primary_type === pt ? "active" : ""}`}
              onClick={() => handlePrimaryTypeChange(pt)}
              disabled={readonly}
            >
              {displayLabel(pt)}
            </button>
          ))}
        </div>
      </div>

      {/* Monster-specific fields */}
      {isMonster && (
        <div className={styles.cardInfoSection}>
          <h4 className={styles.cardInfoSectionTitle}>Monster</h4>
          <div className={styles.cardInfoGrid}>
            <div className={`${styles.cardInfoField} ${styles.cardInfoFieldFull}`}>
              <label className={styles.cardInfoLabel}>Monster Flags</label>
              <div className={styles.monsterFlagsGroup}>
                {ALL_MONSTER_FLAGS.map((flag) => (
                  <button
                    key={flag}
                    type="button"
                    className={`${styles.monsterFlagChip} ${flags.includes(flag) ? "selected" : ""}`}
                    onClick={() => toggleMonsterFlag(flag)}
                    disabled={readonly}
                  >
                    {displayLabel(flag)}
                  </button>
                ))}
              </div>
            </div>

            <div className={styles.cardInfoField}>
              <label className={styles.cardInfoLabel}>Attribute</label>
              <select
                className={styles.cardInfoSelect}
                value={draft.attribute ?? ""}
                onChange={(e) =>
                  emitChange({ attribute: (e.target.value || null) as Attribute | null })
                }
                disabled={readonly}
              >
                <option value="">—</option>
                {ALL_ATTRIBUTES.map((a) => (
                  <option key={a} value={a}>{displayLabel(a)}</option>
                ))}
              </select>
            </div>
            <div className={styles.cardInfoField}>
              <label className={styles.cardInfoLabel}>Race</label>
              <select
                className={styles.cardInfoSelect}
                value={draft.race ?? ""}
                onChange={(e) =>
                  emitChange({ race: (e.target.value || null) as Race | null })
                }
                disabled={readonly}
              >
                <option value="">—</option>
                {ALL_RACES.map((r) => (
                  <option key={r} value={r}>{displayLabel(r)}</option>
                ))}
              </select>
            </div>

            <div className={styles.cardInfoField}>
              <label className={styles.cardInfoLabel}>ATK</label>
              <input
                className={styles.cardInfoInput}
                type="text"
                value={formatStatValue(draft.atk)}
                onChange={(e) => handleNumberInput("atk", e.target.value)}
                placeholder="e.g. 2500 or ?"
                readOnly={readonly}
              />
            </div>
            {!isLink && (
              <div className={styles.cardInfoField}>
                <label className={styles.cardInfoLabel}>DEF</label>
                <input
                  className={styles.cardInfoInput}
                  type="text"
                  value={formatStatValue(draft.def)}
                  onChange={(e) => handleNumberInput("def", e.target.value)}
                  placeholder="e.g. 2000 or ?"
                  readOnly={readonly}
                />
              </div>
            )}
            {!isLink && (
              <div className={styles.cardInfoField}>
                <label className={styles.cardInfoLabel}>
                  {flags.includes("xyz") ? "Rank" : "Level"}
                </label>
                <input
                  className={styles.cardInfoInput}
                  type="text"
                  inputMode="numeric"
                  value={draft.level ?? ""}
                  onChange={(e) => handleNumberInput("level", e.target.value)}
                  readOnly={readonly}
                />
              </div>
            )}

            {isPendulum && (
              <>
                <div className={styles.cardInfoField}>
                  <label className={styles.cardInfoLabel}>Left Scale</label>
                  <input
                    className={styles.cardInfoInput}
                    type="text"
                    inputMode="numeric"
                    value={draft.pendulum?.left_scale ?? 0}
                    onChange={(e) => {
                      const v = Number.parseInt(e.target.value, 10);
                      emitChange({
                        pendulum: {
                          left_scale: Number.isFinite(v) ? v : 0,
                          right_scale: draft.pendulum?.right_scale ?? 0,
                        },
                      });
                    }}
                    readOnly={readonly}
                  />
                </div>
                <div className={styles.cardInfoField}>
                  <label className={styles.cardInfoLabel}>Right Scale</label>
                  <input
                    className={styles.cardInfoInput}
                    type="text"
                    inputMode="numeric"
                    value={draft.pendulum?.right_scale ?? 0}
                    onChange={(e) => {
                      const v = Number.parseInt(e.target.value, 10);
                      emitChange({
                        pendulum: {
                          left_scale: draft.pendulum?.left_scale ?? 0,
                          right_scale: Number.isFinite(v) ? v : 0,
                        },
                      });
                    }}
                    readOnly={readonly}
                  />
                </div>
              </>
            )}

            {isLink && (
              <div className={`${styles.cardInfoField} ${styles.cardInfoFieldFull}`}>
                <label className={styles.cardInfoLabel}>Link Markers</label>
                <div className={styles.linkMarkerGrid}>
                  {LINK_MARKER_POSITIONS.flat().map((marker, i) => {
                    if (marker === null) {
                      return <div key={i} className={`${styles.linkMarkerCell} center`} />;
                    }
                    const selected = draft.link?.markers.includes(marker) ?? false;
                    return (
                      <button
                        key={marker}
                        type="button"
                        className={`${styles.linkMarkerCell} ${selected ? "selected" : ""}`}
                        onClick={() => toggleLinkMarker(marker)}
                        title={displayLabel(marker)}
                        disabled={readonly}
                      >
                        {LINK_MARKER_ARROWS[marker]}
                      </button>
                    );
                  })}
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Spell-specific fields */}
      {isSpell && (
        <div className={styles.cardInfoSection}>
          <h4 className={styles.cardInfoSectionTitle}>Spell</h4>
          <div className={styles.cardInfoGrid}>
            <div className={styles.cardInfoField}>
              <label className={styles.cardInfoLabel}>Spell Subtype</label>
              <select
                className={styles.cardInfoSelect}
                value={draft.spell_subtype ?? ""}
                onChange={(e) =>
                  emitChange({ spell_subtype: (e.target.value || null) as SpellSubtype | null })
                }
                disabled={readonly}
              >
                <option value="">—</option>
                {ALL_SPELL_SUBTYPES.map((s) => (
                  <option key={s} value={s}>{displayLabel(s)}</option>
                ))}
              </select>
            </div>
          </div>
        </div>
      )}

      {/* Trap-specific fields */}
      {isTrap && (
        <div className={styles.cardInfoSection}>
          <h4 className={styles.cardInfoSectionTitle}>Trap</h4>
          <div className={styles.cardInfoGrid}>
            <div className={styles.cardInfoField}>
              <label className={styles.cardInfoLabel}>Trap Subtype</label>
              <select
                className={styles.cardInfoSelect}
                value={draft.trap_subtype ?? ""}
                onChange={(e) =>
                  emitChange({ trap_subtype: (e.target.value || null) as TrapSubtype | null })
                }
                disabled={readonly}
              >
                <option value="">—</option>
                {ALL_TRAP_SUBTYPES.map((t) => (
                  <option key={t} value={t}>{displayLabel(t)}</option>
                ))}
              </select>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
