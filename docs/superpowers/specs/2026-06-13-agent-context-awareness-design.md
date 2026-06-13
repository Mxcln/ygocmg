# Agent 上下文感知增强设计

> 状态：设计稿，待评审。落地稳定后同步进核心文档 `docs/agent.md`。

## 1. 背景与目标

当前 AI Agent 获取 YGOCMG 信息的能力有限：

- **状态 block**（每轮注入 system message）只有 4 个字段：workspace 名、active pack 名、current view，没有 open packs、选中卡片、配置等。
- **工具层**只有 3 个只读工具（`list_cards` / `get_card` / `search_standard_cards`），拿不到 user settings、pack metadata、全部 pack 列表。
- **没有"当前选中卡片"概念**：`shellStore` 不持有 UI 选中态，用户说"这张卡"时 agent 无从知晓。

目标：让 agent 在需要的场合都能了解 YGOCMG 的当前状态，采用**两层模型**——小而稳定的状态常驻 prompt（context 层），大而按需的细节走工具（tool 层）。

### 两层划分原则

- **context 层**：体积小、几乎每轮都可能用到、变化频繁的状态 → 每轮注入 system message。
- **tool 层**：体积大或不常用、按需拉取的信息 → 模型主动调用工具获取。

## 2. current card 状态提升（shellStore）

current card 的语义（已确认）：**编辑抽屉打开的卡（单张）+ 批量勾选的卡集合（多张）**。不包含"列表单击高亮"和"最近操作过的卡"。

### 2.1 shellStore 新增字段

```ts
// 编辑抽屉打开的卡；关闭抽屉时置 null
selectedCard: { id: string; name: string } | null

// 批量勾选集（selection mode）；退出/清空时置 []
checkedCards: { id: string; name: string }[]
```

字段存 `{id, name}` 而非裸 id：context 构建时直接显示名字，对模型更友好，省一次 `get_card` 往返。name 在写入时由 card feature 提供（抽屉/勾选处已有行数据）。

### 2.2 setter

```ts
setSelectedCard: (card: { id: string; name: string } | null) => void
setCheckedCards: (cards: { id: string; name: string }[]) => void
```

### 2.3 同步来源（card feature）

- **编辑抽屉**：打开时 `setSelectedCard({id, name})`，关闭时 `setSelectedCard(null)`。
- **批量勾选**：selection 变化时 `setCheckedCards(...)`，与 card feature 现有本地 selection state 同步。

### 2.4 边界处理（防陈旧）

在 shellStore 现有动作里清空选中态，避免 agent 读到属于别的 pack 的卡：

- `setActivePack`、`setActiveStandardPack`：切换激活 pack/视图时清空 `selectedCard` 和 `checkedCards`。
- `removeOpenPack`：关闭 pack 时清空。
- `setWorkspace`、`clearWorkspace`：切/清 workspace 时清空。

### 2.5 边界约束

card feature 仍持有自己的本地 selection/drawer state 作为真相；shellStore 的这两个字段是"提升出来给 shell/agent 看"的镜像。不把 card 的交互逻辑搬进 shellStore，只同步结果。

## 3. context block 扩展

`buildContextBlock`（`src/features/agent/agentLoop.ts`）扩展输入与输出。每轮发消息时由 `useAgentLoop` 从 shellStore 取最新快照传入（新鲜度语义：每轮注入最新快照，只反映当前轮；对话历史中的旧 block 不回溯更新）。

### 3.1 新格式

```
[Current state]
Workspace: <name 或 (none)>
Active pack: <name 或 (none)>
Current view: <custom_pack / standard_pack / (none)>
Open packs: <name1, name2, ...> 或 (none)
Selected card: <name (id=xxx)> 或 (none)
Checked cards: <N selected: name1, name2, …> 或 (none)
```

### 3.2 要点

- **Open packs**：只列当前 workspace 已打开的 custom pack 名（来自 `openPackIds` + `packMetadataMap`）。standard pack 不计入。
- **Selected card / Checked cards**：轻摘要，给名字 + id 作线索。agent 要完整字段时用 id 调现有 `get_card`，不新增 `get_selected_card` 工具。
- Checked cards 全部列出名字 + 总数（`N selected: ...`）。批量勾选规模通常可控，本设计不设截断；若后续出现超大勾选集导致 token 压力，再单独处理。

## 4. 工具层新增只读工具

