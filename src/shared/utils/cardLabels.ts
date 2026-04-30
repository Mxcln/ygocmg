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
import type { CardCategoryOption } from "../constants/cardCategories";
import { formatAppMessageById } from "../i18n";
import type { AppMessageId } from "../i18n";

const SUBTYPE_DISPLAY_TO_ID: Record<string, AppMessageId> = {
  Normal: "card.flag.normal",
  Effect: "card.flag.effect",
  Fusion: "card.flag.fusion",
  Ritual: "card.flag.ritual",
  Synchro: "card.flag.synchro",
  Xyz: "card.flag.xyz",
  Pendulum: "card.flag.pendulum",
  Link: "card.flag.link",
  Tuner: "card.flag.tuner",
  Token: "card.flag.token",
  Gemini: "card.flag.gemini",
  Spirit: "card.flag.spirit",
  Union: "card.flag.union",
  Flip: "card.flag.flip",
  Toon: "card.flag.toon",
  Continuous: "card.spellSubtype.continuous",
  "Quick-Play": "card.spellSubtype.quick_play",
  Field: "card.spellSubtype.field",
  Equip: "card.spellSubtype.equip",
  Counter: "card.trapSubtype.counter",
};

const CATEGORY_MESSAGE_IDS: Record<number, AppMessageId> = {
  0: "card.category.spellTrapDestruction",
  1: "card.category.monsterDestruction",
  2: "card.category.banish",
  3: "card.category.sendToGy",
  4: "card.category.returnToHand",
  5: "card.category.returnToDeck",
  6: "card.category.handDestruction",
  7: "card.category.deckDestruction",
  8: "card.category.draw",
  9: "card.category.search",
  10: "card.category.recycle",
  11: "card.category.battlePosition",
  12: "card.category.control",
  13: "card.category.atkDefChange",
  14: "card.category.piercingDamage",
  15: "card.category.multipleAttacks",
  16: "card.category.attackRestriction",
  17: "card.category.directAttack",
  18: "card.category.specialSummon",
  19: "card.category.token",
  20: "card.category.raceRelated",
  21: "card.category.attributeRelated",
  22: "card.category.lpDamage",
  23: "card.category.lpRecovery",
  24: "card.category.destructionProtection",
  25: "card.category.effectProtection",
  26: "card.category.counter",
  27: "card.category.luck",
  28: "card.category.fusionRelated",
  29: "card.category.synchroRelated",
  30: "card.category.xyzRelated",
  31: "card.category.effectNegation",
};

export function formatPrimaryType(value: PrimaryType): string {
  return formatAppMessageById(`card.primary.${value}` as AppMessageId);
}

export function formatOt(value: Ot): string {
  return formatAppMessageById(`card.ot.${value}` as AppMessageId);
}

export function formatMonsterFlag(value: MonsterFlag): string {
  return formatAppMessageById(`card.flag.${value}` as AppMessageId);
}

export function formatRace(value: Race): string {
  return formatAppMessageById(`card.race.${value}` as AppMessageId);
}

export function formatAttribute(value: Attribute): string {
  return formatAppMessageById(`card.attribute.${value}` as AppMessageId);
}

export function formatSpellSubtype(value: SpellSubtype): string {
  return formatAppMessageById(`card.spellSubtype.${value}` as AppMessageId);
}

export function formatTrapSubtype(value: TrapSubtype): string {
  return formatAppMessageById(`card.trapSubtype.${value}` as AppMessageId);
}

export function formatLinkMarker(value: LinkMarker): string {
  return formatAppMessageById(`card.linkMarker.${value}` as AppMessageId);
}

export function formatCardCategory(option: CardCategoryOption): string {
  const id = CATEGORY_MESSAGE_IDS[option.bitIndex];
  return id ? formatAppMessageById(id) : option.label;
}

export function formatSubtypeDisplayPart(value: string): string {
  const id = SUBTYPE_DISPLAY_TO_ID[value];
  return id ? formatAppMessageById(id) : value;
}
