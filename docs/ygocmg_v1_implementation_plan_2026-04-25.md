# YGOCMG 首版实现文档 v1

日期：2026-04-25  
状态：Draft  
适用范围：YGOCMG 首版（v1）

关联文档：
- [项目粗略设计方案](./ygocmg.md)
- [YGOCMG 首版功能规范 v1](./ygocmg_v1_functional_spec_2026-04-25.md)
- [YGOCMG 架构与模块职责分析报告](./ygocmg_architecture_module_report_2026-04-25.md)
- [卡片数据模型语义化重构方案 v2](./card_data_model_refactor_v2_2026-04-23.md)

## 1. 文档目标

本文档用于把现有设计文档收敛为一份可执行的实现方案，重点回答以下问题：

1. 首版范围到底冻结到什么程度
2. 现有架构方案如何落成实际代码与目录
3. 后端、前端、持久化、运行时状态分别先做什么
4. 哪些任务在实现顺序上是关键路径
5. 如何把首版拆成可执行、可验收的工作包
6. 哪些能力属于首版完成标准，哪些必须延期

本文档不替代功能规范和架构报告：

1. 功能规范定义“做什么”
2. 架构报告定义“如何分层与划边界”
3. 本文档定义“如何按阶段实现并完成交付”

## 当前代码进度备注（2026-04-26）

截至 2026-04-26，仓库中的实际代码进度可概括为：

1. 作者态 Rust 后端最小闭环已落地，包括 `config/workspace/pack/card` 的核心读写、运行时 session、JSON 存储与安全写入最小实现
2. `code` 变更触发资源改名的最小流程已存在，并有最小作者态集成测试覆盖
3. `P0 工程启动包` 已完成，当前仓库已经具备可启动的 Tauri + React 最小应用壳
4. P1 设置与 Workspace 页面 部分完成：前端技术栈已确定（Zustand + TanStack Query + CSS Modules）；App shell 已重构为线稿图布局（自定义标题栏 + 紧凑侧边栏 + pack 工作区）；Tauri v2 窗口控制权限已配置；Workspace modal 和 Settings modal 已实现；代码已模块化拆分到 features/ 目录
5. 后续实现重点应从“补 pack 生命周期管理 + pack tab 会话”开始（P2）

## 2. 实现基线

YGOCMG 首版的实现基线如下：

1. 产品定位是“作者态自定义卡包管理与编辑工具”，不是通用 YGOPro 启动器，也不是完整创作生态
2. 技术形态采用 `Tauri + Rust + React/TypeScript`
3. 首版架构采用“模块化单体”
4. Rust 后端是唯一业务真相源
5. 作者态数据以 `workspace + pack + json + assets` 持久化
6. CDB、`strings.conf`、`pics/`、`script/` 只作为导入导出边界
7. 前端只负责交互、界面、草稿态和事件消费，不持有最终业务规则

首版成功标准不是“功能很多”，而是以下最小闭环稳定成立：

1. 能配置 YGOPro 路径并接入标准包只读索引
2. 能创建和打开工作区
3. 能创建、编辑、删除作者态 pack
4. 能创建、编辑、删除卡片与 pack strings
5. 能管理主卡图、场地图片、脚本
6. 能把一个或多个 pack 导出为 YGOPro 可用运行时目录
7. 能从现有运行时资源导入为作者态 pack
8. 文件写入在异常情况下尽量可恢复

## 3. 首版范围冻结

### 3.1 首版必须交付

#### 3.1.1 全局配置

1. 程序 UI 语言设置
2. `ygopro_path` 设置与校验
3. 外部文本编辑器路径设置
4. 默认工作区根目录设置
5. 自定义卡推荐编号范围设置
6. 自定义卡最小编号间距阈值设置

#### 3.1.2 工作区能力

1. 程序级 recent workspaces 注册表
2. 新建工作区
3. 打开已有工作区
4. 切换当前工作区
5. recent workspaces 浏览与重开

#### 3.1.3 标准包能力

1. 从 `ygopro_path` 只读接入标准包
2. 标准包摘要索引
3. 标准卡搜索
4. 标准卡号存在性查询
5. 标准字符串只读参考

#### 3.1.4 作者态 Pack 能力

1. 新建空白 pack
2. 编辑 pack 元数据
3. 删除 pack
4. 从运行时资源导入一个 pack
5. 在 pack 页面通过 `CardList` 和 `Strings` 两个 tab 管理内容

#### 3.1.5 Card 能力

1. 单卡新建、查看、编辑、删除
2. 语义化卡片模型持久化
3. 列表搜索、排序、分页或分段加载
4. 基础批量操作
5. 自动建议编号
6. 编号唯一性和间距校验
7. `code` 修改时联动资源迁移

#### 3.1.6 文本与资源能力

1. 多语言 `CardTexts` 编辑
2. 多语言 `PackStrings` 编辑
3. 主卡图导入、替换、删除、预览
4. 场地图片导入、替换、删除、预览
5. 单卡脚本新建、导入、删除、外部打开

#### 3.1.7 导入导出能力

1. 导入预检
2. 导入执行
3. 多 pack 导出预检
4. 多 pack 导出执行
5. 导入导出都采用 `preview_token` 两阶段流程
6. 导入导出都进入 Job 系统

#### 3.1.8 稳定性能力

1. 单文件安全写入
2. 多文件最佳努力提交与回退
3. 统一错误码与 warning 码
4. warning 确认流
5. 关键事件广播

### 3.2 首版冻结边界

以下边界在首版实现中必须冻结，不再反复摇摆：

1. `CardId` 与 `code` 分离，`CardId` 作为作者态持久化身份标识写入 `cards.json`
2. recent workspaces 为程序级注册表，不属于任何 `workspace`
3. 导入和导出都采用“先预检、后执行”两阶段流程
4. 一般写操作的 warning 确认采用 `confirmation_token`，不采用 `force=true`
5. 作者态目录固定使用 `scripts/`，运行态导入导出目录固定使用 `script/`
6. 标准包只读，不进入作者态编辑模型
7. 首版作者态持久化使用 JSON 文件，不引入数据库
8. 外部编辑器只负责打开脚本，不监听回写
9. 后端负责搜索、排序、校验、规范化和冲突检测
10. 前端不复制整个后端 session 为全局 store
11. Job 系统只覆盖标准包索引重建、导入、导出三类长任务