四个新工具放在 `src/features/agent/tools/readTools.ts`，包装现有 `src/shared/api/*` wrapper，注册进 `registry.ts` 的 `AGENT_TOOLS`。**无后端改动**，均为 `readOnly: true`，自动执行不走确认门。

### 4.1 get_config

- **wrapper**：`configApi.loadConfig`（或 initialize 后的 config）。
- **参数**：无。
- **返回**：业务相关配置摘要——编号推荐范围/间隔、文本语言目录、标准包源语言、app 语言、agent 语言等。
- **安全约束**：**绝不返回 `deepseek_api_key`**。实现时显式挑选字段输出，不整个 config 透传。

### 4.2 get_pack_info

- **wrapper**：从 `shellStore.packMetadataMap` 读 `PackMetadata`（已打开 pack 的完整 metadata 已缓存于此）。
- **参数**：`packId?`（省略 = 当前激活 pack，用 `ctx.packId`）。
- **返回**：pack_code、作者、版本、描述、显示语言顺序、默认导出语言等。
- **限制**：后端没有"按 id 查任意 pack 完整 metadata"的 command；`PackMetadata` 仅对已打开的 pack 可得。当前激活 pack 必然已打开，覆盖主诉求。未打开的 pack 只能通过 `list_packs` 看较轻的 overview——工具描述里明确这一点，遇未打开 pack 指引用 `list_packs`。

### 4.3 list_packs

- **wrapper**：`packApi.listPackOverviews`。
- **参数**：无。
- **返回**：当前 workspace 全部 pack 的 `PackOverview`（含未打开的），名字 + id + 类型 + 卡数等。
- **要点**：overview 含 standard pack 时标注类型，让 agent 知道 standard 为只读、不可写。

### 4.4 suggest_card_code

- **wrapper**：`cardApi.suggestCardCode`，后端签名 `{ workspaceId, packId, preferredStart }`。
- **参数**：无（用 `ctx` 的 workspace/pack，`preferredStart: null`）。
- **返回**：后端按编号策略推荐的下一个可用 code（`suggested_code` + warnings）。
- **要点**：需要当前 pack 上下文，复用 `requirePack(ctx)`。

## 5. system prompt 更新

`systemPrompt.ts` 的基础 prompt 补充：

- **新工具用途**：何时调 `get_config`（需了解编号/语言配置）、`get_pack_info`（需 pack 元信息）、`list_packs`（需全部 pack 视野）、`suggest_card_code`（建卡前取推荐编号）。
- **选中态语义**：context 里的 `Selected card` 是用户在编辑抽屉打开的卡，`Checked cards` 是批量勾选集；用户说"这张卡"时优先参考 `Selected card`，说"这些卡"时参考 `Checked cards`。仍无法确定时让用户澄清。

## 6. 涉及文件

**前端：**

- `src/shared/stores/shellStore.ts` — 新增 `selectedCard` / `checkedCards` 字段与 setter，在切换/关闭 pack/workspace 的现有动作里清空。
- `src/features/card/*` — 编辑抽屉开关、批量勾选变化处同步到 shellStore（镜像本地 state）。
- `src/features/agent/agentLoop.ts` — `buildContextBlock` 扩展输入与输出格式。
- `src/features/agent/useAgentLoop.ts` — 从 shellStore 取 open packs / selectedCard / checkedCards 传入 `buildContextBlock`。
- `src/features/agent/tools/readTools.ts` — 新增四个只读工具。
- `src/features/agent/tools/registry.ts` — 注册新工具。
- `src/features/agent/systemPrompt.ts` — 补充工具用途与选中态语义说明。

**后端：** 无改动（四个工具都包现有 command）。

## 7. 非目标（本次不做）

- "列表单击高亮"和"最近操作过的卡"不纳入 current card 语义。
- 不新建 `agentContextStore`（current card 本质是 shell 级用户焦点，归 shellStore；纯派生的 agent 专用上下文将来需要时再引入）。
- 不做对话历史中旧 context block 的回溯刷新。
- 不新增后端 command（含"按 id 查任意 pack 完整 metadata"）。

## 8. 验证

- `npm run typecheck` 通过。
- 手动验证：打开抽屉/勾选卡片后，agent 能在 context 看到选中态；问"当前配置/这个包的信息/有哪些包/推荐编号"时能调对应工具返回正确数据。
- `get_config` 返回中确认不含 `deepseek_api_key`。
