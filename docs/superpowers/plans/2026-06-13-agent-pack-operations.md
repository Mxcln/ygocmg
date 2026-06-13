# Agent Pack 操作（命令层）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把困在 `App.tsx` 组件闭包里的 pack 操作编排（切换/打开/关闭/新建/删除 pack、改 pack metadata）抽成与 React 无关的纯函数命令层，让 agent 通过新工具与 UI 走同一套逻辑，使 agent 能在 pack 这个"卡片作用域 + 容器"层级操作。

**Architecture:**
- 命令是纯函数：`(...args, deps: CommandDeps) => Promise<CommandResult<T>>`。副作用句柄（shell store 的 pack actions、queryClient）通过 `deps` 注入。UI 适配器（`useCommands`）和 agent 适配器（`buildAgentDeps`）各自组装 `deps`，调同一函数。
- `CommandDeps = { shell, queryClient }`——**不含 config/workspace**。本次范围限定在 pack/card 领域（pack ≈ 卡片的目录/容器，对标代码 agent 操作目录与 manifest）；workspace 切换（≈ open 另一个 folder）与 config 修改（≈ 改 editor settings）刻意排除，留给用户，符合成熟代码 agent 的边界。
- 破坏性命令（仅 `deletePack`）返回 `{ status: "needs_confirmation", confirmation: { summary, commit } }`，由适配器弹确认 UI 后调 `commit()`。这是与现有"后端 token 两段式确认"并存的**第二套**确认门（方案 B）。
- React Query 的 `queryClient` 从 `providers.tsx` 提为模块级具名导出，供非 React 的 agent 适配器访问。

**Tech Stack:** TypeScript, React 19, Zustand 5, TanStack React Query 5, Vitest（本计划新引入）, Tauri 2（后端无改动）。

**关键设计决策（实现者必读）：**
1. **范围限定 pack/card**：不做 `switch_workspace`、`update_config`。`shellStore`、`configStore` 等不因本计划改动——shellStore 完全不动。
2. **方案 B 确认门**：命令层的 `commit` 闭包模型与现有卡片写的"后端 token"模型不同，不强行统一。agent 侧在 `agentLoop.ts` 的 `runToolCall` 中新增一条与 `isWriteResult` 并列的分支识别命令确认，确认后调 `confirmation.commit()`。为此 `ConfirmationRequest`（loop 侧）与 `PendingConfirmation`（store 侧）的 `confirmationToken` 字段放宽为 `string | null`。
3. **UI 侧收敛是渐进的**：只收敛 `App.tsx` 中自身持有完整编排的 handler（`persistActivePack`、`handleClosePack`）。Modal 持有后端调用的 handler（open/create/workspace）不收敛，避免二次后端调用——超出本次范围。

## File Structure

**新建：**
- `vitest.config.ts` — Vitest 配置。
- `src/features/commands/types.ts` — `CommandDeps`、`CommandResult<T>`、`ConfirmationRequest`、`ShellCommandActions`。
- `src/features/commands/packCommands.ts` — `switchPack`/`openPack`/`createPack`/`closePack`/`updatePackMeta`/`deletePack`。
- `src/features/commands/packCommands.test.ts`
- `src/features/commands/useCommands.ts` — UI 适配器（React hook）。
- `src/features/commands/buildAgentDeps.ts` — agent 适配器（非 React）。
- `src/features/agent/tools/packTools.ts` — agent pack 写工具。

**修改：**
- `src/app/providers.tsx` — 导出 `queryClient` 单例。
- `src/app/App.tsx` — `persistActivePack`/`handleClosePack` 收敛为转调 `useCommands`。
- `src/features/agent/agentLoop.ts` — 方案 B 确认门分支；`ConfirmationRequest.confirmationToken` 放宽。
- `src/shared/stores/agentStore.ts` — `PendingConfirmation.confirmationToken` 放宽为 `string | null`。
- `src/features/agent/tools/registry.ts` — 注册新工具。
- `src/features/agent/systemPrompt.ts` — 增补新工具用途与确认说明。
- `package.json` — 加 `test` 脚本与 vitest devDeps。
- `docs/agent.md` — 落地后同步。

**不改动（与旧平权计划的差异）：** `shellStore.ts`（无 workspaceMeta）、不新建 `configStore`、不新建 `workspaceCommands`/`configCommands`、`App.tsx` 的 config/currentWorkspace 真相源保持现状。

**后端：** 无改动。

---
### Task 1: 引入 Vitest 测试框架

