# Lua Script Validation Phase 7/8 Handoff

本文是给下一个 agent 的交接说明，目标是继续推进 `docs/history/lua_script_validation_platform_report_2026-06-15.md` 中的阶段 7/8：Agent tool 与最小 UI 展示。

## 先读这些

当前事实以代码和核心文档为准，历史报告只作为阶段背景。

建议阅读顺序：

1. `docs/README.md`
2. `docs/functional_spec.md`
3. `docs/system_architecture.md`
4. `docs/code_structure_api.md`
5. `docs/ui_design.md`
6. `docs/agent.md`
7. `docs/history/lua_script_validation_platform_report_2026-06-15.md` 的阶段 7/8

如果实现改变了稳定事实，完成后同步更新：

- `docs/functional_spec.md`
- `docs/system_architecture.md`
- `docs/code_structure_api.md`
- `docs/ui_design.md`
- `docs/agent.md`

## 当前已完成状态

阶段 1-6 已完成并验证。当前后端 `validate_lua_script` 已可默认运行：

```text
static + ocgcore_init
```

后端能力：

- `validate_lua_script` Tauri command 已存在。
- 前端 API wrapper 已存在：`src/shared/api/scriptApi.ts`。
- 前后端 contract 已存在：`src/shared/contracts/script.ts` 与 `src-tauri/src/application/script/dto.rs`。
- 后端编排入口：`src-tauri/src/application/script/service.rs`。
- 静态检查：`src-tauri/src/application/script/static_checker.rs`。
- 脚本/card 上下文解析：`src-tauri/src/application/script/source_resolver.rs`。
- 报告聚合：`src-tauri/src/application/script/report.rs`。
- ocgcore init adapter：`src-tauri/src/application/script/ocgcore_init.rs`。
- Rust helper client：`src-tauri/src/infrastructure/ocgcore_validator/*`。
- helper 可执行体工程：`tools/script-validator-helper`。

重要行为：

- `levels` 为空或未传时默认 `[static, ocgcore_init]`。
- static 有 error 时，ocgcore 阶段会返回 `skipped_due_to_static_errors` 的 `inconclusive` stage。
- helper 缺失、超时、崩溃、stdout 非法等都返回 `ocgcore_init` 的 `inconclusive` stage，不让 Tauri command 失败。
- `smoke` / `scenario` 仍为 `level_not_implemented`。
- `ocgcore_init` 只证明脚本 load/init，可执行 `initial_effect`；不证明效果语义正确。

最近关键提交：

```text
b7e4156 chore: polish script validation messages
9f4eae5 docs: document ocgcore validation integration
61f281d feat: summarize ocgcore validation reports
5f2f9ec feat: run ocgcore init validation stage
b5ec795 feat: add ocgcore init validation adapter
9e534ec feat: add ocgcore helper process client
3ea1a98 feat: add ocgcore helper contracts
3465085 feat: expose ocgcore card data encoding
cacd513 docs: plan ocgcore helper client integration
```

## 工作流要求

继续使用仓库里的 skill 流程：

- 先用 `using-superpowers`。
- 做新功能前用 `brainstorming`。
- 多步骤实现先用 `writing-plans` 写计划到 `docs/superpowers/plans/YYYY-MM-DD-<name>.md`。
- 实现阶段用 `test-driven-development`，先写失败测试再写生产代码。
- 执行计划用 `executing-plans` 或等价逐任务执行。
- 完成前用 `verification-before-completion`。

用户已接受在当前分支工作；除非用户另有要求，不需要新 worktree。

## 下一步范围

下一步是阶段 7 和阶段 8。

阶段 7：Agent tool

目标：让现有 AI Agent 可以调用验证平台。

建议实现：

1. 新增 `src/features/agent/tools/scriptTools.ts`。
2. 新增只读工具 `validate_lua_script`。
3. 注册到 `src/features/agent/tools/registry.ts` 的 `AGENT_TOOLS`。
4. 更新 `src/features/agent/systemPrompt.ts`，说明何时使用验证工具。
5. 更新 `docs/agent.md`，记录新工具。
6. 加测试，建议新增或扩展 `src/features/agent/tools/*.test.ts`。

工具应该：

- 调用 `scriptApi.validateLuaScript`，不要直接 raw invoke。
- 使用 `ToolContext` 中的 `workspaceId` / `packId`。
- `cardId` 可选。传了就验证该卡；未传时建议使用当前 Selected card。
- 当前 `ToolContext` 只有 `workspaceId` / `packId`，没有 selected card。要支持“默认 selected card”，需要扩展 `ToolContext`，并在 `src/features/agent/useAgentLoop.ts` 传入 `shell.selectedCard`。
- 工具必须只读：`readOnly: true`。
- 不修改脚本，不自动修复。
- 默认不要传 `levels`，让后端使用 `[static, ocgcore_init]`。
- 可允许显式传 `levels`，但要白名单校验为 `static` / `ocgcore_init` / `smoke` / `scenario`。
- 返回 `LuaValidationReport` 的结构化结果给 Agent loop。

需要关注的现有代码：

