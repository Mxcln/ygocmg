我已经完整阅读了 diff、新建文件和文档。下面是评价。

---

## 总体评价

这是一次质量很高的重构。核心架构决策正确，落地干净，测试覆盖充分。具体来说：

### 做得好的地方

**1. 读写分离落地清晰**

`CardService` 收敛为纯读侧（`list_cards` / `get_card` / `suggest_code`），`PackWriteService` 承接写入。职责边界清晰，符合设计文档意图。

**2. snapshot-then-replace 模式**

旧实现是"持有 session 写锁 → 在锁内直接写磁盘"，新实现变成"读快照 → 锁外计算 → 写磁盘 → 构建新 session → 整体替换"。这显著降低了锁持有时间，也消除了"磁盘写失败但内存已修改"的不一致风险。

**3. 改号操作的事务化**

`update_card` 改号时，资源 rename + `cards.json` + `metadata.json` 被收进同一个 `execute_plan` 调用。这比旧代码先 rename 再分别写文件要一致得多。

**4. `build_pack_session` 统一构造**

单一入口构造完整 `PackSession`（含 `asset_index`、`card_list_cache`、`source_stamp`），`open_pack` / `create_card` / `update_card` / `update_pack_metadata` 全部走同一个函数。避免了遗漏字段。

**5. workspace_id 校验**

所有 card 命令都验证 `workspace_id` 与当前 session 一致，并有专门的测试用例覆盖不匹配场景。

**6. 测试扩展**

既有测试适配了新接口形状，新增了 `workspace_id_mismatch` 测试，覆盖了 `list_cards`/`get_card`/`suggest_card_code` 的重新打开场景。

---

### 需要关注的问题

**1. `has_main_image` vs `has_image` 命名不匹配——运行时 bug**

这是最严重的一个。后端 `CardAssetState` 序列化出的字段名是 `has_main_image`：

```5:8:d:\Game\YGODIY\ygocmg\src-tauri\src\domain\resource\model.rs
pub struct CardAssetState {
    pub has_main_image: bool,
    // ...
}
```

但前端 contract 期望的是 `has_image`：

```typescript
export interface CardAssetState {
  has_image: boolean;  // ← 不匹配
  has_script: boolean;
  has_field_image: boolean;
}
```

`CardDetailDto.asset_state` 直接使用了 domain 的 `CardAssetState`，serde 会序列化为 `has_main_image`，前端读到的 `has_image` 将是 `undefined`。

**2. `delete_card` 没有进入 `PackWriteService`**

`create_card` 和 `update_card` 已经迁移到 `PackWriteService`，但 `delete_card` 的逻辑直接写在了 `app_commands.rs` 里（约 20 行内联代码）。这打破了"所有 pack 级写操作走 `PackWriteService`"的模式。虽然你在文档中标记了这是保留项，但这段内联代码已经在做 `save_cards` + `save_pack_metadata` + `build_pack_session` + `replace_open_pack_session` ——和 `PackWriteService` 里的模式完全一样，应该收进去。

**3. `update_card` 代码重复**

`write_service.rs` 的 `update_card` 中，改号和非改号两个分支几乎完全相同（都是 `execute_plan` → `build_pack_session` → `replace_open_pack_session` → `refresh`），唯一区别是改号分支多了 rename 操作。可以合并为一条路径：

```rust
let mut operations = Vec::new();
if old_code != new_code {
    for (from, to) in planned_asset_renames(...) {
        operations.push(FsOperation::Rename { from, to });
    }
}
operations.push(FsOperation::WriteFile { /* cards */ });
operations.push(FsOperation::WriteFile { /* metadata */ });
execute_plan(operations)?;
// ... build_pack_session + replace + refresh
```

这样能删掉约 30 行重复代码。

**4. `suggest_code` 没有校验 `workspace_id`**

`SuggestCodeInput` 包含 `workspace_id` 字段，但 `CardService::suggest_code` 完全没用它——直接用 `pack_id` 调了 `build_code_context`。要么校验一下，要么从 input 里去掉。

**5. `now_utc()` 多次调用**

`PackWriteService::create_card` 中调了两次 `now_utc()`：一次给 `create_card_entity`，一次给 `touch_pack_metadata`。虽然时间差极小，但语义上同一次操作应该共享同一个时间戳。建议在方法开头 `let now = now_utc();` 然后共用。`update_card` 同样有这个问题（甚至调了三次）。

**6. DTO 没有完全隔离 domain 枚举**

接口设计文档要求 DTO 中的枚举用 `String` 表示（如 `primary_type: String`），但当前 `EditableCardDto` / `CardListRowDto` 直接用了 domain 的 `PrimaryType`、`MonsterFlag`、`Race` 等。这在功能上没问题（serde 会序列化为 `"monster"` 等 snake_case 字符串），但意味着 domain 枚举新增 variant 会直接影响 IPC 契约。这是一个有意识的取舍还是待补的工作？

**7. `open_pack` 返回完整 `PackSession` 的克隆**

`open_pack` 成功后返回 `PackSession` 的 clone，包含 `cards: Vec<CardEntity>`、`card_list_cache` 等全部数据。这个返回值在 `app_commands::open_pack` 中只取了 `.metadata`，所以整个 cards + cache 被克隆后立即丢弃。虽然你在文档中标记了 `open_pack` 返回值未升级为 `PackSnapshotDto`，但可以先把方法签名改为返回 `PackMetadata` 以避免不必要的大对象克隆。

---

### 小建议（非问题）

- `card_list_cache` 每次写操作后整包重建，当前阶段完全合理。卡片数量多了以后可以考虑增量更新，但现在不需要。
- `source_stamp` 用 `modified_time:file_size` 组合是好的最小实现，够用。
- 前端 `cardApi.ts` 的 `invokeApi<WriteResult<CardDetail>>` 嵌套泛型很清晰。

---

### 总结

架构方向正确，核心写路径的安全性提升明显。**最需要立即修的是 `has_main_image` / `has_image` 的命名不匹配**，其余问题可以在 P3 前端接线阶段顺手解决。