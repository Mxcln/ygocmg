# YGOCMG 最小实现与功能包规划

日期：2026-04-25
最近更新：2026-04-28

## 目标

这份文档把设计文档里的巨大首版范围，收敛成两个层级：

1. 一个现在就可以持续实现、测试和演进的最小实现
2. 一组之后可以在 Plan 模式下顺序推进的功能包

## 进度更新（2026-04-28）

1. 作者态 Rust 后端最小闭环已继续保持可用
2. `P0 工程启动包` 已完成，当前仓库已经具备可启动的 Tauri + React 最小应用壳
3. `P1 设置与 Workspace 页面` 已部分完成：
   - 前端技术栈已确定并落地：Zustand（全局 UI 状态）+ TanStack Query（服务端数据缓存）+ CSS Modules
   - 前端 App shell 已重构为线稿图布局：自定义标题栏 + 紧凑侧边栏（图标按钮 + pack 列表）+ 右侧 pack 工作区（元数据栏 + Cards/Strings 标签页）
   - 自定义标题栏功能已完整：窗口拖拽、最小化、最大化/还原、关闭均已通过 Tauri v2 capability 权限配置实现
   - Workspace modal（最近工作区列表、按路径打开、新建）已实现
   - Global Settings modal 已实现
   - Tauri capability 权限已添加窗口控制相关项
   - 前端代码已从单体 App.tsx 拆分为 features/ 模块化结构
4. `P2 Pack 列表与 Tab 会话` 已完成：
   - 后端新增 `close_pack` 和 `delete_pack` Tauri command
   - 前端新增 `packApi.ts` 封装 pack CRUD 操作
   - 新增 AddPackModal 组件（Open Pack / Create Pack / Import Pack 三个 tab）
   - 侧边栏 pack 列表已接入后端数据，显示 pack 名称和关闭按钮
   - Metadata bar 展示真实 pack 元数据（名称、作者、版本、语言）
   - Metadata 展开面板显示完整 pack 信息并提供删除操作
   - shellStore 增加 packMetadataMap 和 packOverviews 状态管理
   - Workspace 打开后自动加载 pack overviews
   - 当前 `Delete Pack` 仍使用浏览器原生确认框，尚未统一到应用内确认流
5. 会话恢复与配置精简：
   - 移除 `default_workspace_root` 配置项（GlobalConfig / Settings modal / Create Workspace 路径自动拼接）
   - `WorkspaceMeta` 新增 `open_pack_ids` 字段，open/close/delete pack 时持久化到磁盘
   - 启动时自动恢复上次打开的 workspace（取 registry 中 `last_opened_at` 最新的记录）
   - workspace 打开后自动恢复之前打开的 pack 列表和活跃 pack
6. `P2.5 Pack Metadata Editing` 已完成：
   - 后端 `update_pack_metadata` command 全链路（service → tauri command → 前端 API）
   - 前端 metadata 展开面板改为 overlay drawer，不再挤压主内容区
   - 展开面板内支持只读/编辑模式就地切换，保存后自动刷新所有相关 UI 状态
7. `P3 单卡编辑闭环`、`P3.5 统一确认与 Warning 流`、`P4 PackStrings 与资源管理` 均已完成，作者态 card / strings / resource 主链路已经可用
8. `P6 Job / Event 基础设施` 已完成后端优先版本：
   - 新增 Job DTO、`JobRuntime`、内存态 `JobStore`、`JobContext`
   - 新增 `AppEventBus`、`job:progress` / `job:finished` 事件模型和 Tauri 事件桥
   - `AppState` 增加 `jobs` / `event_bus`，并将运行时共享状态调整为 `Arc<RwLock<...>>`
   - 新增 `get_job_status`、`list_active_jobs` Tauri commands
   - 前端新增 `job` contract 与 `jobApi`
   - 新增 `job_runtime` 集成测试，覆盖成功、失败、active jobs、进度事件
   - 已根据审阅修复 progress 事件发布错误传播、恢复 Debug 能力、补充 store 断言和 TS event payload 类型
9. 下一步建议根据目标选择：
   - 如果优先补生产效率，推进 `P5 批量编辑`
   - 如果优先打通导入导出链路，推进 `P7 标准包只读接入`，随后接 `P8 / P9`

## 当前最小实现

