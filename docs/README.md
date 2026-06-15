# YGOCMG 文档入口

本目录保存当前权威文档和历史资料。当前实现事实优先来自代码和本目录下的核心文档；历史文档只作为背景。

## 阅读顺序

1. `functional_spec.md`：了解当前产品功能、用户工作流和领域术语。
2. `system_architecture.md`：了解前后端职责、后端分层、运行时状态和数据流。
3. `code_structure_api.md`：了解目录结构、API wrapper、Tauri commands 和 contracts 边界。
4. `ui_design.md`：了解当前 UI 布局、交互模式、主题和 i18n 约定。
5. `agent.md`：了解 AI Agent 的功能、架构、工具集、设置与边栏 UI。
6. `history/README.md`：需要追溯历史设计、评审或旧计划时再读。

## 当前权威文档

- `functional_spec.md`：功能描述文档。
- `system_architecture.md`：系统架构文档。
- `code_structure_api.md`：代码结构与 API 边界文档。
- `ui_design.md`：UI 设计与交互约定文档。
- `agent.md`：AI Agent 功能文档。

## 历史文档

`history/` 下的文档来自早期设计、阶段计划、评审、旧规范和 AI 知识草稿。它们可能和当前实现不一致，不能直接作为当前事实来源。

使用历史文档时应先用当前代码或核心文档核对。若历史文档中有仍然有效的稳定事实，应迁移或重写到核心文档，而不是继续引用历史路径。

近期调研：

- `history/lua_script_generation_research_2026-06-14.md`：AI Agent 根据卡片效果生成 YGOPro Lua 脚本的检索、模板、ocgcore 验证和 workflow/tool 分层调研。
- `history/lua_script_validation_platform_report_2026-06-15.md`：Lua 脚本验证平台设计、ocgcore 可行性 spike 结论和 MVP 实现计划。

## 维护规则

- 功能行为变化：更新 `functional_spec.md`。
- 架构或运行边界变化：更新 `system_architecture.md`。
- 目录、API wrapper、contract、Tauri command 调用面变化：更新 `code_structure_api.md`。
- UI 布局、交互、主题或 i18n 约定变化：更新 `ui_design.md`。
- AI Agent 的功能、工具集、设置或边栏 UI 变化：更新 `agent.md`。
- 只归档历史资料，不在 `history/` 中维护当前事实。

