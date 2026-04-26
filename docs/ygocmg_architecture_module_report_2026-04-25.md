# YGOCMG 架构与模块职责分析报告

日期：2026-04-25  
状态：Draft  
适用范围：YGOCMG 首版（v1）

关联文档：
- [项目粗略设计方案](./ygocmg.md)
- [YGOCMG 首版功能规范 v1](./ygocmg_v1_functional_spec_2026-04-25.md)
- [卡片数据模型语义化重构方案 v2](./card_data_model_refactor_v2_2026-04-23.md)

## 1. 文档目标

本文档回答以下问题：

1. YGOCMG 首版整体应采用什么架构
2. 前后端、业务层、存储层如何划分边界
3. 代码目录应如何组织
4. 各模块分别负责什么
5. 模块之间通过什么 API / 接口协作
6. 哪些地方存在重大架构决策，需要在实现前与产品方向一起确认

本文档不替代功能规范。功能规范定义“做什么”，本文档定义“代码如何组织、边界如何收敛、模块如何协作”。

## 2. 结论先行

YGOCMG 首版建议采用：

1. `Tauri + Rust + React/TypeScript` 的桌面应用架构
2. “模块化单体”而不是“前后端分离微服务”
3. Rust 后端作为唯一业务真相源
4. 作者态数据以 `workspace + pack + json + assets` 为持久化主模型
5. CDB / `strings.conf` / `pics/` / `script/` 只作为导入导出边界，不进入核心编辑模型
6. 前端只负责交互、草稿状态、界面展示，不负责最终业务规则
7. 长耗时任务采用“任务中心 + 进度事件”模式，而不是所有操作都阻塞式 `invoke`
8. `CardId` 与 `code` 分离，`CardId` 作为作者态持久化身份标识写入 `cards.json`
9. 程序级工作区注册表独立于任何 `workspace`

一句话概括：

YGOCMG 首版最合适的实现，不是“一个 UI 去直接改文件”，而是“一个 Rust 领域内核驱动的本地作者态卡包管理器”。

## 3. 架构设计原则

### 3.1 作者态与运行态分离

系统内部始终围绕作者态模型工作：

1. `GlobalConfig`
2. `Workspace`
3. `Pack`
4. `CardEntity`
5. `PackStrings`
6. 卡图 / 场地图 / 脚本资源

运行态资源只是边界产物：

1. `.cdb`
2. `pics/`
3. `pics/field/`
4. `script/`
5. `strings.conf`

### 3.2 业务规则后端唯一化

以下逻辑只允许在 Rust 后端实现一次：

1. `CardEntity` 规范化
2. 字段适用性判断
3. 编号策略与冲突检测
4. 导入校验
5. 导出校验
6. `pack strings` 唯一性规则
7. 文件安全写入与回滚
8. 资源路径推导与重命名

前端可以做即时提示，但最终结果以后端为准。

### 3.3 读写分离

持久化真相源与运行时展示模型分离。首版建议明确区分以下四类状态：

1. 持久化真相源：`global_config.json`、`workspace_registry.json`、`workspace.json`、`metadata.json`、`cards.json`、`strings.json`、资源文件
2. 后端运行时快照：`WorkspaceSession`、`PackSession`，它们是从真相源读取出的当前工作区内存镜像，用于支撑读取性能和当前上下文
3. 运行时派生缓存：`CardListRow`、`PackSummary`、搜索索引、导出预检结果，它们必须可丢弃、可重建
4. 前端草稿与 UI 状态：表单输入、当前筛选条件、弹窗状态、当前编辑语言等，它们不回写后端 runtime session

约束：

1. 真相源决定正确性
2. `PackSession` 只是已加载快照，不是第二真相源
3. 写命令必须使用显式 `workspace_id / pack_id / card_id`
4. 删除任意派生缓存后，系统至多变慢，不能变错

### 3.4 `pack` 作为写聚合根

首版建议把自定义 `pack` 明确为作者态写入的聚合根。

这意味着：

1. `card`、`pack strings`、资源、pack metadata 都属于同一个 `pack` 写边界
2. 写入时由 `pack` 统一负责校验、warning、安全写入规划、session 重建和事件发布
3. 这是一种逻辑写边界设计，不等于每次都重写整个 `pack`
4. 实际物理落盘依旧按受影响文件最小化执行

典型例子：

1. 改卡片字段：通常修改 `cards.json` 与 `metadata.json`
2. 改 `pack strings`：通常修改 `strings.json` 与 `metadata.json`
3. 导入主卡图：通常修改资源文件与 `metadata.json`
4. 改卡片 `code`：同时修改 `cards.json`、相关资源文件与 `metadata.json`

### 3.5 边界适配器集中化

所有外部格式都必须通过 Adapter 进入系统：

1. CDB Adapter
2. `strings.conf` Adapter
3. 资源目录 Adapter
4. 外部编辑器 Adapter
5. 文件安全写入 / 最佳努力回退 Adapter

这保证外部格式变化不会污染内部模型。

### 3.6 首版优先稳定性而非“架构炫技”

首版不建议上：

1. 多进程服务
2. 内嵌数据库替代作者态 JSON
3. 过早插件系统
4. 前端复杂全局状态框架堆叠
5. 过度事件驱动

首版建议把复杂度集中在真正难的地方：

1. 语义化模型
2. 导入导出
3. 文件事务
4. 列表性能

## 4. 整体架构

### 4.1 逻辑总览

```text
+-------------------------------------------------------------+
|                        Tauri Desktop App                    |
|                                                             |
|  +----------------------+        invoke / event             |
|  | React / TS Frontend  | <-------------------------------> |
|  | UI + Draft State     |                                   |
|  +----------------------+                                   |
|                |                                            |
|                v                                            |
|  +-------------------------------------------------------+  |
|  | Rust Backend Core                                     |  |
|  |                                                       |  |
|  |  Domain           Application         Runtime         |  |
|  |  Models/Rules  -> UseCases/Services -> Sessions/Jobs  |  |
|  |                                                       |  |
|  |  Infrastructure / Adapters                           |  |
|  |  FS / JSON / CDB / strings.conf / Assets / Editor    |  |
|  +-------------------------------------------------------+  |
+-------------------------------------------------------------+
                 |                         |                  |
                 v                         v                  v
        workspace json/assets       ygopro standard pack   OS/editor
```

