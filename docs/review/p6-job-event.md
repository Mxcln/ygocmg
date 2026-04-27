# P6 实现记录：Job / Event 后端基础设施

## 概要

本轮完成了 P6 的“后端优先”版本，目标是为后续标准包索引、导入、导出三类长任务建立统一运行通道。

当前交付覆盖：

- Job 对外 DTO 与查询命令
- 内存态 `JobRuntime` / `JobStore`
- 后台任务提交与状态更新
- `JobContext` 进度上报
- `job:progress` / `job:finished` 事件模型
- Tauri 事件桥
- 前端 TypeScript 合同与 API 包装
- 测试专用任务的生命周期与事件验证
- 审阅后修复：progress 事件 best-effort、Debug 能力、store 断言、TS 事件 payload 类型

本轮没有实现完整前端任务中心，也没有暴露产品级 demo job。普通短命令仍保持同步 `invoke`，不迁入 Job 系统。

## 设计边界

P6 的边界被刻意收窄为长任务基础设施。

进入 Job 系统的目标任务类型：

- `standard_pack_index_rebuild`
- `import_pack`
- `export_bundle`
- `test`，仅用于测试

不进入 Job 系统的现有短任务：

- card create / update / delete
- strings list / upsert / delete
- 单卡图片、场地图、脚本资源操作
- workspace / pack 的普通 CRUD

原因是这些短任务已经有稳定的同步命令、`WriteResultDto`、`confirmation_token`、`revision/source_stamp` 保护链路。P6 只负责给真正长耗时、可分阶段反馈的任务提供运行方式。

## 后端模块落点

### 1. Job DTO

新增 `src-tauri/src/application/dto/job.rs`。

主要类型：

- `JobKindDto`
- `JobStatusDto`
- `JobAcceptedDto`
- `JobSnapshotDto`
- `GetJobStatusInput`

字段语义：

- `job_id`：运行时任务 ID
- `kind`：任务类型
- `status`：`pending / running / succeeded / failed / cancelled`
- `stage`：当前阶段名
- `progress_percent`：可选百分比，内部限制到 `0..=100`
- `message`：面向 UI 的短消息
- `started_at / finished_at`：运行时间戳
- `error`：失败时保留 `AppErrorDto`

当前 `cancelled` 只是状态模型预留，还没有 `cancel_job` 命令。

### 2. Runtime Jobs

新增 `src-tauri/src/runtime/jobs/mod.rs`。

核心结构：

- `JobRuntime`
- `JobStore`
- `JobSnapshot`
- `JobContext`

`JobRuntime` 负责：

- 生成 UUIDv7 job id
- 插入初始 `pending` snapshot
- 通过 `tauri::async_runtime::spawn_blocking` 后台执行 runner
- 查询单个任务状态
- 查询 active 任务

`JobStore` 当前是内存态 `BTreeMap<JobId, JobSnapshot>`。

当前 `list_active` 只返回：

- `pending`
- `running`

完成、失败、取消任务不会出现在 active 列表中，但仍可通过 `get_job_status` 查询到运行时内存中的 snapshot。

`JobContext` 负责：

- `progress(stage, percent, message)`
- 标记 `running`
- 标记 `succeeded`
- 标记 `failed`
- 发布进度事件和完成事件

失败任务不会只通过后台线程返回错误；错误会落入任务 snapshot 的 `error` 字段，并触发 `job:finished`。

### 3. Runtime Events

新增 `src-tauri/src/runtime/events/mod.rs`。

核心结构：

- `AppEvent`
- `JobProgressEvent`
- `JobFinishedEvent`
- `AppEventBus`
- `NoopEventBus`

事件名固定为：

- `job:progress`
- `job:finished`

事件 payload 使用 snake_case 字段，与 Rust DTO / TS 合同保持一致。

`runtime/events` 不依赖 Tauri。它只定义事件模型和事件总线 trait。

### 4. Tauri 事件桥

新增 `src-tauri/src/infrastructure/tauri_event_bus.rs`。

