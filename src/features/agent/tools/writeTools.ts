import { cardApi } from "../../../shared/api/cardApi";
import type { CardEntity, WriteResult } from "../../../shared/contracts/card";
import type { AgentTool } from "./types";
import { requirePack, ToolError } from "./types";

/** Strip server-managed fields so the entity matches the create/update input shape. */
type CardInput = Omit<CardEntity, "id" | "created_at" | "updated_at">;

function toCardInput(card: CardEntity): CardInput {
  const { id: _id, created_at: _c, updated_at: _u, ...rest } = card;
  return rest;
}

/** Apply scalar patches the model supplied, leaving everything else untouched. */
function applyScalarPatch(input: CardInput, args: Record<string, unknown>): CardInput {
  const next: CardInput = { ...input };
  if (typeof args.atk === "number") next.atk = args.atk;
  if (typeof args.def === "number") next.def = args.def;
  if (typeof args.level === "number") next.level = args.level;
  return next;
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

export const updateCardTool: AgentTool = {
  name: "update_card",
  description:
    "Update fields of an existing card in the active pack. Call this when the user asks " +
    "to change, edit, or modify a card (e.g. its ATK/DEF/level, name, or description). " +
    "Get the card's id from list_cards first. Only the fields you pass are changed; " +
    "the rest are preserved.",
  readOnly: false,
  parameters: {
    type: "object",
    properties: {
      cardId: { type: "string", description: "The card id to update (from list_cards)." },
      atk: { type: "integer", description: "New ATK value." },
      def: { type: "integer", description: "New DEF value." },
      level: { type: "integer", description: "New level/rank/link value." },
      name: { type: "string", description: "New card name." },
      desc: { type: "string", description: "New card description/effect text." },
      language: {
        type: "string",
        description:
          "Language code for name/desc changes (e.g. 'zh-CN'). Defaults to the card's first language.",
      },
    },
    required: ["cardId"],
  },
  async execute(args, ctx): Promise<WriteResult<unknown>> {
    const { workspaceId, packId } = requirePack(ctx);
    const cardId = String(args.cardId);
    const detail = await cardApi.getCard({ workspaceId, packId, cardId });
    let input = toCardInput(detail.card);
    input = applyScalarPatch(input, args);
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
