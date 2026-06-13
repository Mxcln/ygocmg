# YGOCMG AI Agent 功能设计文档（提案）

> 状态：**设计提案，待评审**。本文档描述的是计划中的实现，不是当前事实。评审通过并落地稳定后，再把相应内容同步进核心文档（`functional_spec.md` / `system_architecture.md` / `code_structure_api.md` / `ui_design.md`）。
>
> 关键决策：**LLM 调用层 provider 中立**（支持 Anthropic / OpenAI / OpenAI 兼容端点），不绑定单一 SDK。

> **修订（第 1 轮评审后）**：本版根据 Claude Reviewer + Codex Reviewer 的评审收紧了四处此前不可落地的设计：
> 1. **适配层 / Rust 代理边界**（§2、§3.5、§7.3）—— 明确为"前端生成 provider-specific 请求体，Rust 只注入 key + 转发裸 SSE"，解决"Rust 薄代理转发 provider-neutral 请求"的矛盾。
> 2. **流式 IPC 生命周期协议**（新增 §7.5）—— 定义 `requestId` 关联、listen-before-invoke、`llm_chat_cancel` 取消通道、`done/error` 清理。
> 3. **选中态/编辑态前置改造**（§6.2、阶段 1）—— 这些状态当前是组件局部 state，不在全局 store，agent 够不到；新增前置改造项。
> 4. **AI 写操作确认策略**（§5.3）—— 明确即使后端返 `ok`，Agent 发起的写操作也先内联 review；阶段 2 验收随之更正。
> 其余更正：command 数 45→53、Phase 0 禁止明文 config key、`LLMStreamEvent` 补 `pause_turn`、并发写工具串行确认、确定性序列化对 map 排序、Zod schema 作为独立 runtime source。

---

## 1. 背景与目标

为 YGOCMG 增加一个对话式 AI Agent，用户通过自然语言完成卡片管理操作：修改卡片、移动卡片、批量操作、文本编辑、导入/导出、查询标准卡参考数据等。

### 设计原则

1. **复用后端业务规则**：Agent 是 `src/shared/api/*` 的新调用方，不重新实现校验、确认、编号等业务逻辑。
2. **后端拥有最终业务规则**（遵循 CLAUDE.md）：危险操作的确认门由后端 `WriteResult` / `preview→execute` 机制强制。
3. **Provider 中立**：统一 LLM 抽象层，agent loop 与具体厂商解耦。
4. **起点简单、按需生长**：从单步工具调用循环起步,复杂度进工具集而非控制流。
5. **不绕过 `src/shared/api`**：Agent 工具执行体调用现有 API wrapper。

### 非目标（至少第一阶段）

- 不做 router 层（工具分流）。
- 不做独立 plan/执行两阶段控制流（plan 以工具形式按需引入）。
- 不做多 agent / 子 agent 编排。
- 不接入 Anthropic Managed Agents（其工具跑在 Anthropic 容器,够不到本机文件）。

---

## 2. 架构总览

```
┌─────────────────────────── 前端 (webview, TS/React) ───────────────────────────┐
│                                                                                  │
│  AgentSidebar (UI)  ──→  agentStore (Zustand, 对话历史)                          │
│        │                                                                         │
│        ▼                                                                         │
│  Agent Loop (provider 无关)                                                      │
│   - 组装 LLMRequest（messages + tools + 状态快照）                               │
│   - 解析 LLMStreamEvent（text / tool_call / done）                              │
│   - 工具分发 → 执行体                                                            │
│        │                          │                                              │
│        │ tool_call                │ LLMRequest                                   │
│        ▼                          ▼                                              │
│  Tool Registry            LLMProvider 适配层                                     │
│   - 只读工具              ┌─ AnthropicProvider ─┐                                │
│   - 写工具（确认门）       └─ OpenAIProvider ────┘  (格式翻译在前端)             │
│        │                          │                                              │
│        ▼                          │ 规范化请求 + provider 标识                   │
│  src/shared/api/* wrapper         │                                              │
│   (invokeApi → Tauri command)     │                                              │
└──────────┼────────────────────────┼─────────────────────────────────────────────┘
           │ Tauri IPC               │ Tauri IPC (llm_chat_stream)
           ▼                         ▼
┌─────────────────────────── 后端 (Rust) ────────────────────────────────────────┐
│  现有 53 个 command（卡片/包/文本/资源/导入导出/标准卡/job）                      │
│  + 新增 LLM 代理 command：llm_chat_stream（透明 HTTP transport，见 §3.5）        │
│       - 从 OS keychain 取 key                                                    │
│       - reqwest 转发前端已组好的 provider-specific 请求体                        │
│       - 注入 key header，回传裸 SSE chunk（不解析、不翻译）                       │
│  + 取消 command：llm_chat_cancel（按 requestId 中止 reqwest stream，见 §7.5）    │
│  + key 管理 command：set/has/clear llm credential（写 keychain）                 │
└──────────────────────────────────────────────────────────────────────────────┘
                                     │ HTTPS (key 永不进 webview)
                                     ▼
                    Anthropic / OpenAI / OpenAI 兼容端点
```

