import { useEffect, useMemo, useState } from "react";
import type {
  Attribute,
  CardFilterMatchMode,
  CardSearchFilters,
  LinkMarker,
  MonsterFlag,
  NumericRangeFilter,
  Ot,
  PrimaryType,
  Race,
  SetcodeFilterMode,
  SpellSubtype,
  TrapSubtype,
} from "../../shared/contracts/card";
import {
  ALL_ATTRIBUTES,
  ALL_LINK_MARKERS,
  ALL_MONSTER_FLAGS,
  ALL_OT,
  ALL_PRIMARY_TYPES,
  ALL_RACES,
  ALL_SPELL_SUBTYPES,
  ALL_TRAP_SUBTYPES,
  LINK_MARKER_ARROWS,
} from "../../shared/constants/cardOptions";
import {
  CARD_CATEGORY_OPTIONS,
  formatCardCategoryMask,
} from "../../shared/constants/cardCategories";
import {
  formatAttribute,
  formatCardCategory,
  formatLinkMarker,
  formatMonsterFlag,
  formatOt,
  formatPrimaryType,
  formatRace,
  formatSpellSubtype,
  formatTrapSubtype,
} from "../../shared/utils/cardLabels";
import { useAppI18n } from "../../shared/i18n";
import { sortSetnameEntries, type SetnameEntry } from "../card/setnameEntries";
import shared from "../../shared/styles/shared.module.css";
import styles from "./StandardCardAdvancedSearchPanel.module.css";

interface CardAdvancedSearchPanelProps {
  open: boolean;
  filters: CardSearchFilters | null;
  setnameEntries?: SetnameEntry[];
  onChange: (filters: CardSearchFilters | null) => void;
  onClose: () => void;
}

interface FilterChip {
  id: string;
  label: string;
  onRemove: () => void;
}

type FilterArrayField =
  | "ots"
  | "primaryTypes"
  | "races"
  | "attributes"
  | "monsterFlags"
  | "spellSubtypes"
  | "trapSubtypes"
  | "linkMarkers"
  | "categoryMasks"
  | "setcodes";

const DEFAULT_SET_CODE_MODE: SetcodeFilterMode = "base";
const DEFAULT_MATCH_MODE: CardFilterMatchMode = "any";
type AdvancedSearchTab = "ids" | "text" | "type" | "monsterData" | "setcodes" | "category";
const SEARCH_TABS: AdvancedSearchTab[] = ["ids", "text", "type", "monsterData", "setcodes", "category"];

export function countCardFilters(filters: CardSearchFilters | null): number {
  const normalized = compactFilters(filters);
  if (!normalized) return 0;
  let count = 0;
  for (const [key, value] of Object.entries(normalized)) {
    if (key.endsWith("Match") || key === "setcodeMode") continue;
    if (Array.isArray(value) && value.length > 0) count += 1;
    else if (typeof value === "string" && value.trim()) count += 1;
    else if (isRangeActive(value as NumericRangeFilter | null | undefined)) count += 1;
  }
  return count;
}

export function cardFiltersKey(filters: CardSearchFilters | null): string {
  return JSON.stringify(compactFilters(filters) ?? {});
}

