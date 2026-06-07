---
name: P8 Import Code Review
overview: 对 P8 导入后端闭环代码变更的全面审阅，涵盖 preview-token 生命周期、数据完整性、性能、前端合同一致性等维度。
todos:
  - id: fix-token-cache-clear
    content: 在 workspace open/delete 时同步清理 preview_token_cache（2.1）
    status: completed
  - id: remove-dead-domain-model
    content: 移除未使用的 domain::import::model::ImportPreview 及相关 mod 声明（2.3）
    status: completed
  - id: reduce-prepare-calls
    content: 移除 execute_import_pack 中提交 job 前的冗余 prepare_import 调用（2.2）
    status: completed
  - id: rename-snapshot-hash
    content: 将 snapshot_hash 重命名为 snapshot_stamp 或改用真正的 hash（3.1，应与 export 统一）
    status: pending
  - id: lift-preview-result-type
    content: 将 PreviewResult<T> 从 import.ts 提升到 common.ts（3.4，可在 P9 时处理）
    status: pending
isProject: false
---

# P8 导入后端闭环 — 代码审阅报告

## 一、总体评价

P8 实现质量整体良好，架构设计合理，与现有代码风格保持了一致。主要成果：

- **preview -> token -> execute** 两阶段流完整且安全：token 单次消费、过期检测、snapshot stale 检测都已就位
- 相比 P7 export（无 token cache），import 引入了 `PreviewTokenCache` 做服务端 token 存储，设计上更完整
- 语言映射正确避免了 `"default"` 落盘
- `ImportResourcePlan` 将"扫描计划"与"执行写入"清晰分离
- `execute_plan`（事务写入）保证了 pack 创建的原子性
- 测试覆盖了成功、缺资源 warning、重复 code 阻断三个核心场景

变更范围适中：新增 ~720 行核心服务 + ~63 行 cache + ~43 行 DTO + ~17 行 domain model + ~294 行测试 + ~61 行前端合同/API。

---

## 二、需要修复的问题

### 2.1 `preview_token_cache` 在 workspace 切换/打开时未清理 — **遗留 token 安全漏洞**

[workspace/service.rs](src-tauri/src/application/workspace/service.rs) 中，`open_workspace` 会清理 `confirmation_cache`：

```112:121:src-tauri/src/application/workspace/service.rs
        self.state
            .confirmation_cache
            .write()
            .map_err(|_| {
                AppError::new(
                    "confirmation.cache_lock_poisoned",
                    "confirmation cache lock poisoned",
                )
            })?
            .clear();
```

但**没有**对 `preview_token_cache` 做同样的清理。同理，`clear_current_workspace_if_matches` 调用了 `confirmation_cache.invalidate_workspace()` 但遗漏了 `preview_token_cache.invalidate_workspace()`。

`PreviewTokenCache` 上的 `invalidate_workspace()` 方法已经实现，但在整个代码库中**从未被调用**，是事实上的死代码。

**影响**：切换 workspace 后，旧 workspace 的 import preview token 仍留在 cache 中。虽然 `execute_import_pack` 在 job 内部会通过 `ensure_workspace_matches` 防止跨 workspace 执行，但这属于防御纵深的缺失 — 如果用户在 workspace A 做了 preview，切到 workspace B 又切回 workspace A，旧 token 可能仍在 cache 中且未过期。

**建议**：在 `open_workspace` 的 cache clear 块中加入 `preview_token_cache.clear()`；在 `clear_current_workspace_if_matches` 的 invalidate 块中加入 `preview_token_cache.invalidate_workspace(workspace_id)`。

### 2.2 `prepare_import` 在成功导入路径上被调用 3 次 — 冗余 I/O

一次成功的导入经历三次 `prepare_import`：

1. `preview_import_pack` 中（用于生成 preview）
2. `execute_import_pack` 中、提交 job **之前**（用于预校验）
3. job runner 闭包内部（用于最终执行前复核）

每次 `prepare_import` 都会：
- 读取并解析 `.cdb` 文件
- 遍历 workspace 所有 pack，加载所有 `cards.json`（`build_import_code_context`）
- 扫描资源目录
- 执行全量校验

