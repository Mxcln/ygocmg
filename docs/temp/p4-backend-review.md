好的，我已经完整阅读了 P4 后端的实现计划文档、所有新增和修改的代码文件、以及集成测试。以下是详细的代码审阅报告。

---

## P4 后端代码审阅

### 变更总览

共涉及 **12 个文件**，新增约 **1573 行**，包括：
- 4 个新模块（`dto/strings.rs`、`dto/resource.rs`、`strings/*`、`resource/*`）
- 1 个新基础设施（`infrastructure/assets.rs`）
- 3 个已有文件的扩展（`write_service.rs`、`confirmation_cache.rs`、`transaction.rs`）
- 3 个接线文件（`app_commands.rs`、`tauri_commands.rs`、`main.rs`）
- 1 个测试文件（+773 行覆盖）

---

### 一、整体评价：合格，质量一致

本轮实现在架构层面与 P3 建立的模式保持了高度一致：
- **prepare / commit 两阶段写入** 继续用于 strings upsert
- **confirmation 流** 完整复制了 card 的 token → revision 校验 → 重新 prepare 路径
- **session 重建 + overviews 刷新 + cache invalidation** 保持一致
- **DTO / Service / WriteService / Presentation 四层分工** 延续不变
- 集成测试覆盖面广，正向/反向/幂等/改号迁移全部覆盖

---

### 二、值得肯定的点

1. **`FsOperation::DeleteFile` 补充得当**：rollback 逻辑正确（读取原始内容 → 删除 → 失败时用 `safe_write_bytes` 恢复），使资源删除纳入了事务计划。

2. **`confirmation_cache` 扩展干净**：`invalidate_pack` / `invalidate_workspace` / `clear` 均同步清理 `pack_strings_entries`，没有遗漏。

3. **`open_script_external` 三级错误覆盖完整**：未配置 → 编辑器不存在 → 脚本不存在，测试也覆盖了这三种场景。

4. **`assets.rs` 用 `safe_write_bytes` 落盘**：图片写入走的是 write-temp → rename 的安全路径，不是裸 `fs::write`。

5. **测试覆盖充分**：strings 增删改查 + 过滤排序分页、confirmation 成功 / revision 失效、图片导入尺寸校验、场地图非 field spell 拒绝、脚本创建重复报错、改号后三类资源同步迁移，总计约 761 行新增测试。

---

### 三、需要关注的问题

#### 问题 1（中等）：图片导入未走统一事务计划——已知偏差，但不一致

P4 文档已明确指出此问题，但值得进一步说明具体的不一致：

- **脚本导入 (`import_script`)** 先 `fs::read` 源文件，再把内容和 metadata 一起放进 `execute_plan`，走的是 `apply_asset_write`——**两个文件在同一个事务计划中**。
- **图片导入 (`import_main_image` / `import_field_image`)** 先调 `infrastructure::assets` 直接写入目标图片，再通过 `refresh_asset_only_session` 单独提交 metadata——**两个文件不在同一个事务计划中**。

如果图片写入成功但 metadata 写入失败：
- 图片文件已经落盘，无法自动回滚
- `asset_index` 重建时会检测到图片存在，但 `updated_at` 没有更新

**建议修复方向**：让 `assets.rs` 的 `import_main_image` / `import_field_image` 返回处理后的 `Vec<u8>` 而不是直接写入文件，然后在 `write_service` 中通过 `apply_asset_write` 走统一事务计划。这样可以消除脚本和图片两条路径的不对称。

#### 问题 2（低）：`apply_asset_write` / `apply_asset_delete` / `refresh_asset_only_session` 高度重复

这三个 private 方法共享几乎完全相同的 touch metadata → execute_plan → build_pack_session → replace_and_refresh → get asset_state 管线，只是 `execute_plan` 传入的 operations 不同。可以抽取一个统一的 `fn apply_asset_operation(ops: Vec<FsOperation>, ...)` 消除重复。当前约 **80 行**可以压缩到 **30 行**。

#### 问题 3（低）：`validate_pack_strings_or_err` 丢弃了具体校验细节

```3:12:src-tauri/src/application/pack/write_service.rs
fn validate_pack_strings_or_err(strings: &PackStringsFile) -> AppResult<()> {
    let issues = validate_pack_strings(strings);
    if issues.iter().any(|issue| matches!(issue.level, IssueLevel::Error)) {
        return Err(AppError::new(
            "pack_strings.validation_failed",
            "pack strings contain validation errors",
        ));
    }
    Ok(())
}
```

