# Agent 上下文感知增强 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 AI Agent 通过"context 层（每轮注入状态）+ tool 层（按需拉取）"两层模型感知 YGOCMG 的当前状态——open packs、选中卡片、配置、pack 信息、全部 pack 列表、推荐编号。

**Architecture:** 把"编辑抽屉打开的卡"和"批量勾选的卡"提升到 `shellStore`（单一来源，镜像 card feature 的本地 state）；扩展 `buildContextBlock` 注入这些状态；新增 4 个只读工具包装现有 `src/shared/api/*` wrapper。无后端改动。

**Tech Stack:** TypeScript、React、Zustand（shellStore）、现有 agent loop（DeepSeek 工具调用）。

**测试说明:** 本项目前端无单元测试框架（无 vitest/jest）。每个任务用 `npm run typecheck` 验证类型，并给出手动验证步骤。后端无改动，无需 cargo。

---

## 文件结构

**修改：**
- `src/shared/stores/shellStore.ts` — 新增 `selectedCard` / `checkedCards` 字段与 setter；在切换/关闭 pack/workspace 的现有动作里清空。
- `src/app/PackWorkArea.tsx` — 抽屉生命周期所有者；打开/关闭抽屉时镜像 `selectedCard` 到 shellStore。
- `src/features/card/CardListPanel.tsx` — selection 所有者；维护 id→name 缓存，把 `checkedCards` 镜像到 shellStore；`onEditCard` 回调带上卡名。
- `src/features/agent/agentLoop.ts` — `buildContextBlock` 扩展输入与输出格式。
- `src/features/agent/useAgentLoop.ts` — 从 shellStore 取 open packs / selectedCard / checkedCards 传入 `buildContextBlock`。
- `src/features/agent/tools/readTools.ts` — 新增 4 个只读工具。
- `src/features/agent/tools/registry.ts` — 注册新工具。
- `src/features/agent/systemPrompt.ts` — 补充工具用途与选中态语义。
- `docs/agent.md` — 落地后同步事实。

**新建：** 无。

---

## Task 1: shellStore 新增选中态字段

**Files:**
- Modify: `src/shared/stores/shellStore.ts`

- [ ] **Step 1: 在 `ShellState` 接口加字段与 setter 声明**

在 `activeView: ActiveView | null;` 之后加：

```ts
  /** 编辑抽屉打开的卡（单张）；关闭抽屉时为 null。镜像自 card feature 本地 state。 */
  selectedCard: { id: string; name: string } | null;
  /** 批量勾选集（selection mode）；退出/清空时为 []。镜像自 card feature 本地 state。 */
  checkedCards: { id: string; name: string }[];
```

在 `setActiveStandardPack: () => void;` 之后加：

```ts
  setSelectedCard: (card: { id: string; name: string } | null) => void;
  setCheckedCards: (cards: { id: string; name: string }[]) => void;
```

- [ ] **Step 2: 在 store 初始 state 加默认值**

在 `activeView: null,`（初始 state，第 93 行附近）之后加：

```ts
  selectedCard: null,
  checkedCards: [],
```

- [ ] **Step 3: 实现 setter**

在 `setActiveStandardPack: () => set(...)` 实现之后加：

```ts
  setSelectedCard: (card) => set({ selectedCard: card }),
  setCheckedCards: (cards) => set({ checkedCards: cards }),
```

- [ ] **Step 4: 在切换/关闭 pack 与 workspace 的动作里清空选中态**

`setActivePack`：在 set 对象里加 `selectedCard: null, checkedCards: []`：

```ts
  setActivePack: (id) =>
    set({
      activePackId: id,
      activeView: id ? { type: "custom_pack", packId: id } : null,
      selectedCard: null,
      checkedCards: [],
    }),
```

`setActiveStandardPack`：

```ts
  setActiveStandardPack: () =>
    set({ activeView: { type: "standard_pack" }, selectedCard: null, checkedCards: [] }),
```

`setWorkspace` 和 `clearWorkspace` 的 set 对象里都加 `selectedCard: null, checkedCards: [],`（与现有的 `activeView: null,` 并列）。

`removeOpenPack`：在 return 的对象里加 `selectedCard: null, checkedCards: []`：

```ts
      return { openPackIds: ids, activePackId: activeId, activeView, packMetadataMap: nextMap, selectedCard: null, checkedCards: [] };
```

- [ ] **Step 5: typecheck**

