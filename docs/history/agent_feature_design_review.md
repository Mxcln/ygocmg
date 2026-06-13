# Claude Reviewer

核对完毕。文档对现有架构的关键断言基本属实——WriteResult 两段式写入、confirmation_token、shellStore 字段、export 的 preview→execute、get_job_status 轮询都对得上。下面是评审意见。

总体评价
这是一份质量很高的设计提案。难得的是它把复杂度放在了正确的地方：审批门复用后端已有的 needs_confirmation 机制，而不是前端自己造拦截逻辑；plan 做成工具而非控制流层；状态注入考虑了 prompt cache 的 prefix-match。这些决策都站得住脚。架构分层（适配在前端、HTTP+key 在后端）与 §80 的理由也自洽。可以进入实现，但有几处需要在动工前澄清。

必须修正的事实错误
1. command 数量不对。 §59 和 §410 写"现有 45 个 command"，实测是 53 个（tauri_commands.rs 有 53 处 #[tauri::command]）。这是会被反复引用的数字，落地时还要写进核心文档，建议直接改对或改成"50+ 个，不必精确"。

设计层面的硬伤
2. §6.2 的选中态/编辑态当前不在全局 store，agent loop 够不到。 这是最实质的问题。文档把"当前选中卡 id 列表 / 正在编辑的卡 id"列为每轮注入的核心上下文，并说"来自卡片 feature 的选择态"。但实际上：

shellStore 只有 workspaceId / openPackIds / activePackId / activeView / packMetadataMap——这部分文档说得准。
选中态是 CardBrowserPanel.tsx:56 通过 props (selectedCardIds / onSelectionChange) 传入的，宿主是某个父组件的局部 state，不在任何全局 store 里。编辑态同理（CardEditDrawer）。
后果：agent loop 无法读取选中/编辑态，"这张卡""选中的这几张"这类指代消解会落空——而这恰恰是 §6.1 开篇举的核心用例。文档把它当成现成数据，实则需要先把选择/编辑态上提到全局 store。建议在 §6.2 或 §10 显式加一条前置改造项，否则阶段 1 会卡在这里。

3. §3.4 与阶段 0 的依赖顺序矛盾，是真实的进度风险。 Vercel AI SDK 路线（方案 A）有两个验证点：审批门暂停（§156）+ 自定义 transport 指向 Rust 代理以保 key 不进 webview（§161）。但阶段 0（§369）明确"先临时用环境变量/明文 key，不碰 key 安全"。也就是说阶段 0 验证不了第二个、也是更可能让方案 A 出局的验证点（key 必须经 Rust 代理 ↔ SDK 是否允许把 fetch 重定向到 Tauri command）。建议把"SDK 能否自定义 transport 走 Rust 代理"这个验证前移到阶段 0，哪怕 key 还是临时明文——验证的是"流式 fetch 能否被劫持到 IPC"这个机制，与 key 是否安全无关。否则可能阶段 0 选了 A，到阶段 3 才发现 SDK 不让改 transport，被迫返工。

建议补强（不阻塞）
4. SSE 中继的取消/背压没提。 §7.3 说"流式结果通过 Tauri event 中继回前端"。但 §135 的 chat(req, signal: AbortSignal) 要求可取消——用户在边栏点"停止"时，前端 abort 后Rust 侧的 reqwest stream 怎么终止？Tauri event 是单向的，需要一个配套的 llm_chat_cancel command 或 channel 机制。建议在 §7.4 的新增 command 列表里补上取消通道。

5. 错误事件枚举不全。 §131 的 done.stopReason 列了 refusal，但 §399 又把 refusal 和 pause_turn 当错误处理。两处对 refusal 的归类不一致，且 pause_turn（Anthropic 长任务暂停）没进 §128 的 LLMStreamEvent 类型。这是适配层归一化的关键，建议统一。

6. 多个写工具结果的并发/顺序未定义。 L1（§185"把所有 N 卡 +100"）会让模型连续发多个 update_card。如果其中一个返回 needs_confirmation 暂停了 loop，其余 tool_call 怎么排队？是串行执行（一个确认完再下一个）还是批量收集确认？这关系到 UI 是弹 N 张确认卡片还是 1 张聚合卡片。建议在 §5.3 补一句策略，哪怕先定"串行、逐个确认"。

7. §6.3 的"确定性序列化"对 packMetadataMap 是真约束。 它是 Record<>，JS 对象 key 顺序虽通常稳定但不可依赖。提一句"快照序列化时对 map 的 key 排序"会让这条建议可执行。

# Codex Reviewer

**发现**