**Files:**
- Create: `vitest.config.ts`
- Modify: `package.json:6-11` (scripts), `package.json:23-30` (devDependencies)

- [ ] **Step 1: 安装 vitest**

Run: `npm install -D vitest@^3.0.0`
Expected: `package.json` 的 devDependencies 出现 `vitest`。

- [ ] **Step 2: 创建 vitest 配置**

Create `vitest.config.ts`:

```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    globals: true,
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
```

- [ ] **Step 3: 加 test 脚本**

Modify `package.json` scripts 块，在 `"tauri": "tauri"` 后加：

```json
    "tauri": "tauri",
    "test": "vitest run",
    "test:watch": "vitest"
```

- [ ] **Step 4: 写占位 smoke 测试**

Create `src/features/commands/smoke.test.ts`:

```ts
import { describe, it, expect } from "vitest";

describe("vitest smoke", () => {
  it("runs", () => {
    expect(1 + 1).toBe(2);
  });
});
```

- [ ] **Step 5: 运行测试**

Run: `npm run test`
Expected: PASS，1 passed。

- [ ] **Step 6: 删除占位测试并提交**

删除 `src/features/commands/smoke.test.ts`。

```bash
git add package.json package-lock.json vitest.config.ts
git commit -m "chore: add vitest test framework"
```

---

### Task 2: 命令层类型定义

**Files:**
- Create: `src/features/commands/types.ts`

