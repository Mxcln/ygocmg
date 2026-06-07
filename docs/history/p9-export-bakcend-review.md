现在我已经完整阅读了所有修改文件和相关上下文。下面给出审阅结果。

---

## P9 Export Backend 代码审阅

### 总体评价

本轮实现质量**整体优秀**，代码风格与项目已有约定一致，模块边界清晰，测试覆盖充分。下面按"通过"、"问题"、"建议"三类逐项列出。

---

### 通过项（设计合理、实现正确）

1. **preview / execute 两阶段模型**：完全复用 P8 导入的 token 机制，token 单次消费、过期检查、workspace 切换清理，流程一致且完整。

2. **snapshot_hash 比对**：用 `pack_id:revision:source_stamp` 拼接散列，execute 阶段重新 `prepare_export` 后比对，能有效检测 preview 后的任何 pack 写入。

3. **setname namespace 修正**：`setname_keys`（full key）vs `setname_bases`（low12）的分级设计合理——full key 冲突为 error，base 重叠降级为 warning。`#[serde(default)]` 保证旧索引兼容，`load_index` 的回填逻辑也到位。

4. **CDB writer encode 函数群**：`encode_card`、`encode_level`、`encode_link_markers`、`encode_monster_flag`、`encode_race`、`encode_attribute` 与对应 parse 函数完全对称，常量复用同一组，没有遗漏 flag。

5. **strings.conf writer**：按 kind 分组、kind 内排序、system 用十进制 / 其他用十六进制，与 YGOPro 运行时兼容。

6. **测试**：5 个集成测试覆盖了主链路成功、语言缺失阻断、setname full key 冲突阻断、setname base 重叠 warning 可继续执行、workspace 切换 token 失效、preview 过时 stale 检测——覆盖面很好。

---

### 问题（建议修复）

#### P9-1: `copy_if_exists` 中 `source` 变量名遮蔽

```590:607:src-tauri/src/application/export/service.rs
fn copy_if_exists(source: &Path, target: &Path) -> AppResult<()> {
    if !source.exists() {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|source| {
            AppError::from_io("export.create_dir_failed", source)
                .with_detail("path", parent.display().to_string())
        })?;
    }
    fs::copy(source, target)
        .map(|_| ())
        .map_err(|source_error| {
            AppError::from_io("export.copy_asset_failed", source_error)
                .with_detail("source", source.display().to_string())
                .with_detail("target", target.display().to_string())
        })
}
```

第 595 行 `create_dir_all` 闭包的 `|source|` 遮蔽了外层函数参数 `source: &Path`。虽然闭包内只用 IO error 所以功能没有 bug，但这是潜在的可读性/维护风险，与下方第 602 行刻意取名 `source_error` 的处理不一致。建议将第 595 行闭包参数也改为 `|io_err|` 或 `|source_error|`。

#### P9-2: `write_export_bundle` 中 `output_dir_not_empty` 检查与 `collect_export_issues` 重复

`collect_export_issues`（预检阶段）已经检查了 `output_path.exists() && !is_empty_dir`，如果检查到会产生 blocking error 并阻止 execute。但 `write_export_bundle`（写出阶段）又做了一次同样的检查。两处语义不同——预检是 issue 收集，写出是 hard fail。这本身不是 bug，但存在两段几乎一样的代码。建议抽取为一个公共函数或至少在代码中注明这是故意的防御性双重检查。

#### P9-3: `encode_card` 对 Spell/Trap 非 monster 字段未显式归零

```487:530:src-tauri/src/infrastructure/ygopro_cdb/mod.rs
fn encode_card(card: &CardEntity) -> AppResult<EncodedCardData> {
    let mut raw_type = match card.primary_type { ... };
    let mut atk = 0;
    let mut def = 0;
    let mut raw_level = 0;
    let mut raw_race = 0;
    let mut raw_attribute = 0;
    match card.primary_type {
        PrimaryType::Monster => { ... }
        PrimaryType::Spell => { raw_type |= encode_spell_subtype(...); }
        PrimaryType::Trap => { raw_type |= encode_trap_subtype(...); }
    }
    // ...
```

