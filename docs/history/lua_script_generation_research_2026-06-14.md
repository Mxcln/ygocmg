# Lua Script Generation Research

本文档记录 2026-06-14 对“让 AI Agent 根据卡片效果生成 YGOPro Lua 脚本”的调研结论。它是未来设计和实现的参考，不描述当前已实现功能；当前事实仍以 `../functional_spec.md`、`../system_architecture.md`、`../code_structure_api.md`、`../ui_design.md` 和 `../agent.md` 为准。

## 背景

YGOCMG 当前已经有 AI Agent，用户可以通过对话式工具调用完成卡片、系列名和 pack 管理。Agent 的实现边界是前端驱动工具调用循环，工具执行体包装 `src/shared/api/*` 或 pack 命令层，后端继续拥有业务规则、确认门和文件系统写入。

脚本生成能力的目标不是单纯让模型写一段 Lua，而是帮助用户从卡片效果描述得到一份可落地、可检查、可修复、最终可写入 custom pack 的 YGOPro 兼容脚本。由于 YGOPro 脚本既依赖 Lua 语法，也依赖 ocgcore 暴露的 `Duel`、`Card`、`Effect`、`Group`、`aux` 和常量体系，因此该能力应设计成带检索、模板和验证闭环的工程流程。

## 调研对象

本次主要查看了本仓库现有文档、当前 Agent 架构、`ref/ygopro_src` 中的 YGOPro/ocgcore 源码和脚本库，以及 `ref/expansions` 中的自定义卡包示例。

主要事实：

- `ref/ygopro_src/script` 中有 `11846` 个 Lua 文件，其中 `11843` 个是 `c{id}.lua` 形式的卡片脚本，另外 `3` 个是 `constant.lua`、`utility.lua`、`procedure.lua`。
- `ref/expansions/script` 中有 `87` 个 Lua 文件，其中 `86` 个是卡片脚本，另有 `package/infernacryz.lua` 这类系列公共 helper。
- `ref/ygopro_src/ocgcore` 中包含 YGOPro 脚本引擎核心，C API 暴露 `set_script_reader`、`set_card_reader`、`set_message_handler`、`create_duel`、`new_card`、`preload_script` 等入口。
- YGOCMG 当前标准包 SQLite 索引保存了标准卡详情、文本、查询行、资源状态和 `has_script`，但尚未保存脚本文本、脚本结构化索引或 effect-block 索引。
- YGOCMG 已有 CDB 读写适配，可以从 `datas` / `texts` 读取 card metadata、文本、raw type、setcode、level 等字段，并可导出 CDB。

## 总体结论

Lua script generation 不应被设计成一个“单个 tool”。它更适合放在 **workflow 层**：由 Agent 对话触发，一个脚本生成工作流编排多个小 tool，完成收集上下文、解析效果、检索参考、选择模板、生成草稿、验证、修复和写入确认。

建议抽象：

```text
Agent 对话层
  用户提出：帮这张卡写脚本 / 修脚本 / 参考同系列脚本

Workflow 层
  generate_lua_script
  repair_lua_script

Tool 层
  get_card
  list_setnames
  search_script_cases
  get_script_template
  analyze_lua_script
  validate_lua_script
  write_card_script
  open_script_external

后端 / 基础设施层
  标准脚本索引
  模板库
  ocgcore validator helper
  资源写入
```

核心判断：

- **workflow** 负责“怎么做”：拆解、检索、生成、验证、修复、确认。
- **tool** 负责“能做什么”：搜索案例、取模板、验证脚本、写脚本。
- **后端** 负责事实和边界：文件系统、标准索引、资源归属、ocgcore 验证、写入确认。
- **LLM** 负责语言理解、局部代码生成和修复，但不拥有最终业务规则。

## 不推荐纯 Embedding 检索

卡片效果文本存在大量“表面相似但脚本语义完全不同”的情况。Embedding 可以用于候选重排，但不应成为主检索方式。