当前仓库已经落地的最小实现，现阶段可以概括为“作者态 M1 核心 + P0 可运行应用壳”：

1. Rust 后端分层骨架
2. 作者态 JSON 真相源协议
3. 程序级 `global_config.json` 与 `workspace_registry.json`
4. `workspace.json`、`metadata.json`、`cards.json`、`strings.json` 的读写
5. 单文件安全写入
6. 多文件最佳努力事务的最小实现
7. `WorkspaceSession` / `PackSession` 运行时会话
8. `workspace` 创建与打开
9. `pack` 创建、打开、删除与摘要刷新
10. 单卡创建、更新、删除、列表派生
11. `code` 唯一性与间距规则的最小实现
12. `code` 变更时脚本/图片资源改名的最小实现
13. 前端 TypeScript 合同骨架
14. 一条最小作者态集成测试
15. `tauri.conf.json`、`build.rs`、入口 `main.rs`
16. Tauri command 注册与 `AppState` 注入
17. 最小 React / Vite 前端入口
18. 基础 `invokeApi`
19. 一页用于验证初始化链路的最小启动页面
20. P6 后端长任务基础设施：Job 状态模型、任务查询、进度事件、Tauri 事件桥、前端 job API 合同

这一层的目标不是“首版完成”，而是先建立稳定内核。

## 明确未做

以下能力仍属于后续包，不在当前最小实现内：

1. 设置、workspace、pack、card 等业务页面与完整交互 UI
2. 批量编辑与批量移动
3. 标准包只读接入与真实索引构建 runner
4. 导入预检与导入执行
5. 多包导出执行
6. 前端任务中心 UI / 任务结果展示
7. 前端 i18n

## 推荐功能包

### P0 工程启动包

状态：
已完成（2026-04-26）

目标：
把当前核心接成真正的 Tauri + 前端可运行工程。

内容：
1. 增加 `tauri.conf.json`、入口 `main.rs`
2. 注册 `presentation/commands`
3. 建立最小 React 应用入口
4. 增加基础 `invokeApi`

验收：
1. 应用可以启动
2. 前端可以调用初始化命令

当前完成情况：
1. 已增加 `tauri.conf.json`、`build.rs`、`main.rs`
2. 已将现有 Rust `presentation/commands` 桥接为可 invoke 的 Tauri commands
3. 已建立最小 React / Vite 应用入口
4. 已增加基础 `invokeApi`
5. 已通过本地人工验证，应用可以成功启动并显示初始化页面

### P1 设置与 Workspace 页面

状态：
部分完成（2026-04-26）

目标：
把程序级配置和工作区管理变成可操作 UI。

内容：
1. 设置页
2. recent workspaces 页面
3. 新建、打开工作区

当前完成情况：
1. 前端技术栈已确定并安装（zustand、@tanstack/react-query）
2. App shell 已重构为线稿图布局（自定义标题栏 + 侧边栏 + pack 工作区）
3. 自定义标题栏已完整实现（拖拽、窗口控制），Tauri v2 capability 权限已配置
4. Workspace modal 已实现（recent workspaces、按路径打开、新建工作区）
5. Global Settings modal 已实现（全局配置编辑与保存）
6. 前端代码已模块化拆分到 features/workspace、features/settings
7. Zustand shellStore 已建立（管理 workspace/pack/modal 全局状态）
8. 共享 API 层和 contract 类型已按 feature 拆分

依赖：
1. P0

### P2 Pack 列表与 Tab 会话

状态：
已完成（2026-04-26）

目标：
把 pack 生命周期和运行时 tab 管起来。

内容：
1. workspace 下 pack 列表
2. 新建 pack、删除 pack
3. 打开/关闭 pack tab