对比卡片写入路径会把 warnings 带回给前端，strings 写入路径在遇到 Error 级 issue 时只返回一个 generic 错误消息，具体是哪条 entry 有问题完全被吃掉了。建议在 `AppError` 的 detail 里带上 `issues` 的摘要。

#### 问题 4（低）：`delete_pack_strings` 在 `app_commands.rs` 包裹了 `WriteResultDto` 但永远不会返回 `NeedsConfirmation`

```189:200:src-tauri/src/presentation/commands/app_commands.rs
pub fn delete_pack_strings(
    state: &AppState,
    input: DeletePackStringsInput,
) -> AppResult<WriteResultDto<DeletePackStringsResultDto>> {
    let (_, deleted_count) = crate::application::pack::write_service::PackWriteService::new(state)
        .delete_pack_strings(&input.workspace_id, &input.pack_id, &input.language, &input.entries)?;
    Ok(WriteResultDto::Ok {
        data: DeletePackStringsResultDto { deleted_count },
        warnings: Vec::new(),
    })
}
```

这是一个 API 设计选择——为了前端调用的统一性而使用 `WriteResultDto`。可以接受，但如果确定 delete 永远不走 confirmation，直接返回 `DeletePackStringsResultDto` 会更诚实。需要和前端约定一致即可。

#### 问题 5（极低）：`CardAssetState` 字段顺序在 domain 和 DTO 间不一致

- Domain (`model.rs`): `has_image, has_script, has_field_image`
- DTO (`resource.rs`): `has_image, has_field_image, has_script`

因为使用命名字段和 serde，不会造成功能问题，但可读性上最好统一。

#### 问题 6（极低）：`create_empty_script` 存在 TOCTOU 窗口

```rust
if target_path.exists() {
    return Err(AppError::new("resource.script_exists", ...));
}
self.apply_asset_write(..., target_path, Vec::new())
```

`exists()` 检查和 `apply_asset_write` 之间有一个极小的竞争窗口。在单用户桌面应用中基本不会触发，但如果后续引入并行写入需要注意。

---

### 四、安全性与健壮性检查

| 检查项                    | 结果                                                                   |
| ------------------------- | ---------------------------------------------------------------------- |
| `workspace_id` 校验       | 所有写入路径均通过 `require_open_pack_snapshot` 间接验证               |
| pack 未打开时操作         | 会返回 `pack.not_open`                                                 |
| 卡片不存在时资源操作      | `require_card` 返回 `card.not_found`                                   |
| 非 field spell 导入场地图 | `require_field_spell` 返回 `resource.field_image_requires_field_spell` |
| confirmation token 失效   | 任何写入后 `invalidate_pack` 使旧 token 过期，测试已覆盖               |
| 删除幂等                  | 文件不存在时 `FsOperation::DeleteFile` 跳过，不报错                    |
| 图片格式安全              | 通过 `image` crate 解码再编码，不会把恶意文件原样写入                  |
| 外部编辑器启动            | 验证路径存在后才 spawn，不会盲目执行                                   |

---

### 五、总结

| 维度         | 评分 | 备注                                      |
| ------------ | ---- | ----------------------------------------- |
| 架构一致性   | 良   | 完全延续 P3 的分层和写入模式              |
| 正确性       | 良   | 核心逻辑正确，测试覆盖充分                |
| 事务完整性   | 中   | 图片导入路径与脚本路径不对称（已知偏差）  |
| 代码重复     | 中   | write_service 三个 asset 辅助方法高度重复 |
| 错误信息质量 | 中   | strings 校验错误丢弃了具体 issue          |
| 测试覆盖     | 优   | 761 行新增，正向/反向/边界/改号迁移全覆盖 |

**建议**：
1. **优先修复**：把图片导入也收进 `apply_asset_write` 的事务计划（即 `assets.rs` 只返回字节不写文件），消除和脚本路径的不对称
2. **可选优化**：抽取 `apply_asset_operation` 消除 3 个辅助方法的重复
3. **可选优化**：`validate_pack_strings_or_err` 的错误信息携带具体 issue 摘要

总体而言，这是一次质量合格的交付，代码结构清晰、测试充分、与既有约定保持一致。主要改进点集中在事务一致性和少量代码重复上。