典型风险：

- “送去墓地”可能是 cost，也可能是 effect operation。脚本上分别落在 `SetCost` 和 `SetOperation`，并且 `REASON_COST` / `REASON_EFFECT` 不同。
- “无效发动”和“无效效果”文本相近，但脚本上可能分别是 `Duel.NegateActivation` 和 `Duel.NegateEffect`。
- “加入手卡”可能是检索、回收、抽卡替代、公开确认、非公开移动，涉及 `CATEGORY_SEARCH`、`CATEGORY_TOHAND`、`Duel.ConfirmCards` 等差异。
- “特殊召唤”可能是召唤手续、起动效果、诱发效果、Quick Effect、规则特召、从手牌/墓地/卡组/额外卡组特召，脚本结构差异很大。
- “一回合一次”可能是普通 count limit、同名 count limit、同组多个效果共用或排他，脚本 `SetCountLimit` 参数不同。

更可靠的检索单位是 **effect block**，而不是整张卡：

- 每个 `Effect.CreateEffect(c)` 到 `c:RegisterEffect(eN)` 或 clone/register 视作一个 effect block。
- 从 block 中提取 `SetCategory`、`SetType`、`SetCode`、`SetRange`、`SetCountLimit`、`SetCondition`、`SetCost`、`SetTarget`、`SetOperation`。
- 再提取 `Duel.*`、`Card.*`、`aux.*` 调用和 `CATEGORY_*`、`EVENT_*`、`EFFECT_TYPE_*`、`LOCATION_*` 等常量。
- 检索时先用结构化字段过滤，再使用文本相似度或 embedding 重排。

## 脚本库模板化证据

对 `ref/ygopro_src/script` 标准卡片脚本做了轻量统计，结果显示 YGOPro 脚本有很强模板性：

- `Duel.SetOperationInfo` 出现 `14277` 次，覆盖 `9201` 个文件。
- `Duel.Hint` 出现 `13710` 次，覆盖 `8421` 个文件。
- `Duel.IsExistingMatchingCard` 出现 `9279` 次，覆盖 `6160` 个文件。
- `Duel.GetLocationCount` 出现 `7128` 次，覆盖 `4374` 个文件。
- `aux.Stringid` 出现 `14054` 次，覆盖 `8072` 个文件。
- `EFFECT_TYPE_SINGLE` 出现 `12859` 次，覆盖 `7289` 个文件。
- `EFFECT_TYPE_FIELD` 出现 `7714` 次，覆盖 `5435` 个文件。
- `EFFECT_TYPE_ACTIVATE` 出现 `5019` 次，覆盖 `4511` 个文件。
- `EVENT_FREE_CHAIN` 出现 `4857` 次，覆盖 `4463` 个文件。
- `EVENT_SPSUMMON_SUCCESS` 出现 `1898` 次，覆盖 `1790` 个文件。
- `EVENT_TO_GRAVE` 出现 `1115` 次，覆盖 `1097` 个文件。

按脚本中的 `e1`、`e2` 等 effect 变量粗略统计：

- 有 effect 变量的脚本约 `11714` 个。
- 每个脚本平均 effect 变量数约 `2.26`。
- 中位数为 `2`。
- 最高约 `10`。

高频 effect block 签名示例：

- `SetDescription > SetCategory > SetType > SetCode > SetCondition > SetTarget > SetOperation`
- `SetDescription > SetCategory > SetType > SetRange > SetCountLimit > SetCost > SetTarget > SetOperation`
- `SetDescription > SetCategory > SetType > SetProperty > SetCode > SetCondition > SetTarget > SetOperation`
- `SetDescription > SetCategory > SetType > SetCode > SetTarget > SetOperation`
- `SetType > SetCode > SetValue`
- `SetType > SetCode > SetRange > SetTargetRange > SetTarget > SetValue`

这些数据支持“模板库 + 结构化案例检索 + 局部生成”的路线，而不是让模型从零自由写整张卡脚本。

