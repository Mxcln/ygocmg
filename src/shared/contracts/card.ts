import type { CardId, LanguageCode } from "./common";

export type PrimaryType = "monster" | "spell" | "trap";
export type Ot = "ocg" | "tcg" | "custom";
export type MonsterFlag =
  | "normal"
  | "effect"
  | "fusion"
  | "ritual"
  | "synchro"
  | "xyz"
  | "pendulum"
  | "link"
  | "tuner"
  | "token"
  | "gemini"
  | "spirit"
  | "union"
  | "flip"
  | "toon";
export type Race =
  | "warrior"
  | "spellcaster"
  | "dragon"
  | "zombie"
  | "machine"
  | "aqua"
  | "pyro"
  | "rock"
  | "winged_beast"
  | "plant"
  | "insect"
  | "thunder"
  | "fish"
  | "sea_serpent"
  | "reptile"
  | "psychic"
  | "divine_beast"
  | "beast"
  | "beast_warrior"
  | "dinosaur"
  | "fairy"
  | "fiend"
  | "illusion"
  | "cyberse"
  | "creator_god"
  | "wyrm";
export type Attribute =
  | "light"
  | "dark"
  | "earth"
  | "water"
  | "fire"
  | "wind"
  | "divine";
export type SpellSubtype =
  | "normal"
  | "continuous"
  | "quick_play"
  | "ritual"
  | "field"
  | "equip";
export type TrapSubtype = "normal" | "continuous" | "counter";
export type LinkMarker =
  | "top"
  | "bottom"
  | "left"
  | "right"
  | "top_left"
  | "top_right"
  | "bottom_left"
  | "bottom_right";

export interface CardTexts {
  name: string;
  desc: string;
  strings: string[];
}

export interface Pendulum {
  left_scale: number;
  right_scale: number;
}

export interface LinkData {
  markers: LinkMarker[];
}

export interface CardEntity {
  id: CardId;
  code: number;
  alias: number;
  setcode: number;
  ot: Ot;
  category: number;
  primary_type: PrimaryType;
  texts: Record<LanguageCode, CardTexts>;
  monster_flags: MonsterFlag[] | null;
  atk: number | null;
  def: number | null;
  race: Race | null;
  attribute: Attribute | null;
  level: number | null;
  pendulum: Pendulum | null;
  link: LinkData | null;
  spell_subtype: SpellSubtype | null;
  trap_subtype: TrapSubtype | null;
  created_at: string;
  updated_at: string;
}

export interface CardListRow {
  id: CardId;
  code: number;
  name: string;
  desc: string;
  primary_type: PrimaryType;
  atk: number | null;
  def: number | null;
  level: number | null;
  has_image: boolean;
  has_script: boolean;
  has_field_image: boolean;
}