Run: `npm run typecheck`
Expected: PASS（无报错）

- [ ] **Step 6: Commit**

```bash
git add src/shared/stores/shellStore.ts
git commit -m "feat: add selectedCard/checkedCards to shellStore"
```

---

## Task 2: PackWorkArea 镜像 selected card（抽屉）

**Files:**
- Modify: `src/app/PackWorkArea.tsx`
- Modify: `src/features/card/CardListPanel.tsx`（仅 `onEditCard` 签名）

**背景:** 抽屉生命周期由 `PackWorkArea` 拥有（`editingCardId`），但卡名不在它手里——卡名在 `CardListPanel` 行点击的 `CardListRow` 上。方案：`onEditCard` 回调改为带 `{id, name}`，`PackWorkArea` 收到后写入 store；新建卡（无现有卡）时 `selectedCard` 置 null；关闭抽屉时置 null。

- [ ] **Step 1: 改 `CardListPanel` 的 `onEditCard` 签名并传卡名**

`src/features/card/CardListPanel.tsx`：
- 接口 `CardListPanelProps` 把 `onEditCard: (cardId: string) => void;` 改为 `onEditCard: (card: { id: string; name: string }) => void;`
- `handleRowClick`（约第 100 行）改为：

```ts
  function handleRowClick(card: CardListRow) {
    onEditCard({ id: card.id, name: card.name });
  }
```

- [ ] **Step 2: 在 `PackWorkArea` 引入 store setter**

`src/app/PackWorkArea.tsx`，在现有 `useShellStore` 选择器附近加：

```ts
  const setSelectedCard = useShellStore((s) => s.setSelectedCard);
```

- [ ] **Step 3: 改 `handleEditCard` 接收 `{id, name}` 并写 store**

```ts
  const handleEditCard = useCallback(
    (card: { id: string; name: string }) => {
      setEditingCardId(card.id);
      setIsCreatingCard(false);
      setSelectedCard(card);
    },
    [setSelectedCard],
  );
```

- [ ] **Step 4: 新建卡与关闭抽屉时清空 selectedCard**

```ts
  const handleNewCard = useCallback(() => {
    setEditingCardId(null);
    setIsCreatingCard(true);
    setSelectedCard(null);
  }, [setSelectedCard]);

  const handleDrawerClose = useCallback(() => {
    setEditingCardId(null);
    setIsCreatingCard(false);
    setSelectedCard(null);
  }, [setSelectedCard]);
```

- [ ] **Step 5: 切 pack/view 的 effect 里同步清空**

`PackWorkArea` 现有 effect（约第 35 行，依赖 `[activePackId, activeView]`）里加 `setSelectedCard(null);`：

```ts
  useEffect(() => {
    setEditingCardId(null);
    setIsCreatingCard(false);
    setActiveTab("cards");
    setSelectedCard(null);
  }, [activePackId, activeView, setSelectedCard]);
```

> 注：shellStore 的 `setActivePack` 也会清空，这里是 UI 侧冗余保险，确保抽屉关闭与 store 一致。

- [ ] **Step 6: typecheck**

Run: `npm run typecheck`
Expected: PASS

- [ ] **Step 7: 手动验证**

Run: `npm run dev`，打开一个 custom pack，单击一张卡打开编辑抽屉。
Expected: 浏览器 React devtools 或临时 `console.log(useShellStore.getState().selectedCard)` 显示 `{id, name}`；关闭抽屉后为 null；切换 pack 后为 null。

- [ ] **Step 8: Commit**

```bash
git add src/app/PackWorkArea.tsx src/features/card/CardListPanel.tsx
git commit -m "feat: mirror edit-drawer selected card into shellStore"
```

---

## Task 3: CardListPanel 镜像 checked cards（批量勾选）

**Files:**
- Modify: `src/features/card/CardListPanel.tsx`

**背景:** 选中态权威是 `selectedCardIds`（仅 id，跨页）。卡名需要从已加载页累积的 `id→name` 缓存映射，缺失时回退用 id 当 name（保证 store 始终有项可显示）。`handlePageLoaded` 已能拿到 `page.items`，用它喂缓存。

- [ ] **Step 1: 引入 store setter 与 name 缓存 ref**

`src/features/card/CardListPanel.tsx`，在现有 `useShellStore` 选择器附近加：

```ts
  const setCheckedCards = useShellStore((s) => s.setCheckedCards);
```