### 3.3 首版明确不做

1. AI 脚本生成
2. AI 卡图生成或编辑
3. 内置 Lua IDE
4. `package/` 共享脚本体系
5. 把多个作者态 pack 合并成新的作者态 pack
6. 更深层的 `setcode / category / alias` 语义化编辑器
7. 批量编辑多语言文本
8. 批量编辑 `pendulum`、`link.markers`
9. 插件系统
10. 数据库替代作者态 JSON
11. 文件监听驱动的脚本热刷新
12. 复杂的前端事件总线或重型全局状态框架

## 4. 实现原则

### 4.1 真相源原则

首版所有实现都要围绕“真相源”和“派生模型”分离来做。

真相源：

1. `global_config.json`
2. `workspace_registry.json`
3. `workspace.json`
4. `metadata.json`
5. `cards.json`
6. `strings.json`
7. 资源文件

派生模型：

1. `CardListRow`
2. `PackOverview`
3. `PackSnapshot`
4. 标准包索引
5. 导入预检结果
6. 导出预检结果

实现要求：

1. 派生模型不直接落盘
2. 前端只消费派生模型和 DTO
3. 后端 runtime session 是从真相源读取出的工作区内存快照，不是第二真相源
4. 写操作只更新真相源，再重建受影响的运行时快照和派生缓存
5. 删除任意派生缓存后，系统最多变慢，不能变错

### 4.2 后端唯一业务规则原则

以下逻辑只能在 Rust 后端实现一次：

1. 卡片结构校验
2. 卡片保存前规范化
3. 编号冲突检查
4. 编号间距 warning
5. 资源路径推导
6. `scripts/ <-> script/` 映射
7. `pack strings` 唯一性检查
8. 导入导出预检
9. 安全写入计划与最佳努力回退

补充类型边界原则：

1. `domain` 负责内部真相模型，不直接作为前端接口契约暴露
2. `application dto` 负责用例输入输出，是对外边界的稳定合同
3. 真相源仓储可以直接读写 `domain model`，因为它们承载作者态持久化
4. 标准包、导入导出、事件、任务等跨边界场景优先定义专门 `port model`
5. 不为了“层层不同”而制造无意义重复模型，但必须避免把内部领域实体直接公开成 API 结构

### 4.3 `pack` 作为写一致性边界

首版应把自定义 `pack` 明确为作者态写入的一致性边界。

含义：

1. `metadata.json`、`cards.json`、`strings.json` 与 pack 内资源文件共同构成一个 `pack` 的作者态真相集合
2. 单 `pack` 内卡片、字符串、资源、metadata 的写入都由同一条 pack 级写流程统一编排
3. `pack` 是事务 owner，而不是要求每次都重写整个 `pack`
4. 实际落盘仍按最小必要文件集执行，只提交受影响的文件

实现要求：

1. 单卡字段修改通常只改 `cards.json` 与 `metadata.json`
2. `pack strings` 修改通常只改 `strings.json` 与 `metadata.json`
3. 资源导入或删除通常只改资源文件与 `metadata.json`
4. `code` 变更时允许同时修改 `cards.json`、相关资源文件与 `metadata.json`
5. 写成功后统一重建受影响 `pack` 的运行时快照，而不是由各功能模块分别维护缓存同步

### 4.4 先跑通最小闭环，再扩边界

首版开发顺序不应从最复杂的导入导出开始，而应先完成以下最小闭环：

1. 全局配置
2. 工作区
3. 空白 pack
4. 单卡创建与保存
5. 重启后重新打开并读取
6. `code` 修改触发资源迁移

只有这个闭环稳定后，才继续做：

1. 批量编辑
2. 标准包索引
3. 导入
4. 导出
5. Job 进度体验

### 4.5 首版优先稳定而非过度抽象

首版实现允许适度保守，优先保证：

1. 模型一致性
2. 文件安全
3. 导入导出正确性
4. 调试可读性
5. 测试可覆盖性

不优先追求：

1. 过早的通用插件接口
2. 复杂的元编程
3. 前后端共享类型自动生成
4. 超前的性能优化工程

## 5. 总体实现方案

### 5.1 应用结构

```text
Tauri Desktop App
├─ Frontend (React/TS)
│  ├─ 页面与组件
│  ├─ 表单草稿态
│  ├─ 查询缓存
│  └─ 事件消费
└─ Backend (Rust)
   ├─ Domain
   ├─ Application
   ├─ Infrastructure
   ├─ Runtime
   └─ Presentation
```

### 5.2 推荐仓库目录

```text
ygocmg/
  docs/
  src/
    app/
    features/
    shared/
  src-tauri/
    src/
      bootstrap/
      domain/
      application/
      infrastructure/
      runtime/
      presentation/
      tests/
```

### 5.3 责任边界

前端负责：

1. 页面结构与导航
2. 表单编辑与局部草稿状态
3. 查询结果展示
4. 虚拟滚动、弹窗、通知、进度条
5. 程序 UI 多语言

后端负责：

1. 作者态模型
2. 用例编排
3. JSON 与资源文件读写
4. 标准包读取
5. 导入导出协议
6. 事务、回滚、Job、事件
7. 错误与 warning 语义

## 6. 持久化与磁盘实现

### 6.1 应用数据目录

应用数据目录保存程序级状态，不属于任何 `workspace`。

至少包含：

```text
<app-data>/
  global_config.json
  workspace_registry.json
  standard_pack/
    index_cache.json
    index_meta.json
```

说明：

1. `global_config.json` 保存全局配置
2. `workspace_registry.json` 保存程序已知工作区
3. 标准包只读索引缓存放在应用数据目录
4. Job 运行态默认保存在内存，不要求持久化到磁盘

### 6.2 Workspace 磁盘布局

```text
<workspace>/
  workspace.json
  packs/
    <storage-name>/
      metadata.json
      cards.json
      strings.json
      pics/
        field/
      scripts/
```

实现要求：

1. `workspace.json`、`metadata.json`、`cards.json`、`strings.json` 都有 `schema_version`
2. 首版所有作者态 JSON 的 `schema_version` 固定为 `1`
3. pack 目录名使用文件系统安全的可读 `storage-name`；真实身份来自 `metadata.json` 中的 `pack-id`
4. 资源路径只按 `code` 生成，不把路径写回 `CardEntity`