### 职责划分

| 层 | 位置 | 职责 |
|---|---|---|
| UI | 前端 React | 聊天界面、确认卡片、上下文 chip |
| 对话状态 | 前端 Zustand | 对话历史、pending 状态 |
| Agent loop | 前端 TS | provider 无关的循环、工具分发、状态注入 |
| Provider 适配 | 前端 TS | 各厂商请求/响应/工具格式 ↔ 内部统一类型 |
| 工具执行体 | 前端 TS | 包装 `src/shared/api/*` wrapper |
| LLM HTTP + key | 后端 Rust | reqwest 转发、keychain 存取、SSE 中继 |
| 业务规则 | 后端 Rust | 现有 service 层（不改动） |

**为什么适配在前端、HTTP 在后端**：适配逻辑（消息/工具格式翻译）放前端便于和 loop 内聚;HTTP 转发放后端使 API key 不进 webview。两者通过一个轻量的 Tauri command 边界衔接。

---

## 3. LLM Provider 抽象层

### 3.1 设计目标

OpenAI 与 Anthropic 的请求/响应/工具调用格式不同,但 agent loop 只应写一遍。分两层:**provider 无关的 loop 只认内部统一类型;适配层负责双向翻译。**

### 3.2 内部统一类型（草案，供评审）

```typescript
// src/features/agent/llm/types.ts
type ProviderKind = "anthropic" | "openai_compatible";

interface ProviderConfig {
  kind: ProviderKind;
  baseUrl?: string;        // OpenAI 兼容端点: Ollama / LM Studio / DeepSeek 等
  model: string;
  // key 不在这里 —— key 存 keychain,由后端按 providerId 取
  providerId: string;      // 指向 keychain 条目
  extra?: Record<string, unknown>;  // thinking/effort 等 provider 专有,透传
}

interface LLMMessage {
  role: "user" | "assistant";
  content: LLMContentBlock[];
}
type LLMContentBlock =
  | { type: "text"; text: string }
  | { type: "tool_use"; id: string; name: string; input: unknown }
  | { type: "tool_result"; tool_use_id: string; content: string; is_error?: boolean };

interface LLMToolDef {
  name: string;
  description: string;
  input_schema: object;    // JSON Schema(由 Zod 生成)
}

interface LLMRequest {
  messages: LLMMessage[];
  tools: LLMToolDef[];
  system?: string;
  contextBlock?: string;   // 当前程序状态快照(见 §6)
}

// 规范化流式事件 —— 两家不同的 tool_use / function_call 在此归一
type LLMStreamEvent =
  | { type: "text"; delta: string }
  | { type: "tool_call"; id: string; name: string; input: unknown }
  | { type: "done"; stopReason: "end_turn" | "tool_use" | "max_tokens" | "refusal" }
  | { type: "error"; message: string };

interface LLMProvider {
  chat(req: LLMRequest, signal: AbortSignal): AsyncIterable<LLMStreamEvent>;
}
```

### 3.3 适配层职责

| Provider | tool 调用回写格式 | 适配要点 |
|---|---|---|
| Anthropic | `tool_use` block + `tool_result` block(`tool_use_id` 配对) | 工具结果作为 user 消息里的 `tool_result` block |
| OpenAI 兼容 | `tool_calls` + `role:"tool"` 消息 | 工具结果作为独立 `role:"tool"` 消息,`tool_call_id` 配对 |

- 工具 JSON Schema 用 **Zod 定义一次**,适配层分别转成各 provider 格式(两家工具定义几乎一致,差异最小)。
- provider 专有能力(Anthropic `thinking`/`effort`)放 `extra` 透传,适配层各取所需,OpenAI 侧忽略。
- 一个 `OpenAIProvider` 配自定义 `baseUrl` 即可覆盖 Ollama / LM Studio / DeepSeek / 硅基流动 等一大片本地与第三方模型。