## 建议的数据模型

### Script Case

脚本案例建议从标准脚本和 custom pack 脚本中抽取，按 effect block 存储。

示例结构：

```ts
type ScriptCase = {
  code: number;
  name: string;
  desc: string;
  descBullet?: string;
  source: "standard" | "custom_pack" | "workspace";
  packId?: string;
  setcodes: number[];
  cardMeta: {
    primaryType: "monster" | "spell" | "trap";
    monsterFlags?: string[];
    spellSubtype?: string;
    trapSubtype?: string;
    race?: string;
    attribute?: string;
    level?: number;
  };
  effectBlock: {
    index: number;
    signature: string[];
    categories: string[];
    effectTypes: string[];
    events: string[];
    locations: string[];
    countLimit?: string;
    hasCondition: boolean;
    hasCost: boolean;
    hasTarget: boolean;
    hasOperation: boolean;
    duelCalls: string[];
    cardCalls: string[];
    auxCalls: string[];
    constants: string[];
    luaSnippet: string;
  };
};
```

### Script Template

模板库应手工或半自动维护，覆盖常见效果类型。模板不是完整脚本，而是带 slot 的结构化片段。

示例结构：

```ts
type ScriptTemplate = {
  id: string;
  title: string;
  intentTags: string[];
  requiredCardMeta?: Record<string, unknown>;
  effectSignature: string[];
  slots: Array<{
    name: string;
    description: string;
    type: "setcode" | "location" | "category" | "predicate" | "operation" | "number" | "code";
    required: boolean;
  }>;
  luaSkeleton: string;
  examples: number[];
};
```

模板库第一阶段建议覆盖：

- 通常魔法/陷阱发动：`EFFECT_TYPE_ACTIVATE + EVENT_FREE_CHAIN`。
- 检索：`CATEGORY_SEARCH + CATEGORY_TOHAND`，`Duel.IsExistingMatchingCard`，`Duel.SelectMatchingCard`，`Duel.SendtoHand`，`Duel.ConfirmCards`。
- 特殊召唤：`CATEGORY_SPECIAL_SUMMON`，`Duel.GetLocationCount`，`IsCanBeSpecialSummoned`，`Duel.SpecialSummon`。
- 破坏、除外、送墓、回卡组：`Duel.Destroy`、`Duel.Remove`、`Duel.SendtoGrave`、`Duel.SendtoDeck`。
- 手牌诱发 / 连锁无效：`EVENT_CHAINING`，`Duel.IsChainDisablable`，`Duel.NegateEffect` / `Duel.NegateActivation`。
- 墓地触发：`EVENT_TO_GRAVE`，`IsPreviousLocation`，`EFFECT_FLAG_DELAY`。
- 永续限制：`EFFECT_TYPE_FIELD`，`EFFECT_CANNOT_SPECIAL_SUMMON`，`SetTargetRange`，`SetTarget`。
- 召唤手续：`aux.AddSynchroProcedure`、`aux.AddXyzProcedure`、`aux.AddLinkProcedure`、融合/仪式相关 procedure。
- 灵摆：`aux.EnablePendulumAttribute(c)`，灵摆效果与怪兽效果分离。
- 系列共通锁：同字段 `splimit`、`sfilter`、`setcode` 过滤。

## 检索策略

检索应优先使用结构化过滤和符号匹配，再用文本相似度或 embedding 重排。

推荐排序：

1. 当前 pack 内同 setcode / 同系列的已写脚本。
2. 当前 workspace 其他已打开 custom pack 中同 setcode / 同系列脚本。
3. 标准包中同 setcode / 同字段脚本。
4. 标准包中同 card metadata + effect signature 的脚本。
5. 全库弱语义候选。

硬过滤条件示例：

