# Lua Script Validation Platform Report

本文档记录 2026-06-15 对“先建立一个可用的 YGOPro Lua 脚本验证平台”的设计结论和 ocgcore 可行性 spike。它是未来设计和实现参考，不描述当前已实现功能；当前事实仍以 `../functional_spec.md`、`../system_architecture.md`、`../code_structure_api.md`、`../ui_design.md` 和 `../agent.md` 为准。

## 背景

YGOCMG 当前已经支持 custom pack、卡片、资源和 AI Agent 管理，但脚本资源 API 仍主要围绕创建、导入、删除和外部打开脚本文件，尚未提供一套可复用的 Lua 脚本验证平台。

在 Lua 脚本生成能力真正进入实现前，应先建设一个独立、确定、可复用的验证平台。原因是：脚本生成不是只让模型写一段 Lua，而是需要把生成结果放入真实 YGOPro / ocgcore 环境中验证。没有验证平台，后续的生成、修复、Agent tool 和 UI 工作台都会缺少可靠反馈闭环。

本次设计将目标从“专门做一个 Lua 验证 subagent”收束为“先做一个后端验证平台 + Agent/UI 入口”。如果第一阶段只做验证和反馈，不自动修复、不自动规划复杂场景，那么 subagent 暂时不是必要结构。验证能力应先是 deterministic service/tool；subagent 只应在未来需要自动规划多个残局、根据消息动态操作、汇总覆盖率和推进多轮策略时再引入。

## 总体结论

建议第一阶段建设 **Lua Script Validation Platform MVP**，核心能力为：

```text
validate_lua_script
  - 静态检查
  - ocgcore card load/init 检查
  - 统一结构化 diagnostics
  - 明确 limitations
  - Agent tool 与 UI 共用同一个后端能力
```

推荐架构：

```text
用户入口
  - Card UI: 验证当前卡片脚本
  - Agent tool: validate_lua_script
  - Future: 批量验证 / 生成 workflow / scenario runner

Frontend API Layer
  src/shared/contracts/script.ts
  src/shared/api/scriptApi.ts

Tauri Command
  validate_lua_script

Backend Application Layer
  ScriptValidationService
    - 加载 card/script 上下文
    - 运行 StaticChecker
    - 调用 ocgcore validator helper
    - 合并报告

Validation Engines
  StaticChecker
  OcgcoreInitValidator
  Future: SmokeValidator
  Future: ScenarioRunner

Infrastructure
  ocgcore-validator-helper sidecar
  temp validation workspace
  script reader manifest
  card reader data manifest
  core script resolver
```

关键设计判断：

- 验证平台本身不应绑定 Agent。Agent 只是入口之一。
- 第一版不需要 LuaValidationAgent / subagent。
- 第一版应真实调用 ocgcore 做 card load/init 验证。
- 卡片脚本验证不能只用裸 `preload_script("./script/c{id}.lua")`。
- `preload_script` 适合 helper / package / puzzle / scenario setup script。
- 卡片脚本应通过 `new_card(...)` 触发 ocgcore 内部 `load_card_script(code)`，这样 `GetID()` 环境才正确。
- ocgcore 应隔离在 helper/sidecar 进程中，不建议直接嵌入 Tauri 主进程。
- scenario/残局测试是可行方向，但不是 MVP 阻塞项。

## ocgcore 可行性 Spike

### Spike 目标

在写正式验证平台报告前，先实际确认以下问题：

- 本仓库引用的 `ref/ygopro_src/ocgcore` 是否能在当前 Windows 开发环境编译。
- ocgcore 是否可以由外部程序通过 C API 调用。
- `set_script_reader`、`set_card_reader`、`set_message_handler` 是否能支撑验证器。
- `new_card` 是否能触发现代 `local s,id,o=GetID()` 卡片脚本的 `initial_effect`。
- 裸 `preload_script` 直接加载 `GetID()` 风格卡片脚本是否可靠。
- ocgcore 的错误信息是否能被捕获成 diagnostics。

### Spike 环境

本次 spike 未修改项目代码。临时构建目录为：

```text
%TEMP%\ygocmg-ocgcore-spike
```

使用的本机工具：

- Visual Studio Community 2022，MSBuild 17.6。
- CMake 存在，但本次实际采用 ocgcore 官方 CI 的 Premake + MSBuild 路线。
- 系统 PATH 中没有可直接使用的 Lua、vcpkg、gcc。
- ocgcore 本身不包含 Lua 源码，需要额外提供 Lua 依赖。

参考 ocgcore 官方 CI 的路线：

