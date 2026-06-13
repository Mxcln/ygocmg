import { describe, it, expect, vi, beforeEach } from "vitest";
import { switchPack, openPack, createPack, closePack, updatePackMeta } from "./packCommands";
import type { CommandDeps } from "./types";
import type { PackMetadata } from "../../shared/contracts/pack";
import { packApi } from "../../shared/api/packApi";

vi.mock("../../shared/api/packApi", () => ({
  packApi: {
    setActivePack: vi.fn().mockResolvedValue(undefined),
    openPack: vi.fn(),
    createPack: vi.fn(),
    closePack: vi.fn().mockResolvedValue(undefined),
    updatePackMetadata: vi.fn(),
    deletePack: vi.fn().mockResolvedValue(undefined),
    listPackOverviews: vi.fn().mockResolvedValue([]),
  },
}));

function meta(id: string): PackMetadata {
  return {
    id, kind: "custom", name: `Pack ${id}`, pack_code: null, author: "a",
    version: "1.0.0", description: null, created_at: "", updated_at: "",
    display_language_order: ["en-US"], default_export_language: "en-US",
  };
}

function makeDeps() {
  const shell = {
    setActivePack: vi.fn(), addOpenPack: vi.fn(), removeOpenPack: vi.fn(),
    updatePackMetadata: vi.fn(), setPackOverviews: vi.fn(),
  };
  const queryClient = { invalidateQueries: vi.fn() } as any;
  const deps: CommandDeps = { shell, queryClient };
  return { deps, shell, queryClient };
}

beforeEach(() => vi.clearAllMocks());

describe("switchPack", () => {
  it("updates store, calls backend, invalidates cards", async () => {
    const { deps, shell, queryClient } = makeDeps();
    const res = await switchPack("p1", deps);
    expect(shell.setActivePack).toHaveBeenCalledWith("p1");
    expect(packApi.setActivePack).toHaveBeenCalledWith({ packId: "p1" });
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({ queryKey: ["cards"] });
    expect(res).toEqual({ status: "ok", data: undefined });
  });
});

describe("openPack", () => {
  it("opens pack, adds to store, refreshes overviews", async () => {
    const { deps, shell } = makeDeps();
    (packApi.openPack as any).mockResolvedValue(meta("p2"));
    const res = await openPack("p2", deps);
    expect(packApi.openPack).toHaveBeenCalledWith({ packId: "p2" });
    expect(shell.addOpenPack).toHaveBeenCalledWith("p2", meta("p2"));
    expect(shell.setPackOverviews).toHaveBeenCalled();
    expect(res.status).toBe("ok");
  });
});

describe("createPack", () => {
  it("creates then opens the pack, adds to store", async () => {
    const { deps, shell } = makeDeps();
    (packApi.createPack as any).mockResolvedValue(meta("p3"));
    (packApi.openPack as any).mockResolvedValue(meta("p3"));
    const res = await createPack(
      { name: "P3", packCode: null, author: "a", version: "1.0.0",
        description: null, displayLanguageOrder: ["en-US"], defaultExportLanguage: "en-US" },
      deps,
    );
    expect(packApi.createPack).toHaveBeenCalled();
    expect(packApi.openPack).toHaveBeenCalledWith({ packId: "p3" });
    expect(shell.addOpenPack).toHaveBeenCalledWith("p3", meta("p3"));
    expect((res as any).data.id).toBe("p3");
  });
});

describe("closePack", () => {
  it("closes backend then removes from store", async () => {
    const { deps, shell } = makeDeps();
    const res = await closePack("p1", deps);
    expect(packApi.closePack).toHaveBeenCalledWith({ packId: "p1" });
    expect(shell.removeOpenPack).toHaveBeenCalledWith("p1");
    expect(res.status).toBe("ok");
  });
});

describe("updatePackMeta", () => {
  it("updates backend, store, refreshes overviews and cards cache", async () => {
    const { deps, shell, queryClient } = makeDeps();
    (packApi.updatePackMetadata as any).mockResolvedValue(meta("p1"));
    const res = await updatePackMeta(
      { packId: "p1", name: "New", packCode: null, author: "a", version: "1.0.0",
        description: null, displayLanguageOrder: ["en-US"], defaultExportLanguage: "en-US" },
      deps,
    );
    expect(packApi.updatePackMetadata).toHaveBeenCalled();
    expect(shell.updatePackMetadata).toHaveBeenCalledWith("p1", meta("p1"));
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({ queryKey: ["cards", "p1"] });
    expect(shell.setPackOverviews).toHaveBeenCalled();
    expect(res.status).toBe("ok");
  });
});