- 卡片主类型：monster / spell / trap。
- 具体 subtype：Quick-Play、Continuous、Counter、Field、Equip、Ritual、Link、Xyz、Synchro、Pendulum。
- 触发位置：hand / grave / field / monster zone / spell & trap zone / extra deck。
- 触发事件：summon success、special summon success、to grave、destroyed、chaining、battle damage。
- 操作类别：search、special summon、destroy、remove、negate、draw、send to grave。
- 是否 target。
- 是否 cost。
- 是否 quick effect。
- 是否 once per turn。
- 是否本家 setcode。

这样可以避免 embedding 把“表面相近但脚本语义错”的案例排在前面。

## 同字段 / 同系列参考

同字段参考非常值得做。`ref/expansions` 中的 CDB 和脚本可以按 code join，setcode 聚类也能发现相近模式。例如：

- `pixel.cdb` 的 `0xffa` 系列。
- `Tzyceedra.cdb` 的 `0xffe` 系列。
- `Infernacryz` / `地火军` 的 `0xffd` 系列。
- `trap_pend.cdb` 的 `0xff1` 系列。

同系列脚本可以提供：

- 系列特召限制的写法。
- 本家过滤函数命名与复用方式。
- setcode 常量写法。
- 是否存在 package helper。
- 系列字段中常见的 `SetCountLimit` 编号策略。
- 同字段卡在多个效果之间共享限制的模式。

未来 YGOCMG 可以在 workspace/custom pack 层建立“本地脚本案例索引”，并优先提供给 Agent。

## ocgcore 验证调研

### 可用入口

`ref/ygopro_src/ocgcore/ocgapi.h` 暴露了验证需要的关键 C API：

- `set_script_reader(script_reader f)`：提供脚本内容读取。
- `set_card_reader(card_reader f)`：提供 `cards.cdb` 的 `datas` 信息。
- `set_message_handler(message_handler f)`：捕获错误消息。
- `create_duel(seed)`：创建 duel 实例。
- `set_player_info(...)`：设置玩家初始信息。
- `new_card(...)`：向 duel 状态加入卡片。
- `process(...)`：推进 duel。
- `query_card(...)` / `query_field_card(...)`：查询状态。
- `preload_script(...)`：预加载脚本。

解释器初始化时会加载：

- `./script/constant.lua`
- `./script/utility.lua`
- `./script/procedure.lua`

ocgcore 在 `interpreter::load_script` 中通过 `luaL_loadbuffer` 和 `lua_pcall` 加载并执行脚本，错误写入 `pduel->strbuffer` 并调用 message handler。

### 重要细节：不要只用 preload_script 验证卡片脚本

很多脚本使用新式风格：

```lua
local s,id,o=GetID()
```

`GetID()` 依赖 `self_code` 和 `self_table`。这两个变量是在 `interpreter::load_card_script` 加载 `c{id}.lua` 前设置的，而不是单纯 `preload_script` 自动具备的。

因此验证卡片脚本时更稳的方式是：

1. 提供 `script_reader`，能读取 `constant.lua`、`utility.lua`、`procedure.lua`、生成中的 `c{id}.lua` 和可能的 package helper。
2. 提供 `card_reader`，返回该卡及必要参考卡的 `card_data`。
3. 调用 `create_duel`。
4. 用 `new_card` 把该卡放入合适区域，让 ocgcore 注册 card 并触发 `load_card_script + initial_effect`。
5. 捕获 message/log。

`preload_script` 更适合验证普通 helper 脚本是否能加载，而不是直接验证卡片脚本的完整初始化环境。

## 验证层级

脚本正确性不能一次性完全验证。建议分层建设：

### Level 0：静态检查

目标：抓明显错误，不依赖 ocgcore。

可检查：

- 文件名和 code 是否一致。
- 是否存在 `initial_effect`。
- `c{id}` 风格和 `GetID()` 风格是否混用错误。
- 常见 API 拼写错误。
- 未定义本地函数引用。
- `SetTarget` / `SetOperation` 指向的函数是否存在。
- target 函数是否包含 `chk==0` 分支。
- operation 中是否处理 `IsRelateToEffect`。
- filter 函数参数数量是否明显不一致。
- 是否使用禁止的 Lua API。