```text
1. 复制 ref/ygopro_src/ocgcore 到临时目录。
2. 下载 lua-5.4.7.tar.gz。
3. 下载 premake-5.0.0-beta2-windows.zip。
4. 将 ocgcore/premake/lua.lua 复制为 lua/premake5.lua。
5. 将 ocgcore/premake/dll.lua 复制为 dll.lua。
6. 运行 premake5.exe vs2022 --file=dll.lua。
7. 使用 MSBuild 编译 Release|x64。
```

编译结果：

```text
%TEMP%\ygocmg-ocgcore-spike\build\bin\x64\Release\ocgcore.dll
```

结论：Windows x64 下构建 `ocgcore.dll` 可行。产品实现不能假设用户系统里有 Lua；应把 Lua 作为 helper 的固定构建依赖，或者将 Lua 源码以可复现方式纳入 sidecar 构建流程。

### 最小 card init 调用验证

Spike 使用临时 PowerShell/C# PInvoke 调用 `ocgcore.dll`，注册以下 C API callback：

```text
set_script_reader(...)
set_card_reader(...)
set_message_handler(...)
```

最小验证流程：

```text
create_duel(seed)
set_player_info(pduel, 0, 8000, 0, 1)
set_player_info(pduel, 1, 8000, 0, 1)
new_card(pduel, 99999999, 0, 0, LOCATION_MZONE, 0, POS_FACEUP_ATTACK)
query_field_count(pduel, 0, LOCATION_MZONE)
end_duel(pduel)
```

测试脚本：

```lua
local s,id,o=GetID()

function s.initial_effect(c)
    local e1=Effect.CreateEffect(c)
    e1:SetType(EFFECT_TYPE_SINGLE)
    e1:SetCode(EFFECT_CANNOT_ATTACK)
    c:RegisterEffect(e1)
end
```

测试结果：

```text
OkFieldCount: 1
OkMessages: 空
```

结论：通过 `new_card` 触发卡片脚本加载和 `initial_effect` 执行是可行的，且能正确支持 `GetID()` 风格脚本。

### 错误捕获验证

将脚本故意改坏，去掉末尾 `end`：

```lua
local s,id,o=GetID()

function s.initial_effect(c)
    local e1=Effect.CreateEffect(c)
    e1:SetType(EFFECT_TYPE_SINGLE)
    e1:SetCode(EFFECT_CANNOT_ATTACK)
    c:RegisterEffect(e1)
-- missing end on purpose
```

ocgcore 捕获到：

```text
[string "./script/c99999999.lua"]:7: 'end' expected (to close 'function' at line 2) near <eof>
"CallCardFunction"(c99999999.initial_effect): attempt to call an error function
```

结论：`set_message_handler` + `get_log_message` 可以捕获 Lua 语法错误和 `initial_effect` 调用错误，足以转换为验证平台 diagnostics。

### preload_script 对照验证

直接使用：

```text
preload_script("./script/c99999999.lua")
```

加载以下 `GetID()` 风格卡片脚本：

```lua
local s,id,o=GetID()

function s.initial_effect(c)
end
```

结果：

```text
PreloadGetIdResult: 0
PreloadGetIdMessages:
[string "./script/utility.lua"]:9: attempt to compare nil with number
```

原因是 `GetID()` 在 `utility.lua` 中依赖 `self_code`：

```lua
function GetID()
    local offset=self_code<100000000 and 1 or 100
    return self_table,self_code,offset
end
```

而 `self_code` / `self_table` 是 ocgcore 在 `interpreter::load_card_script(code)` 中设置的。裸 `preload_script` 不会自动设置这个卡片上下文。

对照测试普通 helper：

```lua
function hello_helper()
  return 42
end
```

使用 `preload_script("./script/helper.lua")` 成功：

```text
PreloadPlainResult: 1
PreloadPlainMessages: 空
```

结论：

- 卡片脚本验证应通过 `new_card` 触发 `load_card_script + initial_effect`。
- `preload_script` 可以用于 helper、package、puzzle、scenario setup。
- 将“卡片脚本 ocgcore 验证”称为 preload 检查是不准确的；应称为 ocgcore load/init 检查。

## ocgcore 的实际使用方式

### C API 入口

验证平台需要用到的核心 API 来自 `ref/ygopro_src/ocgcore/ocgapi.h`：

```text
set_script_reader(script_reader f)
set_card_reader(card_reader f)
set_message_handler(message_handler f)

create_duel(seed)
set_player_info(pduel, playerid, lp, startcount, drawcount)
new_card(pduel, code, owner, playerid, location, sequence, position)
start_duel(pduel, options)
process(pduel)
get_message(pduel, buf)
get_log_message(pduel, buf)
query_card(...)
query_field_card(...)
query_field_count(...)
set_responsei(...)
set_responseb(...)
preload_script(pduel, script_name)
end_duel(pduel)
```