- `src/features/agent/tools/types.ts`
- `src/features/agent/tools/registry.ts`
- `src/features/agent/tools/readTools.ts`
- `src/features/agent/systemPrompt.ts`
- `src/features/agent/useAgentLoop.ts`
- `src/features/agent/agentLoop.ts`
- `src/shared/api/scriptApi.ts`
- `src/shared/contracts/script.ts`

建议测试点：

- 无 active pack 时抛出 `ToolError`。
- 未传 `cardId` 且没有 selected card 时抛出清晰错误，要求用户指定卡片。
- 未传 `cardId` 但有 selected card 时，用 selected card id 调用 `scriptApi.validateLuaScript`。
- 传入 `cardId` 时优先用传入值。
- tool 返回 report 原样 JSON-serializable。
- registry 中包含 `validate_lua_script`，`TOOL_DEFINITIONS` 会发送给模型。

阶段 8：最小 UI 展示

目标：用户不通过 Agent 也能验证当前脚本。

建议落点：

- 首选 `src/features/card/CardAssetBar.tsx` 的脚本按钮组。
- 当前脚本资源 UI 已在 `CardAssetBar` 中处理 create/import/edit/delete script。
- 可以在 `assetState.has_script` 时新增一个“验证”按钮。
- 验证结果展示可以先做在 `CardAssetBar` 内部或拆成 `ScriptValidationReportPanel.tsx`。如果内容超过简单状态条，建议拆组件。

当前相关代码：

- `src/features/card/CardAssetBar.tsx`
- `src/features/card/CardAssetBar.module.css`
- `src/features/card/CardEditDrawer.tsx`
- `src/shared/api/scriptApi.ts`
- `src/shared/contracts/script.ts`
- i18n 消息定义位置需从现有 i18n 文件中确认，不要写死用户可见中文/英文。

UI 建议：

- 不做完整脚本编辑器。
- 不读取或展示完整脚本文本。
- 点击验证时调用：

```ts
scriptApi.validateLuaScript({ workspaceId, packId, cardId })
```

- 展示内容至少包括：
  - 总状态 `pass` / `warning` / `fail` / `inconclusive`
  - summary
  - stages 列表：`static`、`ocgcore_init`
  - issues 列表：severity、stage、code、message、line、suggestion
  - limitations
- loading 状态禁用验证按钮，避免重复调用。
- 无脚本时按钮可禁用，或允许点击后展示后端 `script_not_found`；历史阶段 8 写的是“无脚本展示”，两种都可，但要测试。
- helper 未构建时 UI 应展示 `inconclusive` 和 `helper_not_found` issue，不要把它当程序错误。

视觉约束：

- 沿用 `CardAssetBar` 的紧凑资源栏风格。
- 不做 landing page，不做新视觉系统。
- 资源栏宽度固定 300px，注意 issue 文本换行和滚动，不要撑破布局。
- 用户可见文本走现有 i18n。

建议测试点：

- 点击验证会调用 `scriptApi.validateLuaScript`。
- loading 状态显示并禁用按钮。
- `pass` / `warning` / `fail` / `inconclusive` 都能展示。
- issue 的 line/suggestion 可见。
- limitations 可见。
- `script_not_found` 或无脚本状态有明确展示。

## 不要做的事

- 不要在 UI 或 Agent 中重新实现验证规则。
- 不要绕过 `src/shared/api/scriptApi.ts` 直接调用 Tauri invoke。
- 不要让 Agent tool 修改脚本或自动修复脚本。
- 不要宣称 smoke/scenario 已实现。
- 不要把 helper 打包为 Tauri sidecar，除非用户明确把发布打包纳入本轮范围。
- 不要把 `docs/history/lua_script_validation_platform_report_2026-06-15.md` 当作当前事实来源；它只说明下一阶段意图。

## 验证命令

阶段 7/8 完成后至少运行：

```powershell
npm test -- src/shared/api/scriptApi.test.ts
npm run typecheck
Push-Location src-tauri
cargo test application::script
cargo test infrastructure::ocgcore_validator
cargo check
Pop-Location
```

如果改了 helper 或要证明真实 ocgcore 路径仍可用，再运行：

```powershell
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/build.ps1
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/run-fixtures.ps1
```

当前最后一次完整验证结果：

```text
helper build: pass
helper fixtures: valid_getid => pass, missing_end => fail, missing_core => fail
cargo test application::script: 32 passed
cargo test infrastructure::ocgcore_validator: 7 passed
cargo check: pass
npm test -- src/shared/api/scriptApi.test.ts: 1 passed
npm run typecheck: pass
```

## 完成后文档更新

阶段 7 完成后：

- 更新 `docs/agent.md`，新增 `validate_lua_script` 只读工具说明。
- 更新 `docs/code_structure_api.md`，如新增 `src/features/agent/tools/scriptTools.ts`。

阶段 8 完成后：

- 更新 `docs/ui_design.md`，说明 Card 资源栏/编辑抽屉中的脚本验证入口和报告展示约定。
- 更新 `docs/functional_spec.md`，如果用户可见能力从“后端支持验证”变成“UI 可触发验证”。
- 更新 `docs/code_structure_api.md`，如新增 UI 组件或 hook。

完成后再用 `rg` 检查旧事实：

```powershell
rg "尚未调用 helper|不运行 ocgcore|当前阶段返回静态检查报告|Agent/UI 入口仍属于后续阶段" docs
```
