import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeApi } from "./invoke";
import { scriptApi } from "./scriptApi";
import type { LuaValidationReport } from "../contracts/script";

vi.mock("./invoke", () => ({
  invokeApi: vi.fn(),
}));

beforeEach(() => {
  vi.clearAllMocks();
});

describe("scriptApi", () => {
  it("calls validate_lua_script with a typed input payload", async () => {
    const report = {
      status: "pass",
      confidence: "high",
      summary: "Static validation passed.",
      issues: [],
      stages: [
        {
          stage: "static",
          status: "pass",
          durationMs: 2,
          issues: [],
          log: [],
        },
      ],
      limitations: [
        "Static validation does not prove ocgcore load/init success or effect semantics.",
      ],
    } satisfies LuaValidationReport;
    vi.mocked(invokeApi).mockResolvedValue(report);

    await expect(
      scriptApi.validateLuaScript({
        workspaceId: "workspace-1",
        packId: "pack-1",
        cardId: "card-1",
        scriptText: "local s,id,o=GetID()\nfunction s.initial_effect(c)\nend",
        levels: ["static"],
      }),
    ).resolves.toBe(report);

    expect(invokeApi).toHaveBeenCalledWith("validate_lua_script", {
      input: {
        workspaceId: "workspace-1",
        packId: "pack-1",
        cardId: "card-1",
        scriptText: "local s,id,o=GetID()\nfunction s.initial_effect(c)\nend",
        levels: ["static"],
      },
    });
  });
});