在现有 `selectionCleanupRef` 声明附近加一个名字缓存 ref：

```ts
  const cardNameCacheRef = useRef<Map<string, string>>(new Map());
```

- [ ] **Step 2: 在 `handlePageLoaded` 里用已加载页喂缓存**

注意 `handlePageLoaded` 收到的是 `CardBrowserPage`，其 `items` 是 `CardListRow[]`（含 `id`、`name`）。在该 `useCallback` 体的最前面（`if (...) return;` 之前）加：

```ts
      for (const row of page.items) {
        cardNameCacheRef.current.set(row.id, row.name);
      }
```

- [ ] **Step 3: selection 变化时镜像到 store**

新增一个把 ids 映射成 `{id, name}[]` 并写 store 的回调，替换传给 `CardBrowserPanel` 的 `onSelectionChange`。在组件内（`handleRowClick` 附近）加：

```ts
  const handleSelectionChange = useCallback(
    (cardIds: string[]) => {
      setSelectedCardIds(cardIds);
      const cache = cardNameCacheRef.current;
      setCheckedCards(cardIds.map((id) => ({ id, name: cache.get(id) ?? id })));
    },
    [setCheckedCards],
  );
```

把 `<CardBrowserPanel>` 的 `onSelectionChange={setSelectedCardIds}` 改为 `onSelectionChange={handleSelectionChange}`。

> `useCallback` 依赖只列 `setCheckedCards`：`setSelectedCardIds` 是 useState setter（引用稳定），`cardNameCacheRef` 是 ref（稳定），都无需列入。

- [ ] **Step 4: 批量操作后与切 pack 时清空 store 的 checkedCards**

`refreshAfterBatch`（约第 138 行）里 `setSelectedCardIds([]);` 之后加：

```ts
    setCheckedCards([]);
```

切 pack 的 effect（约第 70 行，依赖 `[activePackId]`，内部已 `setSelectedCardIds([])`）里加：

```ts
    setCheckedCards([]);
    cardNameCacheRef.current.clear();
```

并把该 effect 的依赖数组改为 `[activePackId, setCheckedCards]`。

> 注：`handlePageLoaded` 里的 selection cleanup（剔除已不存在的卡）调用的是 `setSelectedCardIds`，不直接更新 store。下次用户改动 selection 时 `handleSelectionChange` 会重算 store；为避免 store 残留已删除卡的短暂不一致，见 Step 5。

- [ ] **Step 5: selection cleanup 后同步 store**

`handlePageLoaded` 里现有的 `setSelectedCardIds((current) => current.filter((id) => existingIds.has(id)));` 改为同时更新 store：

```ts
          setSelectedCardIds((current) => {
            const next = current.filter((id) => existingIds.has(id));
            const cache = cardNameCacheRef.current;
            setCheckedCards(next.map((id) => ({ id, name: cache.get(id) ?? id })));
            return next;
          });
```

并把 `handlePageLoaded` 的 `useCallback` 依赖数组从 `[activePackId, selectedCardIds.length, workspaceId]` 改为 `[activePackId, selectedCardIds.length, workspaceId, setCheckedCards]`。

- [ ] **Step 6: typecheck**

Run: `npm run typecheck`
Expected: PASS

- [ ] **Step 7: 手动验证**

Run: `npm run dev`，打开 custom pack，进入批量选择模式，勾选几张卡（含翻页后勾选）。
Expected: 临时 `console.log(useShellStore.getState().checkedCards)` 显示 `{id, name}[]`，名字正确（翻过的页有名字，未翻到的回退 id）；完成/退出选择或切 pack 后为 `[]`。

- [ ] **Step 8: Commit**

```bash
git add src/features/card/CardListPanel.tsx
git commit -m "feat: mirror batch-checked cards into shellStore"
```

---

## Task 4: 扩展 buildContextBlock

**Files:**
- Modify: `src/features/agent/agentLoop.ts`

**背景:** 现有 `buildContextBlock` 接收 `{workspaceName, activePackId, activePackName, activeView}` 输出 4 行。扩展输入与输出，加 Open packs、Selected card、Checked cards。

- [ ] **Step 1: 扩展 `buildContextBlock` 签名与实现**

把现有函数（约第 52-64 行）整体替换为：