当前完成情况：
1. 后端 `close_pack` 和 `delete_pack` 已暴露为 Tauri command（服务层方法已有，补齐了 IPC 表面）
2. 前端 `packApi.ts` 封装 `listPackOverviews`、`createPack`、`openPack`、`closePack`、`deletePack`
3. AddPackModal 组件已实现：Open Pack tab 展示未打开的 pack 列表，Create Pack tab 创建新 pack 表单，Import Pack tab 标记为未来版本
4. 侧边栏 pack 列表显示真实 pack 名称，hover 时显示关闭按钮
5. 打开/关闭/切换 pack tab 全链路已通
6. Pack metadata bar 显示真实 author/version/languages
7. Metadata 展开面板显示完整字段（描述、语言、时间戳）并提供 Delete Pack 操作
8. shellStore 扩展：packMetadataMap 缓存已打开 pack 的元数据，packOverviews 缓存 workspace 内所有 pack 概览
9. Workspace 打开后自动加载 pack overviews
10. 切换 active pack 时会立即持久化到 workspace，会话恢复语义已从“last opened”收敛到“last viewed”
11. Metadata 摘要栏与展开面板已按当前 UI 调整：collapsed 单行显示 name/author/version/preferred text languages，expanded 将 description 放在最后一整行并做显示截断
12. 侧边栏顶部三个按钮已居中，支持在 `140px - 280px` 范围内拖拽调宽，并把宽度持久化到全局配置
13. 拖拽侧边栏时会临时全局禁选文本，避免误选
14. 程序启动窗口默认改为最小尺寸 `960x640`，并新增窗口普通尺寸 / 最大化状态的全局配置字段与恢复逻辑

依赖：
1. P1

### P2.5 Pack Metadata Editing

状态：
已完成（2026-04-26）

目标：
把 pack metadata 从“只读摘要 + 展开查看”补齐到可编辑闭环。

内容：
1. 实现 `update_pack_metadata` 后端 command 与前端 API
2. 在当前 metadata 展开区或独立 modal 中提供可编辑表单
3. 支持修改 `name`、`author`、`version`、`description`
4. 支持修改 `display_language_order`
5. 支持修改 `default_export_language`
6. 保存后刷新当前打开 pack 的 metadata 和 workspace pack overviews

当前完成情况：
1. 后端 `PackService::update_pack_metadata` 已实现，走 `validate → touch → save_pack_metadata → 更新 session → 刷新 overviews` 流程
2. Tauri command `update_pack_metadata` 已注册
3. 前端 `packApi.updatePackMetadata` 已封装
4. 前端 metadata 展开面板已从 inline 推挤改为 overlay drawer（`position: absolute` + backdrop），不再挤压下方 card list / strings 主内容区
5. 展开面板支持只读/编辑两种模式就地切换：只读态显示 `[Edit] [Delete Pack]`，点击 Edit 切换为表单态显示 `[Save] [Cancel]`
6. 保存成功后自动刷新 `packMetadataMap`、`packOverviews` 和 collapsed meta-bar 摘要
7. shellStore 新增 `updatePackMetadata` action
8. 切换 active pack 时自动关闭 drawer 并重置编辑态

验收：
1. 用户可以直接在 UI 中修改 pack metadata
2. 保存后 metadata bar 与展开区立即刷新
3. 重新打开应用后，修改结果可从磁盘正确恢复

依赖：
1. P2

### P3 单卡编辑闭环

状态：
前后端均已完成（2026-04-26）

目标：
完成首个用户真正可用的编辑闭环。

内容：
1. CardList UI
2. 单卡详情/编辑表单
3. 新建卡片
4. 改号 warning/错误展示

当前完成情况（后端）：
1. 后端已新增 `application/dto/card.rs`，提供 `CardListPageDto`、`CardDetailDto`、`SuggestCodeInput`、`CreateCardInput`、`UpdateCardInput` 等单卡 DTO
2. `list_cards` 已切为分页返回，`get_card` 已新增，`suggest_card_code` 已返回 `suggested_code + warnings`
3. `create_card` / `update_card` 已切为 `WriteResultDto::Ok { data, warnings }`
4. 卡片命令已显式携带 `workspace_id / pack_id / card_id`，并校验 `workspace_id` 与当前会话一致
5. `CardService` 已收敛为读侧服务：`list_cards` / `get_card` / `suggest_code`
6. `PackWriteService` 已承接 `create_card`、`update_card`、`delete_card`
7. `PackSession` 已扩展 `revision`、`source_stamp`、`asset_index`、`card_list_cache`
8. `open_pack` 已构建完整 pack 快照，`set_active_pack` 不会重建 `card_list_cache`
9. 改号时资源 rename、`cards.json`、`metadata.json` 已收进同一个事务计划
10. review 修复已完成：`delete_card` 收口、`suggest_card_code` 的 `workspace_id` 校验、`update_card` 去重、单次写操作时间戳统一、`open_pack` 返回成本收窄
11. `CardListRow` 新增 `subtype_display` 字段，由 `derive_card_list_row` 根据卡片类型拼接：monster 拼接 monster_flags（如 "Effect / Tuner"），spell/trap 取对应 subtype
12. `default_global_config` 中 `custom_code_recommended_min/max/min_gap` 修正为与功能规格一致的 100M-200M / gap 5

