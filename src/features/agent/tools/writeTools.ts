import { cardApi } from "../../../shared/api/cardApi";
import { stringsApi } from "../../../shared/api/stringsApi";
import { useShellStore } from "../../../shared/stores/shellStore";
import { parseHexInput } from "../../../shared/utils/format";
import type {
  Attribute,
  CardEntity,
  LinkMarker,
  MonsterFlag,
  Ot,
  PrimaryType,
  Race,
  SpellSubtype,
  TrapSubtype,
  WriteResult,
} from "../../../shared/contracts/card";
import type { AgentTool } from "./types";
import { requirePack, ToolError } from "./types";

/** Strip server-managed fields so the entity matches the create/update input shape. */
type CardInput = Omit<CardEntity, "id" | "created_at" | "updated_at">;

function toCardInput(card: CardEntity): CardInput {
  const { id: _id, created_at: _c, updated_at: _u, ...rest } = card;
  return rest;
}

// Enum value sets, mirroring src/shared/contracts/card.ts. Used both for the
// tool JSON Schema (so the model sees valid values) and for runtime validation
// (so a bad value fails here with a clear message rather than confusing the backend).
const PRIMARY_TYPES: PrimaryType[] = ["monster", "spell", "trap"];
const OTS: Ot[] = ["ocg", "tcg", "custom"];
const MONSTER_FLAGS: MonsterFlag[] = [
  "normal", "effect", "fusion", "ritual", "synchro", "xyz", "pendulum", "link",
  "tuner", "token", "gemini", "spirit", "union", "flip", "toon",
];
const RACES: Race[] = [
  "warrior", "spellcaster", "dragon", "zombie", "machine", "aqua", "pyro", "rock",
  "winged_beast", "plant", "insect", "thunder", "fish", "sea_serpent", "reptile",
  "psychic", "divine_beast", "beast", "beast_warrior", "dinosaur", "fairy", "fiend",
  "illusion", "cyberse", "creator_god", "wyrm",
];
const ATTRIBUTES: Attribute[] = ["light", "dark", "earth", "water", "fire", "wind", "divine"];
const SPELL_SUBTYPES: SpellSubtype[] = [
  "normal", "continuous", "quick_play", "ritual", "field", "equip",
];
const TRAP_SUBTYPES: TrapSubtype[] = ["normal", "continuous", "counter"];
const LINK_MARKERS: LinkMarker[] = [
  "top", "bottom", "left", "right",
  "top_left", "top_right", "bottom_left", "bottom_right",
];

/** Validate a scalar enum value the model supplied; throws ToolError on a bad value. */
function asEnum<T extends string>(field: string, value: unknown, allowed: T[]): T {
  if (typeof value !== "string" || !(allowed as string[]).includes(value)) {
    throw new ToolError(
      `Invalid ${field}: ${JSON.stringify(value)}. Allowed values: ${allowed.join(", ")}.`,
    );
  }
  return value as T;
}

/** Validate an array of enum values; throws ToolError on any bad element. */
function asEnumArray<T extends string>(field: string, value: unknown, allowed: T[]): T[] {
  if (!Array.isArray(value)) {
    throw new ToolError(`Invalid ${field}: expected an array.`);
  }
  return value.map((v) => asEnum(field, v, allowed));
}

/** Patch name/desc for a given language (defaults to the card's first language). */
function applyTextPatch(input: CardInput, args: Record<string, unknown>): CardInput {
  if (typeof args.name !== "string" && typeof args.desc !== "string") return input;
  const langs = Object.keys(input.texts);
  const lang = typeof args.language === "string" ? args.language : langs[0];
  if (!lang) {
    throw new ToolError("Card has no text language to update; specify a language.");
  }
  const existing = input.texts[lang] ?? { name: "", desc: "", strings: [] };
  return {
    ...input,
    texts: {
      ...input.texts,
      [lang]: {
        ...existing,
        name: typeof args.name === "string" ? args.name : existing.name,
        desc: typeof args.desc === "string" ? args.desc : existing.desc,
      },
    },
  };
}