export function compactFilters(
  filters: CardSearchFilters | null | undefined,
): CardSearchFilters | null {
  if (!filters) return null;
  const next: CardSearchFilters = {};

  copyNumberArray(next, "codes", filters.codes);
  copyRange(next, "codeRange", filters.codeRange);
  copyNumberArray(next, "aliases", filters.aliases);
  copyRange(next, "aliasRange", filters.aliasRange);
  copyStringArray(next, "ots", filters.ots);
  copyText(next, "nameContains", filters.nameContains);
  copyText(next, "descContains", filters.descContains);
  copyStringArray(next, "primaryTypes", filters.primaryTypes);
  copyStringArray(next, "races", filters.races);
  copyStringArray(next, "attributes", filters.attributes);
  copyStringArray(next, "monsterFlags", filters.monsterFlags);
  if (next.monsterFlags?.length) next.monsterFlagMatch = filters.monsterFlagMatch ?? DEFAULT_MATCH_MODE;
  copyStringArray(next, "spellSubtypes", filters.spellSubtypes);
  copyStringArray(next, "trapSubtypes", filters.trapSubtypes);
  copyRange(next, "pendulumLeftScale", filters.pendulumLeftScale);
  copyRange(next, "pendulumRightScale", filters.pendulumRightScale);
  copyStringArray(next, "linkMarkers", filters.linkMarkers);
  if (next.linkMarkers?.length) next.linkMarkerMatch = filters.linkMarkerMatch ?? DEFAULT_MATCH_MODE;
  copyNumberArray(next, "setcodes", filters.setcodes);
  if (next.setcodes?.length) {
    next.setcodeMode = filters.setcodeMode ?? DEFAULT_SET_CODE_MODE;
    next.setcodeMatch = filters.setcodeMatch ?? DEFAULT_MATCH_MODE;
  }
  copyNumberArray(next, "categoryMasks", filters.categoryMasks);
  if (next.categoryMasks?.length) next.categoryMatch = filters.categoryMatch ?? DEFAULT_MATCH_MODE;
  copyRange(next, "atk", filters.atk);
  copyRange(next, "def", filters.def);
  copyRange(next, "level", filters.level);

  return Object.keys(next).length > 0 ? next : null;
}

export const countStandardCardFilters = countCardFilters;
export const standardCardFiltersKey = cardFiltersKey;

