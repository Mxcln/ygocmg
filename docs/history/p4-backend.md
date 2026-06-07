# P4 实现记录：PackStrings、卡片资源与冲突预检

## 概要

本轮已经把 P4 的主体能力做完整了，不再只是“后端闭环”。当前交付覆盖了：

- `PackStrings` 的作者态列表、查询、编辑、删除、确认流
- 单卡资源管理：主卡图、场地图、脚本、外部编辑器打开
- `PackStrings` 多语言聚合模型
- `code / setname / counter / victory` 的作者态 warning 与导出期冲突预检基础
- 前后端接线、样式、集成测试与类型校验

从产品能力上看，P4 主要完成的是两条链：

1. `card assets`
2. `Strings List`

其余改动主要是在为这两条链补齐 DTO、命令、模型迁移、命名空间校验和导出预检基础设施。

## 主体成果

### 1. Card Assets

当前单卡资源管理已经从占位状态变成真实可操作能力。

后端已实现：

- 主卡图导入 / 删除
- 场地图导入 / 删除
- 空脚本创建
- 脚本导入 / 删除
- 外部编辑器打开脚本

行为约束：

- 主卡图统一落盘到 `pics/<code>.jpg`
- 主卡图导入后固定拉伸到 `400 x 580`
- 场地图统一落盘到 `pics/field/<code>.jpg`
- 场地图只允许 `spell + field`
- 脚本路径为 `scripts/c<code>.lua`
- 外部编辑器要求配置的是实际可执行文件路径

写入一致性：

- 资源写入统一经由 `PackWriteService`
- 写入后刷新 `metadata.updated_at`
- 重建 `PackSession`
- 刷新 workspace overviews
- 失效当前 pack 的 confirmation cache

资源与 metadata 当前已经统一进入文件事务计划，不再存在“图已写入但 metadata 未提交”的已知偏差。

### 2. Strings List

当前 `Strings` tab 已接成真实作者态能力，而不是占位页。

前端当前支持：

- 选择语言查看 strings
- 按 kind 过滤
- 按 value 搜索
- 分页查看
- 新增 strings
- 编辑已有 value
- 删除 strings
- warning / confirmation 对话框

最近补充的作者态体验修正：

- 自定义包不再允许新增 `system`
- `key` 输入与显示全部改成十六进制
- warning / confirm 文案中的相关 key/base/range 也统一显示为十六进制
- 修复了 key 输入自动跳焦点
- 修复了新建行 value 输入框和确认按钮重叠

## PackStrings 模型重构

这轮不只是做了一个 strings 列表，还把底层模型改成了更适合多语言的结构。

旧模型：

- `entries: Record<language, PackStringEntry[]>`

新模型：

- `entries: PackStringRecord[]`
- `PackStringRecord { kind, key, values: Record<language, string> }`

当前语义是：

- 一个 `(kind, key)` 是主实体
- 不同语言的 value 挂在同一条记录下
- `list_pack_strings(language)` 只是从聚合模型投影出的语言视图

兼容策略：

- 读取旧 schema 时自动迁移
- 写回只写新 schema

额外约束：

- 自定义包作者态禁止新增 `system`
- `setname / counter / victory` 继续允许写入，但会产生推荐区和潜在冲突 warning

## 冲突模型与导出预检基础

为了给后续导出和标准冲突检查铺路，这轮还补了命名空间与预检骨架。

### 1. 作者态 warning

当前已经对以下命名空间提供作者态 warning：

- `code`
- `setname`
- `counter`
- `victory`

原则是：

- 编辑期宽松
- 推荐区与潜在冲突给 warning
- 导出期再做严格 block

其中：

- `setname` 按 `base = key & 0x0fff` 看推荐区和冲突
- `counter` 按 full key 冲突、按 low12 看推荐区
- `victory` 按 full key 看推荐区和冲突

### 2. 标准基线

新增了仓库内置标准基线加载：

- 标准卡号基线
- 标准 `strings.conf` 的 `system / setname / counter / victory` 基线

它用于：

- 作者态 warning
- 导出预检冲突判断

### 3. 导出预检骨架

新增了最小可用的 `preview_export_bundle` 后端能力。

当前会检查：

- 多 pack 之间重复 `code`
- 与标准基线重复 `code`
- `setname base` 冲突
- `counter full key` 冲突
- `victory full key` 冲突
- `PackStrings` 缺少目标导出语言

这部分还没有完整前端接线，但已经是后续 P8 / P9 的可复用基础。

## 技术落点

### 后端新增 / 重点修改

主要模块：

- `src-tauri/src/application/resource/*`
- `src-tauri/src/application/strings/*`
- `src-tauri/src/application/export/*`
- `src-tauri/src/application/pack/write_service.rs`
- `src-tauri/src/domain/strings/model.rs`
- `src-tauri/src/domain/strings/validate.rs`
- `src-tauri/src/domain/namespace/*`
- `src-tauri/src/infrastructure/assets.rs`
- `src-tauri/src/infrastructure/standard_baseline.rs`
- `src-tauri/src/runtime/confirmation_cache.rs`

命令接线：

- `src-tauri/src/presentation/commands/app_commands.rs`
- `src-tauri/src/tauri_commands.rs`
- `src-tauri/src/main.rs`

### 前端新增 / 重点修改

主要模块：

- `src/features/card/CardAssetBar.tsx`
- `src/features/card/CardEditDrawer.tsx`
- `src/features/strings/StringsListPanel.tsx`
- `src/shared/api/resourceApi.ts`
- `src/shared/api/stringsApi.ts`
- `src/shared/contracts/resource.ts`
- `src/shared/contracts/strings.ts`
- `src/shared/utils/format.ts`
- `src/app/App.tsx`
- `src/app/styles.css`

## 验证

本轮已经通过：

- `cargo test --offline`
- `npm.cmd run typecheck`

其中后端集成测试已覆盖：

- `PackStrings` 的新增、过滤、分页、覆盖确认、删除、幂等删除
- 旧 schema 到新 schema 的读兼容
- 主卡图导入与 `400x580` 校验
- 场地图 field spell 限制
- 脚本创建、覆盖导入、删除
- 外部编辑器三类错误
- 改号后主卡图、场地图、脚本迁移

## 当前结论

P4 现在可以认为已经完成，不再是“只有后端完成”。

从用户可见能力看，已经具备：

- 单卡资源管理闭环
- `Strings List` 作者态闭环
- `PackStrings` 多语言主模型
- 基础冲突 warning 与导出预检骨架

如果继续往后推进，优先方向会是：

1. P5 批量编辑
2. P6 Job / Event 基础设施
3. P8 / P9 导入导出完整接线
