import { useEffect, useMemo, useState } from "react";

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
  formatCardCategory,
  formatAttribute,
  formatLinkMarker,
  formatMonsterFlag,
  formatOt,
  formatPrimaryType,
  formatRace,
  formatSpellSubtype,
  formatTrapSubtype,
} from "../../shared/utils/cardLabels";
import { useAppI18n } from "../../shared/i18n";
import {
  CARD_CATEGORY_MAX_MASK,
  CARD_CATEGORY_OPTIONS,
  formatCardCategoryMask,
  hasCardCategoryMask,
  normalizeCardCategoryMask,
} from "../../shared/constants/cardCategories";
import {
  ALL_ATTRIBUTES,
  ALL_MONSTER_FLAGS,
  ALL_OT,
  ALL_PRIMARY_TYPES,
  ALL_RACES,
  ALL_SPELL_SUBTYPES,
  ALL_TRAP_SUBTYPES,
  LINK_MARKER_ARROWS,
  LINK_MARKER_POSITIONS,
} from "../../shared/constants/cardOptions";

import styles from "./CardInfoForm.module.css";

export interface SetnameEntry {
  key: number;
  name: string;
  source: "pack" | "standard";
}

interface CardInfoFormProps {
  draft: CardEntity;
  onChange?: (patch: Partial<CardEntity>) => void;
  readonly?: boolean;
  setnameEntries?: SetnameEntry[];
}

function formatSetcodeHex(value: number): string {
  return `0x${value.toString(16).toUpperCase()}`;
}

function formatSetcodePackedHex(slots: number[]): string {
  if (slots.length === 0) return "0x0";
  let packed = BigInt(0);
  for (let i = slots.length - 1; i >= 0; i--) {
    packed = (packed << BigInt(16)) | BigInt(slots[i] & 0xffff);
  }
  return `0x${packed.toString(16).toUpperCase()}`;
}