### Level 1：ocgcore load/init 检查

目标：确认脚本能被 ocgcore 加载，并能执行 `initial_effect`。

能发现：

- Lua 语法错误。
- 常量/API 名字错误。
- `aux` 使用错误。
- `GetID()` 环境错误。
- effect 注册阶段立即报错。
- `Set*` 参数类型明显错误。

不能证明：

- 效果语义是否等价于文本。
- 是否能在正确时点发动。
- 是否正确处理所有边界状态。

### Level 2：通用 smoke 场景

目标：把卡放入常见区域并推进 duel，捕获注册后立即触发的问题。

示例：

- 魔法/陷阱放手牌或 SZONE。
- 怪兽放手牌、场上、墓地。
- 额外怪兽放 EXTRA。
- 跑最小 duel tick。
- 查询状态，确认无崩溃和无错误消息。

Level 2 仍然只能证明“没有明显崩”，不能证明语义正确。

### Level 3：场景测试 / 谜题测试

目标：为某类效果生成具体 duel 场景并断言结果。

示例：

- 检索效果：卡组放一个合法目标，发动后断言目标进手牌。
- 特召效果：墓地/手牌放合法目标，断言召唤成功。
- 破坏效果：对方场上放可破坏目标，断言进入墓地。
- 无效效果：构造 chain，断言对应 chain 被无效。

Level 3 才接近“语义正确性验证”，但工程量最大，应作为中长期目标。

## Validator 集成建议

不建议第一版把 ocgcore 直接嵌入 Tauri 主进程。更稳的方式是做一个独立 validator helper：

```text
Tauri 后端
  -> 创建临时验证目录 / 输入 JSON
  -> 启动 script-validator helper
  -> helper 链接或加载 ocgcore
  -> 输出 JSON diagnostics
  -> 后端转换为 ScriptValidationResult
```

优点：

- ocgcore 崩溃不会带崩主应用。
- helper 的 build、ABI、动态库加载和日志隔离更清晰。
- 可以用 job 模式展示长任务或重复 repair。
- 后续可以替换不同 ocgcore 版本。

缺点：

- 需要额外可执行文件发布。
- 需要设计临时输入目录和 helper 协议。
- Windows 下构建链路需要单独维护。

建议 diagnostics 输出：

```ts
type ScriptValidationResult = {
  status: "ok" | "warning" | "error";
  staticCheck: {
    ok: boolean;
    issues: ScriptIssue[];
  };
  ocgcoreInit?: {
    ok: boolean;
    messages: string[];
    log?: string;
  };
  smoke?: {
    ok: boolean;
    scenario: string;
    messages: string[];
  };
};

type ScriptIssue = {
  severity: "info" | "warning" | "error";
  code: string;
  message: string;
  line?: number;
  column?: number;
};
```

## Agent Workflow 设计

### Generate Script Draft

用于从卡片描述生成新脚本。

流程：

1. 读取当前 card 全量信息。
2. 如果 card 没有文本或效果为空，要求用户补充。
3. 将描述拆成效果条目，识别 cost、condition、target、operation、once per turn、location、timing、category。
4. 根据 card metadata 和 setcodes 检索同系列案例。
5. 根据每个效果条目检索 effect-block 案例。
6. 选择模板或混合案例。
7. 生成 Lua 草稿。
8. 跑 Level 0 静态检查。
9. 跑 Level 1 ocgcore 初始化检查。
10. 若失败，把 diagnostics 和局部脚本反馈给模型修复，最多重试 2-3 轮。
11. 给用户展示摘要、参考案例、验证结果和草稿。
12. 用户确认后通过后端 resource API 写入脚本。

### Repair Script

用于修复已有脚本。

流程：