### 6.3 JSON 文件模型

首版作者态 JSON 文件建议统一使用包裹结构：

1. 顶层包含 `schema_version`
2. 数据主体放在固定字段中
3. 由单一 `json_store` 模块负责读写

建议结构：

```ts
interface WorkspaceFile {
  schema_version: 1;
  data: WorkspaceMeta;
}

interface PackMetadataFile {
  schema_version: 1;
  data: PackMetadata;
}

interface CardsFile {
  schema_version: 1;
  cards: CardEntity[];
}

interface PackStringsFile {
  schema_version: 2;
  entries: PackStringRecord[];
}
```

其中：

```ts
interface PackStringRecord {
  kind: PackStringKind;
  key: number;
  values: Record<LanguageCode, string>;
}
```

### 6.4 资源命名规则

作者态 pack 内部资源规则固定为：

1. 主卡图：`pics/<code>.jpg`
2. 场地图片：`pics/field/<code>.jpg`
3. 脚本：`scripts/c<code>.lua`

运行态导出规则固定为：

1. 主卡图：`pics/<code>.jpg`
2. 场地图片：`pics/field/<code>.jpg`
3. 脚本：`script/c<code>.lua`

实现要求：

1. `scripts/ <-> script/` 映射只能出现在 adapter 层
2. `code` 改变时所有相关资源必须一起迁移
3. 卡片移动 pack 时资源按目标 pack 目录整体迁移
4. 卡片资源存在性扫描结果只进入运行时 `CardAssetState`

### 6.5 图片资源处理策略

主卡图的产品要求是导入源图尺寸不固定，但程序落盘时统一输出为 `.jpg` 且目标尺寸为 `400 x 580`。首版实现建议采用保守策略：

1. 程序内部落盘统一保存为 `.jpg`
2. 主卡图导入时由程序自动缩放为 `400 x 580`
3. 不提供用户可交互的裁剪、缩放、修图 UI
4. 若导入源文件不可读或转换失败，应在导入阶段阻断并明确提示
5. 场地图片首版只校验文件可读和扩展能力，不额外定义尺寸规范

这样做的原因是：

1. 与“首版不做交互式图片编辑”保持一致
2. 仍然满足 YGOPro 主卡图输出尺寸的统一要求
3. 把复杂度收敛到可控的导入处理，而不是完整图片编辑器
4. 把重点继续放在模型、事务和导入导出稳定性

### 6.6 身份与编号规则

实现要求：

1. `workspace_id`、`pack_id`、`card_id` 推荐使用 `UUIDv7` 或 `ULID`
2. `card_id` 在新建、导入、复制时生成
3. `card_id` 创建后不可修改
4. 移动卡片到其他 pack 时保留原 `card_id`
5. `code` 是业务编号，不充当内部身份标识
6. 自动分配编号只从推荐范围内寻找候选值

## 7. 后端实现落地

### 7.1 Bootstrap 层

`bootstrap` 负责程序启动和依赖装配。

最低实现要求：

1. 读取应用数据目录位置
2. 初始化 `AppState`
3. 装配 repositories、gateways、services、event bus、job center
4. 注册 Tauri commands

建议 `AppState` 至少持有：

1. `ConfigService`
2. `WorkspaceService`
3. `PackService`
4. `CardQueryService`
5. `CardWriteService`
6. `PackStringsService`
7. `ResourceService`
8. `StandardPackService`
9. `ImportService`
10. `ExportService`
11. `JobService`
12. `SessionManager`

### 7.2 Domain 层

#### 7.2.1 `domain/common`

交付内容：

1. 基础 ID 类型
2. 通用错误码与 warning 码
3. 通用结果类型
4. 时间与路径相关值对象

#### 7.2.2 `domain/config`

交付内容：

1. `GlobalConfig`
2. 默认值策略
3. 配置字段校验

#### 7.2.3 `domain/workspace`

交付内容：

1. `WorkspaceMeta`
2. `pack_order` 约束
3. 工作区级编号冲突判定接口

#### 7.2.4 `domain/pack`

交付内容：

1. `PackMetadata`
2. `PackKind`
3. `display_language_order` 规则
4. `default_export_language` 语义
5. `PackOverview` 派生规则

#### 7.2.5 `domain/card`

这是首版核心域模块，必须完整实现。

交付内容：

1. `CardEntity`
2. `CardTexts`
3. `CardUpdateInput`
4. `BulkCardPatch`
5. `CardListRow`
6. `normalize()`
7. `structure_errors()`
8. `domain_warnings()`
9. `apply_bulk_patch()`
10. `derive_list_row()`

实现要求：

1. 类型适用性按 `primary_type` 和 `monster_flags` 决定
2. 不适用字段保存前自动清空
3. `texts.strings` 规范化为固定 16 项
4. `monster_flags`、`link.markers` 去重并按固定顺序保存

#### 7.2.6 `domain/strings`

交付内容：

1. `PackStringEntry`
2. `PackStringsFile`
3. `PackStringRecord`
4. pack 内 `(kind, key)` 唯一性校验
5. 旧 schema 到聚合模型的兼容迁移

#### 7.2.7 `domain/resource`

交付内容：

1. `ResourceKind`
2. `CardAssetState`
3. 资源路径推导规则
4. 场地图适用性判断

#### 7.2.8 `domain/import` 与 `domain/export`

交付内容：

1. 预检结果模型
2. 冲突分类
3. 缺失资源统计
4. 预检摘要 DTO 内核模型

#### 7.2.9 `domain/validation`

交付内容：

1. 统一 `ValidationIssue`
2. `error` / `warning` 分级
3. 目标对象定位模型

### 7.3 Application 层

Application 层负责把 Domain、Infrastructure、Runtime 串成真实用例。

#### 7.3.1 用例服务分组

必须实现以下服务：

1. `ConfigService`
2. `WorkspaceService`
3. `PackService`
4. `CardQueryService`
5. `CardWriteService`
6. `PackStringsService`
7. `ResourceService`
8. `StandardPackService`
9. `ImportService`
10. `ExportService`
11. `JobService`

写服务组织原则：