export function CardInfoForm({ draft, onChange, readonly = false, setnameEntries = [] }: CardInfoFormProps) {
  const { t } = useAppI18n();
  const [categoryRawInput, setCategoryRawInput] = useState("");
  const [categoryPickerOpen, setCategoryPickerOpen] = useState(false);
  const [setcodePickerOpen, setSetcodePickerOpen] = useState(false);
  const [setcodeSearch, setSetcodeSearch] = useState("");

  const setnameMap = useMemo(() => {
    const map = new Map<number, SetnameEntry>();
    for (const entry of setnameEntries) {
      if (!map.has(entry.key) || entry.source === "pack") {
        map.set(entry.key, entry);
      }
    }
    return map;
  }, [setnameEntries]);
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
    field: "code" | "alias" | "atk" | "def" | "level",
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
      {/* Code / Alias / OT - compact triple row */}
      <div className={styles.tripleRow}>
        <div className={styles.cardInfoField}>
          <label className={styles.cardInfoLabel}>{t("card.info.code")}</label>
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
          <label className={styles.cardInfoLabel}>{t("card.info.alias")}</label>
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
          <label className={styles.cardInfoLabel}>{t("card.info.ot")}</label>
          <select
            className={styles.cardInfoSelect}
            value={draft.ot}
            onChange={(e) => emitChange({ ot: e.target.value as Ot })}
            disabled={readonly}
          >
            {ALL_OT.map((o) => (
              <option key={o} value={o}>{formatOt(o)}</option>
            ))}
          </select>
        </div>
      </div>

      {/* Setcodes + Categories - same row */}
      <div className={styles.setcodeCategoryRow}>
        <div className={styles.cardInfoField}>
          <div className={styles.tagFieldHeader}>
            <label className={styles.cardInfoLabel}>{t("card.info.setcodes")}</label>
            {draft.setcodes.length > 0 && (
              <span className={styles.tagFieldRaw}>
                {t("card.info.rawValue", { value: formatSetcodePackedHex(draft.setcodes) })}
              </span>
            )}
          </div>
          <div className={styles.setcodeTags}>
            {draft.setcodes.length === 0 ? (
              <span className={styles.setcodeEmptyTag}>{t("card.info.noArchetypes")}</span>
            ) : (
              draft.setcodes.map((code) => {
                const entry = setnameMap.get(code);
                const label = entry ? entry.name : formatSetcodeHex(code);
                const tooltip = entry
                  ? `${entry.name} (${formatSetcodeHex(code)})${entry.source === "pack" ? " [Pack]" : " [Std]"}`
                  : formatSetcodeHex(code);
                return (
                  <button
                    key={code}
                    type="button"
                    className={styles.setcodeTag}
                    onClick={() => {
                      emitChange({ setcodes: draft.setcodes.filter((c) => c !== code) });
                    }}
                    disabled={readonly}
                    title={t("card.info.removeTag", { target: tooltip })}
                  >
                    {label} &times;
                  </button>
                );
              })
            )}
            {draft.setcodes.length < 4 && (
              <button
                type="button"
                className={`${styles.setcodeAddButton} ${setcodePickerOpen ? "active" : ""}`}
                onClick={() => {
                  setSetcodePickerOpen((open) => !open);
                  setSetcodeSearch("");
                }}
                disabled={readonly}
                aria-expanded={setcodePickerOpen}
              >
                +
              </button>
            )}
          </div>
        </div>

        <div className={styles.cardInfoField}>
          <div className={styles.tagFieldHeader}>
            <label className={styles.cardInfoLabel}>{t("card.info.effectCategories")}</label>
            {normalizedCategory > 0 && (
              <span className={styles.tagFieldRaw}>{t("card.info.rawValue", { value: categoryRawDisplay })}</span>
            )}
          </div>
          <div className={styles.categoryTags}>
            {selectedCategoryOptions.length === 0 ? (
              <span className={styles.categoryEmptyTag}>{t("card.info.noCategories")}</span>
            ) : (
              selectedCategoryOptions.map((option) => (
                <button
                  key={option.bitIndex}
                  type="button"
                  className={styles.categoryTag}
                  onClick={() => toggleCategoryMask(option.mask)}
                  disabled={readonly}
                  title={t("card.info.removeTag", { target: formatCardCategory(option) })}
                >
                  {formatCardCategory(option)}
                </button>
              ))
            )}
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
        </div>

        {/* Expanded panels span full width below the row */}
        {setcodePickerOpen && (
          <div className={styles.expandedPanel}>
            <SetcodePicker
              search={setcodeSearch}
              onSearchChange={setSetcodeSearch}
              setnameEntries={setnameEntries}
              existingCodes={draft.setcodes}
              onSelect={(code) => {
                if (!draft.setcodes.includes(code) && draft.setcodes.length < 4) {
                  emitChange({ setcodes: [...draft.setcodes, code] });
                }
                setSetcodePickerOpen(false);
                setSetcodeSearch("");
              }}
              readonly={readonly}
            />
          </div>
        )}
        {categoryPickerOpen && (
          <div className={styles.expandedPanel}>
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
                    <span>{formatCardCategory(option)}</span>
                  </label>
                );
              })}
            </div>
            <details className={styles.categoryAdvanced}>
              <summary>{t("card.info.advancedRawMask")}</summary>
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
        <label className={styles.cardInfoLabel}>{t("card.info.primaryType")}</label>
        <div className={styles.cardTypeRadioGroup}>
          {ALL_PRIMARY_TYPES.map((pt) => (
            <button
              key={pt}
              type="button"
              className={`${styles.cardTypeRadio} ${draft.primary_type === pt ? "active" : ""}`}
              onClick={() => handlePrimaryTypeChange(pt)}
              disabled={readonly}
            >
              {formatPrimaryType(pt)}
            </button>
          ))}
        </div>
      </div>

      {/* Monster-specific fields */}
      {isMonster && (
        <div className={styles.cardInfoSection}>
          <h4 className={styles.cardInfoSectionTitle}>{formatPrimaryType("monster")}</h4>
          <div className={styles.cardInfoGrid}>
            <div className={`${styles.cardInfoField} ${styles.cardInfoFieldFull}`}>
              <label className={styles.cardInfoLabel}>{t("card.info.monsterFlags")}</label>
              <div className={styles.monsterFlagsGroup}>
                {ALL_MONSTER_FLAGS.map((flag) => (
                  <button
                    key={flag}
                    type="button"
                    className={`${styles.monsterFlagChip} ${flags.includes(flag) ? "selected" : ""}`}
                    onClick={() => toggleMonsterFlag(flag)}
                    disabled={readonly}
                  >
                    {formatMonsterFlag(flag)}
                  </button>
                ))}
              </div>
            </div>

            <div className={styles.cardInfoField}>
              <label className={styles.cardInfoLabel}>{t("card.info.attribute")}</label>
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
                  <option key={a} value={a}>{formatAttribute(a)}</option>
                ))}
              </select>
            </div>
            <div className={styles.cardInfoField}>
              <label className={styles.cardInfoLabel}>{t("card.info.race")}</label>
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
                  <option key={r} value={r}>{formatRace(r)}</option>
                ))}
              </select>
            </div>

            <div className={styles.cardInfoField}>
              <label className={styles.cardInfoLabel}>{t("card.info.atk")}</label>
              <input
                className={styles.cardInfoInput}
                type="text"
                value={formatStatValue(draft.atk)}
                onChange={(e) => handleNumberInput("atk", e.target.value)}
                placeholder={t("card.info.atkPlaceholder")}
                readOnly={readonly}
              />
            </div>
            {!isLink && (
              <div className={styles.cardInfoField}>
                <label className={styles.cardInfoLabel}>{t("card.info.def")}</label>
                <input
                  className={styles.cardInfoInput}
                  type="text"
                  value={formatStatValue(draft.def)}
                  onChange={(e) => handleNumberInput("def", e.target.value)}
                  placeholder={t("card.info.defPlaceholder")}
                  readOnly={readonly}
                />
              </div>
            )}
            {!isLink && (
              <div className={styles.cardInfoField}>
                <label className={styles.cardInfoLabel}>
                  {flags.includes("xyz") ? t("card.info.rank") : t("card.info.level")}
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
                  <label className={styles.cardInfoLabel}>{t("card.info.leftScale")}</label>
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
                  <label className={styles.cardInfoLabel}>{t("card.info.rightScale")}</label>
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
                <label className={styles.cardInfoLabel}>{t("card.info.linkMarkers")}</label>
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
                        title={formatLinkMarker(marker)}
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
          <h4 className={styles.cardInfoSectionTitle}>{formatPrimaryType("spell")}</h4>
          <div className={styles.cardInfoGrid}>
            <div className={styles.cardInfoField}>
              <label className={styles.cardInfoLabel}>{t("card.info.spellSubtype")}</label>
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
                  <option key={s} value={s}>{formatSpellSubtype(s)}</option>
                ))}
              </select>
            </div>
          </div>
        </div>
      )}

      {/* Trap-specific fields */}
      {isTrap && (
        <div className={styles.cardInfoSection}>
          <h4 className={styles.cardInfoSectionTitle}>{formatPrimaryType("trap")}</h4>
          <div className={styles.cardInfoGrid}>
            <div className={styles.cardInfoField}>
              <label className={styles.cardInfoLabel}>{t("card.info.trapSubtype")}</label>
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
                  <option key={t} value={t}>{formatTrapSubtype(t)}</option>
                ))}
              </select>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function SetcodePicker({
  search,
  onSearchChange,
  setnameEntries,
  existingCodes,
  onSelect,
  readonly,
}: {
  search: string;
  onSearchChange: (value: string) => void;
  setnameEntries: SetnameEntry[];
  existingCodes: number[];
  onSelect: (code: number) => void;
  readonly: boolean;
}) {
  const { t } = useAppI18n();
  const query = search.trim().toLowerCase();

  const filtered = useMemo(() => {
    if (!query) return setnameEntries;
    const hexQuery = query.replace(/^0x/i, "");
    return setnameEntries.filter((entry) => {
      if (entry.name.toLowerCase().includes(query)) return true;
      const hexKey = entry.key.toString(16);
      return hexKey.includes(hexQuery);
    });
  }, [setnameEntries, query]);

  const sorted = useMemo(() => {
    return [...filtered].sort((a, b) => {
      if (a.source !== b.source) return a.source === "pack" ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
  }, [filtered]);

  const parsedCustomHex = useMemo(() => {
    const raw = query.replace(/^0x/i, "");
    if (!raw || !/^[0-9a-fA-F]+$/.test(raw)) return null;
    const value = Number.parseInt(raw, 16);
    if (!Number.isFinite(value) || value <= 0 || value > 0xffff) return null;
    if (setnameEntries.some((e) => e.key === value)) return null;
    return value;
  }, [query, setnameEntries]);

  return (
    <div className={styles.setcodePickerPanel}>
      <input
        className={styles.setcodeSearchInput}
        type="text"
        value={search}
        onChange={(e) => onSearchChange(e.target.value)}
        placeholder={t("card.info.searchSetcode")}
        autoFocus
        readOnly={readonly}
      />
      <div className={styles.setcodePickerList}>
        {parsedCustomHex !== null && (
          <button
            type="button"
            className={`${styles.setcodePickerItem} ${
              existingCodes.includes(parsedCustomHex) ? styles.setcodePickerItemDisabled : ""
            }`}
            disabled={readonly || existingCodes.includes(parsedCustomHex)}
            onClick={() => onSelect(parsedCustomHex)}
          >
            <span className={styles.setcodePickerName}>
              {t("card.info.addCustomSetcode", { code: formatSetcodeHex(parsedCustomHex) })}
            </span>
          </button>
        )}
        {sorted.map((entry) => {
          const isAdded = existingCodes.includes(entry.key);
          return (
            <button
              key={`${entry.source}-${entry.key}`}
              type="button"
              className={`${styles.setcodePickerItem} ${
                isAdded ? styles.setcodePickerItemDisabled : ""
              }`}
              disabled={readonly || isAdded}
              onClick={() => onSelect(entry.key)}
            >
              <span className={styles.setcodePickerName}>{entry.name}</span>
              <span className={styles.setcodePickerMeta}>
                {formatSetcodeHex(entry.key)}
                <span className={styles.setcodeSourceBadge}>
                  {entry.source === "pack" ? "Pack" : "Std"}
                </span>
              </span>
            </button>
          );
        })}
        {sorted.length === 0 && parsedCustomHex === null && (
          <div className={styles.setcodePickerEmpty}>{t("card.info.noMatches")}</div>
        )}
      </div>
    </div>
  );
}
