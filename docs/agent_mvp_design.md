# YGOCMG AI Agent MVP 设计文档（提案）

> 状态：**设计提案，待评审**。本文档描述计划中的 MVP 实现，不是当前事实。落地稳定后再同步进核心文档（`functional_spec.md` / `system_architecture.md` / `code_structure_api.md` / `ui_design.md`）。
>
> 本文档替代早期的 `agent_feature_design.md`（provider 中立、流式 IPC、双层确认那版）。那版对 MVP 阶段过度设计，本版按"先跑通最小可用，按需生长"重写。早期版本的扩展性考量（provider 抽象、流式、plan 模式）作为后续阶段参考保留在 §9。

---

## 1. 目标与范围

为 YGOCMG 增加一个对话式 AI Agent，用户用自然语言完成卡片管理操作。MVP 目标是**跑通一条端到端可用的链路**，而不是把所有扩展点提前焊死。

### MVP 范围（明确做）

- **单一 provider：DeepSeek**。写死 DeepSeek 调用，不做 provider 抽象层。
- **API key 存本地明文 config**（`GlobalConfig`），不做 keychain。
- **显式右侧边栏 UI**（不做"先 panel 后边栏"的渐进路径，直接边栏）。
- **简单工具调用循环**：只读工具 + 写工具，写工具复用后端现成确认门。
- **非流式**：Rust 一次性返回完整响应，不做 SSE 中继与流式 IPC 协议。

### MVP 不做（推迟到 §9 后续阶段）

- Provider 中立 / 统一抽象层 / 多家适配器。
- 流式输出（打字机效果）与 requestId 关联、取消通道等流式 IPC 协议。
- keychain / OS 凭据库。
- AI 写操作 review 层（每个写操作再叠一层确认）—— MVP 只靠后端确认门。
- `agentContextStore` 状态提升（选中态/编辑态全局化）。
- 显式 plan 模式、router、子 agent、异步 job 工具（导入/导出）。

### 沿用的设计内核（来自旧文档，仍然成立）

1. **工具执行体复用 `src/shared/api/*`**，不重新实现校验/确认/编号逻辑（CLAUDE.md 规则）。
2. **后端拥有最终业务规则**：危险写操作的确认门由后端 `WriteResult` 强制。
3. **不绕过 `src/shared/api`**：Agent 工具调用现有 API wrapper。

---

## 2. 架构总览

```
┌──────────────────── 前端 (webview, TS/React) ────────────────────┐
│                                                                   │
│  AgentSidebar (右侧边栏 UI)  ──→  agentStore (Zustand, 对话历史)  │
│        │                                                          │
│        ▼                                                          │
│  Agent Loop (DeepSeek 专用，OpenAI 兼容格式)                      │
│   - 组装 messages + tools                                         │
│   - 调 agentApi.chat() → 后端 → DeepSeek                          │
│   - 解析 tool_calls → 工具分发 → 执行体                           │
│        │                          │                               │
│        │ tool_call                │ chat 请求                     │
│        ▼                          ▼                               │
│  Tool Registry            agentApi.chat(messages, tools)          │
│   - 只读工具                      │                               │
│   - 写工具（确认门）              │                               │
│        │                          │                               │
│        ▼                          │                               │
│  src/shared/api/* wrapper         │                               │
│   (invokeApi → Tauri command)     │                               │
└──────────┼────────────────────────┼──────────────────────────────┘
           │ Tauri IPC               │ Tauri IPC (llm_chat)
           ▼                         ▼
┌──────────────────── 后端 (Rust) ─────────────────────────────────┐
│  现有 command（卡片/包/文本/资源/导入导出/标准卡/job）            │
│  + 新增 llm_chat command（非流式）                                │
│       - 从 GlobalConfig 读 deepseek_api_key（明文）              │
│       - reqwest POST 到 https://api.deepseek.com/chat/completions │
│       - 注入 Authorization header                                 │
│       - 返回完整 JSON 响应                                         │
└───────────────────────────────────────────────────────────────────┘
                                     │ HTTPS
                                     ▼
                          DeepSeek API
```

### 职责划分