### 4.2 责任边界

前端负责：

1. 页面结构
2. 表单交互
3. 草稿编辑状态
4. 列表展示与虚拟滚动
5. 弹窗、确认、通知
6. App UI 多语言

后端负责：

1. 模型真相源
2. 校验与规范化
3. 工作区与卡包管理
4. 搜索、排序、摘要派生
5. 导入导出
6. 文件事务与回滚
7. 外部文件系统与 YGOPro 的交互

### 4.3 运行流

典型写操作路径：

```text
UI 编辑表单
  -> 调用 Tauri command（显式携带 workspace_id / pack_id / card_id）
  -> Application Service 执行业务用例
  -> 根据显式 ID 取得受影响 pack 的当前运行时快照
  -> Domain 校验/规范化
  -> 生成 warnings/errors
  -> Transaction Plan 规划涉及的文件与资源
  -> Safe Writer / Asset Store 落盘
  -> 重建或替换受影响 pack 的 runtime snapshot
  -> 失效并重建相关派生缓存
  -> 返回结果或 warning / error
```

典型读操作路径：

```text
UI 请求 pack 卡表
  -> QueryService
  -> 根据显式 pack_id 获取 PackSession；若 pack 尚未处于打开态，则先执行 pack 打开流程
  -> 若派生缓存命中则直接返回
  -> 未命中则基于 PackSession 现算 CardListRow / 搜索 / 排序结果
  -> 回填可复用派生缓存
  -> 返回分页/排序/搜索结果
```

## 5. 推荐代码层级

## 5.1 仓库目录

```text
ygocmg/
  docs/
  src/
    app/
    features/
    shared/
  src-tauri/
    src/
      domain/
      application/
      infrastructure/
      runtime/
      presentation/
      bootstrap/
      tests/
```

说明：

运行时数据例如 `global_config.json`、`workspace_registry.json`、标准包索引缓存，应该存放在应用数据目录，而不是仓库目录。

## 5.2 Rust 后端目录

```text
src-tauri/src/
  bootstrap/
    mod.rs
    app_state.rs
    wiring.rs

  domain/
    common/
    config/
    workspace/
    pack/
    card/
    strings/
    resource/
    validation/
    export/
    import/

  application/
    dto/
    ports/
    config/
    workspace/
    pack/
    card/
    strings/
    resource/
    standard_pack/
    import/
    export/
    jobs/

  infrastructure/
    fs/
    json_store/
    sqlite_cdb/
    strings_conf/
    assets/
    external_editor/
    transaction/
    thumbnails/
    standard_pack/

  runtime/
    sessions/
    cache/
    index/
    jobs/
    events/

  presentation/
    commands/
    dto/
    errors/
    events/
```

## 5.3 前端目录

```text
src/
  app/
    App.tsx
    router.tsx
    providers/
    layouts/

  features/
    settings/
    workspace/
    pack/
    card/
    strings/
    export_bundle/
    import_pack/
    standard_pack/

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

说明：

1. 前端采用“按业务特性组织”的结构，而不是按技术类型横切到处散落
2. 后端采用“分层 + 按领域分包”的结构
3. `presentation` 只暴露命令和 DTO，不放业务逻辑
4. `runtime` 负责进程内会话态，不直接负责持久化协议

## 6. 后端分层设计

## 6.1 Domain 层

Domain 层是纯业务规则层。

它负责：

1. 定义核心实体和值对象
2. 定义结构一致性约束
3. 定义 warning / error 规则
4. 定义规范化规则
5. 定义不依赖外部系统的纯计算逻辑

它不负责：

1. 文件读写
2. SQLite 操作
3. Tauri command
4. 任务调度
5. UI DTO

### 6.1.1 `domain/common`

职责：

1. 公共 ID 类型
2. 时间戳/路径等通用值对象
3. 错误码与 warning 码
4. 通用结果模型

建议类型：

```rust
pub type WorkspaceId = String;
pub type PackId = String;
pub type CardId = String;
pub type LanguageCode = String;

pub enum DomainErrorCode {
    InvalidInput,
    CodeConflict,
    SchemaMismatch,
    FileCorrupted,
    UnsupportedOperation,
}

pub enum DomainWarningCode {
    CodeTooClose,
    CodeInReservedRange,
    MissingImage,
    MissingScript,
    MissingFieldImage,
    UnknownEnumValue,
}
```

说明：

1. `CardId` 是作者态持久化主键，不从 `code` 推导
2. `CardId` 在新建、导入、复制时生成，推荐使用 `UUIDv7` 或 `ULID`
3. `CardId` 至少在同一 `workspace` 内唯一

### 6.1.2 `domain/config`

职责：

1. `GlobalConfig` 模型
2. 配置字段约束
3. 默认值策略

核心模型：

```rust
pub struct GlobalConfig {
    pub app_language: LanguageCode,
    pub ygopro_path: Option<String>,
    pub external_text_editor_path: Option<String>,
    pub custom_code_recommended_min: u32,
    pub custom_code_recommended_max: u32,
    pub custom_code_min_gap: u32,
}
```

### 6.1.3 `domain/workspace`

职责：

1. `WorkspaceMeta`
2. `pack_order`
3. 工作区级约束
4. 工作区上下文中的编号冲突规则

说明：

工作区不是卡片真相源本身，但它是多个 `pack` 的管理边界和导出边界。

### 6.1.4 `domain/pack`

职责：

1. `PackMetadata`
2. `PackKind`
3. 显示语言顺序
4. 导出默认语言语义
5. pack 摘要逻辑
6. pack 级作者态聚合模型

建议补充一个内部聚合模型：

```rust
pub struct PackAuthorState {
    pub metadata: PackMetadata,
    pub cards: Vec<CardEntity>,
    pub strings: PackStringsFile,
}
```

说明：

1. `PackAuthorState` 是单 `pack` 作者态写编排使用的内部模型
2. 它不要求把资源文件内容整体读入内存
3. 资源仍通过 `AssetRepository` 扫描与规划，但其归属边界仍然是 `pack`

### 6.1.5 `domain/card`

职责：

1. `CardEntity`
2. `CardTexts`
3. `CardUpdateInput`
4. `BulkCardPatch`
5. `CardListRow` 派生规则
6. `normalize()`
7. `structure_errors()`
8. `domain_warnings()`

这是首版的核心域模块。

### 6.1.6 `domain/strings`

职责：

1. `PackStringEntry`
2. `PackStringsFile`
3. `(kind, key)` 唯一性约束
4. 语言维度组织

### 6.1.7 `domain/resource`

职责：

1. 资源类型定义
2. 资源路径规则
3. `code` 到资源路径的映射
4. 场地图适用性判断

建议模型：

```rust
pub enum ResourceKind {
    MainImage,
    FieldImage,
    Script,
}