- **[高] Provider 适配职责和 Rust 代理职责互相打架。**  
  文档一处说 `Provider 适配` 在前端，负责“各厂商请求/响应/工具格式 ↔ 内部统一类型”（[docs/agent_feature_design.md](D:/Game/YGODIY/ygocmg/docs/agent_feature_design.md:74)），但安全章节又说前端把“规范化的 `LLMRequest`”交给 `llm_chat_stream`，Rust 只用 `reqwest` 转发到 provider endpoint（[docs/agent_feature_design.md](D:/Game/YGODIY/ygocmg/docs/agent_feature_design.md:311)）。这两件事不能同时成立：OpenAI/Anthropic endpoint 不接受 provider-neutral `LLMRequest`，所以 Rust 不能只是“薄代理”转发。  
  建议二选一写清楚：  
  1. **前端适配，Rust 只做安全 HTTP transport**：前端 adapter 生成 provider-specific body/path，Rust 只注入 key 并转发 raw SSE chunk，前端再解析 provider-specific stream。  
  2. **Rust 适配，前端只认统一事件**：Rust 接收统一 `LLMRequest`，在后端完成 provider 格式转换和 SSE 解析，再 emit `LLMStreamEvent`。  
  当前文档如果不先定这个边界，`LLMProvider.chat()`、Vercel AI SDK 试验、`agentApi.ts` 和 `llm_chat_stream` 都没法稳定落实现。

- **[高] 流式 IPC 缺少可实现的生命周期协议。**  
  设计把前端接口写成 `chat(req, signal): AsyncIterable<LLMStreamEvent>`（[docs/agent_feature_design.md](D:/Game/YGODIY/ygocmg/docs/agent_feature_design.md:134)），同时说后端通过 Tauri event 中继流式结果（[docs/agent_feature_design.md](D:/Game/YGODIY/ygocmg/docs/agent_feature_design.md:312)）。但当前 API wrapper 是普通 Promise `invokeApi`（[src/shared/api/invoke.ts](D:/Game/YGODIY/ygocmg/src/shared/api/invoke.ts:13)），现有事件总线也只是全局 job 事件名，没有 request correlation（[src-tauri/src/runtime/events/mod.rs](D:/Game/YGODIY/ygocmg/src-tauri/src/runtime/events/mod.rs:10)，[src-tauri/src/infrastructure/tauri_event_bus.rs](D:/Game/YGODIY/ygocmg/src-tauri/src/infrastructure/tauri_event_bus.rs:16)）。  
  文档需要补一个明确 wire protocol：`requestId` 如何生成，前端是先 listen 再 invoke 还是 command 返回 stream id，事件名是否全局，payload schema 是什么，`done/error` 如何清理，多个并发聊天如何避免串流，`AbortSignal` 如何映射到 Rust 取消请求，以及前端 unlisten 时 Rust reqwest 是否终止。缺这个，MVP 的核心链路会在实现时反复返工。

- **[高] Phase 0 允许明文 config key，直接违反同一文档的安全目标。**  
  安全章节正确指出主要威胁之一是 key 明文进入 `GlobalConfig` JSON（[docs/agent_feature_design.md](D:/Game/YGODIY/ygocmg/docs/agent_feature_design.md:295)），并建议 keychain、config 只存引用（[docs/agent_feature_design.md](D:/Game/YGODIY/ygocmg/docs/agent_feature_design.md:307)）。但阶段 0 又允许“明文 config 里的 key”（[docs/agent_feature_design.md](D:/Game/YGODIY/ygocmg/docs/agent_feature_design.md:367)）。当前 `GlobalConfig` 会保存到 `global_config.json`（[src-tauri/src/infrastructure/json_store/mod.rs](D:/Game/YGODIY/ygocmg/src-tauri/src/infrastructure/json_store/mod.rs:21)，[src-tauri/src/infrastructure/json_store/mod.rs](D:/Game/YGODIY/ygocmg/src-tauri/src/infrastructure/json_store/mod.rs:87)），所以这不是抽象风险。  
  建议把 Phase 0 改成：只允许环境变量、一次性内存 key、mock provider、本地无 key provider，或直接提前实现最小 `set/has/clear_llm_credential`。不要把“明文 config”作为可接受过渡方案写进设计。