### 3.5 适配层 ↔ Rust 代理边界（评审修正 #1）

**评审指出原文矛盾**:一处说前端适配层做"格式翻译",另一处说前端把"provider-neutral 的 `LLMRequest`"发给 Rust 薄代理转发。这两件不能同时成立——OpenAI/Anthropic 的 endpoint **不接受** provider-neutral 请求体,所以 Rust 不能只做"薄转发"。必须明确边界。本设计采用 **方案 1**:

| | 方案 1(采用):前端适配,Rust 做安全 HTTP transport | 方案 2(备选):Rust 适配 |
|---|---|---|
| 前端 adapter | 生成 **provider-specific** 的 body / path / headers(除 key 外),解析 provider-specific SSE 流 → 归一为 `LLMStreamEvent` | 只发统一 `LLMRequest`,只认统一事件 |
| Rust | 注入 key,转发到 endpoint,**原样回传裸 SSE chunk**(不解析、不翻译) | 接收统一请求,后端完成格式转换 + SSE 解析,emit 统一事件 |
| 取舍 | 适配逻辑与 loop 内聚在前端;Rust 极薄、provider 无关 | key 与解析都在后端,但 Rust 要随每个 provider 演进 |

**采用方案 1 的理由**:provider 适配频繁变化(新 provider、格式微调),放前端 TS 迭代快、与 loop 内聚;Rust 端保持极薄且 provider 无关,只承担"注入 key + 转发字节 + 取消"三件事,长期维护成本低。

因此 `llm_chat_stream` 的输入**不是** `LLMRequest`,而是 `{ providerId, url, method, headers(不含 key), body }` —— 前端 adapter 已组好的 provider-specific HTTP 请求;Rust 注入 key header 后转发,裸 SSE 回传。§7.3 的描述据此修正。

> ⚠️ 这把"是否支持 Vercel AI SDK"的问题收窄了:方案 1 下 Rust 是一个 raw-SSE 代理,SDK 必须允许把它的 fetch/transport 指向这个 Tauri 边界(见 §3.6 与 §9 阶段 0 的前移验证项)。

### 3.6 是否直接用 Vercel AI SDK

**评审决策点。** Vercel AI SDK(`ai`)已经把"多 provider 抽象 + 工具调用 + 流式"做成成品,与本节诉求几乎一一对应,可省掉自写适配器。两条路:

| 路径 | 优点 | 风险 |
|---|---|---|
| **A. 用 Vercel AI SDK 当抽象层** | 成品、TS 原生、省维护 | 框架接管 loop,需验证能否干净地"暂停 → 弹确认 → 续跑"(见 §5 审批门) |
| **B. 自写薄适配层(§3.2)** | 完全控制 loop 与审批门 | 多写并维护 adapter |

**建议:** 先用一个最小切片验证 Vercel AI SDK 的审批门暂停能力(让 agent 调一个返回 `needs_confirmation` 的写工具)。链路顺 → 选 A;别扭 → 退回 B,或混合(框架只做 LLM 调用,loop 自写)。**两条路的上层 loop 接口一致,这个选择不影响其它章节设计。**

> ⚠️ key 安全约束(§7)要求 LLM HTTP 经 Rust 代理。Vercel AI SDK 跑在前端,需要它支持自定义 transport/fetch 指向 Rust 代理 command,否则 key 会进 webview。这是选 A 的第二个验证点。

---

## 4. Agent Loop（控制流复杂度）

### 4.1 核心决策:简单 loop,复杂度进工具集而非控制流

**第一阶段只做单一的工具调用循环,不引入 router 层,不引入独立 plan 阶段。** 把 loop 设计成"能长出 plan 层"的形状,而不是把 plan 焊死进去。

```
1. 组装 LLMRequest(system + tools + 状态快照 + 历史)
2. provider.chat() 流式返回
3. 收到 tool_call → 工具注册表分发 → 执行体调 src/shared/api wrapper
4. 写工具返回 needs_confirmation → 暂停,UI 弹确认 → 用户确认 → confirm_*_write
5. 工具结果作为 tool_result 回写历史
6. stopReason == tool_use → 回到 2;== end_turn → 结束本轮
```

### 4.2 三个能力层次（按需生长）

