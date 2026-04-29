# P9 Export Backend 实现记录

日期：2026-04-29

## 范围

本轮只实现 P9 后端导出闭环，不接前端 modal / 按钮 / Job UI。

目标能力：

1. `preview_export_bundle` 生成可执行的预检 token
2. `execute_export_bundle(preview_token)` 提交 `ExportBundle` Job
3. Job 复核预检快照后写出 YGOPro 风格运行态目录
4. 导出产物包含 `.cdb`、`strings.conf`、`pics/`、`pics/field/`、`script/`

## 主要改动

### 1. Export preview token

`runtime/preview_token_cache.rs` 新增 `ExportPreviewEntry` 与 export token map：

- 保存 `preview_token`
- 保存 `workspace_id`
- 保存 `pack_ids`
- 保存 `snapshot_hash`
- 保存 `expires_at`
- 保存 `input_snapshot`

`PreviewTokenCache::clear()` 与 `invalidate_workspace()` 已同时清理 import/export tokens，workspace 切换后旧 export token 会失效。

### 2. Execute export command

新增后端入口：

- `ExportService::execute_export_bundle`
- `app_commands::execute_export_bundle`
- `tauri_commands::execute_export_bundle`
- `main.rs` command 注册

执行规则：

1. 只接收 `preview_token`
2. token 会被 consume，重复执行返回 `export.preview_token_invalid`
3. token 过期返回 `export.preview_token_expired`
4. Job kind 为 `JobKindDto::ExportBundle`
5. Job 内重新 `prepare_export`
6. 若预检仍有 error，Job 失败为 `export.preview_has_errors`
7. 若当前 `snapshot_hash` 与 preview 时不一致，Job 失败为 `export.preview_stale`

### 3. 导出预检规则

`application/export/service.rs` 中的预检规则扩展为：

- `pack_ids` 不能为空
- `pack_ids` 不能包含重复项；重复选择会直接返回 `export.pack_ids_duplicate`
- `output_name` 不能为空
- `output_name` 必须是单个安全文件名片段；路径分隔符、`.`、`..`、绝对路径、Windows 非法文件名字符与保留设备名会返回 `export.output_name_invalid`
- 输出目录 `<output_dir>/<output_name>/` 若已存在且非空，blocking error
- 本轮只支持 custom pack；非 custom pack blocking error
- 卡片结构错误会进入导出 issues
- card text 缺目标语言为 blocking error
- pack string 缺目标语言为 blocking error
- selected packs 间重复 code 为 blocking error
- 与标准包 code 重复为 blocking error
- code 落在标准保留范围为 warning
- `counter` / `victory` 按 full key 冲突，blocking error
- `setname` 按 full key 冲突，blocking error
- `setname` 仅 low12/base 重叠时降级为 warning

### 4. setname namespace 修正

之前 namespace baseline 只记录 `setname_bases`，不够区分父系列和子字段。

本轮在 `domain/namespace/model.rs` 中新增：

- `StandardStringNamespaceBaseline::setname_keys`
- `PackStringNamespaceIndex::setname_keys`

规则变更：

- full key 相同：真实导出冲突，error
- `key & 0x0fff` 相同但 full key 不同：可能是合法父/子字段关系，warning

兼容处理：

- `standard_pack::load_index` 在读取旧 index 时，如果 baseline 缺 `setname_keys`，会从完整 strings records 重新计算 baseline
- `setname_keys` 字段使用 serde default，旧缓存不会反序列化失败

### 5. CDB writer

`infrastructure/ygopro_cdb/mod.rs` 新增 `write_cards_to_cdb`。

写出内容：

- 创建 `datas` 与 `texts`
- 将作者态 `CardEntity` 编码回 YGOPro CDB raw 字段
- `texts` 只写目标导出语言
- `str1..str16` 不足 16 项时补空字符串
- `ot` 映射：`Ocg -> 1`、`Tcg -> 2`、`Custom -> 3`
- Link 怪 `def` 写 link marker bitmask
- Pendulum 怪 `level` 写 scale / level packed 值

### 6. strings.conf writer

`infrastructure/strings_conf/mod.rs` 新增 `write_records`。

写出规则：

