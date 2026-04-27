---
name: P6 Code Review
overview: 对 P6 Job / Event 后端基础设施代码变更的全面审阅，涵盖架构设计、线程安全、错误处理、序列化一致性、测试覆盖等维度。
todos:
  - id: fix-progress-error
    content: 修复 JobContext::progress 中事件发布错误传播问题 — 分离持久化错误与事件发布错误
    status: completed
  - id: restore-debug
    content: 为 JobRuntime 手动实现 Debug，恢复 AppState 的 Debug derive
    status: completed
  - id: add-store-assertions
    content: 在 JobStore::insert/update 中添加 debug_assert 防止误用
    status: completed
  - id: add-ts-event-types
    content: 在 src/shared/contracts/job.ts 中补充 JobProgressEvent 和 JobFinishedEvent 类型
    status: completed
  - id: consider-runtime-types
    content: 考虑在 runtime 层定义独立的 JobKind/JobStatus 枚举，解除对 DTO 层的直接依赖
    status: deferred
isProject: false
---

# P6 Job / Event 后端基础设施 — 代码审阅报告

## 一、总体评价

P6 的实现质量较高，架构分层清晰，目标边界收敛得当。主要成果：

- runtime 层（`events`、`jobs`）不依赖 Tauri，保持了可测试性
- 事件总线抽象（`AppEventBus` trait）使得测试可以注入 `RecordingEventBus`
- `AppState` 改造为 `Arc<RwLock<...>>` 对现有同步链路侵入极小
- 测试覆盖了成功、失败、active 列表三个核心场景

变更范围适中（新增约 500 行核心代码 + 180 行测试），同时删除了 1300+ 行过期临时文档，保持了仓库卫生。

---

## 二、需要修复的问题

### 2.1 `AppState` 移除了 `Debug` derive — 可能影响日志与调试

原本 `AppState` 派生了 `#[derive(Debug)]`，改动后变成了 `#[derive(Clone)]`，丢失了 `Debug`。这是因为 `JobRuntime` 和 `SharedEventBus`（`Arc<dyn AppEventBus>`）没有实现 `Debug`。

```1:2:src-tauri/src/bootstrap/app_state.rs
// Before: #[derive(Debug)]
// After:  #[derive(Clone)]
```

**影响**：如果任何地方对 `AppState` 调用 `{:?}` 格式化（日志、错误上下文），编译会失败。目前碰巧没有，但这是一个脆弱点。

**建议**：为 `JobRuntime` 手动实现 `Debug`（打印 store 中的任务数量即可），为 `AppState` 恢复 `Debug` derive 或手动实现。

### 2.2 `JobStore::insert` 与 `JobStore::update` 完全相同

```38:48:src-tauri/src/runtime/jobs/mod.rs
    pub fn insert(&mut self, snapshot: JobSnapshot) {
        self.jobs.insert(snapshot.job_id.clone(), snapshot);
    }

    pub fn get(&self, job_id: &JobId) -> Option<JobSnapshot> {
        self.jobs.get(job_id).cloned()
    }

    pub fn update(&mut self, snapshot: JobSnapshot) {
        self.jobs.insert(snapshot.job_id.clone(), snapshot);
    }
```

两个方法的实现完全一致。语义上 `insert` 用于首次插入、`update` 用于后续更新，但实现上没有任何区别，也没有做"是否已存在"的断言。

**建议**：保留语义命名没问题，但至少在 `insert` 中加一个 `debug_assert!(!self.jobs.contains_key(...))` 来捕获意外重复插入，或在 `update` 中加一个 `debug_assert!(self.jobs.contains_key(...))` 来确保更新的是已存在的条目。

### 2.3 `JobContext::progress` 返回事件发布错误，可能中断 runner

```152:169:src-tauri/src/runtime/jobs/mod.rs
    pub fn progress(
        &self,
        stage: impl Into<String>,
        progress_percent: Option<u8>,
        message: Option<String>,
    ) -> AppResult<()> {
        // ...
        self.persist(snapshot.clone())?;
        self.publish_progress(&snapshot)
    }
```

`progress()` 返回 `AppResult<()>`，如果事件发布失败（如 Tauri 窗口已关闭），错误会向上传播到 runner。如果 runner 使用了 `?` 运算符，一个事件发布失败会导致整个任务被标记为 `failed`。

对比 `succeed()` / `fail()` 方法已经用了 `if result.is_err() {}` 来吞掉发布错误，说明作者意识到了事件发布可能失败。但 `progress()` 没有做同样的处理。

**建议**：将 `progress()` 的事件发布与持久化分离 — 持久化失败应该报错，但事件发布失败应该只记日志（或静默忽略），不影响 runner 执行。或者在文档/注释中明确说明 runner 应该自行决定是否忽略 progress 错误。

### 2.4 `AppEvent` 序列化格式可能与 Tauri emit 语义不匹配

```13:18:src-tauri/src/runtime/events/mod.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum AppEvent {
    JobProgress(JobProgressEvent),
    JobFinished(JobFinishedEvent),
}
```

