import type {
  Attribute,
  LinkMarker,
  MonsterFlag,
  Ot,
  PrimaryType,
  Race,
  SpellSubtype,
  TrapSubtype,
} from "../contracts/card";

export const ALL_PRIMARY_TYPES: PrimaryType[] = ["monster", "spell", "trap"];

export const ALL_OT: Ot[] = ["ocg", "tcg", "custom"];

export const ALL_MONSTER_FLAGS: MonsterFlag[] = [
  "normal", "effect", "fusion", "ritual", "synchro", "xyz",
  "pendulum", "link", "tuner", "token", "gemini", "spirit",
  "union", "flip", "toon",
];

export const ALL_RACES: Race[] = [
  "warrior", "spellcaster", "fairy", "fiend", "zombie", "machine",
  "aqua", "pyro", "rock", "winged_beast", "plant", "insect",
  "thunder", "dragon", "beast", "beast_warrior", "dinosaur",
  "fish", "sea_serpent", "reptile", "psychic", "divine_beast",
  "creator_god", "wyrm", "cyberse", "illusion",
];

export const ALL_ATTRIBUTES: Attribute[] = [
  "light", "dark", "earth", "water", "fire", "wind", "divine",
];

export const ALL_SPELL_SUBTYPES: SpellSubtype[] = [
  "normal", "continuous", "quick_play", "ritual", "field", "equip",
];

export const ALL_TRAP_SUBTYPES: TrapSubtype[] = [
  "normal", "continuous", "counter",
];

export const ALL_LINK_MARKERS: LinkMarker[] = [
  "top_left", "top", "top_right",
  "left", "right",
  "bottom_left", "bottom", "bottom_right",
];

export const LINK_MARKER_POSITIONS: (LinkMarker | null)[][] = [
  ["top_left", "top", "top_right"],
  ["left", null, "right"],
  ["bottom_left", "bottom", "bottom_right"],
];

export const LINK_MARKER_ARROWS: Record<LinkMarker, string> = {
  top_left: "\u2196",
  top: "\u2191",
  top_right: "\u2197",
  left: "\u2190",
  right: "\u2192",
  bottom_left: "\u2199",
  bottom: "\u2193",
  bottom_right: "\u2198",
};