```ts
/** Build the per-turn context block (current shell snapshot). */
export function buildContextBlock(snapshot: {
  workspaceName: string | null;
  activePackId: string | null;
  activePackName: string | null;
  activeView: string;
  openPackNames: string[];
  selectedCard: { id: string; name: string } | null;
  checkedCards: { id: string; name: string }[];
}): string {
  const lines = [
    "[Current state]",
    `Workspace: ${snapshot.workspaceName ?? "(none)"}`,
    `Active pack: ${snapshot.activePackName ?? snapshot.activePackId ?? "(none)"}`,
    `Current view: ${snapshot.activeView}`,
    `Open packs: ${snapshot.openPackNames.length ? snapshot.openPackNames.join(", ") : "(none)"}`,
    `Selected card: ${
      snapshot.selectedCard
        ? `${snapshot.selectedCard.name || "(unnamed)"} (id=${snapshot.selectedCard.id})`
        : "(none)"
    }`,
    `Checked cards: ${
      snapshot.checkedCards.length
        ? `${snapshot.checkedCards.length} selected: ${snapshot.checkedCards
            .map((c) => c.name || `(id=${c.id})`)
            .join(", ")}`
        : "(none)"
    }`,
  ];
  return lines.join("\n");
}
```

- [ ] **Step 2: typecheck（预期此处会因调用方未更新而报错）**

Run: `npm run typecheck`
Expected: FAIL，报 `useAgentLoop.ts` 调用 `buildContextBlock` 缺少 `openPackNames`/`selectedCard`/`checkedCards` 参数。Task 5 修复。

> 这是预期的中间状态——Task 4 改函数签名，Task 5 改调用方。两个任务一起达成可编译。若希望单任务即可编译，可与 Task 5 合并提交；此处按文件职责分两个 commit。

- [ ] **Step 3: Commit（与 Task 5 紧邻执行）**

```bash
git add src/features/agent/agentLoop.ts
git commit -m "feat: extend agent context block with open packs and selected cards"
```

---

## Task 5: useAgentLoop 注入新状态

**Files:**
- Modify: `src/features/agent/useAgentLoop.ts`

**背景:** `sendMessage` 里从 `useShellStore.getState()` 取快照传给 `buildContextBlock`。补齐 openPackNames、selectedCard、checkedCards。

- [ ] **Step 1: 在 `sendMessage` 里计算 openPackNames 并扩展 contextBlock 调用**

现有代码（约第 39-51 行）：

```ts
    const shell = useShellStore.getState();
    const ctx: ToolContext = {
      workspaceId: shell.workspaceId,
      packId: shell.activePackId,
    };
    const activePackName =
      (shell.activePackId && shell.packMetadataMap[shell.activePackId]?.name) || null;
    const contextBlock = buildContextBlock({
      workspaceName: shell.workspaceName,
      activePackId: shell.activePackId,
      activePackName,
      activeView: shell.activeView?.type ?? "(none)",
    });
```

替换为：

```ts
    const shell = useShellStore.getState();
    const ctx: ToolContext = {
      workspaceId: shell.workspaceId,
      packId: shell.activePackId,
    };
    const activePackName =
      (shell.activePackId && shell.packMetadataMap[shell.activePackId]?.name) || null;
    const openPackNames = shell.openPackIds
      .map((id) => shell.packMetadataMap[id])
      .filter((meta) => meta && meta.kind === "custom")
      .map((meta) => meta.name);
    const contextBlock = buildContextBlock({
      workspaceName: shell.workspaceName,
      activePackId: shell.activePackId,
      activePackName,
      activeView: shell.activeView?.type ?? "(none)",
      openPackNames,
      selectedCard: shell.selectedCard,
      checkedCards: shell.checkedCards,
    });
```

> `meta.kind === "custom"` 过滤掉非 custom pack（与 CardListPanel 的 targetPackOptions 过滤一致）。`PackMetadata.kind` 字段已存在。

- [ ] **Step 2: typecheck**

Run: `npm run typecheck`
Expected: PASS（Task 4 的报错此时消除）

- [ ] **Step 3: 手动验证**

Run: `npm run dev`，配置好 DeepSeek key，打开 pack、开抽屉/勾选卡，向 agent 发一句"我现在选中了什么？"。
Expected: agent 的回复能反映当前 selected/checked card 和 open packs（说明 context block 注入生效）。

- [ ] **Step 4: Commit**

```bash
git add src/features/agent/useAgentLoop.ts
git commit -m "feat: inject open packs and selected cards into agent context"
```

---

## Task 6: 新增 4 个只读工具