1. 读侧可以继续按 feature 拆分，例如 `CardQueryService`、`StandardPackService`
2. 单 `pack` 内写操作应统一委托到一个 pack 级写编排服务，例如 `PackWriteService`
3. `CardWriteService`、`PackStringsService`、`ResourceService` 在首版可以保留对外 trait 和 command 名称，以减少接口震荡
4. 但这些写接口的内部实现不应各自独立维护事务、session 重建和事件发布，而应统一委托给 `PackWriteService`
5. 跨 pack 或 workspace 级写入，例如批量移动、导入、导出，仍由更高层用例编排

实现要求：

1. `service` 输入输出使用 `application/dto`
2. `service` 内部可把 DTO 映射为 `domain model`、`domain patch` 或 `port model`
3. `service` 返回前端时再次映射为 DTO，而不是直接返回内部领域实体

#### 7.3.2 写操作统一流程

普通写操作统一采用以下流程：

```text
接收输入
  -> 校验显式输入中的 workspace_id / pack_id / card_id
  -> 从当前运行时快照中定位受影响 pack；若目标 pack 尚未打开，则临时加载或显式打开所需 pack 快照
  -> 构建 pack 级变更意图
  -> Domain 结构校验
  -> Domain 规范化
  -> 生成 warnings/errors
  -> 若有 error 直接返回
  -> 若有 warning 且未确认，返回 confirmation_token
  -> 计算 dirty 集合（metadata / cards / strings / asset_ops）
  -> 构建 FileOperationPlan
  -> 执行事务
  -> 重建或替换受影响 pack 的 PackSession
  -> 重建或失效相关 cache/index
  -> 发出 pack/workspace 变更事件
  -> 返回最新 DTO
```

补充约束：

1. 普通写操作的事务 owner 为目标 `pack`
2. `pack` 级写编排负责决定“本次实际需要修改哪些文件”
3. 不允许 `CardWriteService`、`PackStringsService`、`ResourceService` 各自独立实现一套落盘和缓存同步逻辑

#### 7.3.3 `confirmation_token` 机制

用于普通写操作的 warning 确认。

实现要求：

1. token 绑定一次具体预检结果
2. token 绑定操作目标和输入快照
3. token 绑定生成时的 `pack revision`
4. token 可额外绑定生成时的 `source_stamp`
5. token 过期、`pack revision` 变化、`source_stamp` 不匹配或 pack 被重载后必须重新发起写操作
6. 手动刷新、重新打开 pack、检测到外部变更并重载后，旧 token 一律失效
7. token 默认保存在运行时内存即可

适用操作：

1. 新建卡片
2. 更新卡片
3. 批量 patch
4. 批量删除
5. 批量移动
6. 字符串写操作
7. 资源写操作中涉及 warning 的场景

#### 7.3.4 `preview_token` 机制

用于导入导出两阶段流程。

实现要求：

1. token 绑定预检时的源输入快照
2. token 绑定目标对象和关键状态摘要
3. token 绑定相关 `pack revision` 集合
4. token 可额外绑定预检时的 `source_stamp` 集合
5. 执行时只接收 token，不重复接收整套原始输入
6. 若源文件、目标 pack、导出选择、语言、输出目录、`pack revision` 或 `source_stamp` 变化，则 token 失效

### 7.4 Infrastructure 层

#### 7.4.1 `infrastructure/fs`

交付内容：

1. 路径规范化
2. 文件存在性检查
3. 目录创建
4. 复制、移动、删除
5. 临时目录管理

#### 7.4.2 `infrastructure/json_store`

交付内容：

1. `global_config.json` 读写
2. `workspace_registry.json` 读写
3. `workspace.json` 读写
4. `metadata.json` 读写
5. `cards.json` 读写
6. `strings.json` 读写
7. `schema_version` 校验

约束：

1. JSON 协议只在这一层理解
2. 其他层拿到的是领域模型或 DTO，不直接拼 JSON
3. Application 层虽然以 `pack` 为写边界，但 `json_store` 仍可以按 `metadata.json`、`cards.json`、`strings.json` 分文件读写

#### 7.4.3 `infrastructure/sqlite_cdb`

交付内容：

1. 读取 `datas` 和 `texts`
2. CDB 到语义化模型的解码
3. 语义化模型到 CDB 的编码
4. round-trip 测试

约束：

1. 只有这一层理解 CDB 位编码
2. Domain 不感知 SQLite 行结构

#### 7.4.4 `infrastructure/strings_conf`

交付内容：

1. 读取 `strings.conf`
2. 写出 `strings.conf`
3. 处理 `system`、`victory`、`counter`、`setname`

#### 7.4.5 `infrastructure/assets`

交付内容：

1. 主卡图存储
2. 场地图片存储
3. 脚本创建/导入/删除
4. 基于 `code` 的命名
5. `code` 变化时资源迁移
6. pack 级资源扫描

#### 7.4.6 `infrastructure/external_editor`

交付内容：

1. 调用系统外部编辑器打开 `.lua`
2. 错误转换为统一应用错误

#### 7.4.7 `infrastructure/transaction`

这是首版稳定性的关键模块，但首版目标不是实现完整的崩溃可恢复事务系统。

交付内容：

1. `FileOperationPlan`
2. `FileOperationStep`
3. 临时写入
4. 提交
5. 最佳努力回退
6. 中间态清理
7. 启动残留清理

建议最小能力：

1. 单文件原子替换
2. 文件复制失败时的最佳努力清理
3. 文件移动失败时的最佳努力回退
4. 新建目录失败时的最佳努力清理
5. 删除前临时保留
6. 程序启动时清理未完成的临时文件

#### 7.4.8 `infrastructure/standard_pack`

交付内容：

1. 从 `ygopro_path` 读取标准包
2. 构建轻量只读索引
3. 按关键词搜索标准卡
4. 查询标准卡号是否存在

#### 7.4.9 `infrastructure/thumbnails`

该模块在首版中不是阻塞项。

建议策略：

1. 先预留模块目录
2. 首版默认不生成持久化缩略图缓存
3. 列表缩略图先采用原图懒加载加前端缩放
4. 若大卡包性能不足，再补缩略图缓存实现

### 7.5 Runtime 层

#### 7.5.1 `runtime/sessions`

最低要求：