### script_reader

`script_reader` 的职责是根据 ocgcore 请求的脚本名返回脚本内容。

验证平台至少需要支持：

```text
./script/constant.lua
./script/utility.lua
./script/procedure.lua
./script/c{id}.lua
```

未来还需要支持：

```text
./script/package/helper.lua
./script/{series_helper}.lua
./single/{scenario}.lua
```

实现建议：

- 后端 `ScriptValidationService` 创建临时验证目录或输入 manifest。
- helper 进程读取 manifest，维护 `script_name -> content/path` 映射。
- `script_reader` 只负责按 ocgcore 的请求名返回 byte buffer。
- helper 负责 buffer 生命周期，避免返回已释放内存。
- 读取不到脚本时返回 null，并把缺失脚本记录为 diagnostics。

注意：`interpreter` 构造时会自动尝试加载：

```text
./script/constant.lua
./script/utility.lua
./script/procedure.lua
```

所以验证器必须提供这些核心脚本，否则 duel 初始化阶段就不完整。

### card_reader

`card_reader` 的职责是根据 code 填充 `card_data`。

`card_data` 字段包括：

```text
code
alias
setcode[16]
type
level
attribute
race
attack
defense
lscale
rscale
link_marker
```

MVP 需要能为待测卡返回准确数据。对于 scenario 测试，未来还需要为辅助卡、目标卡、token 或同系列卡返回数据。

实现建议：

- 后端从 YGOCMG 当前 card model / pack CDB 写入逻辑中复用 raw datas 映射规则。
- 不让 frontend 拼 `card_data`。
- 标准卡辅助数据应从标准包只读索引或 CDB 读取。
- custom pack 卡片优先使用当前 pack 数据。
- 如果 scenario 中出现未知 code，返回明确 issue，而不是让 ocgcore 静默使用空数据。

### message_handler

`message_handler` 用于捕获 ocgcore / Lua 错误。Spike 已验证语法错误会进入 message handler，并可通过 `get_log_message` 取到文本。

MVP 建议转换为：

```ts
type LuaValidationIssue = {
  severity: "error" | "warning" | "info";
  stage: "ocgcore_init";
  code: "lua_runtime_error" | "lua_syntax_error" | "ocgcore_message";
  message: string;
  line?: number;
  column?: number;
  suggestion?: string;
};
```

Lua 报错文本可解析出常见格式：

```text
[string "./script/c99999999.lua"]:7: 'end' expected ...
```

MVP 可先解析 `script path + line + message`，column 可以留空。

### 卡片脚本 load/init 路径

卡片脚本验证应使用：

```text
create_duel
set_player_info
new_card
```

`new_card` 内部会：

```text
duel::new_card(code)
  -> read_card(code, &pcard->data)
  -> pcard->data.code = code
  -> interpreter::register_card(pcard)
       -> load_card_script(pcard->data.get_original_code())
            -> set self_table
            -> set self_code
            -> load ./script/c{id}.lua
       -> call_card_function(pcard, "initial_effect", 1, 0)
```

因此 `new_card` 才是验证现代卡片脚本初始化的正确入口。

### puzzle / scenario 路径

复杂残局验证应使用另一条路径：

```text
create_duel
set_script_reader / set_card_reader / set_message_handler
preload_script("./single/puzzle.lua")
start_duel
process loop
get_message
set_responseb / set_responsei
query_card / query_field_card
```

YGOPro `single_mode.cpp` 也是通过 `preload_script` 加载 single/puzzle 脚本，再进入消息循环。

残局 setup 可使用 ocgcore 暴露的 Debug Lua API：

```lua
Debug.ReloadFieldBegin(DUEL_ATTACK_FIRST_TURN + DUEL_SIMPLE_AI + DUEL_PSEUDO_SHUFFLE, 5)
Debug.SetPlayerInfo(0, 8000, 0, 1)
Debug.SetPlayerInfo(1, 8000, 0, 1)

Debug.AddCard(99999999, 0, 0, LOCATION_HAND, 0, POS_FACEUP)
Debug.AddCard(100000002, 0, 0, LOCATION_DECK, 0, POS_FACEDOWN)

Debug.ReloadFieldEnd()
aux.BeginPuzzle()
```

这证明复杂残局方向可行。但自动 scenario runner 还需要消息协议解析、自动响应和断言系统，应作为后续阶段。

## 验证平台分层

### Level 0: Static Check

目标：不启动 ocgcore，快速发现明显问题。

MVP 可检查：