pub struct CardAssetState {
    pub has_main_image: bool,
    pub has_field_image: bool,
    pub has_script: bool,
}
```

### 6.1.8 `domain/import` 与 `domain/export`

职责：

1. 导入预检结果
2. 导出预检结果
3. 冲突/缺失/警告分类

这两个模块不负责真正读写文件，但负责定义预检结果的数据形状。

### 6.1.9 `domain/validation`

职责：

1. 通用校验结果模型
2. warning / error 聚合
3. 保存前规范化输出

建议统一模型：

```rust
pub struct ValidationIssue {
    pub code: String,
    pub level: IssueLevel,
    pub message_args: Vec<String>,
    pub target: ValidationTarget,
}

pub enum IssueLevel {
    Error,
    Warning,
}
```

## 6.2 Application 层

Application 层是用例编排层。

它负责：

1. 协调多个 Domain 模块
2. 调用 Infrastructure 提供的 ports
3. 组织安全写入与最佳努力回退
4. 更新运行时会话
5. 向 Presentation 返回用例结果 DTO

它不负责：

1. 真正的文件协议细节
2. UI 呈现
3. 原始 Tauri 参数解析

类型边界约束：

1. `application service` 的输入输出使用 `application dto`，不直接把 `domain entity` 暴露给前端
2. 真相源仓储可以直接读写 `domain model`，因为它们服务的就是作者态持久化模型
3. `application/ports` 不返回 `presentation dto` 或 `application dto`
4. `event bus` 只发布 `application` 语义事件，不直接依赖 `runtime` 对外事件结构
5. `runtime` 中的缓存、session、job store 都是内部支撑对象，不应成为前端 API 契约的一部分

补充组织原则：

1. 读侧可以按 `card`、`strings`、`standard_pack` 等 feature 拆分
2. 单 `pack` 内写侧应统一收敛到一个 pack 级写编排入口
3. 跨 pack 或 workspace 级用例再由更高层服务负责

### 6.2.1 `application/ports`

这是最重要的解耦层，定义后端依赖的外部能力接口。

建议 ports：

```rust
pub trait ConfigRepository { ... }
pub trait WorkspaceRegistryRepository { ... }
pub trait WorkspaceRepository { ... }
pub trait PackRepository { ... }
pub trait AssetRepository { ... }
pub trait StandardPackRepository { ... }
pub trait CdbGateway { ... }
pub trait StringsConfGateway { ... }
pub trait ExternalEditorGateway { ... }
pub trait TransactionManager { ... }
pub trait EventBus { ... }
pub trait JobScheduler { ... }
```

### 6.2.2 `application/config`

职责：

1. 读取和更新全局配置
2. 校验 `ygopro_path`
3. 触发标准包重建索引

建议服务接口：

```rust
pub trait ConfigService {
    fn get_global_config(&self) -> AppResult<GlobalConfigDto>;
    fn update_global_config(&self, input: UpdateGlobalConfigInput) -> AppResult<GlobalConfigDto>;
    fn validate_ygopro_path(&self, input: ValidateYgoProPathInput) -> AppResult<YgoProPathCheckResult>;
}
```

### 6.2.3 `application/workspace`

职责：

1. 新建工作区
2. 打开工作区
3. 切换工作区
4. 获取工作区概览
5. 更新 pack 顺序
6. 提供 workspace 级 pack 摘要与已打开 tab 状态

建议服务接口：

```rust
pub trait WorkspaceService {
    fn list_recent_workspaces(&self) -> AppResult<Vec<WorkspaceListItemDto>>;
    fn create_workspace(&self, input: CreateWorkspaceInput) -> AppResult<WorkspaceOpenedDto>;
    fn open_workspace(&self, input: OpenWorkspaceInput) -> AppResult<WorkspaceOpenedDto>;
    fn close_workspace(&self) -> AppResult<()>;
    fn reorder_packs(&self, input: ReorderPacksInput) -> AppResult<WorkspaceMetaDto>;
}
```

说明：

1. `list_recent_workspaces` 读取程序级工作区注册表，而不是扫描磁盘
2. 首版 UI 只消费 recent workspaces、create、open 相关能力，不提供 workspace 删除入口

### 6.2.4 `application/pack`

职责：

1. 新建空白 pack
2. 编辑 pack 元数据
3. 删除 pack
4. 打开 pack tab 并加载 pack 页面数据
5. 关闭 pack tab
6. 提供列表摘要
7. 管理当前 active pack session

建议服务接口：

```rust
pub trait PackService {
    fn create_pack(&self, input: CreatePackInput) -> AppResult<PackOverviewDto>;
    fn update_pack_metadata(&self, input: UpdatePackMetadataInput) -> AppResult<PackMetadataDto>;
    fn delete_pack(&self, input: DeletePackInput) -> AppResult<()>;
    fn open_pack(&self, input: OpenPackInput) -> AppResult<PackSnapshotDto>;
    fn close_pack(&self, input: ClosePackInput) -> AppResult<WorkspaceOpenedDto>;
    fn get_pack_overview(&self, input: GetPackInput) -> AppResult<PackOverviewDto>;
}
```

### 6.2.5 `application/card`

职责：

1. 查询卡片详情
2. 查询卡表
3. 自动分配编号
4. 单 `pack` 内卡片写操作的接口外观
5. 跨 pack 卡片移动用例

建议服务接口：

```rust
pub trait CardQueryService {
    fn list_cards(&self, input: ListCardsInput) -> AppResult<CardListPageDto>;
    fn get_card(&self, input: GetCardInput) -> AppResult<CardDetailDto>;
    fn suggest_next_code(&self, input: SuggestCodeInput) -> AppResult<CodeSuggestionDto>;
}