/** Apply every supplied field onto the card input, validating enums. Unsupplied fields are left untouched. */
function applyPatch(input: CardInput, args: Record<string, unknown>): CardInput {
  const next: CardInput = { ...input };

  // Numeric scalars (atk/def/level are nullable; null clears them for spells/traps).
  if (typeof args.code === "number") next.code = args.code;
  if (typeof args.alias === "number") next.alias = args.alias;
  if (typeof args.category === "number") next.category = args.category;
  if (typeof args.atk === "number" || args.atk === null) next.atk = args.atk as number | null;
  if (typeof args.def === "number" || args.def === null) next.def = args.def as number | null;
  if (typeof args.level === "number" || args.level === null) {
    next.level = args.level as number | null;
  }

  // Enum scalars.
  if (args.primary_type !== undefined) {
    next.primary_type = asEnum("primary_type", args.primary_type, PRIMARY_TYPES);
  }
  if (args.ot !== undefined) next.ot = asEnum("ot", args.ot, OTS);
  if (args.race !== undefined) {
    next.race = args.race === null ? null : asEnum("race", args.race, RACES);
  }
  if (args.attribute !== undefined) {
    next.attribute = args.attribute === null ? null : asEnum("attribute", args.attribute, ATTRIBUTES);
  }
  if (args.spell_subtype !== undefined) {
    next.spell_subtype =
      args.spell_subtype === null ? null : asEnum("spell_subtype", args.spell_subtype, SPELL_SUBTYPES);
  }
  if (args.trap_subtype !== undefined) {
    next.trap_subtype =
      args.trap_subtype === null ? null : asEnum("trap_subtype", args.trap_subtype, TRAP_SUBTYPES);
  }

  // Enum / numeric arrays.
  if (args.monster_flags !== undefined) {
    next.monster_flags =
      args.monster_flags === null ? null : asEnumArray("monster_flags", args.monster_flags, MONSTER_FLAGS);
  }
  if (args.setcodes !== undefined) {
    if (!Array.isArray(args.setcodes) || args.setcodes.some((s) => typeof s !== "number")) {
      throw new ToolError("Invalid setcodes: expected an array of integers.");
    }
    next.setcodes = args.setcodes as number[];
  }

  // Nested structures.
  if (args.pendulum !== undefined) {
    if (args.pendulum === null) {
      next.pendulum = null;
    } else {
      const p = args.pendulum as Record<string, unknown>;
      if (typeof p.left_scale !== "number" || typeof p.right_scale !== "number") {
        throw new ToolError("Invalid pendulum: expected { left_scale: number, right_scale: number }.");
      }
      next.pendulum = { left_scale: p.left_scale, right_scale: p.right_scale };
    }
  }
  if (args.link !== undefined) {
    if (args.link === null) {
      next.link = null;
    } else {
      const l = args.link as Record<string, unknown>;
      next.link = { markers: asEnumArray("link.markers", l.markers, LINK_MARKERS) };
    }
  }

  return next;
}

export const updateCardTool: AgentTool = {
  name: "update_card",
  description:
    "Update any editable field of an existing card in the active pack. Call this when the user " +
    "asks to change, edit, or modify a card — including its name/description, ATK/DEF/level, " +
    "card type (monster/spell/trap), monster race and attribute, monster abilities " +
    "(normal/effect/fusion/synchro/xyz/link/pendulum/tuner/etc.), spell/trap subtype, " +
    "pendulum scales, link markers, setcodes, ot, alias, category, or code. " +
    "Get the card's id and current values from list_cards / get_card first. Only the fields " +
    "you pass are changed; the rest are preserved. When changing the card type you usually " +
    "also need to set the fields that type requires (e.g. spell_subtype for a spell).",
  readOnly: false,
  parameters: {
    type: "object",
    properties: {
      cardId: { type: "string", description: "The card id to update (from list_cards)." },
      name: { type: "string", description: "New card name." },
      desc: { type: "string", description: "New card description/effect text." },
      language: {
        type: "string",
        description:
          "Language code for name/desc changes (e.g. 'zh-CN'). Defaults to the card's first language.",
      },
      atk: { type: ["integer", "null"], description: "New ATK value. null clears it (non-monsters)." },
      def: { type: ["integer", "null"], description: "New DEF value. null clears it (non-monsters)." },
      level: {
        type: ["integer", "null"],
        description: "New level/rank/link rating. null clears it (non-monsters).",
      },
      primary_type: {
        type: "string",
        enum: PRIMARY_TYPES,
        description: "Card kind: monster, spell, or trap.",
      },
      race: {
        type: ["string", "null"],
        enum: [...RACES, null],
        description: "Monster race/type (e.g. dragon, spellcaster, warrior). null for non-monsters.",
      },
      attribute: {
        type: ["string", "null"],
        enum: [...ATTRIBUTES, null],
        description: "Monster attribute (light/dark/earth/water/fire/wind/divine). null for non-monsters.",
      },
      monster_flags: {
        type: ["array", "null"],
        items: { type: "string", enum: MONSTER_FLAGS },
        description:
          "Monster abilities/summon types, e.g. ['normal'], ['effect','tuner'], ['synchro']. null for non-monsters.",
      },
      spell_subtype: {
        type: ["string", "null"],
        enum: [...SPELL_SUBTYPES, null],
        description: "Spell subtype (normal/continuous/quick_play/ritual/field/equip). Spells only.",
      },
      trap_subtype: {
        type: ["string", "null"],
        enum: [...TRAP_SUBTYPES, null],
        description: "Trap subtype (normal/continuous/counter). Traps only.",
      },
      pendulum: {
        type: ["object", "null"],
        properties: {
          left_scale: { type: "integer" },
          right_scale: { type: "integer" },
        },
        required: ["left_scale", "right_scale"],
        description: "Pendulum scales. null to clear pendulum data.",
      },
      link: {
        type: ["object", "null"],
        properties: {
          markers: { type: "array", items: { type: "string", enum: LINK_MARKERS } },
        },
        required: ["markers"],
        description: "Link arrow markers. null to clear link data.",
      },
      setcodes: {
        type: "array",
        items: { type: "integer" },
        description: "Archetype/set codes as integers.",
      },
      ot: { type: "string", enum: OTS, description: "Origin/format scope: ocg, tcg, or custom." },
      alias: { type: "integer", description: "Alias code (alternate-art original code), 0 if none." },
      category: { type: "integer", description: "Category bitmask integer." },
      code: { type: "integer", description: "Card passcode. Usually auto-assigned; change with care." },
    },
    required: ["cardId"],
  },
  async execute(args, ctx): Promise<WriteResult<unknown>> {
    const { workspaceId, packId } = requirePack(ctx);
    const cardId = String(args.cardId);
    const detail = await cardApi.getCard({ workspaceId, packId, cardId });
    let input = toCardInput(detail.card);
    input = applyPatch(input, args);
    input = applyTextPatch(input, args);
    return cardApi.updateCard({ workspaceId, packId, cardId, card: input });
  },
};