- 文件名/card code 与脚本 code 是否一致。
- 是否存在 `initial_effect(c)`。
- 是否混用 `local s,id,o=GetID()` 与旧式 `c{id}.xxx` 风格。
- `SetCondition` / `SetCost` / `SetTarget` / `SetOperation` 引用的函数是否存在。
- `s.xxx` 或 `c{id}.xxx` 引用的局部函数是否存在。
- target 函数是否包含 `if chk==0 then ... end` 分支。
- operation 中对目标卡是否缺少常见 `IsRelateToEffect(e)` 检查。
- 常见 API 拼写错误。
- 常见常量拼写错误。
- 是否使用高风险 Lua API，如 `io.*`、`os.execute`、`require`。

这一层不应过度承诺语义判断。它负责“快、稳定、可解释”，不是替代 ocgcore。

### Level 1: Ocgcore Load/Init

目标：确认脚本能被真实 ocgcore 加载，并能执行 `initial_effect`。

能发现：

- Lua 语法错误。
- `Duel` / `Card` / `Effect` / `Group` / `aux` API 名称错误。
- 常量未定义。
- `GetID()` 环境问题。
- helper 缺失。
- `initial_effect` 注册阶段 runtime error。
- `SetType` / `SetCode` / `RegisterEffect` 等基础调用错误。

不能证明：

- 效果文本语义正确。
- 发动时点正确。
- cost/target/operation 与文本一致。
- 连锁、伤害步骤、时点错过、once per turn 编号完全正确。
- 复杂召唤手续正确。

报告里必须保留 limitations，避免用户把 load/init pass 理解为脚本语义 pass。

### Level 2: Smoke

目标：把卡放入若干常见区域，启动最小 duel tick，检查是否立即报错。

第一版可先预留，不一定实现。未来可按类型做保守 smoke：

- 怪兽：手牌、场上、墓地。
- 魔法：手牌、SZONE。
- 陷阱：手牌、SZONE。
- 额外卡：EXTRA、场上。

Smoke 仍然只是“不明显崩”，不是语义验证。

### Level 3: Scenario / Puzzle

目标：构造具体残局，自动操作消息循环，并断言局面变化。

例子：

- 检索效果：卡组放合法目标，发动后断言目标进手牌。
- 特召效果：墓地/手牌放合法目标，断言目标到 MZONE。
- 破坏效果：对方场上放目标，断言目标进墓地。
- 除外效果：断言目标进入 removed。
- 无效效果：构造 chain，断言对应 chain 被无效。

这一层需要：

- Scenario DSL。
- 消息 parser。
- response planner。
- field query decoder。
- assertion engine。

这不是 MVP 阻塞项。

## MVP 范围

### MVP 必须包含

- 新增后端 `ScriptValidationService`。
- 新增 Tauri command `validate_lua_script`。
- 新增 frontend contract 和 API wrapper。
- 支持验证当前 custom pack 中某张卡的保存脚本。
- 支持传入 `scriptText` 验证未保存草稿。
- StaticChecker。
- Ocgcore init helper client。
- 独立 ocgcore validator helper/sidecar。
- 统一 `LuaValidationReport`。
- Agent tool `validate_lua_script`。
- 最小 UI 入口，至少可在卡片编辑上下文中触发验证并查看报告。

### MVP 不包含

- 自动修复脚本。
- 自动生成 Lua 脚本。
- 自动根据文本创建残局。
- 多场景覆盖率判断。
- 批量验证全部卡片。
- 标准脚本 effect-block 索引。
- 完整脚本编辑器。
- LuaValidationAgent / subagent。

### MVP 验收标准

至少满足：

- 对一个正确的 `GetID()` 风格脚本，`validate_lua_script` 返回 `pass` 或 `warning`，且 `ocgcore_init.ok == true`。
- 对一个缺少 `end` 的脚本，返回 `fail`，issue 中包含 Lua line number 和错误文本。
- 对一个引用不存在函数的脚本，静态检查能返回明确 error。
- 对一个直接依赖缺失 helper 的脚本，ocgcore init 返回 helper/script missing 或 runtime error diagnostics。
- 对 normal monster 或无脚本卡，报告能明确说明“不需要/没有脚本”或“未找到脚本”，而不是崩溃。
- helper 超时或崩溃时，主应用不崩溃，报告返回 `inconclusive`。
- Agent tool 与 UI 调用同一个 `scriptApi.validateLuaScript`。

## 数据契约建议

### ValidateLuaScriptInput

```ts
export type LuaValidationLevel =
  | "static"
  | "ocgcore_init"
  | "smoke"
  | "scenario";

export type ValidateLuaScriptInput = {
  packId: string;
  cardId: string;
  scriptText?: string;
  levels: LuaValidationLevel[];
  scenario?: LuaScenario;
};
```

