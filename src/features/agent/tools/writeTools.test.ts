import { beforeEach, describe, expect, it, vi } from "vitest";
import { cardApi } from "../../../shared/api/cardApi";
import type {
  BulkDeleteCardsResult,
  CardBatchWriteResult,
  WriteResult,
} from "../../../shared/contracts/card";
import { deleteCardsTool } from "./writeTools";
import type { ToolContext } from "./types";

vi.mock("../../../shared/api/cardApi", () => ({
  cardApi: {
    bulkDeleteCards: vi.fn(),
    confirmCardBatchWrite: vi.fn(),
  },
}));

const ctx: ToolContext = {
  workspaceId: "workspace-1",
  packId: "pack-1",
};

beforeEach(() => {
  vi.clearAllMocks();
});

describe("deleteCardsTool", () => {
  it("rejects missing cardIds without calling the API", async () => {
    await expect(deleteCardsTool.execute({}, ctx)).rejects.toThrow(
      "cardIds must be a non-empty array of card ids.",
    );

    expect(cardApi.bulkDeleteCards).not.toHaveBeenCalled();
  });

  it("rejects an empty cardIds array without calling the API", async () => {
    await expect(deleteCardsTool.execute({ cardIds: [] }, ctx)).rejects.toThrow(
      "cardIds must be a non-empty array of card ids.",
    );

    expect(cardApi.bulkDeleteCards).not.toHaveBeenCalled();
  });

  it("calls bulkDeleteCards with active workspace and pack, stringified ids, and deleteAssets true by default", async () => {
    const result = {
      status: "ok",
      data: { deleted_card_ids: ["card-1", "42"], deleted_asset_count: 3 },
      warnings: [],
    } satisfies WriteResult<BulkDeleteCardsResult>;
    vi.mocked(cardApi.bulkDeleteCards).mockResolvedValue(result);

    await expect(
      deleteCardsTool.execute({ cardIds: ["card-1", 42] }, ctx),
    ).resolves.toBe(result);

    expect(cardApi.bulkDeleteCards).toHaveBeenCalledWith({
      workspaceId: "workspace-1",
      packId: "pack-1",
      cardIds: ["card-1", "42"],
      deleteAssets: true,
    });
  });

  it("passes deleteAssets false when explicitly requested", async () => {
    const result = {
      status: "ok",
      data: { deleted_card_ids: ["card-1"], deleted_asset_count: 0 },
      warnings: [],
    } satisfies WriteResult<BulkDeleteCardsResult>;
    vi.mocked(cardApi.bulkDeleteCards).mockResolvedValue(result);

    await deleteCardsTool.execute({ cardIds: ["card-1"], deleteAssets: false }, ctx);

    expect(cardApi.bulkDeleteCards).toHaveBeenCalledWith({
      workspaceId: "workspace-1",
      packId: "pack-1",
      cardIds: ["card-1"],
      deleteAssets: false,
    });
  });

  it("confirms batch writes through confirmCardBatchWrite", async () => {
    const confirmed = {
      operation: "bulk_delete",
      data: { deleted_card_ids: ["card-1"], deleted_asset_count: 1 },
    } satisfies CardBatchWriteResult;
    vi.mocked(cardApi.confirmCardBatchWrite).mockResolvedValue(confirmed);

    await expect(deleteCardsTool.confirmWrite?.("token-1")).resolves.toBe(confirmed);

    expect(cardApi.confirmCardBatchWrite).toHaveBeenCalledWith({
      confirmationToken: "token-1",
    });
  });
});
