# P8 实现记录：导入后端闭环

## 概要

本轮完成了 P8 的后端优先版本：把一个 YGOPro runtime-style 资源集导入为新的作者态 custom pack。

当前交付覆盖：

- 导入 DTO、Tauri command、presentation command
- `preview_import_pack` 同步预检
- `execute_import_pack(preview_token)` 后台 Job 执行
- 内存态 import `preview_token` cache
- CDB、`strings.conf`、图片与脚本资源导入转换
- 前端 TypeScript contract 与 API 包装
- Rust 集成测试与前端类型校验

本轮没有实现完整 Import Pack UI。`Add Pack -> Import Pack` tab 仍可在后续前端包中接入本轮新增 API。

## 设计边界

导入对象是 YGOPro runtime-style 资源集，不是作者态 pack 目录。

必选输入：

- 源 `.cdb`
- `source_language`
- 新 pack 的 name / author / version
- 新 pack 的 display language order
- 当前 workspace id

可选输入：

- 新 pack description
- default export language
- `pics/`
- `pics/field/`
- `script/`
- `strings.conf`

导入始终创建一个新的 custom pack，不导入到已有 pack，也不自动打开导入后的 pack。由于当前 Job 模型没有 result payload，preview 阶段会预分配 `target_pack_id`，前端可在 job 成功后用该 id 调 `open_pack`。

## 后端模块落点

### 1. DTO 与命令

新增 `src-tauri/src/application/dto/import.rs`：

- `PreviewImportPackInput`
- `ImportPreviewDto`
- `ExecuteImportPackInput`

新增命令：

- `preview_import_pack(input) -> PreviewResultDto<ImportPreviewDto>`
- `execute_import_pack(input) -> JobAcceptedDto`

命令已接入：

- `src-tauri/src/presentation/commands/app_commands.rs`
- `src-tauri/src/tauri_commands.rs`
- `src-tauri/src/main.rs`

### 2. Preview Token Cache

新增 `src-tauri/src/runtime/preview_token_cache.rs`，当前只实现 import preview entry。

`ImportPreviewEntry` 记录：

- `preview_token`
- `workspace_id`
- `target_pack_id`
- `snapshot_hash`
- `expires_at`
- `input_snapshot`

cache 目前为内存态，随应用重启丢失。`execute_import_pack` 会 consume token，重复执行同一 token 会失败。

### 3. ImportService

新增 `src-tauri/src/application/import/service.rs`。

`preview_import_pack` 负责：

- 校验当前 workspace 与输入 workspace id 一致
- 预分配 target pack id
- 构造目标 metadata 和 pack storage path
- 读取 `.cdb`
- 将 CDB 文本语言从 `"default"` 改为用户指定的 `source_language`
- 读取可选 `strings.conf`
- 将 strings 语言从 `"default"` 改为用户指定的 `source_language`
- 扫描可选资源目录
- 汇总 warning/error issues 和缺失资源统计
- 生成 `preview_token / snapshot_hash / expires_at`

`execute_import_pack` 负责：

- consume preview token
- 检查 token 未过期
- 检查 workspace 未切换
- 重新 prepare import
- 若 preview 有 error，则拒绝提交 job
- 复核 source snapshot hash
- 检查目标 pack path 尚不存在
- 提交 `JobKindDto::ImportPack`

Job runner 阶段：

- `validating_preview`
- `writing_pack`
- `refreshing_workspace`
- `import_ready`

写入完成后会刷新 current workspace overviews，但不会自动 open pack。

## 导入转换规则

### CDB

复用现有 `infrastructure/ygopro_cdb::load_cards_from_cdb`。

转换规则：

- `datas/texts` -> `CardEntity`
- `texts.name / desc / str1..str16` -> `CardEntity.texts[source_language]`
- 导入时为每张卡分配新的 `card.id`
- 不落盘 `"default"` 文本语言 key

### strings.conf

复用现有 `infrastructure/strings_conf::load_records`。

转换规则：

- `strings.conf` -> `PackStringsFile.entries`
- 每条 record 的 value 写入 `values[source_language]`
- 不落盘 `"default"` 字符串语言 key

### 资源

运行时资源到作者态资源的映射：

- `pics/<code>.jpg` -> `pics/<code>.jpg`
- `pics/field/<code>.jpg` -> `pics/field/<code>.jpg`
- `script/c<code>.lua` -> `scripts/c<code>.lua`

主卡图复用现有 `assets::import_main_image`，导入后统一为 `400 x 580` jpg。场地图复用 `assets::import_field_image`，转 jpg 但不缩放。脚本按字节复制。

## 校验行为

阻断错误：

- workspace mismatch
- CDB 不可读或 schema 不符合预期
- pack metadata validation error
- CDB 内重复 code
- 导入 code 与标准包 code 完全冲突
- card 结构错误
- 目标 pack path 已存在
- preview token 缺失、过期、已消费或 source snapshot 变化

warning 可继续：

- 缺主图
- 缺脚本
- 场地魔法缺场地图
- code 在标准保留范围
- code 在推荐范围外
- code 与其他 custom pack 冲突
- code 与已有 code 间距过小
- `display_language_order` 不包含 `source_language`

如果 `display_language_order` 不包含 `source_language`，preview 会给 warning；执行时会把 `source_language` 插入到实际落盘 language order 的最前面。

source snapshot 当前包含：

- workspace id
- source language
- CDB 文件 stamp
- 可选 `strings.conf` 文件 stamp
- 可选资源目录内文件 stamp
- 目标 pack path

因此 preview 后修改 CDB、strings 或资源目录内文件，会导致 execute 阶段 stale。

## 前端合同

新增：

- `src/shared/contracts/import.ts`
- `src/shared/api/importApi.ts`

并导出到：

- `src/shared/contracts/app.ts`
- `src/shared/api/app.ts`

当前只提供 API 合同，不改 Import Pack UI。

## 测试覆盖

新增 `src-tauri/tests/import_pack_flow.rs`。

覆盖场景：

- 构造临时 `.cdb`、`pics/`、`pics/field/`、`script/`、`strings.conf`，preview 后 execute job 成功
- 导入后 metadata、cards、strings、图片、场地图、脚本均正确落盘
- CDB 和 strings 文本写入 `zh-CN`，不出现 `"default"`
- 缺资源只产生 warning 和统计，不阻断 execute
- CDB 内重复 code 产生 error，`execute_import_pack` 拒绝提交 job

已执行验证：

- `cargo test --offline`
- `npm.cmd run typecheck`

## 已知边界

当前 P8 后端闭环还没有：

- 完整 Import Pack UI
- Job result payload
- cancel import job
- preview token 持久化
- 导入到已有 pack
- 多语言自动识别
- 非 `.jpg` runtime 图片名兼容
- CDB 写入或导出执行

当前导入执行会在 job 内重新读取源文件和转换资源。预检与执行之间若源文件或资源目录内容变化，execute 会失败并要求重新 preview。

## 审阅重点

建议重点审阅：

1. `ImportService::prepare_import` 的校验分级是否符合产品预期
2. `execute_import_pack` 是否正确 consume token，并在提交 job 前后都复核 preview
3. source snapshot 对 CDB、strings、资源目录变化的覆盖是否足够
4. 语言映射是否彻底避免 `"default"` 落盘
5. 导入 pack 写入是否和现有 `PackService::create_pack` 的 workspace/session 语义一致
6. 资源缺失 warning 与资源转换失败 error 的边界是否合理
7. 前端 contract 的 camelCase 输入与 snake_case 返回是否和现有 API 风格一致
