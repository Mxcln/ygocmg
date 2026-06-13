# Agent 操作平权（命令层）设计

> 状态：设计稿，待评审。落地稳定后同步进核心文档 `docs/agent.md`。
>
> 本文是更大构想"agent 对整个 YGOCMG 功能平权"拆分出的**子项目 A**。子项目 B（全 app 撤销/历史/审计）独立设计，不在本文范围。

## 1. 背景与目标

当前 AI Agent 只能查询和修改卡片。用户希望 agent 能做用户能做的几乎一切：切换 workspace、切换 pack、改 pack metadata、改 config、增删开关 pack 等。

缺口不在"缺工具"，而在**编排逻辑困在 React 组件里**。以"切 pack"为例，它不是一次后端调用，而是「调 `packApi.setActivePack` + 更新 `shellStore` + 刷新 React Query 缓存」三件事的编排。这套逻辑现在是 `App.tsx`（`AppShell`）的组件闭包（`handlePackOpened`、`handleClosePack`、`handleConfigSaved`、`persistActivePack` 等），agent 够不着，只能调最底层 api，导致 store/缓存与 agent 操作脱节。

**目标**：建立一个 UI 与 agent 共用的**命令层**——确定性的操作编排，让两者通过各自的薄适配器调用同一套逻辑，实现操作平权。

### 业界依据

调研（CQRS for agents、Compensating Action 模式、HITL 确认）支持本设计：读/写分离、业务规则留在确定性代码、破坏性操作需人工确认、撤销靠"补偿动作 + provenance 账本"（属子项目 B，本文不做）。命令层的纯函数形态天然为 B 阶段给每个命令配补偿动作留好接口。

## 2. 命令层形态：纯函数 + 注入依赖

每个命令是**与 React 无关的纯函数**，签名 `(...args, deps: CommandDeps) => Promise<CommandResult<T>>`。命令不持有 React 状态，所有副作用句柄通过 `deps` 注入。同一函数因此能在 UI 和 agent 两种环境运行——这是平权的根。

```ts
// src/features/commands/types.ts (示意)
export interface CommandDeps {
  shell: ShellStoreApi;        // shellStore 的 actions（setActivePack/addOpenPack/removeOpenPack...）
  queryClient: QueryClient;    // invalidateQueries 刷缓存
  getConfig: () => GlobalConfig;
  setConfig: (next: GlobalConfig) => void;
}

export type CommandResult<T> =
  | { status: "ok"; data: T }
  | { status: "needs_confirmation"; confirmation: ConfirmationRequest };

export interface ConfirmationRequest {
  summary: string;             // 人类可读的操作摘要
  commit: () => Promise<unknown>; // 确认后真正执行的闭包
}
```

排除的备选：Zustand 命令 store（把 queryClient 塞进 store 是反模式）、事件总线（fire-and-forget 难传错误与确认结果，且撤销已拆到 B）。

## 3. config 提升进 Zustand store

`config` 现在困在 `App.tsx` 的 `useState`，agent 读不到、写不回。新建 `configStore`（与 shellStore 平级）：

```ts
// src/shared/stores/configStore.ts (示意)
interface ConfigState {
  config: GlobalConfig | null;
  setConfig: (next: GlobalConfig) => void;
}
export const useConfigStore = create<ConfigState>()((set) => ({
  config: null,
  setConfig: (next) => set({ config: next }),
}));
```

**最小改造（取舍 a）**：只把"真相源"从 `App.tsx` 的 useState 挪到 store；现有组件仍通过 props 接收 config（不重构整条传递链，YAGNI）。agent 命令通过 `getConfig: () => useConfigStore.getState().config` 拿最新值，改 config 走 `configCommands.updateConfig`。

## 4. 命令清单与签名（v1 范围）

v1 覆盖同步操作；**导入/导出暂缓**（两阶段 + 后台 job + 进度轮询，异步语义单列一轮）。命令按领域分文件。

**workspaceCommands.ts**
- `openWorkspace(path, deps)` — `workspaceApi.openWorkspace` + `listPackOverviews` + 逐个 `openPack`/`addOpenPack` + 刷缓存。抽自 `handleWorkspaceOpened`。
- `switchWorkspace(path, deps)` — 切到另一个最近 workspace，编排同上。

> **agent 暴露范围**：`openWorkspace` 接受任意文件系统路径，agent 没有文件选择器、也不应臆造路径，故**不做 agent 工具**，仅供 UI 适配器（用户选目录后）调用。agent 侧只暴露 `switch_workspace`（切到一个**已知的最近 workspace**，按其在最近列表中的标识/路径）。两者底层可共用 `openWorkspace` 编排，区别仅在路径来源。