`AppEvent` 使用了 adjacently-tagged enum 序列化（`tag = "type", content = "payload"`）。但在 `TauriEventBus` 中，emit 的是内部 payload 而不是整个 `AppEvent`：

```16:29:src-tauri/src/infrastructure/tauri_event_bus.rs
    fn publish(&self, event: AppEvent) -> AppResult<()> {
        match event {
            AppEvent::JobProgress(payload) => self
                .app_handle
                .emit(JOB_PROGRESS_EVENT, payload)
                // ...
            AppEvent::JobFinished(payload) => self
                .app_handle
                .emit(JOB_FINISHED_EVENT, payload)
                // ...
        }
        Ok(())
    }
```

这意味着 `AppEvent` 上的 `#[serde(tag = "type", content = "payload")]` 注解实际上从未在 Tauri 事件路径中被使用 — emit 的是 `JobProgressEvent` / `JobFinishedEvent`，不是 `AppEvent`。这个 serde 配置只在 `RecordingEventBus` 的测试（`events.lock().unwrap().push(event)`）中间接涉及。

**不是 bug**，但 `AppEvent` 上的 serde tag 配置目前是冗余的/误导的。如果未来有人试图将 `AppEvent` 整体序列化发送，会产生与当前 Tauri emit 不同的格式。

**建议**：要么移除 `AppEvent` 的 serde derive（它只是一个内部枚举），要么添加注释说明 serde 配置仅用于内部/测试用途。

---

## 三、值得改进但非阻塞的建议

### 3.1 `mark_running()` 中的错误被静默忽略

```171:173:src-tauri/src/runtime/jobs/mod.rs
    fn mark_running(&self) {
        let _ = self.progress(RUNNING_STAGE, Some(0), None);
    }
```

`let _ =` 吞掉了所有错误（包括 store lock poisoned）。如果 `mark_running` 失败，任务状态可能停留在 `pending` 但 runner 已经在执行，导致状态不一致。

**建议**：至少记录一条日志，或者让 `mark_running` 返回 `AppResult` 并在 `submit` 的 spawn 闭包里处理。

### 3.2 `catch_unwind(AssertUnwindSafe(...))` 的安全性

```102:116:src-tauri/src/runtime/jobs/mod.rs
        tauri::async_runtime::spawn_blocking(move || {
            let run_result = catch_unwind(AssertUnwindSafe(|| {
                context.mark_running();
                runner(context.clone())
            }));
            match run_result {
                Ok(Ok(())) => context.succeed(),
                Ok(Err(error)) => context.fail(error),
                Err(_) => context.fail(AppError::new(
                    "job.panic",
                    "job runner panicked during execution",
                )),
            }
        });
```

使用 `AssertUnwindSafe` 包裹整个闭包是可以接受的（runner 是一次性执行，panic 后不再使用共享状态），但需注意 `context.clone()` 在 `runner(context.clone())` 中传递的是 clone，panic 后 `context` 自身仍然可用来调用 `context.fail()`。这部分逻辑是正确的。

不过，如果 `context.fail()` 内部的 store 写入也 panic（极端情况下 lock poisoned），这个 panic 不会被捕获，会导致 `spawn_blocking` 线程 panic。

**建议**：可以在 `succeed()` / `fail()` 外层再套一层 `catch_unwind` 作为最终兜底，或者接受这个极端情况（lock poison 本身意味着系统状态已损坏）。

### 3.3 `submit` 的返回值 `JobAcceptedDto` 中 `kind` 字段直接使用了 DTO 层类型

```78:80:src-tauri/src/runtime/jobs/mod.rs
    pub fn submit<F>(&self, kind: JobKindDto, runner: F) -> AppResult<JobAcceptedDto>
```

`JobRuntime` 属于 runtime 层，但直接依赖了 `application/dto` 层的 `JobKindDto`、`JobAcceptedDto`、`JobStatusDto`、`JobSnapshotDto`。按照现有分层，runtime 不应该直接引用 application DTO。

在项目的其他部分（如 `runtime/sessions`、`runtime/confirmation_cache`），runtime 结构使用 domain 模型类型，由 application 层负责转换到 DTO。

**建议**：考虑在 runtime 层定义自己的 `JobKind` / `JobStatus` 枚举，让 `From` trait 在 application 层做转换。当前 P6 范围小且只有 4 种 job kind，暂时可接受，但随着 P7-P9 加入真实 runner，这个耦合会变得更明显。

### 3.4 事件 payload 中 `kind` 字段不在 TS 合约的事件类型中

前端 `job.ts` 定义了 `JobSnapshot` 和 `GetJobStatusInput`，但没有定义 `JobProgressEvent` 和 `JobFinishedEvent` 的 TypeScript 类型。后续前端监听 `job:progress` / `job:finished` 时需要手动匹配 payload 结构。

