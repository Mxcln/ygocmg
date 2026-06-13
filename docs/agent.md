# YGOCMG AI Agent 功能

本文档描述当前实现中的 AI Agent 功能事实，是 agent 相关的当前权威文档。它不复述历史设计稿中的提案或未实现承诺；历史设计与评审见 `history/`。

## 产品定位

AI Agent 是嵌入 YGOCMG 的对话式助手，用户用自然语言对**当前激活的 custom pack** 完成卡片管理操作（查询、创建、修改、移动）。它由 DeepSeek 模型驱动，通过工具调用复用应用已有的后端业务规则，不绕过校验与确认流程。

## 范围

- 单一 provider：DeepSeek，非流式（后端一次性返回完整响应）。
- 工具调用循环：7 个只读工具 + 3 个写工具，写操作复用后端两段式确认门。
- 对话历史仅存内存，关闭应用即清空，不持久化。
- API key 以明文存于全局配置，不使用系统凭据库。

## 架构与数据流

```
前端 (React/TS)
  AgentSidebar (右侧边栏 UI)  ──  agentStore (Zustand: wire 历史 / display / pending)
        │
        ▼
  Agent Loop (useAgentLoop → agentLoop.runAgentTurn)
   - 组装 system prompt + 状态 block + 历史 messages + 工具定义
   - agentApi.chat() → 后端 llm_chat → DeepSeek
   - 解析 tool_calls → 工具注册表分发 → 执行体（包装 src/shared/api/*）
   - 写工具返回 needs_confirmation 时暂停，由 UI 内联确认
        │ Tauri IPC (llm_chat)
        ▼
后端 (Rust)
  llm_chat command → application/llm/LlmService
   - 从 GlobalConfig 读 deepseek_api_key（明文）
   - reqwest POST https://api.deepseek.com/chat/completions，注入 Authorization
   - 返回完整 JSON，不解析业务字段
        │ HTTPS
        ▼
  DeepSeek API
```

职责划分：

| 层 | 位置 | 职责 |
|---|---|---|
| 边栏 UI / 对话渲染 | 前端 React | 右侧边栏、消息渲染、内联确认卡片 |
| 对话状态 | 前端 Zustand (`agentStore`) | wire 历史、display 列表、pending 确认、状态机 |
| Agent loop | 前端 TS (`agentLoop` / `useAgentLoop`) | 循环、工具分发、组装 DeepSeek 请求 |
| 工具执行体 | 前端 TS (`features/agent/tools/*`) | 包装 `src/shared/api/*` wrapper |
| LLM HTTP 转发 | 后端 Rust (`application/llm`) | reqwest 转发 + 注入 key（非流式） |
| 业务规则 | 后端 Rust | 现有 service 层，agent 不引入新业务规则 |

HTTP 经后端转发而非 webview 直连：避免 CORS 与在网络面板暴露 key，并为后续迁移凭据库保留边界（届时只改 Rust 取 key 的方式）。

## Agent Loop

- 每轮请求由 `runAgentTurn` 组装：system message（system prompt + 语言指令 + 当前状态 block）、wire 历史、工具定义，`stream: false`。
- 模型返回 `finish_reason == "tool_calls"` 时，串行执行每个工具调用（写确认逐个处理），把 tool 结果回写进 wire 历史后再次请求。
- 模型返回最终文本时结束本轮。
- 最大工具循环轮数为 8，超过则提示停止以避免死循环。
- 当前状态 block 注入工作区名、激活 pack 名/id、当前视图、已打开 pack 列表，以及用户当前选中态——编辑抽屉打开的卡（Selected card）和批量勾选的卡（Checked cards），均来自 `useShellStore`。选中态只给名字+id 轻摘要；模型需要完整字段时用 id 调 `get_card`。仍看不到列表单击高亮态，指代不明时让用户澄清或用 `list_cards` 定位。

## 工具集与确认门

只读工具：`list_cards`、`get_card`、`search_standard_cards`（标准卡为只读参考库，与用户 pack 严格区分）、`get_config`（业务相关配置，不含 API key）、`get_pack_info`（已打开 pack 的完整 metadata，省略 packId 用当前激活 pack）、`list_packs`（workspace 内全部 pack 的 overview，含未打开的）、`suggest_card_code`（按编号策略推荐下一个可用 code）。

写工具：`create_card`、`update_card`、`move_cards`。

- 工具执行体调用 `src/shared/api/*` 现有 wrapper，不重新实现校验/编号/确认逻辑。
- 写工具返回的 `WriteResult` 若为 `needs_confirmation`，loop 暂停并在对话流内联渲染确认卡片（warnings/preview），用户点"应用 / 取消"。应用时用 confirmation token 调后端完成写入。