**packCommands.ts**
- `switchPack(packId, deps)` — `setActivePack` + `packApi.setActivePack` + 刷 `["cards"]`。抽自 `persistActivePack`。
- `openPack(packId, deps)` — `packApi.openPack` + `addOpenPack` + 刷 overviews。抽自 `handlePackOpened`。
- `createPack(input, deps)` — `packApi.createPack` + `addOpenPack` + 刷 overviews。抽自 `handlePackCreated`。
- `closePack(packId, deps)` — `packApi.closePack` + `removeOpenPack`。抽自 `handleClosePack`。
- `updatePackMeta(input, deps)` — `packApi.updatePackMetadata` + `updatePackMetadata`(store) + 刷 overviews。
- `deletePack(packId, deps)` — **破坏性**，返回 `needs_confirmation`；确认后 `packApi.deletePack` + `removeOpenPack` + 刷 overviews。

**configCommands.ts**
- `updateConfig(patch, deps)` — `configApi.saveConfig`(merge patch) + `setConfig`(store) + 按需刷 standard-pack 相关缓存。抽自 `handleConfigSaved`。

删 workspace 不在 v1（可后续单列）。

## 5. 确认门机制

破坏性命令（v1 仅 `deletePack`）的确认由**前端命令层发起**，因为后端 `deletePack` 没有卡片写入那样的两段式 token。

1. 命令检测到破坏性操作 → 不立即执行，返回 `{ status: "needs_confirmation", confirmation: { summary, commit } }`。
2. 适配器弹确认 UI：**UI 适配器**走现有 `AppDialog`/`openDialog`；**agent 适配器**走 `useAgentLoop` 已有的 `requestConfirmation`（对话流内联卡片）。
3. 用户确认后，适配器调 `confirmation.commit()` 真正执行。
4. 非破坏性命令直接返回 `{status:"ok"}`，不触发这套。

**两套确认门并存，不强行统一**：卡片/strings 写入用后端 token（已有）；命令层破坏性操作用前端 commit 闭包（新增）。设计中写明差异，避免误以为可复用。

## 6. 适配器与 agent 工具

**UI 适配器 `useCommands()`**（`src/features/commands/useCommands.ts`）：在 React 内组装 `deps`（`useQueryClient()` + `useShellStore` + `useConfigStore`），返回绑好 deps 的命令；破坏性命令用一个 helper 接到 `openDialog`。`App.tsx` 现有 handler 收敛为一行转调。

**agent 适配器 `buildAgentDeps(queryClient)`**（`src/features/commands/buildAgentDeps.ts`）：从 `useShellStore.getState()`/`useConfigStore.getState()` + 模块级 queryClient 单例组装 deps。前提：把 React Query 的 `queryClient` 提为模块级单例导出，使非 React 代码可访问。

**新增 agent 写工具**（`src/features/agent/tools/`，`readOnly: false`）：`switch_workspace`、`switch_pack`、`open_pack`、`create_pack`、`close_pack`、`update_pack_meta`、`update_config`，各自调对应命令。`delete_pack` 破坏性：命令返回 `needs_confirmation` 时走 `useAgentLoop` 的 `requestConfirmation`，确认后调 `confirmation.commit`。注册进 `registry.ts` 的 `AGENT_TOOLS`；system prompt 增补这些工具的用途与"破坏性操作会要确认"。

## 7. 非目标（本次不做）

- **导入/导出命令**：两阶段 + job + 进度，异步语义单列一轮。
- **删 workspace**：v1 不含。
- **CardList 的 filter / pageSize / 排序 / 分页 / 搜索框**：纯本地视图状态，保持组件 `useState`；agent 用 `list_cards` 的查询参数达成等效目的，不操纵 UI 视图。
- **勾选（checkedCards）**：已是 `shellStore` 状态镜像（agent 读不写）；agent 批量操作走带 id 的工具（如 `move_cards`），不模拟勾选 UI。
- **撤销/历史/审计（子项目 B）**：命令层为其留接口（未来每命令配补偿动作），本轮不实现。
- **config 传递链重构**：只换真相源到 store，保留现有 props 透传。

## 8. 涉及文件

**新建：**
- `src/features/commands/types.ts` — `CommandDeps`、`CommandResult`、`ConfirmationRequest`。
- `src/features/commands/workspaceCommands.ts` / `packCommands.ts` / `configCommands.ts`。
- `src/features/commands/useCommands.ts` — UI 适配器。
- `src/features/commands/buildAgentDeps.ts` — agent 适配器。
- `src/shared/stores/configStore.ts` — config store。
- `src/features/agent/tools/` 下新增写工具文件。

**修改：**
- `src/app/App.tsx` — config 源头改 store；handler 收敛为转调 `useCommands`。
- React Query client — 提为模块级单例导出（供 agent 适配器访问）。
- `src/features/agent/tools/registry.ts` — 注册新工具。
- `src/features/agent/systemPrompt.ts` — 增补新工具用途与确认说明。
- `docs/agent.md` — 落地后同步。

**后端：** 无改动（命令调现有 `src/shared/api/*` wrapper）。

## 9. 验证

- `npm run typecheck` 通过。
- 手动：通过 agent 切 workspace/pack、改 pack meta、改 config、创建/打开/关闭 pack，确认 UI 立即反映（store/缓存同步）；删 pack 时 agent 面板弹内联确认，取消则不执行。
- 确认 UI 操作与 agent 操作走同一命令、效果一致。
