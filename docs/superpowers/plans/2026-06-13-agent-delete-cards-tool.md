# Agent Delete Cards Tool Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `delete_cards` AI Agent tool that deletes one or more cards from the active custom pack by reusing the existing batch card deletion API and confirmation flow.

**Architecture:** The tool lives in the existing agent tool layer and calls `cardApi.bulkDeleteCards`, mirroring the existing batch-shaped `move_cards` tool. It declares `confirmWrite` so `agentLoop` uses `cardApi.confirmCardBatchWrite` for confirmation tokens, while successful writes use the existing default card cache invalidation.

**Tech Stack:** TypeScript, Vitest, React Query cache invalidation through the existing agent loop, Tauri API wrappers via `src/shared/api/cardApi.ts`.

---

## File Structure

**Create:**
- `src/features/agent/tools/writeTools.test.ts` — focused Vitest coverage for `deleteCardsTool`.

**Modify:**
- `src/features/agent/tools/writeTools.ts` — add `deleteCardsTool` beside `moveCardsTool`.
- `src/features/agent/tools/registry.ts` — import and register `deleteCardsTool`.
- `src/features/agent/systemPrompt.ts` — teach the model when and how to call `delete_cards`.
- `docs/agent.md` — update agent tool counts and card write tool documentation.
- `docs/functional_spec.md` — update the AI Agent feature summary to include card deletion.

**No backend changes:** Existing `bulk_delete_cards` and `confirm_card_batch_write` commands already provide the needed behavior.

---

### Task 1: Add Failing Tool Tests

**Files:**
- Create: `src/features/agent/tools/writeTools.test.ts`

- [ ] **Step 1: Write the failing test file**

Create `src/features/agent/tools/writeTools.test.ts`:

```ts
import { beforeEach, describe, expect, it, vi } from "vitest";
import { cardApi } from "../../../shared/api/cardApi";
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
    };
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
    };
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
    };
    vi.mocked(cardApi.confirmCardBatchWrite).mockResolvedValue(confirmed);

    await expect(deleteCardsTool.confirmWrite?.("token-1")).resolves.toBe(confirmed);

    expect(cardApi.confirmCardBatchWrite).toHaveBeenCalledWith({
      confirmationToken: "token-1",
    });
  });
});
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```bash
npm run test -- writeTools
```

Expected: FAIL because `deleteCardsTool` is not exported from `src/features/agent/tools/writeTools.ts`.

- [ ] **Step 3: Commit the failing test only if your workflow requires checkpointing**

Do not commit a known failing test to the main branch. Keep this as the RED state for Task 2.

---

### Task 2: Implement `deleteCardsTool`

**Files:**
- Modify: `src/features/agent/tools/writeTools.ts`
- Test: `src/features/agent/tools/writeTools.test.ts`

- [ ] **Step 1: Add the tool implementation**

In `src/features/agent/tools/writeTools.ts`, add this export after `moveCardsTool`:

```ts
export const deleteCardsTool: AgentTool = {
  name: "delete_cards",
  description:
    "Delete one or more cards from the active pack. This is destructive and may ask " +
    "the user to confirm before deleting. Get card ids from list_cards first, or use " +
    "the selected/checked card ids from the current context. Pass a one-element array " +
    "to delete a single card. Associated images, field images, and scripts are deleted by default.",
  readOnly: false,
  confirmWrite: (confirmationToken) =>
    cardApi.confirmCardBatchWrite({ confirmationToken }),
  parameters: {
    type: "object",
    properties: {
      cardIds: {
        type: "array",
        items: { type: "string" },
        description:
          "Ids of the cards to delete. Use one id for a single-card deletion.",
      },
      deleteAssets: {
        type: "boolean",
        description:
          "Whether to delete associated images, field images, and scripts too. Defaults to true.",
      },
    },
    required: ["cardIds"],
  },
  async execute(args, ctx): Promise<WriteResult<unknown>> {
    const { workspaceId, packId } = requirePack(ctx);
    if (!Array.isArray(args.cardIds) || args.cardIds.length === 0) {
      throw new ToolError("cardIds must be a non-empty array of card ids.");
    }
    return cardApi.bulkDeleteCards({
      workspaceId,
      packId,
      cardIds: args.cardIds.map(String),
      deleteAssets: args.deleteAssets !== false,
    });
  },
};
```

- [ ] **Step 2: Run the focused test and verify GREEN**

Run:

```bash
npm run test -- writeTools
```

Expected: PASS for all `deleteCardsTool` tests.

- [ ] **Step 3: Run typecheck**

Run:

```bash
npm run typecheck
```

Expected: PASS.

- [ ] **Step 4: Commit tool implementation and tests**

Run:

```bash
git add src/features/agent/tools/writeTools.ts src/features/agent/tools/writeTools.test.ts
git commit -m "feat: add agent delete cards tool"
```

---

### Task 3: Register the Tool and Update Agent Guidance

**Files:**
- Modify: `src/features/agent/tools/registry.ts`
- Modify: `src/features/agent/systemPrompt.ts`

- [ ] **Step 1: Register `deleteCardsTool`**

In `src/features/agent/tools/registry.ts`, change the write tools import to include `deleteCardsTool`:

```ts
import {
  createCardTool,
  createSetnameTool,
  deleteCardsTool,
  deleteSetnameTool,
  moveCardsTool,
  updateCardTool,
} from "./writeTools";
```

In the `AGENT_TOOLS` array, insert `deleteCardsTool` with the card write tools, after `moveCardsTool`:

```ts
  createCardTool,
  updateCardTool,
  createSetnameTool,
  deleteSetnameTool,
  moveCardsTool,
  deleteCardsTool,