当前完成情况（前端）：
1. `CardListPanel` — CSS Grid 表格布局，带表头行，列包含：缩略图、Code、Name、Type（固定宽度 badge）、Subtype（彩色 tag，monster flag 各有独立配色）、ATK、DEF、Lv、资源图标（has_image / has_script）
2. `CardEditDrawer` — 全区域覆盖式 drawer（覆盖 meta-bar），滑入/滑出动画，左栏卡图 + 右栏 Text/Info 双 tab 表单
3. `CardAssetBar` — 350px 宽、400/580 比例的卡图预览占位，分段按钮组样式（Image: Import/Delete, Script: Create/Import/Edit/Delete）
4. `CardInfoForm` — 全字段编辑（code/alias/setcode/ot/category/primary_type/monster_flags/atk/def/race/attribute/level/pendulum/link/spell_subtype/trap_subtype），类型联动显示/隐藏
5. `CardTextForm` — 多语言切换 + name/effect/strings(16) 编辑
6. `cardApi.ts` — 新增 `deleteCard` 方法
7. `card.ts` 合约 — 新增 `DeleteCardInput`、`subtype_display` 字段
8. `App.tsx` — 集成 CardListPanel，drawer 状态提升至 work-area 层级
9. `styles.css` — 约 800 行新增样式（列表、drawer、表单、动画、分段按钮、badge 配色等）
10. Create/Update/Delete 全流程接通，warnings 展示，TanStack Query 列表刷新
11. Delete 按钮改为红色醒目样式 `danger-button`

验收（后端）：
1. Rust 测试 `cargo test --offline` 通过
2. 单卡 create/update/list/get/suggest 主链路可用
3. 改号后资源 rename 与 session 重建正确
4. `workspace_id` 不匹配时返回 `workspace.mismatch`

验收（前端）：
1. Cards tab 显示可搜索/排序/分页的卡片列表
2. 点击卡片行打开编辑 drawer，可编辑全部字段并保存
3. "+ New Card" 创建新卡片，保存后列表自动刷新
4. Delete 按钮可删除卡片
5. drawer 覆盖 meta-bar，不覆盖左侧卡包列表
6. 切换卡包时自动关闭 drawer

依赖：
1. P2

### P3.5 统一确认与 Warning 流

目标：
把当前零散的浏览器原生确认框和未来写操作 warning 收敛成统一的应用内交互流。

内容：
1. 统一 `ConfirmDialog` / `WarningDialog` 组件
2. 禁止在应用内继续使用浏览器原生 `alert` / `confirm` / `prompt`
3. 为 destructive action 提供统一标题、正文、危险按钮、取消按钮样式
4. 支持展示多条 warning / issue，而不是只显示单句文本
5. 首批接入 `Delete Pack`
6. 后续复用到 `Delete Cards`、未保存关闭确认、批量操作 warning 确认

验收：
1. 应用内不再出现浏览器原生确认框
2. `Delete Pack` 使用统一确认弹窗
3. warning 可以显示多条 issue
4. 同一套确认流可复用于 card/strings/import/export

依赖：
1. P2

### P4 PackStrings 与资源管理

目标：
补齐包内主要编辑能力。

内容：
1. `Strings` tab
2. 主卡图、场地图、脚本管理
3. 外部编辑器打开脚本

当前状态：
1. 已完成
2. `Strings` tab 已接入真实查询、编辑、删除与确认流
3. 单卡资源管理已支持主卡图、场地图、脚本的导入 / 删除 / 创建 / 外部打开
4. `PackStrings` 底层模型已升级为 `(kind, key) -> values[language]` 的多语言聚合模型
5. 已补充 `code / setname / counter / victory` 的作者态 warning 与导出预检基础能力

依赖：
1. P3

### P5 批量编辑

目标：
让 pack 内操作从单卡迈向批量生产。

内容：
1. 批量删除
2. 批量 patch
3. 批量移动到其他 pack

依赖：
1. P4

### P6 Job / Event 基础设施

状态：
后端优先版本已完成（2026-04-28）

