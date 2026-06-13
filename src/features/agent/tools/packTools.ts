import { configApi } from "../../../shared/api/configApi";
import { useShellStore } from "../../../shared/stores/shellStore";
import { preferredAuthoringLanguage } from "../../../shared/utils/language";
import { buildAgentDeps } from "../../commands/buildAgentDeps";
import * as packCommands from "../../commands/packCommands";
import type { AgentTool } from "./types";
import { ToolError } from "./types";

// Pack-level operations derive their targets from args and live shell state via
// buildAgentDeps(); they intentionally do not use ToolContext's workspaceId/packId
// (those are card-level concerns), so `execute` omits the `ctx` parameter here.

export const switchPackTool: AgentTool = {
  name: "switch_pack",
  description:
    "Make an already-open pack the active pack. Get pack ids from list_packs. " +
    "Use open_pack for packs that are not yet open.",
  readOnly: false,
  parameters: {
    type: "object",
    properties: { packId: { type: "string", description: "The pack id to activate." } },
    required: ["packId"],
  },
  async execute(args) {
    return packCommands.switchPack(String(args.packId), buildAgentDeps());
  },
};

export const openPackTool: AgentTool = {
  name: "open_pack",
  description:
    "Open a pack in the current workspace (and make it active). Get pack ids from list_packs.",
  readOnly: false,
  parameters: {
    type: "object",
    properties: { packId: { type: "string", description: "The pack id to open." } },
    required: ["packId"],
  },
  async execute(args) {
    return packCommands.openPack(String(args.packId), buildAgentDeps());
  },
};

export const closePackTool: AgentTool = {
  name: "close_pack",
  description: "Close an open pack (does not delete it). Get pack ids from list_packs.",
  readOnly: false,
  parameters: {
    type: "object",
    properties: { packId: { type: "string", description: "The pack id to close." } },
    required: ["packId"],
  },
  async execute(args) {
    return packCommands.closePack(String(args.packId), buildAgentDeps());
  },
};

export const createPackTool: AgentTool = {
  name: "create_pack",
  description:
    "Create a new custom pack in the current workspace and open it. Requires name, author, version.",
  readOnly: false,
  parameters: {
    type: "object",
    properties: {
      name: { type: "string", description: "Pack name." },
      author: { type: "string", description: "Pack author." },
      version: { type: "string", description: "Version string, e.g. '1.0.0'." },
      packCode: { type: "string", description: "Optional pack code." },
      description: { type: "string", description: "Optional description." },
    },
    required: ["name", "author", "version"],
  },
  async execute(args) {
    const config = await configApi.loadConfig();
    const lang = preferredAuthoringLanguage(config);
    return packCommands.createPack(
      {
        name: String(args.name),
        author: String(args.author),
        version: String(args.version),
        packCode: typeof args.packCode === "string" ? args.packCode : null,
        description: typeof args.description === "string" ? args.description : null,
        displayLanguageOrder: [lang],
        defaultExportLanguage: lang,
      },
      buildAgentDeps(),
    );
  },
};

export const updatePackMetaTool: AgentTool = {
  name: "update_pack_meta",
  description:
    "Update an open pack's metadata. Only the fields you pass change; others are preserved. " +
    "Get the pack's current values from get_pack_info first. Defaults to the active pack if packId omitted.",
  readOnly: false,
  parameters: {
    type: "object",
    properties: {
      packId: { type: "string", description: "Pack id. Omit for the active pack." },
      name: { type: "string", description: "New pack name." },
      author: { type: "string", description: "New author." },
      version: { type: "string", description: "New version." },
      packCode: { type: "string", description: "New pack code." },
      description: { type: "string", description: "New description." },
    },
  },
  async execute(args) {
    const shell = useShellStore.getState();
    const packId =
      typeof args.packId === "string" && args.packId ? args.packId : shell.activePackId;
    if (!packId) throw new ToolError("No active pack. Open a pack first.");
    const current = shell.packMetadataMap[packId];
    if (!current) throw new ToolError(`Pack ${packId} is not open. Open it first.`);
    return packCommands.updatePackMeta(
      {
        packId,
        name: typeof args.name === "string" ? args.name : current.name,
        author: typeof args.author === "string" ? args.author : current.author,
        version: typeof args.version === "string" ? args.version : current.version,
        packCode: typeof args.packCode === "string" ? args.packCode : current.pack_code,
        description: typeof args.description === "string" ? args.description : current.description,
        displayLanguageOrder: current.display_language_order,
        defaultExportLanguage: current.default_export_language,
      },
      buildAgentDeps(),
    );
  },
};

export const deletePackTool: AgentTool = {
  name: "delete_pack",
  description:
    "Delete a pack permanently. This is destructive and asks the user to confirm before deleting. " +
    "Get pack ids from list_packs.",
  readOnly: false,
  parameters: {
    type: "object",
    properties: { packId: { type: "string", description: "The pack id to delete." } },
    required: ["packId"],
  },
  async execute(args) {
    const packId = String(args.packId);
    const shell = useShellStore.getState();
    const name = shell.packMetadataMap[packId]?.name ?? packId;
    return packCommands.deletePack(packId, name, buildAgentDeps());
  },
};