```

- [ ] **Step 2: Update the system prompt writing section**

In `src/features/agent/systemPrompt.ts`, replace the first bullet under `## Writing changes` with:

```ts
- Write operations (create_card, update_card, move_cards, delete_cards) go through the app's backend rules. Some changes require user confirmation; that is handled by the app UI, not by you — just call the tool.
```

Add this bullet after the existing bulk-update guidance:

```ts
- delete_cards deletes one or more cards from the active pack. It is destructive and may ask the user to confirm in the chat UI. Use list_cards or the current Selected/Checked card context to identify ids first; pass a one-element cardIds array for a single card. Deletion removes associated card images, field images, and scripts by default.
```

- [ ] **Step 3: Verify the tool definition is exposed**

Run:

```bash
npm run test -- writeTools
```

Expected: PASS.

Run:

```bash
npm run typecheck
```

Expected: PASS.

- [ ] **Step 4: Commit registration and prompt updates**

Run:

```bash
git add src/features/agent/tools/registry.ts src/features/agent/systemPrompt.ts
git commit -m "feat: register delete cards agent tool"
```

---

### Task 4: Sync Current Documentation

**Files:**
- Modify: `docs/agent.md`
- Modify: `docs/functional_spec.md`

- [ ] **Step 1: Update `docs/agent.md` tool counts and scope**

In `docs/agent.md`, update the scope line to match the actual registry after adding `delete_cards`:

```md
- 工具调用循环：8 个只读工具 + 12 个写工具（6 个卡片/系列名写 + 6 个 pack 写）。
```

In the tools section, replace the card write tool list sentence with:

```md
卡片写工具：`create_card`、`update_card`、`move_cards`、`delete_cards`、`create_setname`、`delete_setname`。`update_card` 覆盖全部可编辑字段（name/desc、atk/def/level、primary_type、race、attribute、monster_flags、spell_subtype、trap_subtype、pendulum、link markers、setcodes、ot、alias、category、code）：读全卡 → 仅对传入字段打补丁 → 回写，枚举字段在工具层校验取值（非法值就地报错，不丢给后端）。`move_cards` 与 `delete_cards` 均为批量形态，传一个 card id 即表示单张操作；`delete_cards` 调用后端批量删除并默认同时删除主卡图、场地图和脚本资源，确认时走 `confirmCardBatchWrite`。
```