第 2 次调用（job 提交前）与第 3 次调用（job 内部）做的事完全相同，仅间隔 job spawn 的延迟。第 2 次调用的意义是"尽早失败"，但由于 job 内部也会重复相同检查，这个"early fail"的收益很有限，代价是多一次完整的 CDB 解析和全 workspace 卡片加载。

**建议**：移除 `execute_import_pack` 中提交 job 前的 `prepare_import` 调用。只保留 token 消费、过期检查、workspace 匹配检查和 pack path 存在性检查这些轻量校验即可。真正的 prepare 全交给 job runner 内部完成。

### 2.3 `domain::import::model::ImportPreview` 是死代码

[domain/import/model.rs](src-tauri/src/domain/import/model.rs) 定义了 `ImportPreview` 结构体，字段与 DTO 层的 `ImportPreviewDto` **完全相同**，但在整个代码库中**从未被引用**。搜索 `domain::import` 和 `import::model` 均无结果。

**影响**：这个结构增加了维护成本 — 如果修改 DTO 的字段，开发者可能忘记同步修改这个 domain model，或误以为它在某处被使用。

**建议**：二选一：
- (a) 移除 `domain/import/model.rs` 和 `domain/import/mod.rs`，及 `domain/mod.rs` 中的 `pub mod import`
- (b) 在 `ImportService` 中使用 domain model 进行内部处理，只在返回时转换为 DTO（与 card/pack 的分层模式一致）

---

## 三、值得改进但非阻塞的建议

### 3.1 `source_snapshot_hash` 不是 hash — 命名具有误导性

```638:660:src-tauri/src/application/import/service.rs
fn source_snapshot_hash(input: &PreviewImportPackInput, pack_path: &Path) -> AppResult<String> {
    let mut parts = vec![
        format!("pack_path:{}", pack_path.display()),
        format!("workspace_id:{}", input.workspace_id),
        format!("source_language:{}", input.source_language),
        format!("cdb:{}", path_stamp(&input.cdb_path)?),
    ];
    // ...
    Ok(parts.join("|"))
}
```

这个函数返回的是路径/大小/时间戳的明文拼接字符串，而非密码学 hash。功能上对于 stale 检测是够用的，但：
- 对于包含大量资源文件的目录，这个"hash"字符串可能非常长
- 字段名 `snapshot_hash` 和函数名 `source_snapshot_hash` 暗示了 hash 语义
- `PreviewResultDto` 的 `snapshot_hash` 字段会直接暴露给前端，前端可能基于命名做出错误假设

**建议**：要么真正做一次 hash（如 `sha256(parts.join("|"))`），要么将字段/函数重命名为 `snapshot_stamp` / `source_snapshot_stamp` 以避免语义误导。（注：export service 使用的也是明文拼接且命名为 `snapshot_hash`，如果修改应两处一致。）

### 3.2 `write_imported_pack` 将所有资源同时持有在内存中

```435:456:src-tauri/src/application/import/service.rs
        for (source, code) in prepared.resource_plan.main_images {
            operations.push(FsOperation::WriteFile {
                path: card_image_path(&prepared.pack_path, code),
                contents: crate::infrastructure::assets::import_main_image(&source)?,
            });
        }
        // ... field_images, scripts similarly ...
        execute_plan(operations)?;
```

所有图片（经 `import_main_image` 缩放为 400x580 jpg）和脚本都先读入 `Vec<u8>` 构成 `operations` 列表，然后一次性交给 `execute_plan`。对于一个拥有数百张卡的 CDB，内存峰值会比较高（每张主图 ~50-200KB jpg，加上场地图和脚本）。

当前阶段可接受，但如果未来需要支持大规模导入（1000+ 卡），建议将资源写入改为分批处理：先用 `execute_plan` 写入 metadata/cards/strings（核心数据），再逐张写入资源文件（资源可以容忍部分失败）。

### 3.3 `write_imported_pack` 未复用 `json_store::ensure_pack_layout`

现有 `PackService::create_pack` 使用 `json_store::ensure_pack_layout(&pack_path)` 来创建 pack 目录结构，而 `write_imported_pack` 手动构造了 `CreateDir` 操作。两种方式功能等价，但如果 `ensure_pack_layout` 未来增加新的子目录（如 `audio/`），import 路径不会自动跟随。

