import { scriptApi } from "../../../shared/api/scriptApi";
import type { LuaValidationLevel } from "../../../shared/contracts/script";
import type { AgentTool } from "./types";
import { requirePack, ToolError } from "./types";

const VALID_LEVELS = new Set<LuaValidationLevel>([
  "static",
  "ocgcore_init",
  "smoke",
  "scenario",
]);

function normalizeCardId(value: unknown): string | null {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function normalizeLevels(value: unknown): LuaValidationLevel[] | undefined {
  if (value === undefined) return undefined;
  if (!Array.isArray(value)) {
    throw new ToolError("levels must be an array of validation level strings.");
  }

  const levels = value.map((item) => String(item));
  const invalid = levels.find((level) => !VALID_LEVELS.has(level as LuaValidationLevel));
  if (invalid) {
    throw new ToolError(
      `Unsupported validation level "${invalid}". Use static, ocgcore_init, smoke, or scenario.`,
    );
  }

  return levels as LuaValidationLevel[];
}

export const validateLuaScriptTool: AgentTool = {
  name: "validate_lua_script",
  description:
    "Validate a custom card's saved Lua script in the active pack. This read-only tool " +
    "returns the backend LuaValidationReport with static and ocgcore_init results by default. " +
    "It checks script load/init only and does not edit or fix scripts.",
  readOnly: true,
  parameters: {
    type: "object",
    properties: {
      cardId: {
        type: "string",
        description:
          "Optional card id. Omit to validate the current Selected card from the editor.",
      },
      levels: {
        type: "array",
        items: {
          type: "string",
          enum: ["static", "ocgcore_init", "smoke", "scenario"],
        },
        description:
          "Optional validation levels. Omit to use the backend default static + ocgcore_init.",
      },
    },
  },
  async execute(args, ctx) {
    const { workspaceId, packId } = requirePack(ctx);
    const cardId = normalizeCardId(args.cardId) ?? ctx.selectedCardId;
    if (!cardId) {
      throw new ToolError(
        "Specify a card id or open a card in the editor before validating its Lua script.",
      );
    }

    const levels = normalizeLevels(args.levels);
    return scriptApi.validateLuaScript({
      workspaceId,
      packId,
      cardId,
      ...(levels ? { levels } : {}),
    });
  },
};