| 层次 | 任务示例 | 实现 |
|---|---|---|
| **L0 单步** | "把这张卡 ATK 改成 3000" | 基础 loop，1 次工具调用 |
| **L1 隐式多步** | "把所有 N 卡攻击力 +100" | 同一个 loop,模型自己先 `list_cards` 再循环 `update` |
| **L2 显式 plan(第二阶段)** | "把这个系列重编号并导出" | 注册 `present_plan` 工具,渲染计划清单 UI、阻塞 loop、等用户确认/编辑 |

**关键:** L2 的规划做成**工具**而非**控制流层**。`present_plan(steps)` 渲染清单、阻塞循环、可被用户改——与 Claude Code 把"提问"做成工具是同一个"Rendering"模式。这样基础实现简单,要支持复杂任务时只是多注册一个工具,不重写架构。

### 4.3 明确排除

- **Router 层**:工具集精选后约 10 个,远未到需要分流的规模。真到工具膨胀(几十个),先上 **tool search**(模型按需发现工具,不破坏 prompt cache),而不是手写 router。
- **独立 plan/执行两阶段**:两套控制流、两次状态传递,复杂易错。用 §4.2 的工具式 plan 替代。

---

## 5. 工具注册表与审批门

### 5.1 工具分类（精选,不要把 45 个命令全暴露）

工具太多会降低模型选择准确率。精选一组面向用户意图的工具,分读/写两类:

**只读工具（自动执行,无需确认）:**
- `list_cards` / `get_card` / `search_standard_cards` / `list_pack_overviews` / `list_pack_strings` / `get_standard_card`

**写工具（走已有确认门）:**
- `create_card` / `update_card` / `delete_card` / `move_cards` / `bulk_delete_cards`
- `upsert_pack_string` / `delete_pack_strings`
- 资源类:`import_main_image` / `delete_main_image` / `import_script` 等(视范围裁剪)

**异步任务工具:**
- `preview_export_bundle` + `execute_export_bundle` / `preview_import_pack` + `execute_import_pack`(返回 job,需轮询)

**状态/视图工具(见 §6):**
- `get_app_state` / `get_current_view`

### 5.2 工具定义方式

每个工具:
- 参数用 **Zod schema** 定义(新增 `zod` 依赖)。**评审修正**:当前项目无 `zod`,且 TS `interface` 运行时不可用,**不能直接"复用 contracts 类型"自动生成 schema**。正确做法二选一:
  - (a) 新增 `src/shared/contracts/*Schema.ts` 作为运行时 schema 的事实来源,再 `z.infer` 出 TS 类型;或
  - (b) 保留现有 TS contracts,Agent 工具 schema 作为独立 runtime schema,用类型测试保证与 API input 对齐。
- 执行体 = 调对应 `src/shared/api/*` wrapper(`cardApi.updateCard` 等)。
- 描述里**写明触发条件**(不只是功能):如 *"当用户要求新建或添加一张卡片时调用"*。近期 Opus 模型对工具更保守,触发条件能显著提升调用率。

### 5.3 审批门 —— 复用后端两段式写入 + AI 写操作 review

这是本项目相对一般 agent 的**核心优势**:后端已经强制了"危险操作要确认",前端几乎不用自己设计拦截逻辑。

**评审修正(产品策略澄清):** 后端 `WriteResult` 只有 `ok` / `needs_confirmation` 两态。普通改 ATK 很可能返 `ok`、不弹后端确认。但 "AI 误解了这张卡" 的成本偏高,所以本设计采用 **双层确认**:

- **后端确认门**(`needs_confirmation`):后端业务规则要求的确认,强制。
- **AI 操作 review 层**:**对 Agent 发起的任何写操作**,即使后端返 `ok`,也先在对话流内联展示一次可取消的 review(展示将写入什么),用户点"应用"才真正提交。MVP 即启用,默认开;可在 settings 提供"信任 AI 写操作、跳过 review"开关给进阶用户。

```
写工具执行体调 cardApi.updateCard(input)
   ↓ 先内联渲染 "AI 想执行的写操作" review 卡片(展示 input/diff)
   ├─ 用户取消 → 把 "用户取消" 作为 tool_result 回写,模型据此调整
   └─ 用户应用 → 调 cardApi.updateCard(input)
        ↓ 后端返回 WriteResult<T>
        ├─ status: "ok" → 把 data 作为 tool_result 回写,loop 继续
        └─ status: "needs_confirmation"
              → 再内联渲染后端确认卡片(warnings/preview)
              → 用户确认 → confirmCardWrite(confirmation_token) → 回写,loop 继续
```