**建议**：考虑将 `ensure_pack_layout` 的目录列表抽为常量或独立函数，让 import 路径也可以复用。

### 3.4 前端 `PreviewResult<T>` 在 import.ts 中局部定义，应提升到 common

```32:37:src/shared/contracts/import.ts
export interface PreviewResult<T> {
  preview_token: PreviewToken;
  snapshot_hash: string;
  expires_at: string;
  data: T;
}
```

这个泛型接口对应后端的 `PreviewResultDto<T>`，P9 export 也会需要它。应提升到 `common.ts`，避免 P9 重复定义。

### 3.5 `directory_stamp` 仅扫描直接子文件，不递归

```662:679:src-tauri/src/application/import/service.rs
fn directory_stamp(path: &Path) -> AppResult<String> {
    // ... only entry_path.is_file() at top level
}
```

如果 `pics/` 下有意外的子目录（极端情况），其中的文件变化不会被检测到。对于 YGOPro 标准目录结构这不是问题，但值得在函数签名或文档中注明"仅扫描一层"。

### 3.6 `normalize_display_language_order` 的静默重排可能超出用户预期

```602:614:src-tauri/src/application/import/service.rs
fn normalize_display_language_order(
    languages: &[LanguageCode],
    source_language: &str,
) -> Vec<LanguageCode> {
    let mut normalized = Vec::new();
    normalized.push(source_language.to_string());
    for language in languages {
        if !normalized.iter().any(|current| current == language) {
            normalized.push(language.clone());
        }
    }
    normalized
}
```

用户传入 `["en-US", "zh-CN"]` + `source_language = "zh-CN"` 时，结果为 `["zh-CN", "en-US"]` — 用户指定的顺序被静默重排，`zh-CN` 被强制提到最前面。虽然 preview 阶段会给出 `source_language_not_in_display_order` 的 warning，但 warning 的措辞只说明 source language 不在列表中，并未告知会发生重排。

**建议**：如果 source language 已在列表中但不在首位，当前行为会保留原始顺序（因为 `normalized` 先 push source，然后跳过重复的 source）。这其实会把 source 提到最前面。建议检查这个行为是否符合产品预期，并在 warning 消息中明确说明自动重排的行为。

---

## 四、确认正常的方面

### 4.1 Token 消费语义正确

`execute_import_pack` 通过 `cache.remove_import_entry()` 消费 token，确保同一 token 只能执行一次。消费操作在 `write_cache` 锁内完成，并发安全。

### 4.2 语言映射彻底

`remap_card_language` 和 `remap_string_language` 正确地将 CDB/strings 中的 `"default"` key 替换为用户指定的 `source_language`。如果 source language 的 key 已经存在则跳过，避免覆盖。测试断言了 `"default"` 不出现在落盘数据中。

### 4.3 资源扫描逻辑合理

- 主图和脚本对所有卡片检查，缺失产生 warning
- 场地图仅对场地魔法卡检查缺失（`is_field_spell`），但如果非场地卡也有场地图文件则也导入（第 531-533 行），这个 "bonus" 行为合理
- `find_main_image` / `find_script` / `find_field_image` 使用 `find_existing` 做文件存在性检查，简洁清晰

### 4.4 代码校验上下文正确

`build_import_code_context` 加载了 workspace 内所有其他 pack 的卡片 code 到 `other_custom_codes`，然后对每张导入卡设置 `current_pack_codes` 为"导入批次内除自身外的所有 code"。这样每张卡都能检测到：
- 与标准包的冲突（hard error）
- 与其他自定义包的冲突（warning）
- 与导入批次内其他卡的间距不足（warning）

### 4.5 workspace meta 更新正确

`update_workspace_for_import` 只向 `pack_order` 追加新 pack id（带去重检查），然后 touch workspace timestamp。不修改 `open_pack_ids` 或 `last_opened_pack_id` — 这与 P8 记录文档中"不自动 open pack"的设计意图一致。

### 4.6 前端合同与 Rust DTO 一致