pub trait CardWriteService {
    fn create_card(&self, input: CreateCardInput) -> AppResult<WriteResult<CardDetailDto>>;
    fn update_card(&self, input: UpdateCardInput) -> AppResult<WriteResult<CardDetailDto>>;
    fn delete_cards(&self, input: DeleteCardsInput) -> AppResult<WriteResult<DeleteCardsResultDto>>;
    fn batch_patch_cards(&self, input: BatchPatchCardsInput) -> AppResult<WriteResult<BatchPatchResultDto>>;
    fn move_cards(&self, input: MoveCardsInput) -> AppResult<WriteResult<MoveCardsResultDto>>;
}
```

说明：

1. `create_card`、`update_card`、`delete_cards`、`batch_patch_cards` 属于单 `pack` 写入，应内部委托给统一的 `PackWriteService`
2. `move_cards` 涉及跨 pack，一般由更高层用例编排，并在内部同时协调两个 `pack`

### 6.2.6 `application/strings`

职责：

1. 查询 `pack strings`
2. 新增/修改/删除字符串条目
3. 搜索 `(kind, key, value)`
4. 校验唯一性冲突

建议服务接口：

```rust
pub trait PackStringsService {
    fn list_pack_strings(&self, input: ListPackStringsInput) -> AppResult<PackStringsPageDto>;
    fn upsert_pack_string(&self, input: UpsertPackStringInput) -> AppResult<WriteResult<PackStringsPageDto>>;
    fn delete_pack_strings(&self, input: DeletePackStringsInput) -> AppResult<WriteResult<DeletePackStringsResultDto>>;
}
```

说明：

1. `PackStringsService` 的写操作属于单 `pack` 写入
2. 首版可以保留这个 service trait，但内部应委托给 `PackWriteService`

### 6.2.7 `application/resource`

职责：

1. 主卡图导入/替换/删除
2. 场地图导入/替换/删除
3. 脚本创建/导入/删除
4. 脚本外部打开
5. 资源状态刷新

建议服务接口：

```rust
pub trait ResourceService {
    fn import_main_image(&self, input: ImportMainImageInput) -> AppResult<WriteResult<CardAssetStateDto>>;
    fn delete_main_image(&self, input: DeleteMainImageInput) -> AppResult<WriteResult<CardAssetStateDto>>;
    fn import_field_image(&self, input: ImportFieldImageInput) -> AppResult<WriteResult<CardAssetStateDto>>;
    fn delete_field_image(&self, input: DeleteFieldImageInput) -> AppResult<WriteResult<CardAssetStateDto>>;
    fn create_empty_script(&self, input: CreateEmptyScriptInput) -> AppResult<WriteResult<CardAssetStateDto>>;
    fn import_script(&self, input: ImportScriptInput) -> AppResult<WriteResult<CardAssetStateDto>>;
    fn delete_script(&self, input: DeleteScriptInput) -> AppResult<WriteResult<CardAssetStateDto>>;
    fn open_script_external(&self, input: OpenScriptExternalInput) -> AppResult<()>;
}
```

说明：

1. 资源写入仍走独立 API，更符合前端交互
2. 但其内部安全写入、metadata 更新时间、session 重建和事件发布都应委托给统一的 `PackWriteService`

### 6.2.8 `application/standard_pack`

职责：

1. 标准包只读接入
2. 标准包搜索
3. 标准包摘要展示
4. 编号冲突检测辅助

建议服务接口：

```rust
pub trait StandardPackService {
    fn get_standard_pack_status(&self) -> AppResult<StandardPackStatusDto>;
    fn rebuild_standard_pack_index(&self) -> AppResult<JobAcceptedDto>;
    fn search_standard_cards(&self, input: SearchStandardCardsInput) -> AppResult<StandardCardPageDto>;
}
```

### 6.2.9 `application/import`

职责：

1. 解析导入输入
2. 预检冲突和警告
3. 生成导入计划
4. 执行导入安全写入

建议服务接口：

```rust
pub trait ImportService {
    fn preview_import_pack(&self, input: PreviewImportPackInput) -> AppResult<ImportPreviewDto>;
    fn execute_import_pack(&self, input: ExecuteImportPackInput) -> AppResult<JobAcceptedDto>;
}
```

说明：

1. `preview_import_pack` 负责生成一次性 `preview_token`
2. `execute_import_pack` 只接收 `preview_token`，不重复接收整套原始预检参数
3. 执行前必须校验 `preview_token` 对应快照仍然有效

### 6.2.10 `application/export`

职责：

1. 多 pack 导出预检
2. 语言完整性检查
3. 导出资源冲突检查
4. 生成运行时 bundle

建议服务接口：

```rust
pub trait ExportService {
    fn preview_export_bundle(&self, input: PreviewExportBundleInput) -> AppResult<ExportPreviewDto>;
    fn execute_export_bundle(&self, input: ExecuteExportBundleInput) -> AppResult<JobAcceptedDto>;
}
```

说明：

1. `preview_export_bundle` 负责生成一次性 `preview_token`
2. `execute_export_bundle` 只接收 `preview_token`
3. 执行前必须复核相关输入和源数据快照未变化

### 6.2.11 `application/jobs`

职责：

1. 长任务创建
2. 进度上报
3. 任务状态查询
4. 任务结果归档

说明：

首版建议只让以下操作进入 Job 系统：

1. 标准包索引重建
2. 导入 pack
3. 导出 bundle

## 6.3 Infrastructure 层

Infrastructure 层实现所有 ports。

### 6.3.1 `infrastructure/fs`

职责：

1. 路径规范化
2. 目录存在性检测
3. 文件复制/移动/删除
4. 临时目录管理

### 6.3.2 `infrastructure/json_store`

职责：

1. `workspace.json` 读写
2. `metadata.json` 读写
3. `cards.json` 读写
4. `strings.json` 读写
5. `schema_version` 校验
6. 程序级 `global_config.json` 与 `workspace_registry.json` 读写

说明：

这里是作者态 JSON 的唯一文件协议模块。

### 6.3.3 `infrastructure/sqlite_cdb`

职责：

1. 读取 CDB `datas/texts`
2. 将 CDB 行解码为语义模型
3. 将语义模型编码回 CDB
4. round-trip 测试

说明：

它是唯一的 CDB Adapter，不允许别的模块再理解 CDB 位编码细节。

### 6.3.4 `infrastructure/strings_conf`

职责：

1. 读取 `strings.conf`
2. 写出 `strings.conf`
3. 处理 `system / victory / counter / setname`

### 6.3.5 `infrastructure/assets`

职责：

1. 按规则存储图片和脚本
2. 处理基于 `code` 的资源命名
3. `code` 变化时资源迁移
4. 资源状态扫描

说明：

1. 作者态 `pack` 内部目录固定使用 `scripts/`
2. 运行时导入导出目录固定使用 `script/`
3. `scripts/ <-> script/` 的映射只允许出现在 adapter 层

### 6.3.6 `infrastructure/external_editor`

职责：

1. 调用系统外部编辑器打开 `.lua`
2. 错误转换为统一应用错误

### 6.3.7 `infrastructure/transaction`

职责：

1. 规划多文件操作
2. 临时写入
3. 提交
4. 最佳努力回退
5. 清理中间态
6. 启动残留清理

建议核心接口：

```rust
pub trait TransactionManager {
    fn execute(&self, plan: FileOperationPlan) -> AppResult<TransactionReport>;
}
```

建议计划模型：

```rust
pub struct FileOperationPlan {
    pub tx_id: String,
    pub steps: Vec<FileOperationStep>,
}
```

### 6.3.8 `infrastructure/standard_pack`

职责：

1. 从 `ygopro_path` 加载标准包
2. 构建标准包只读索引
3. 执行标准卡搜索
4. 提供标准包编号存在性查询

## 6.4 Runtime 层

Runtime 层负责进程内状态，而不是磁盘持久化协议。

### 6.4.1 `runtime/sessions`

职责：

1. 当前工作区上下文
2. 当前工作区内已打开自定义 `pack` 的已加载快照
3. 当前已打开 pack tab 顺序
4. 当前 active pack 指针
5. 各已打开 `pack` 的基础读模型入口

建议会话对象：

```rust
pub struct WorkspaceSession {
    pub workspace_id: WorkspaceId,
    pub workspace_meta: WorkspaceMeta,
    pub pack_summaries: Vec<PackOverview>,
    pub open_pack_ids: Vec<PackId>,
    pub active_pack_id: Option<PackId>,
}

