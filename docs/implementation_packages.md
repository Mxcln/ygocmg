# YGOCMG 最小实现与功能包规划

日期：2026-04-25
最近更新：2026-04-26

## 目标

这份文档把设计文档里的巨大首版范围，收敛成两个层级：

1. 一个现在就可以持续实现、测试和演进的最小实现
2. 一组之后可以在 Plan 模式下顺序推进的功能包

## 进度更新（2026-04-26）

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
5. 会话恢复与配置精简：
   - 移除 `default_workspace_root` 配置项（GlobalConfig / Settings modal / Create Workspace 路径自动拼接）
   - `WorkspaceMeta` 新增 `open_pack_ids` 字段，open/close/delete pack 时持久化到磁盘
   - 启动时自动恢复上次打开的 workspace（取 registry 中 `last_opened_at` 最新的记录）
   - workspace 打开后自动恢复之前打开的 pack 列表和活跃 pack
6. 下一步建议推进 `P3 单卡编辑闭环`

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

这一层的目标不是“首版完成”，而是先建立稳定内核。

## 明确未做

以下能力仍属于后续包，不在当前最小实现内：

1. 设置、workspace、pack、card 等业务页面与完整交互 UI
3. 标准包只读接入
4. 导入预检与导入执行
5. 多包导出预检与导出执行
6. Job / Event 统一长任务系统
7. `PackStrings` 编辑服务
8. 批量编辑与批量移动
9. 外部编辑器联动
10. 标准包冲突检测
11. 前端 i18n、通知、确认流

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
2. 新建 pack、删除 pack、编辑 metadata
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

依赖：
1. P1

### P3 单卡编辑闭环

目标：
完成首个用户真正可用的编辑闭环。

内容：
1. CardList UI
2. 单卡详情/编辑表单
3. 新建卡片
4. 改号 warning/错误展示

依赖：
1. P2

### P4 PackStrings 与资源管理

目标：
补齐包内主要编辑能力。

内容：
1. `Strings` tab
2. 主卡图、场地图、脚本管理
3. 外部编辑器打开脚本

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

目标：
为长任务建立统一运行方式。

内容：
1. Job 状态模型
2. 任务查询
3. 进度事件
4. 前端任务反馈区

依赖：
1. P2

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
6. P6
7. P7
8. P8
9. P9
10. P10

## 下一步建议

下一次进入 Plan 模式时，建议从 `P3 单卡编辑闭环` 开始。

原因：

1. P0、P2 已完成，P1 设置页和 Workspace 页已可操作
2. Pack 生命周期管理（创建、打开、关闭、删除、切换）已全链路通
3. 接下来最值得补的是 card 列表 + 单卡编辑表单 + 新建卡片
4. 只有先把 card 编辑做成真实闭环，后续 strings、资源管理才有上下文
