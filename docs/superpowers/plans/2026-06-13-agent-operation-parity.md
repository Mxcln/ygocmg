# Agent 操作平权（命令层）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建立一个 UI 与 agent 共用的确定性命令层，把困在 `App.tsx` 组件闭包里的操作编排（切 workspace/pack、改 pack metadata、改 config、增删开关 pack）抽成纯函数，让 agent 通过新工具与 UI 走同一套逻辑，实现操作平权。

**Architecture:**
- 命令是与 React 无关的纯函数：`(...args, deps: CommandDeps) => Promise<CommandResult<T>>`。副作用句柄（shell store actions、queryClient、config getter/setter）通过 `deps` 注入。UI 适配器（`useCommands`）和 agent 适配器（`buildAgentDeps`）各自组装 `deps`，调同一函数。
- 破坏性命令（v1 仅 `deletePack`）返回 `{ status: "needs_confirmation", confirmation: { summary, commit } }`，由适配器负责弹确认 UI 后调 `commit()`。这是与现有"后端 token 两段式确认"并存的**第二套**确认门（方案 B）。
- config 与 currentWorkspace 的真相源从 `App.tsx` 的 `useState` 上移到 Zustand store（沿用 spec §3 的最小改造取舍：只换真相源，保留现有 props 透传），使 agent 写操作能即时反映到 UI。
- React Query 的 `queryClient` 从 `providers.tsx` 提为模块级具名导出，供非 React 的 agent 适配器访问。

**Tech Stack:** TypeScript, React 19, Zustand 5, TanStack React Query 5, Vitest（本计划新引入的测试框架）, Tauri 2（后端无改动）。

**关键设计决策（实现者必读）：**
1. **方案 B 确认门**：命令层的 `commit` 闭包模型与现有卡片写的"后端 token"模型不同。不强行统一。agent 侧在 `agentLoop.ts` 的 `runToolCall` 中新增一条与 `isWriteResult` 并列的分支识别命令确认，确认后调 `confirmation.commit()`（而非 `cardApi.confirmCardWrite`）。为此 `ConfirmationRequest`（loop 侧）与 `PendingConfirmation`（store 侧）的 `confirmationToken` 字段放宽为 `string | null`。
2. **currentWorkspace 上移到 store**：spec §3 只点名 config，但 agent 的 `switch_workspace` 若不更新 `App.tsx` 的 `currentWorkspace` 本地 state，标题栏 workspace 名不会刷新（spec §9 要求"UI 立即反映"）。故按与 config 完全相同的最小改造模式，把 currentWorkspace 真相源移入 `shellStore`（新增 `workspaceMeta` 字段），`App.tsx` 从 store 计算 `currentWorkspace` 后仍按原样 props 透传。
3. **新增 `list_recent_workspaces` 只读工具**：spec §4 说 agent `switch_workspace` 切到"最近列表中的"workspace，但 spec 未列出让 agent 看到该列表的读工具。没有它 agent 无从得知可切的路径。故补一个只读工具，作为 spec 工具清单的必要补充。

---

## File Structure

**新建：**
- `vitest.config.ts` — Vitest 配置（node 环境，globals）。
- `src/features/commands/types.ts` — `CommandDeps`、`CommandResult<T>`、`ConfirmationRequest`、`ShellCommandActions`。
- `src/features/commands/packCommands.ts` — `switchPack`/`openPack`/`createPack`/`closePack`/`updatePackMeta`/`deletePack`。
- `src/features/commands/packCommands.test.ts`
- `src/features/commands/workspaceCommands.ts` — `openWorkspace`/`switchWorkspace`。
- `src/features/commands/workspaceCommands.test.ts`
- `src/features/commands/configCommands.ts` — `updateConfig`。
- `src/features/commands/configCommands.test.ts`
- `src/features/commands/useCommands.ts` — UI 适配器（React hook）。
- `src/features/commands/buildAgentDeps.ts` — agent 适配器（非 React）。
- `src/shared/stores/configStore.ts` — config store。
- `src/shared/stores/configStore.test.ts`
- `src/features/agent/tools/commandTools.ts` — agent 写工具 + `list_recent_workspaces` 读工具。

**修改：**
- `src/app/providers.tsx` — 导出 `queryClient` 单例。
- `src/shared/stores/shellStore.ts` — 新增 `workspaceMeta` 字段与 setter 改造。
- `src/app/App.tsx` — config / currentWorkspace 真相源改 store；handler 收敛为转调 `useCommands`。
- `src/app/hooks/useAppWindow.ts` / `useSidebarResize.ts` / `useRightSidebarResize.ts` — `setConfig` 参数类型从 `Dispatch<SetStateAction>` 改为 `(c: GlobalConfig) => void`，并改掉 `useAppWindow` 里唯一的函数式 setState 调用。
- `src/features/agent/agentLoop.ts` — 方案 B 确认门分支；`ConfirmationRequest.confirmationToken` 放宽。
- `src/shared/stores/agentStore.ts` — `PendingConfirmation.confirmationToken` 放宽为 `string | null`。
- `src/features/agent/tools/registry.ts` — 注册新工具。
- `src/features/agent/systemPrompt.ts` — 增补新工具用途与确认说明。
- `package.json` — 加 `test` 脚本与 vitest devDeps。
- `docs/agent.md` — 落地后同步（最后一个 task）。

**后端：** 无改动。

---
### Task 1: 引入 Vitest 测试框架

**Files:**
- Create: `vitest.config.ts`
- Modify: `package.json:6-11` (scripts), `package.json:23-30` (devDependencies)

- [ ] **Step 1: 安装 vitest**

Run: `npm install -D vitest@^3.0.0`
Expected: `package.json` 的 devDependencies 出现 `vitest`，`package-lock.json` 更新。

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

Modify `package.json` scripts 块，在 `"tauri": "tauri"` 后加一行：

```json
    "tauri": "tauri",
    "test": "vitest run",
    "test:watch": "vitest"
```

- [ ] **Step 4: 写一个占位 smoke 测试确认框架可跑**

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