规则：

- `packId` 和 `cardId` 必填。
- `scriptText` 为空时，后端读取当前保存脚本。
- `scriptText` 非空时，验证草稿，不写入文件。
- MVP 默认 levels 为 `["static", "ocgcore_init"]`。
- `scenario` MVP 可不支持，字段可预留。

### LuaValidationReport

```ts
export type LuaValidationStatus =
  | "pass"
  | "warning"
  | "fail"
  | "inconclusive";

export type LuaValidationReport = {
  status: LuaValidationStatus;
  confidence: "low" | "medium" | "high";
  summary: string;
  issues: LuaValidationIssue[];
  stages: LuaValidationStageResult[];
  limitations: string[];
};
```

状态含义：

- `pass`：请求的验证阶段全部通过，且没有 warning/error。
- `warning`：没有 fatal error，但存在风险或未覆盖语义。
- `fail`：发现确定错误。
- `inconclusive`：由于 helper 不可用、超时、依赖缺失或场景不足，无法给出有效结论。

### LuaValidationIssue

```ts
export type LuaValidationIssue = {
  severity: "error" | "warning" | "info";
  stage: "static" | "ocgcore_init" | "smoke" | "scenario";
  code: string;
  message: string;
  line?: number;
  column?: number;
  suggestion?: string;
};
```

### Stage Result

```ts
export type LuaValidationStageResult = {
  stage: LuaValidationLevel;
  status: LuaValidationStatus;
  durationMs: number;
  issues: LuaValidationIssue[];
  log?: string[];
};
```

## 后端架构建议

## 第三方依赖管理

MVP 阶段将 ocgcore 和 YGOPro 脚本库作为 YGOCMG 根仓库的第三方 submodule 管理：

```text
third_party/ocgcore
  https://github.com/Fluorohydride/ygopro-core.git

third_party/ygopro-scripts
  https://github.com/Fluorohydride/ygopro-scripts.git
```

这样做的目的：

- 父仓库固定 ocgcore 和脚本库的具体 commit，验证器构建可复现。
- 第三方源码与 YGOCMG 业务代码保持边界。
- 未来可单独升级 ocgcore / scripts，再由父仓库提交 submodule 指针。
- 当前不需要 fork，也不需要修改第三方源码。

如果未来需要定制 ocgcore，建议 fork `ygopro-core`，然后将 `third_party/ocgcore` 的 `origin` 或 submodule URL 切到 YGOCMG 自己的 fork，同时在 fork 中保留官方仓库作为 `upstream`：

```bash
git remote add upstream https://github.com/Fluorohydride/ygopro-core.git
git fetch upstream
git merge upstream/master
```

父仓库更新 submodule 版本时，需要进入 submodule 更新 commit，再回到父仓库提交指针：

```bash
cd third_party/ocgcore
git fetch origin
git checkout master
git pull

cd ../..
git add third_party/ocgcore
git commit -m "chore: update ocgcore submodule"
```

新环境 clone 时应使用：

```bash
git clone --recurse-submodules <repo>
```

或在已有 checkout 中运行：

```bash
git submodule update --init --recursive
```

CI 中 checkout 也应启用 recursive submodules。

Lua 依赖不作为 submodule 引入，也不展开源码进 git。`third_party/cache/` 是本地构建缓存目录，整体被 `.gitignore` 忽略，不提交 tarball。MVP helper 固定使用官方 Lua 5.4.7 tarball，并在 bootstrap/build 阶段下载到本地缓存：

```text
third_party/cache/lua-5.4.7.tar.gz
third_party/cache/lua-5.4.7.sha256
```

当前 SHA-256：

```text
9fbf5e28ef86c69858f6d3d34eccc32e911c1a28b4120ff3e84aaa70cfbf1e30
```

helper 构建脚本应：

1. 如果本地缓存中没有 `lua-5.4.7.tar.gz`，从 `https://www.lua.org/ftp/lua-5.4.7.tar.gz` 下载。
2. 校验 SHA-256 必须等于上面的固定值。
3. 在构建目录中解压该 tarball。
4. 不把解压后的 Lua 源码或 tarball 提交到 git。

这样可以避免 Lua 展开源码污染仓库，也不依赖系统 Lua。需要完全离线构建时，可由开发者或 CI 预先把 tarball 放入 `third_party/cache/`，构建脚本仍然必须校验 checksum。

## 后端架构建议

### Application 层

新增：

```text
src-tauri/src/application/script/
  mod.rs
  service.rs
  dto.rs
  static_checker.rs
  source_resolver.rs
  report.rs
```

职责：