1. `WorkspaceSession`
2. `PackSession`
3. 已打开 `pack` 集合与 tab 顺序
4. active pack 切换
5. 打开 workspace 时只加载 workspace 元数据与 pack 摘要，不加载全部自定义 `pack` 的作者态数据
6. `PackSession` 只表示“已打开 `pack`”的运行时快照
7. 打开 pack tab 时加载对应作者态数据并构建 `PackSession`
8. 关闭 pack tab 时释放对应 `PackSession`
9. 切换 active pack 时只切换上下文，不重新读取已经打开的 `pack`
10. 每个 `PackSession` 应维护一个运行时 `revision`
11. 每个 `PackSession` 应维护一个 `source_stamp`，用于表示加载时观测到的磁盘状态摘要
12. 程序内成功写入后递增 `revision`
13. 手动刷新或重载 pack 时重新计算 `source_stamp`，并使旧 token 失效

#### 7.5.2 `runtime/cache`

交付内容：

1. 每个 `pack` 的基础派生结果，例如 `CardListRow`
2. 标准包索引缓存
3. 预检 token / confirmation token 临时缓存

约束：

1. 缓存必须可丢弃、可重建
2. 首版不做多级缓存、后台预热、复杂淘汰策略
3. 查询结果级缓存不是首版重点，优先保证运行时快照和基础派生稳定

#### 7.5.3 `runtime/index`

交付内容：

1. `id -> entity`
2. `code -> id`
3. CardList 搜索索引
4. CardList 排序索引
5. 工作区级编号冲突索引

说明：

1. `id -> entity`、`code -> id` 属于已打开 `pack` 内部索引
2. 工作区级编号冲突索引不能依赖“当前已打开 `pack` 集合”是否完整
3. 首版可通过扫描全部自定义 `pack` 的 `cards.json` 构建轻量工作区级 `code` 索引

#### 7.5.4 `runtime/jobs`

交付内容：

1. Job 状态机
2. Job 进度
3. Job 结果缓存
4. 失败原因缓存

#### 7.5.5 `runtime/events`

交付内容：

1. `job:progress`
2. `job:finished`
3. `workspace:changed`
4. `pack:changed`
5. `standard-pack:index-updated`

### 7.6 Presentation 层

Presentation 是 Tauri 边界层，不承载业务规则。

最低要求：

1. 每个服务模块有对应 commands 文件
2. 输入 DTO 与输出 DTO 独立定义
3. 所有错误统一转换为前端可消费格式
4. 所有命令名称稳定，不把内部模块名泄露给前端

建议命令分组：

1. `config`
2. `workspace`
3. `pack`
4. `card`
5. `strings`
6. `resource`
7. `standard_pack`
8. `import`
9. `export`
10. `job`

## 8. 前端实现落地

### 8.1 前端目录

```text
src/
  app/
  features/
    settings/
    workspace/
    pack/
    card/
    strings/
    standard_pack/
    import_pack/
    export_bundle/
  shared/
    api/
    contracts/
    ui/
    forms/
    hooks/
    state/
    i18n/
    utils/
    types/
```

### 8.2 前端状态策略

实现要求：

1. 服务端数据缓存使用 TanStack Query
2. 纯 UI 状态使用 Zustand（已落地：shellStore 管理 workspace/pack/modal 全局状态）
3. 单卡详情页的 `current_edit_language` 只保留在页面草稿态
4. 不把整个 `PackSession` 镜像进前端全局状态

### 8.3 页面与功能模块

#### 8.3.1 `features/settings`

交付内容：

1. 全局配置页面
2. `ygopro_path` 校验交互
3. 外部编辑器路径设置
4. 程序语言设置

#### 8.3.2 `features/workspace`

交付内容：

1. recent workspaces 列表
2. 新建工作区
3. 打开工作区
4. pack 列表与顺序编辑
5. 已打开 pack 的侧边栏 tab 容器

#### 8.3.3 `features/pack`

交付内容：

1. pack 页面壳
2. `CardList` tab
3. `Strings` tab
4. pack 元数据编辑
5. pack 删除确认
6. 打开 pack tab
7. 关闭 pack tab
8. 多个已打开 pack 的切换

#### 8.3.4 `features/card`

交付内容：

1. CardList 过滤与排序 UI
2. 单卡详情页
3. 单卡编辑表单
4. 批量编辑弹窗
5. warning 确认弹窗

#### 8.3.5 `features/strings`

交付内容：

1. 字符串列表
2. 按语言切换
3. 按 `kind` / `key` 搜索
4. 新增、编辑、删除

#### 8.3.6 `features/standard_pack`

交付内容：

1. 标准包只读搜索页面
2. 标准卡只读详情
3. 标准包索引状态展示

#### 8.3.7 `features/import_pack`

交付内容：

1. 导入向导
2. 预检结果展示
3. 缺失资源统计
4. 提交导入任务

#### 8.3.8 `features/export_bundle`

交付内容：

1. 导出向导
2. pack 多选
3. 导出语言选择
4. 预检结果展示
5. 提交导出任务

### 8.4 交互规则

实现要求：

1. 所有写操作都基于稳定错误码和 warning 码展示文案
2. `needs_confirmation` 的写命令必须进入统一确认弹窗流
3. 导入导出页面统一显示预检结果，再允许执行
4. CardList 列表使用虚拟滚动
5. 卡图缩略图采用懒加载

## 9. 关键技术方案

### 9.1 Pack 加载策略

首版采用“workspace 加载摘要 + pack 通过侧边栏 tab 显式打开/关闭”的保守策略：

1. 打开 workspace 时读取 `workspace.json` 和全部自定义 `pack` 的摘要信息
2. 默认不加载全部 `cards.json`、`strings.json` 和资源状态
3. 用户显式打开某个 `pack` 时，为其构建一个 `PackSession`
4. `WorkspaceSession` 维护已打开 `pack` 的顺序以及当前 `active_pack_id`
5. 已打开 `pack` 之间切换时直接复用已加载的 `PackSession`
6. 用户显式关闭某个 `pack` 时，释放其 `PackSession`
7. 当用户手动刷新或检测到外部变化时，只重载受影响 `pack`
8. 首版限制“同时打开的自定义 `pack` 数量”，建议固定为 `8`