1. 读取现有脚本和 card。
2. 跑静态检查和 ocgcore 验证。
3. 如果用户提供错误日志，将日志并入诊断。
4. 检索相似案例和模板。
5. 让模型做最小修改。
6. 再验证。
7. 用户确认后覆盖写入。

### Review Script

后续可以增加纯审查工作流，不写入，只输出风险：

- 是否 target/非 target 与文本一致。
- 是否 cost/effect 处理一致。
- 是否 count limit 编号可疑。
- 是否缺少 `EFFECT_FLAG_DELAY`。
- 是否忘记 `Duel.ConfirmCards`。
- 是否没有检查 `Duel.GetLocationCount`。
- 是否忘记 `IsRelateToEffect`。

## Tool 建议

建议工具保持小而可组合。

只读工具：

- `get_card`：复用现有 Agent 工具。
- `get_card_script`：读取当前 custom card 脚本。
- `search_script_cases`：按结构化 query 搜标准/自定义脚本案例。
- `get_script_case`：读取具体案例脚本片段。
- `list_script_templates`：按 tags 或 intent 获取模板。
- `analyze_lua_script`：返回 effect block、调用、常量、函数引用。
- `validate_lua_script`：运行静态检查和 ocgcore 验证。

写工具：

- `write_card_script`：写入/覆盖当前 card 脚本，走后端资源边界和确认。
- `create_card_script`：如果当前没有脚本，可以创建草稿脚本。

注意：`generate_lua_script` 本身不建议是 tool，它应该是 workflow。

## 与现有 YGOCMG 架构的落点

推荐代码落点：

```text
src/features/agent/workflows/scriptGeneration/
  generateLuaScriptWorkflow.ts
  repairLuaScriptWorkflow.ts
  promptBuilders.ts
  types.ts

src/features/agent/tools/
  scriptTools.ts

src/shared/api/
  scriptApi.ts 或扩展 resourceApi.ts

src/shared/contracts/
  script.ts

src-tauri/src/application/script/
  service.rs
  analyzer.rs
  validator.rs
  templates.rs
  repository.rs

src-tauri/src/infrastructure/script_index/
  standard_script_index.rs
  lua_analyzer.rs

src-tauri/src/infrastructure/ocgcore_validator/
  helper_client.rs
```

如果验证耗时较长，后端应使用现有 job 模式。

文档维护建议：

- 稳定落地后更新 `../agent.md`：Agent 新增脚本生成/验证工具和 workflow。
- 如果新增 Tauri commands、contracts、API wrapper，更新 `../code_structure_api.md`。
- 如果新增用户可见脚本生成 UI 或确认体验，更新 `../ui_design.md`。
- 如果脚本资源行为变化，更新 `../functional_spec.md`。
- ocgcore helper 和脚本索引边界稳定后，更新 `../system_architecture.md`。

## 标准包索引扩展建议

当前标准包索引只暴露 `has_script`。为了支持脚本生成，应考虑新增脚本相关索引表。

可能 schema：

```sql
create table standard_card_scripts (
  code integer primary key,
  script_text text not null,
  script_modified integer,
  script_len integer not null,
  analysis_json text not null
);

create table standard_script_effect_blocks (
  code integer not null,
  block_index integer not null,
  signature_json text not null,
  categories_json text not null,
  effect_types_json text not null,
  events_json text not null,
  duel_calls_json text not null,
  card_calls_json text not null,
  aux_calls_json text not null,
  constants_json text not null,
  snippet text not null,
  primary key (code, block_index)
);

create index idx_standard_script_effect_block_code
  on standard_script_effect_blocks(code);
```

也可以先不存完整脚本文本，只存 `analysis_json` 和 snippet；但完整文本对后续调试、引用和修复更方便。由于标准脚本只读，索引体积可接受。

## Prompt / 生成策略

生成时建议模型不要直接从卡片全文写脚本，而是先输出结构化计划：