- 只写目标导出语言 value
- 按 kind 分组输出：`system`、`victory`、`counter`、`setname`
- `system` key 按十进制输出
- 其他 key 按十六进制输出

### 7. 运行态资源写出

Job 写出目录：

```text
<output_dir>/<output_name>/
  <output_name>.cdb
  strings.conf
  pics/<code>.jpg
  pics/field/<code>.jpg
  script/c<code>.lua
```

资源采用复制，不转换、不改写作者态文件。

缺失资源不会阻断导出；当前预检统计会显示已有资源数量。

## 新增测试

新增 `src-tauri/tests/export_bundle_flow.rs`，覆盖：

1. 两个已打开 pack 成功导出
2. `.cdb` 写出并可用现有 CDB reader 读取
3. `strings.conf` 写出目标语言文本
4. main image / field image / script 复制到运行态路径
5. card text 缺目标语言阻断
6. setname full key 重复阻断
7. setname low12/base 重叠只 warning 且不阻断 execute
8. workspace 切换后 export token 失效
9. preview 后修改 pack，execute Job 返回 `export.preview_stale`
10. unsafe `output_name` 被拒绝，防止路径逃逸
11. 重复 `pack_ids` 被拒绝，避免重复导出同一 pack
12. preview 后输出目录变为非空时，execute Job 失败且不会覆盖已有文件

全量验证：

```text
cargo test --offline
```

结果：全部通过。

## 审阅后修复

根据 `docs/review/p9-export-bakcend-review.md` 与二次审阅，本轮补充了以下修复：

1. `output_name` 从“非空”升级为“安全文件名片段”校验，避免 `../bundle`、`nested/bundle`、`bundle\nested`、`CON` 等名称造成路径逃逸或 Windows 文件名问题
2. `prepare_export` 增加重复 `pack_ids` hard error，错误码为 `export.pack_ids_duplicate`
3. preview issue 收集与 execute 写出前 hard fail 复用同一个输出目录非空判断；execute 阶段仍保留二次检查，用于处理 preview 后目录被外部占用的情况
4. `copy_if_exists` 内部 I/O error 参数改名，避免遮蔽外层 `source` 路径参数
5. 补充 `export_bundle_flow` 回归测试覆盖 unsafe output name、重复 pack 选择、preview 后输出目录被占用

仍保留的取舍：

1. export Job 直接写出 `.cdb`、`strings.conf` 并复制资源；中途失败仍可能留下部分导出产物，后续 P10 可考虑临时目录 + rename
2. `snapshot_hash` 当前覆盖 pack session revision/source stamp；应用内资源操作会刷新 revision，但外部直接修改图片或脚本文件内容不纳入 hash
3. `counter_low12` 没有移除，它仍用于作者态 namespace 校验

## 前端接入

本轮后续补齐了 Export modal 前端入口：

1. 侧边栏 `Export Expansions` 按钮启用，并打开 `ExportModal`
2. Step 1 支持选择已打开 packs、填写导出语言、选择输出目录、填写输出名
3. Step 2 调用 `preview_export_bundle`，展示 packs/cards/resources/errors/warnings 统计和 issue 列表
4. Step 3 调用 `execute_export_bundle`，轮询 `get_job_status` 并展示成功/失败状态
5. 新增 `shared/contracts/export.ts`、`shared/api/exportApi.ts`，并将通用 `PreviewResult<T>` 上移到 common contract

## Cursor 审阅重点

建议重点看以下风险点：

1. CDB 反向编码是否完全符合 YGOPro 语义，尤其 Pendulum level packing、Link marker bitmask、monster flags 组合
2. `strings.conf` 写出格式是否符合目标运行时兼容性，特别是 inline / grouped 格式选择
3. 输出目录策略是否符合预期：当前不会覆盖非空目录
4. export Job 直接复制资源文件，没有使用多文件事务；若中途失败，输出目录可能留下部分文件
5. `setname_keys` 对旧 standard index 的回填逻辑是否足够稳妥
6. 本轮只支持已打开 custom packs；未打开 pack 与前端选择流程后续再接

## 未做

1. 导出语言 fallback
2. 覆盖已有输出目录
3. 导出资源缺失 warning
4. Job result payload / 打开输出目录按钮
5. 临时目录 + rename 的原子导出