命令层与 React 解耦的核心契约。`ShellCommandActions` 只挑命令真正用到的 shellStore action 子集（来自 [shellStore.ts:80-89](src/shared/stores/shellStore.ts#L80-L89)），且**不含 workspace/config**。

- [ ] **Step 1: 写类型文件（纯类型，无单测，由后续命令测试覆盖）**

Create `src/features/commands/types.ts`:

```ts
import type { QueryClient } from "@tanstack/react-query";
import type { PackMetadata, PackOverview } from "../../shared/contracts/pack";

/** The subset of shellStore actions the pack command layer drives. */
export interface ShellCommandActions {
  setActivePack: (id: string | null) => void;
  addOpenPack: (id: string, metadata: PackMetadata) => void;
  removeOpenPack: (id: string) => void;
  updatePackMetadata: (id: string, metadata: PackMetadata) => void;
  setPackOverviews: (overviews: PackOverview[]) => void;
}

/** Side-effect handles injected into every command. No React state lives here. */
export interface CommandDeps {
  shell: ShellCommandActions;
  queryClient: QueryClient;
}

/** A destructive operation paused for confirmation. */
export interface ConfirmationRequest {
  /** Human-readable summary of the operation. */
  summary: string;
  /** The closure that actually performs the operation once confirmed. */
  commit: () => Promise<unknown>;
}

export type CommandResult<T> =
  | { status: "ok"; data: T }
  | { status: "needs_confirmation"; confirmation: ConfirmationRequest };
```

- [ ] **Step 2: 类型检查**

Run: `npm run typecheck`
Expected: PASS。

- [ ] **Step 3: 提交**

```bash
git add src/features/commands/types.ts
git commit -m "feat: add pack command layer type contracts"
```

---

### Task 3: 导出 queryClient 模块级单例

**Files:**
- Modify: `src/app/providers.tsx:4-12`

agent 适配器是非 React 代码，需模块级访问 queryClient。`providers.tsx` 已是模块级 const，只差 `export`。

- [ ] **Step 1: 加 export**

Modify `src/app/providers.tsx`，把 [providers.tsx:4](src/app/providers.tsx#L4) 的 `const queryClient = ` 改为 `export const queryClient = `：

```ts
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: false,
      refetchOnWindowFocus: false,
      staleTime: 30_000,
    },
  },
});
```

- [ ] **Step 2: 类型检查**

Run: `npm run typecheck`
Expected: PASS。

- [ ] **Step 3: 提交**

```bash
git add src/app/providers.tsx
git commit -m "feat: export queryClient singleton for non-react access"
```

---
### Task 4: packCommands — 非破坏性命令

**Files:**
- Create: `src/features/commands/packCommands.ts`
- Test: `src/features/commands/packCommands.test.ts`

抽自 [App.tsx](src/app/App.tsx) 的 `persistActivePack`/`handlePackOpened`/`handlePackCreated`/`handleClosePack` 及 [PackMetadataPanel.tsx:142-162](src/features/pack/PackMetadataPanel.tsx#L142-L162) 的 metadata 保存编排。`deletePack` 留到 Task 5。

> **错误处理约定**：命令本身不吞错。`packApi.*` 抛错时直接向上抛，由适配器 catch（UI 转 notice / agent 转 tool error JSON）。`listPackOverviews` 刷新是 best-effort：`try/catch` 包住、失败静默（沿用 App.tsx 现有行为）。

- [ ] **Step 1: 写失败测试**

Create `src/features/commands/packCommands.test.ts`:

```ts
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
```

- [ ] **Step 2: 运行测试确认失败**

Run: `npm run test -- packCommands`
Expected: FAIL，无法解析 `./packCommands`。

- [ ] **Step 3: 实现 packCommands**

Create `src/features/commands/packCommands.ts`:

```ts
import { packApi } from "../../shared/api/packApi";
import type { CreatePackInput, UpdatePackMetadataInput } from "../../shared/api/packApi";
import type { PackMetadata } from "../../shared/contracts/pack";
import type { CommandDeps, CommandResult } from "./types";

/** Best-effort overview refresh; failures are swallowed (mirrors App.tsx). */
async function refreshOverviews(deps: CommandDeps): Promise<void> {
  try {
    const overviews = await packApi.listPackOverviews();
    deps.shell.setPackOverviews(overviews);
  } catch {
    // overview refresh is best-effort
  }
}

export async function switchPack(packId: string, deps: CommandDeps): Promise<CommandResult<void>> {
  deps.shell.setActivePack(packId);
  await packApi.setActivePack({ packId });
  void deps.queryClient.invalidateQueries({ queryKey: ["cards"] });
  return { status: "ok", data: undefined };
}

export async function openPack(packId: string, deps: CommandDeps): Promise<CommandResult<PackMetadata>> {
  const metadata = await packApi.openPack({ packId });
  deps.shell.addOpenPack(packId, metadata);
  await refreshOverviews(deps);
  return { status: "ok", data: metadata };
}

export async function createPack(input: CreatePackInput, deps: CommandDeps): Promise<CommandResult<PackMetadata>> {
  const created = await packApi.createPack(input);
  const opened = await packApi.openPack({ packId: created.id });
  deps.shell.addOpenPack(created.id, opened);
  await refreshOverviews(deps);
  return { status: "ok", data: opened };
}

export async function closePack(packId: string, deps: CommandDeps): Promise<CommandResult<void>> {
  await packApi.closePack({ packId });
  deps.shell.removeOpenPack(packId);
  return { status: "ok", data: undefined };
}

export async function updatePackMeta(input: UpdatePackMetadataInput, deps: CommandDeps): Promise<CommandResult<PackMetadata>> {
  const updated = await packApi.updatePackMetadata(input);
  deps.shell.updatePackMetadata(input.packId, updated);
  void deps.queryClient.invalidateQueries({ queryKey: ["cards", input.packId] });
  await refreshOverviews(deps);
  return { status: "ok", data: updated };
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `npm run test -- packCommands`
Expected: PASS，5 passed。

- [ ] **Step 5: 提交**

```bash
git add src/features/commands/packCommands.ts src/features/commands/packCommands.test.ts
git commit -m "feat: add non-destructive pack commands"
```

---

### Task 5: packCommands — deletePack（破坏性，方案 B 确认门）

**Files:**
- Modify: `src/features/commands/packCommands.ts`
- Modify: `src/features/commands/packCommands.test.ts`

`deletePack` 返回 `needs_confirmation` + `commit` 闭包，不立即执行。`commit` 内才调 `packApi.deletePack` + 善后（抽自 [App.tsx:412-421](src/app/App.tsx#L412-L421) 的 `handlePackDeleted`）。

- [ ] **Step 1: 追加失败测试**

在 `packCommands.test.ts` 顶部 import 行追加 `deletePack`：

```ts
import { switchPack, openPack, createPack, closePack, updatePackMeta, deletePack } from "./packCommands";
```

在文件末尾追加：

```ts
describe("deletePack", () => {
  it("returns needs_confirmation without touching the backend", async () => {
    const { deps } = makeDeps();
    const res = await deletePack("p1", "Pack One", deps);
    expect(res.status).toBe("needs_confirmation");
    if (res.status !== "needs_confirmation") throw new Error("expected confirmation");
    expect(res.confirmation.summary).toContain("Pack One");
    expect(packApi.deletePack).not.toHaveBeenCalled();
  });

  it("commit deletes the pack, removes from store, refreshes overviews", async () => {
    const { deps, shell } = makeDeps();
    const res = await deletePack("p1", "Pack One", deps);
    if (res.status !== "needs_confirmation") throw new Error("expected confirmation");
    await res.confirmation.commit();
    expect(packApi.deletePack).toHaveBeenCalledWith({ packId: "p1" });
    expect(shell.removeOpenPack).toHaveBeenCalledWith("p1");
    expect(shell.setPackOverviews).toHaveBeenCalled();
  });
});
```

（`packApi.deletePack` 已在 Task 4 的 `vi.mock` 中。）

- [ ] **Step 2: 运行测试确认失败**

Run: `npm run test -- packCommands`
Expected: FAIL，`deletePack` 未导出。

- [ ] **Step 3: 实现 deletePack**

在 `packCommands.ts` 末尾追加：

```ts
export async function deletePack(
  packId: string,
  packName: string,
  deps: CommandDeps,
): Promise<CommandResult<void>> {
  return {
    status: "needs_confirmation",
    confirmation: {
      summary: `Delete pack "${packName}" (${packId}). This cannot be undone.`,
      commit: async () => {
        await packApi.deletePack({ packId });
        deps.shell.removeOpenPack(packId);
        await refreshOverviews(deps);
      },
    },
  };
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `npm run test -- packCommands`
Expected: PASS，7 passed。

- [ ] **Step 5: 提交**

```bash
git add src/features/commands/packCommands.ts src/features/commands/packCommands.test.ts
git commit -m "feat: add deletePack command with confirmation gate"
```

---
### Task 6: UI 适配器 useCommands

**Files:**
- Create: `src/features/commands/useCommands.ts`

在 React 内组装 `deps`（`useQueryClient` + shellStore pack actions），返回绑好 deps 的命令。破坏性命令用 helper 接到 `openDialog`（[shellStore.ts:120](src/shared/stores/shellStore.ts#L120)，`ConfirmDialogState`）。

> 此 task 无单元测试（React hook + store 集成，由手动验证覆盖）。只需 typecheck 通过。

- [ ] **Step 1: 实现 useCommands**

Create `src/features/commands/useCommands.ts`:

```ts
import { useQueryClient } from "@tanstack/react-query";
import { useShellStore } from "../../shared/stores/shellStore";
import type { CreatePackInput, UpdatePackMetadataInput } from "../../shared/api/packApi";
import type { CommandDeps } from "./types";
import * as packCommands from "./packCommands";

/** Build CommandDeps from React/store context. */
function useCommandDeps(): CommandDeps {
  const queryClient = useQueryClient();
  const setActivePack = useShellStore((s) => s.setActivePack);
  const addOpenPack = useShellStore((s) => s.addOpenPack);
  const removeOpenPack = useShellStore((s) => s.removeOpenPack);
  const updatePackMetadata = useShellStore((s) => s.updatePackMetadata);
  const setPackOverviews = useShellStore((s) => s.setPackOverviews);

  return {
    shell: { setActivePack, addOpenPack, removeOpenPack, updatePackMetadata, setPackOverviews },
    queryClient,
  };
}

export function useCommands() {
  const deps = useCommandDeps();
  const openDialog = useShellStore((s) => s.openDialog);
  const closeDialog = useShellStore((s) => s.closeDialog);

  return {
    switchPack: (packId: string) => packCommands.switchPack(packId, deps),
    openPack: (packId: string) => packCommands.openPack(packId, deps),
    createPack: (input: CreatePackInput) => packCommands.createPack(input, deps),
    closePack: (packId: string) => packCommands.closePack(packId, deps),
    updatePackMeta: (input: UpdatePackMetadataInput) => packCommands.updatePackMeta(input, deps),

    /** Delete a pack via the UI confirmation dialog. */
    deletePackWithDialog: async (packId: string, packName: string) => {
      const result = await packCommands.deletePack(packId, packName, deps);
      if (result.status !== "needs_confirmation") return;
      openDialog({
        kind: "confirm",
        danger: true,
        title: "Delete pack",
        message: result.confirmation.summary,
        confirmLabel: "Delete",
        cancelLabel: "Cancel",
        onConfirm: async () => {
          await result.confirmation.commit();
          closeDialog();
        },
      });
    },
  };
}
```

- [ ] **Step 2: 类型检查**

Run: `npm run typecheck`
Expected: PASS。

- [ ] **Step 3: 提交**

```bash
git add src/features/commands/useCommands.ts
git commit -m "feat: add useCommands UI adapter"
```

---

### Task 7: agent 适配器 buildAgentDeps

**Files:**
- Create: `src/features/commands/buildAgentDeps.ts`

从 `useShellStore.getState()` + 模块级 `queryClient` 组装 deps。非 React，可在工具 execute 内调用。

- [ ] **Step 1: 实现 buildAgentDeps**

Create `src/features/commands/buildAgentDeps.ts`:

```ts
import { useShellStore } from "../../shared/stores/shellStore";
import { queryClient } from "../../app/providers";
import type { CommandDeps } from "./types";

/** Assemble CommandDeps outside React, for agent tools. */
export function buildAgentDeps(): CommandDeps {
  const shell = useShellStore.getState();
  return {
    shell: {
      setActivePack: shell.setActivePack,
      addOpenPack: shell.addOpenPack,
      removeOpenPack: shell.removeOpenPack,
      updatePackMetadata: shell.updatePackMetadata,
      setPackOverviews: shell.setPackOverviews,
    },
    queryClient,
  };
}
```

- [ ] **Step 2: 类型检查**

Run: `npm run typecheck`
Expected: PASS。

- [ ] **Step 3: 提交**

```bash
git add src/features/commands/buildAgentDeps.ts
git commit -m "feat: add buildAgentDeps agent adapter"
```

---

### Task 8: 方案 B 确认门 — 放宽确认类型

**Files:**
- Modify: `src/shared/stores/agentStore.ts:6-15`
- Modify: `src/features/agent/agentLoop.ts:16-23`

命令确认（commit 闭包）没有后端 token。把两处 `confirmationToken` 放宽为 `string | null`，使 token 流与 commit 流共用同一 UI 通道。

- [ ] **Step 1: 放宽 PendingConfirmation**

Modify `src/shared/stores/agentStore.ts`，把 [agentStore.ts:10](src/shared/stores/agentStore.ts#L10) 的 `confirmationToken: string;` 改为：

```ts
  confirmationToken: string | null;
```

- [ ] **Step 2: 放宽 loop 的 ConfirmationRequest**

Modify `src/features/agent/agentLoop.ts`，把 [agentLoop.ts:19](src/features/agent/agentLoop.ts#L19) 的 `confirmationToken: string;` 改为：

```ts
  confirmationToken: string | null;
```

- [ ] **Step 3: 类型检查**

Run: `npm run typecheck`
Expected: PASS（放宽向后兼容；现有传 `string` 的调用仍合法）。

- [ ] **Step 4: 提交**

```bash
git add src/shared/stores/agentStore.ts src/features/agent/agentLoop.ts
git commit -m "refactor: widen confirmationToken to allow command confirmations"
```

---

### Task 9: 方案 B 确认门 — loop 识别命令确认

**Files:**
- Modify: `src/features/agent/agentLoop.ts:41-116`

在 `runToolCall` 中新增与 `isWriteResult` 并列的分支：识别命令层的 `needs_confirmation`（含 `commit` 闭包），走 `hooks.requestConfirmation`（token/warnings/preview 传 null/[]），确认后调 `commit()`；命令的 `ok` 结果 unwrap 成 data。

- [ ] **Step 1: 加识别函数**

在 `agentLoop.ts` 的 `isWriteResult`（[agentLoop.ts:41-49](src/features/agent/agentLoop.ts#L41-L49)）后追加：

```ts
interface CommandConfirmation {
  status: "needs_confirmation";
  confirmation: { summary: string; commit: () => Promise<unknown> };
}

function isCommandConfirmation(value: unknown): value is CommandConfirmation {
  return (
    typeof value === "object" &&
    value !== null &&
    (value as { status?: unknown }).status === "needs_confirmation" &&
    typeof (value as { confirmation?: unknown }).confirmation === "object" &&
    (value as { confirmation?: { commit?: unknown } }).confirmation != null &&
    typeof (value as { confirmation: { commit?: unknown } }).confirmation.commit === "function"
  );
}

function isCommandResultOk(value: unknown): value is { status: "ok"; data: unknown } {
  return (
    typeof value === "object" &&
    value !== null &&
    (value as { status?: unknown }).status === "ok" &&
    "data" in (value as object) &&
    !("warnings" in (value as object)) // distinguish from card WriteResult
  );
}
```

- [ ] **Step 2: 加 commit 流处理函数**

在 `handleWriteResult`（[agentLoop.ts:118-151](src/features/agent/agentLoop.ts#L118-L151)）后追加：

```ts
async function handleCommandConfirmation(
  result: CommandConfirmation,
  call: ToolCall,
  hooks: LoopHooks,
): Promise<string> {
  const apply = await hooks.requestConfirmation({
    toolCallId: call.id,
    toolName: call.function.name,
    confirmationToken: null,
    warnings: [],
    preview: null,
    summary: result.confirmation.summary,
  });
  if (!apply) {
    return JSON.stringify({ status: "cancelled_by_user" });
  }
  try {
    const data = await result.confirmation.commit();
    return JSON.stringify({ status: "ok", data: data ?? null });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return JSON.stringify({ error: `Operation failed: ${message}` });
  }
}
```

- [ ] **Step 3: 在 runToolCall 接线**

Modify `runToolCall` 的 `try` 块（[agentLoop.ts:103-115](src/features/agent/agentLoop.ts#L103-L115)）。把现有：

```ts
    const result = await tool.execute(args, ctx);

    // Write tools return a WriteResult that may require backend confirmation.
    if (!tool.readOnly && isWriteResult(result)) {
      return handleWriteResult(result, call, args, hooks);
    }
    return JSON.stringify(result ?? null);
```

改为：

```ts
    const result = await tool.execute(args, ctx);

    // Command-layer confirmation (delete_pack): commit-closure model.
    if (!tool.readOnly && isCommandConfirmation(result)) {
      return handleCommandConfirmation(result, call, hooks);
    }
    // Card write tools: backend two-phase token model.
    if (!tool.readOnly && isWriteResult(result)) {
      return handleWriteResult(result, call, args, hooks);
    }
    // Command-layer ok results: unwrap to data for the model.
    if (!tool.readOnly && isCommandResultOk(result)) {
      return JSON.stringify({ status: "ok", data: result.data ?? null });
    }
    return JSON.stringify(result ?? null);
```

- [ ] **Step 4: 类型检查**

Run: `npm run typecheck`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add src/features/agent/agentLoop.ts
git commit -m "feat: handle command-layer confirmation in agent loop"
```

---
### Task 10: agent pack 写工具

**Files:**
- Create: `src/features/agent/tools/packTools.ts`

各工具调对应命令（通过 `buildAgentDeps()`）。沿用 [writeTools.ts](src/features/agent/tools/writeTools.ts) 的 `AgentTool` 形态。`delete_pack` 返回命令的 `needs_confirmation`，由 Task 9 的 loop 分支处理。

> `create_pack` 的语言默认值：像 [readTools.ts:123](src/features/agent/tools/readTools.ts#L123) 的 `get_config` 一样直接调 `configApi.loadConfig()`，用 `preferredAuthoringLanguage(config)`（[language.ts:47](src/shared/utils/language.ts#L47)）算出建包语言——无需把 config 塞进 deps。

- [ ] **Step 1: 实现 packTools（switch/open/close）**

Create `src/features/agent/tools/packTools.ts`:

```ts
import { configApi } from "../../../shared/api/configApi";
import { useShellStore } from "../../../shared/stores/shellStore";
import { preferredAuthoringLanguage } from "../../../shared/utils/language";
import { buildAgentDeps } from "../../commands/buildAgentDeps";
import * as packCommands from "../../commands/packCommands";
import type { AgentTool } from "./types";
import { ToolError } from "./types";

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
```

- [ ] **Step 2: 追加 create_pack / update_pack_meta / delete_pack**

在 `packTools.ts` 末尾追加：

```ts
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
```

- [ ] **Step 3: 类型检查**

Run: `npm run typecheck`
Expected: PASS。

- [ ] **Step 4: 提交**

```bash
git add src/features/agent/tools/packTools.ts
git commit -m "feat: add agent pack tools"
```

---

### Task 11: 注册工具 + 更新 system prompt

**Files:**
- Modify: `src/features/agent/tools/registry.ts:1-25`
- Modify: `src/features/agent/systemPrompt.ts:17-24`

- [ ] **Step 1: 注册新工具**

Modify `src/features/agent/tools/registry.ts`。在 import 块（[registry.ts:11](src/features/agent/tools/registry.ts#L11) 后）加：

```ts
import {
  switchPackTool,
  openPackTool,
  closePackTool,
  createPackTool,
  updatePackMetaTool,
  deletePackTool,
} from "./packTools";
```

把 `AGENT_TOOLS` 数组（[registry.ts:14-25](src/features/agent/tools/registry.ts#L14-L25)）改为：

```ts
export const AGENT_TOOLS: AgentTool[] = [
  listCardsTool,
  getCardTool,
  searchStandardCardsTool,
  getConfigTool,
  getPackInfoTool,
  listPacksTool,
  suggestCardCodeTool,
  createCardTool,
  updateCardTool,
  moveCardsTool,
  switchPackTool,
  openPackTool,
  closePackTool,
  createPackTool,
  updatePackMetaTool,
  deletePackTool,
];
```

- [ ] **Step 2: 更新 system prompt**

Modify `src/features/agent/systemPrompt.ts`。在 "## Writing changes" 段（[systemPrompt.ts:17-20](src/features/agent/systemPrompt.ts#L17-L20)）后、`## Style` 之前，追加：

```ts
## Managing packs
- You can open/switch/close packs (open_pack / switch_pack / close_pack), create packs (create_pack), and edit pack metadata (update_pack_meta). These take effect in the UI immediately. To work on a different pack the user mentions, switch_pack (if open) or open_pack (if not) first.
- delete_pack is destructive: it asks the user to confirm in the chat before deleting. If the user cancels, nothing happens.
- You operate within the user's current workspace. You cannot switch workspaces or change app settings — ask the user to do those in the UI.
```

- [ ] **Step 3: 类型检查 + 全部测试**

Run: `npm run typecheck && npm run test`
Expected: 均 PASS。

- [ ] **Step 4: 提交**

```bash
git add src/features/agent/tools/registry.ts src/features/agent/systemPrompt.ts
git commit -m "feat: register agent pack tools and update system prompt"
```

---

### Task 12: App.tsx — 收敛 persistActivePack / handleClosePack

**Files:**
- Modify: `src/app/App.tsx:304-410` (AppShell handlers)

只收敛 `AppShell` 中自身持有完整编排的两个 handler。保留 notice 包装（命令不吞错，handler 负责 catch + notice）。

> **不收敛**：`handlePackOpened`/`handlePackCreated`（AddPackModal 已调后端再回调）、`handleWorkspaceOpened`（WorkspaceModal 已调后端）、`handleConfigSaved`（SettingsModal 已调后端）、`handlePackDeleted`（PackMetadataPanel 删除善后）——改它们会二次调用后端或牵动 Modal 调用链，超出本次范围。

- [ ] **Step 1: 在 AppShell 顶部取 useCommands**

Modify `src/app/App.tsx`，import 块加：

```ts
import { useCommands } from "../features/commands/useCommands";
```

在 `AppShell` 函数体内（[App.tsx:304](src/app/App.tsx#L304) `const { t } = useAppI18n();` 后）加：

```ts
  const commands = useCommands();
```

- [ ] **Step 2: 收敛 persistActivePack**

把 `AppShell` 的 `persistActivePack`（[App.tsx:346-353](src/app/App.tsx#L346-L353)）改为：

```ts
  async function persistActivePack(packId: string) {
    try {
      await commands.switchPack(packId);
    } catch (err) {
      handleNotice("error", t("app.notice.switchPackFailed"), formatError(err));
    }
  }
```

- [ ] **Step 3: 收敛 handleClosePack**

把 `handleClosePack`（[App.tsx:403-410](src/app/App.tsx#L403-L410)）改为：

```ts
  async function handleClosePack(packId: string) {
    try {
      await commands.closePack(packId);
    } catch (err) {
      handleNotice("error", t("app.notice.closePackFailed"), formatError(err));
    }
  }
```

- [ ] **Step 4: 清理未使用的 setActivePack 选择器**

收敛后 `AppShell` 内 `persistActivePack` 不再直接用 `setActivePack`。用 Grep 确认 `AppShell` 函数体内 `setActivePack` 仅此一处引用：

Run: `grep -n "setActivePack" src/app/App.tsx`
若 `AppShell`（约 [App.tsx:311](src/app/App.tsx#L311)）内的 `const setActivePack = useShellStore((s) => s.setActivePack);` 已无其他使用者，删除该行（避免 `noUnusedLocals` 报错）。`App` 顶层组件（[App.tsx:75](src/app/App.tsx#L75)）的 `setActivePack` 仍被 bootstrap 的 `persistActivePack` 使用，保留。

> 注意：`App` 顶层与 `AppShell` 各有一份 `persistActivePack`/`setActivePack`。本 task 只动 `AppShell` 那份；`App` 顶层的 bootstrap 版本不在 AppShell 作用域，无 `commands` 可用，保持原样。

- [ ] **Step 5: 类型检查 + 测试**

Run: `npm run typecheck && npm run test`
Expected: 均 PASS。

- [ ] **Step 6: 提交**

```bash
git add src/app/App.tsx
git commit -m "refactor: converge AppShell active-pack and close-pack handlers to commands"
```

---

### Task 13: 手动验证 + 同步 docs/agent.md

**Files:**
- Modify: `docs/agent.md`

- [ ] **Step 1: 全量自动验证**

Run: `npm run typecheck && npm run test && npm run build`
Expected: 三者均 PASS（build 含 tsc + vite build）。

- [ ] **Step 2: 手动验证 agent pack 操作**

Run: `npm run dev`，在 agent 面板依次验证：
- "打开 <pack 名>" / "切换到 <pack>" → 侧栏与卡片列表即时反映。
- "新建一个叫 X 的卡包" → 卡包出现并激活。
- "把当前卡包改名为 Y" → pack metadata 即时更新（标题/侧栏）。
- "关闭这个卡包" → 从打开列表移除。
- "删除卡包 Z" → agent 面板内联弹确认；点取消 → 卡包仍在；再删 → 点确认 → 卡包消失。
- "切换到另一个 workspace" / "把主题改成深色" → agent 应回答它做不到，请用户在 UI 操作（验证范围边界）。

Expected: pack 操作 UI 立即反映；workspace/config 请求被婉拒。

- [ ] **Step 3: 验证 UI 与 agent 走同一命令**

手动在 UI 切 pack、关 pack，确认行为与 agent 操作一致（同一命令、同一副作用）。

- [ ] **Step 4: 同步 docs/agent.md**

读 `docs/agent.md` 当前内容，补充：新工具清单（switch_pack / open_pack / close_pack / create_pack / update_pack_meta / delete_pack）、命令层架构（纯函数 + `{shell, queryClient}` deps 注入 + UI/agent 双适配器）、方案 B 确认门（commit 闭包 vs 后端 token 并存）、范围边界（agent 限 pack/card，不碰 workspace/config）。依 `docs/agent.md` 现有结构补写，不复述历史计划。

- [ ] **Step 5: 提交**

```bash
git add docs/agent.md
git commit -m "docs: sync agent.md with pack command layer and tools"
```

---

## Self-Review

**1. 范围覆盖（窄化后的目标）：**
- pack 作用域切换（switch/open/close）→ Task 4 + Task 10 ✓
- pack CRUD（create/delete）→ Task 4/5 + Task 10 ✓
- pack metadata（update_pack_meta）→ Task 4 + Task 10 ✓
- 命令层纯函数 + `{shell, queryClient}` deps → Task 2/4/5 ✓
- 方案 B 确认门 → Task 5/8/9；UI 侧 deletePackWithDialog → Task 6 ✓
- 双适配器 + queryClient 单例 → Task 3/6/7 ✓
- agent 工具注册 + system prompt（含边界声明）→ Task 11 ✓
- UI 收敛（限两个 handler）→ Task 12 ✓
- 验证 + 文档 → Task 13 ✓
- **明确排除**：switch_workspace、update_config、configStore、workspaceMeta —— 不在任何 task，符合窄化决策。

**2. Placeholder scan:** 无 TBD/TODO；所有 code step 含完整代码；测试含真实断言。Task 12 Step 4 的"清理未使用变量"是条件性操作，给了 Grep 判断依据，非占位。

**3. Type consistency:**
- `CommandDeps = { shell, queryClient }`：types(Task 2) / packCommands(Task 4-5) / useCommands(Task 6) / buildAgentDeps(Task 7) 一致，均无 config/workspace 字段 ✓
- `ShellCommandActions` 五个 action：types(Task 2) 与 useCommands(Task 6)/buildAgentDeps(Task 7) 组装字段完全一致 ✓
- `CommandResult<T>` 的 `ok`/`needs_confirmation` 分支：types(Task 2) / 各命令(Task 4-5) / loop 识别(Task 9) 一致 ✓
- `ConfirmationRequest`（命令层 `{summary, commit}`，Task 2）与 loop 层同名 `ConfirmationRequest`（token 形状，Task 8）分属不同文件作用域，无碰撞 ✓
- `confirmationToken: string | null`：agentStore(Task 8) / agentLoop(Task 8) / handleCommandConfirmation 传 null(Task 9) 一致 ✓
- `createPack`/`updatePackMeta` 的 input 形状与 [packApi.ts](src/shared/api/packApi.ts) 的 `CreatePackInput`/`UpdatePackMetadataInput` 一致 ✓