`TauriEventBus` 持有 `AppHandle`，实现 `AppEventBus`，把 runtime 事件转成 Tauri window event：

- `AppEvent::JobProgress` -> `emit("job:progress", payload)`
- `AppEvent::JobFinished` -> `emit("job:finished", payload)`

这样 runtime 层保持纯净，Tauri 依赖只落在 infrastructure。

### 5. AppState 接入

`AppState` 新增：

- `jobs: JobRuntime`
- `event_bus: SharedEventBus`

同时 `sessions` 和 `confirmation_cache` 改为：

- `Arc<RwLock<SessionManager>>`
- `Arc<RwLock<ConfirmationCache>>`

这样后台任务和未来真实长任务可以安全持有可共享状态句柄。现有代码里的 `state.sessions.read()` / `state.sessions.write()` 调用仍然可用，避免大面积改写。

`AppState::new` 默认使用 `NoopEventBus`，测试和 Tauri 启动路径可以注入自定义事件总线。

### 6. Commands

新增两个 Tauri command：

- `get_job_status`
- `list_active_jobs`

它们通过 `application/jobs/service.rs` 调用 `JobRuntime`，再映射成 DTO。

当前没有新增 `submit_job` 或 demo command；真实任务提交会在 P7 / P8 / P9 对应服务里接入。

## 前端合同与 API

新增：

- `src/shared/contracts/job.ts`
- `src/shared/api/jobApi.ts`

`job.ts` 导出：

- `JobKind`
- `JobStatus`
- `JobAccepted`
- `JobSnapshot`
- `GetJobStatusInput`

`jobApi.ts` 导出：

- `getJobStatus(input)`
- `listActiveJobs()`

同时更新：

- `src/shared/contracts/common.ts` 增加 `JobId`
- `src/shared/contracts/app.ts` 导出 job 合同
- `src/shared/api/app.ts` 导出 `jobApi`

本轮不添加全局任务 UI，也不添加前端事件监听封装。后续任务中心可以基于 `@tauri-apps/api/event` 监听 `job:progress` 和 `job:finished`。

## 任务生命周期

当前 Job 生命周期如下：

1. `submit(kind, runner)`
2. 插入 `pending` snapshot
3. 后台线程启动后标记 `running`
4. runner 可多次调用 `ctx.progress(...)`
5. runner 返回 `Ok(())` 后标记 `succeeded`
6. runner 返回 `Err(AppError)` 后标记 `failed`
7. runner panic 时标记为 `failed`，错误码为 `job.panic`

完成事件会在最终状态写入 store 后发布。

当前成功结束时会统一设置：

- `status = succeeded`
- `stage = succeeded`
- `progress_percent = 100`
- `finished_at = now`

失败结束时会统一设置：

- `status = failed`
- `stage = failed`
- `message = error.message`
- `error = AppErrorDto`
- `finished_at = now`

## 与现有架构的关系

P6 没有替代现有同步写入链路。

现有 card / strings / resource 写入仍使用：

- `WriteResultDto`
- `confirmation_token`
- `ConfirmationCache`
- `PackSession.revision`
- `PackSession.source_stamp`

P6 后续主要服务于 `preview_token` 执行阶段。

例如后续导出流程应收敛为：

1. `preview_export_bundle` 返回 `preview_token / snapshot_hash / expires_at / preview data`
2. `execute_export_bundle(preview_token)` 提交 `export_bundle` job
3. job runner 开始时复核 `preview_token` 对应快照仍有效
4. job 通过 `JobContext` 上报 `validating / writing_cdb / writing_strings / copying_assets / finished`
5. UI 通过事件和查询展示进度与结果

## 测试覆盖

新增 `src-tauri/tests/job_runtime.rs`。

覆盖场景：

1. 成功任务：
   - 提交 `test` job
   - 上报多次进度
   - 最终状态为 `succeeded`
   - `progress_percent = 100`
   - `started_at / finished_at` 存在
   - `job:progress` 和 `job:finished` 事件均被记录

