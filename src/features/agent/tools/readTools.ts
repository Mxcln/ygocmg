import { cardApi } from "../../../shared/api/cardApi";
import { standardPackApi } from "../../../shared/api/standardPackApi";
import type { AgentTool } from "./types";
import { requirePack } from "./types";

const DEFAULT_PAGE_SIZE = 50;

export const listCardsTool: AgentTool = {
  name: "list_cards",
  description:
    "List cards in the currently active pack. Call this when the user asks to list, " +
    "show, browse, or find cards in the current pack, or when you need a card's id " +
    "before updating or moving it. Returns a page of summary rows (id, code, name, " +
    "type, atk, def, level).",
  readOnly: true,
  parameters: {
    type: "object",
    properties: {
      keyword: {
        type: "string",
        description: "Optional case-insensitive substring to match against card name/desc.",
      },
      page: { type: "integer", description: "1-based page number. Defaults to 1." },
      pageSize: {
        type: "integer",
        description: `Rows per page (max ${DEFAULT_PAGE_SIZE}). Defaults to ${DEFAULT_PAGE_SIZE}.`,
      },
    },
  },
  async execute(args, ctx) {
    const { workspaceId, packId } = requirePack(ctx);
    const pageSize = Math.min(
      typeof args.pageSize === "number" ? args.pageSize : DEFAULT_PAGE_SIZE,
      DEFAULT_PAGE_SIZE,
    );
    const page = await cardApi.listCards({
      workspaceId,
      packId,
      keyword: typeof args.keyword === "string" ? args.keyword : null,
      sortBy: "code",
      sortDirection: "asc",
      page: typeof args.page === "number" ? args.page : 1,
      pageSize,
    });
    return {
      items: page.items,
      page: page.page,
      page_size: page.page_size,
      total: page.total,
    };
  },
};

export const getCardTool: AgentTool = {
  name: "get_card",
  description:
    "Get the full detail of a single card in the active pack by its id. Call this when " +
    "you need a card's complete fields (e.g. before updating it). Get the id from list_cards first.",
  readOnly: true,
  parameters: {
    type: "object",
    properties: {
      cardId: { type: "string", description: "The card id (from list_cards)." },
    },
    required: ["cardId"],
  },
  async execute(args, ctx) {
    const { workspaceId, packId } = requirePack(ctx);
    return cardApi.getCard({ workspaceId, packId, cardId: String(args.cardId) });
  },
};

export const searchStandardCardsTool: AgentTool = {
  name: "search_standard_cards",
  description:
    "Search the read-only standard (official) card reference database by keyword. Call " +
    "this when the user asks about official cards or wants reference data, NOT for cards " +
    "in their own pack (use list_cards for those).",
  readOnly: true,
  parameters: {
    type: "object",
    properties: {
      keyword: { type: "string", description: "Search keyword (name substring)." },
      page: { type: "integer", description: "1-based page number. Defaults to 1." },
      pageSize: { type: "integer", description: "Rows per page (max 50). Defaults to 50." },
    },
    required: ["keyword"],
  },
  async execute(args) {
    const pageSize = Math.min(
      typeof args.pageSize === "number" ? args.pageSize : DEFAULT_PAGE_SIZE,
      DEFAULT_PAGE_SIZE,
    );
    const page = await standardPackApi.searchCards({
      keyword: typeof args.keyword === "string" ? args.keyword : null,
      filters: null,
      sortBy: "code",
      sortDirection: "asc",
      page: typeof args.page === "number" ? args.page : 1,
      pageSize,
    });
    return {
      items: page.items,
      page: page.page,
      page_size: page.page_size,
      total: page.total,
    };
  },
};
