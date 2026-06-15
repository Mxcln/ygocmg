# YGOCMG AI Agent 功能

本文档描述当前实现中的 AI Agent 功能事实，是 agent 相关的当前权威文档。它不复述历史设计稿中的提案或未实现承诺；历史设计与评审见 `history/`。

## 产品定位

AI Agent 是嵌入 YGOCMG 的对话式助手，用户用自然语言完成两个层级的操作：**卡片管理**（在当前激活的 custom pack 内查询、创建、修改、移动、删除卡片）和 **pack 管理**（在当前 workspace 内打开/切换/关闭/新建/删除 pack、改 pack metadata）。它由 DeepSeek 模型驱动，通过工具调用复用应用已有的后端业务规则，不绕过校验与确认流程。

agent 的操作边界对标成熟代码 agent：pack 是卡片的作用域与容器（≈ 目录 / manifest），归 agent；而切换 workspace（≈ 打开另一个 folder）与修改应用设置（≈ 改 editor settings）刻意排除，留给用户在 UI 操作。

## 范围

- 单一 provider：DeepSeek，非流式（后端一次性返回完整响应）。
- 工具调用循环：9 个只读工具 + 12 个写工具（6 个卡片/系列名写 + 6 个 pack 写）。
- pack 写操作经一个 UI/agent 共用的**命令层**（纯函数 + 注入依赖）编排；卡片写复用后端两段式确认门。
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
   - 解析 tool_calls → 工具注册表分发 → 执行体（包装 src/shared/api/* 或 pack 命令层）
   - 卡片写工具返回 needs_confirmation 时暂停（后端 token）；pack 删除返回命令层 needs_confirmation 时暂停（commit 闭包），均由 UI 内联确认
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
| 工具执行体 | 前端 TS (`features/agent/tools/*`) | 包装 `src/shared/api/*` wrapper 或 pack 命令层 |
| pack 命令层 | 前端 TS (`features/commands/*`) | 纯函数编排 pack 操作，UI 与 agent 共用 |
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

只读工具：`list_cards`、`get_card`、`search_standard_cards`（标准卡为只读参考库，与用户 pack 严格区分）、`get_config`（业务相关配置，不含 API key）、`get_pack_info`（已打开 pack 的完整 metadata，省略 packId 用当前激活 pack）、`list_packs`（workspace 内全部 pack 的 overview，含未打开的）、`suggest_card_code`（按编号策略推荐下一个可用 code）、`list_setnames`（合并 pack 与标准 setname，返回 `{key, name, source}` 列表，供模型在系列名与 setcode 数字之间双向翻译；pack 同 key 覆盖 standard，镜像 `useMergedSetnameEntries` 的合并逻辑）、`validate_lua_script`（验证当前选中卡或指定卡片的已保存 Lua 脚本，默认返回 static + `ocgcore_init` 报告）。

`validate_lua_script` 只读并复用 `scriptApi.validateLuaScript`，不会读取脚本文件、修改脚本或自动修复。省略 `cardId` 时使用当前 Selected card；没有 Selected card 时要求模型让用户指定卡片或先打开卡片。工具可显式传 `levels`，但仅允许 `static`、`ocgcore_init`、`smoke`、`scenario`；默认不传 levels，让后端使用当前默认。`ocgcore_init` 只证明脚本可加载并执行 `initial_effect`，不证明效果语义正确。

卡片写工具：`create_card`、`update_card`、`move_cards`、`delete_cards`、`create_setname`、`delete_setname`。`update_card` 覆盖全部可编辑字段（name/desc、atk/def/level、primary_type、race、attribute、monster_flags、spell_subtype、trap_subtype、pendulum、link markers、setcodes、ot、alias、category、code）：读全卡 → 仅对传入字段打补丁 → 回写，枚举字段在工具层校验取值（非法值就地报错，不丢给后端）。`move_cards` 与 `delete_cards` 均为批量形态，传一个 card id 即表示单张操作；`delete_cards` 调用后端批量删除并默认同时删除主卡图、场地图和脚本资源，确认时走 `confirmCardBatchWrite`。系列成员关系是卡片的 `setcodes` 数字数组，模型应先用 `list_setnames` 查到对应 key 再写入。`create_setname` 新建/重命名 pack 自定义系列名（写 setname string）。setcode key 是 16 位值：低 12 位为 base（系列本体），高 4 位为 child（子系列索引，顶级系列为 0）。可传 hex key（含 child 半字节，用于构造子系列）；省略时由后端 `suggest_setname_key` 命令在 config 推荐 base 区段（`setname_base_recommended_min/max`，默认 0x0300–0x0FFF）内分配下一个空闲顶级 base，避开当前包/工作区其他包/标准包已用 base。返回的 key 再由 `update_card` 写入卡片 setcodes。该工具走 pack-strings 确认门（`confirmPackStringsWrite`）而非卡片确认门，刷新 strings 与 setname 相关缓存。该 config 区段同时驱动 setname 写入时的越界警告（`validate.rs`），与 card code 推荐区段对称。`delete_setname` 按 setcode key（hex）删除自定义系列名：后端 `delete_pack_strings` 直接返回 ok、无确认 token，故该工具走命令层 commit 闭包确认模式（同 `delete_pack`，loop 的 `isCommandConfirmation` 分支处理），闭包内调 `deletePackStrings` 并自行刷新 strings/setname 缓存。它只删名字，不解除卡片 setcodes 中对该 key 的引用。

pack 写工具：`switch_pack`、`open_pack`、`close_pack`、`create_pack`、`update_pack_meta`、`delete_pack`。它们经命令层编排（见下），在 UI 即时生效。`delete_pack` 为破坏性操作，需用户确认。agent 不暴露 workspace 切换与 config 修改工具——这些超出 pack/card 边界，由用户在 UI 操作。

### 命令层（UI / agent 共用）

pack 写操作的编排逻辑收敛在一个与 React 无关的**命令层**（`src/features/commands/`）：每个命令是纯函数 `(...args, deps) => Promise<CommandResult<T>>`，副作用句柄（shellStore 的 pack actions + queryClient）通过 `deps` 注入。`CommandDeps = { shell, queryClient }`，不含 config/workspace。

- UI 适配器 `useCommands()` 在 React 内用 `useQueryClient` + shellStore 选择器组装 `deps`；agent 适配器 `buildAgentDeps()` 在 React 外用 `useShellStore.getState()` + 模块级 `queryClient` 单例组装。两者调同一组命令，实现 UI 与 agent 在 pack 操作上的一致。
- `App.tsx` 的 `persistActivePack` / `handleClosePack` 已收敛为转调命令；其余受 Modal 持有后端调用的 handler 维持原状（避免二次后端调用）。

### 两套确认门并存

- **卡片写（后端 token）**：写工具返回的 `WriteResult` 若为 `needs_confirmation`，loop 暂停并内联渲染确认卡片（warnings/preview），用户应用时用 confirmation token 调后端完成写入。确认端点和写后缓存刷新由工具声明：`AgentTool` 的可选 `confirmWrite`（默认 `cardApi.confirmCardWrite`）与 `invalidateKeys`（默认卡片缓存 `["cards"]`/`["card"]`）。`create_setname` 借此走 `confirmPackStringsWrite` 并刷新 strings/setname 缓存；`create_card` 和 `update_card` 沿用默认单卡确认；`move_cards` 与 `delete_cards` 使用批量确认端点 `confirmCardBatchWrite`。
- **pack 删除（命令层 commit 闭包）**：`delete_pack` 命令返回 `{status:"needs_confirmation", confirmation:{summary, commit}}`，不立即执行。agent 侧由 loop 的命令确认分支走 `requestConfirmation`，确认后调 `commit()`。命令层另提供 UI 适配器 `useCommands().deletePackWithDialog`（经 `AppDialog` 确认后调 `commit()`）供 UI 复用；现阶段 `PackMetadataPanel` 的删除入口仍走其自有的 `openDialog` + `packApi.deletePack` 路径，UI 侧收敛是渐进的。
- 两套机制数据模型不同（后端 token vs 前端闭包），有意并存不强行统一；loop 中 `confirmationToken` 放宽为 `string | null` 以共用同一确认 UI 通道。
- 工具执行体调用 `src/shared/api/*` 或 pack 命令层，不重新实现校验/编号/确认逻辑。

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

- `src/features/agent/` — 边栏 UI（`AgentSidebar`）、loop（`agentLoop.ts` / `useAgentLoop.ts`）、system prompt（`systemPrompt.ts`）、Markdown 渲染（`MarkdownMessage.tsx`）、工具注册表与执行体（`tools/*`，卡片工具包装 `src/shared/api/*`，`tools/scriptTools.ts` 包装 Lua 脚本验证，pack 工具 `tools/packTools.ts` 转调命令层）。
- `src/features/commands/` — pack 命令层：`types.ts`（`CommandDeps` / `CommandResult` / `ConfirmationRequest`）、`packCommands.ts`（六个 pack 命令）、`useCommands.ts`（UI 适配器）、`buildAgentDeps.ts`（agent 适配器）。`queryClient` 单例从 `src/app/providers.tsx` 导出供 agent 适配器访问。
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
- 脚本验证只读：`validate_lua_script` 仅返回后端验证报告，不修改脚本，不承诺效果语义正确。
- 新 feature 在 `src/features/agent/`，新 API wrapper 在 `src/shared/api/`，新边界类型在 `src/shared/contracts/`，遵循既有分层。

## 已知限制

- API key 明文存配置文件，未用系统凭据库（`global_config.json` 不应提交）。
- 对话历史不持久化，关闭即清。
- 选中态感知限于编辑抽屉与批量勾选，看不到列表单击高亮态。无流式输出，无 provider 抽象，无 AI 写操作额外 review 层。agent 操作限 pack/card 层级，不含 workspace 切换、config 修改、导入导出。这些是当前阶段的有意取舍，扩展方向见 `history/` 中的设计稿。