命令层与 React 解耦的核心契约。`ShellCommandActions` 只挑命令真正用到的 shellStore action 子集（来自 [shellStore.ts:77-89](src/shared/stores/shellStore.ts#L77-L89)），避免命令依赖整个 store 类型。

- [ ] **Step 1: 写类型文件（无测试 — 纯类型，由后续命令测试覆盖）**

Create `src/features/commands/types.ts`:

```ts
import type { QueryClient } from "@tanstack/react-query";
import type { GlobalConfig } from "../../shared/contracts/config";
import type { PackMetadata } from "../../shared/contracts/pack";
import type { WorkspaceMeta } from "../../shared/contracts/workspace";

/** The subset of shellStore actions the command layer drives. */
export interface ShellCommandActions {
  setActivePack: (id: string | null) => void;
  addOpenPack: (id: string, metadata: PackMetadata) => void;
  removeOpenPack: (id: string) => void;
  updatePackMetadata: (id: string, metadata: PackMetadata) => void;
  setPackOverviews: (overviews: import("../../shared/contracts/pack").PackOverview[]) => void;
  setWorkspace: (id: string, name: string, path: string, meta: WorkspaceMeta) => void;
}

/** Side-effect handles injected into every command. No React state lives here. */
export interface CommandDeps {
  shell: ShellCommandActions;
  queryClient: QueryClient;
  getConfig: () => GlobalConfig | null;
  setConfig: (next: GlobalConfig) => void;
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
Expected: PASS（无错误；`setWorkspace` 的 4 参签名将在 Task 4 落地到 shellStore）。

> 注：此处 `setWorkspace` 已按 Task 4 改造后的 4 参签名（含 `meta`）定义。typecheck 此刻只查 types.ts 自身，不连 shellStore，故通过。

- [ ] **Step 3: 提交**

```bash
git add src/features/commands/types.ts
git commit -m "feat: add command layer type contracts"
```

---

### Task 3: configStore

**Files:**
- Create: `src/shared/stores/configStore.ts`
- Test: `src/shared/stores/configStore.test.ts`

把 config 真相源从 `App.tsx` 的 `useState` 移出（spec §3）。沿用 [shellStore.ts:92](src/shared/stores/shellStore.ts#L92) 的 zustand `create` 模式。

- [ ] **Step 1: 写失败测试**

Create `src/shared/stores/configStore.test.ts`:

```ts
import { describe, it, expect, beforeEach } from "vitest";
import { useConfigStore } from "./configStore";
import type { GlobalConfig } from "../contracts/config";

function makeConfig(): GlobalConfig {
  return {
    app_language: "en-US",
    ygopro_path: null,
    external_text_editor_path: null,
    custom_code_recommended_min: 100000000,
    custom_code_recommended_max: 199999999,
    custom_code_min_gap: 1,
    shell_sidebar_width: 150,
    shell_sidebar_collapsed: false,
    shell_right_sidebar_width: 320,
    shell_right_sidebar_collapsed: true,
    shell_window_width: 1280,
    shell_window_height: 800,
    shell_window_is_maximized: false,
    text_language_catalog: [],
    standard_pack_source_language: null,
    theme_mode: "system",
    high_contrast: false,
    custom_brand_color: null,
    deepseek_api_key: null,
    agent_language: "auto",
  };
}

describe("configStore", () => {
  beforeEach(() => {
    useConfigStore.setState({ config: null });
  });

  it("starts with null config", () => {
    expect(useConfigStore.getState().config).toBeNull();
  });

  it("setConfig replaces the config", () => {
    const cfg = makeConfig();
    useConfigStore.getState().setConfig(cfg);
    expect(useConfigStore.getState().config).toBe(cfg);
    expect(useConfigStore.getState().config?.app_language).toBe("en-US");
  });
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `npm run test -- configStore`
Expected: FAIL，无法解析 `./configStore`（文件不存在）。

- [ ] **Step 3: 实现 configStore**

Create `src/shared/stores/configStore.ts`:

```ts
import { create } from "zustand";
import type { GlobalConfig } from "../contracts/config";

interface ConfigState {
  config: GlobalConfig | null;
  setConfig: (next: GlobalConfig) => void;
}

export const useConfigStore = create<ConfigState>()((set) => ({
  config: null,
  setConfig: (next) => set({ config: next }),
}));
```

- [ ] **Step 4: 运行测试确认通过**

Run: `npm run test -- configStore`
Expected: PASS，2 passed。

- [ ] **Step 5: 提交**

```bash
git add src/shared/stores/configStore.ts src/shared/stores/configStore.test.ts
git commit -m "feat: add configStore for shared config truth source"
```

---

### Task 4: shellStore 增加 workspaceMeta

**Files:**
- Modify: `src/shared/stores/shellStore.ts:48-90` (interface), `:146-160` (setWorkspace), `:92-99` (initial state)

agent 的 `switch_workspace` 需要让 `App.tsx` 标题栏 workspace 名即时刷新（设计决策 2）。把 `WorkspaceMeta` 存进 shellStore，`setWorkspace` 加第 4 个参数 `meta`。

- [ ] **Step 1: 在 ShellState 接口加字段**

Modify `src/shared/stores/shellStore.ts`，在 import 块加 `WorkspaceMeta`（[shellStore.ts:2](src/shared/stores/shellStore.ts#L2) 附近）：

```ts
import type { PackMetadata, PackOverview } from "../contracts/pack";
import type { WorkspaceMeta } from "../contracts/workspace";
```

在 `ShellState` 接口的 `workspacePath: string | null;`（[shellStore.ts:50](src/shared/stores/shellStore.ts#L50)）后加：

```ts
  workspacePath: string | null;
  workspaceMeta: WorkspaceMeta | null;
```

- [ ] **Step 2: 改 setWorkspace 签名**

Modify `ShellState` 接口里 `setWorkspace` 声明（[shellStore.ts:77](src/shared/stores/shellStore.ts#L77)）：

```ts
  setWorkspace: (id: string, name: string, path: string, meta: WorkspaceMeta) => void;
```

- [ ] **Step 3: 改初始 state 与实现**

在 `create` 初始 state 的 `workspacePath: null,`（[shellStore.ts:95](src/shared/stores/shellStore.ts#L95)）后加 `workspaceMeta: null,`。

改 `setWorkspace` 实现（[shellStore.ts:146-160](src/shared/stores/shellStore.ts#L146-L160)）：

```ts
  setWorkspace: (id, name, path, meta) =>
    set({
      workspaceId: id,
      workspaceName: name,
      workspacePath: path,
      workspaceMeta: meta,
      openPackIds: [],
      activePackId: null,
      activeView: null,
      selectedCard: null,
      checkedCards: [],
      packMetadataMap: {},
      packOverviews: [],
      dialog: null,
      dialogBusy: false,
    }),
```

在 `clearWorkspace` 实现（[shellStore.ts:162](src/shared/stores/shellStore.ts#L162)）的 `workspacePath: null,` 后加 `workspaceMeta: null,`。

- [ ] **Step 4: 类型检查（预期报错，标记现有调用点）**

Run: `npm run typecheck`
Expected: FAIL，`App.tsx` 三处 `setWorkspace(...)` 调用缺第 4 参数 —— 这些将在 Task 9 修复。记录报错位置，暂不改。

> 此 task 暂不能独立通过 typecheck，因为调用点在 App.tsx。Task 9 完成后整体通过。为保证可提交，下一步先把 App.tsx 的调用点补 `meta`（最小改动）。

- [ ] **Step 5: 补 App.tsx 现有三处 setWorkspace 调用**

Modify `src/app/App.tsx`：
- [App.tsx:162](src/app/App.tsx#L162) `setWorkspace(meta.id, meta.name, lastEntry.path);` → `setWorkspace(meta.id, meta.name, lastEntry.path, meta);`
- [App.tsx:361](src/app/App.tsx#L361) `setWorkspace(meta.id, meta.name, path);` → `setWorkspace(meta.id, meta.name, path, meta);`

（这两处的 `meta` 变量已是 `WorkspaceMeta` 类型，直接传入。）

- [ ] **Step 6: 类型检查确认通过**

Run: `npm run typecheck`
Expected: PASS。

- [ ] **Step 7: 提交**

```bash
git add src/shared/stores/shellStore.ts src/app/App.tsx
git commit -m "feat: store WorkspaceMeta in shellStore"
```

---

### Task 5: 导出 queryClient 模块级单例

**Files:**
- Modify: `src/app/providers.tsx:4-12`

agent 适配器是非 React 代码，需要模块级访问 queryClient（spec §6）。`providers.tsx` 已是模块级 const，只差 `export`。

- [ ] **Step 1: 加 export**

Modify `src/app/providers.tsx`，把 [providers.tsx:4](src/app/providers.tsx#L4) 的 `const queryClient = ` 改为：

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
### Task 6: packCommands — 非破坏性命令

**Files:**
- Create: `src/features/commands/packCommands.ts`
- Test: `src/features/commands/packCommands.test.ts`

抽自 [App.tsx](src/app/App.tsx) 的 `persistActivePack`/`handlePackOpened`/`handlePackCreated`/`handleClosePack` 及 [PackMetadataPanel.tsx:142-162](src/features/pack/PackMetadataPanel.tsx#L142-L162) 的 metadata 保存编排。`deletePack` 留到 Task 7。

> **错误处理约定**：命令本身不吞错。`packApi.*` 抛错时命令直接向上抛，由适配器负责 catch + notice（UI）或转成 tool error JSON（agent）。`listPackOverviews` 刷新是 best-effort：用 `try/catch` 包住、失败静默（沿用 App.tsx 现有行为）。

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
    setActivePack: vi.fn(),
    addOpenPack: vi.fn(),
    removeOpenPack: vi.fn(),
    updatePackMetadata: vi.fn(),
    setPackOverviews: vi.fn(),
    setWorkspace: vi.fn(),
  };
  const queryClient = { invalidateQueries: vi.fn() } as any;
  const deps: CommandDeps = {
    shell, queryClient,
    getConfig: () => null,
    setConfig: vi.fn(),
  };
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

### Task 7: packCommands — deletePack（破坏性，方案 B 确认门）

**Files:**
- Modify: `src/features/commands/packCommands.ts`
- Modify: `src/features/commands/packCommands.test.ts`

`deletePack` 返回 `needs_confirmation` + `commit` 闭包，不立即执行（spec §5）。`commit` 内才调 `packApi.deletePack` + 善后（抽自 [App.tsx:412-421](src/app/App.tsx#L412-L421) 的 `handlePackDeleted`）。

- [ ] **Step 1: 追加失败测试**

在 `packCommands.test.ts` 末尾追加：

```ts
import { deletePack } from "./packCommands";

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
    (packApi.deletePack as any) = vi.fn().mockResolvedValue(undefined);
    const res = await deletePack("p1", "Pack One", deps);
    if (res.status !== "needs_confirmation") throw new Error("expected confirmation");
    await res.confirmation.commit();
    expect(packApi.deletePack).toHaveBeenCalledWith({ packId: "p1" });
    expect(shell.removeOpenPack).toHaveBeenCalledWith("p1");
    expect(shell.setPackOverviews).toHaveBeenCalled();
  });
});
```

同时在文件顶部的 `vi.mock` 的 `packApi` 对象里补一行 `deletePack: vi.fn().mockResolvedValue(undefined),`。

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
### Task 8: workspaceCommands

**Files:**
- Create: `src/features/commands/workspaceCommands.ts`
- Test: `src/features/commands/workspaceCommands.test.ts`

抽自 [App.tsx:359-381](src/app/App.tsx#L359-L381) 的 `handleWorkspaceOpened`。`openWorkspace` 接受任意路径（UI 用），`switchWorkspace` 切到最近列表中的已知路径（agent 用）；两者共用同一编排，区别仅路径来源（spec §4）。

> 编排顺序（必须保持）：`openWorkspace(api)` → `setWorkspace(store)` → `listPackOverviews` + `setPackOverviews` → 逐个 `openPack`/`addOpenPack` → 若有 `last_opened_pack_id` 则 `switchPack`。`open_pack_ids` 逐个 open 时单个失败要跳过（pack 可能已删，沿用 App.tsx）。

- [ ] **Step 1: 写失败测试**

Create `src/features/commands/workspaceCommands.test.ts`:

```ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { openWorkspace } from "./workspaceCommands";
import type { CommandDeps } from "./types";
import type { WorkspaceMeta } from "../../shared/contracts/workspace";
import type { PackMetadata } from "../../shared/contracts/pack";
import { workspaceApi } from "../../shared/api/workspaceApi";
import { packApi } from "../../shared/api/packApi";

vi.mock("../../shared/api/workspaceApi", () => ({
  workspaceApi: { openWorkspace: vi.fn() },
}));
vi.mock("../../shared/api/packApi", () => ({
  packApi: {
    listPackOverviews: vi.fn().mockResolvedValue([]),
    openPack: vi.fn(),
    setActivePack: vi.fn().mockResolvedValue(undefined),
  },
}));

function wsMeta(): WorkspaceMeta {
  return {
    id: "ws1", name: "WS One", description: null, created_at: "", updated_at: "",
    pack_order: [], last_opened_pack_id: "p1", open_pack_ids: ["p1"],
  };
}
function packMeta(id: string): PackMetadata {
  return {
    id, kind: "custom", name: `Pack ${id}`, pack_code: null, author: "a",
    version: "1.0.0", description: null, created_at: "", updated_at: "",
    display_language_order: ["en-US"], default_export_language: "en-US",
  };
}

function makeDeps() {
  const shell = {
    setActivePack: vi.fn(), addOpenPack: vi.fn(), removeOpenPack: vi.fn(),
    updatePackMetadata: vi.fn(), setPackOverviews: vi.fn(), setWorkspace: vi.fn(),
  };
  const queryClient = { invalidateQueries: vi.fn() } as any;
  const deps: CommandDeps = { shell, queryClient, getConfig: () => null, setConfig: vi.fn() };
  return { deps, shell };
}

beforeEach(() => vi.clearAllMocks());

describe("openWorkspace", () => {
  it("opens workspace, sets store, opens saved packs, activates last", async () => {
    const { deps, shell } = makeDeps();
    (workspaceApi.openWorkspace as any).mockResolvedValue(wsMeta());
    (packApi.openPack as any).mockResolvedValue(packMeta("p1"));
    const res = await openWorkspace("/path/ws", deps);
    expect(workspaceApi.openWorkspace).toHaveBeenCalledWith({ path: "/path/ws" });
    expect(shell.setWorkspace).toHaveBeenCalledWith("ws1", "WS One", "/path/ws", wsMeta());
    expect(shell.setPackOverviews).toHaveBeenCalled();
    expect(shell.addOpenPack).toHaveBeenCalledWith("p1", packMeta("p1"));
    expect(shell.setActivePack).toHaveBeenCalledWith("p1");
    expect((res as any).data.id).toBe("ws1");
  });

  it("skips packs that fail to open", async () => {
    const { deps, shell } = makeDeps();
    (workspaceApi.openWorkspace as any).mockResolvedValue(wsMeta());
    (packApi.openPack as any).mockRejectedValue(new Error("gone"));
    const res = await openWorkspace("/path/ws", deps);
    expect(shell.addOpenPack).not.toHaveBeenCalled();
    expect(shell.setActivePack).not.toHaveBeenCalled(); // last pack never opened
    expect(res.status).toBe("ok");
  });
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `npm run test -- workspaceCommands`
Expected: FAIL，无法解析 `./workspaceCommands`。

- [ ] **Step 3: 实现 workspaceCommands**

Create `src/features/commands/workspaceCommands.ts`:

```ts
import { workspaceApi } from "../../shared/api/workspaceApi";
import { packApi } from "../../shared/api/packApi";
import type { WorkspaceMeta } from "../../shared/contracts/workspace";
import type { CommandDeps, CommandResult } from "./types";
import { switchPack } from "./packCommands";

/**
 * Open the workspace at `path` and restore its pack session.
 * Shared by the UI (user picks a path) and agent (path from recent list).
 */
export async function openWorkspace(path: string, deps: CommandDeps): Promise<CommandResult<WorkspaceMeta>> {
  const meta = await workspaceApi.openWorkspace({ path });
  deps.shell.setWorkspace(meta.id, meta.name, path, meta);

  try {
    const overviews = await packApi.listPackOverviews();
    deps.shell.setPackOverviews(overviews);
  } catch {
    // overviews remain empty
  }

  for (const packId of meta.open_pack_ids) {
    try {
      const packMeta = await packApi.openPack({ packId });
      deps.shell.addOpenPack(packId, packMeta);
    } catch {
      // pack may no longer exist; skip
    }
  }

  if (meta.last_opened_pack_id && meta.open_pack_ids.includes(meta.last_opened_pack_id)) {
    await switchPack(meta.last_opened_pack_id, deps);
  }

  return { status: "ok", data: meta };
}

/** Switch to a known recent workspace by path. Same orchestration as openWorkspace. */
export async function switchWorkspace(path: string, deps: CommandDeps): Promise<CommandResult<WorkspaceMeta>> {
  return openWorkspace(path, deps);
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `npm run test -- workspaceCommands`
Expected: PASS，2 passed。

- [ ] **Step 5: 提交**

```bash
git add src/features/commands/workspaceCommands.ts src/features/commands/workspaceCommands.test.ts
git commit -m "feat: add workspace open/switch commands"
```

---

### Task 9: configCommands

**Files:**
- Create: `src/features/commands/configCommands.ts`
- Test: `src/features/commands/configCommands.test.ts`

抽自 [App.tsx:336-344](src/app/App.tsx#L336-L344) 的 `handleConfigSaved`。`updateConfig` 接受 `Partial<GlobalConfig>` patch，merge 进当前 config（来自 `deps.getConfig()`），保存后回写 store（`deps.setConfig`）并按需刷 standard-pack 缓存。

> **主题副作用归属**：`handleConfigSaved` 里还有 `applyThemeSettings`/`writeThemeMirror`（DOM 操作）。这些**不进命令层**（命令层与 React/DOM 无关）。UI 适配器在调 `updateConfig` 后自己补主题副作用（见 Task 10）；agent 改 config 一般不动主题字段，即使动了，下次 `App.tsx` 的 config effect 也会兜底应用（[App.tsx:84-91](src/app/App.tsx#L84-L91) 的 watchSystemTheme 与主题 effect）。

- [ ] **Step 1: 写失败测试**

Create `src/features/commands/configCommands.test.ts`:

```ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { updateConfig } from "./configCommands";
import type { CommandDeps } from "./types";
import type { GlobalConfig } from "../../shared/contracts/config";
import { configApi } from "../../shared/api/configApi";

vi.mock("../../shared/api/configApi", () => ({
  configApi: { saveConfig: vi.fn() },
}));

function makeConfig(): GlobalConfig {
  return {
    app_language: "en-US", ygopro_path: null, external_text_editor_path: null,
    custom_code_recommended_min: 100000000, custom_code_recommended_max: 199999999,
    custom_code_min_gap: 1, shell_sidebar_width: 150, shell_sidebar_collapsed: false,
    shell_right_sidebar_width: 320, shell_right_sidebar_collapsed: true,
    shell_window_width: 1280, shell_window_height: 800, shell_window_is_maximized: false,
    text_language_catalog: [], standard_pack_source_language: null, theme_mode: "system",
    high_contrast: false, custom_brand_color: null, deepseek_api_key: null, agent_language: "auto",
  };
}

function makeDeps(config: GlobalConfig | null) {
  const setConfig = vi.fn();
  const queryClient = { invalidateQueries: vi.fn() } as any;
  const deps: CommandDeps = {
    shell: {} as any, queryClient, getConfig: () => config, setConfig,
  };
  return { deps, setConfig, queryClient };
}

beforeEach(() => vi.clearAllMocks());

describe("updateConfig", () => {
  it("merges patch, saves, writes back to store", async () => {
    const cfg = makeConfig();
    const { deps, setConfig } = makeDeps(cfg);
    const saved = { ...cfg, agent_language: "zh-CN" as const };
    (configApi.saveConfig as any).mockResolvedValue(saved);
    const res = await updateConfig({ agent_language: "zh-CN" }, deps);
    expect(configApi.saveConfig).toHaveBeenCalledWith({ ...cfg, agent_language: "zh-CN" });
    expect(setConfig).toHaveBeenCalledWith(saved);
    expect((res as any).data.agent_language).toBe("zh-CN");
  });

  it("invalidates standard-pack caches when source language changes", async () => {
    const cfg = makeConfig();
    const { deps, queryClient } = makeDeps(cfg);
    (configApi.saveConfig as any).mockResolvedValue({ ...cfg, standard_pack_source_language: "ja-JP" });
    await updateConfig({ standard_pack_source_language: "ja-JP" }, deps);
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({ queryKey: ["standard-pack-status"] });
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({ queryKey: ["standard-cards"] });
  });

  it("throws if no config is loaded", async () => {
    const { deps } = makeDeps(null);
    await expect(updateConfig({ agent_language: "zh-CN" }, deps)).rejects.toThrow();
  });
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `npm run test -- configCommands`
Expected: FAIL，无法解析 `./configCommands`。

- [ ] **Step 3: 实现 configCommands**

Create `src/features/commands/configCommands.ts`:

```ts
import { configApi } from "../../shared/api/configApi";
import type { GlobalConfig } from "../../shared/contracts/config";
import type { CommandDeps, CommandResult } from "./types";

export async function updateConfig(
  patch: Partial<GlobalConfig>,
  deps: CommandDeps,
): Promise<CommandResult<GlobalConfig>> {
  const current = deps.getConfig();
  if (!current) {
    throw new Error("Config is not loaded yet.");
  }
  const merged: GlobalConfig = { ...current, ...patch };
  const saved = await configApi.saveConfig(merged);
  deps.setConfig(saved);

  // Standard-pack reference data depends on the source language; refresh on change.
  if (patch.standard_pack_source_language !== undefined) {
    void deps.queryClient.invalidateQueries({ queryKey: ["standard-pack-status"] });
    void deps.queryClient.invalidateQueries({ queryKey: ["standard-cards"] });
  }
  return { status: "ok", data: saved };
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `npm run test -- configCommands`
Expected: PASS，3 passed。

- [ ] **Step 5: 提交**

```bash
git add src/features/commands/configCommands.ts src/features/commands/configCommands.test.ts
git commit -m "feat: add updateConfig command"
```

---
### Task 10: UI 适配器 useCommands

**Files:**
- Create: `src/features/commands/useCommands.ts`

在 React 内组装 `deps`（`useQueryClient` + shellStore actions + configStore），返回绑好 deps 的命令。破坏性命令用 helper 接到 `openDialog`（[shellStore.ts:120](src/shared/stores/shellStore.ts#L120)，`ConfirmDialogState`）。config 写操作在命令返回后补主题副作用（见 Task 9 注）。

> 此 task 无单元测试（React hook + store 集成，由 spec §9 手动验证覆盖）。只需 typecheck 通过。

- [ ] **Step 1: 实现 useCommands**

Create `src/features/commands/useCommands.ts`:

```ts
import { useQueryClient } from "@tanstack/react-query";
import { useShellStore } from "../../shared/stores/shellStore";
import { useConfigStore } from "../../shared/stores/configStore";
import type { CreatePackInput, UpdatePackMetadataInput } from "../../shared/api/packApi";
import type { GlobalConfig } from "../../shared/contracts/config";
import type { CommandDeps } from "./types";
import * as packCommands from "./packCommands";
import * as workspaceCommands from "./workspaceCommands";
import { updateConfig as updateConfigCommand } from "./configCommands";
import { applyThemeSettings, writeThemeMirror } from "../../shared/theme/theme";

/** Build CommandDeps from React/store context. */
function useCommandDeps(): CommandDeps {
  const queryClient = useQueryClient();
  const setActivePack = useShellStore((s) => s.setActivePack);
  const addOpenPack = useShellStore((s) => s.addOpenPack);
  const removeOpenPack = useShellStore((s) => s.removeOpenPack);
  const updatePackMetadata = useShellStore((s) => s.updatePackMetadata);
  const setPackOverviews = useShellStore((s) => s.setPackOverviews);
  const setWorkspace = useShellStore((s) => s.setWorkspace);
  const setConfig = useConfigStore((s) => s.setConfig);

  return {
    shell: { setActivePack, addOpenPack, removeOpenPack, updatePackMetadata, setPackOverviews, setWorkspace },
    queryClient,
    getConfig: () => useConfigStore.getState().config,
    setConfig,
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

    openWorkspace: (path: string) => workspaceCommands.openWorkspace(path, deps),
    switchWorkspace: (path: string) => workspaceCommands.switchWorkspace(path, deps),

    /** Update config, then apply theme side-effects (DOM) that the command layer omits. */
    updateConfig: async (patch: Partial<GlobalConfig>) => {
      const result = await updateConfigCommand(patch, deps);
      if (result.status === "ok") {
        const settings = {
          mode: result.data.theme_mode,
          highContrast: result.data.high_contrast,
          customBrandColor: result.data.custom_brand_color,
        };
        applyThemeSettings(settings);
        writeThemeMirror(settings);
      }
      return result;
    },

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

> 若 `theme.ts` 的 `applyThemeSettings`/`writeThemeMirror` 导入路径或签名不符，按 [App.tsx:29](src/app/App.tsx#L29) 的实际导入修正（它们已在 App.tsx 被使用，签名以 `themeSettingsFromConfig` 的返回对象为入参）。

- [ ] **Step 3: 提交**

```bash
git add src/features/commands/useCommands.ts
git commit -m "feat: add useCommands UI adapter"
```

---

### Task 11: agent 适配器 buildAgentDeps

**Files:**
- Create: `src/features/commands/buildAgentDeps.ts`

从 `useShellStore.getState()` / `useConfigStore.getState()` + 模块级 `queryClient` 组装 deps（spec §6）。非 React，可在工具 execute 内调用。

- [ ] **Step 1: 实现 buildAgentDeps**

Create `src/features/commands/buildAgentDeps.ts`:

```ts
import { useShellStore } from "../../shared/stores/shellStore";
import { useConfigStore } from "../../shared/stores/configStore";
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
      setWorkspace: shell.setWorkspace,
    },
    queryClient,
    getConfig: () => useConfigStore.getState().config,
    setConfig: (next) => useConfigStore.getState().setConfig(next),
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

### Task 12: 方案 B 确认门 — 放宽确认类型

**Files:**
- Modify: `src/shared/stores/agentStore.ts:6-15`
- Modify: `src/features/agent/agentLoop.ts:16-23`

命令确认（commit 闭包）没有后端 token。把两处 `confirmationToken` 放宽为 `string | null`，使 token 流与 commit 流共用同一 UI 通道（设计决策 1）。

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
Expected: PASS（放宽是向后兼容的；现有传 `string` 的调用仍合法）。

- [ ] **Step 4: 提交**

```bash
git add src/shared/stores/agentStore.ts src/features/agent/agentLoop.ts
git commit -m "refactor: widen confirmationToken to allow command confirmations"
```

---

### Task 13: 方案 B 确认门 — loop 识别命令确认

**Files:**
- Modify: `src/features/agent/agentLoop.ts:83-116` (runToolCall), 新增 helper

在 `runToolCall` 中新增一条与 `isWriteResult` 并列的分支：识别命令层的 `{ status: "needs_confirmation", confirmation: { summary, commit } }`，走 `hooks.requestConfirmation`（token/warnings/preview 传 null/[]），确认后调 `commit()`。

- [ ] **Step 1: 加 CommandResult 识别函数**

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

Modify `runToolCall`（[agentLoop.ts:103-115](src/features/agent/agentLoop.ts#L103-L115)）的 `try` 块。把现有：

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

    // Command-layer confirmation (delete_pack etc.): commit-closure model.
    if (!tool.readOnly && isCommandConfirmation(result)) {
      return handleCommandConfirmation(result, call, hooks);
    }
    // Card write tools: backend two-phase token model.
    if (!tool.readOnly && isWriteResult(result)) {
      return handleWriteResult(result, call, args, hooks);
    }
    // Command-layer ok results: unwrap to data for the model.
    if (!tool.readOnly && isCommandResultOk(result)) {
      return JSON.stringify({ status: "ok", data: (result as { data: unknown }).data ?? null });
    }
    return JSON.stringify(result ?? null);
```

并在 Step 1 的 helper 旁追加：

```ts
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

- [ ] **Step 4: 类型检查**

Run: `npm run typecheck`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add src/features/agent/agentLoop.ts
git commit -m "feat: handle command-layer confirmation in agent loop"
```

---
### Task 14: agent 写工具 + list_recent_workspaces 读工具

**Files:**
- Create: `src/features/agent/tools/commandTools.ts`

各工具调对应命令（通过 `buildAgentDeps()`）。沿用 [writeTools.ts](src/features/agent/tools/writeTools.ts) 的 `AgentTool` 形态。`delete_pack` 返回命令的 `needs_confirmation` 结果，由 Task 13 的 loop 分支处理。`list_recent_workspaces` 是只读工具（设计决策 3）。

> 工具 execute 的返回值：非破坏性命令返回 `CommandResult<T>`（loop 的 `isCommandResultOk` 分支会 unwrap）；`delete_pack` 返回 `CommandResult` 的 needs_confirmation 分支（loop 的 `isCommandConfirmation` 分支处理）。`switch_workspace` 需把 agent 给的 workspace 标识解析成路径——从 `workspaceApi.listRecentWorkspaces()` 按 path 或 name_cache 匹配。

- [ ] **Step 1: 实现 commandTools（读工具 + 简单写工具）**

Create `src/features/agent/tools/commandTools.ts`:

```ts
import { workspaceApi } from "../../../shared/api/workspaceApi";
import { useShellStore } from "../../../shared/stores/shellStore";
import { buildAgentDeps } from "../../commands/buildAgentDeps";
import * as packCommands from "../../commands/packCommands";
import * as workspaceCommands from "../../commands/workspaceCommands";
import { updateConfig } from "../../commands/configCommands";
import type { AgentTool } from "./types";
import { ToolError } from "./types";

export const listRecentWorkspacesTool: AgentTool = {
  name: "list_recent_workspaces",
  description:
    "List the user's recent workspaces (name and path). Call this before switch_workspace " +
    "to discover which workspaces the user can switch to.",
  readOnly: true,
  parameters: { type: "object", properties: {} },
  async execute() {
    const registry = await workspaceApi.listRecentWorkspaces();
    return registry.workspaces.map((w) => ({
      path: w.path,
      name: w.name_cache,
      last_opened_at: w.last_opened_at,
    }));
  },
};

export const switchWorkspaceTool: AgentTool = {
  name: "switch_workspace",
  description:
    "Switch to one of the user's recent workspaces, identified by its path or name. " +
    "Use list_recent_workspaces first to see the options. Opens the workspace and restores its packs.",
  readOnly: false,
  parameters: {
    type: "object",
    properties: {
      workspace: {
        type: "string",
        description: "The workspace path (preferred) or its display name, from list_recent_workspaces.",
      },
    },
    required: ["workspace"],
  },
  async execute(args) {
    const target = String(args.workspace);
    const registry = await workspaceApi.listRecentWorkspaces();
    const match =
      registry.workspaces.find((w) => w.path === target) ??
      registry.workspaces.find((w) => w.name_cache === target);
    if (!match) {
      throw new ToolError(
        `No recent workspace matches "${target}". Use list_recent_workspaces to see valid options.`,
      );
    }
    return workspaceCommands.switchWorkspace(match.path, buildAgentDeps());
  },
};

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

- [ ] **Step 2: 追加 create_pack / update_pack_meta / delete_pack 工具**

在 `commandTools.ts` 末尾追加：

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
    return packCommands.createPack(
      {
        name: String(args.name),
        author: String(args.author),
        version: String(args.version),
        packCode: typeof args.packCode === "string" ? args.packCode : null,
        description: typeof args.description === "string" ? args.description : null,
        displayLanguageOrder: ["en-US"],
        defaultExportLanguage: "en-US",
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
        packCode:
          typeof args.packCode === "string" ? args.packCode : current.pack_code,
        description:
          typeof args.description === "string" ? args.description : current.description,
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

export const updateConfigTool: AgentTool = {
  name: "update_config",
  description:
    "Update the user's global settings (e.g. custom card code range/gap, standard pack source " +
    "language, agent reply language). Only pass the fields you want to change. Never sets secrets.",
  readOnly: false,
  parameters: {
    type: "object",
    properties: {
      custom_code_recommended_min: { type: "integer" },
      custom_code_recommended_max: { type: "integer" },
      custom_code_min_gap: { type: "integer" },
      standard_pack_source_language: { type: "string" },
      agent_language: { type: "string" },
      app_language: { type: "string" },
    },
  },
  async execute(args) {
    const patch: Record<string, unknown> = {};
    for (const key of [
      "custom_code_recommended_min",
      "custom_code_recommended_max",
      "custom_code_min_gap",
      "standard_pack_source_language",
      "agent_language",
      "app_language",
    ]) {
      if (args[key] !== undefined) patch[key] = args[key];
    }
    return updateConfig(patch, buildAgentDeps());
  },
};
```

- [ ] **Step 3: 类型检查**

Run: `npm run typecheck`
Expected: PASS。

- [ ] **Step 4: 提交**

```bash
git add src/features/agent/tools/commandTools.ts
git commit -m "feat: add agent command tools"
```

---

### Task 15: 注册工具 + 更新 system prompt

**Files:**
- Modify: `src/features/agent/tools/registry.ts:1-25`
- Modify: `src/features/agent/systemPrompt.ts:13-24`

- [ ] **Step 1: 注册新工具**

Modify `src/features/agent/tools/registry.ts`。在 import 块（[registry.ts:11](src/features/agent/tools/registry.ts#L11) 后）加：

```ts
import { createCardTool, moveCardsTool, updateCardTool } from "./writeTools";
import {
  listRecentWorkspacesTool,
  switchWorkspaceTool,
  switchPackTool,
  openPackTool,
  closePackTool,
  createPackTool,
  updatePackMetaTool,
  deletePackTool,
  updateConfigTool,
} from "./commandTools";
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
  listRecentWorkspacesTool,
  createCardTool,
  updateCardTool,
  moveCardsTool,
  switchWorkspaceTool,
  switchPackTool,
  openPackTool,
  closePackTool,
  createPackTool,
  updatePackMetaTool,
  deletePackTool,
  updateConfigTool,
];
```

- [ ] **Step 2: 更新 system prompt**

Modify `src/features/agent/systemPrompt.ts`。把 "## Writing changes" 段（[systemPrompt.ts:17-20](src/features/agent/systemPrompt.ts#L17-L20)）后追加一个新段（在 `## Style` 之前）：

```ts
## Managing workspaces, packs, and settings
- You can switch workspaces (switch_workspace — use list_recent_workspaces first to see options), open/switch/close packs (open_pack / switch_pack / close_pack), create packs (create_pack), edit pack metadata (update_pack_meta), and change settings (update_config). These take effect in the UI immediately.
- delete_pack is destructive: it asks the user to confirm in the chat before deleting. If the user cancels, nothing happens.
- update_config only changes the fields you pass. Never attempt to set secrets like the API key.
```

- [ ] **Step 3: 类型检查 + 全部测试**

Run: `npm run typecheck && npm run test`
Expected: 均 PASS。

- [ ] **Step 4: 提交**

```bash
git add src/features/agent/tools/registry.ts src/features/agent/systemPrompt.ts
git commit -m "feat: register agent command tools and update system prompt"
```

---
### Task 16: App.tsx — config 真相源迁移到 configStore

**Files:**
- Modify: `src/app/App.tsx:46-57` (state), `:124-149` (bootstrap), `:80-99` (effects)
- Modify: `src/app/hooks/useAppWindow.ts:12-15`, `:52-54`

spec §3 最小改造：把 config 的 `useState` 换成 configStore 读写，保留现有 props 透传。`useAppWindow` 的 `setConfig` 参数当前是 `Dispatch<SetStateAction>` 且内部用函数式更新（[useAppWindow.ts:54](src/app/hooks/useAppWindow.ts#L54)），需改为普通 setter。

> 三个 hook（useSidebarResize/useRightSidebarResize 已是 `(c: GlobalConfig) => void`；useAppWindow 是 `Dispatch<SetStateAction>`）。统一传 configStore 的 `setConfig`，并改掉 useAppWindow 里唯一的函数式调用。

- [ ] **Step 1: 改 useAppWindow 的 setConfig 类型与函数式调用**

Modify `src/app/hooks/useAppWindow.ts`：

把 [useAppWindow.ts:1-2](src/app/hooks/useAppWindow.ts#L1-L2) 的 import 去掉 `Dispatch, SetStateAction`（保留 `MutableRefObject`）：

```ts
import { useEffect, useState } from "react";
import type { MutableRefObject } from "react";
```

把签名（[useAppWindow.ts:12-15](src/app/hooks/useAppWindow.ts#L12-L15)）改为：

```ts
export function useAppWindow(
  configRef: MutableRefObject<GlobalConfig | null>,
  setConfig: (config: GlobalConfig) => void,
) {
```

把函数式调用（[useAppWindow.ts:54](src/app/hooks/useAppWindow.ts#L54)）`setConfig((prev) => prev ?? currentConfig);` 改为：

```ts
          if (!configRef.current) setConfig(currentConfig);
```

（`configRef.current` 已在上一行赋值为 `currentConfig`，故这里判断改用一个局部标志更稳妥。最简实现：保留上方 `configRef.current = currentConfig;`，把本行改为 `setConfig(currentConfig);` —— 重复 set 同值在 zustand 中无害，且 bootstrap 已先 set。）

实际写：

```ts
          configRef.current = currentConfig;
          setConfig(currentConfig);
```

- [ ] **Step 2: App.tsx 用 configStore 替换 config useState**

Modify `src/app/App.tsx`：

import 块加（[App.tsx:3](src/app/App.tsx#L3) 附近）：

```ts
import { useConfigStore } from "../shared/stores/configStore";
```

把 [App.tsx:47](src/app/App.tsx#L47) `const [config, setConfig] = useState<GlobalConfig | null>(null);` 改为：

```ts
  const config = useConfigStore((s) => s.config);
  const setConfig = useConfigStore((s) => s.setConfig);
```

- [ ] **Step 3: 修正 setConfig 在 App 内的调用兼容性**

`useConfigStore` 的 `setConfig` 是 `(next: GlobalConfig) => void`，不接受 updater。检查 App.tsx 里所有 `setConfig(...)` 调用：[App.tsx:135](src/app/App.tsx#L135)（`setConfig(nextConfig)`）、[App.tsx:341](src/app/App.tsx#L341)（`setConfig(nextConfig)`）均传具体值，兼容。`AppShell` props 里 `setConfig` 的类型声明（[App.tsx:284](src/app/App.tsx#L284)）需从 `Dispatch<SetStateAction<GlobalConfig | null>>` 改为 `(next: GlobalConfig) => void`：

```ts
  setConfig: (next: GlobalConfig) => void;
```

- [ ] **Step 4: 类型检查**

Run: `npm run typecheck`
Expected: PASS。（若 `useAppWindow`/`useSidebarResize`/`useRightSidebarResize` 调用点因类型变化报错，确认它们现在都接收同一个 `(c: GlobalConfig) => void` setter。）

- [ ] **Step 5: 提交**

```bash
git add src/app/App.tsx src/app/hooks/useAppWindow.ts
git commit -m "refactor: move config truth source to configStore"
```

---

### Task 17: App.tsx — currentWorkspace 改从 shellStore 派生

**Files:**
- Modify: `src/app/App.tsx:49`, `:159-162`, `:359-361`, `:224-253` (props)

设计决策 2：currentWorkspace 真相源移入 shellStore（Task 4 已加 `workspaceMeta`）。`App` 从 store 派生 `currentWorkspace`，删掉本地 `useState`。

> `CurrentWorkspaceRef` 形状是 `{ meta, path }`。store 现有 `workspaceMeta` + `workspacePath` 即可重建。`setCurrentWorkspace` 的调用全部由 `setWorkspace` 取代（Task 4 已让 `setWorkspace` 存 meta）。

- [ ] **Step 1: 派生 currentWorkspace**

Modify `src/app/App.tsx`。删掉 [App.tsx:49](src/app/App.tsx#L49) `const [currentWorkspace, setCurrentWorkspace] = useState<CurrentWorkspaceRef | null>(null);`，改为从 store 派生（在 store 选择器区，[App.tsx:75-78](src/app/App.tsx#L75-L78) 附近）：

```ts
  const workspaceMeta = useShellStore((s) => s.workspaceMeta);
  const workspacePath = useShellStore((s) => s.workspacePath);
  const currentWorkspace: CurrentWorkspaceRef | null =
    workspaceMeta && workspacePath ? { meta: workspaceMeta, path: workspacePath } : null;
```

- [ ] **Step 2: 删除 setCurrentWorkspace 调用**

Modify `src/app/App.tsx`。`tryRestoreLastSession` 里 [App.tsx:161](src/app/App.tsx#L161) `setCurrentWorkspace({ meta, path: lastEntry.path });` 删除（下一行的 `setWorkspace(...meta)` 已存 meta，Task 4 Step 5 已改为 4 参）。

`handleWorkspaceOpened` 里 [App.tsx:360](src/app/App.tsx#L360) `setCurrentWorkspace({ meta, path });` 删除（同理，下方 setWorkspace 已存）。

- [ ] **Step 3: 删除 props 透传中的 setCurrentWorkspace**

Modify `src/app/App.tsx`：
- 删 [App.tsx:233](src/app/App.tsx#L233) 传参 `setCurrentWorkspace={setCurrentWorkspace}`。
- 删 `AppShell` 解构（[App.tsx:262](src/app/App.tsx#L262) 附近）的 `setCurrentWorkspace,`。
- 删 props 类型声明（[App.tsx:286](src/app/App.tsx#L286)）的 `setCurrentWorkspace: ...;` 一行。
- `AppShell` 内 `currentWorkspace` 仍作为 prop 传入（[App.tsx:232](src/app/App.tsx#L232) `currentWorkspace={currentWorkspace}`）—— 保留，它现在来自派生值。

- [ ] **Step 4: 处理 WorkspaceModal 的 onWorkspaceOpened**

`handleWorkspaceOpened`（[App.tsx:359](src/app/App.tsx#L359)）现在不再需要自己 `setCurrentWorkspace`。Task 18 会把它整体替换为命令转调，此处只需保证删 `setCurrentWorkspace` 后仍编译（`setWorkspace` 调用保留）。

- [ ] **Step 5: 类型检查**

Run: `npm run typecheck`
Expected: PASS。

- [ ] **Step 6: 提交**

```bash
git add src/app/App.tsx
git commit -m "refactor: derive currentWorkspace from shellStore"
```

---

### Task 18: App.tsx — handler 收敛为转调 useCommands

**Files:**
- Modify: `src/app/App.tsx:304-421` (AppShell handlers)

把 `AppShell` 里**自身持有完整编排**的 handler 收敛为调 `useCommands`，保留 notice 包装（命令不吞错，handler 负责 catch + notice）。

> **v1 收敛范围（明确边界）**：只收敛 `persistActivePack`→`switchPack` 与 `handleClosePack`→`closePack`。这两个 handler 的整条"后端调用 + store 同步"链都在 App.tsx 内，可干净抽走。
>
> **不收敛的 handler 及原因**：`handlePackOpened`/`handlePackCreated`/`handleWorkspaceOpened` 由 AddPackModal/WorkspaceModal 在**自己已调用后端 API 之后**回调（传入已得的 metadata），它们只负责 store 同步。若改为调命令会**二次调用后端**。要正确收敛须把后端调用从 Modal 移进命令、Modal 改调命令——这是 Modal 调用链重构，超出 spec §7 划定的 v1 边界。`handleConfigSaved` 同理（SettingsModal 已 `saveConfig`），仅保持其 `setConfig` 现在指向 configStore（Task 16 已完成），本 task 不动它。`handlePackDeleted` 是 PackMetadataPanel 删除后的善后回调，独立入口，保留。
>
> 命令层的平权价值在 agent 侧（Task 14）已完整落地；UI 侧收敛是渐进的，本 task 只做无歧义的两个。

- [ ] **Step 1: 在 AppShell 顶部取 useCommands**

Modify `src/app/App.tsx`，在 `AppShell` 函数体内（[App.tsx:304](src/app/App.tsx#L304) `const { t } = useAppI18n();` 后）加：

```ts
  const commands = useCommands();
```

并在 import 块加：

```ts
import { useCommands } from "../features/commands/useCommands";
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

- [ ] **Step 4: 移除 AppShell 内不再使用的 setActivePack**

收敛后 `persistActivePack` 不再直接调 `setActivePack`（[App.tsx:311](src/app/App.tsx#L311) 的 `const setActivePack = useShellStore((s) => s.setActivePack);`）。检查 `AppShell` 内是否还有其他 `setActivePack` 用处；若无，删除该选择器行以免 typecheck 报未使用变量（项目 tsc 配置若开启 `noUnusedLocals` 会报错）。

> 若 typecheck 不报未使用变量错，可跳过本步。删除前用 Grep 确认 `AppShell` 函数体内 `setActivePack` 仅 `persistActivePack` 一处引用。

- [ ] **Step 5: 类型检查 + 测试**

Run: `npm run typecheck && npm run test`
Expected: 均 PASS。

- [ ] **Step 6: 提交**

```bash
git add src/app/App.tsx
git commit -m "refactor: converge active-pack and close-pack handlers to commands"
```

---

### Task 19: 手动验证 + 同步 docs/agent.md

**Files:**
- Modify: `docs/agent.md`

spec §9 验证。先跑自动检查，再手动验证 agent 操作，最后同步文档。

- [ ] **Step 1: 全量自动验证**

Run: `npm run typecheck && npm run test && npm run build`
Expected: 三者均 PASS（build 含 tsc + vite build）。

- [ ] **Step 2: 手动验证 agent 操作平权（spec §9）**

Run: `npm run dev`，在 agent 面板依次验证：
- "切换到 <最近 workspace 名>" → 标题栏 workspace 名即时变化。
- "打开 <pack 名>" / "切换到 <pack>" → 侧栏与卡片列表即时反映。
- "新建一个叫 X 的卡包" → 卡包出现并激活。
- "把当前卡包改名为 Y" → pack metadata 即时更新。
- "把 agent 回复语言改成中文"（update_config）→ 设置生效。
- "删除卡包 Z" → agent 面板内联弹确认；点取消 → 卡包仍在；再删 → 点确认 → 卡包消失。

Expected: 每项 UI 立即反映（store/缓存同步），删除取消时不执行。

- [ ] **Step 3: 验证 UI 与 agent 走同一命令**

手动在 UI 切 pack、关 pack，确认行为与 agent 操作一致（同一命令、同一副作用）。

- [ ] **Step 4: 同步 docs/agent.md**

读 `docs/agent.md` 当前内容，补充：新工具清单（switch_workspace / list_recent_workspaces / switch_pack / open_pack / close_pack / create_pack / update_pack_meta / delete_pack / update_config）、命令层架构（纯函数 + deps 注入 + 双适配器）、方案 B 确认门（commit 闭包 vs 后端 token 并存）。具体文案依 `docs/agent.md` 现有结构补写，不复述历史计划。

- [ ] **Step 5: 提交**

```bash
git add docs/agent.md
git commit -m "docs: sync agent.md with command layer and new tools"
```

---

## Self-Review

**1. Spec coverage:**
- §2 命令层纯函数 + deps → Task 2/6/7/8/9 ✓
- §3 config 提升 store → Task 3/16 ✓
- §4 命令清单（workspace/pack/config）→ Task 6/7/8/9；导入导出明确非目标 ✓
- §5 确认门（前端 commit 闭包）→ Task 7/12/13；UI 侧 deletePackWithDialog → Task 10 ✓
- §6 适配器 + agent 工具 + 注册 + system prompt → Task 10/11/14/15 ✓；queryClient 模块级 → Task 5 ✓
- §7 非目标 → 计划未触碰导入导出/删 workspace/CardList 视图状态/checkedCards/撤销 ✓
- §8 涉及文件 → 全部出现在对应 task ✓
- §9 验证 → Task 19 ✓
- **补充超出 spec 但必要**：currentWorkspace 上移（决策 2）、list_recent_workspaces 工具（决策 3）—— 均在架构说明中标注理由。

**2. Placeholder scan:** 无 TBD/TODO；所有 code step 含完整代码；测试含真实断言。Task 18 的 Step 3/5/6 经分析后判定为"不收敛"并给出理由，非占位——这是真实的 v1 边界结论（避免二次后端调用）。

**3. Type consistency:**
- `setWorkspace(id, name, path, meta)` 4 参签名：types.ts(Task 2) / shellStore(Task 4) / 调用点(Task 4 Step 5, Task 8) 一致 ✓
- `CommandResult<T>` 的 `ok`/`needs_confirmation` 分支：types(Task 2) / 各命令(Task 6-9) / loop 识别(Task 13) 一致 ✓
- `ConfirmationRequest`：命令层 `{summary, commit}`(Task 2) 与 loop 层 `ConfirmationRequest`(token 形状，Task 12) 是**两个不同类型**，同名但分属命令层与 agentLoop——已在架构说明区分，无碰撞（不同文件作用域）✓
- `confirmationToken: string | null`：agentStore(Task 12) / agentLoop(Task 12) / handleCommandConfirmation 传 null(Task 13) 一致 ✓
- `CommandDeps.shell` 子集与 useCommands(Task 10)/buildAgentDeps(Task 11) 组装字段一致 ✓