目标：
为长任务建立统一运行方式。

内容：
1. Job 状态模型
2. 任务查询
3. 进度事件
4. 前端任务反馈区

当前完成情况：
1. 后端新增 `application/dto/job.rs`，提供 `JobKindDto`、`JobStatusDto`、`JobAcceptedDto`、`JobSnapshotDto`、`GetJobStatusInput`
2. `runtime/jobs` 已实现内存态 `JobRuntime` / `JobStore` / `JobContext`
3. `JobRuntime::submit` 支持后台执行测试/未来真实 runner，并把成功、失败、panic 都落入任务状态
4. `JobContext::progress` 支持阶段、百分比和消息更新；状态持久化失败会返回错误，事件发布失败按 best-effort 处理，不会误杀任务
5. `runtime/events` 已定义 `AppEventBus`、`JobProgressEvent`、`JobFinishedEvent`，事件名固定为 `job:progress` / `job:finished`
6. `infrastructure/tauri_event_bus.rs` 已把 runtime 事件桥接到 Tauri `emit`
7. `AppState` 已接入 `jobs` / `event_bus`，并将 `sessions`、`confirmation_cache` 调整为可后台共享的 `Arc<RwLock<...>>`
8. Presentation / Tauri command 已新增 `get_job_status`、`list_active_jobs`
9. 前端已新增 `src/shared/contracts/job.ts` 与 `src/shared/api/jobApi.ts`，并导出 `JobProgressEvent` / `JobFinishedEvent` 类型
10. 新增 `src-tauri/tests/job_runtime.rs`，覆盖成功任务、失败任务、active jobs 和事件记录
11. 代码审阅后已补充：`AppState` / `JobRuntime` Debug、`JobStore` insert/update debug assert、移除误导性的 `AppEvent` serde tag

本轮未做：
1. 完整前端任务中心 UI
2. `cancel_job`
3. Job 历史持久化或容量裁剪
4. 标准包索引、导入、导出的真实 runner
5. `preview_token` cache 与 execute 阶段复核

依赖：
1. P2

后续衔接：
1. `P7` 应首先把 `rebuild_standard_pack_index -> JobAcceptedDto` 接入 `JobRuntime`
2. `P8` 的 `execute_import_pack(preview_token)` 应提交 `import_pack` job，并在 runner 开始时复核 preview 快照
3. `P9` 的 `execute_export_bundle(preview_token)` 应提交 `export_bundle` job，复用当前 `preview_export_bundle` 的 `snapshot_hash` 思路
4. 后续真实 runner 必须避免长时间持有 `sessions.write()`，只在读取快照和最终提交状态时短暂加锁

### P7 标准包只读接入

目标：
接入 YGOPro 标准包作为只读参考源。

内容：
1. 标准包索引缓存
2. 标准卡搜索
3. 标准卡号冲突检查

依赖：
1. P1
2. P6

### P8 导入

目标：
把运行时资源导入成作者态 pack。

内容：
1. `cdb` / `strings.conf` 解析
2. 预检
3. `preview_token`
4. Job 执行

依赖：
1. P4
2. P6
3. P7

### P9 导出

目标：
把多个作者态 pack 导出成运行时资源目录。

内容：
1. 多包冲突预检
2. `cdb` / `strings.conf` 生成
3. 资源写出
4. Job 执行

依赖：
1. P8

### P10 稳定性收尾

目标：
把“能跑”收束成“可交付”。

内容：
1. 故障注入测试
2. schema mismatch 处理
3. 空状态/错误态/加载态
4. 回归清单

依赖：
1. P9

## 建议的 Plan 模式执行顺序

建议按下面顺序逐包推进：

1. P1
2. P2
3. P3
4. P4
5. P5
6. P6（已完成后端基础设施）
7. P7
8. P8
9. P9
10. P10

## 下一步建议

下一次进入 Plan 模式时，建议从 `P5 批量编辑` 或 `P7 标准包只读接入` 开始。

原因：

1. P0、P1、P2、P3、P4 已形成连续可用链路
2. 单卡编辑、资源管理、strings 管理都已具备作者态闭环
3. P6 已经把长任务基础设施落地，后续可以直接服务标准包索引、导入和导出执行
4. 若优先提升编辑效率，推进 P5；若优先打通导入导出链路，推进 P7，然后接 P8 / P9