### 9.2 CardList 性能策略

1. `PackSession` 建立时同步生成 `CardListRow`
2. 搜索和排序在后端基于当前已打开 `pack` 的 `PackSession` 完成
3. 前端只展示当前页或当前窗口可见范围
4. 单卡修改、批量修改、移动卡片、资源变化提交成功后，重建受影响 `pack` 的 `CardListRow`
5. 首版不维护复杂查询结果缓存；需要时现场从内存中的 `PackSession` 计算

### 9.3 标准包索引策略

首版采用轻量只读索引，而不是每次都直接查标准包 CDB。

索引至少包含：

1. `code`
2. 标准卡名称摘要
3. 主要类型摘要
4. 用于搜索的文本摘要

索引用途：

1. 标准卡搜索
2. 标准卡号存在性查询
3. 标准包摘要展示

### 9.4 安全写入策略

单文件写入流程：

```text
write temp file
  -> flush/sync
  -> atomic rename replace
```

多文件最佳努力提交流程：

```text
Precheck
  -> Build Plan
  -> Stage temp outputs
  -> Commit in deterministic order
  -> Refresh runtime cache/index
  -> On failure attempt best-effort revert/cleanup
  -> Leave recoverable state if full revert is impossible
```

pack 级写入的最小提交原则：

1. `pack` 是逻辑写边界，不等于每次重写整个 `pack`
2. 若只修改卡片数据，则只提交 `cards.json`、必要时的 `metadata.json`
3. 若只修改 `pack strings`，则只提交 `strings.json`、必要时的 `metadata.json`
4. 若只修改资源，则只提交资源文件与 `metadata.json`
5. 若修改跨越多个作者态文件，则通过单次 `FileOperationPlan` 统一提交

首版保证级别：

1. 保证单文件安全写入
2. 多文件操作只承诺最佳努力回退，不承诺完整的崩溃可恢复原子事务
3. 若中途失败，应优先保证主 JSON 文件不被写坏
4. 允许残留孤儿资源或临时文件，但启动时应进行清理和一致性检查

### 9.5 `code` 变更的一致性处理

单卡修改 `code` 必须作为一个完整程序内操作处理，但首版不承诺严格原子事务语义。

至少包含：

1. 更新 `cards.json`
2. 迁移主卡图
3. 迁移场地图片
4. 迁移脚本
5. 更新 `code -> id` 索引
6. 更新 CardList 派生缓存

失败处理原则：

1. 优先保证 `cards.json` 与 `metadata.json` 不被写坏
2. 若资源迁移中途失败，可接受残留旧文件或新文件，但应保证后续刷新与人工修复可行
3. 启动或手动刷新时应能重新扫描资源状态

### 9.6 错误与 Warning 策略

首版统一使用稳定错误码和 warning 码。

实现要求：

1. 错误码面向程序处理
2. 文案由前端 i18n 映射
3. 后端可附带参数化字段
4. 前端不依赖后端自然语言长句做逻辑判断

### 9.7 UI 多语言策略

首版只保证程序 UI 多语言架构到位。

实现要求：

1. `AppI18n` 与卡片文本、pack strings 分离
2. 错误码、warning 码都有 i18n 映射入口
3. 卡片列表显示语言完全遵从 `pack.display_language_order`
4. 详情页语言切换只是页面临时状态

## 10. 测试与验收策略

### 10.1 单元测试

重点覆盖：

1. 卡片规范化
2. 卡片结构校验
3. 卡片 warning 生成
4. `BulkCardPatch` 应用
5. `PackStrings` 唯一性检查
6. 资源路径推导
7. 编号冲突和间距检查

### 10.2 集成测试

重点覆盖：

1. JSON round-trip
2. CDB 解码与编码 round-trip
3. `strings.conf` 读写 round-trip
4. `code` 修改事务
5. 批量移动卡片事务
6. 导入预检到执行
7. 导出预检到执行

### 10.3 端到端测试

重点覆盖：

1. 首次启动配置
2. 新建 workspace
3. 新建 pack
4. 新建卡片并保存
5. 编辑文本和资源
6. 删除和移动卡片
7. 导入 pack
8. 导出 bundle

### 10.4 手工验收清单

首版发布前至少人工验证：

1. 重启后工作区与 pack 能正确恢复
2. 非法编号被阻断
3. 过近编号触发 warning
4. `code` 变更后资源文件被同步迁移
5. 标准包只读搜索可用
6. 导入导出预检结果与实际执行一致
7. 导出结果可被 YGOPro 正常识别

## 11. 任务分块与里程碑

### 11.1 拆分原则

任务拆分遵循以下原则：

1. 先完成后端真相源，再接前端交互
2. 先完成最小闭环，再扩展外部边界
3. 把高风险能力单独成包，不混入一般 CRUD
4. 每个工作包都必须能定义输入、输出、依赖和验收标准

### 11.2 里程碑总览

建议按四个里程碑推进：

1. M1：作者态最小闭环可运行
2. M2：编辑能力完整
3. M3：标准包、导入、导出打通
4. M4：稳定性、测试与发布收尾

### 11.3 工作包明细

#### WP0 项目骨架与基础约定

目标：

1. 建立目录结构
2. 建立 Tauri command 装配方式
3. 建立 Rust 分层骨架
4. 建立前端 feature 目录骨架
5. 建立基础错误 DTO、结果 DTO、事件命名约定

后端任务：

1. 创建 `bootstrap/domain/application/infrastructure/runtime/presentation` 目录
2. 建立 `AppState` 与依赖注入骨架
3. 建立统一 `AppResult`、`AppErrorDto`
4. 注册空实现 commands

前端任务：

1. 创建 `app/features/shared` 目录
2. 建立 `shared/api` 和基础 `invokeApi`
3. 建立 `I18nProvider`、`QueryClientProvider`、路由壳

交付物：

1. 可编译的前后端项目骨架
2. 基础 command 调用链
3. 错误返回约定

依赖：

1. 无

验收标准：

1. 应用可以启动
2. 前端可以成功调用一个示例命令
3. 目录结构与架构报告一致

#### WP1 核心领域模型与 DTO

目标：

1. 固化所有首版领域模型
2. 固化输入输出 DTO
3. 固化错误码与 warning 码

后端任务：