In the confirmation section, replace:

```md
三个卡片写工具沿用默认。
```

with:

```md
`create_card` 和 `update_card` 沿用默认单卡确认；`move_cards` 与 `delete_cards` 使用批量确认端点 `confirmCardBatchWrite`。
```

- [ ] **Step 2: Update `docs/functional_spec.md` AI Agent summary**

In `docs/functional_spec.md`, replace this AI Agent bullet:

```md
- AI Agent 是对话式助手，用户用自然语言对当前激活的 custom pack 完成卡片查询、创建、修改和移动，由 DeepSeek 模型驱动。
```

with:

```md
- AI Agent 是对话式助手，用户用自然语言对当前激活的 custom pack 完成卡片查询、创建、修改、移动和删除，由 DeepSeek 模型驱动。
```

Replace this bullet:

```md
- agent 通过工具调用复用后端业务规则：只读工具（列卡、读卡、搜索标准卡、读配置、列 pack、建议 code、列 setname）和写工具（建卡、改卡、移动卡、建/改/删系列名）；改卡覆盖全部可编辑字段，建系列名复用后端 setname key 建议，删系列名经用户确认；写操作经统一确认流程。
```

with:

```md
- agent 通过工具调用复用后端业务规则：只读工具（列卡、读卡、搜索标准卡、读配置、列 pack、建议 code、列 setname）和写工具（建卡、改卡、移动卡、删卡、建/改/删系列名）；改卡覆盖全部可编辑字段，移动和删除均使用批量工具形态，单卡操作传一个 card id；建系列名复用后端 setname key 建议，删系列名经用户确认；写操作经统一确认流程。
```

- [ ] **Step 3: Run documentation-adjacent verification**

Run:

```bash
npm run typecheck
```

Expected: PASS.

- [ ] **Step 4: Commit documentation updates**

Run:

```bash
git add docs/agent.md docs/functional_spec.md
git commit -m "docs: sync agent delete cards tool"
```

---

### Task 5: Final Verification

**Files:**
- Verify only.

- [ ] **Step 1: Run full tests**

Run:

```bash
npm run test
```

Expected: PASS.

- [ ] **Step 2: Run TypeScript typecheck**

Run:

```bash
npm run typecheck
```

Expected: PASS.

- [ ] **Step 3: Run frontend build**

Run:

```bash
npm run build
```

Expected: PASS. This runs `tsc --noEmit` and `vite build`.

- [ ] **Step 4: Inspect working tree**

Run:

```bash
git status --short
```

Expected: no uncommitted changes from this task.

---

## Self-Review

**Spec coverage:**
- `delete_cards` single and multi-card support via `cardIds: string[]` → Task 2.
- `cardApi.bulkDeleteCards` reuse → Task 2 tests and implementation.
- `deleteAssets` defaults to `true`, explicit `false` supported → Task 1 and Task 2.
- `confirmCardBatchWrite` for batch confirmation → Task 1 and Task 2.
- Tool registration and prompt guidance → Task 3.
- Current docs updated → Task 4.
- No backend command or raw invoke → File structure and Task 2 use `cardApi` only.

**Placeholder scan:** No TBD/TODO/fill-in-later language remains. The only conditional text is the RED/GREEN testing checkpoint, with exact commands and expected outcomes.

**Type consistency:**
- `deleteCardsTool` is exported from `writeTools.ts` and imported by `registry.ts`.
- `confirmWrite` matches `AgentTool.confirmWrite?: (confirmationToken: string) => Promise<unknown>`.
- `cardApi.bulkDeleteCards` input fields match `BulkDeleteCardsInput`: `workspaceId`, `packId`, `cardIds`, `deleteAssets`.
- Test mock only includes `bulkDeleteCards` and `confirmCardBatchWrite`, which are the only `cardApi` methods used by `deleteCardsTool`.
- Tool counts in `docs/agent.md` should reflect the actual post-change registry: 8 read-only tools and 12 write tools.
