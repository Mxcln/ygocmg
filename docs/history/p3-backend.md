# P3 单卡编辑后端实现记录

## 概要

本次实现完成了 P3 单卡编辑后端的首个可用闭环，重点包括：

1. 单卡查询接口收口
2. 单卡创建/更新写入闭环
3. `PackSession` 快照、`cardlist` 缓存与版本骨架
4. 卡片 IPC/前端契约更新
5. review 后的结构修正
6. 基础自动化验证

本次实现采用“直接提交并返回 warnings”的语义，没有实现 staged confirmation。

## 已完成内容

### 1. 卡片 DTO 与 IPC 形状收口

新增后端 DTO 文件：

- `src-tauri/src/application/dto/card.rs`

当前已提供的 DTO / 输入输出包括：

- `CardListRowDto`
- `CardListPageDto`
- `CardDetailDto`
- `ListCardsInput`
- `GetCardInput`
- `SuggestCodeInput`
- `CodeSuggestionDto`
- `CreateCardInput`
- `UpdateCardInput`

同时扩展了：

- `src-tauri/src/application/dto/common.rs`

新增：

- `WriteResultDto<T>`

当前 `create_card` / `update_card` 返回：

- `WriteResultDto::Ok { data, warnings }`

保留了：

- `NeedsConfirmation`

但本轮不会产出该状态。

### 2. 卡片命令层更新

已更新 Tauri command 与 presentation command：

- `list_cards` 现在返回分页 `CardListPageDto`
- 新增 `get_card`
- `suggest_card_code` 返回 `CodeSuggestionDto`
- `create_card` / `update_card` 返回 `WriteResultDto<CardDetailDto>`

相关文件：

- `src-tauri/src/tauri_commands.rs`
- `src-tauri/src/presentation/commands/app_commands.rs`
- `src-tauri/src/main.rs`

所有卡片命令现在都显式携带：

- `workspace_id`
- `pack_id`
- `card_id`（当操作需要时）

服务层会校验传入的 `workspace_id` 与当前打开的 workspace 是否一致；若不一致，返回：

- `workspace.mismatch`

### 3. 读侧服务更新

`CardService` 已重写为读侧服务，当前职责为：

- `list_cards`
- `get_card`
- `suggest_code`
- `build_code_context`

相关文件：

- `src-tauri/src/application/card/service.rs`

当前行为：

#### `list_cards`

- 数据源来自已打开 `pack` 的 `card_list_cache`
- 不再每次重新扫描资源状态
- 支持：
  - `keyword`
  - `sort_by`
  - `sort_direction`
  - `page`
  - `page_size`
- 本轮最小排序字段支持：
  - `code`
  - `name`

#### `get_card`

- 从当前打开 `pack` 的快照中按 `card_id` 读取
- 返回：
  - 完整卡片数据
  - 当前资源状态
  - `available_languages`

#### `suggest_card_code`

- 基于现有 code policy 与 workspace 内已有卡号计算建议值
- 返回：
  - `suggested_code`
  - `warnings`
- review 修复后，现已显式校验 `workspace_id`

### 4. 写侧改为 pack 级提交

新增：

- `src-tauri/src/application/pack/write_service.rs`

当前已由 `PackWriteService` 承接：

- `create_card`
- `update_card`
- `delete_card`

当前写入流程为：

1. 校验显式 `workspace_id / pack_id / card_id`
2. 读取并克隆当前打开的 `PackSession`
3. 在锁外做 normalize / validate / warnings / 改号规划
4. 通过 `FsOperation` 计划执行写盘
5. 成功后重建新的 `PackSession`
6. 用整体替换覆盖旧 session
7. 刷新 workspace summary / pack overviews

与旧实现相比，本轮去掉了“持有 session 写锁时直接写盘”的路径。

### 5. 改号资源提交收口

`update_card` 改号时，以下操作已进入统一事务计划：

- 资源 rename
- `cards.json` 写入
- `metadata.json` 写入

即：

- 不再先改内存，再分别尝试写磁盘
- 写入成功后才替换运行时 session

这使得“改号 + 资源移动 + 数据落盘”成为一个更一致的提交单元。

review 修复后，这条路径也已去掉“改号分支 / 非改号分支”的重复后处理逻辑，统一为：

1. 组装 `operations`
2. 一次 `execute_plan`
3. 重建 session
4. 替换 session
5. 刷新 summary

### 6. PackSession 快照与缓存骨架

`PackSession` 已扩展为：

- `pack_id`
- `pack_path`
- `revision`
- `source_stamp`
- `metadata`
- `cards`
- `strings`
- `asset_index`
- `card_list_cache`

相关文件：

- `src-tauri/src/runtime/sessions/mod.rs`
- `src-tauri/src/application/pack/service.rs`

并新增了统一 helper：

- `build_pack_session(...)`

用于从磁盘作者态构造完整快照。

#### `revision`

当前规则：

- `open_pack` 初次构建 session 时为 `0`
- 程序内每次成功写入后 `revision + 1`
- `set_active_pack` 不改变 `revision`

#### `source_stamp`

当前采用最小稳定摘要，基于：

- `metadata.updated_at`
- `cards.json` 的修改时间/长度
- `strings.json` 的修改时间/长度

本轮没有引入 hash，也没有实现文件监控。

#### `asset_index`

当前在 `open_pack` / session 重建时统一扫描构建。