pub struct PackSession {
    pub pack_id: PackId,
    pub revision: u64,
    pub source_stamp: String,
    pub metadata: PackMetadata,
    pub cards_by_id: HashMap<CardId, CardEntity>,
    pub code_index: HashMap<u32, CardId>,
    pub card_list_cache: Vec<CardListRow>,
    pub asset_index: HashMap<CardId, CardAssetState>,
    pub strings_index: PackStringsIndex,
}
```

说明：

1. `PackSession` 是从 `metadata.json + cards.json + strings.json + 资源状态` 读取出的运行时快照，不是持久化真相源
2. 首版在打开 `workspace` 时只加载工作区元数据与全部 pack 摘要，不一次性加载全部自定义 `pack`
3. `PackSession` 只在 pack 进入“已打开 tab”状态后才创建
4. 切换已打开 `pack` 只更新 `active_pack_id`，不重新从磁盘读取整包作者态数据
5. 关闭 pack tab 时直接移除对应 `PackSession`
6. 列表语言回退只从 `PackMetadata.display_language_order` 派生
7. 单卡详情页的 `current_edit_language` 只应保留在前端页面草稿态，不进入后端全局 runtime session
8. `revision` 用于表示程序内已知的 pack 版本，每次成功写入后递增
9. `source_stamp` 用于表示加载或刷新时观测到的磁盘状态摘要
10. 手动刷新或重载 pack 后，应更新 `source_stamp` 并使旧 token 失效

### 6.4.2 `runtime/cache`

职责：

1. 基于 `PackSession` 的可丢弃派生结果，例如 `CardListRow`、排序中间结果、搜索中间结果
2. 标准包搜索索引缓存
3. 预检 / 确认 token 临时缓存

说明：

1. 缓存是可丢弃的，不是持久化真相源
2. 首版不引入多级缓存、LRU、后台预热等复杂策略
3. 自定义 `pack` 的基础读模型跟随 `PackSession` 生命周期存在；复杂查询结果可以按需生成、按需丢弃

### 6.4.3 `runtime/index`

职责：

1. 搜索索引
2. 排序索引
3. 编号查重索引

说明：

1. pack 内部的 `code -> id` 只覆盖当前已打开 `pack`
2. 工作区级编号查重不能建立在“所有 `pack` 都已打开”的假设上
3. 首版建议单独维护轻量工作区级 `code` 查重索引，来源可以是扫描全部自定义 `pack` 的 `cards.json`

### 6.4.4 `runtime/jobs`

职责：

1. Job 状态机
2. Job 结果暂存
3. 进度百分比和阶段状态

### 6.4.5 `runtime/events`

职责：

1. 对前端广播 job 进度
2. 广播 pack 数据刷新事件
3. 广播工作区切换事件

## 6.5 Presentation 层

Presentation 是 Tauri 边界层。

它负责：

1. 定义 commands
2. 入参 DTO 解析
3. 调用 application service
4. 将错误转换为前端可消费格式

它不负责：

1. 业务规则
2. 直接访问文件系统
3. 复杂事务编排

建议结构：

```text
presentation/
  commands/
    config_commands.rs
    workspace_commands.rs
    pack_commands.rs
    card_commands.rs
    strings_commands.rs
    resource_commands.rs
    standard_pack_commands.rs
    import_commands.rs
    export_commands.rs
    job_commands.rs
  dto/
  errors/
  events/