- **[中] 审批门语义与阶段 2 验收标准不一致。**  
  §5.3 写明 `status: "ok"` 时直接回写 tool result，单卡写入后端返 ok 会自动执行（[docs/agent_feature_design.md](D:/Game/YGODIY/ygocmg/docs/agent_feature_design.md:229)，[docs/agent_feature_design.md](D:/Game/YGODIY/ygocmg/docs/agent_feature_design.md:243)）；现有 `WriteResult` 也确实只有 `ok` / `needs_confirmation` 两种（[src/shared/contracts/card.ts](D:/Game/YGODIY/ygocmg/src/shared/contracts/card.ts:294)）。但阶段 2 的验收又说“把这张卡 ATK 改成 3000”会“弹确认、用户确认后写入成功”（[docs/agent_feature_design.md](D:/Game/YGODIY/ygocmg/docs/agent_feature_design.md:383)）。普通 ATK 修改很可能是 `ok`，不会弹确认。  
  建议明确产品策略：Agent 是否对**所有写操作**增加一层“AI 操作确认”，还是只在后端 `needs_confirmation` 时暂停。两者都合理，但验收、UI 和安全文案必须一致。我的倾向是：MVP 至少对 Agent 发起的写操作显示一次可取消的 inline review，即使后端返回 `ok`，否则“AI 误解了这张卡”的成本会偏高。

- **[中] “当前选中卡 / 正在编辑卡”并不能直接从 `useShellStore` 快照得到。**  
  设计说核心上下文“直接对应现有 `useShellStore` + 选中态”，包括选中卡和正在编辑卡（[docs/agent_feature_design.md](D:/Game/YGODIY/ygocmg/docs/agent_feature_design.md:265)）。但当前 `shellStore` 只有 workspace、open pack、active view、modal/dialog 等 shell 状态（[src/shared/stores/shellStore.ts](D:/Game/YGODIY/ygocmg/src/shared/stores/shellStore.ts:48)）；选中卡 ID 是 `CardListPanel` 的本地 state（[src/features/card/CardListPanel.tsx](D:/Game/YGODIY/ygocmg/src/features/card/CardListPanel.tsx:34)），编辑中的 card id 是 `PackWorkArea` 的本地 state（[src/app/PackWorkArea.tsx](D:/Game/YGODIY/ygocmg/src/app/PackWorkArea.tsx:28)）。  
  建议在设计里新增一个 `agentContextStore` 或把相关 feature state 提升/同步出来，并定义生命周期：切换 pack、退出选择模式、关闭 drawer、标准包视图时这些 context 如何清空。否则“这张卡”“选中的卡”会是 MVP 最容易出错的指代。

- **[中] “复用 contracts 类型自动生成 Zod schema”表述过于乐观。**  
  文档说工具参数用 Zod schema，复用 `src/shared/contracts/*` 类型并自动生成 JSON Schema（[docs/agent_feature_design.md](D:/Game/YGODIY/ygocmg/docs/agent_feature_design.md:217)），但当前项目没有 `zod` 依赖（[package.json](D:/Game/YGODIY/ygocmg/package.json:12)），而 TypeScript `interface` 在运行时不可用，不能直接“复用类型”生成工具 schema。  
  建议把这里改成更可执行的方案：新增 `src/shared/contracts/*Schema.ts` 作为运行时 schema source of truth，再 `z.infer` 出 TS 类型；或者保持现有 TS contracts，把 Agent 工具 schema 作为独立 runtime schema，并用类型测试/适配测试保证和 API input 对齐。

**开放问题**

- Vercel AI SDK 路线如果保留，需要先回答它到底调用什么：浏览器 fetch 一个真实 HTTP URL、Tauri command transport、还是 Rust raw-SSE proxy。当前 §3.4 已经意识到 key 不能进 webview（[docs/agent_feature_design.md](D:/Game/YGODIY/ygocmg/docs/agent_feature_design.md:150)），但还没把“SDK transport 能否落在 Tauri IPC 上”变成明确验收项。
- 自动执行只读工具会把卡片列表、描述、标准卡查询结果继续送进 LLM 上下文。§7.4 有外发告警（[docs/agent_feature_design.md](D:/Game/YGODIY/ygocmg/docs/agent_feature_design.md:327)），但建议补一条数据最小化规则，例如默认分页上限、只传摘要、敏感路径不传、用户可关闭自动读取大批量内容。

**总结**

整体方向是对的：Agent 作为 `src/shared/api/*` 的新调用方、业务规则留在后端、标准包只读，这些都和当前架构一致。真正需要在评审阶段收紧的是“LLM transport/adapter 到底在哪一层”“流式 Tauri 协议怎么取消和关联”“key 是否允许任何明文过渡”“AI 写入是否需要额外确认”。这些定下来后，分阶段计划会稳很多。

我没有修改文件，也没有运行测试；这次是文档与现有代码/核心文档对照审阅。