**建议**：在 `src/shared/contracts/job.ts` 中补充 `JobProgressEvent` 和 `JobFinishedEvent` 接口，方便 `@tauri-apps/api/event.listen<JobProgressEvent>("job:progress", ...)` 使用。

### 3.5 `JobStore` 无容量上限

文档已明确提到了这一点。当前可接受，因为还没有产品级入口会频繁创建 job。但建议在 `JobStore` 中预留一个 `const MAX_COMPLETED_JOBS: usize = 100` 之类的常量，在 `update` 时检查是否需要清理最旧的已完成任务。

---

## 四、确认正常的方面

### 4.1 `AppState` 改为 `Arc<RwLock<...>>` 后现有同步服务兼容性

检查了 `app_commands.rs`、`workspace/service.rs`、`pack/service.rs` 中所有 `state.sessions.read()` / `state.sessions.write()` 调用，`Arc<RwLock<T>>` 的 deref 行为使得 `.read()` / `.write()` 调用签名完全不变。兼容性没有问题。

### 4.2 serde 命名约定一致性

- Input 类型（`GetJobStatusInput`）使用 `camelCase` — 与现有 `card.rs`、`strings.rs` 等 Input 类型一致
- Enum 变体（`JobKindDto`、`JobStatusDto`）使用 `snake_case` — 与 `CardCategoryDto` 等一致
- 输出 DTO（`JobSnapshotDto`、`JobAcceptedDto`）使用 Rust 默认 snake_case — 与 `CardListRowDto` 等一致
- 事件 payload 也使用默认 snake_case — 与 TS 合约的 snake_case 字段一致

### 4.3 前端合约与 Rust DTO 字段对齐

对比 `JobSnapshotDto`（Rust）和 `JobSnapshot`（TS）：字段名、类型、可空性完全一致。`GetJobStatusInput` 的 `job_id`（Rust snake_case）对应 `jobId`（TS camelCase），通过 `#[serde(rename_all = "camelCase")]` 正确桥接。

### 4.4 测试设计

三个测试用例覆盖了文档中声明的核心场景。`RecordingEventBus` 是一个很好的测试工具。`wait_for_status` 带超时的轮询方式对于并发测试是合适的做法。active jobs 的第三个测试用了 `mpsc::channel` 来精确控制时序，避免了竞态条件。

### 4.5 Tauri command 注册

`main.rs` 中注册了 `get_job_status` 和 `list_active_jobs`。`tauri_commands.rs` 的包装层正确地做了 `State<'_, AppState>` 到 `&AppState` 的转换。

---

## 五、非 P6 变更说明

本次 diff 中还包含了一些 **纯格式化变更**（`app_commands.rs`、`tauri_commands.rs` 中的函数签名换行调整），这些是 `rustfmt` 格式化的结果，不影响功能。

前端侧的 `App.tsx`、`styles.css`、`CardAssetBar.tsx`、`CardEditDrawer.tsx`、`StringsListPanel.tsx`、`shellStore.ts`、`strings.ts` 在 git status 中显示已修改，但 `git diff` 显示无实质变更（仅行尾符 LF/CRLF 差异），不影响审阅。

---

## 六、审阅结论

| 分类         | 评价                                      |
| ------------ | ----------------------------------------- |
| 架构分层     | 良好，runtime 不依赖 Tauri                |
| 线程安全     | 基本正确，`Arc<RwLock>` 使用合理          |
| 错误处理     | 有一处需关注（progress 事件发布错误传播） |
| 序列化一致性 | 与现有约定一致                            |
| 测试覆盖     | 三个场景覆盖了核心生命周期                |
| 代码卫生     | 清理了过期文档，diff 整洁                 |

**建议优先修复**：第 2.3 条（progress 事件发布错误不应中断 runner）。

**建议关注但可延后**：第 2.1 条（恢复 Debug）、第 2.2 条（insert/update 断言）、第 3.3 条（runtime 对 DTO 的依赖）。

其余均为改进建议，不阻塞 P6 合入。

---

## 七、审阅后修复状态（2026-04-28）

已完成：

1. `JobContext::progress` 已改为只返回状态持久化错误；事件发布失败按 best-effort 忽略，不再导致 runner 失败
2. `mark_running()` 已改为返回 `AppResult<()>`，启动状态落盘失败会进入正常失败路径
3. `JobStore::insert` / `JobStore::update` 已增加 `debug_assert`
4. `JobRuntime` 和 `AppState` 已补充手动 `Debug`
5. `AppEvent` 已移除误导性的 serde tag/derive，Tauri 路径继续发送具体 payload
6. `src/shared/contracts/job.ts` 已补充 `JobProgressEvent` 和 `JobFinishedEvent`

暂缓：

1. runtime 层自有 `JobKind / JobStatus` 枚举暂未拆出，建议在 P7/P8/P9 接入真实 runner 前处理
2. JobStore 容量上限与完成任务清理策略暂未实现，当前没有产品级高频任务入口

复验：

1. `rustfmt --edition 2024 ...` 通过
2. `cargo test --offline` 通过
3. `npm.cmd run typecheck` 通过