- `service.rs`: 编排验证流程。
- `dto.rs`: Rust 侧 input/report DTO。
- `static_checker.rs`: 纯静态规则。
- `source_resolver.rs`: 读取 card/script/core/helper 上下文。
- `report.rs`: 合并 stage 结果，计算最终 status/confidence。

`ScriptValidationService::validate` 建议流程：

```text
1. 校验 pack/card 是否存在。
2. 加载 card context。
3. 读取 scriptText 或当前卡片脚本。
4. 如果没有脚本，返回明确 report。
5. 运行 StaticChecker。
6. 如果 static 有 fatal error，仍可按配置跳过 ocgcore_init。
7. 准备 ocgcore helper input。
8. 调 helper client，带 timeout。
9. 合并 helper 输出。
10. 返回 LuaValidationReport。
```

### Infrastructure 层

新增：

```text
src-tauri/src/infrastructure/ocgcore_validator/
  mod.rs
  helper_client.rs
  input.rs
  output.rs
```

职责：

- 创建临时验证 workspace。
- 写入 helper input JSON 或 script manifest。
- 启动 helper/sidecar。
- 设置超时。
- 读取 stdout JSON。
- 捕获 stderr。
- 清理临时目录。
- 把 helper 崩溃/超时转换为 `inconclusive`。

### Helper / Sidecar

推荐第一版使用独立 helper 进程，而不是把 ocgcore 直接链接进 Tauri 主进程。

原因：

- ocgcore / Lua / C++ crash 不应拖垮主应用。
- helper 输入输出 JSON，边界清楚。
- 后续可替换 ocgcore 版本。
- 可以单独测试、打包、超时和杀进程。

helper 可以选择：

```text
方案 A: C++ helper 静态/动态链接 ocgcore + Lua
方案 B: Rust helper 通过 bindgen/libloading 调 ocgcore.dll
```

MVP 推荐方案 A：

- 距离 ocgcore 原生 API 最近。
- 不需要 Rust FFI 先行投入。
- 可复用 ocgcore 官方 Premake 构建路线。
- helper 输出 JSON 给 Rust 后端即可。

产品化时应避免构建时隐式联网。Spike 下载 Lua 是为了验证可行性；正式实现应采用 pinned dependency：

- 将 Lua 5.4.7 源码纳入可复现 sidecar 构建流程；或
- 提供带 checksum 的 bootstrap 脚本；或
- 以明确 third-party dependency 方式管理。

## Frontend / Agent 集成建议

### API Wrapper

新增：

```text
src/shared/contracts/script.ts
src/shared/api/scriptApi.ts
```

UI 不直接 raw invoke。所有调用走：

```ts
scriptApi.validateLuaScript(input)
```

### Agent Tool

新增 Agent tool：

```text
validate_lua_script
```

工具职责：

- 包装 `scriptApi.validateLuaScript`。
- 默认验证当前选中 card。
- 没有选中 card 时要求用户明确 card。
- 不修改脚本。
- 不自动修复。
- 将 `LuaValidationReport` 返回给主 Agent。

主 Agent 只负责解释结果，例如：

```text
静态检查发现 s.thop 被引用但未定义。
因此我没有继续运行 ocgcore 初始化验证。
建议先补齐 s.thop 后重新验证。
```

### UI 入口

MVP UI 可以非常克制：

- 在卡片编辑抽屉或资源栏中提供“验证脚本”入口。
- 展示总状态、阶段状态、issue 列表、limitations。
- 不需要内置完整脚本编辑器。
- 如果未来支持草稿验证，再将脚本编辑面板接入 `scriptText`。

## MVP 实现计划

### 阶段 1: 契约和后端骨架

目标：建立不含 ocgcore 的验证 command 骨架，先能返回静态检查报告。

任务：

1. 新增 `src/shared/contracts/script.ts`。
   - 定义 `LuaValidationLevel`、`ValidateLuaScriptInput`、`LuaValidationReport`、`LuaValidationIssue`、`LuaValidationStageResult`。

2. 新增 `src/shared/api/scriptApi.ts`。
   - 包装 Tauri command `validate_lua_script`。

3. 新增 Rust DTO。
   - `src-tauri/src/application/script/dto.rs`。
   - 与前端 contract 保持 snake/camel 序列化一致。

4. 新增 `ScriptValidationService` 骨架。
   - `validate(input) -> LuaValidationReport`。
   - 初始只做 script/card 解析和空 report。

5. 注册 Tauri command。
   - presentation command 调 application service。
   - `src-tauri/src/tauri_commands.rs` 加 handler。

6. 测试。
   - `cargo check`。
   - `npm run typecheck`。