**并发写工具的排队策略(评审补充):** L1 任务(如"所有 N 卡 +100")会让模型连发多个 `update_card`。MVP 采用 **串行、逐个 review**:一个写工具 review/确认完再处理下一个,避免一次弹 N 张卡片。后续可优化为"聚合成一张批量 review 卡片"。

异步 job 同理:写工具调 `executeXxx` 拿到 `job_id` → 轮询 `getJobStatus` → 把最终状态(成功/失败/进度)作为 tool_result 回写。

| 操作类型 | 审批策略 |
|---|---|
| 只读 | 自动执行 |
| 任何 Agent 写操作 | 先 AI review(可在 settings 关闭),再走后端 |
| 后端返 ok | review 通过后自动提交,UI 显示结果 |
| 后端返 needs_confirmation | review 后再叠加后端确认卡片 |
| 导入/导出 | preview 内联展示 → 用户确认 → execute → 轮询进度 |

---

## 6. 程序状态感知

### 6.1 原则:有选择地注入,而非全量灌入

Agent 需要知道当前状态以消解指代("这张卡""选中的""当前包"),但不能把整个程序状态塞进上下文。分两类处理:

| 状态类型 | 例子 | 处理方式 |
|---|---|---|
| **核心上下文**(小、稳定、几乎每条指令都用) | active pack、选中卡、正在编辑的卡、当前视图 | 每轮**快照**,放消息层(非 system) |
| **视图条件**(中等、偶尔需要) | 排序/过滤方式 | 不主动给,提供 `get_current_view()` 工具按需查 |
| **数据内容**(大、易变) | 卡片列表、卡片完整字段 | 不主动给,用 `list_cards`/`get_card` 拉 |

### 6.2 核心上下文快照来源（含前置改造，评审修正 #3）

**评审指出一个实质问题:** 文档原文把"当前选中卡 / 正在编辑的卡"当作现成的全局状态,但实际上它们**不在 `useShellStore` 里**,agent loop 够不到:

- `useShellStore`(`src/shared/stores/shellStore.ts`)只有 `workspaceId` / `openPackIds` / `activePackId` / `activeView` / `packMetadataMap` / `modal` / `dialog` —— 这部分文档说得准,可直接用。
- **选中卡 id** 是 `CardListPanel` 的组件局部 state(props `selectedCardIds` / `onSelectionChange` 传入)。
- **正在编辑的卡 id** 是 `PackWorkArea` / `CardEditDrawer` 的局部 state。

后果:不做改造的话,"这张卡""选中的这几张"这类指代消解会落空 —— 而这正是 §6.1 的核心用例。

**前置改造项(必须先做,放进阶段 1):** 新增一个 `agentContextStore`(或把相关 feature 选择/编辑态提升/同步到全局),让 agent loop 能读到。并定义其**生命周期**:切换 pack、退出选择模式、关闭编辑 drawer、切到标准包视图时,这些上下文如何清空,避免给 agent 陈旧的选中态。

核心快照字段:
- `workspaceId` / `workspaceName`(来自 shellStore)
- `openPackIds` / `activePackId` / `packMetadataMap`(来自 shellStore)
- `activeView`(来自 shellStore)
- 当前选中卡 id 列表 / 正在编辑的卡 id（**来自新增的 `agentContextStore`,非现成**）

### 6.3 注入方式（关键:不破坏 prompt cache）

**状态易变,绝不能进 system prompt 或工具定义** —— 否则每条指令都让 prefix 变化,整个缓存失效(prefix-match 原则)。

- **稳定部分**(system prompt、工具定义)放最前,永不随状态变 → 可缓存。
- **当前状态**作为轻量上下文块,放在**消息序列**里:
  - 起步方案:拼到当轮 user message 前的一个上下文块(provider 无关,简单)。
  - 进阶:在支持的 provider 上用 mid-conversation system message(更具操作者权威);抽象层吸收差异。
- 快照要**确定性序列化**(字段顺序固定),否则序列化抖动也会影响后续缓存命中。

### 6.4 快照式 vs 工具式（结合使用）

- **快照式**:每次用户发消息时读一份当前状态附在消息前 → 让基础指代能消解。
- **工具式**:`get_app_state()` / `get_current_view()` 只读工具 → agent 需要最新/精确值时自取。
- 两者结合:快照给基础上下文,工具给按需精确数据。

