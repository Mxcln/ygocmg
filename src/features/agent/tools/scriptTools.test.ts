import { beforeEach, describe, expect, it, vi } from "vitest";
import { scriptApi } from "../../../shared/api/scriptApi";
import type { LuaValidationReport } from "../../../shared/contracts/script";
import { AGENT_TOOLS, TOOL_DEFINITIONS } from "./registry";
import { validateLuaScriptTool } from "./scriptTools";
import type { ToolContext } from "./types";

vi.mock("../../../shared/api/scriptApi", () => ({
  scriptApi: {
    validateLuaScript: vi.fn(),
  },
}));

const baseCtx: ToolContext = {
  workspaceId: "workspace-1",
  packId: "pack-1",
  selectedCardId: "selected-card",
};

const report: LuaValidationReport = {
  status: "pass",
  confidence: "high",
  summary: "Script loaded.",
  issues: [],
  stages: [],
  limitations: [],
};

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(scriptApi.validateLuaScript).mockResolvedValue(report);
});

describe("validateLuaScriptTool", () => {
  it("rejects missing active pack before calling the API", async () => {
    await expect(
      validateLuaScriptTool.execute(
        {},
        { workspaceId: null, packId: null, selectedCardId: null },
      ),
    ).rejects.toThrow("No active pack");

    expect(scriptApi.validateLuaScript).not.toHaveBeenCalled();
  });

  it("requires a card id when no selected card is available", async () => {
    await expect(
      validateLuaScriptTool.execute({}, { ...baseCtx, selectedCardId: null }),
    ).rejects.toThrow("Specify a card id or open a card in the editor");

    expect(scriptApi.validateLuaScript).not.toHaveBeenCalled();
  });

  it("uses the selected card when cardId is omitted", async () => {
    await expect(validateLuaScriptTool.execute({}, baseCtx)).resolves.toBe(report);

    expect(scriptApi.validateLuaScript).toHaveBeenCalledWith({
      workspaceId: "workspace-1",
      packId: "pack-1",
      cardId: "selected-card",
    });
  });

  it("prefers an explicit cardId over the selected card", async () => {
    await validateLuaScriptTool.execute({ cardId: "explicit-card" }, baseCtx);

    expect(scriptApi.validateLuaScript).toHaveBeenCalledWith({
      workspaceId: "workspace-1",
      packId: "pack-1",
      cardId: "explicit-card",
    });
  });

  it("forwards valid explicit levels", async () => {
    await validateLuaScriptTool.execute(
      { cardId: "card-1", levels: ["static", "ocgcore_init"] },
      baseCtx,
    );

    expect(scriptApi.validateLuaScript).toHaveBeenCalledWith({
      workspaceId: "workspace-1",
      packId: "pack-1",
      cardId: "card-1",
      levels: ["static", "ocgcore_init"],
    });
  });

  it("rejects invalid levels before calling the API", async () => {
    await expect(
      validateLuaScriptTool.execute({ levels: ["static", "made_up"] }, baseCtx),
    ).rejects.toThrow("Unsupported validation level");

    expect(scriptApi.validateLuaScript).not.toHaveBeenCalled();
  });
});

describe("script validation tool registry", () => {
  it("registers validate_lua_script as an agent tool", () => {
    expect(AGENT_TOOLS.some((tool) => tool.name === "validate_lua_script")).toBe(true);
  });

  it("includes validate_lua_script in model tool definitions", () => {
    expect(
      TOOL_DEFINITIONS.some((definition) => definition.function.name === "validate_lua_script"),
    ).toBe(true);
  });
});