```

## 7. 前端模块划分

## 7.1 `app`

职责：

1. 路由
2. 页面布局
3. 全局 Provider
4. App 初始化
5. 错误边界

建议内容：

1. `router.tsx`
2. `AppProviders.tsx`
3. `QueryClientProvider`
4. `I18nProvider`
5. `ModalHost`

## 7.2 `features/settings`

职责：

1. 全局配置页面
2. `ygopro_path` 设置
3. 外部编辑器路径设置
4. 程序语言设置

## 7.3 `features/workspace`

职责：

1. 工作区列表
2. 新建/打开/切换工作区
3. 展示当前工作区 pack 列表
4. pack 排序 UI

## 7.4 `features/pack`

职责：

1. pack 页面壳
2. `CardList` 与 `Strings` tab
3. pack 元数据编辑
4. pack 删除

## 7.5 `features/card`

职责：

1. 单卡详情页
2. 单卡编辑表单
3. 批量编辑弹窗
4. 卡图 / 场地图 / 脚本区块
5. warning 确认流程

## 7.6 `features/strings`

职责：

1. 字符串列表
2. 字符串搜索
3. 新增 / 删除 / 修改字符串
4. 语言切换

## 7.7 `features/import_pack`

职责：

1. 导入向导
2. 导入预检展示
3. 错误/警告/缺失资源展示
4. 提交导入任务

## 7.8 `features/export_bundle`

职责：

1. 导出向导
2. pack 多选
3. 导出语言选择
4. 预检结果展示
5. 提交导出任务

## 7.9 `features/standard_pack`

职责：

1. 标准包只读浏览
2. 标准包搜索
3. 作为参考来源展示

## 7.10 `shared/api`

职责：

1. 封装 Tauri `invoke`
2. 封装事件订阅
3. 转换 command 名称和 DTO

建议接口分组：

```ts
export const configApi = { ... }
export const workspaceApi = { ... }
export const packApi = { ... }
export const cardApi = { ... }
export const stringsApi = { ... }
export const resourceApi = { ... }
export const standardPackApi = { ... }
export const importApi = { ... }
export const exportApi = { ... }
export const jobApi = { ... }
```

## 7.11 `shared/state`

职责：

1. 当前 workspace / pack UI 状态
2. 弹窗状态
3. 表单草稿状态
4. 短期交互状态

建议：

1. 服务端数据缓存使用 TanStack Query
2. 纯 UI 状态使用轻量 store 或 React Context
3. 不要把整个后端 session 复制成前端全局 store

## 7.12 `shared/i18n`

职责：

1. App UI 多语言文案
2. 错误码到文案映射
3. warning 码到文案映射

注意：

`AppI18n` 只能用于程序 UI，不应混入卡片文本或 `pack strings`。

## 8. 关键运行时设计

## 8.1 真相源与派生模型

真相源：

1. `global_config.json`
2. `workspace_registry.json`
3. `workspace.json`
4. `metadata.json`
5. `cards.json`
6. `strings.json`
7. 资源文件

运行时快照：

1. `WorkspaceSession`
2. `PackSession`

派生模型：

1. `CardListRow`
2. `PackOverview`
3. `ImportPreview`
4. `ExportPreview`
5. 标准包搜索索引

### 8.2 Pack 加载策略

推荐策略：

1. 以当前规模作为首版假设：标准包约 `20k` 卡且只读；单个自定义 `pack` 一般在 `0-100` 卡；同时打开的自定义 `pack` 建议不超过 `8`
2. 打开 `workspace` 时，只读取 `workspace.json` 与各 `pack` 的摘要信息
3. 打开 pack tab 时，再读取该 `pack` 的 `metadata.json`、`cards.json`、`strings.json`、资源状态并构建 `PackSession`
4. 切换已打开 `pack` 时只切换当前 active pack 指针，不重新读取磁盘作者态数据
5. 标准包与作者态分开处理，仍通过轻量只读索引提供搜索与编号查重
6. 若检测到外部文件变化或用户手动刷新，则只重载受影响 `pack` 的运行时快照

原因：

1. 当前规模下，按 tab 显式打开/关闭 `pack` 仍能避免复杂缓存系统
2. 运行时成本被限制在“已打开 `pack` 数量”，而不是整个 `workspace` 的总 pack 数
3. 这可以把 pack 切换路径稳定成“切换当前上下文”，同时避免 workspace 打开时的全量读盘
4. 复杂度集中在正确性和事务边界，而不是冷热缓存维护

### 8.3 CardList 性能策略

推荐策略：

1. 每个 `PackSession` 在创建或重载时预先建立基础读模型：`cards_by_id`、`code_index`、`CardListRow`
2. 搜索和排序在后端基于内存中的 `PackSession` 完成
3. 前端列表使用虚拟滚动
4. 图片缩略图继续懒加载，不把图片本体纳入作者态快照
5. 单卡或单 `pack` 修改提交成功后，直接替换受影响 `PackSession`，并重建该 `pack` 的基础派生结果
6. 首版不为自定义 `pack` 引入额外的多级查询缓存

### 8.4 标准包只读索引策略

推荐策略：

1. 标准包接入后构建只读搜索索引
2. 索引用于标准卡搜索和编号存在性查询
3. 索引可缓存在应用数据目录
4. 当 `ygopro_path` 或相关文件时间戳变化时触发重建

这是一个重大决策，详见第 12 节。

## 9. 事务与安全写入设计

## 9.1 基本原则

所有程序内文件写入必须满足：

1. 单文件写入要么新内容完整成功，要么保持旧内容
2. 多文件操作失败时尽量回到旧状态或至少保持可恢复状态
3. 操作对象标识以 `CardEntity.id` 为准
4. 外部编辑器直接修改脚本不纳入程序内安全写入范围
5. 单 `pack` 内写操作应统一由 `pack` 负责组织安全写入
6. 统一编排不等于每次都重写整个 `pack`
7. 首版不承诺完整的崩溃可恢复原子事务

## 9.2 单文件写入

建议流程：

```text
old file
  -> write temp file
  -> flush / sync
  -> atomic rename replace