---

## 7. API Key 存储与安全

### 7.1 威胁模型

这是**本地单用户**工具,key 在用户自己机器上。主要威胁不是"别人偷看",而是:
1. key 明文进了会被同步/提交的文件(如 `GlobalConfig` JSON)。
2. 被 webview 里的任意代码读到(将来 agent 写的脚本、第三方依赖)。

### 7.2 方案对比

| 方案 | 安全性 | 成本 |
|---|---|---|
| A. 存 `GlobalConfig` 明文 | 低 | 最低 |
| **B. OS 密钥库(推荐)** | 高 | 中 |
| C. 环境变量 | 中 | 桌面 app 不便 |

**推荐方案 B**:用 Rust `keyring` crate 把 key 存进操作系统凭据管理器(Windows Credential Manager / macOS Keychain)。`GlobalConfig` 只存「provider 配置 + 指向 keychain 条目的引用(providerId)」,真正的 key 不落明文文件。

### 7.3 架构决策:key 由 Rust 后端持有

**LLM HTTP 请求收到 Rust 后端做透明 HTTP transport（不是 provider-neutral 代理）:**
- 前端 adapter 组好 **provider-specific** 的 HTTP 请求(url / method / headers(不含 key) / body),发给 Tauri command `llm_chat_stream`。
- Rust 从 keychain 取 key,注入到 header,用 `reqwest` 转发给 provider endpoint。
- Rust **原样回传裸 SSE chunk**(不解析、不翻译),前端 adapter 解析 provider-specific 流 → 归一为 `LLMStreamEvent`。

> 这与 §3.5 的方案 1 边界一致:Rust 是安全 HTTP transport,不理解 provider 格式。流式 IPC 的具体协议(requestId 关联、取消、清理)见 §7.5。

这样:
- key 永不进 webview,前端代码/依赖偷不到。
- agent loop 逻辑(provider 适配、工具分发)仍在前端。
- 只有「发 HTTP」这一步在 Rust。

### 7.4 需要新增的后端能力

- `Cargo.toml`:`reqwest`(带 `stream` feature)、`keyring`。
- 新 command:`llm_chat_stream`(代理 + SSE 中继)、`set_llm_credential` / `has_llm_credential` / `clear_llm_credential`(keychain 读写)。
- `GlobalConfig`:加 provider 配置列表(不含 key)。

> ⚠️ 安全护栏:LLM 请求会把对话内容(可能含卡片数据)发到外部 provider。这是用户配置 provider 时的明确选择,但 settings UI 应明示「对话与相关卡片数据会发送给所选 LLM 服务商」。
>
> **数据最小化(评审补充):** 只读工具会把卡片列表/描述/标准卡结果送进上下文。应设默认上限:`list_cards` 默认分页上限、优先传摘要行而非完整字段、用户可关闭"自动读取大批量内容"。避免一条"列出所有卡"指令把整个包灌进 LLM。

### 7.5 流式 IPC 生命周期协议（评审修正 #2）

**评审指出**:现有 `invokeApi` 是普通 Promise(`src/shared/api/invoke.ts`),现有事件总线只有全局 job 事件名、无 request correlation(`runtime/events/`、`infrastructure/tauri_event_bus.rs`)。`chat(req, signal): AsyncIterable` + "Tauri event 中继"缺一个可实现的 wire protocol,否则 MVP 核心链路会反复返工。本节定义该协议:

