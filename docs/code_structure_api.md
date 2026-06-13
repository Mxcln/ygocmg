# 代码结构与 API 边界

本文档说明当前代码组织和前后端调用边界。它不列出每个 DTO 字段的完整参考，字段详情以 `src/shared/contracts/*` 和 Rust DTO 为准。

## 前端目录

- `src/main.tsx`：React 入口。
- `src/app`：App shell、标题栏、侧边栏、工作区恢复、modal layer、notice、窗口和返回导航 hooks。
- `src/features/card`：卡片列表、批量选择/移动/删除、搜索、编辑抽屉、卡片信息表单、文本表单、资源栏和 setname 合并逻辑。
- `src/features/pack`：打开/创建/导入 pack、pack metadata。
- `src/features/strings`：Pack Strings 列表和浏览。
- `src/features/standardPack`：标准包状态、索引重建、标准卡/strings 浏览、高级筛选、只读详情。
- `src/features/workspace`：workspace 创建、打开和最近列表。
- `src/features/settings`：全局配置编辑。
- `src/features/agent`：AI Agent 右侧边栏、对话 loop、system prompt、Markdown 渲染、工具注册表与执行体（详见 `agent.md`）。
- `src/features/export`：导出 preview 和 execute。
- `src/features/dialogs`：确认和 warning 对话框。
- `src/shared/api`：Tauri command wrappers。
- `src/shared/contracts`：前后端边界类型。
- `src/shared/i18n`、`styles`、`theme`、`stores`、`utils`：共享基础设施。

## 后端目录

- `src-tauri/src/main.rs`：Tauri app bootstrap。
- `src-tauri/src/tauri_commands.rs`：command handler surface。
- `src-tauri/src/bootstrap`：应用状态 wiring。
- `src-tauri/src/application`：use cases，按 config、workspace、pack、card、strings、resource、import、export、standard_pack、jobs 等模块拆分。
- `src-tauri/src/domain`：领域模型和规则。
- `src-tauri/src/infrastructure`：文件系统、JSON store、YGOPro CDB、标准包、strings conf 等适配。
- `src-tauri/src/runtime`：sessions、jobs、events。
- `src-tauri/src/presentation`：对外 DTO/适配层。

## API Wrapper 分组

- `configApi`：initialize、load/save config。
- `workspaceApi`：最近 workspace、创建 workspace、打开 workspace。
- `packApi`：pack overview、创建/打开/关闭/激活/更新/删除 pack。
- `cardApi`：list/get/create/update/delete card、bulk delete/move card、推荐编号、确认 card 写入和确认 card 批量写入。
- `stringsApi`：list/get/upsert/delete Pack Strings、删除翻译、确认 strings 写入、`suggestSetnameKey`（按 config 推荐 base 区段建议下一个空闲顶级 setname key）。
- `resourceApi`：主卡图、场地图、脚本的导入/删除/创建/外部打开。
- `importApi`：preview/execute import pack。
- `exportApi`：preview/execute export bundle。
- `standardPackApi`：标准包状态、重建索引、搜索标准卡/strings、读取标准卡、打开标准脚本、列出标准 setnames。
- `jobApi`：查询 job 状态和 active jobs。
- `agentApi`：转发 DeepSeek chat 请求（`llm_chat`）。

## Tauri Command Surface

`src-tauri/src/main.rs` 注册当前命令表面。主要类别包括：

- Config：`initialize`、`load_config`、`save_config`
- Workspace：`list_recent_workspaces`、`create_workspace`、`open_workspace`、`delete_workspace`
- Pack：`create_pack`、`open_pack`、`close_pack`、`set_active_pack`、`update_pack_metadata`、`delete_pack`、`list_pack_overviews`
- Card：`list_cards`、`get_card`、`create_card`、`update_card`、`delete_card`、`bulk_delete_cards`、`move_cards`、`confirm_card_write`、`confirm_card_batch_write`、`suggest_card_code`
- Pack Strings：`list_pack_strings`、`get_pack_string`、`suggest_setname_key`、`upsert_pack_string`、`upsert_pack_string_record`、`delete_pack_strings`、`remove_pack_string_translation`、confirm commands
- Resource：main image、field image、script import/delete/create/open commands
- Import/Export：preview 和 execute commands
- Standard Pack：status、rebuild、search、get、open standard script、list setnames
- Jobs：`get_job_status`、`list_active_jobs`
- Agent：`llm_chat`（转发 DeepSeek chat completion，注入 API key，非流式）

## Contracts

- Common identifiers and validation shapes live in `common.ts`.
- Config shape lives in `config.ts`.
- Workspace and pack metadata live in `workspace.ts` and `pack.ts`.
- Card model, list rows, filters, write result, batch delete/move inputs/results, and confirmation inputs live in `card.ts`.
- Pack Strings types live in `strings.ts`.
- Import/export preview and job acceptance types live in `import.ts` and `export.ts`.
- Resource asset state and inputs live in `resource.ts`.
- Standard pack status/search/detail types live in `standardPack.ts`.
- DeepSeek chat 请求/响应消息类型（OpenAI 兼容格式）live in `agent.ts`；`config.ts` 含 `deepseek_api_key` 与 `agent_language`。

## Boundary Rules

- UI should call `src/shared/api/*`, not raw Tauri invoke.
- New commands should have a frontend wrapper and contract type when they cross the frontend/backend boundary.
- Backend should enforce write rules and validation; frontend can assist but should not be the source of truth.
- If a new feature needs long-running work, prefer the existing job pattern.