### 阶段 2: ScriptSourceResolver

目标：后端能获取待验证脚本文本和卡片上下文。

任务：

1. 新增 `source_resolver.rs`。
   - 支持 `scriptText` override。
   - `scriptText` 为空时，从当前 pack 脚本资源读取 `c{id}.lua`。

2. 读取 card context。
   - 复用现有 card service / repository。
   - 获取 card code、type、attribute、race、atk/def、level、setcodes 等。

3. 处理没有脚本的情况。
   - 返回 `inconclusive` 或 `warning`，issue code 如 `script_not_found`。
   - 不把“没有脚本”伪装成 ocgcore 错误。

4. 测试。
   - 有 override script 时优先使用 override。
   - 无 override 时读取保存脚本。
   - 无脚本时返回结构化 issue。

### 阶段 3: StaticChecker

目标：实现第一批确定性静态检查。

任务：

1. 新增 `static_checker.rs`。
2. 实现基础扫描：
   - `initial_effect` 存在性。
   - `GetID()` 风格识别。
   - `s.xxx` / `c{id}.xxx` 定义和引用收集。
   - `SetCondition/SetCost/SetTarget/SetOperation` 引用检查。
3. 实现常见规则：
   - target 函数是否有 `chk==0`。
   - 危险 API 检查。
   - 常见 API/常量拼写可先维护小表。
4. 返回 line number。
5. 单元测试：
   - 正确脚本无 error。
   - 缺 `initial_effect` 返回 error。
   - 引用未定义函数返回 error。
   - 缺 `chk==0` 返回 warning。

### 阶段 4: ocgcore helper 构建 spike 产品化

目标：把临时 spike 转为可维护的 sidecar/helper。

任务：

1. 确定 helper 落点。
   - 可放在 `src-tauri/sidecars/script-validator-helper/` 或独立 `tools/script-validator-helper/`。
   - MVP 以独立 helper executable 为目标。

2. 建立 helper 输入 JSON。
   - 包含 card data、script map、validation levels、timeout hint。

3. 建立 helper 输出 JSON。
   - 包含 init ok、messages、lua errors、duration。

4. 编写 C++ helper。
   - 注册 `script_reader`。
   - 注册 `card_reader`。
   - 注册 `message_handler`。
   - 调用 `create_duel`、`set_player_info`、`new_card`、`end_duel`。
   - stdout 输出 JSON。
   - stderr 输出调试日志。

5. 构建 ocgcore + Lua。
   - 复用官方 Premake 路线或 CMake 路线。
   - 明确 Lua 5.4.7 依赖。
   - 不依赖系统 Lua。

6. helper 测试。
   - valid `GetID()` script pass。
   - missing `end` script fail，包含 line number。
   - missing core script fail/inconclusive。
   - unknown card data fail/inconclusive。

### 阶段 5: Rust helper client

目标：Tauri 后端能安全调用 helper。

任务：

1. 新增 `helper_client.rs`。
2. 创建 temp validation workspace。
3. 写入 helper input JSON。
4. 启动 helper。
5. 设置 timeout。
6. 读取 stdout 并解析 JSON。
7. 捕获 stderr。
8. helper exit code 非 0 时返回 `inconclusive`。
9. 清理临时目录。
10. 测试：
    - helper 正常输出。
    - helper 超时。
    - helper 输出非法 JSON。
    - helper 不存在。

### 阶段 6: OcgcoreInitValidator 集成

目标：`validate_lua_script` 同时返回 static 和 ocgcore init 结果。

任务：

1. 在 `ScriptValidationService` 中调用 helper client。
2. 将 helper messages 转换为 `LuaValidationIssue`。
3. 解析 Lua line number。
4. 根据 stage result 计算最终 status。
5. 加入 limitations：
   - `ocgcore_init_pass_does_not_prove_semantics`。
6. 测试：
   - static pass + init pass -> report pass/warning。
   - static error -> report fail，可跳过 init。
   - init error -> report fail。
   - helper crash -> report inconclusive。

### 阶段 7: Agent tool

目标：让现有 Agent 可以调用验证平台。

任务：

1. 新增 `src/features/agent/tools/scriptTools.ts`。
2. 注册 `validate_lua_script` tool。
3. 工具输入支持 cardId / 默认 selected card。
4. 工具只读，不修改脚本。
5. system prompt 增加验证工具说明。
6. 测试：
   - 未选中卡时提示需要 card。
   - 选中卡时调用 API。
   - report 被原样返回给 Agent loop。

### 阶段 8: 最小 UI 展示

目标：用户不通过 Agent 也能验证当前脚本。

任务：