- Input 使用 `camelCase`（`#[serde(rename_all = "camelCase")]`）— 与 card/pack 等现有 Input 类型一致
- Output（`ImportPreviewDto`）使用 Rust 默认 `snake_case` — 与 `CardListRowDto`、`ExportPreviewDto` 等一致
- `PreviewToken` 类型已正确导出到 `common.ts`
- `ExecuteImportPackResult = JobAccepted` 复用了 job 合约

### 4.7 Tauri command 注册链路完整

`main.rs` -> `tauri_commands.rs` -> `app_commands.rs` -> `ImportService` 四层串联正确，`State` 注入和 `CommandResult` 转换与现有 command 一致。

### 4.8 测试覆盖充分

三个测试场景覆盖了核心路径：
- 完整导入（CDB + 图片 + 脚本 + strings）：验证 metadata、cards、texts、strings、资源文件、语言映射
- 缺资源导入：验证 warning 计数和 execute 不被阻断
- 重复 code 导入：验证 error 计数和 execute 被拒绝

`write_test_cdb` 和 `wait_for_job_success` 是很好的测试工具函数。

---

## 五、与 Export（P7）的模式差异说明

| 维度          | Export (P7)                            | Import (P8)                                  |
| ------------- | -------------------------------------- | -------------------------------------------- |
| Token 存储    | 无服务端存储，token 仅返回给前端       | `PreviewTokenCache` 服务端存储，consume 语义 |
| Execute       | 未实现                                 | 已实现，通过 Job 异步执行                    |
| Snapshot hash | 基于 session revision/stamp 的明文拼接 | 基于文件 metadata 的明文拼接                 |

Import 的 token cache 设计比 export 更完整，建议 P9 implement export execute 时也采用相同的 cache 模式。

---

## 六、审阅结论

| 分类     | 评价                                                    |
| -------- | ------------------------------------------------------- |
| 架构设计 | 良好，preview-token-execute 两阶段流完整                |
| 数据安全 | 有一处需修复（token cache 清理遗漏）                    |
| 性能     | 有一处可优化（三次 prepare 冗余）                       |
| 代码卫生 | 有一处死代码（domain model 未使用）                     |
| 前端合同 | 与现有约定一致，一处可提升（PreviewResult 应在 common） |
| 测试覆盖 | 三个场景覆盖核心路径，质量好                            |

**建议优先修复**：
1. 第 2.1 条 — workspace 切换时清理 `preview_token_cache`
2. 第 2.3 条 — 移除未使用的 `domain::import::model`

**建议关注但可延后**：
1. 第 2.2 条 — 移除 execute 中冗余的 `prepare_import`
2. 第 3.1 条 — `snapshot_hash` 命名问题（应与 export 一起处理）
3. 第 3.4 条 — `PreviewResult<T>` 提升到 common（可在 P9 时处理）

其余均为改进建议，不阻塞 P8 合入。

---

## 七、审阅后修复状态（2026-04-28）

已完成修复：

1. `preview_token_cache` 生命周期补齐
   - `open_workspace` 时同步 `clear()` import preview token
   - 删除当前 workspace 时同步 `invalidate_workspace(workspace_id)`
   - 新增集成测试覆盖 workspace 切换后旧 import token 失效

2. 移除死代码
   - 删除未使用的 `domain/import/model.rs`
   - 删除 `domain/import/mod.rs`
   - 移除 `domain/mod.rs` 中的 `pub mod import`

3. 降低 execute 前同步 I/O
   - 移除 `execute_import_pack` 提交 job 前的完整 `prepare_import`
   - token consume、过期检查、workspace 匹配保留在同步路径
   - blocking preview error 改为由 import job 进入 failed 状态返回
   - 对应测试已从同步错误改为等待 job failed

4. 调整导入语言顺序策略
   - 如果 `display_language_order` 已包含 `source_language`，保留用户原顺序
   - 只有缺失 `source_language` 时才插入到列表最前

已执行验证：

- `cargo test --offline`
- `npm.cmd run typecheck`

仍建议后续统一处理：

1. `snapshot_hash` 命名与真实语义不一致，应与 export 一起统一为真正 hash 或 `snapshot_stamp`
2. `PreviewResult<T>` 可在 P9 接 export execute 时提升到前端 `common.ts`
3. `directory_stamp` 当前只扫描一层目录，若未来支持嵌套资源目录需扩展