export function CardAdvancedSearchPanel({
  open,
  filters,
  setnameEntries = [],
  onChange,
  onClose,
}: CardAdvancedSearchPanelProps) {
  const { t } = useAppI18n();
  const [activeTab, setActiveTab] = useState<AdvancedSearchTab>("ids");
  const [nameContains, setNameContains] = useState(filters?.nameContains ?? "");
  const [descContains, setDescContains] = useState(filters?.descContains ?? "");
  const [setnameSearch, setSetnameSearch] = useState("");

  useEffect(() => {
    setNameContains(filters?.nameContains ?? "");
  }, [filters?.nameContains]);

  useEffect(() => {
    setDescContains(filters?.descContains ?? "");
  }, [filters?.descContains]);

  useEffect(() => {
    const handle = window.setTimeout(() => {
      const value = normalizeText(nameContains);
      if ((filters?.nameContains ?? null) !== value) {
        update({ nameContains: value });
      }
    }, 250);
    return () => window.clearTimeout(handle);
  }, [filters, nameContains]);

  useEffect(() => {
    const handle = window.setTimeout(() => {
      const value = normalizeText(descContains);
      if ((filters?.descContains ?? null) !== value) {
        update({ descContains: value });
      }
    }, 250);
    return () => window.clearTimeout(handle);
  }, [descContains, filters]);

  useEffect(() => {
    if (!open) return;
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose, open]);

  const selectedSetcodes = filters?.setcodes ?? [];
  const filteredSetnames = useMemo(() => {
    const query = setnameSearch.trim().toLowerCase();
    const hexQuery = query.replace(/^0x/i, "");
    const entries = sortSetnameEntries(setnameEntries);
    if (!query) return entries.slice(0, 24);
    return entries
      .filter((entry) => {
        if (entry.name.toLowerCase().includes(query)) return true;
        return entry.key.toString(16).includes(hexQuery);
      })
      .slice(0, 24);
  }, [setnameEntries, setnameSearch]);
  const customSetcode = useMemo(() => parseHexSetcode(setnameSearch), [setnameSearch]);
  const chips = buildChips();

  function update(patch: Partial<CardSearchFilters>) {
    onChange(compactFilters({ ...(filters ?? {}), ...patch }));
  }

  function clearField(field: keyof CardSearchFilters) {
    update({ [field]: undefined } as Partial<CardSearchFilters>);
  }

  function toggleArrayValue<T extends string | number>(
    field: FilterArrayField,
    value: T,
  ) {
    const current = ((filters?.[field] ?? []) as T[]);
    const next = current.includes(value)
      ? current.filter((item) => item !== value)
      : [...current, value];
    update({ [field]: next } as Partial<CardSearchFilters>);
  }

  function setRange(
    field: keyof Pick<
      CardSearchFilters,
      | "codeRange"
      | "aliasRange"
      | "pendulumLeftScale"
      | "pendulumRightScale"
      | "atk"
      | "def"
      | "level"
    >,
    side: keyof NumericRangeFilter,
    raw: string,
  ) {
    const current = filters?.[field] ?? null;
    update({
      [field]: compactRange({
        min: side === "min" ? parseNullableNumber(raw) : current?.min ?? null,
        max: side === "max" ? parseNullableNumber(raw) : current?.max ?? null,
      }),
    } as Partial<CardSearchFilters>);
  }

  function setNumberList(field: "codes" | "aliases", raw: string) {
    update({ [field]: parseDecimalList(raw) } as Partial<CardSearchFilters>);
  }

  function selectSetcode(value: number) {
    if (value <= 0 || value > 0xffff) return;
    toggleArrayValue("setcodes", value);
  }

  function buildChips(): FilterChip[] {
    const result: FilterChip[] = [];
    const f = compactFilters(filters);
    if (!f) return result;
    addArrayChip(result, "codes", f.codes, () => t("standard.search.codesChip", { value: numberListLabel(f.codes) }), () => clearField("codes"));
    addLabelChip(result, "codeRange", rangeChipLabel(t("standard.search.codeRange"), f.codeRange), () => clearField("codeRange"));
    addArrayChip(result, "aliases", f.aliases, () => t("standard.search.aliasesChip", { value: numberListLabel(f.aliases) }), () => clearField("aliases"));
    addLabelChip(result, "aliasRange", rangeChipLabel(t("standard.search.aliasRange"), f.aliasRange), () => clearField("aliasRange"));
    addArrayChip(result, "ots", f.ots, () => t("standard.search.otChip", { value: labelList(f.ots, formatOt) }), () => clearField("ots"));
    addValueChip(result, "name", f.nameContains, () => t("standard.search.nameChip", { value: f.nameContains ?? "" }), () => clearField("nameContains"));
    addValueChip(result, "desc", f.descContains, () => t("standard.search.descChip", { value: f.descContains ?? "" }), () => clearField("descContains"));
    addArrayChip(result, "primary", f.primaryTypes, () => t("standard.search.primaryChip", { value: labelList(f.primaryTypes, formatPrimaryType) }), () => clearField("primaryTypes"));
    addArrayChip(result, "race", f.races, () => t("standard.search.raceChip", { value: labelList(f.races, formatRace) }), () => clearField("races"));
    addArrayChip(result, "attribute", f.attributes, () => t("standard.search.attributeChip", { value: labelList(f.attributes, formatAttribute) }), () => clearField("attributes"));
    addArrayChip(result, "flags", f.monsterFlags, () => t("standard.search.flagsChip", { value: labelList(f.monsterFlags, formatMonsterFlag) }), () => clearField("monsterFlags"));
    addArrayChip(result, "spell", f.spellSubtypes, () => t("standard.search.spellChip", { value: labelList(f.spellSubtypes, formatSpellSubtype) }), () => clearField("spellSubtypes"));
    addArrayChip(result, "trap", f.trapSubtypes, () => t("standard.search.trapChip", { value: labelList(f.trapSubtypes, formatTrapSubtype) }), () => clearField("trapSubtypes"));
    addLabelChip(result, "leftScale", rangeChipLabel(t("standard.search.leftScale"), f.pendulumLeftScale), () => clearField("pendulumLeftScale"));
    addLabelChip(result, "rightScale", rangeChipLabel(t("standard.search.rightScale"), f.pendulumRightScale), () => clearField("pendulumRightScale"));
    addArrayChip(result, "markers", f.linkMarkers, () => t("standard.search.markersChip", { value: labelList(f.linkMarkers, formatLinkMarker) }), () => clearField("linkMarkers"));
    addArrayChip(result, "setcodes", f.setcodes, () => t("standard.search.setcodesChip", { value: setcodeListLabel(f.setcodes) }), () => clearField("setcodes"));
    addArrayChip(result, "category", f.categoryMasks, () => t("standard.search.categoryChip", { value: categoryLabel(f.categoryMasks) }), () => clearField("categoryMasks"));
    addLabelChip(result, "atk", rangeChipLabel(t("card.info.atk"), f.atk), () => clearField("atk"));
    addLabelChip(result, "def", rangeChipLabel(t("card.info.def"), f.def), () => clearField("def"));
    addLabelChip(result, "level", rangeChipLabel(t("card.list.levelShort"), f.level), () => clearField("level"));
    return result;
  }

  function tabLabel(tab: AdvancedSearchTab): string {
    switch (tab) {
      case "ids":
        return t("standard.search.group.ids");
      case "text":
        return t("standard.search.group.text");
      case "type":
        return t("standard.search.group.type");
      case "monsterData":
        return t("standard.search.group.monsterData");
      case "setcodes":
        return t("standard.search.group.setcodes");
      case "category":
        return t("standard.search.group.category");
    }
  }

  function renderActiveTab() {
    switch (activeTab) {
      case "ids":
        return (
          <section className={styles.filterSection}>
            <h3>{t("standard.search.group.ids")}</h3>
            <div className={styles.formGrid}>
              <TextField label={t("standard.search.codes")} value={numberListInput(filters?.codes)} onChange={(value) => setNumberList("codes", value)} />
              <RangeField label={t("standard.search.codeRange")} value={filters?.codeRange} onChange={(side, value) => setRange("codeRange", side, value)} />
              <TextField label={t("standard.search.aliases")} value={numberListInput(filters?.aliases)} onChange={(value) => setNumberList("aliases", value)} />
              <RangeField label={t("standard.search.aliasRange")} value={filters?.aliasRange} onChange={(side, value) => setRange("aliasRange", side, value)} />
            </div>
            <ChipGroup
              label={t("card.info.ot")}
              options={ALL_OT}
              selected={filters?.ots ?? []}
              format={formatOt}
              onToggle={(value) => toggleArrayValue("ots", value)}
            />
          </section>
        );
      case "text":
        return (
          <section className={styles.filterSection}>
            <h3>{t("standard.search.group.text")}</h3>
            <div className={styles.formGrid}>
              <TextField label={t("standard.search.nameContains")} value={nameContains} onChange={setNameContains} />
              <TextField label={t("standard.search.descContains")} value={descContains} onChange={setDescContains} />
            </div>
          </section>
        );
      case "type":
        return (
          <section className={styles.filterSection}>
            <h3>{t("standard.search.group.type")}</h3>
            <ChipGroup
              label={t("card.info.primaryType")}
              options={ALL_PRIMARY_TYPES}
              selected={filters?.primaryTypes ?? []}
              format={formatPrimaryType}
              onToggle={(value) => toggleArrayValue("primaryTypes", value)}
            />
            <MatchModeRow value={filters?.monsterFlagMatch ?? DEFAULT_MATCH_MODE} onChange={(value) => update({ monsterFlagMatch: value })} />
            <ChipGroup
              label={t("card.info.monsterFlags")}
              options={ALL_MONSTER_FLAGS}
              selected={filters?.monsterFlags ?? []}
              format={formatMonsterFlag}
              onToggle={(value) => toggleArrayValue("monsterFlags", value)}
            />
            <ChipGroup
              label={t("card.info.spellSubtype")}
              options={ALL_SPELL_SUBTYPES}
              selected={filters?.spellSubtypes ?? []}
              format={formatSpellSubtype}
              onToggle={(value) => toggleArrayValue("spellSubtypes", value)}
            />
            <ChipGroup
              label={t("card.info.trapSubtype")}
              options={ALL_TRAP_SUBTYPES}
              selected={filters?.trapSubtypes ?? []}
              format={formatTrapSubtype}
              onToggle={(value) => toggleArrayValue("trapSubtypes", value)}
            />
          </section>
        );
      case "monsterData":
        return (
          <section className={styles.filterSection}>
            <h3>{t("standard.search.group.monsterData")}</h3>
            <ChipGroup
              label={t("card.info.race")}
              options={ALL_RACES}
              selected={filters?.races ?? []}
              format={formatRace}
              onToggle={(value) => toggleArrayValue("races", value)}
            />
            <ChipGroup
              label={t("card.info.attribute")}
              options={ALL_ATTRIBUTES}
              selected={filters?.attributes ?? []}
              format={formatAttribute}
              onToggle={(value) => toggleArrayValue("attributes", value)}
            />
            <div className={styles.formGrid}>
              <RangeField label={t("card.info.atk")} value={filters?.atk} onChange={(side, value) => setRange("atk", side, value)} />
              <RangeField label={t("card.info.def")} value={filters?.def} onChange={(side, value) => setRange("def", side, value)} />
              <RangeField label={t("card.list.levelShort")} value={filters?.level} onChange={(side, value) => setRange("level", side, value)} />
              <RangeField label={t("standard.search.leftScale")} value={filters?.pendulumLeftScale} onChange={(side, value) => setRange("pendulumLeftScale", side, value)} />
              <RangeField label={t("standard.search.rightScale")} value={filters?.pendulumRightScale} onChange={(side, value) => setRange("pendulumRightScale", side, value)} />
            </div>
            <MatchModeRow value={filters?.linkMarkerMatch ?? DEFAULT_MATCH_MODE} onChange={(value) => update({ linkMarkerMatch: value })} />
            <div className={styles.linkMarkerGrid}>
              {ALL_LINK_MARKERS.map((marker) => (
                <button
                  key={marker}
                  type="button"
                  className={`${styles.linkMarkerButton} ${(filters?.linkMarkers ?? []).includes(marker) ? styles.selected : ""}`}
                  onClick={() => toggleArrayValue("linkMarkers", marker)}
                  title={formatLinkMarker(marker)}
                >
                  {LINK_MARKER_ARROWS[marker]}
                </button>
              ))}
            </div>
          </section>
        );
      case "setcodes":
        return (
          <section className={styles.filterSection}>
            <h3>{t("standard.search.group.setcodes")}</h3>
            <div className={styles.modeRow}>
              <label>
                <span>{t("standard.search.setcodeMode")}</span>
                <select value={filters?.setcodeMode ?? DEFAULT_SET_CODE_MODE} onChange={(event) => update({ setcodeMode: event.target.value as SetcodeFilterMode })}>
                  <option value="base">{t("standard.search.setcodeMode.base")}</option>
                  <option value="exact">{t("standard.search.setcodeMode.exact")}</option>
                </select>
              </label>
              <MatchModeRow value={filters?.setcodeMatch ?? DEFAULT_MATCH_MODE} onChange={(value) => update({ setcodeMatch: value })} />
            </div>
            <input
              className={styles.searchInput}
              type="text"
              value={setnameSearch}
              onChange={(event) => setSetnameSearch(event.target.value)}
              placeholder={t("standard.search.setnameSearch")}
            />
            <div className={styles.setnameList}>
              {customSetcode !== null && (
                <button type="button" className={styles.setnameItem} onClick={() => selectSetcode(customSetcode)}>
                  <span>{t("standard.search.customSetcode", { code: formatSetcodeHex(customSetcode) })}</span>
                </button>
              )}
              {filteredSetnames.map((entry) => (
                <button
                  key={`${entry.source}-${entry.key}`}
                  type="button"
                  className={`${styles.setnameItem} ${selectedSetcodes.includes(entry.key) ? styles.selected : ""}`}
                  onClick={() => selectSetcode(entry.key)}
                >
                  <span>{entry.name}</span>
                  <small>
                    {formatSetcodeHex(entry.key)}
                    {entry.source === "pack" ? " · Pack" : " · Std"}
                  </small>
                </button>
              ))}
            </div>
          </section>
        );
      case "category":
        return (
          <section className={styles.filterSection}>
            <h3>{t("standard.search.group.category")}</h3>
            <MatchModeRow value={filters?.categoryMatch ?? DEFAULT_MATCH_MODE} onChange={(value) => update({ categoryMatch: value })} />
            <div className={styles.categoryGrid}>
              {CARD_CATEGORY_OPTIONS.map((option) => (
                <button
                  key={option.bitIndex}
                  type="button"
                  className={`${styles.categoryButton} ${(filters?.categoryMasks ?? []).includes(option.mask) ? styles.selected : ""}`}
                  onClick={() => toggleArrayValue("categoryMasks", option.mask)}
                >
                  {formatCardCategory(option)}
                </button>
              ))}
            </div>
          </section>
        );
    }
  }

  return (
    <div className={styles.advancedSearchWrap}>
      {chips.length > 0 && (
        <div className={styles.activeFilters}>
          {chips.map((chip) => (
            <button
              key={chip.id}
              type="button"
              className={styles.filterChip}
              onClick={chip.onRemove}
              title={t("standard.search.removeFilter", { label: chip.label })}
            >
              <span>{chip.label}</span>
              <strong aria-hidden="true">x</strong>
            </button>
          ))}
          <button type="button" className={styles.clearButton} onClick={() => onChange(null)}>
            {t("standard.search.clearAll")}
          </button>
        </div>
      )}
      {open && (
        <div className={styles.modalLayer}>
          <div className={styles.modalBackdrop} onClick={onClose} />
          <section
            className={styles.modalBox}
            role="dialog"
            aria-modal="true"
            aria-labelledby="standard-search-title"
          >
            <header className={shared.modalHeader}>
              <h2 id="standard-search-title">{t("standard.search.title")}</h2>
              <button type="button" className={shared.modalCloseButton} onClick={onClose}>
                {t("action.close")}
              </button>
            </header>
            <div className={styles.modalBody}>
              <aside className={styles.modalTabs}>
                {SEARCH_TABS.map((tab) => (
                  <button
                    key={tab}
                    type="button"
                    className={activeTab === tab ? "active" : ""}
                    onClick={() => setActiveTab(tab)}
                  >
                    {tabLabel(tab)}
                  </button>
                ))}
              </aside>
              <div className={styles.modalPanel}>{renderActiveTab()}</div>
            </div>
          </section>
        </div>
      )}
    </div>
  );
}