```json
{
  "effects": [
    {
      "label": "search on summon",
      "timing": "EVENT_SUMMON_SUCCESS",
      "type": "trigger_o",
      "location": "MZONE",
      "category": ["SEARCH", "TOHAND"],
      "cost": null,
      "condition": "this card normal/special summoned",
      "target": "1 same-set monster except self in deck",
      "operation": "add selected card to hand and reveal",
      "oncePerTurn": "same-name"
    }
  ]
}
```

然后 workflow 用结构化计划检索模板和案例。生成 Lua 时要求：

- 优先复用模板中的骨架。
- 函数命名保持局部一致。
- 不发明不存在的 ocgcore API。
- 参考案例必须注明 code。
- 对不确定语义输出 warning，不静默猜测。
- 验证失败时做最小修复，不重写整张脚本。

## 错误处理和用户体验

用户不应该只看到“生成失败”。建议展示分阶段结果：

- `已读取卡片信息`
- `已找到 5 个同系列参考`
- `已选择 3 个效果模板`
- `已生成草稿`
- `静态检查通过 / 有 2 个 warning`
- `ocgcore 初始化通过 / 失败`
- `已自动修复 1 次`
- `等待用户确认写入`

写入前展示：

- 生成脚本摘要。
- 参考案例列表。
- 验证结果。
- 是否覆盖已有脚本。
- 主要 warning。

如果已有脚本，默认应生成 patch 或草稿，不直接覆盖。覆盖必须明确确认。

## MVP 建议

第一阶段目标应克制：

1. 建立标准脚本 effect-block analyzer。
2. 建立最小模板库。
3. 增加脚本案例检索。
4. 增加 Level 0 静态检查。
5. 增加 Level 1 ocgcore 初始化验证。
6. 做 `Generate Script Draft` workflow。
7. 生成后只写入 custom pack 脚本资源，不修改卡片文本或其他数据。

第一阶段不建议承诺：

- 完整语义正确性。
- 所有复杂效果自动通过。
- 自动构造 duel 场景断言。
- 自动学习任意 package helper。
- 自动处理所有同名限制边界。

第二阶段：

- 本地 custom pack 脚本索引。
- repair workflow。
- 同系列 helper 检测。
- 常见 warning 规则。
- 更好的 prompt repair loop。

第三阶段：

- Level 2 smoke scenarios。
- Level 3 场景测试。
- script review workflow。
- 更完整模板库和模板编辑 UI。

## 风险

### 语义风险

ocgcore init 通过不等于效果正确。时点、target、cost、chain、damage step、once per turn、召唤条件等仍然容易错。

缓解：

- 模板优先。
- 案例优先。
- 结构化计划先行。
- 输出 warning。
- 不静默写入。

### 运行风险

ocgcore 直接嵌入主进程可能导致崩溃影响应用。

缓解：

- 使用独立 validator helper。
- 后端通过 job 调用。
- 超时和进程隔离。

### 数据风险

标准脚本索引可能变大，脚本文本和 snippets 需要注意 rebuild 性能。

缓解：

- 首版只索引 effect metadata 和必要 snippets。
- 后续再存完整文本。
- 使用标准包索引 rebuild job。

### Agent 风险

模型可能编造 API 或过度自信。

缓解：

- prompt 中提供可用 API 摘要。
- 验证器强制检查。
- 失败 diagnostics 进入 repair loop。
- 用户确认后才写入。

## 推荐决策

建议将 Lua script generation 设计为：

```text
Agent workflow: generate_lua_script
  uses tools:
    get_card
    search_script_cases
    get_script_templates
    analyze_lua_script
    validate_lua_script
    write_card_script
```

不要把 `generate_lua_script` 做成一个胖 tool。不要第一版引入 subagent。Subagent 可以作为后续扩展，例如复杂脚本 review 或系列风格分析，但 MVP 中 workflow + tools 更透明、可测、可维护。

最终用户体验应是“生成草稿 -> 验证 -> 修复 -> 确认写入”，而不是“一键生成并覆盖”。