**Files:**
- Modify: `src/features/agent/tools/readTools.ts`

**背景:** 4 个工具都包现有 wrapper。`get_config` 必须排除 `deepseek_api_key`。`get_pack_info` 从 `shellStore.packMetadataMap` 读已打开 pack 的完整 metadata。`list_packs` 用 `packApi.listPackOverviews`。`suggest_card_code` 用 `cardApi.suggestCardCode`，需 `requirePack`。

工具读 shellStore 用 `useShellStore.getState()`（非 hook 场景，agentLoop 同样做法）。

- [ ] **Step 1: 在 readTools.ts 顶部补 import**

现有 import：

```ts
import { cardApi } from "../../../shared/api/cardApi";
import { standardPackApi } from "../../../shared/api/standardPackApi";
import type { AgentTool } from "./types";
import { requirePack } from "./types";
```

改为：

```ts
import { cardApi } from "../../../shared/api/cardApi";
import { standardPackApi } from "../../../shared/api/standardPackApi";
import { packApi } from "../../../shared/api/packApi";
import { configApi } from "../../../shared/api/configApi";
import { useShellStore } from "../../../shared/stores/shellStore";
import type { AgentTool } from "./types";
import { requirePack, ToolError } from "./types";
```

- [ ] **Step 2: get_config 工具（排除 API key）**

在文件末尾（`searchStandardCardsTool` 之后）追加：

```ts
export const getConfigTool: AgentTool = {
  name: "get_config",
  description:
    "Get the user's business-relevant global settings: custom card code recommended " +
    "range and minimum gap, text language catalog, standard-pack source language, app " +
    "language, and agent reply language. Call this when you need to know numbering rules " +
    "or language configuration. Never includes secrets.",
  readOnly: true,
  parameters: { type: "object", properties: {} },
  async execute() {
    const config = await configApi.loadConfig();
    // Explicitly pick business fields — never expose deepseek_api_key.
    return {
      custom_code_recommended_min: config.custom_code_recommended_min,
      custom_code_recommended_max: config.custom_code_recommended_max,
      custom_code_min_gap: config.custom_code_min_gap,
      app_language: config.app_language,
      agent_language: config.agent_language,
      standard_pack_source_language: config.standard_pack_source_language,
      text_language_catalog: config.text_language_catalog.map((lang) => ({
        id: lang.id,
        label: lang.label,
        kind: lang.kind,
        hidden: lang.hidden,
      })),
    };
  },
};
```

- [ ] **Step 3: get_pack_info 工具（从 packMetadataMap 读已打开 pack）**

继续追加：

```ts
export const getPackInfoTool: AgentTool = {
  name: "get_pack_info",
  description:
    "Get full metadata of an OPEN custom pack: pack_code, author, version, description, " +
    "display language order, default export language. Omit packId for the currently active " +
    "pack. For packs that are not open, use list_packs (only lighter overview is available).",
  readOnly: true,
  parameters: {
    type: "object",
    properties: {
      packId: {
        type: "string",
        description: "Optional pack id. Omit to use the currently active pack.",
      },
    },
  },
  async execute(args) {
    const shell = useShellStore.getState();
    const packId =
      typeof args.packId === "string" && args.packId ? args.packId : shell.activePackId;
    if (!packId) {
      throw new ToolError("No active pack. Ask the user to open a pack first.");
    }
    const metadata = shell.packMetadataMap[packId];
    if (!metadata) {
      throw new ToolError(
        `Pack ${packId} is not open. Use list_packs to see all packs, or ask the user to open it.`,
      );
    }
    return metadata;
  },
};
```

- [ ] **Step 4: list_packs 工具**

继续追加：

```ts
export const listPacksTool: AgentTool = {
  name: "list_packs",
  description:
    "List all packs in the current workspace (including packs that are not open). Returns " +
    "overview rows (id, name, kind, card count, etc.). Standard packs are read-only reference " +
    "and cannot be edited.",
  readOnly: true,
  parameters: { type: "object", properties: {} },
  async execute() {
    return packApi.listPackOverviews();
  },
};
```

- [ ] **Step 5: suggest_card_code 工具**

继续追加：

```ts
export const suggestCardCodeTool: AgentTool = {
  name: "suggest_card_code",
  description:
    "Suggest the next available custom card code for the active pack, following the user's " +
    "numbering policy. Call this before creating a card when you need a code.",
  readOnly: true,
  parameters: { type: "object", properties: {} },
  async execute(_args, ctx) {
    const { workspaceId, packId } = requirePack(ctx);
    return cardApi.suggestCardCode({ workspaceId, packId, preferredStart: null });
  },
};
```