## 回复语言

agent 回复语言由全局配置 `agent_language` 决定：

- 取值 `auto`（默认，跟随程序 UI 语言）或显式 locale（`en-US` / `ja-JP` / `zh-CN`）。
- `auto` 在前端 `useAgentLoop` 解析为当前 UI locale；解析结果传给 `runAgentTurn` → `buildSystemPrompt(locale)`。
- `buildSystemPrompt` 在基础 system prompt 后追加语言指令，要求模型用目标语言撰写回复，但保持卡名、code 等数据值不变。
- 这是 prompt 层的软约束（引导而非强制），不做回复后语言检测重试。
- 后端 `agent_language` 受 `SUPPORTED_AGENT_LANGUAGES`（`auto` + 三种 locale）校验与归一，非法值回落 `auto`；旧配置文件缺该字段时 serde 默认 `auto`。

## 设置 UI

agent 相关设置集中在设置面板的独立 **"AI 助手"** tab（`settings.tab.agent`）：

- **DeepSeek API key** 输入框（密码型，写入 `deepseek_api_key`），附数据外发提示："对话内容与相关卡片数据会发送给 DeepSeek 服务商。"
- **回复语言** 下拉（写入 `agent_language`）：选项为"跟随程序语言"（`auto`）与三种显式 locale。
- key 未配置时，agent 边栏显示"未配置 API key"引导，并提供打开设置的入口。

## 边栏 UI

- agent 以可收缩的右侧边栏形态挂在主 shell（与左侧 pack list 对称），可收缩到图标条，宽度与折叠态由 `shell_right_sidebar_*` 配置承载。
- 顶部上下文 chip 显示"当前包：XXX"，呼应状态注入。
- **assistant 回复 Markdown 渲染**：助手消息经 `MarkdownMessage`（react-markdown + remark-gfm）渲染，支持 GFM（表格、任务列表、代码块等）；**不启用原始 HTML**，模型输出无法注入 markup（防 XSS），链接强制外部打开。user / error / system-notice 消息仍为纯文本。
- 写操作确认卡片内联渲染在对话流中。
- 非流式意味着请求期间整条 assistant 消息一次性出现，等待时显示"思考中"loading 指示。

## 代码落点

前端：

- `src/features/agent/` — 边栏 UI（`AgentSidebar`）、loop（`agentLoop.ts` / `useAgentLoop.ts`）、system prompt（`systemPrompt.ts`）、Markdown 渲染（`MarkdownMessage.tsx`）、工具注册表与执行体（`tools/*`，包装 `src/shared/api/*`）。
- `src/shared/api/agentApi.ts` — 包装 `llm_chat` command。
- `src/shared/contracts/agent.ts` — DeepSeek 请求/响应消息类型（OpenAI 兼容格式）。
- `src/shared/stores/agentStore.ts` — 对话历史、display、pending 状态。
- `src/shared/contracts/config.ts` — `GlobalConfig` 含 `deepseek_api_key`、`agent_language`（`AgentLanguage = "auto" | LanguageCode`）。
- 依赖：`react-markdown` + `remark-gfm`。

后端：

- `llm_chat` command — 注册于 `tauri_commands.rs`，实现在 `presentation/commands/app_commands`，转发逻辑在 `application/llm/LlmService`。
- `domain/config/model.rs` — `GlobalConfig` 含 `deepseek_api_key`、`agent_language` 字段。
- `domain/config/rules.rs` — `agent_language` 默认值、`SUPPORTED_AGENT_LANGUAGES` 校验与归一。

## 与架构边界的一致性

- agent 工具体走 `src/shared/api/*`，不绕过（CLAUDE.md 规则）。
- 业务规则、校验、确认 token、编号策略全部留在后端。
- 标准卡只读：agent 仅暴露 `search_standard_cards` 等只读工具，不当可编辑包处理。
- 新 feature 在 `src/features/agent/`，新 API wrapper 在 `src/shared/api/`，新边界类型在 `src/shared/contracts/`，遵循既有分层。

## 已知限制

- API key 明文存配置文件，未用系统凭据库（`global_config.json` 不应提交）。
- 对话历史不持久化，关闭即清。
- 选中态感知限于编辑抽屉与批量勾选，看不到列表单击高亮态。无流式输出，无 provider 抽象，无 AI 写操作额外 review 层。这些是当前阶段的有意取舍，扩展方向见 `history/` 中的设计稿。