这里 `atk`/`def`/`raw_level` 等默认已经是 0，所以对 Spell/Trap 来说没有 bug。但 `CardEntity` 的 `atk`/`def` 等字段是 `Option<i32>`——如果用户在前端给一张 Spell 卡错误地设置了 `atk = Some(1000)`，当前实现会把 atk 写成 0 而不是 1000，这是正确行为。只是建议确认这个"忽略非法字段"的设计意图，如果后续 domain model 的 `validate_card_structure` 不阻止这种情况，可能需要在导出端做一个显式的 warning。

#### P9-4: 测试中 `load_cards_from_cdb` 返回的 texts key 是 `"default"` 而不是导出语言

```115:125:src-tauri/tests/export_bundle_flow.rs
    let exported_cards = ygocmg_core::infrastructure::ygopro_cdb::load_cards_from_cdb(
        &bundle_dir.join("bundle.cdb"),
    )
    .unwrap();
    assert_eq!(exported_cards.len(), 2);
    let monster = exported_cards
        .iter()
        .find(|record| record.card.code == 100_000_100)
        .unwrap();
    assert_eq!(monster.card.texts["default"].name, "Export Monster");
```

这里之所以能过测试，是因为 `load_cards_from_cdb` 的 `decode_card_row` 总是把 CDB 中读到的文本放进 `"default"` key（第 294 行）。这在功能上没有问题——CDB 本身不存储语言 key，reader 固定用 `"default"`。但测试用 `"default"` 做断言可能让读者误解为"导出时文本写成了 default 语言"，实际上导出确实正确地写了 `zh-CN` 对应的文本。可以考虑加个注释说明这是 reader 的固定行为。

---

### 建议（可选改进，不影响正确性）

#### S1: Job 内 `prepare_export` 再次读取全部 pack snapshot + 全部卡 + 全部 strings

`execute_export_bundle` 在 Job 闭包内重新调用 `prepare_export`，会再次从 session 读取所有 pack snapshot（包括 cards、strings、asset_index）并重新跑一遍完整预检。这是为了 stale 检测和重新验证 error，设计上合理。但如果 pack 很大（数百张卡、大量 strings），这实质上做了两次完整预检 + 一次写出。

当前没有实际性能问题（桌面应用、pack 规模有限），但如果后续 pack 规模增大，可以考虑只做 `snapshot_hash` 比对 + error 重检，而直接复用 `prepare_export` 的 packs 数据用于写出，避免第二次全量快照收集。

#### S2: `write_export_bundle` 非事务性写出

文档中已指出了这一点：导出 Job 直接 `fs::copy` 资源文件，中途失败会留下部分文件。当前阶段可以接受（导出不修改源数据），但未来可以考虑先写到临时目录再 `rename`。

#### S3: `strings.conf` 写出时跳过无目标语言的 record 而没有 warning

```94:101:src-tauri/src/infrastructure/strings_conf/mod.rs
        for record in kind_records {
            if let Some(value) = record.values.get(export_language) {
                contents.push_str(&format_record_key(&kind, record.key));
                contents.push(' ');
                contents.push_str(value.trim());
                contents.push('\n');
            }
        }
```

如果某条 string record 没有目标语言的 value，`write_records` 会静默跳过。预检阶段已经会对缺目标语言的 record 产生 blocking error，所以实际不会走到这个分支。但作为防御，写出函数不报 warning 可能在未来维护中隐藏问题。

#### S4: `ExportPreviewDto` 没有 `string_count` 字段

preview 返回了 `card_count`、`main_image_count`、`field_image_count`、`script_count`，但没有 `string_count`。前端 modal 可能想展示导出多少条 strings，建议添加。

#### S5: 考虑 `counter_low12` 未使用的清理

diff 中移除了 `counter_low12` 的 use（之前在 collect_export_issues 中有 `let _low12 = counter_low12(record.key)` 的无用绑定），这很好。但 `counter_low12` 函数本身仍然在 `domain/namespace/model.rs` 中定义。如果全局没有其他调用者，可以考虑加 `#[allow(dead_code)]` 或移除。

---

### 总结

| 类别 | 数量 | 严重度                 |
| ---- | ---- | ---------------------- |
| 通过 | 6 项 | -                      |
| 问题 | 4 项 | 均为低风险，无功能 bug |
| 建议 | 5 项 | 可选改进               |

**结论**：代码质量良好，导出后端的 preview/execute 两阶段协议、CDB/strings.conf 双向编解码、namespace 冲突检测都实现得扎实且测试充分。P9-1（变量名遮蔽）和 P9-2（重复检查）建议修复以保持一致性，其余为可选改进。整体可以合入。