const EMPTY_STRINGS = Array.from({ length: 16 }, () => "");

/** Minimal valid blank monster card, mirroring CardEditDrawer's makeBlankCard. */
function makeBlankCardInput(code: number, lang: string, name: string): CardInput {
  return {
    code,
    alias: 0,
    setcodes: [],
    ot: "custom",
    category: 0,
    primary_type: "monster",
    texts: { [lang]: { name, desc: "", strings: [...EMPTY_STRINGS] } },
    monster_flags: ["normal"],
    atk: 0,
    def: 0,
    race: "warrior",
    attribute: "earth",
    level: 4,
    pendulum: null,
    link: null,
    spell_subtype: null,
    trap_subtype: null,
  };
}

export const createCardTool: AgentTool = {
  name: "create_card",
  description:
    "Create a new card in the active pack. Call this when the user asks to add, create, " +
    "or make a new card. A card code is auto-assigned. The card starts as a basic Normal " +
    "monster; pass name/atk/def/level to set those. The user can refine other fields later in the UI.",
  readOnly: false,
  parameters: {
    type: "object",
    properties: {
      name: { type: "string", description: "The card name." },
      language: {
        type: "string",
        description: "Language code for the name (e.g. 'zh-CN'). Defaults to 'en-US'.",
      },
      atk: { type: "integer", description: "ATK value. Defaults to 0." },
      def: { type: "integer", description: "DEF value. Defaults to 0." },
      level: { type: "integer", description: "Level. Defaults to 4." },
    },
    required: ["name"],
  },
  async execute(args, ctx): Promise<WriteResult<unknown>> {
    const { workspaceId, packId } = requirePack(ctx);
    const lang = typeof args.language === "string" ? args.language : "en-US";
    const suggestion = await cardApi.suggestCardCode({
      workspaceId,
      packId,
      preferredStart: null,
    });
    const card = makeBlankCardInput(
      suggestion.suggested_code ?? 100000000,
      lang,
      String(args.name),
    );
    if (typeof args.atk === "number") card.atk = args.atk;
    if (typeof args.def === "number") card.def = args.def;
    if (typeof args.level === "number") card.level = args.level;
    return cardApi.createCard({ workspaceId, packId, card });
  },
};