1. 实现 `domain/common/config/workspace/pack/card/strings/resource/validation`
2. 定义 `application/dto`
3. 定义 `presentation/dto`

前端任务：

1. 建立对应 `shared/contracts` 类型
2. 建立错误码与 warning 码映射框架

交付物：

1. 可测试的领域模型
2. 前后端稳定 DTO 合同

依赖：

1. WP0

验收标准：

1. Card 规范化与校验单测通过
2. `CardId`、`PackStrings`、编号规则都已建模
3. DTO 能覆盖所有首版命令

#### WP2 作者态 JSON 存储与安全写入基础

目标：

1. 跑通作者态 JSON 读写
2. 建立单文件安全写入
3. 建立多文件最佳努力提交最小实现

后端任务：

1. 实现 `json_store`
2. 实现 `fs`
3. 实现 `transaction`
4. 建立测试 fixtures

前端任务：

1. 无强依赖，可并行准备错误展示和 loading 基础组件

交付物：

1. `global_config/workspace/pack/cards/strings` 读写能力
2. 安全写入基础设施

依赖：

1. WP1

验收标准：

1. JSON round-trip 测试通过
2. 单文件写入在失败时不破坏原文件
3. 多文件操作能覆盖创建、替换、移动、删除的最佳努力清理或回退

#### WP3 全局配置与 Workspace 管理

目标：

1. 打通应用级状态和工作区管理
2. 完成 recent workspaces 注册表

后端任务：

1. 实现 `ConfigService`
2. 实现 `WorkspaceService`
3. 建立程序级应用数据目录管理

前端任务：

1. 完成设置页
2. 完成 recent workspaces 列表
3. 完成新建、打开、切换工作区 UI

交付物：

1. 配置页
2. 工作区页
3. recent workspaces 行为闭环

依赖：

1. WP2

验收标准：

1. 可以创建并重新打开 workspace
2. recent workspaces 正确更新
3. 无效 recent workspace 路径会得到明确反馈

#### WP4 Pack 管理与 PackSession

目标：

1. 跑通 pack 生命周期
2. 建立 workspace 打开后的 session 管理

后端任务：

1. 实现 `PackService`
2. 实现 `WorkspaceSession` 与 `PackSession` 基础
3. 实现 pack metadata 读写
4. 实现 pack 概览派生

前端任务：

1. 完成 workspace 下 pack 列表页面
2. 完成新建 pack、编辑 metadata、删除 pack
3. 完成 pack 页面壳与 tab 结构

交付物：

1. 可创建和删除 pack
2. 可显式打开和关闭 pack tab
3. 可显示 pack 摘要

依赖：

1. WP3

验收标准：

1. 打开 workspace 时只加载 workspace 元数据与 pack 摘要
2. 打开 pack tab 时加载对应 pack 的作者态数据并构建 `PackSession`
3. 在已打开 pack tab 之间切换时不重新读取已打开 pack 的作者态数据
4. 关闭 pack tab 时释放对应 `PackSession`
5. `updated_at` 在 pack 变更后正确刷新

#### WP5 Card 查询、单卡 CRUD 与列表能力

目标：

1. 完成首版核心编辑闭环
2. 建立 CardList 搜索、排序、局部刷新

后端任务：

1. 实现 `CardQueryService`
2. 实现 `CardWriteService`
3. 实现 `CardListRow` 缓存与索引
4. 实现编号建议、冲突检测、warning 确认

前端任务：

1. 完成 `CardList`
2. 完成单卡详情与编辑表单
3. 完成新建卡片流程
4. 完成 warning 确认弹窗

交付物：

1. 单卡新建、编辑、删除
2. CardList 搜索和排序
3. 自动编号建议

依赖：

1. WP4

验收标准：

1. 单卡保存后能立即反映到列表
2. `code` 冲突会被阻断
3. 间距过近会触发 warning 确认
4. 重启后卡片数据不丢失

#### WP6 批量操作、PackStrings 与资源管理

目标：

1. 补齐 pack 内编辑能力
2. 打通文本、图片、脚本资源管理

后端任务：

1. 实现 `BatchPatchCardsInput`
2. 实现 `move_cards`
3. 实现 `PackStringsService`
4. 实现 `ResourceService`
5. 实现 `code` 变更资源迁移的最佳努力一致性处理
6. 实现 `PackStrings` 聚合多语言模型
7. 实现最小 `preview_export_bundle` 冲突预检骨架

前端任务：

1. 完成批量编辑和批量移动 UI
2. 完成 `Strings` tab
3. 完成主卡图、场地图、脚本区块 UI
4. 完成脚本外部打开入口
5. 完成 `Strings` 的十六进制 key 输入与显示

交付物：

1. 批量删除、批量移动、批量 patch
2. `Strings` tab
3. 主卡图/场地图/脚本管理
4. `PackStrings` 多语言聚合模型
5. 导出冲突预检基础能力

依赖：

1. WP5

验收标准：

1. 批量移动后 `CardId` 保持不变
2. `(kind, key)` 唯一性正确阻断
3. `code` 变更后资源文件被正确迁移，或在失败时留下可刷新、可恢复的状态
4. 场地图片只允许场地魔法绑定
5. `Strings` key 在 UI 中按十六进制输入和显示
6. `PackStrings` 可按目标语言正确投影列表视图

#### WP7 Job/Event 基础设施

目标：

1. 为长任务建立统一调度与展示通道
2. 让导入、导出、标准包索引重建拥有统一运行方式

后端任务：

1. 实现 `JobService`
2. 实现 `runtime/jobs`
3. 实现 `runtime/events`
4. 实现 `job:progress` 与 `job:finished`

前端任务：

1. 完成全局任务通知区或任务弹层
2. 完成任务进度订阅
3. 完成任务结果展示

交付物：

1. Job 状态查询
2. Job 进度事件
3. 前端任务反馈 UI

依赖：

1. WP4

验收标准：

1. 前端可看到任务进度变化
2. 任务失败原因可回显
3. 不影响普通短命令调用

#### WP8 标准包只读接入

目标：

1. 打通标准包只读浏览与编号冲突检测
2. 建立标准包索引缓存

后端任务：

1. 实现 `standard_pack` repository/gateway
2. 实现索引构建
3. 实现只读搜索
4. 接入编号存在性查询