| 关注点 | 约定 |
|---|---|
| **请求关联** | 前端生成 `requestId`(uuid),随 `llm_chat_stream` 一起传;所有该请求的 event 携带此 `requestId`。 |
| **监听顺序** | **listen-before-invoke**:前端先 `listen("llm_chat_chunk")`(按 requestId 过滤),再调用 `llm_chat_stream`。避免早期 chunk 丢失。 |
| **事件名** | 单一全局事件名 `llm_chat_chunk`,payload 内含 `requestId` 区分并发会话;前端按 requestId 路由到对应 loop。 |
| **payload schema** | `{ requestId, seq, kind: "chunk"｜"done"｜"error", data }` —— `chunk` 带裸 SSE 字节/行,`done` 收尾,`error` 带 message。`seq` 单调递增用于排序与丢包检测。 |
| **取消** | 新增 `llm_chat_cancel(requestId)` command。前端 `AbortSignal` → 调 cancel → Rust 侧中止该 `requestId` 的 reqwest stream。**Tauri event 单向,必须有独立 cancel 通道**(评审补充 #4)。 |
| **清理** | 收到 `done`/`error` 后前端 `unlisten`;Rust 在 stream 结束或 cancel 后释放该 requestId 的资源。前端组件卸载时也调 cancel。 |
| **背压** | MVP 阶段可不做精细背压(对话流量低);若后续出现快producer/慢consumer,再在 Rust 侧加缓冲上限。 |

这套协议是 MVP 必须先定的,后端 `llm_chat_stream` / `llm_chat_cancel` 和前端 `agentApi` 都依赖它。

---

## 8. UI 设计

### 8.1 形态选型:可收缩的右侧边栏(入口用图标)

| 方案 | 评价 | 适配度 |
|---|---|---|
| 独立 agent 窗口(VSCode/Cursor 式) | 脱离主工作区上下文,多窗口状态同步麻烦 | 低 |
| AI 图标 → 弹出子聊天窗口 | 实现轻,但浮层窄、塞不下确认卡片/富 UI | 中(适合 MVP) |
| **可 toggle 的右侧边栏** | 与左侧 pack list 对称,与主工作区并排可见,宽度够承载确认 UI | **高(目标形态)** |

**结论:目标形态是右侧边栏,入口用图标。** 核心理由:
1. **上下文可见性**:agent 操作的是用户当前看的卡;边栏让对话与卡列表同屏,改完即见。
2. **确认门要渲染富 UI**:`needs_confirmation` / `preview` 的确认卡片(带 diff、受影响卡片列表)需要边栏的宽度。
3. **布局对称**:左 pack list | 中工作区 | 右 agent,经典三栏,无新窗口管理范式。
4. **Tauri 单窗口最省事**:状态(`useShellStore`)直接共享,不跨窗口同步。

### 8.2 边栏设计要点

- **可收缩到细图标条**(类似左侧边栏 toggle/shrink),一键唤出 —— 入口=图标,展开=边栏。
- **顶部上下文 chip**:"当前包:XXX · 选中 3 张卡",呼应 §6 的状态注入,也给用户手动纠正上下文的入口。
- **确认操作内联渲染**:写操作的确认卡片直接出现在对话流(复用 `AppDialog` 视觉),"应用 / 取消"按钮在消息里点。
- **非模态**:边栏开着时主工作区仍可交互。

### 8.3 渐进路径

- **MVP**:可先做轻量 panel(方案 2 的稍大版本),跑通"对话 → 只读工具 → 显示结果"。
- **稳定后**:演进到右侧边栏。底层同一个 React 组件 + 同一个 agent loop,容器从浮层换边栏,逻辑不重写。

---

## 9. 分阶段实现计划

每个阶段都可独立验证、独立合并。

### 阶段 0 — 垂直切片(验证链路)

目标:跑通"对话 → 只读工具 → 显示结果"。

- 后端:`llm_chat_stream` 透明 transport command(§3.5 方案 1)+ `llm_chat_cancel`,实现 §7.5 的 IPC 协议(requestId 关联、listen-before-invoke、`done/error` 清理)。
- **Key 处理(评审修正 #3):不允许明文 config key。** 阶段 0 用以下之一:① 环境变量;② 一次性内存 key(进程级,不落盘);③ mock provider 或本地无 key provider(如 Ollama)。或直接提前实现最小 `set/has/clear_llm_credential`(keychain)。**绝不把明文写进 `GlobalConfig`** —— 它会落到 `global_config.json`,与本文档安全目标(§7.1)直接冲突。
- 前端:一个 provider 适配器(先做一家)、统一类型、最小 agent loop。
- 工具:2-3 个只读工具(`list_cards`、`get_card`)。
- UI:临时聊天 panel(不必是最终边栏)。
- **验收**:用户问"列出当前包里的卡",agent 调 `list_cards`,正确显示。
- **关键风险前移验证(评审修正 #2):** 若考虑 Vercel AI SDK 路线,在此阶段验证**两个**点——(a) 审批门暂停能力;(b) **SDK 的 transport/fetch 能否被劫持到 Rust 代理 IPC**(§3.5)。第 (b) 点与 key 是否安全无关,但决定方案 A 是否可行,必须在阶段 0 验证,否则可能阶段 3 才发现 SDK 不让改 transport 而被迫返工。

### 阶段 1 — Provider 抽象层成型

- 抽出 `LLMProvider` 接口,实现 `AnthropicProvider` + `OpenAIProvider` 两个适配器。
- 配置面:settings 里可选 provider 类型 / endpoint / model。
- 状态注入(§6):核心上下文快照拼到消息层。
- **验收**:同一段对话能在两家 provider 间切换(切换时按缓存规则重建会话)。

### 阶段 2 — 写操作 + 审批门

- 接入写工具(`create_card`、`update_card`、`move_cards` 等)。
- 实现 §5.3 的确认门:`needs_confirmation` → 内联确认卡片 → `confirm_*_write`。
- **验收**:用户说"把这张卡 ATK 改成 3000",agent 调用、弹确认、用户确认后写入成功。

### 阶段 3 — Key 安全 + 异步任务

- key 迁移到 OS keychain(`keyring`),config 只存引用。
- `set/has/clear_llm_credential` command + settings UI。
- 接入导入/导出工具:`preview→execute→轮询 job`,进度上报。
- **验收**:key 不出现在任何明文文件;导出当前包全程对话驱动。

### 阶段 4 — UI 定型 + 体验打磨

- 演进到可收缩右侧边栏(§8.2),上下文 chip、内联确认、非模态。
- 错误处理:`refusal` / `pause_turn` / rate limit / 网络错误的友好提示。
- 工具描述调优(触发条件)、对话历史持久化(可选)。
- **验收**:边栏与左侧 pack list 对称,完整可用。

### 阶段 5(可选,看需求)— plan 模式

- 出现真正多步、需先审计划的任务时,加 `present_plan` 工具(§4.2)。
- **验收**:复杂任务先出计划清单、用户确认后逐步执行。

---

## 10. 需要新增的代码(汇总)

**前端:**
- 依赖:LLM 抽象层(自写适配器或 Vercel AI SDK)、`zod`(工具 schema)。
- `src/shared/api/agentApi.ts` — 包装 `llm_chat_stream` 等 command。
- `src/shared/contracts/agent.ts` — 统一 LLM 类型(§3.2)。
- `src/features/agent/` — 边栏 UI、agent loop hook、工具注册表(包装 `src/shared/api/*` wrapper)、provider 适配层。
- `src/shared/stores/agentStore.ts` — 对话历史 + pending 状态。

**后端:**
- `Cargo.toml`:`reqwest`(`stream` feature)、`keyring`。
- 新 command:`llm_chat_stream`、`set/has/clear_llm_credential`(`tauri_commands.rs` + `presentation/commands`)。
- 新 application 服务:`application/llm/`(代理 + keychain 取值)。
- `domain/config/model.rs`:`GlobalConfig` 加 provider 配置列表(不含 key)。

---

## 11. 待评审的开放问题

1. **Provider 抽象:自写 vs Vercel AI SDK?** §3.6 列了取舍,核心未知数是审批门暂停能否与框架自动 loop 干净配合,以及 SDK transport 能否劫持到 Rust 代理 IPC(§3.5)—— 两点都建议阶段 0 用最小切片定论。
2. **状态注入方式:** 起步用消息层快照;是否在支持的 provider 上启用 mid-conversation system message(更具操作者权威)留待阶段 1 评估。
3. **工具集范围:** §5.1 是初始建议集,资源类工具(图片/脚本导入)是否纳入第一批待定。
4. **对话历史持久化:** 是否跨会话保存对话(涉及存储位置与隐私),阶段 4 决定。
5. **数据外发告警:** settings UI 如何明示"对话与卡片数据会发送给所选 LLM 服务商"(§7.4)。
6. **与 CLAUDE.md skill 规则的张力:** 本设计是 provider 中立的,与仓库内 Claude-only skill 方向相反 —— 已按用户多 provider 需求优先,评审请确认这是预期方向。

---

## 12. 与现有架构边界的一致性

- ✅ Agent 工具体走 `src/shared/api/*`,不绕过(CLAUDE.md 规则)。
- ✅ 业务规则、校验、确认 token、编号策略全部留在后端(CLAUDE.md "后端拥有最终业务规则")。
- ✅ 标准卡只读:agent 只暴露 `search_standard_*` / `get_standard_*` 只读工具,不当可编辑包处理。
- ✅ 新 feature 放 `src/features/agent/`,新 API wrapper 放 `src/shared/api/`,新边界类型放 `src/shared/contracts/`,遵循既有分层。
- ⚠️ 评审通过后,需同步更新 `docs/README.md` 阅读顺序,并在稳定后把事实写入核心文档。