export const moveCardsTool: AgentTool = {
  name: "move_cards",
  description:
    "Move one or more cards from the active pack to another pack. Call this when the user " +
    "asks to move or transfer cards between packs. Get card ids from list_cards first.",
  readOnly: false,
  parameters: {
    type: "object",
    properties: {
      targetPackId: { type: "string", description: "The destination pack id." },
      cardIds: {
        type: "array",
        items: { type: "string" },
        description: "Ids of the cards to move (from list_cards).",
      },
      moveAssets: {
        type: "boolean",
        description: "Whether to move card assets (images/scripts) too. Defaults to true.",
      },
    },
    required: ["targetPackId", "cardIds"],
  },
  async execute(args, ctx): Promise<WriteResult<unknown>> {
    const { workspaceId, packId } = requirePack(ctx);
    if (!Array.isArray(args.cardIds) || args.cardIds.length === 0) {
      throw new ToolError("cardIds must be a non-empty array of card ids.");
    }
    return cardApi.moveCards({
      workspaceId,
      sourcePackId: packId,
      targetPackId: String(args.targetPackId),
      cardIds: args.cardIds.map(String),
      moveAssets: args.moveAssets !== false,
    });
  },
};

/** Resolve the pack's primary display language, used for setname strings. */
function packDisplayLanguage(packId: string): string {
  const meta = useShellStore.getState().packMetadataMap[packId];
  return meta?.display_language_order?.[0] ?? "en-US";
}

export const createSetnameTool: AgentTool = {
  name: "create_setname",
  description:
    "Create (or rename) a custom series/archetype name in the active pack, so it can be " +
    "assigned to cards via update_card's setcodes field. Call this when the user wants a " +
    "NEW series that does not yet exist — check list_setnames first to avoid duplicates. " +
    "A setcode key is a 16-bit value: the low 12 bits are the base (the series), the high " +
    "4 bits are a child index used for sub-archetypes (0 for a normal top-level series). " +
    "If you omit key, the backend allocates the next free top-level base (child=0) within " +
    "the recommended range from config, avoiding conflicts with this pack, other packs, and " +
    "the standard reference. Pass key (hex, e.g. '0x1234') to choose a specific value or to " +
    "construct a sub-archetype; writing a name for an existing key renames that series. " +
    "After creating, use update_card to add the returned key to a card's setcodes.",
  readOnly: false,
  // Setname writes go through the pack-strings confirmation gate, not the card one.
  confirmWrite: (confirmationToken) =>
    stringsApi.confirmPackStringsWrite({ confirmationToken }),
  // Refresh the strings browser and the setname maps used by editors and list_setnames.
  invalidateKeys: [["strings"], ["pack-setnames"], ["standard-setnames"]],
  parameters: {
    type: "object",
    properties: {
      name: { type: "string", description: "The series/archetype name to set." },
      key: {
        type: "string",
        description:
          "Optional hex setcode key (e.g. '0x1234'). Omit to auto-allocate the next free key.",
      },
      language: {
        type: "string",
        description:
          "Language code for the name (e.g. 'zh-CN'). Defaults to the pack's primary display language.",
      },
    },
    required: ["name"],
  },
  async execute(args, ctx): Promise<WriteResult<unknown>> {
    const { workspaceId, packId } = requirePack(ctx);
    const name = typeof args.name === "string" ? args.name.trim() : "";
    if (!name) {
      throw new ToolError("name is required and must be a non-empty string.");
    }
    const language =
      typeof args.language === "string" && args.language
        ? args.language
        : packDisplayLanguage(packId);

    const key = await resolveSetnameKey(workspaceId, packId, args.key);

    return stringsApi.upsertPackString({
      workspaceId,
      packId,
      language,
      entry: { kind: "setname", key, value: name },
    });
  },
};

/**
 * Resolve the setcode key to write. If the model supplied one, parse it as hex
 * and use it (create-or-rename at that exact 16-bit value, including any child
 * half-byte for sub-archetypes). Otherwise ask the backend to allocate the next
 * free top-level base within the config recommended range.
 */
async function resolveSetnameKey(
  workspaceId: string,
  packId: string,
  rawKey: unknown,
): Promise<number> {
  if (typeof rawKey === "string" && rawKey.trim()) {
    const parsed = parseHexInput(rawKey);
    if (Number.isNaN(parsed) || parsed < 0) {
      throw new ToolError(
        `Invalid setcode key: ${JSON.stringify(rawKey)}. Use a hex value like '0x1234'.`,
      );
    }
    return parsed;
  }

  const suggestion = await stringsApi.suggestSetnameKey({ workspaceId, packId });
  if (suggestion.suggested_key === null) {
    throw new ToolError(
      "The recommended setname base range is full; no free key could be allocated. " +
        "Ask the user to widen the range in settings, or pass an explicit hex key.",
    );
  }
  return suggestion.suggested_key;
}