2. 失败任务：
   - runner 返回 `AppError`
   - 最终状态为 `failed`
   - snapshot 中保留错误 code/message
   - `job:finished` 事件包含错误信息

3. active 列表：
   - 运行中任务会出现在 `list_active_jobs`
   - 任务完成后从 active 列表消失

验证命令：

- `cargo test --offline`
- `npm.cmd run typecheck`

注意：本机 PowerShell 执行策略会阻止 `npm.ps1`，因此使用 `npm.cmd run typecheck`。

## 审阅后修复

P6 代码审阅后已完成以下收口：

1. `JobContext::progress` 不再传播事件发布错误；状态落盘失败仍会返回错误，事件发送失败只按 best-effort 忽略
2. `mark_running()` 改为返回 `AppResult<()>`，避免任务实际执行但状态停留在 `pending`
3. `JobStore::insert` / `JobStore::update` 增加 `debug_assert`，防止重复插入或更新不存在的 job
4. `JobRuntime` 与 `AppState` 补充手动 `Debug`
5. `AppEvent` 移除外层 serde tag/derive，避免和实际 Tauri payload 语义混淆
6. 前端 `job` contract 补充 `JobProgressEvent` 与 `JobFinishedEvent`

暂缓项：

1. runtime 自有 `JobKind / JobStatus` 类型暂未拆出，待真实 runner 接入前再处理
2. `JobStore` 容量上限与完成任务清理策略暂未实现

## 已知边界

当前 P6 还没有：

- `cancel_job`
- 持久化任务历史
- 完整前端任务中心 UI
- 产品级任务提交命令
- 导入 / 导出 / 标准索引的真实 runner
- 预检 token cache
- job 结果归档结构

当前完成/失败任务仍留在内存 `JobStore`，可用 `get_job_status` 查询；但应用重启后不会恢复。

`JobStore` 当前不做容量裁剪。短期可接受，因为 P6 还没有产品级大量任务入口。后续若任务中心长时间运行，应增加最近完成任务上限或清理策略。

## 后续接入建议

### P7 标准包只读接入

建议新增：

- `rebuild_standard_pack_index -> JobAcceptedDto`

runner 阶段建议：

- `scanning_ygopro`
- `reading_cdb`
- `reading_strings`
- `building_index`
- `writing_cache`
- `finished`

完成后发布或补充 `StandardPackIndexUpdated` 事件。

### P8 导入

建议流程：

- `preview_import_pack` 仍为同步预检
- `execute_import_pack(preview_token) -> JobAcceptedDto`
- job runner 内复核 token 快照
- 执行 CDB / strings / pics / script 转换与写入

runner 阶段建议：

- `validating_preview`
- `reading_source`
- `converting_cards`
- `copying_assets`
- `writing_pack`
- `refreshing_workspace`
- `finished`

### P9 导出

当前已有 `preview_export_bundle` 后端骨架。

建议新增：

- `execute_export_bundle(preview_token) -> JobAcceptedDto`

runner 阶段建议：

- `validating_preview`
- `generating_cdb`
- `generating_strings`
- `copying_images`
- `copying_scripts`
- `finished`

导出执行前必须复核：

- preview token 存在且未过期
- pack id 集合一致
- export language 一致
- output dir / output name 一致
- 相关 `revision/source_stamp` 未变化

## 审阅重点

后续审阅 P6 时建议重点看：

1. `AppState` 改为 `Arc<RwLock<...>>` 后，现有同步服务是否仍保持行为一致
2. `JobRuntime::submit` 是否有足够清晰的失败落盘语义
3. `JobContext::progress` 的状态落盘错误与事件发布错误是否继续保持分离
4. `list_active_jobs` 只返回 pending/running 是否符合前端任务中心预期
5. Tauri 事件 payload 与 TS 合同字段是否保持一致
6. 后续真实长任务是否避免长时间持有 `sessions.write()`

总体而言，本轮 P6 已经提供了后续 P7/P8/P9 所需的最小异步任务骨架，但还没有进入具体业务任务执行层。