1. 在卡片资源栏或编辑抽屉中增加验证入口。
2. 点击后调用 `scriptApi.validateLuaScript`。
3. 展示：
   - 总状态。
   - static stage。
   - ocgcore init stage。
   - issue list。
   - limitations。
4. 不做复杂编辑器。
5. 测试：
   - loading 状态。
   - pass/warning/fail/inconclusive 展示。
   - 无脚本展示。

### 阶段 9: 文档更新

目标：稳定后更新当前权威文档。

需要更新：

- `docs/functional_spec.md`: 脚本验证用户能力。
- `docs/system_architecture.md`: ocgcore helper / sidecar 边界。
- `docs/code_structure_api.md`: 新 contracts、API wrapper、Tauri command。
- `docs/ui_design.md`: 验证入口和报告展示。
- `docs/agent.md`: 新 Agent tool。

注意：实现前本报告仍留在 `docs/history/`，不作为当前事实。

## 风险和缓解

### ocgcore 崩溃风险

风险：C++/Lua 层崩溃拖垮主进程。

缓解：使用独立 helper 进程；Rust 后端只通过 JSON/stdio 交互；设置 timeout 和进程终止。

### Lua 依赖风险

风险：开发机或用户机器没有 Lua。

缓解：helper 构建时固定 Lua 版本；不要依赖系统 Lua；构建流程记录 checksum 或 vendor source。

### ABI / 构建风险

风险：ocgcore、Lua、MSVC runtime、Tauri sidecar 打包之间出现 ABI 或发布问题。

缓解：helper 做成独立 executable；优先静态链接可控依赖；在 CI 中构建 sidecar；发布前做 clean machine smoke。

### card_data 映射风险

风险：YGOCMG card model 到 ocgcore `card_data` 映射不准确，导致验证假失败或假通过。

缓解：复用现有 CDB 写出逻辑；对 raw type/setcode/level/link marker 写单元测试；用标准卡样本对照。

### 静态检查误报风险

风险：脚本写法多样，简单 regex 可能误判。

缓解：第一批规则保持保守；error 只用于确定问题；可疑项用 warning；未来再引入更完整 Lua parser。

### init pass 误导风险

风险：用户误以为 ocgcore init pass 等于效果正确。

缓解：报告中固定加入 limitation；状态可以是 `warning` 而不是绝对 pass；UI 文案说明“未覆盖语义场景”。

### helper / package 脚本缺失风险

风险：自定义系列依赖 helper，验证环境未加载。

缓解：`ScriptSourceResolver` 检测 `require` 或常见 package 路径；MVP 无法解析时返回明确 issue；未来支持 package resolver。

### scenario 复杂度风险

风险：自动残局测试需要解析大量 ocgcore 消息协议。

缓解：MVP 不承诺自动 scenario；先预留 DSL；未来从显式 scenario 和少量模板开始。

## 后续演进

### Phase 2: Script Workbench

在 UI 中加入脚本查看/编辑/验证工作台：

- 读取脚本文本。
- 验证未保存草稿。
- issue 点击跳转行。
- 保存前验证。

### Phase 3: Scenario Runner

实现显式 scenario DSL：

```text
players
cards
steps
assertions
```

先支持手写或模板 scenario，不自动由 AI 生成。

### Phase 4: Agent-assisted Scenario Planning

当 scenario runner 稳定后，再考虑 LuaValidationWorkflow / subagent：

- 根据卡片文本识别效果意图。
- 选择 scenario 模板。
- 填充合法目标/非法目标。
- 跑多个 scenario。
- 汇总覆盖率和 limitations。

### Phase 5: Script Generation / Repair Workflow

验证平台稳定后，再接入脚本生成和修复：

```text
生成草稿
  -> static check
  -> ocgcore init
  -> optional scenario
  -> 用户确认写入
```

此时验证平台作为生成 workflow 的底座，而不是混在生成 prompt 内部。

## 推荐决策

推荐立即采用以下设计方向：

```text
第一阶段:
  Lua Script Validation Platform MVP
    - static check
    - ocgcore load/init check
    - helper sidecar
    - structured report
    - Agent/UI 双入口

暂不做:
  - LuaValidationAgent
  - 自动修复
  - 自动生成残局
  - 完整语义证明
```

最重要的工程约束：

```text
卡片脚本验证:
  new_card -> load_card_script -> initial_effect

helper / puzzle / scenario script:
  preload_script
```

最重要的产品约束：

```text
ocgcore init 通过只说明脚本能加载和注册。
它不证明脚本效果语义完全正确。
```

最重要的架构约束：

```text
验证能力属于后端平台能力。
Agent tool 和 UI 只是调用入口。
subagent 是未来策略层，不是 MVP 必需结构。
```