```

适用：

1. `workspace.json`
2. `metadata.json`
3. `cards.json`
4. `strings.json`

## 9.3 多文件最佳努力提交

这里的“多文件最佳努力提交”在首版中不应被理解为严格原子事务。

典型场景：

1. 单卡 `code` 修改
2. 批量移动卡片
3. 导入 pack
4. 导出 bundle
5. 删除 pack

pack 级最小提交原则：

1. `pack` 是逻辑写边界，不是物理文件写入粒度
2. 事务计划应只包含本次变更真正受影响的文件
3. `metadata.json` 作为 pack 更新时间承载文件，会在大多数写操作中一并更新
4. `cards.json`、`strings.json`、资源文件只在对应内容脏时进入事务计划

首版保证级别：

1. 保证单文件安全写入
2. 多文件提交失败时执行最佳努力回退与清理
3. 若无法完全回退，允许留下可恢复状态
4. 程序启动时应清理临时残留，并对受影响 `pack` 做一致性检查

建议流程：

```text
Precheck
  -> Build Plan
  -> Stage temp outputs
  -> Commit files in deterministic order
  -> Refresh runtime cache
  -> On failure attempt best-effort revert/cleanup
  -> Leave recoverable state if full revert is impossible
```

## 9.4 `code` 变更的一致性处理

单卡保存修改 `code` 时，必须视为一个完整程序内操作：

1. 更新 `cards.json`
2. 若存在主卡图，迁移 `pics/<old>.jpg -> pics/<new>.jpg`
3. 若存在场地图，迁移 `pics/field/<old>.jpg -> pics/field/<new>.jpg`
4. 若存在脚本，迁移 `scripts/c<old>.lua -> scripts/c<new>.lua`
5. 更新 pack session 索引

任一步失败，都应尝试最佳努力回退；若无法完全回退，也应保证主 JSON 文件可恢复且资源状态可重新扫描。

## 10. 长任务与事件设计

## 10.1 为什么需要 Job 系统

以下操作可能耗时较长：

1. 导入 CDB + 资源
2. 多 pack 导出
3. 标准包索引构建
4. 大量资源复制

如果全部做成阻塞式 `invoke`：

1. UI 难以展示稳定进度
2. 用户难以区分“卡住”和“处理中”
3. 错误恢复与结果展示会变乱

## 10.2 Job 状态模型

建议模型：

```rust
pub enum JobStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

pub struct JobProgress {
    pub job_id: String,
    pub status: JobStatus,
    pub stage: String,
    pub progress_percent: Option<u8>,
    pub message: Option<String>,
}
```

## 10.3 事件接口

建议事件：

1. `job:progress`
2. `job:finished`
3. `workspace:changed`
4. `pack:changed`
5. `standard-pack:index-updated`

## 11. API 设计建议

## 11.1 API 风格

建议采用：

1. 查询与写入分离
2. 长任务与短任务分离
3. warning 确认流统一建模
4. 错误码稳定化
5. 对外边界使用 DTO，对内边界使用领域模型或端口模型

## 11.2 统一结果结构

建议短命令统一返回：

```ts
type ApiResult<T> =
  | { ok: true; data: T }
  | { ok: false; error: AppErrorDto };
```

建议写命令统一返回：

```ts
type WriteResult<T> =
  | { status: "ok"; data: T }
  | {
      status: "needs_confirmation";
      confirmation_token: string;
      warnings: ValidationIssueDto[];
      preview?: unknown;
    };
```

对于导入/导出预检，建议单独使用预检结果模型：

```ts
interface PreviewResult<T> {
  preview_token: string;
  snapshot_hash: string;
  expires_at: string;
  data: T;
}
```

边界模型建议：

1. 列表页、详情页、表单输入都定义独立 DTO，即使字段和 `CardEntity` 高度相似
2. 稳定的小值对象可以复用，例如 `PackId`、`CardId`、`LanguageCode`
3. 不允许 `CardDetailDto.card = CardEntity` 这种“DTO 外壳里直接塞整个领域实体”的设计
4. 不允许 `repository` 或 `gateway` 直接返回前端 DTO；它们应返回 `domain model` 或专门的 `port model`
5. `presentation/dto` 可以镜像 `application/dto`，但不应镜像 `domain` 内部结构

## 11.3 建议的 Tauri Commands

### 11.3.1 Config Commands

```text
get_global_config
update_global_config
validate_ygopro_path
```

### 11.3.2 Workspace Commands

```text
list_recent_workspaces
create_workspace
open_workspace
close_workspace
reorder_packs
```

### 11.3.3 Pack Commands

```text
create_pack
update_pack_metadata
delete_pack
open_pack
get_pack_overview
```

### 11.3.4 Card Commands

```text
list_cards
get_card
suggest_next_code
create_card
update_card
delete_cards
batch_patch_cards
move_cards
confirm_card_write
```

### 11.3.5 Strings Commands

```text
list_pack_strings
upsert_pack_string
delete_pack_strings
confirm_pack_strings_write
```

### 11.3.6 Resource Commands

```text
import_main_image
delete_main_image
import_field_image
delete_field_image
create_empty_script
import_script
delete_script
open_script_external
```

### 11.3.7 Standard Pack Commands

```text
get_standard_pack_status
rebuild_standard_pack_index
search_standard_cards
```

### 11.3.8 Import / Export Commands

```text
preview_import_pack
execute_import_pack
preview_export_bundle
execute_export_bundle
get_job_status
list_active_jobs
```

说明：

1. `execute_import_pack` 和 `execute_export_bundle` 的核心输入应为 `preview_token`
2. 若后续需要中止长任务，再补充 `cancel_job`

## 11.4 前端 API 包装层

建议 TS API 形式：

```ts
export const cardApi = {
  listCards(input: ListCardsInput) {
    return invokeApi<CardListPageDto>("list_cards", input);
  },
  getCard(input: GetCardInput) {
    return invokeApi<CardDetailDto>("get_card", input);
  },
  createCard(input: CreateCardInput) {
    return invokeApi<WriteResult<CardDetailDto>>("create_card", input);
  },
  updateCard(input: UpdateCardInput) {
    return invokeApi<WriteResult<CardDetailDto>>("update_card", input);
  },
};
```

## 11.5 错误与 Warning 的接口约束

建议前端永远只消费：

1. 稳定错误码
2. 稳定 warning 码
3. 参数化消息字段

不要让后端直接返回只适合展示的自然语言长句作为唯一语义。

建议 DTO：

```ts
interface AppErrorDto {
  code: string;
  message: string;
  details?: Record<string, unknown>;
}