export const StandardCardAdvancedSearchPanel = CardAdvancedSearchPanel;

function TextField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className={styles.field}>
      <span>{label}</span>
      <input type="text" value={value} onChange={(event) => onChange(event.target.value)} />
    </label>
  );
}

function RangeField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: NumericRangeFilter | null | undefined;
  onChange: (side: keyof NumericRangeFilter, value: string) => void;
}) {
  const { t } = useAppI18n();
  return (
    <div className={styles.rangeField}>
      <span>{label}</span>
      <input
        type="number"
        value={value?.min ?? ""}
        placeholder={t("standard.search.min")}
        onChange={(event) => onChange("min", event.target.value)}
      />
      <input
        type="number"
        value={value?.max ?? ""}
        placeholder={t("standard.search.max")}
        onChange={(event) => onChange("max", event.target.value)}
      />
    </div>
  );
}

function ChipGroup<T extends string>({
  label,
  options,
  selected,
  format,
  onToggle,
}: {
  label: string;
  options: T[];
  selected: T[];
  format: (value: T) => string;
  onToggle: (value: T) => void;
}) {
  return (
    <div className={styles.chipGroup}>
      <span>{label}</span>
      <div>
        {options.map((option) => (
          <button
            key={option}
            type="button"
            className={selected.includes(option) ? styles.selected : ""}
            onClick={() => onToggle(option)}
          >
            {format(option)}
          </button>
        ))}
      </div>
    </div>
  );
}

