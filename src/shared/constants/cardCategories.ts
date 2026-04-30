export interface CardCategoryOption {
  bitIndex: number;
  mask: number;
  label: string;
}

// YGOPro CDB datas.category search tags. These correspond to strings.conf
// system strings 1100-1131, not Lua CATEGORY_* effect constants.
export const CARD_CATEGORY_OPTIONS: CardCategoryOption[] = [
  { bitIndex: 0, mask: 0x00000001, label: "Spell/Trap Destruction" },
  { bitIndex: 1, mask: 0x00000002, label: "Monster Destruction" },
  { bitIndex: 2, mask: 0x00000004, label: "Banish" },
  { bitIndex: 3, mask: 0x00000008, label: "Send to GY" },
  { bitIndex: 4, mask: 0x00000010, label: "Return to Hand" },
  { bitIndex: 5, mask: 0x00000020, label: "Return to Deck" },
  { bitIndex: 6, mask: 0x00000040, label: "Hand Destruction" },
  { bitIndex: 7, mask: 0x00000080, label: "Deck Destruction" },
  { bitIndex: 8, mask: 0x00000100, label: "Draw" },
  { bitIndex: 9, mask: 0x00000200, label: "Search" },
  { bitIndex: 10, mask: 0x00000400, label: "Recycle" },
  { bitIndex: 11, mask: 0x00000800, label: "Battle Position" },
  { bitIndex: 12, mask: 0x00001000, label: "Control" },
  { bitIndex: 13, mask: 0x00002000, label: "ATK/DEF Change" },
  { bitIndex: 14, mask: 0x00004000, label: "Piercing Damage" },
  { bitIndex: 15, mask: 0x00008000, label: "Multiple Attacks" },
  { bitIndex: 16, mask: 0x00010000, label: "Attack Restriction" },
  { bitIndex: 17, mask: 0x00020000, label: "Direct Attack" },
  { bitIndex: 18, mask: 0x00040000, label: "Special Summon" },
  { bitIndex: 19, mask: 0x00080000, label: "Token" },
  { bitIndex: 20, mask: 0x00100000, label: "Race Related" },
  { bitIndex: 21, mask: 0x00200000, label: "Attribute Related" },
  { bitIndex: 22, mask: 0x00400000, label: "LP Damage" },
  { bitIndex: 23, mask: 0x00800000, label: "LP Recovery" },
  { bitIndex: 24, mask: 0x01000000, label: "Destruction Protection" },
  { bitIndex: 25, mask: 0x02000000, label: "Effect Protection" },
  { bitIndex: 26, mask: 0x04000000, label: "Counter" },
  { bitIndex: 27, mask: 0x08000000, label: "Luck" },
  { bitIndex: 28, mask: 0x10000000, label: "Fusion Related" },
  { bitIndex: 29, mask: 0x20000000, label: "Synchro Related" },
  { bitIndex: 30, mask: 0x40000000, label: "Xyz Related" },
  { bitIndex: 31, mask: 0x80000000, label: "Effect Negation" },
];

export const CARD_CATEGORY_MAX_MASK = 0xffffffff;

export function formatCardCategoryMask(value: number): string {
  const normalized = normalizeCardCategoryMask(value);
  return `0x${normalized.toString(16).toUpperCase().padStart(8, "0")}`;
}

export function hasCardCategoryMask(category: number, mask: number): boolean {
  const normalized = normalizeCardCategoryMask(category);
  return Math.floor(normalized / mask) % 2 === 1;
}

export function normalizeCardCategoryMask(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.min(Math.max(Math.trunc(value), 0), CARD_CATEGORY_MAX_MASK);
}