interface ValidationIssueDto {
  code: string;
  level: "error" | "warning";
  target: string;
  params: Record<string, unknown>;
}
```

## 12. 已决策架构项

本节记录首版已冻结的关键架构决策。

## 12.1 决策 A：长任务是否引入 Job 系统

方案 A1：全部使用同步 `invoke`

优点：

1. 实现更简单
2. 首版更快落地

缺点：

1. 导入导出体验差
2. 难做稳定进度
3. 错误恢复和结果展示会更混乱

方案 A2：仅对长任务引入 Job 系统

优点：

1. 复杂度可控
2. 导入导出体验明显更好
3. 对未来扩展更稳

缺点：

1. 要额外设计任务状态和事件

结论：

采用 A2。只把“标准包索引构建、导入、导出”放进 Job 系统，其余命令保持同步。

## 12.2 决策 B：Warning 确认流是 `force=true` 还是 `confirmation_token`

方案 B1：前端再次调用同一命令并带 `force: true`

优点：

1. API 简单

缺点：

1. 容易误确认过期状态
2. 批量写入和导入场景更危险
3. 前后端约束不够明确

方案 B2：后端先返回 `confirmation_token`，前端确认后单独调用 `confirm_*`

优点：

1. 明确绑定一次具体预检结果
2. 更安全
3. 更适合批量和长任务

缺点：

1. API 略复杂

结论：

采用 B2。YGOCMG 文件操作重，`confirmation_token` 更稳。

补充收敛：

1. 普通写命令使用 `confirmation_token`
2. 导入和导出使用 `preview_token`
3. 两类 token 都必须绑定一次具体预检快照

## 12.3 决策 C：标准包索引策略

方案 C1：每次搜索都直接查 YGOPro CDB

优点：

1. 实现简单
2. 不需要额外缓存

缺点：

1. 搜索体验可能不稳定
2. 标准卡包只读浏览性能较差
3. 编号冲突检测会频繁触盘

方案 C2：接入时构建只读索引，并按文件变化重建

优点：

1. 搜索稳定
2. 编号冲突检测快
3. 后续标准包浏览能力更容易扩展

缺点：

1. 多一层缓存管理

结论：

采用 C2，但只建立轻量摘要索引，不做复杂全文检索引擎。

## 12.4 决策 D：外部编辑器回写监听

方案 D1：只提供“打开外部编辑器”，不监听改动

优点：

1. 最简单
2. 平台兼容性风险低

缺点：

1. 用户改完脚本后，程序内状态可能滞后
2. 切回应用时需要手动刷新

方案 D2：对当前 active pack 的 `scripts/` 建立文件监听

优点：

1. 体验更顺
2. 卡片详情中的脚本状态可自动刷新

缺点：

1. 文件监听需要额外处理抖动、重复事件、跨平台差异

结论：

首版采用 D1。只提供“打开外部编辑器”和“手动刷新脚本状态”，不引入持续文件监听。

理由：

1. 首版优先收敛跨平台复杂度
2. 外部编辑器改动本就不纳入程序内事务
3. 手动刷新已足够覆盖 v1 核心流程

## 12.5 决策 E：前后端共享类型是否上代码生成

方案 E1：手写 TS 类型，与 Rust DTO 保持同步

优点：

1. 工具链简单
2. 更容易理解

缺点：

1. 类型漂移风险高
2. 复杂 DTO 多时维护成本上升

方案 E2：引入 DTO 类型生成工具

优点：

1. 边界更稳定
2. 减少手写重复

缺点：

1. 增加构建复杂度

我的建议：

首版可以先手写 DTO，但只允许对 `presentation/dto` 这一层镜像，不直接镜像内部 domain 类型。等 API 稳定后再考虑生成。

这里暂不作为阻塞决策。

## 13. 分阶段落地建议

建议实现顺序：

1. 先搭骨架：目录、分层、AppState、ports、commands 空实现
2. 先落 Domain：`config/workspace/pack/card/strings/resource`
3. 再落作者态 JSON 仓储与安全写入
4. 再落 Workspace / Pack / Card 核心用例
5. 再落 `CardListRow` 缓存与列表查询
6. 再落资源管理
7. 再落标准包只读索引
8. 再落导入预检与导入执行
9. 再落导出预检与导出执行
10. 再落 Job 系统
11. 最后补端到端测试

如果你想更保守，也可以把第 7 步放后，但第 10 步已经是首版冻结方案的一部分，不建议再延期到架构之外。

## 14. 最终建议

YGOCMG 首版最适合的不是“前端拼文件 + 后端做薄壳”，而是：

1. Rust 后端持有完整业务真相源
2. 前端按 feature 组织交互
3. 外部格式通过 Adapter 隔离
4. 作者态 JSON 作为首版持久化主格式
5. 用“workspace 摘要 + 已打开 pack 的运行时快照 + 简单派生缓存”支撑性能，而不是多级热缓存体系
6. 用 Transaction Plan 支撑单文件安全写入与多文件最佳努力提交

如果这套方案成立，下一步最值得继续细化的不是 UI，而是：

1. Rust `presentation/application/domain/infrastructure/runtime` 的空目录骨架
2. 核心 DTO 与 command 名单
3. `TransactionManager` 与 `PackSession` 的最小实现草案
4. 首批 3 个端到端用例的时序设计