前端任务：

1. 完成标准包搜索页面
2. 完成索引状态显示和重建入口

交付物：

1. 标准包状态页
2. 标准卡搜索
3. 编号冲突接入

依赖：

1. WP3
2. WP7

验收标准：

1. `ygopro_path` 合法时可以建立索引
2. 标准包搜索可用
3. 新建或编辑卡片时可检测标准卡号冲突

#### WP9 导入能力

目标：

1. 打通运行时资源导入为作者态 pack
2. 完成预检和执行两阶段流程

后端任务：

1. 实现 `sqlite_cdb`
2. 实现 `strings_conf`
3. 实现 `ImportService`
4. 接入 `preview_token`
5. 接入 Job 执行

前端任务：

1. 完成导入向导
2. 完成预检结果展示
3. 完成提交导入任务

交付物：

1. 导入预检
2. 导入任务执行
3. 导入结果展示

依赖：

1. WP6
2. WP7
3. WP8

验收标准：

1. 导入前能看到错误、warning、缺失资源统计
2. 导入执行只接收 `preview_token`
3. 导入后 pack、卡片、资源、strings 可正常打开

#### WP10 导出能力

目标：

1. 打通多 pack 融合导出
2. 保证导出冲突预检和执行一致

后端任务：

1. 实现 `ExportService`
2. 实现多 pack 冲突检查
3. 实现 CDB 与 `strings.conf` 导出
4. 实现运行时目录资源写出
5. 接入 Job 执行

前端任务：

1. 完成导出向导
2. 完成 pack 多选
3. 完成语言选择
4. 完成预检与执行 UI

交付物：

1. 导出预检
2. 导出任务执行
3. 导出结果摘要

依赖：

1. WP9

验收标准：

1. 多 pack 冲突能在预检阶段暴露
2. 缺失导出语言会阻断
3. 导出目录能被 YGOPro 识别

#### WP11 稳定性加固与测试收尾

目标：

1. 把首版从“能跑”收束到“可交付”
2. 完成测试、文档、错误处理和边界回归

后端任务：

1. 补全安全写入失败注入测试
2. 补全导入导出 fixtures
3. 补全 schema mismatch 错误处理
4. 补全事件与缓存失效边界

前端任务：

1. 补全空状态、错误态、loading 态
2. 补全确认弹窗与任务状态体验
3. 补全手工回归清单

交付物：

1. 稳定版首版文档
2. 测试报告
3. 发布前回归清单

依赖：

1. WP10

验收标准：

1. 核心流程都有自动化测试覆盖
2. 失败场景有明确错误反馈
3. 文档、实现、UI 行为三者一致

## 12. 关键路径与并行建议

### 12.1 关键路径

最关键的依赖顺序如下：

1. `WP0 -> WP1 -> WP2`
2. `WP2 -> WP3 -> WP4 -> WP5 -> WP6`
3. `WP7` 可以在 `WP4` 之后启动
4. `WP8` 依赖 `WP3 + WP7`
5. `WP9` 依赖 `WP6 + WP7 + WP8`
6. `WP10` 依赖 `WP9`
7. `WP11` 在全部功能闭环后执行

### 12.2 可并行部分

若多人协作，建议按以下方向并行：

1. 一人负责 Domain + JSON + Transaction 基建
2. 一人负责 Workspace/Pack/Card 前端页面与 API 包装
3. 一人负责 Standard Pack / Import / Export / Fixtures

并行前提：

1. DTO 和错误码先冻结
2. 命令名称先冻结
3. 资源路径规则先冻结

## 13. 实现风险与注意事项

### 13.1 高风险点

1. Card 语义模型与 CDB 编码之间的双向转换
2. `code` 变化时的资源迁移一致性
3. 多 pack 导出时的资源路径冲突
4. 标准包索引与实际 YGOPro 文件变动的同步
5. 导入导出 token 失效判断

### 13.2 首版建议的保守处理

1. 不做脚本文件监听，只提供手动刷新
2. 不做缩略图文件持久化，先用懒加载
3. 不做 DTO 自动生成，先手写并保持边界清晰
4. 不做复杂批量编辑，按功能规范的字段范围收敛
5. 不实现完整的崩溃可恢复多文件事务；首版采用单文件安全写入 + 多文件最佳努力回退

DTO / 模型取舍建议：

1. `PackRepository`、`WorkspaceRepository`、`ConfigRepository` 这类真相源仓储允许直接读写 `domain model`
2. 卡片详情输出、卡片编辑输入、字符串编辑输入、标准包搜索结果、Job 状态输出都应保留独立 DTO
3. 列表页一律只返回列表 DTO，不返回完整领域实体
4. 事件总线、任务调度、标准包仓储不返回 presentation DTO
5. 单 `pack` 内写操作的内部实现应共享统一的 `PackWriteService` / `PackMutationPlan`

### 13.3 实现时必须持续检查的一致性

1. 文档中的模型字段名与 DTO 字段名一致
2. 前端筛选排序字段与后端 `CardListRow` 一致
3. 标准包只读规则不被任何写用例绕过
4. `CardId` 永远不因移动、导出、改号而改变
5. `AppI18n` 不混入卡片文本和 `pack strings`

## 14. 最终建议

YGOCMG 首版不应把精力分散在过多横向功能上，而应集中完成三件事：

1. 建立稳定的作者态编辑模型
2. 建立可靠的安全写入与导入导出边界
3. 建立清晰的前后端协作与运行时缓存机制

按当前规模，首版建议进一步明确：

1. `workspace` 只常驻摘要与已打开 tab 状态，不默认常驻全部自定义 `pack` 作者态
2. `PackSession` 只为已打开 `pack` 存在
3. pack tab 切换只是切换 active pack 指针
4. 缓存只承担可丢弃读模型职责，不承担正确性职责
5. 写成功后优先重建受影响 `pack` 的运行时快照，而不是手工维护复杂缓存同步

按本文档的实现顺序推进，首版可以先拿到一个稳定的作者态最小闭环，再逐步接入标准包、导入、导出和 Job 系统。这样做的好处是：

1. 风险集中在真正困难的地方
2. 每个阶段都能形成可验证成果
3. 后续扩展 AI、图片处理、脚本辅助时不会推翻首版内核