function MatchModeRow({
  value,
  onChange,
}: {
  value: CardFilterMatchMode;
  onChange: (value: CardFilterMatchMode) => void;
}) {
  const { t } = useAppI18n();
  return (
    <label className={styles.matchMode}>
      <span>{t("standard.search.matchMode")}</span>
      <select value={value} onChange={(event) => onChange(event.target.value as CardFilterMatchMode)}>
        <option value="any">{t("standard.search.match.any")}</option>
        <option value="all">{t("standard.search.match.all")}</option>
      </select>
    </label>
  );
}

function addArrayChip<T>(
  chips: FilterChip[],
  id: string,
  values: T[] | undefined,
  label: () => string,
  onRemove: () => void,
) {
  if (!values || values.length === 0) return;
  chips.push({ id, label: label(), onRemove });
}

function addValueChip(
  chips: FilterChip[],
  id: string,
  value: string | null | undefined,
  label: () => string,
  onRemove: () => void,
) {
  if (!value?.trim()) return;
  chips.push({ id, label: label(), onRemove });
}

function addLabelChip(
  chips: FilterChip[],
  id: string,
  label: string | null,
  onRemove: () => void,
) {
  if (!label) return;
  chips.push({ id, label, onRemove });
}

function copyText<T extends keyof CardSearchFilters>(
  target: CardSearchFilters,
  key: T,
  value: string | null | undefined,
) {
  const normalized = normalizeText(value);
  if (normalized) {
    (target[key] as string | null | undefined) = normalized;
  }
}