| 层 | 位置 | 职责 |
|---|---|---|
| UI | 前端 React | 右侧边栏、对话、内联确认卡片 |
| 对话状态 | 前端 Zustand | 对话历史、pending 状态 |
| Agent loop | 前端 TS | 循环、工具分发、组装 DeepSeek 请求 |
| 工具执行体 | 前端 TS | 包装 `src/shared/api/*` wrapper |
| LLM HTTP | 后端 Rust | reqwest 转发 + 注入 key（非流式） |
| 业务规则 | 后端 Rust | 现有 service 层（不改动） |

**为什么 HTTP 走后端而不前端直连**：即使 key 是明文存 config，让请求经 Rust 转发仍有两个好处——(1) 避免浏览器 CORS / 把 key 暴露在 webview 网络面板；(2) 为后续迁移 keychain 留好边界，那时只换 Rust 取 key 的方式，前端不动。成本只是一个薄 command，值得现在做。

---

## 3. DeepSeek 调用方式（已核对官方文档）

DeepSeek API 是 **OpenAI 兼容**格式。来源：[Tool Calls 指南](https://api-docs.deepseek.com/guides/tool_calls)、[Create Chat Completion](https://api-docs.deepseek.com/api/create-chat-completion)、[Your First API Call](https://api-docs.deepseek.com/)。

### 3.1 端点与鉴权

- Base URL：`https://api.deepseek.com`（OpenAI 兼容格式）。另有 Anthropic 格式端点 `https://api.deepseek.com/anthropic`，MVP 不用。
- 端点：`POST /chat/completions`
- 鉴权：HTTP header `Authorization: Bearer <API_KEY>`
- Content-Type：`application/json`

### 3.1.1 ⚠️ 模型选择（重要：旧模型名即将弃用）

- **MVP 用 `deepseek-v4-flash`**：快、便宜，适合工具调用循环这种高频小请求。`model` 字段官方仅接受 `deepseek-v4-flash` / `deepseek-v4-pro` 两个值。
- **不要用 `deepseek-chat` / `deepseek-reasoner`**：这两个旧名现对应 `deepseek-v4-flash` 的非思考/思考模式，且**官方公告将于 2026/07/24 15:59 UTC 弃用**（距今约六周）。新代码直接用 v4 名字，避免一上线就踩弃用。
- **关掉思考模式**：v4 模型 `thinking.type` **默认 `enabled`**。MVP 的 flash 当工具调用器用，不需要推理开销，请求里显式传 `"thinking": { "type": "disabled" }`。
- 其他默认值：`temperature` 默认 1；`tool_choice` 在有 tools 时默认 `auto`（无需显式传）；`tools` 上限 128 个；`frequency_penalty`/`presence_penalty` 已弃用无效。

### 3.2 请求体（含工具定义）

```jsonc
{
  "model": "deepseek-v4-flash",
  "thinking": { "type": "disabled" },   // flash 默认 enabled，工具调用器关掉
  "messages": [
    { "role": "system", "content": "<system prompt + 语言指令 + 当前状态 block>" },
    { "role": "user", "content": "列出当前包里的卡" }
  ],
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "list_cards",
        "description": "当用户要求列出/查看当前包中的卡片时调用",
        "parameters": { /* JSON Schema */ }
      }
    }
  ],
  // tool_choice 省略 → 有 tools 时默认 auto
  "stream": false
}
```

### 3.3 模型发起工具调用时的响应

`choices[0].finish_reason == "tool_calls"`，assistant 消息带 `tool_calls` 数组：

```jsonc
{
  "choices": [{
    "finish_reason": "tool_calls",
    "message": {
      "role": "assistant",
      "content": null,
      "tool_calls": [{
        "id": "call_abc123",
        "type": "function",
        "function": {
          "name": "list_cards",
          "arguments": "{\"pack_id\":\"...\"}"   // ⚠️ 字符串，需 JSON.parse
        }
      }]
    }
  }]
}
```

> ⚠️ `arguments` 是 **JSON 编码的字符串**，不是对象。前端 loop 拿到后要 `JSON.parse`，并对解析失败做容错（把错误作为 tool 结果回写，让模型重试）。

### 3.4 回写工具结果

执行工具后，把结果作为 `role:"tool"` 消息追加，`tool_call_id` 与上面的 `id` 配对，然后整个对话重新发回去：

```jsonc
{
  "messages": [
    { "role": "user", "content": "..." },
    { "role": "assistant", "content": null, "tool_calls": [{ "id": "call_abc123", ... }] },
    { "role": "tool", "tool_call_id": "call_abc123", "content": "<工具返回的 JSON 字符串>" }
  ]
}
```

`finish_reason == "stop"` 表示模型给出最终回答、本轮结束。


---

## 4. Agent Loop（前端，DeepSeek 专用）

### 4.1 控制流

```
1. 组装 messages（system + 历史 + 当轮 user message）+ tools
2. agentApi.chat({ messages, tools }) → 后端 → DeepSeek，拿完整响应
3. finish_reason == "tool_calls":
     for each tool_call:
       a. JSON.parse(arguments)（失败 → 错误作为 tool result 回写）
       b. 工具注册表按 name 分发 → 执行体调 src/shared/api wrapper
       c. 写工具返回 needs_confirmation → 暂停，UI 弹确认 → 用户确认 → confirmCardWrite
       d. 结果序列化为 JSON 字符串，作为 role:"tool" 消息回写历史
     回到 2（带上新消息重新请求）
4. finish_reason == "stop": 把 assistant 文本展示给用户，本轮结束
```

要点：

- **整段对话每轮重发**（OpenAI 兼容协议要求），历史在 `agentStore` 累积。
- **工具调用循环上限**：设一个最大轮数（如 8），防止模型卡在工具循环里烧 token。超限则向用户报告并停止。
- **错误容错**：`JSON.parse` 失败、工具抛错、后端 `AppError`，都序列化成 tool result 回写，让模型自己决定重试或换路；不要直接崩 loop。

### 4.2 不做的控制流复杂度

- **不做 router 层**：MVP 工具约 5-8 个，远未到需要分流的规模。
- **不做独立 plan/执行两阶段**：模型自己用多次工具调用完成隐式多步（如"所有 N 卡 +100" → 先 `list_cards` 再逐个 `update_card`），不引入额外控制流层。
- **不做流式**：见 §1。Loop 每轮等后端返回完整响应即可，实现最简。

---

## 5. 工具注册表与确认门

### 5.1 工具集（精选，不暴露全部 command）

工具太多会降低模型选择准确率。MVP 精选一组面向用户意图的工具：

**只读工具（自动执行，无需确认）：**
- `list_cards`：列出当前包卡片（包装 `cardApi.listCards`）
- `get_card`：取单卡详情（`cardApi.getCard`）
- `search_standard_cards`：查标准卡参考数据（`standardPackApi.searchStandardCards`）

**写工具（走后端确认门）：**
- `create_card`（`cardApi.createCard`）
- `update_card`（`cardApi.updateCard`）
- `move_cards`（`cardApi.moveCards`）

> 资源类（图片/脚本导入）、文本编辑、删除、批量删、导入/导出 job 推迟到后续阶段（§9）。MVP 先把"查 + 改卡"这条主链路跑通。

### 5.2 工具定义方式

每个工具：
- **参数 schema**：MVP 直接**手写 JSON Schema 字面量**（DeepSeek `tools[].function.parameters` 需要的就是 JSON Schema），不引入 `zod`。工具少、字段稳定，手写最直接；等工具膨胀或要和 contracts 类型自动对齐时再考虑引 `zod`（§9）。
- **执行体** = 调对应 `src/shared/api/*` wrapper（如 `cardApi.updateCard`）。
- **描述写明触发条件**（不只功能）：如 *"当用户要求新建或添加一张卡片时调用"*，能显著提升调用准确率。

工具注册表形状（草案）：

```typescript
// src/features/agent/tools/registry.ts
interface AgentTool {
  name: string;
  description: string;
  parameters: object;            // JSON Schema 字面量
  readOnly: boolean;             // 只读 → 自动执行；写 → 走确认门
  execute(args: unknown): Promise<unknown>;  // 调 src/shared/api wrapper
}
```

### 5.3 确认门 —— 只复用后端两段式写入

MVP **不做 AI 写操作 review 层**（旧文档的"双层确认"推迟到 §9）。写操作只走后端现成的确认门：

后端 `WriteResult<T>`（来自 `src/shared/contracts/card.ts`）有两态：

```typescript
type WriteResult<T> =
  | { status: "ok"; data: T; warnings: ValidationIssue[] }
  | { status: "needs_confirmation"; confirmation_token: string; warnings: ValidationIssue[]; preview: unknown | null };
```

写工具执行体流程：

```
写工具执行体调 cardApi.updateCard(input)
   ↓ 后端返回 WriteResult<T>
   ├─ status: "ok"
   │    → 把 data 作为 tool_result 回写，loop 继续
   └─ status: "needs_confirmation"
        → 暂停 loop，在对话流内联渲染确认卡片（warnings/preview）
        → 用户取消 → 把"用户已取消"作为 tool_result 回写，模型据此调整
        → 用户确认 → cardApi.confirmCardWrite({ confirmation_token })
                    → 把结果作为 tool_result 回写，loop 继续
```

**并发写工具串行确认**：若模型一轮发多个写工具，逐个执行、逐个确认（一个确认完再处理下一个），避免一次弹多张确认卡片。

| 操作类型 | 确认策略 |
|---|---|
| 只读 | 自动执行 |
| 写操作返 ok | 自动提交，对话流显示结果 |
| 写操作返 needs_confirmation | 内联确认卡片 → confirmCardWrite |


---

## 6. 程序状态感知（MVP 最小做法）

Agent 需要知道"当前是哪个包"才能正确调 `list_cards` 等工具。MVP 用**最小快照**，不做状态提升重构。

### 6.1 可直接读到的状态（来自 `useShellStore`）

`src/shared/stores/shellStore.ts` 现成可读，无需改造：
- `workspaceId` / `workspaceName`
- `openPackIds` / `activePackId` / `packMetadataMap`
- `activeView`（`custom_pack` / `standard_pack`）

每轮用户发消息时，把这份快照拼成一个轻量上下文块，附在当轮 user message 前。让模型知道"当前活动包"，消解"当前包里的卡"这类指代。

```
[当前状态]
工作区: <workspaceName>
活动包: <activePackId>（<packMetadataMap[activePackId].name>）
当前视图: <activeView.type>
```

### 6.2 MVP 明确不做：选中态/编辑态注入

"这张卡""选中的这几张"依赖**选中卡 id / 正在编辑的卡 id**，这些当前是组件局部 state（`CardListPanel` / `CardEditDrawer`），不在全局 store，agent 够不到。

**MVP 不做状态提升**。后果与对策：
- 模型无法消解"这张卡" → 让用户在指令里给出可识别信息（卡名/编号），或先 `list_cards` 让模型按名字定位 id。
- 这是 MVP 的已知限制，写进验收说明。状态提升（`agentContextStore` + 生命周期管理）推迟到 §9。

### 6.3 数据最小化

只读工具会把卡片数据送进上下文。MVP 设默认上限：`list_cards` 走现有分页（不一次拉全包），优先返回摘要字段而非完整卡。避免"列出所有卡"把整个包灌进 LLM 上下文。

---

## 7. API Key 存储（MVP：本地明文 config）

### 7.1 MVP 决策：明文存 `GlobalConfig`

MVP **不做 keychain**。在 `GlobalConfig` 加一个字段存 DeepSeek key：

```typescript
// src/shared/contracts/config.ts，GlobalConfig 新增
deepseek_api_key: string | null;
```

对应后端 `domain/config/model.rs` 的 `GlobalConfig` 同步加字段。key 随 `save_config` 落盘到 `global_config.json`。

### 7.2 已知风险（接受，后续迁移）

- key 明文落盘在 `global_config.json`。这是**本地单用户**工具，威胁等级低，MVP 接受此风险。
- ⚠️ **不要把 `global_config.json` 提交进 git**——确认它在 `.gitignore` 里（落地时核对）。
- 迁移路径：§7.3 的边界设计让后续换 keychain 时前端几乎不动。

### 7.3 为后续迁移留的边界

key 的读取**只发生在 Rust 的 `llm_chat` command 里**（从 `GlobalConfig` 读 → 注入 header）。前端 loop 从不接触 key，只发 messages/tools。后续迁移 keychain 时，只改 Rust 里"从哪取 key"这一行，前端和 IPC 边界都不动。

### 7.4 设置 UI

- agent 相关设置集中在 settings 的独立 **"AI 助手"** tab（`settings.tab.agent`），不再混在 General tab 里。
- **DeepSeek API key 输入框**（密码型输入，写入 `deepseek_api_key`）。
- 明示提示：**"对话内容与相关卡片数据会发送给 DeepSeek 服务商。"**
- **回复语言下拉**（写入 `agent_language`）：`auto`（跟随程序 UI 语言，默认）或显式 locale（`en-US` / `ja-JP` / `zh-CN`）。
- key 为空时，agent 边栏显示"未配置 API key"引导。

### 7.5 回复语言（system prompt 注入）

agent 回复语言由 `agent_language` 决定，不再隐式跟随对话输入语言：

- `auto` → 解析为当前程序 UI locale（`useAppI18n().locale`）；否则用显式值。
- 解析在前端 `useAgentLoop` 完成，把目标 locale 传给 `runAgentTurn` → `buildSystemPrompt(locale)`。
- `buildSystemPrompt` 在基础 system prompt 后追加一条语言指令，要求模型用目标语言撰写回复，但保持卡名 / code 等数据值不变。
- 这是 prompt 层的软约束（引导而非强制），不做回复后语言检测重试。
- 后端 `GlobalConfig.agent_language` 取值受 `SUPPORTED_AGENT_LANGUAGES`（`auto` + 三种 locale）校验与归一，非法值回落 `auto`；旧 config 文件无此字段时 serde 默认 `auto`。

---

## 8. UI 设计（MVP：显式右侧边栏）

### 8.1 形态：可收缩右侧边栏

MVP 直接做右侧边栏（不走"先 panel 后边栏"渐进路径）。理由：
1. 与左侧 pack list 对称，三栏布局（左 pack list | 中工作区 | 右 agent），无新窗口范式。
2. 确认卡片需要宽度承载（warnings/preview）。
3. Tauri 单窗口，`useShellStore` 直接共享，无跨窗口同步。

### 8.2 边栏要点

- **可收缩到图标条**，一键唤出（参考左侧边栏 toggle 模式，复用 `shell_sidebar_*` 的样式约定）。
- **顶部上下文 chip**：显示"当前包：XXX"，呼应 §6.1 的状态注入。
- **确认操作内联渲染**：写操作的 `needs_confirmation` 确认卡片直接出现在对话流（复用现有 `AppDialog` 视觉），"应用 / 取消"按钮在消息里点。
- **assistant 回复 Markdown 渲染**：助手消息经 `MarkdownMessage`（react-markdown + remark-gfm）渲染，支持 GFM（表格 / 任务列表 / 代码块等）；**不启用原始 HTML**，模型输出无法注入 markup（防 XSS），链接强制外部打开。user / error / notice 仍是纯文本。
- **非模态**：边栏开着时主工作区仍可交互。
- **加载态**：非流式意味着请求期间整条 assistant 消息一次性出现，需要一个"思考中"的 loading 指示（spinner / 占位气泡）覆盖等待时间。

### 8.3 组件落点

- `src/features/agent/` — 边栏 UI（`AgentSidebar`）、agent loop（`agentLoop.ts` / `useAgentLoop.ts`）、system prompt（`systemPrompt.ts`）、Markdown 渲染（`MarkdownMessage.tsx`）、工具注册表。
- 边栏挂进 `src/app/App.tsx` 主 shell（与左侧 pack list 对称的右槽），并接收 `agentLanguage` prop。


---

## 9. 后续阶段（MVP 之后，按需生长）

MVP 跑通后，按真实需求逐步引入。每项都是独立增量，不需要重写 MVP：

| 阶段 | 内容 | 触发条件 |
|---|---|---|
| **Provider 抽象** | 抽出 `LLMProvider` 接口，加 Anthropic/OpenAI 适配器，统一类型 + 双向格式翻译 | 真的要接第二家 provider 时 |
| **流式输出** | SSE 中继 + `requestId` 关联 + listen-before-invoke + `llm_chat_cancel` 取消通道 + `done/error` 清理 | "逐字输出"成为体验痛点时 |
| **Key 安全** | 迁移到 OS keychain（Rust `keyring` crate），config 只存引用 | 需要更强 key 保护时；只改 Rust 取 key 处（§7.3） |
| **AI 写操作 review** | 对 Agent 发起的写操作叠一层内联 review（即使后端返 ok），settings 可关 | 用户反馈"AI 误改"成本高时 |
| **状态提升** | `agentContextStore`，把选中态/编辑态全局化 + 生命周期管理，支持"这张卡"指代 | 指代消解成为高频需求时 |
| **更多工具** | 删除/批量、文本编辑、资源导入、导入/导出 job（preview→execute→轮询） | 主链路稳定后逐步加 |
| **plan 模式** | `present_plan` 工具：渲染计划清单、阻塞 loop、用户确认/编辑 | 出现需先审计划的多步任务时 |
| **工具 schema** | 引 `zod` 统一 schema 来源，与 contracts 类型对齐 | 工具膨胀、手写 schema 难维护时 |

设计原则：复杂度进工具集和增量阶段，不提前焊进 MVP 控制流。

---

## 10. 需要新增的代码（汇总）

**前端：**
- `src/shared/api/agentApi.ts` — 包装 `llm_chat` command。
- `src/shared/contracts/agent.ts` — DeepSeek 请求/响应消息类型（OpenAI 兼容格式）。
- `src/features/agent/` — 边栏 UI、agent loop hook、`MarkdownMessage`、工具注册表（包装 `src/shared/api/*` wrapper）。
- `src/shared/stores/agentStore.ts` — 对话历史 + pending 状态。
- `src/shared/contracts/config.ts` — `GlobalConfig` 加 `deepseek_api_key`、`agent_language`（`AgentLanguage = "auto" | LanguageCode`）。
- settings UI — 独立 "AI 助手" tab：DeepSeek key 输入 + 数据外发提示 + 回复语言下拉。
- 依赖：`react-markdown` + `remark-gfm`（assistant 回复 Markdown 渲染）。

**后端：**
- `Cargo.toml`：`reqwest`（带 `json` feature；MVP 非流式，**不需要** `stream`）。
- 新 command `llm_chat`：注册到 `tauri_commands.rs` + `src-tauri/src/main.rs` 的 `generate_handler!`，实现放 `presentation/commands`。
- 新 application 服务 `application/llm/`：从 `GlobalConfig` 取 key、reqwest POST 转发、返回完整 JSON。
- `domain/config/model.rs`：`GlobalConfig` 加 `deepseek_api_key`、`agent_language` 字段（与前端 contract 同步）。
- `domain/config/rules.rs`：`agent_language` 的默认值、`SUPPORTED_AGENT_LANGUAGES` 校验与归一。

> `llm_chat` 的契约：输入 `{ body: <DeepSeek 请求 JSON> }`（前端组好，不含 key），后端注入 `Authorization` header 后转发，输出 DeepSeek 完整响应 JSON。Rust 不解析业务字段，只做"注入 key + 转发"。

---

## 11. 评审决策（已定）

1. **工具集范围**：保留 §5.1 全集——3 个只读工具（`list_cards` / `get_card` / `search_standard_cards`）+ 3 个写工具（`create_card` / `update_card` / `move_cards`）。
2. **指代消解**：MVP **不做状态提升**，"这张卡"靠用户给卡名/编号 + `list_cards` 定位（§6.2 的已知限制可接受）。
3. **明文 key gitignore**：`global_config.json` 不会提交（落地时核对 `.gitignore`）。
4. **对话历史持久化**：MVP **不做**，仅内存，关窗即清。
5. **最大工具循环轮数**：8。

---

## 12. 与现有架构边界的一致性

- ✅ Agent 工具体走 `src/shared/api/*`，不绕过（CLAUDE.md 规则）。
- ✅ 业务规则、校验、确认 token、编号策略全部留在后端（CLAUDE.md "后端拥有最终业务规则"）。
- ✅ 标准卡只读：agent 只暴露 `search_standard_cards` 等只读工具，不当可编辑包处理。
- ✅ 新 feature 放 `src/features/agent/`，新 API wrapper 放 `src/shared/api/`，新边界类型放 `src/shared/contracts/`，遵循既有分层。
- ⚠️ MVP 是 DeepSeek 专用、明文 key，与旧文档的 provider 中立 / keychain 方向不同——这是 MVP 阶段的有意取舍，扩展点保留在 §9。
- ⚠️ 评审通过、落地稳定后，需同步更新 `docs/README.md` 阅读顺序，并把事实写入核心文档。