#### `card_list_cache`

当前规则：

- `open_pack` 时构建
- `create_card` 成功后整包重建
- `update_card` 成功后整包重建
- `delete_card` 成功后整包重建
- `update_pack_metadata` 若 pack 已打开，也会重建 session，从而重建缓存
- `close_pack` 时缓存随 session 一起丢弃

### 7. open / set_active / list 语义

当前语义如下：

#### `open_pack`

- 从磁盘读取作者态并构建完整 `PackSession`
- 若该 pack 已经打开，则直接复用现有 session
- 对外返回值保持为 `PackMetadata`

review 修复后，`open_pack` 不再把完整 `PackSession` clone 出来再在上层丢弃。

#### `set_active_pack`

- 只更新 `active_pack_id`
- 只持久化 workspace 会话信息
- 不重建 `card_list_cache`
- 不重新读取磁盘 pack 数据

#### `list_cards`

- 仅对已打开 pack 生效
- 若 pack 未打开，返回：
  - `pack.not_open`
- 不会隐式自动 open pack

### 8. 前端共享契约更新

已更新：

- `src/shared/contracts/card.ts`

新增/补齐：

- `CardDetail`
- `CardAssetState`
- `CardListPage`
- `WriteResult<T>`
- `SuggestCodeResult`
- `ListCardsInput`
- `GetCardInput`
- `CreateCardInput`
- `UpdateCardInput`
- `SuggestCodeInput`

并新增：

- `src/shared/api/cardApi.ts`

提供：

- `listCards`
- `getCard`
- `suggestCardCode`
- `createCard`
- `updateCard`

## review 后修复结果

针对 `docs/temp/p3-backend-review.md` 中当前确实成立的问题，本轮已处理：

1. `delete_card` 已收口到 `PackWriteService`
2. `suggest_card_code` 已显式校验 `workspace_id`
3. `update_card` 已去掉改号/非改号路径的大段重复后处理
4. `create_card` / `update_card` / `delete_card` 已统一复用单次写操作内的同一个 `now`
5. `open_pack` 已收窄为对外返回 `PackMetadata`，避免无意义的大对象 clone

未处理但当前可接受的项：

1. DTO 仍直接复用部分 domain 枚举，而非完全映射为字符串 DTO
2. `WriteResultDto::NeedsConfirmation` 仅占位，尚无实际确认流

已确认不是当前代码问题的项：

1. `has_main_image` / `has_image` 命名不匹配

当前代码中的后端与前端契约均为：

- `has_image`
- `has_script`
- `has_field_image`

## 与原计划相比的保留项

本轮没有完全覆盖计划中的所有延伸点，当前保留如下：

### 1. `open_pack` 对外返回值未升级

当前 `open_pack` 的 Tauri / presentation 返回值仍然是：

- `PackMetadata`

而不是完整 `PackSnapshotDto` 或 `PackSession` 映射 DTO。

内部完整快照已经存在，但没有把这个新形状继续暴露到 IPC 表面。

### 2. staged confirmation 未实现

虽然 `WriteResultDto` 中保留了：

- `NeedsConfirmation`

但本轮所有单卡写操作均采用：

- 成功写盘后直接返回 `warnings`

没有实现：

- `confirm_card_write`
- token 失效判定
- confirmation staging

### 3. `source_stamp` 仍是最小实现

当前 `source_stamp` 只用于构建版本骨架，并不承担：

- 外部文件变化监测
- 自动失效
- confirm token 校验

这些仍留待后续版本。

### 4. DTO 与 domain 仍部分耦合

当前 `EditableCardDto` / `CardListRowDto` 等 DTO 仍直接使用 domain 枚举类型，通过 serde 输出 snake_case 字符串。

这在当前阶段可用，但后续若需要更严格的 IPC 契约隔离，可以进一步拆分为独立 DTO enum / string 字段。

## 验证结果

Rust 测试已通过：

- `cargo test --offline`

通过内容包括：

1. `minimal_authoring_flow_persists_and_renames_assets`
2. `workspace_id_mismatch_rejected_for_card_commands`
3. 既有 workspace / pack 相关测试

新增覆盖点包括：

- `create/update` 返回 `WriteResult::Ok`
- warnings 正常透出
- 改号后脚本 rename 成功
- 重开后 `list_cards` / `get_card` 可正确恢复
- `workspace_id` 不匹配时返回 `workspace.mismatch`
- review 修复后 `delete_card`、`suggest_card_code` 路径仍不回归

前端验证方面：

- `tsc --noEmit` 已通过

但：

- `vite build` 在当前环境下因 `spawn EPERM` 失败

该失败看起来是环境权限问题，不是本轮新增类型契约本身的错误。

## 当前结论

本轮已经把 P3 单卡编辑后端的核心闭环搭起来了，并完成了第一轮 review 修复：

- 有明确的查询接口
- 有明确的详情接口
- 有 pack 级写入编排入口
- 有 warnings 返回
- 有 `cardlist` 缓存
- 有 `revision/source_stamp` 骨架
- `delete_card` 已统一收口
- `suggest_card_code` 已补齐 workspace 校验
- `open_pack` 已收窄返回成本

下一步可以直接进入：

1. P3 前端 card list / drawer 接线
2. P3.5 staged confirmation / warning dialog
3. P4 资源管理与 strings 写侧继续接入 `PackWriteService`