function copyNumberArray<T extends keyof CardSearchFilters>(
  target: CardSearchFilters,
  key: T,
  value: number[] | null | undefined,
) {
  const normalized = uniqueNumbers(value);
  if (normalized.length > 0) {
    (target[key] as number[] | undefined) = normalized;
  }
}

function copyStringArray<T extends keyof CardSearchFilters, V extends string>(
  target: CardSearchFilters,
  key: T,
  value: V[] | null | undefined,
) {
  const normalized = uniqueStrings(value);
  if (normalized.length > 0) {
    (target[key] as V[] | undefined) = normalized;
  }
}

function copyRange<T extends keyof CardSearchFilters>(
  target: CardSearchFilters,
  key: T,
  value: NumericRangeFilter | null | undefined,
) {
  const normalized = compactRange(value);
  if (normalized) {
    (target[key] as NumericRangeFilter | null | undefined) = normalized;
  }
}

function compactRange(value: NumericRangeFilter | null | undefined): NumericRangeFilter | null {
  if (!value) return null;
  const min = value.min !== null && Number.isFinite(value.min) ? Math.trunc(value.min) : null;
  const max = value.max !== null && Number.isFinite(value.max) ? Math.trunc(value.max) : null;
  if (min === null && max === null) return null;
  return { min, max };
}