> `execute(_args, ctx)`：第一个参数命名 `_args` 表示不使用，避免 lint 未用告警；`ctx` 来自工具签名 `execute(args, ctx)`。

- [ ] **Step 6: typecheck**

Run: `npm run typecheck`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src/features/agent/tools/readTools.ts
git commit -m "feat: add get_config/get_pack_info/list_packs/suggest_card_code agent tools"
```

---

## Task 7: 注册新工具

**Files:**
- Modify: `src/features/agent/tools/registry.ts`

- [ ] **Step 1: import 并加入 AGENT_TOOLS**

现有 import：

```ts
import { getCardTool, listCardsTool, searchStandardCardsTool } from "./readTools";
```

改为：

```ts
import {
  getCardTool,
  listCardsTool,
  searchStandardCardsTool,
  getConfigTool,
  getPackInfoTool,
  listPacksTool,
  suggestCardCodeTool,
} from "./readTools";
```

`AGENT_TOOLS` 数组（现有 6 个）改为：

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
];
```

- [ ] **Step 2: typecheck**

Run: `npm run typecheck`
Expected: PASS

- [ ] **Step 3: 手动验证**

Run: `npm run dev`，向 agent 分别问："当前有哪些卡包？"、"这个包的信息"、"推荐一个新卡编号"、"我的编号范围设置是多少？"
Expected: agent 调用对应工具并返回正确数据；`get_config` 的返回不含 API key。

- [ ] **Step 4: Commit**

```bash
git add src/features/agent/tools/registry.ts
git commit -m "feat: register new read-only agent tools"
```

---

## Task 8: 更新 system prompt

**Files:**
- Modify: `src/features/agent/systemPrompt.ts`

- [ ] **Step 1: 在基础 prompt 补充工具用途与选中态语义**

把现有 `## Resolving which card` 段（"You cannot see the user's UI selection..."）替换为：

```ts
## Current selection
- The context block reports the user's current selection: "Selected card" is the card open in the edit drawer; "Checked cards" are the cards ticked in batch-selection mode.
- When the user says "this card", prefer the Selected card. When they say "these cards"/"the selected cards", use the Checked cards. If neither is present or it is ambiguous, ask the user to clarify or use list_cards to locate by name/code before writing.

## Discovering state
- Use get_config to read numbering rules (recommended code range / gap) and language settings.
- Use get_pack_info for the active (or a named open) pack's metadata; use list_packs to see all packs in the workspace, including unopened ones.
- Use suggest_card_code to get the next available code before creating a card.
```

> 注：原 `## Resolving which card` 整段被上面两段取代（选中态语义已覆盖原"无法看到 UI 选中"的说明，并升级为现在能看到）。

- [ ] **Step 2: typecheck**

Run: `npm run typecheck`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/features/agent/systemPrompt.ts
git commit -m "feat: teach agent about selection context and new tools"
```

---

## Task 9: 同步核心文档

**Files:**
- Modify: `docs/agent.md`

- [ ] **Step 1: 更新 agent.md 的状态 block 与工具集描述**

在 `agent.md` 的 "Agent Loop" 节，把"当前状态 block 注入工作区名、激活 pack 名/id、当前视图"更新为还包含 open packs、selected card、checked cards。

在工具集节，把只读工具从 3 个更新为 7 个：补 `get_config`、`get_pack_info`、`list_packs`、`suggest_card_code` 的一句话说明。

在"已知限制"节，删除/修订"无指代消解（看不到 UI 选中态）"——现在 agent 能看到编辑抽屉与批量勾选的选中态（但仍看不到列表单击高亮）。

> 具体措辞实现时按 agent.md 现有风格写；保持与本 plan 的事实一致。

- [ ] **Step 2: Commit**

```bash
git add docs/agent.md
git commit -m "docs: sync agent.md with context-awareness changes"
```

---

## 完成标准

- `npm run typecheck` 全程通过。
- agent 能在 context 看到 open packs、selected card、checked cards。
- agent 能调 `get_config`（不泄露 key）、`get_pack_info`、`list_packs`、`suggest_card_code`。
- 切换/关闭 pack、切 workspace、完成批量操作后，选中态正确清空。
- `docs/agent.md` 与实现一致。
