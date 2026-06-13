import { cardApi } from "../../../shared/api/cardApi";
import { standardPackApi } from "../../../shared/api/standardPackApi";
import { packApi } from "../../../shared/api/packApi";
import { configApi } from "../../../shared/api/configApi";
import { stringsApi } from "../../../shared/api/stringsApi";
import { useShellStore } from "../../../shared/stores/shellStore";
import { mergeSetnameEntries } from "../../card/setnameEntries";
import type { AgentTool } from "./types";
import { requirePack, ToolError } from "./types";

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

export const getConfigTool: AgentTool = {
  name: "get_config",
  description:
    "Get the user's business-relevant global settings: custom card code recommended " +
    "range and minimum gap, custom setname base recommended range, text language catalog, " +
    "standard-pack source language, app language, and agent reply language. Call this when " +
    "you need to know numbering rules or language configuration. Never includes secrets.",
  readOnly: true,
  parameters: { type: "object", properties: {} },
  async execute() {
    const config = await configApi.loadConfig();
    // Explicitly pick business fields — never expose deepseek_api_key.
    return {
      custom_code_recommended_min: config.custom_code_recommended_min,
      custom_code_recommended_max: config.custom_code_recommended_max,
      custom_code_min_gap: config.custom_code_min_gap,
      setname_base_recommended_min: config.setname_base_recommended_min,
      setname_base_recommended_max: config.setname_base_recommended_max,
      app_language: config.app_language,
      agent_language: config.agent_language,
      standard_pack_source_language: config.standard_pack_source_language,
      text_language_catalog: config.text_language_catalog.map((lang) => ({
        id: lang.id,
        label: lang.label,
        kind: lang.kind,
        hidden: lang.hidden,
      })),
    };
  },
};

export const getPackInfoTool: AgentTool = {
  name: "get_pack_info",
  description:
    "Get full metadata of an OPEN custom pack: pack_code, author, version, description, " +
    "display language order, default export language. Omit packId for the currently active " +
    "pack. For packs that are not open, use list_packs (only lighter overview is available).",
  readOnly: true,
  parameters: {
    type: "object",
    properties: {
      packId: {
        type: "string",
        description: "Optional pack id. Omit to use the currently active pack.",
      },
    },
  },
  async execute(args) {
    const shell = useShellStore.getState();
    const packId =
      typeof args.packId === "string" && args.packId ? args.packId : shell.activePackId;
    if (!packId) {
      throw new ToolError("No active pack. Ask the user to open a pack first.");
    }
    const metadata = shell.packMetadataMap[packId];
    if (!metadata) {
      throw new ToolError(
        `Pack ${packId} is not open. Use list_packs to see all packs, or ask the user to open it.`,
      );
    }
    return metadata;
  },
};

export const listPacksTool: AgentTool = {
  name: "list_packs",
  description:
    "List all packs in the current workspace (including packs that are not open). Returns " +
    "overview rows (id, name, kind, card count, etc.). Standard packs are read-only reference " +
    "and cannot be edited.",
  readOnly: true,
  parameters: { type: "object", properties: {} },
  async execute() {
    return packApi.listPackOverviews();
  },
};

export const suggestCardCodeTool: AgentTool = {
  name: "suggest_card_code",
  description:
    "Suggest the next available custom card code for the active pack, following the user's " +
    "numbering policy. Call this before creating a card when you need a code.",
  readOnly: true,
  parameters: { type: "object", properties: {} },
  async execute(_args, ctx) {
    const { workspaceId, packId } = requirePack(ctx);
    return cardApi.suggestCardCode({ workspaceId, packId, preferredStart: null });
  },
};

export const listSetnamesTool: AgentTool = {
  name: "list_setnames",
  description:
    "List archetype/series (setname) entries available for the active pack, each as a " +
    "{ key, name, source } row where key is the numeric setcode written onto cards' " +
    "`setcodes` field, name is the human-readable series name, and source is 'pack' " +
    "(defined in this pack) or 'standard' (from the official reference). Call this to " +
    "translate between series names and setcodes — e.g. to find which key to put in a " +
    "card's setcodes when the user names a series, or to explain a card's existing " +
    "setcodes by name. Pack entries take precedence over standard ones with the same key.",
  readOnly: true,
  parameters: {
    type: "object",
    properties: {
      keyword: {
        type: "string",
        description: "Optional case-insensitive substring to filter series by name.",
      },
    },
  },
  async execute(args, ctx) {
    const { workspaceId, packId } = requirePack(ctx);
    const shell = useShellStore.getState();
    const metadata = shell.packMetadataMap[packId];
    const packLang = metadata?.display_language_order?.[0] ?? "en-US";

    const config = await configApi.loadConfig();
    const standardLang = config.standard_pack_source_language ?? null;

    const [packStrings, standardSetnames] = await Promise.all([
      stringsApi.listPackStrings({
        workspaceId,
        packId,
        language: packLang,
        kindFilter: "setname",
        keyFilter: null,
        keyword: null,
        page: 1,
        pageSize: 10000,
      }),
      standardLang
        ? standardPackApi.listSetnames({ language: standardLang })
        : Promise.resolve([]),
    ]);

    const entries = mergeSetnameEntries(
      packStrings.items.map((item) => ({
        key: item.key,
        name: item.value,
        source: "pack" as const,
      })),
      standardSetnames.map((item) => ({
        key: item.key,
        name: item.value,
        source: "standard" as const,
      })),
    );

    const keyword = typeof args.keyword === "string" ? args.keyword.toLowerCase() : null;
    const filtered = keyword
      ? entries.filter((e) => e.name.toLowerCase().includes(keyword))
      : entries;

    return { items: filtered, total: filtered.length };
  },
};