function isRangeActive(value: NumericRangeFilter | null | undefined): boolean {
  return Boolean(value && (value.min !== null || value.max !== null));
}

function normalizeText(value: string | null | undefined): string | null {
  const trimmed = value?.trim() ?? "";
  return trimmed ? trimmed : null;
}

function uniqueNumbers(value: number[] | null | undefined): number[] {
  return [...new Set((value ?? []).map((item) => Math.trunc(item)).filter((item) => Number.isFinite(item) && item > 0))]
    .sort((left, right) => left - right);
}

function uniqueStrings<T extends string>(value: T[] | null | undefined): T[] {
  return [...new Set(value ?? [])].sort();
}

function parseNullableNumber(raw: string): number | null {
  if (raw.trim() === "") return null;
  const parsed = Number.parseInt(raw, 10);
  return Number.isFinite(parsed) ? parsed : null;
}

function parseDecimalList(raw: string): number[] {
  return uniqueNumbers(
    raw
      .split(/[,\s]+/)
      .map((part) => Number.parseInt(part, 10))
      .filter(Number.isFinite),
  );
}

function parseHexSetcode(raw: string): number | null {
  const value = raw.trim().replace(/^0x/i, "");
  if (!value || !/^[0-9a-fA-F]+$/.test(value)) return null;
  const parsed = Number.parseInt(value, 16);
  if (!Number.isFinite(parsed) || parsed <= 0 || parsed > 0xffff) return null;
  return parsed;
}

function numberListInput(value: number[] | null | undefined): string {
  return value?.join(", ") ?? "";
}

function numberListLabel(value: number[] | undefined): string {
  return value?.join(", ") ?? "";
}

function labelList<T>(values: T[] | undefined, format: (value: T) => string): string {
  return values?.map(format).join(", ") ?? "";
}

function setcodeListLabel(values: number[] | undefined): string {
  return values?.map(formatSetcodeHex).join(", ") ?? "";
}

function categoryLabel(values: number[] | undefined): string {
  return values
    ?.map((mask) => CARD_CATEGORY_OPTIONS.find((option) => option.mask === mask))
    .map((option, index) => option ? formatCardCategory(option) : formatCardCategoryMask(values?.[index] ?? 0))
    .join(", ") ?? "";
}

function rangeChipLabel(label: string, value: NumericRangeFilter | null | undefined): string | null {
  if (!isRangeActive(value)) return null;
  const min = value?.min ?? "";
  const max = value?.max ?? "";
  return `${label}: ${min}-${max}`;
}

function formatSetcodeHex(value: number): string {
  return `0x${value.toString(16).toUpperCase()}`;
}
