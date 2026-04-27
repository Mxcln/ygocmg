# YGOCMG 首版接口设计文档 v1

日期：2026-04-25  
状态：Draft  
适用范围：YGOCMG 首版（v1）

关联文档：
- [项目粗略设计方案](./ygocmg.md)
- [YGOCMG 首版功能规范 v1](./ygocmg_v1_functional_spec_2026-04-25.md)
- [YGOCMG 架构与模块职责分析报告](./ygocmg_architecture_module_report_2026-04-25.md)
- [YGOCMG 首版实现文档 v1](./ygocmg_v1_implementation_plan_2026-04-25.md)
- [卡片数据模型语义化重构方案 v2](./card_data_model_refactor_v2_2026-04-23.md)

## 1. 文档目标

本文档用于把 YGOCMG 首版的“公开边界”冻结到可以直接搭代码骨架的粒度。

本文档重点冻结：

1. 模块划分
2. 推荐文件组织
3. 公开 trait / service / command / DTO
4. 关键函数签名
5. 事件 payload
6. JSON 文件协议

本文档不冻结：

1. 私有 helper 函数
2. 每个模块内部的具体算法实现
3. 具体缓存结构的最终选型
4. 某些实现细节的最终 crate 选型

换句话说，本文档的目标不是“把所有代码先写成伪代码”，而是“把所有稳定接口先定下来”。

## 2. 粒度约定

本接口设计文档采用以下粒度：

1. `domain`：冻结核心模型、纯函数入口、公开规则函数
2. `application`：冻结 service trait、ports、input/output DTO
3. `infrastructure`：冻结 adapter trait 和关键实现入口
4. `runtime`：冻结 session、job、event 的公开结构

补充约定：

1. `runtime session` 在本文件中表示后端根据真相源加载出的运行时快照，不表示独立业务真相源
2. 所有写接口仍以显式输入 DTO 为准，不依赖“当前 active pack”这类隐式上下文完成写入定位
3. `runtime cache` 只表示可丢弃的读模型缓存
4. `application dto` 是对外边界合同，不直接暴露完整 `domain entity`
5. 真相源仓储可直接读写 `domain model`；跨边界 port 优先返回专门 `port model`
5. `presentation`：冻结 Tauri command 名称、输入输出 DTO、错误返回形状
6. `frontend`：冻结 `shared/api`、事件订阅接口、页面消费的 contract

特别说明：

1. Rust 内部私有实现允许后续根据开发情况调整
2. 文件数可以比本文档略多，但不应少掉本文档定义的公开边界
3. 若后续接口要修改，应优先更新本文档而不是直接改代码

## 3. 命名与设计约定

### 3.1 Rust 命名

1. 领域模型使用 `Entity`、`Metadata`、`File`、`Entry`、`State`、`Summary`
2. 输入 DTO 统一使用 `*Input`
3. 输出 DTO 统一使用 `*Dto`
4. 预检结果统一使用 `*PreviewDto`
5. 确认流 token 类型统一显式命名，不使用裸 `String`
6. 纯规则函数优先使用动词开头，例如 `normalize_card`、`validate_pack_strings`

### 3.2 TypeScript 命名

1. 前端 contract 与 Rust DTO 保持同名
2. API 包装使用 camelCase
3. command 名称保持 snake_case，与 Tauri command 一致
4. 前端错误消费只依赖稳定错误码和 warning 码

### 3.3 同步与异步约定

1. `domain` 函数全部同步
2. `application` service trait 默认同步定义
3. `presentation` command 使用 `async fn`
4. 前端 API 全部返回 `Promise`

### 3.4 公开结果约定

Rust 侧建议：

```rust
pub type AppResult<T> = Result<T, AppError>;
pub type CommandResult<T> = Result<T, AppErrorDto>;
```

前端侧建议：

```ts
export type ApiResult<T> =
  | { ok: true; data: T }
  | { ok: false; error: AppErrorDto };
```

写操作返回：

```ts
export type WriteResult<T> =
  | { status: "ok"; data: T }
  | {
      status: "needs_confirmation";
      confirmation_token: ConfirmationToken;
      warnings: ValidationIssueDto[];
      preview?: unknown;
    };
```

导入导出预检返回：

```ts
export interface PreviewResult<T> {
  preview_token: PreviewToken;
  snapshot_hash: string;
  expires_at: string;
  data: T;
}
```

## 4. 推荐代码目录与文件

## 4.1 后端目录

```text
src-tauri/src/
  bootstrap/
    mod.rs
    app_state.rs
    wiring.rs

  domain/
    common/
      mod.rs
      ids.rs
      time.rs
      error.rs
      issue.rs
      paging.rs
    config/
      mod.rs
      model.rs
      rules.rs
    workspace/
      mod.rs
      model.rs
      rules.rs
    pack/
      mod.rs
      model.rs
      summary.rs
    card/
      mod.rs
      model.rs
      code.rs
      normalize.rs
      validate.rs
      derive.rs
      patch.rs
    strings/
      mod.rs
      model.rs
      validate.rs
    resource/
      mod.rs
      model.rs
      path_rules.rs
    import/
      mod.rs
      model.rs
    export/
      mod.rs
      model.rs

  application/
    dto/
      mod.rs
      common.rs
      config.rs
      workspace.rs
      pack.rs
      card.rs
      strings.rs
      resource.rs
      standard_pack.rs
      import.rs
      export.rs
      job.rs
    ports/
      mod.rs
      config_repository.rs
      workspace_registry_repository.rs
      workspace_repository.rs
      pack_repository.rs
      asset_repository.rs
      standard_pack_repository.rs
      cdb_gateway.rs
      strings_conf_gateway.rs
      external_editor_gateway.rs
      transaction_manager.rs
      event_bus.rs
      job_scheduler.rs
      clock.rs
      id_generator.rs
    config/
      mod.rs
      service.rs
    workspace/
      mod.rs
      service.rs
    pack/
      mod.rs
      service.rs
    card/
      mod.rs
      query_service.rs
      write_service.rs
      confirmation_service.rs
    strings/
      mod.rs
      service.rs
      confirmation_service.rs
    resource/
      mod.rs
      service.rs
    standard_pack/
      mod.rs
      service.rs
    import/
      mod.rs
      service.rs
    export/
      mod.rs
      service.rs
    jobs/
      mod.rs
      service.rs

  infrastructure/
    fs/
      mod.rs
      local_fs.rs
    json_store/
      mod.rs
      config_store.rs
      workspace_registry_store.rs
      workspace_store.rs
      pack_store.rs
    sqlite_cdb/
      mod.rs
      reader.rs
      writer.rs
    strings_conf/
      mod.rs
      reader.rs
      writer.rs
    assets/
      mod.rs
      asset_store.rs
      image_ops.rs
    external_editor/
      mod.rs
      shell_editor.rs
    transaction/
      mod.rs
      planner.rs
      executor.rs
    standard_pack/
      mod.rs
      repository.rs
      index_builder.rs
      search.rs

  runtime/
    sessions/
      mod.rs
      workspace_session.rs
      pack_session.rs
      session_manager.rs
    cache/
      mod.rs
      preview_token_cache.rs
      confirmation_cache.rs
    index/
      mod.rs
      card_index.rs
      strings_index.rs
    jobs/
      mod.rs
      job_store.rs
    events/
      mod.rs
      app_event.rs

  presentation/
    commands/
      mod.rs
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
      mod.rs
      common.rs
      config.rs
      workspace.rs
      pack.rs
      card.rs
      strings.rs
      resource.rs
      standard_pack.rs
      import.rs
      export.rs
      job.rs
    errors/
      mod.rs
      app_error_dto.rs
      mapper.rs
    events/
      mod.rs
      payloads.rs
```

## 4.2 前端目录

```text
src/
  app/
    App.tsx
    router.tsx
    AppProviders.tsx

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
      invokeApi.ts
      configApi.ts
      workspaceApi.ts
      packApi.ts
      cardApi.ts
      stringsApi.ts
      resourceApi.ts
      standardPackApi.ts
      importApi.ts
      exportApi.ts
      jobApi.ts
      events.ts
    contracts/
      common.ts
      config.ts
      workspace.ts
      pack.ts
      card.ts
      strings.ts
      resource.ts
      standardPack.ts
      import.ts
      export.ts
      job.ts
    state/
    i18n/
```

## 5. 共享基础类型

## 5.1 `domain/common/ids.rs`

```rust
pub type WorkspaceId = String;
pub type PackId = String;
pub type CardId = String;
pub type JobId = String;
pub type ConfirmationToken = String;
pub type PreviewToken = String;
pub type LanguageCode = String;
```

## 5.2 `domain/common/time.rs`

```rust
pub type AppTimestamp = chrono::DateTime<chrono::Utc>;
```

## 5.3 `domain/common/error.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub details: std::collections::BTreeMap<String, serde_json::Value>,
}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self;
    pub fn with_detail(self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self;
}
```

## 5.4 `domain/common/issue.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueLevel {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationTarget {
    pub scope: String,
    pub entity_id: Option<String>,
    pub field: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub code: String,
    pub level: IssueLevel,
    pub target: ValidationTarget,
    pub params: std::collections::BTreeMap<String, serde_json::Value>,
}
```

## 5.5 `domain/common/paging.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRequest {
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageResponse<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
}
```

## 6. Domain 层接口

## 6.1 `domain/config`

### 6.1.1 `domain/config/model.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub app_language: LanguageCode,
    pub ygopro_path: Option<std::path::PathBuf>,
    pub external_text_editor_path: Option<std::path::PathBuf>,
    pub custom_code_recommended_min: u32,
    pub custom_code_recommended_max: u32,
    pub custom_code_min_gap: u32,
    pub shell_sidebar_width: u32,
    pub shell_window_width: u32,
    pub shell_window_height: u32,
    pub shell_window_is_maximized: bool,
}
```

### 6.1.2 `domain/config/rules.rs`

```rust
pub fn default_global_config() -> GlobalConfig;
pub fn validate_global_config(config: &GlobalConfig) -> Vec<ValidationIssue>;
pub fn validate_code_policy(config: &GlobalConfig) -> Vec<ValidationIssue>;
```

## 6.2 `domain/workspace`

### 6.2.1 `domain/workspace/model.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMeta {
    pub id: WorkspaceId,
    pub name: String,
    pub description: Option<String>,
    pub created_at: AppTimestamp,
    pub updated_at: AppTimestamp,
    pub pack_order: Vec<PackId>,
    pub last_opened_pack_id: Option<PackId>,
    pub open_pack_ids: Vec<PackId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRegistryFile {
    pub schema_version: u32,
    pub workspaces: Vec<WorkspaceRegistryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRegistryEntry {
    pub workspace_id: WorkspaceId,
    pub path: std::path::PathBuf,
    pub name_cache: Option<String>,
    pub last_opened_at: Option<AppTimestamp>,
}
```

### 6.2.2 `domain/workspace/rules.rs`

```rust
pub fn validate_workspace_meta(meta: &WorkspaceMeta) -> Vec<ValidationIssue>;
pub fn reorder_pack_ids(current: &[PackId], target_order: &[PackId]) -> Result<Vec<PackId>, AppError>;
pub fn touch_workspace(meta: &WorkspaceMeta, now: AppTimestamp) -> WorkspaceMeta;
```

## 6.3 `domain/pack`

### 6.3.1 `domain/pack/model.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PackKind {
    Standard,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackMetadata {
    pub id: PackId,
    pub kind: PackKind,
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: Option<String>,
    pub created_at: AppTimestamp,
    pub updated_at: AppTimestamp,
    pub display_language_order: Vec<LanguageCode>,
    pub default_export_language: Option<LanguageCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackOverview {
    pub id: PackId,
    pub kind: PackKind,
    pub name: String,
    pub author: String,
    pub version: String,
    pub card_count: usize,
    pub updated_at: AppTimestamp,
}
```

### 6.3.2 `domain/pack/summary.rs`

```rust
pub fn validate_pack_metadata(metadata: &PackMetadata) -> Vec<ValidationIssue>;
pub fn derive_pack_overview(metadata: &PackMetadata, card_count: usize) -> PackOverview;
pub fn touch_pack_metadata(metadata: &PackMetadata, now: AppTimestamp) -> PackMetadata;
```

## 6.4 `domain/card`

### 6.4.1 `domain/card/model.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrimaryType {
    Monster,
    Spell,
    Trap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardTexts {
    pub name: String,
    pub desc: String,
    pub strings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pendulum {
    pub left_scale: i32,
    pub right_scale: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkData {
    pub markers: Vec<LinkMarker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardEntity {
    pub id: CardId,
    pub code: u32,
    pub alias: u32,
    pub setcode: u64,
    pub ot: Ot,
    pub category: u64,
    pub primary_type: PrimaryType,
    pub texts: std::collections::BTreeMap<LanguageCode, CardTexts>,
    pub monster_flags: Option<Vec<MonsterFlag>>,
    pub atk: Option<i32>,
    pub def: Option<i32>,
    pub race: Option<Race>,
    pub attribute: Option<Attribute>,
    pub level: Option<i32>,
    pub pendulum: Option<Pendulum>,
    pub link: Option<LinkData>,
    pub spell_subtype: Option<SpellSubtype>,
    pub trap_subtype: Option<TrapSubtype>,
    pub created_at: AppTimestamp,
    pub updated_at: AppTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardUpdateInput {
    pub code: u32,
    pub alias: u32,
    pub setcode: u64,
    pub ot: Ot,
    pub category: u64,
    pub primary_type: PrimaryType,
    pub texts: std::collections::BTreeMap<LanguageCode, CardTexts>,
    pub monster_flags: Option<Vec<MonsterFlag>>,
    pub atk: Option<i32>,
    pub def: Option<i32>,
    pub race: Option<Race>,
    pub attribute: Option<Attribute>,
    pub level: Option<i32>,
    pub pendulum: Option<Pendulum>,
    pub link: Option<LinkData>,
    pub spell_subtype: Option<SpellSubtype>,
    pub trap_subtype: Option<TrapSubtype>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatchValue<T> {
    Set(T),
    Clear,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BulkCardPatch {
    pub primary_type: Option<PatchValue<PrimaryType>>,
    pub monster_flags: Option<PatchValue<Vec<MonsterFlag>>>,
    pub spell_subtype: Option<PatchValue<SpellSubtype>>,
    pub trap_subtype: Option<PatchValue<TrapSubtype>>,
    pub atk: Option<PatchValue<i32>>,
    pub def: Option<PatchValue<i32>>,
    pub race: Option<PatchValue<Race>>,
    pub attribute: Option<PatchValue<Attribute>>,
    pub level: Option<PatchValue<i32>>,
    pub setcode: Option<PatchValue<u64>>,
    pub ot: Option<PatchValue<Ot>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardListRow {
    pub id: CardId,
    pub code: u32,
    pub name: String,
    pub desc: String,
    pub primary_type: PrimaryType,
    pub atk: Option<i32>,
    pub def: Option<i32>,
    pub level: Option<i32>,
    pub has_image: bool,
    pub has_script: bool,
    pub has_field_image: bool,
}
```

### 6.4.2 `domain/card/code.rs`

```rust
#[derive(Debug, Clone)]
pub struct CodePolicy {
    pub reserved_max: u32,
    pub recommended_min: u32,
    pub recommended_max: u32,
    pub hard_max: u32,
    pub min_gap: u32,
}

#[derive(Debug, Clone)]
pub struct CodeValidationContext {
    pub policy: CodePolicy,
    pub workspace_custom_codes: std::collections::BTreeSet<u32>,
    pub standard_codes: std::collections::BTreeSet<u32>,
}

pub fn validate_card_code(code: u32, ctx: &CodeValidationContext) -> Vec<ValidationIssue>;
pub fn suggest_next_code(ctx: &CodeValidationContext, preferred_start: Option<u32>) -> Option<u32>;
```

### 6.4.3 `domain/card/normalize.rs`

```rust
pub fn normalize_card_input(input: CardUpdateInput) -> CardUpdateInput;
pub fn create_card_entity(
    new_id: CardId,
    input: CardUpdateInput,
    now: AppTimestamp,
) -> CardEntity;
pub fn apply_card_update(
    existing: &CardEntity,
    input: CardUpdateInput,
    now: AppTimestamp,
) -> CardEntity;
```

### 6.4.4 `domain/card/validate.rs`

```rust
pub fn validate_card_structure(card: &CardEntity) -> Vec<ValidationIssue>;
pub fn validate_card_update_input(input: &CardUpdateInput) -> Vec<ValidationIssue>;
pub fn collect_card_warnings(
    card: &CardEntity,
    code_ctx: &CodeValidationContext,
) -> Vec<ValidationIssue>;
```

### 6.4.5 `domain/card/derive.rs`

```rust
pub fn derive_card_list_row(
    card: &CardEntity,
    asset_state: &crate::domain::resource::model::CardAssetState,
    display_language_order: &[LanguageCode],
) -> CardListRow;

pub fn resolve_display_texts<'a>(
    texts: &'a std::collections::BTreeMap<LanguageCode, CardTexts>,
    display_language_order: &[LanguageCode],
) -> Option<&'a CardTexts>;
```

### 6.4.6 `domain/card/patch.rs`

```rust
pub fn apply_bulk_patch(
    existing: &CardEntity,
    patch: &BulkCardPatch,
    now: AppTimestamp,
) -> CardEntity;
```

## 6.5 `domain/strings`

### 6.5.1 `domain/strings/model.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PackStringKind {
    System,
    Victory,
    Counter,
    Setname,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackStringEntry {
    pub kind: PackStringKind,
    pub key: u32,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackStringsFile {
    pub schema_version: u32,
    pub entries: Vec<PackStringRecord>,
}
```

### 6.5.2 `domain/strings/validate.rs`

```rust
pub fn validate_pack_strings(file: &PackStringsFile) -> Vec<ValidationIssue>;
pub fn validate_pack_strings_language(
    language: &LanguageCode,
    entries: &[PackStringEntry],
) -> Vec<ValidationIssue>;
```

## 6.6 `domain/resource`

### 6.6.1 `domain/resource/model.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceKind {
    MainImage,
    FieldImage,
    Script,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CardAssetState {
    pub has_image: bool,
    pub has_field_image: bool,
    pub has_script: bool,
}
```

### 6.6.2 `domain/resource/path_rules.rs`

```rust
pub fn main_image_relative_path(code: u32) -> std::path::PathBuf;
pub fn field_image_relative_path(code: u32) -> std::path::PathBuf;
pub fn script_relative_path(code: u32) -> std::path::PathBuf;
pub fn can_have_field_image(card: &crate::domain::card::model::CardEntity) -> bool;
```

## 6.7 `domain/import`

### 6.7.1 `domain/import/model.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreview {
    pub target_pack_name: String,
    pub card_count: usize,
    pub warning_count: usize,
    pub error_count: usize,
    pub missing_main_image_count: usize,
    pub missing_script_count: usize,
    pub missing_field_image_count: usize,
    pub issues: Vec<ValidationIssue>,
}
```

## 6.8 `domain/export`

### 6.8.1 `domain/export/model.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPreview {
    pub pack_count: usize,
    pub card_count: usize,
    pub main_image_count: usize,
    pub field_image_count: usize,
    pub script_count: usize,
    pub warning_count: usize,
    pub error_count: usize,
    pub issues: Vec<ValidationIssue>,
}
```

## 7. Application DTO 接口

## 7.1 `application/dto/common.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppErrorDto {
    pub code: String,
    pub message: String,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssueDto {
    pub code: String,
    pub level: String,
    pub target: String,
    pub params: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WriteResultDto<T> {
    Ok {
        data: T,
        warnings: Vec<ValidationIssueDto>,
    },
    NeedsConfirmation {
        confirmation_token: ConfirmationToken,
        warnings: Vec<ValidationIssueDto>,
        preview: Option<serde_json::Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewResultDto<T> {
    pub preview_token: PreviewToken,
    pub snapshot_hash: String,
    pub expires_at: AppTimestamp,
    pub data: T,
}
```

说明：

1. 首版对外仍保留按 feature 划分的写 DTO
2. 这些写 DTO 在应用层内部应统一收敛为 pack 级写编排命令
3. `snapshot_hash` 在首版可用于承载 `revision + source_stamp` 的组合摘要

## 7.2 `application/dto/config.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfigDto {
    pub app_language: LanguageCode,
    pub ygopro_path: Option<std::path::PathBuf>,
    pub external_text_editor_path: Option<std::path::PathBuf>,
    pub custom_code_recommended_min: u32,
    pub custom_code_recommended_max: u32,
    pub custom_code_min_gap: u32,
    pub shell_sidebar_width: u32,
    pub shell_window_width: u32,
    pub shell_window_height: u32,
    pub shell_window_is_maximized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGlobalConfigInput {
    pub app_language: LanguageCode,
    pub ygopro_path: Option<std::path::PathBuf>,
    pub external_text_editor_path: Option<std::path::PathBuf>,
    pub custom_code_recommended_min: u32,
    pub custom_code_recommended_max: u32,
    pub custom_code_min_gap: u32,
    pub shell_sidebar_width: u32,
    pub shell_window_width: u32,
    pub shell_window_height: u32,
    pub shell_window_is_maximized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateYgoProPathInput {
    pub ygopro_path: std::path::PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YgoProPathCheckResultDto {
    pub exists: bool,
    pub has_cards_cdb: bool,
    pub has_script_dir: bool,
    pub has_pics_dir: bool,
    pub warnings: Vec<ValidationIssueDto>,
}
```

## 7.3 `application/dto/workspace.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMetaDto {
    pub id: WorkspaceId,
    pub name: String,
    pub description: Option<String>,
    pub created_at: AppTimestamp,
    pub updated_at: AppTimestamp,
    pub pack_order: Vec<PackId>,
    pub last_opened_pack_id: Option<PackId>,
    pub open_pack_ids: Vec<PackId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceListItemDto {
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub path: std::path::PathBuf,
    pub last_opened_at: Option<AppTimestamp>,
    pub exists_on_disk: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceInput {
    pub parent_dir: std::path::PathBuf,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenWorkspaceInput {
    pub path: std::path::PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorderPacksInput {
    pub workspace_id: WorkspaceId,
    pub pack_order: Vec<PackId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceOpenedDto {
    pub workspace: WorkspaceMetaDto,
    pub pack_summaries: Vec<crate::application::dto::pack::PackOverviewDto>,
    pub open_pack_ids: Vec<PackId>,
    pub active_pack_id: Option<PackId>,
}
```

## 7.4 `application/dto/pack.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackMetadataDto {
    pub id: PackId,
    pub kind: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: Option<String>,
    pub created_at: AppTimestamp,
    pub updated_at: AppTimestamp,
    pub display_language_order: Vec<LanguageCode>,
    pub default_export_language: Option<LanguageCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackOverviewDto {
    pub id: PackId,
    pub kind: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub card_count: usize,
    pub updated_at: AppTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackSnapshotDto {
    pub overview: PackOverviewDto,
    pub metadata: PackMetadataDto,
    pub card_count: usize,
    pub available_string_languages: Vec<LanguageCode>,
    pub open_pack_ids: Vec<PackId>,
    pub active_pack_id: PackId,
    pub revision: u64,
    pub source_stamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePackInput {
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: Option<String>,
    pub display_language_order: Vec<LanguageCode>,
    pub default_export_language: Option<LanguageCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePackMetadataInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: Option<String>,
    pub display_language_order: Vec<LanguageCode>,
    pub default_export_language: Option<LanguageCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletePackInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenPackInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosePackInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetActivePackInput {
    pub pack_id: PackId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPackInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
}
```

## 7.5 `application/dto/card.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CardSortField {
    Code,
    Name,
    PrimaryType,
    Atk,
    Def,
    Level,
    UpdatedAt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardListRowDto {
    pub id: CardId,
    pub code: u32,
    pub name: String,
    pub desc: String,
    pub primary_type: String,
    pub atk: Option<i32>,
    pub def: Option<i32>,
    pub level: Option<i32>,
    pub has_image: bool,
    pub has_script: bool,
    pub has_field_image: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditableCardDto {
    pub id: CardId,
    pub code: u32,
    pub alias: u32,
    pub setcode: u64,
    pub ot: String,
    pub category: u64,
    pub primary_type: String,
    pub texts: std::collections::BTreeMap<LanguageCode, crate::application::dto::card::CardTextsDto>,
    pub monster_flags: Option<Vec<String>>,
    pub atk: Option<i32>,
    pub def: Option<i32>,
    pub race: Option<String>,
    pub attribute: Option<String>,
    pub level: Option<i32>,
    pub pendulum: Option<crate::application::dto::card::PendulumDto>,
    pub link: Option<crate::application::dto::card::LinkDataDto>,
    pub spell_subtype: Option<String>,
    pub trap_subtype: Option<String>,
    pub created_at: AppTimestamp,
    pub updated_at: AppTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardTextsDto {
    pub name: String,
    pub desc: String,
    pub strings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendulumDto {
    pub left_scale: i32,
    pub right_scale: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkDataDto {
    pub markers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDetailDto {
    pub card: EditableCardDto,
    pub asset_state: crate::application::dto::resource::CardAssetStateDto,
    pub available_languages: Vec<LanguageCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardListPageDto {
    pub items: Vec<CardListRowDto>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListCardsInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub keyword: Option<String>,
    pub sort_by: CardSortField,
    pub sort_direction: SortDirection,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCardInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestCodeInput {
    pub workspace_id: WorkspaceId,
    pub preferred_start: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSuggestionDto {
    pub suggested_code: Option<u32>,
    pub warnings: Vec<ValidationIssueDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCardInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub data: EditableCardInputDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCardInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
    pub data: EditableCardInputDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditableCardInputDto {
    pub code: u32,
    pub alias: u32,
    pub setcode: u64,
    pub ot: String,
    pub category: u64,
    pub primary_type: String,
    pub texts: std::collections::BTreeMap<LanguageCode, CardTextsDto>,
    pub monster_flags: Option<Vec<String>>,
    pub atk: Option<i32>,
    pub def: Option<i32>,
    pub race: Option<String>,
    pub attribute: Option<String>,
    pub level: Option<i32>,
    pub pendulum: Option<PendulumDto>,
    pub link: Option<LinkDataDto>,
    pub spell_subtype: Option<String>,
    pub trap_subtype: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmCardWriteInput {
    pub confirmation_token: ConfirmationToken,
}

说明：

1. `confirmation_token` 的内部记录在首版应至少绑定 `pack_id + revision + 输入快照`
2. 为了处理程序外修改，建议再绑定 `source_stamp`

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteCardsInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_ids: Vec<CardId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteCardsResultDto {
    pub deleted_count: usize,
    pub deleted_ids: Vec<CardId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPatchCardsInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_ids: Vec<CardId>,
    pub patch: CardBulkPatchDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CardBulkPatchDto {
    pub primary_type: Option<PatchValueDto<String>>,
    pub monster_flags: Option<PatchValueDto<Vec<String>>>,
    pub spell_subtype: Option<PatchValueDto<String>>,
    pub trap_subtype: Option<PatchValueDto<String>>,
    pub atk: Option<PatchValueDto<i32>>,
    pub def: Option<PatchValueDto<i32>>,
    pub race: Option<PatchValueDto<String>>,
    pub attribute: Option<PatchValueDto<String>>,
    pub level: Option<PatchValueDto<i32>>,
    pub setcode: Option<PatchValueDto<u64>>,
    pub ot: Option<PatchValueDto<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatchValueDto<T> {
    Set(T),
    Clear,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPatchResultDto {
    pub updated_count: usize,
    pub updated_ids: Vec<CardId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MoveCardsTargetInput {
    ExistingPack { pack_id: PackId },
    NewPack {
        name: String,
        author: String,
        version: String,
        description: Option<String>,
        display_language_order: Vec<LanguageCode>,
        default_export_language: Option<LanguageCode>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveCardsInput {
    pub workspace_id: WorkspaceId,
    pub source_pack_id: PackId,
    pub card_ids: Vec<CardId>,
    pub target: MoveCardsTargetInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveCardsResultDto {
    pub moved_count: usize,
    pub moved_ids: Vec<CardId>,
    pub target_pack_id: PackId,
}
```

## 7.6 `application/dto/strings.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackStringEntryDto {
    pub kind: PackStringKind,
    pub key: u32,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackStringsPageDto {
    pub language: LanguageCode,
    pub items: Vec<PackStringEntryDto>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackStringValueDto {
    pub language: LanguageCode,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackStringRecordDto {
    pub kind: PackStringKind,
    pub key: u32,
    pub values: Vec<PackStringValueDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackStringRecordDetailDto {
    pub record: PackStringRecordDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPackStringsInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub language: LanguageCode,
    pub kind_filter: Option<PackStringKind>,
    pub key_filter: Option<u32>,
    pub keyword: Option<String>,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertPackStringInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub language: LanguageCode,
    pub entry: PackStringEntryDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPackStringInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub kind: PackStringKind,
    pub key: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertPackStringRecordInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub record: PackStringRecordDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletePackStringsInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub entries: Vec<PackStringKeyDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovePackStringTranslationInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub kind: PackStringKind,
    pub key: u32,
    pub language: LanguageCode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackStringKeyDto {
    pub kind: PackStringKind,
    pub key: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletePackStringsResultDto {
    pub deleted_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmPackStringsWriteInput {
    pub confirmation_token: ConfirmationToken,
}
```

## 7.7 `application/dto/resource.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardAssetStateDto {
    pub has_image: bool,
    pub has_field_image: bool,
    pub has_script: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportMainImageInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
    pub source_path: std::path::PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteMainImageInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportFieldImageInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
    pub source_path: std::path::PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteFieldImageInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEmptyScriptInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportScriptInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
    pub source_path: std::path::PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteScriptInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenScriptExternalInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
}
```

## 7.8 `application/dto/standard_pack.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardPackStatusDto {
    pub configured: bool,
    pub ygopro_path: Option<std::path::PathBuf>,
    pub index_exists: bool,
    pub indexed_at: Option<AppTimestamp>,
    pub card_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardCardRowDto {
    pub code: u32,
    pub name: String,
    pub primary_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardCardPageDto {
    pub items: Vec<StandardCardRowDto>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchStandardCardsInput {
    pub keyword: Option<String>,
    pub language_order: Vec<LanguageCode>,
    pub page: u32,
    pub page_size: u32,
}
```

## 7.9 `application/dto/import.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewImportPackInput {
    pub workspace_id: WorkspaceId,
    pub new_pack_name: String,
    pub new_pack_author: String,
    pub new_pack_version: String,
    pub new_pack_description: Option<String>,
    pub display_language_order: Vec<LanguageCode>,
    pub default_export_language: Option<LanguageCode>,
    pub cdb_path: std::path::PathBuf,
    pub pics_dir: Option<std::path::PathBuf>,
    pub field_pics_dir: Option<std::path::PathBuf>,
    pub script_dir: Option<std::path::PathBuf>,
    pub strings_conf_path: Option<std::path::PathBuf>,
    pub source_language: LanguageCode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreviewDto {
    pub target_pack_name: String,
    pub card_count: usize,
    pub warning_count: usize,
    pub error_count: usize,
    pub missing_main_image_count: usize,
    pub missing_script_count: usize,
    pub missing_field_image_count: usize,
    pub issues: Vec<ValidationIssueDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteImportPackInput {
    pub preview_token: PreviewToken,
}
```

说明：

1. `preview_token` 的内部记录在首版应至少绑定目标对象、输入快照、相关 `pack revision`
2. 为了处理程序外修改，建议再绑定相关 `source_stamp`

## 7.10 `application/dto/export.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewExportBundleInput {
    pub workspace_id: WorkspaceId,
    pub pack_ids: Vec<PackId>,
    pub export_language: LanguageCode,
    pub output_dir: std::path::PathBuf,
    pub output_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPreviewDto {
    pub pack_count: usize,
    pub card_count: usize,
    pub main_image_count: usize,
    pub field_image_count: usize,
    pub script_count: usize,
    pub warning_count: usize,
    pub error_count: usize,
    pub issues: Vec<ValidationIssueDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteExportBundleInput {
    pub preview_token: PreviewToken,
}
```

## 7.11 `application/dto/job.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobKindDto {
    StandardPackIndexRebuild,
    ImportPack,
    ExportBundle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobStatusDto {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobAcceptedDto {
    pub job_id: JobId,
    pub kind: JobKindDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSnapshotDto {
    pub job_id: JobId,
    pub kind: JobKindDto,
    pub status: JobStatusDto,
    pub stage: String,
    pub progress_percent: Option<u8>,
    pub message: Option<String>,
    pub started_at: Option<AppTimestamp>,
    pub finished_at: Option<AppTimestamp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetJobStatusInput {
    pub job_id: JobId,
}
```

## 8. Application Ports 接口

## 8.1 `application/ports/config_repository.rs`

```rust
pub trait ConfigRepository: Send + Sync {
    fn load(&self) -> AppResult<crate::domain::config::model::GlobalConfig>;
    fn save(&self, config: &crate::domain::config::model::GlobalConfig) -> AppResult<()>;
}
```

## 8.2 `application/ports/workspace_registry_repository.rs`

```rust
pub trait WorkspaceRegistryRepository: Send + Sync {
    fn load(&self) -> AppResult<crate::domain::workspace::model::WorkspaceRegistryFile>;
    fn save(&self, registry: &crate::domain::workspace::model::WorkspaceRegistryFile) -> AppResult<()>;
}
```

## 8.3 `application/ports/workspace_repository.rs`

```rust
pub trait WorkspaceRepository: Send + Sync {
    fn create_empty(
        &self,
        root: &std::path::Path,
        meta: &crate::domain::workspace::model::WorkspaceMeta,
    ) -> AppResult<()>;

    fn exists(&self, root: &std::path::Path) -> AppResult<bool>;

    fn load_meta(
        &self,
        root: &std::path::Path,
    ) -> AppResult<crate::domain::workspace::model::WorkspaceMeta>;

    fn save_meta(
        &self,
        root: &std::path::Path,
        meta: &crate::domain::workspace::model::WorkspaceMeta,
    ) -> AppResult<()>;
}
```

## 8.4 `application/ports/pack_repository.rs`

```rust
pub trait PackRepository: Send + Sync {
    fn list_pack_ids(&self, workspace_root: &std::path::Path) -> AppResult<Vec<PackId>>;

    fn load_author_state(
        &self,
        workspace_root: &std::path::Path,
        pack_id: &PackId,
    ) -> AppResult<crate::domain::pack::model::PackAuthorState>;

    fn create_empty_pack(
        &self,
        workspace_root: &std::path::Path,
        metadata: &crate::domain::pack::model::PackMetadata,
    ) -> AppResult<()>;

    fn delete_pack(
        &self,
        workspace_root: &std::path::Path,
        pack_id: &PackId,
    ) -> AppResult<()>;

    fn load_metadata(
        &self,
        workspace_root: &std::path::Path,
        pack_id: &PackId,
    ) -> AppResult<crate::domain::pack::model::PackMetadata>;

    fn save_metadata(
        &self,
        workspace_root: &std::path::Path,
        metadata: &crate::domain::pack::model::PackMetadata,
    ) -> AppResult<()>;

    fn load_cards(
        &self,
        workspace_root: &std::path::Path,
        pack_id: &PackId,
    ) -> AppResult<Vec<crate::domain::card::model::CardEntity>>;

    fn save_cards(
        &self,
        workspace_root: &std::path::Path,
        pack_id: &PackId,
        cards: &[crate::domain::card::model::CardEntity],
    ) -> AppResult<()>;

    fn load_strings(
        &self,
        workspace_root: &std::path::Path,
        pack_id: &PackId,
    ) -> AppResult<crate::domain::strings::model::PackStringsFile>;

    fn save_strings(
        &self,
        workspace_root: &std::path::Path,
        pack_id: &PackId,
        strings: &crate::domain::strings::model::PackStringsFile,
    ) -> AppResult<()>;
}

说明：
1. `PackId` 是稳定身份，不要求等于工作区内的物理目录名
2. 工作区内的 pack 路径应通过扫描 `packs/*/metadata.json` 并读取 `metadata.id` 解析
3. 物理目录名可以使用文件系统安全的可读 `storage-name`
```

说明：

1. `load_author_state` 是 pack 级写编排的首选读取入口
2. `save_metadata`、`save_cards`、`save_strings` 仍然保留，用于按最小必要文件集生成落盘步骤
3. 也就是说，逻辑写边界是 `pack`，物理写入粒度仍然是文件

## 8.5 `application/ports/asset_repository.rs`

```rust
#[derive(Debug, Clone)]
pub struct AssetMutationPlan {
    pub steps: Vec<crate::application::ports::transaction_manager::FileOperationStep>,
    pub resulting_state: crate::domain::resource::model::CardAssetState,
}

pub trait AssetRepository: Send + Sync {
    fn scan_card_assets(
        &self,
        workspace_root: &std::path::Path,
        pack_id: &PackId,
        cards: &[crate::domain::card::model::CardEntity],
    ) -> AppResult<std::collections::HashMap<CardId, crate::domain::resource::model::CardAssetState>>;

    fn plan_import_main_image(
        &self,
        workspace_root: &std::path::Path,
        pack_id: &PackId,
        code: u32,
        source_path: &std::path::Path,
    ) -> AppResult<AssetMutationPlan>;

    fn plan_delete_main_image(
        &self,
        workspace_root: &std::path::Path,
        pack_id: &PackId,
        code: u32,
    ) -> AppResult<AssetMutationPlan>;

    fn plan_import_field_image(
        &self,
        workspace_root: &std::path::Path,
        pack_id: &PackId,
        code: u32,
        source_path: &std::path::Path,
    ) -> AppResult<AssetMutationPlan>;

    fn plan_delete_field_image(
        &self,
        workspace_root: &std::path::Path,
        pack_id: &PackId,
        code: u32,
    ) -> AppResult<AssetMutationPlan>;

    fn plan_create_empty_script(
        &self,
        workspace_root: &std::path::Path,
        pack_id: &PackId,
        code: u32,
    ) -> AppResult<AssetMutationPlan>;

    fn plan_import_script(
        &self,
        workspace_root: &std::path::Path,
        pack_id: &PackId,
        code: u32,
        source_path: &std::path::Path,
    ) -> AppResult<AssetMutationPlan>;

    fn plan_delete_script(
        &self,
        workspace_root: &std::path::Path,
        pack_id: &PackId,
        code: u32,
    ) -> AppResult<AssetMutationPlan>;

    fn plan_rename_code_bound_assets(
        &self,
        workspace_root: &std::path::Path,
        pack_id: &PackId,
        old_code: u32,
        new_code: u32,
    ) -> AppResult<Vec<crate::application::ports::transaction_manager::FileOperationStep>>;

    fn plan_move_code_bound_assets(
        &self,
        workspace_root: &std::path::Path,
        source_pack_id: &PackId,
        target_pack_id: &PackId,
        code: u32,
    ) -> AppResult<Vec<crate::application::ports::transaction_manager::FileOperationStep>>;
}
```

## 8.6 `application/ports/standard_pack_repository.rs`

```rust
#[derive(Debug, Clone)]
pub struct StandardPackStatus {
    pub configured: bool,
    pub ygopro_path: Option<std::path::PathBuf>,
    pub index_exists: bool,
    pub indexed_at: Option<AppTimestamp>,
    pub card_count: usize,
}

#[derive(Debug, Clone)]
pub struct StandardCardRow {
    pub code: u32,
    pub name: String,
    pub primary_type: String,
}

#[derive(Debug, Clone)]
pub struct StandardCardPage {
    pub items: Vec<StandardCardRow>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
}

#[derive(Debug, Clone)]
pub struct SearchStandardCardsQuery {
    pub keyword: Option<String>,
    pub language_order: Vec<LanguageCode>,
    pub page: u32,
    pub page_size: u32,
}

pub trait StandardPackRepository: Send + Sync {
    fn get_status(&self) -> AppResult<StandardPackStatus>;

    fn rebuild_index(&self, ygopro_path: &std::path::Path) -> AppResult<StandardPackStatus>;

    fn search_cards(&self, query: &SearchStandardCardsQuery) -> AppResult<StandardCardPage>;

    fn exists_code(&self, code: u32) -> AppResult<bool>;
}
```

## 8.7 `application/ports/cdb_gateway.rs`

```rust
#[derive(Debug, Clone)]
pub struct ReadCdbInput {
    pub cdb_path: std::path::PathBuf,
    pub source_language: LanguageCode,
}

#[derive(Debug, Clone)]
pub struct ImportedCardRecord {
    pub card: crate::domain::card::model::CardEntity,
}

#[derive(Debug, Clone)]
pub struct WriteCdbInput {
    pub cdb_path: std::path::PathBuf,
    pub cards: Vec<crate::domain::card::model::CardEntity>,
    pub export_language: LanguageCode,
}

pub trait CdbGateway: Send + Sync {
    fn read_cards(&self, input: &ReadCdbInput) -> AppResult<Vec<ImportedCardRecord>>;
    fn write_cards(&self, input: &WriteCdbInput) -> AppResult<()>;
}
```

## 8.8 `application/ports/strings_conf_gateway.rs`

```rust
#[derive(Debug, Clone)]
pub struct ReadStringsConfInput {
    pub path: std::path::PathBuf,
    pub source_language: LanguageCode,
}

#[derive(Debug, Clone)]
pub struct WriteStringsConfInput {
    pub path: std::path::PathBuf,
    pub export_language: LanguageCode,
    pub strings: crate::domain::strings::model::PackStringsFile,
}

pub trait StringsConfGateway: Send + Sync {
    fn read_pack_strings(
        &self,
        input: &ReadStringsConfInput,
    ) -> AppResult<crate::domain::strings::model::PackStringsFile>;

    fn write_pack_strings(&self, input: &WriteStringsConfInput) -> AppResult<()>;
}
```

## 8.9 `application/ports/external_editor_gateway.rs`

```rust
pub trait ExternalEditorGateway: Send + Sync {
    fn open_file(
        &self,
        editor_path: Option<&std::path::Path>,
        file_path: &std::path::Path,
    ) -> AppResult<()>;
}
```

## 8.10 `application/ports/transaction_manager.rs`

```rust
#[derive(Debug, Clone)]
pub enum FileOperationStep {
    WriteFile {
        target: std::path::PathBuf,
        content: Vec<u8>,
    },
    CopyFile {
        source: std::path::PathBuf,
        target: std::path::PathBuf,
    },
    MoveFile {
        source: std::path::PathBuf,
        target: std::path::PathBuf,
    },
    DeleteFile {
        target: std::path::PathBuf,
    },
    EnsureDir {
        path: std::path::PathBuf,
    },
}

#[derive(Debug, Clone)]
pub struct FileOperationPlan {
    pub tx_id: String,
    pub steps: Vec<FileOperationStep>,
}

#[derive(Debug, Clone)]
pub struct TransactionReport {
    pub tx_id: String,
    pub applied_steps: usize,
    pub fully_reverted: bool,
}

pub trait TransactionManager: Send + Sync {
    fn execute(&self, plan: FileOperationPlan) -> AppResult<TransactionReport>;
}
```

说明：

1. 首版 `TransactionManager` 负责单文件安全写入与多文件最佳努力提交
2. 它不承诺完整的崩溃可恢复原子事务
3. `fully_reverted` 用于表达失败后的最佳努力回退结果

## 8.10.1 `application/ports/pack_write_port_models.rs`

```rust
#[derive(Debug, Clone)]
pub struct PackMutationPlan {
    pub pack_id: PackId,
    pub metadata_dirty: bool,
    pub cards_dirty: bool,
    pub strings_dirty: bool,
    pub asset_steps: Vec<FileOperationStep>,
}
```

说明：

1. `PackMutationPlan` 是单 `pack` 写编排内部的脏数据摘要
2. 它用于表达“本次应该提交哪些文件”，而不是强迫整包重写
3. `asset_steps` 由 `AssetRepository` 规划，`metadata/cards/strings` 的写入步骤由应用层统一补齐

## 8.11 `application/ports/event_bus.rs`

```rust
#[derive(Debug, Clone)]
pub enum ApplicationEvent {
    JobProgress {
        job_id: JobId,
        status: String,
        stage: String,
        progress_percent: Option<u8>,
        message: Option<String>,
    },
    JobFinished {
        job_id: JobId,
        status: String,
    },
    WorkspaceChanged {
        workspace_id: WorkspaceId,
    },
    PackChanged {
        workspace_id: WorkspaceId,
        pack_id: PackId,
    },
    StandardPackIndexUpdated {
        indexed_at: AppTimestamp,
    },
}

pub trait EventBus: Send + Sync {
    fn publish(&self, event: ApplicationEvent) -> AppResult<()>;
}
```

## 8.12 `application/ports/job_scheduler.rs`

```rust
#[derive(Debug, Clone)]
pub enum JobKind {
    StandardPackIndexRebuild,
    ImportPack,
    ExportBundle,
}

#[derive(Debug, Clone)]
pub enum JobPayload {
    StandardPackIndexRebuild,
    ImportPack { preview_token: PreviewToken },
    ExportBundle { preview_token: PreviewToken },
}

#[derive(Debug, Clone)]
pub struct JobAccepted {
    pub job_id: JobId,
    pub kind: JobKind,
}

pub trait JobScheduler: Send + Sync {
    fn submit(&self, kind: JobKind, payload: JobPayload) -> AppResult<JobAccepted>;
    fn get_status(&self, job_id: &JobId) -> AppResult<crate::runtime::jobs::job_store::JobSnapshot>;
    fn list_active(&self) -> AppResult<Vec<crate::runtime::jobs::job_store::JobSnapshot>>;
}
```

## 8.13 `application/ports/clock.rs`

```rust
pub trait Clock: Send + Sync {
    fn now(&self) -> AppTimestamp;
}
```

## 8.14 `application/ports/id_generator.rs`

```rust
pub trait IdGenerator: Send + Sync {
    fn new_workspace_id(&self) -> WorkspaceId;
    fn new_pack_id(&self) -> PackId;
    fn new_card_id(&self) -> CardId;
    fn new_confirmation_token(&self) -> ConfirmationToken;
    fn new_preview_token(&self) -> PreviewToken;
    fn new_job_id(&self) -> JobId;
}
```

## 9. Application Service 接口

## 9.1 `application/config/service.rs`

```rust
pub trait ConfigService: Send + Sync {
    fn get_global_config(&self) -> AppResult<crate::application::dto::config::GlobalConfigDto>;
    fn update_global_config(
        &self,
        input: crate::application::dto::config::UpdateGlobalConfigInput,
    ) -> AppResult<crate::application::dto::config::GlobalConfigDto>;
    fn validate_ygopro_path(
        &self,
        input: crate::application::dto::config::ValidateYgoProPathInput,
    ) -> AppResult<crate::application::dto::config::YgoProPathCheckResultDto>;
}
```

## 9.2 `application/workspace/service.rs`

```rust
pub trait WorkspaceService: Send + Sync {
    fn list_recent_workspaces(
        &self,
    ) -> AppResult<Vec<crate::application::dto::workspace::WorkspaceListItemDto>>;

    fn create_workspace(
        &self,
        input: crate::application::dto::workspace::CreateWorkspaceInput,
    ) -> AppResult<crate::application::dto::workspace::WorkspaceOpenedDto>;

    fn open_workspace(
        &self,
        input: crate::application::dto::workspace::OpenWorkspaceInput,
    ) -> AppResult<crate::application::dto::workspace::WorkspaceOpenedDto>;

    fn close_workspace(&self) -> AppResult<()>;

    fn reorder_packs(
        &self,
        input: crate::application::dto::workspace::ReorderPacksInput,
    ) -> AppResult<crate::application::dto::workspace::WorkspaceMetaDto>;
}
```

## 9.3 `application/pack/service.rs`

```rust
pub trait PackService: Send + Sync {
    fn create_pack(
        &self,
        input: crate::application::dto::pack::CreatePackInput,
    ) -> AppResult<crate::application::dto::pack::PackOverviewDto>;

    fn update_pack_metadata(
        &self,
        input: crate::application::dto::pack::UpdatePackMetadataInput,
    ) -> AppResult<crate::application::dto::pack::PackMetadataDto>;

    fn delete_pack(
        &self,
        input: crate::application::dto::pack::DeletePackInput,
    ) -> AppResult<()>;

    fn open_pack(
        &self,
        input: crate::application::dto::pack::OpenPackInput,
    ) -> AppResult<crate::application::dto::pack::PackSnapshotDto>;

    fn close_pack(
        &self,
        input: crate::application::dto::pack::ClosePackInput,
    ) -> AppResult<crate::application::dto::workspace::WorkspaceOpenedDto>;

    fn get_pack_overview(
        &self,
        input: crate::application::dto::pack::GetPackInput,
    ) -> AppResult<crate::application::dto::pack::PackOverviewDto>;
}
```

## 9.3.1 `application/pack/write_service.rs`

```rust
pub trait PackWriteService: Send + Sync {
    fn create_card(
        &self,
        input: crate::application::dto::card::CreateCardInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::card::CardDetailDto>>;

    fn update_card(
        &self,
        input: crate::application::dto::card::UpdateCardInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::card::CardDetailDto>>;

    fn delete_cards(
        &self,
        input: crate::application::dto::card::DeleteCardsInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::card::DeleteCardsResultDto>>;

    fn batch_patch_cards(
        &self,
        input: crate::application::dto::card::BatchPatchCardsInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::card::BatchPatchResultDto>>;

    fn upsert_pack_string(
        &self,
        input: crate::application::dto::strings::UpsertPackStringInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::strings::PackStringsPageDto>>;

    fn delete_pack_strings(
        &self,
        input: crate::application::dto::strings::DeletePackStringsInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::strings::DeletePackStringsResultDto>>;

    fn import_main_image(
        &self,
        input: crate::application::dto::resource::ImportMainImageInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::resource::CardAssetStateDto>>;

    fn delete_main_image(
        &self,
        input: crate::application::dto::resource::DeleteMainImageInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::resource::CardAssetStateDto>>;

    fn import_field_image(
        &self,
        input: crate::application::dto::resource::ImportFieldImageInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::resource::CardAssetStateDto>>;

    fn delete_field_image(
        &self,
        input: crate::application::dto::resource::DeleteFieldImageInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::resource::CardAssetStateDto>>;

    fn create_empty_script(
        &self,
        input: crate::application::dto::resource::CreateEmptyScriptInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::resource::CardAssetStateDto>>;

    fn import_script(
        &self,
        input: crate::application::dto::resource::ImportScriptInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::resource::CardAssetStateDto>>;

    fn delete_script(
        &self,
        input: crate::application::dto::resource::DeleteScriptInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::resource::CardAssetStateDto>>;
}
```

说明：

1. `PackWriteService` 是单 `pack` 写编排的统一入口
2. 现有 `CardWriteService`、`PackStringsService`、`ResourceService` 可作为兼容层保留
3. 它们的内部实现应统一委托给 `PackWriteService`
4. `move_cards`、导入、导出这类跨 pack / workspace 用例不放进这里

## 9.4 `application/card/query_service.rs`

```rust
pub trait CardQueryService: Send + Sync {
    fn list_cards(
        &self,
        input: crate::application::dto::card::ListCardsInput,
    ) -> AppResult<crate::application::dto::card::CardListPageDto>;

    fn get_card(
        &self,
        input: crate::application::dto::card::GetCardInput,
    ) -> AppResult<crate::application::dto::card::CardDetailDto>;

    fn suggest_next_code(
        &self,
        input: crate::application::dto::card::SuggestCodeInput,
    ) -> AppResult<crate::application::dto::card::CodeSuggestionDto>;
}
```

## 9.5 `application/card/write_service.rs`

```rust
pub trait CardWriteService: Send + Sync {
    fn create_card(
        &self,
        input: crate::application::dto::card::CreateCardInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::card::CardDetailDto>>;

    fn update_card(
        &self,
        input: crate::application::dto::card::UpdateCardInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::card::CardDetailDto>>;

    fn delete_cards(
        &self,
        input: crate::application::dto::card::DeleteCardsInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::card::DeleteCardsResultDto>>;

    fn batch_patch_cards(
        &self,
        input: crate::application::dto::card::BatchPatchCardsInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::card::BatchPatchResultDto>>;

    fn move_cards(
        &self,
        input: crate::application::dto::card::MoveCardsInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::card::MoveCardsResultDto>>;
}
```

说明：

1. `CardWriteService` 在首版可继续作为对外 API 分组存在
2. 但其实现应转发给 `PackWriteService`

## 9.6 `application/card/confirmation_service.rs`

```rust
pub trait CardWriteConfirmationService: Send + Sync {
    fn confirm_card_write(
        &self,
        input: crate::application::dto::card::ConfirmCardWriteInput,
    ) -> AppResult<serde_json::Value>;
}
```

说明：

1. `confirm_card_write` 的返回值在首版可以统一为 `serde_json::Value`
2. 实际序列化内容必须与对应待确认写操作的最终 DTO 一致
3. 若后续觉得过于宽松，可再收敛为 enum DTO

## 9.7 `application/strings/service.rs`

```rust
pub trait PackStringsService: Send + Sync {
    fn list_pack_strings(
        &self,
        input: crate::application::dto::strings::ListPackStringsInput,
    ) -> AppResult<crate::application::dto::strings::PackStringsPageDto>;

    fn get_pack_string(
        &self,
        input: crate::application::dto::strings::GetPackStringInput,
    ) -> AppResult<crate::application::dto::strings::PackStringRecordDetailDto>;

    fn upsert_pack_string(
        &self,
        input: crate::application::dto::strings::UpsertPackStringInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::strings::PackStringsPageDto>>;

    fn upsert_pack_string_record(
        &self,
        input: crate::application::dto::strings::UpsertPackStringRecordInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::strings::PackStringRecordDetailDto>>;

    fn delete_pack_strings(
        &self,
        input: crate::application::dto::strings::DeletePackStringsInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::strings::DeletePackStringsResultDto>>;

    fn remove_pack_string_translation(
        &self,
        input: crate::application::dto::strings::RemovePackStringTranslationInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::strings::PackStringRecordDetailDto>>;
}
```

说明：

1. `PackStringsService` 的写接口应转发给 `PackWriteService`

## 9.8 `application/strings/confirmation_service.rs`

```rust
pub trait PackStringsConfirmationService: Send + Sync {
    fn confirm_pack_strings_write(
        &self,
        input: crate::application::dto::strings::ConfirmPackStringsWriteInput,
    ) -> AppResult<crate::application::dto::strings::PackStringsPageDto>;
}
```

## 9.9 `application/resource/service.rs`

```rust
pub trait ResourceService: Send + Sync {
    fn import_main_image(
        &self,
        input: crate::application::dto::resource::ImportMainImageInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::resource::CardAssetStateDto>>;

    fn delete_main_image(
        &self,
        input: crate::application::dto::resource::DeleteMainImageInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::resource::CardAssetStateDto>>;

    fn import_field_image(
        &self,
        input: crate::application::dto::resource::ImportFieldImageInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::resource::CardAssetStateDto>>;

    fn delete_field_image(
        &self,
        input: crate::application::dto::resource::DeleteFieldImageInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::resource::CardAssetStateDto>>;

    fn create_empty_script(
        &self,
        input: crate::application::dto::resource::CreateEmptyScriptInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::resource::CardAssetStateDto>>;

    fn import_script(
        &self,
        input: crate::application::dto::resource::ImportScriptInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::resource::CardAssetStateDto>>;

    fn delete_script(
        &self,
        input: crate::application::dto::resource::DeleteScriptInput,
    ) -> AppResult<crate::application::dto::common::WriteResultDto<crate::application::dto::resource::CardAssetStateDto>>;

    fn open_script_external(
        &self,
        input: crate::application::dto::resource::OpenScriptExternalInput,
    ) -> AppResult<()>;
}
```

说明：

1. `open_script_external` 仍可直接调用外部编辑器，不属于 pack 内安全写入提交
2. 其余资源写接口应转发给 `PackWriteService`

## 9.10 `application/standard_pack/service.rs`

```rust
pub trait StandardPackService: Send + Sync {
    fn get_standard_pack_status(
        &self,
    ) -> AppResult<crate::application::dto::standard_pack::StandardPackStatusDto>;

    fn rebuild_standard_pack_index(
        &self,
    ) -> AppResult<crate::application::dto::job::JobAcceptedDto>;

    fn search_standard_cards(
        &self,
        input: crate::application::dto::standard_pack::SearchStandardCardsInput,
    ) -> AppResult<crate::application::dto::standard_pack::StandardCardPageDto>;
}
```

## 9.11 `application/import/service.rs`

```rust
pub trait ImportService: Send + Sync {
    fn preview_import_pack(
        &self,
        input: crate::application::dto::import::PreviewImportPackInput,
    ) -> AppResult<crate::application::dto::common::PreviewResultDto<crate::application::dto::import::ImportPreviewDto>>;

    fn execute_import_pack(
        &self,
        input: crate::application::dto::import::ExecuteImportPackInput,
    ) -> AppResult<crate::application::dto::job::JobAcceptedDto>;
}
```

## 9.12 `application/export/service.rs`

```rust
pub trait ExportService: Send + Sync {
    fn preview_export_bundle(
        &self,
        input: crate::application::dto::export::PreviewExportBundleInput,
    ) -> AppResult<crate::application::dto::common::PreviewResultDto<crate::application::dto::export::ExportPreviewDto>>;
}
```

说明：

1. `preview_import_pack` 与 `preview_export_bundle` 的 `snapshot_hash` 在首版建议由相关 `revision/source_stamp` 组合计算

## 9.13 `application/jobs/service.rs`

```rust
pub trait JobService: Send + Sync {
    fn get_job_status(
        &self,
        input: crate::application::dto::job::GetJobStatusInput,
    ) -> AppResult<crate::application::dto::job::JobSnapshotDto>;

    fn list_active_jobs(
        &self,
    ) -> AppResult<Vec<crate::application::dto::job::JobSnapshotDto>>;
}
```

## 10. Runtime 层接口

## 10.1 `runtime/sessions/workspace_session.rs`

```rust
#[derive(Debug, Clone)]
pub struct WorkspaceSession {
    pub workspace_id: WorkspaceId,
    pub workspace_root: std::path::PathBuf,
    pub workspace_meta: crate::domain::workspace::model::WorkspaceMeta,
    pub pack_summaries: Vec<crate::domain::pack::model::PackOverview>,
    pub open_pack_ids: Vec<PackId>,
    pub active_pack_id: Option<PackId>,
}
```

## 10.2 `runtime/sessions/pack_session.rs`

```rust
#[derive(Debug, Clone)]
pub struct PackSession {
    pub pack_id: PackId,
    pub revision: u64,
    pub source_stamp: String,
    pub metadata: crate::domain::pack::model::PackMetadata,
    pub cards_by_id: std::collections::HashMap<CardId, crate::domain::card::model::CardEntity>,
    pub code_index: std::collections::HashMap<u32, CardId>,
    pub card_list_cache: Vec<crate::domain::card::model::CardListRow>,
    pub asset_index: std::collections::HashMap<CardId, crate::domain::resource::model::CardAssetState>,
    pub strings: crate::domain::strings::model::PackStringsFile,
}
```

说明：

1. `WorkspaceSession` 表示当前已打开工作区的运行时上下文
2. `PackSession` 表示单个 `pack` 的已加载作者态快照与基础读模型
3. 打开 `workspace` 时只加载工作区元数据和 `pack` 摘要，不一次性构建全部自定义 `pack` 的 `PackSession`
4. `open_pack_ids` 表示当前侧边栏 tab 中已打开 `pack` 的顺序
5. 打开 pack tab 时构建对应 `PackSession`
6. 切换 active pack 只更新 `active_pack_id`，不要求重新读取已打开 pack 的磁盘作者态数据
7. 关闭 pack tab 时移除对应 `PackSession`
8. 若外部文件变化或用户执行手动刷新，则替换受影响 `PackSession`
9. `revision` 只在程序内成功写入后递增
10. `source_stamp` 表示当前 `PackSession` 加载时观测到的磁盘状态摘要
11. 手动刷新、重载 pack 或发现 `source_stamp` 不匹配后，旧 token 必须失效

## 10.3 `runtime/sessions/session_manager.rs`

```rust
pub trait SessionManager: Send + Sync {
    fn current_workspace(&self) -> Option<WorkspaceSession>;
    fn set_workspace(&self, session: WorkspaceSession);
    fn clear_workspace(&self);
    fn list_open_pack_ids(&self) -> Vec<PackId>;
    fn get_pack(&self, pack_id: &PackId) -> Option<PackSession>;
    fn set_pack(&self, session: PackSession);
    fn remove_pack(&self, pack_id: &PackId);
    fn add_open_pack(&self, pack_id: PackId);
    fn close_open_pack(&self, pack_id: &PackId);
    fn set_active_pack(&self, pack_id: Option<PackId>);
}
```

## 10.4 `runtime/jobs/job_store.rs`

```rust
#[derive(Debug, Clone)]
pub enum RuntimeJobStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct JobSnapshot {
    pub job_id: JobId,
    pub kind: crate::application::ports::job_scheduler::JobKind,
    pub status: RuntimeJobStatus,
    pub stage: String,
    pub progress_percent: Option<u8>,
    pub message: Option<String>,
    pub started_at: Option<AppTimestamp>,
    pub finished_at: Option<AppTimestamp>,
}

pub trait JobStore: Send + Sync {
    fn insert(&self, snapshot: crate::runtime::jobs::job_store::JobSnapshot);
    fn get(&self, job_id: &JobId) -> Option<crate::runtime::jobs::job_store::JobSnapshot>;
    fn list_active(&self) -> Vec<crate::runtime::jobs::job_store::JobSnapshot>;
    fn update(&self, snapshot: crate::runtime::jobs::job_store::JobSnapshot);
}
```

说明：

1. `JobStore` 保存 runtime 内部任务快照，不直接存 application DTO
2. `application/job service` 负责把内部任务快照映射为 `JobSnapshotDto`

## 10.5 `runtime/events/app_event.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppEvent {
    JobProgress(JobProgressEvent),
    JobFinished(JobFinishedEvent),
    WorkspaceChanged(WorkspaceChangedEvent),
    PackChanged(PackChangedEvent),
    StandardPackIndexUpdated(StandardPackIndexUpdatedEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgressEvent {
    pub job_id: JobId,
    pub status: String,
    pub stage: String,
    pub progress_percent: Option<u8>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobFinishedEvent {
    pub job_id: JobId,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceChangedEvent {
    pub workspace_id: WorkspaceId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackChangedEvent {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardPackIndexUpdatedEvent {
    pub indexed_at: AppTimestamp,
}
```

## 11. Presentation 层接口

## 11.1 命令返回策略

Tauri command 建议使用：

```rust
pub type CommandResult<T> = Result<T, crate::presentation::errors::app_error_dto::AppErrorDto>;
```

前端通过 `invokeApi` 把：

1. 成功值包装成 `{ ok: true, data }`
2. 异常值包装成 `{ ok: false, error }`

## 11.2 `presentation/commands/config_commands.rs`

```rust
#[tauri::command]
pub async fn get_global_config(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
) -> CommandResult<crate::presentation::dto::config::GlobalConfigDto>;

#[tauri::command]
pub async fn update_global_config(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::config::UpdateGlobalConfigInput,
) -> CommandResult<crate::presentation::dto::config::GlobalConfigDto>;

#[tauri::command]
pub async fn validate_ygopro_path(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::config::ValidateYgoProPathInput,
) -> CommandResult<crate::presentation::dto::config::YgoProPathCheckResultDto>;
```

## 11.3 `presentation/commands/workspace_commands.rs`

```rust
#[tauri::command]
pub async fn list_recent_workspaces(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
) -> CommandResult<Vec<crate::presentation::dto::workspace::WorkspaceListItemDto>>;

#[tauri::command]
pub async fn create_workspace(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::workspace::CreateWorkspaceInput,
) -> CommandResult<crate::presentation::dto::workspace::WorkspaceOpenedDto>;

#[tauri::command]
pub async fn open_workspace(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::workspace::OpenWorkspaceInput,
) -> CommandResult<crate::presentation::dto::workspace::WorkspaceOpenedDto>;

#[tauri::command]
pub async fn close_workspace(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
) -> CommandResult<()>;

#[tauri::command]
pub async fn reorder_packs(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::workspace::ReorderPacksInput,
) -> CommandResult<crate::presentation::dto::workspace::WorkspaceMetaDto>;
```

## 11.4 `presentation/commands/pack_commands.rs`

```rust
#[tauri::command]
pub async fn create_pack(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::pack::CreatePackInput,
) -> CommandResult<crate::presentation::dto::pack::PackOverviewDto>;

#[tauri::command]
pub async fn update_pack_metadata(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::pack::UpdatePackMetadataInput,
) -> CommandResult<crate::presentation::dto::pack::PackMetadataDto>;

#[tauri::command]
pub async fn delete_pack(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::pack::DeletePackInput,
) -> CommandResult<()>;

#[tauri::command]
pub async fn open_pack(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::pack::OpenPackInput,
) -> CommandResult<crate::presentation::dto::pack::PackSnapshotDto>;

#[tauri::command]
pub async fn close_pack(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::pack::ClosePackInput,
) -> CommandResult<crate::presentation::dto::workspace::WorkspaceOpenedDto>;

#[tauri::command]
pub async fn set_active_pack(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::pack::SetActivePackInput,
) -> CommandResult<()>;

#[tauri::command]
pub async fn get_pack_overview(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::pack::GetPackInput,
) -> CommandResult<crate::presentation::dto::pack::PackOverviewDto>;
```

## 11.5 `presentation/commands/card_commands.rs`

```rust
#[tauri::command]
pub async fn list_cards(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::card::ListCardsInput,
) -> CommandResult<crate::presentation::dto::card::CardListPageDto>;

#[tauri::command]
pub async fn get_card(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::card::GetCardInput,
) -> CommandResult<crate::presentation::dto::card::CardDetailDto>;

#[tauri::command]
pub async fn suggest_next_code(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::card::SuggestCodeInput,
) -> CommandResult<crate::presentation::dto::card::CodeSuggestionDto>;

#[tauri::command]
pub async fn create_card(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::card::CreateCardInput,
) -> CommandResult<crate::presentation::dto::common::WriteResultDto<crate::presentation::dto::card::CardDetailDto>>;

#[tauri::command]
pub async fn update_card(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::card::UpdateCardInput,
) -> CommandResult<crate::presentation::dto::common::WriteResultDto<crate::presentation::dto::card::CardDetailDto>>;

#[tauri::command]
pub async fn delete_cards(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::card::DeleteCardsInput,
) -> CommandResult<crate::presentation::dto::common::WriteResultDto<crate::presentation::dto::card::DeleteCardsResultDto>>;

#[tauri::command]
pub async fn batch_patch_cards(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::card::BatchPatchCardsInput,
) -> CommandResult<crate::presentation::dto::common::WriteResultDto<crate::presentation::dto::card::BatchPatchResultDto>>;

#[tauri::command]
pub async fn move_cards(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::card::MoveCardsInput,
) -> CommandResult<crate::presentation::dto::common::WriteResultDto<crate::presentation::dto::card::MoveCardsResultDto>>;

#[tauri::command]
pub async fn confirm_card_write(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::card::ConfirmCardWriteInput,
) -> CommandResult<serde_json::Value>;
```

## 11.6 `presentation/commands/strings_commands.rs`

```rust
#[tauri::command]
pub async fn list_pack_strings(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::strings::ListPackStringsInput,
) -> CommandResult<crate::presentation::dto::strings::PackStringsPageDto>;

#[tauri::command]
pub async fn get_pack_string(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::strings::GetPackStringInput,
) -> CommandResult<crate::presentation::dto::strings::PackStringRecordDetailDto>;

#[tauri::command]
pub async fn upsert_pack_string(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::strings::UpsertPackStringInput,
) -> CommandResult<crate::presentation::dto::common::WriteResultDto<crate::presentation::dto::strings::PackStringsPageDto>>;

#[tauri::command]
pub async fn upsert_pack_string_record(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::strings::UpsertPackStringRecordInput,
) -> CommandResult<crate::presentation::dto::common::WriteResultDto<crate::presentation::dto::strings::PackStringRecordDetailDto>>;

#[tauri::command]
pub async fn delete_pack_strings(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::strings::DeletePackStringsInput,
) -> CommandResult<crate::presentation::dto::common::WriteResultDto<crate::presentation::dto::strings::DeletePackStringsResultDto>>;

#[tauri::command]
pub async fn remove_pack_string_translation(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::strings::RemovePackStringTranslationInput,
) -> CommandResult<crate::presentation::dto::common::WriteResultDto<crate::presentation::dto::strings::PackStringRecordDetailDto>>;

#[tauri::command]
pub async fn confirm_pack_strings_write(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::strings::ConfirmPackStringsWriteInput,
) -> CommandResult<crate::presentation::dto::strings::PackStringsPageDto>;
```

## 11.7 `presentation/commands/resource_commands.rs`

```rust
#[tauri::command]
pub async fn import_main_image(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::resource::ImportMainImageInput,
) -> CommandResult<crate::presentation::dto::common::WriteResultDto<crate::presentation::dto::resource::CardAssetStateDto>>;

#[tauri::command]
pub async fn delete_main_image(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::resource::DeleteMainImageInput,
) -> CommandResult<crate::presentation::dto::common::WriteResultDto<crate::presentation::dto::resource::CardAssetStateDto>>;

#[tauri::command]
pub async fn import_field_image(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::resource::ImportFieldImageInput,
) -> CommandResult<crate::presentation::dto::common::WriteResultDto<crate::presentation::dto::resource::CardAssetStateDto>>;

#[tauri::command]
pub async fn delete_field_image(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::resource::DeleteFieldImageInput,
) -> CommandResult<crate::presentation::dto::common::WriteResultDto<crate::presentation::dto::resource::CardAssetStateDto>>;

#[tauri::command]
pub async fn create_empty_script(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::resource::CreateEmptyScriptInput,
) -> CommandResult<crate::presentation::dto::common::WriteResultDto<crate::presentation::dto::resource::CardAssetStateDto>>;

#[tauri::command]
pub async fn import_script(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::resource::ImportScriptInput,
) -> CommandResult<crate::presentation::dto::common::WriteResultDto<crate::presentation::dto::resource::CardAssetStateDto>>;

#[tauri::command]
pub async fn delete_script(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::resource::DeleteScriptInput,
) -> CommandResult<crate::presentation::dto::common::WriteResultDto<crate::presentation::dto::resource::CardAssetStateDto>>;

#[tauri::command]
pub async fn open_script_external(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::resource::OpenScriptExternalInput,
) -> CommandResult<()>;
```

## 11.8 `presentation/commands/standard_pack_commands.rs`

```rust
#[tauri::command]
pub async fn get_standard_pack_status(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
) -> CommandResult<crate::presentation::dto::standard_pack::StandardPackStatusDto>;

#[tauri::command]
pub async fn rebuild_standard_pack_index(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
) -> CommandResult<crate::presentation::dto::job::JobAcceptedDto>;

#[tauri::command]
pub async fn search_standard_cards(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::standard_pack::SearchStandardCardsInput,
) -> CommandResult<crate::presentation::dto::standard_pack::StandardCardPageDto>;
```

## 11.9 `presentation/commands/import_commands.rs`

```rust
#[tauri::command]
pub async fn preview_import_pack(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::import::PreviewImportPackInput,
) -> CommandResult<crate::presentation::dto::common::PreviewResultDto<crate::presentation::dto::import::ImportPreviewDto>>;

#[tauri::command]
pub async fn execute_import_pack(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::import::ExecuteImportPackInput,
) -> CommandResult<crate::presentation::dto::job::JobAcceptedDto>;
```

## 11.10 `presentation/commands/export_commands.rs`

```rust
#[tauri::command]
pub async fn preview_export_bundle(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::export::PreviewExportBundleInput,
) -> CommandResult<crate::presentation::dto::common::PreviewResultDto<crate::presentation::dto::export::ExportPreviewDto>>;
```

## 11.11 `presentation/commands/job_commands.rs`

```rust
#[tauri::command]
pub async fn get_job_status(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
    input: crate::presentation::dto::job::GetJobStatusInput,
) -> CommandResult<crate::presentation::dto::job::JobSnapshotDto>;

#[tauri::command]
pub async fn list_active_jobs(
    state: tauri::State<'_, crate::bootstrap::app_state::AppState>,
) -> CommandResult<Vec<crate::presentation::dto::job::JobSnapshotDto>>;
```

## 12. 前端接口设计

## 12.1 `shared/api/invokeApi.ts`

```ts
import { invoke } from "@tauri-apps/api/core";

export async function invokeApi<T>(command: string, input?: unknown): Promise<ApiResult<T>>;
```

约定：

1. `invokeApi` 捕获 Rust command 抛出的 `AppErrorDto`
2. 成功时包装为 `{ ok: true, data }`
3. 失败时包装为 `{ ok: false, error }`

## 12.2 `shared/api/configApi.ts`

```ts
export interface ConfigApi {
  getGlobalConfig(): Promise<ApiResult<GlobalConfigDto>>;
  updateGlobalConfig(input: UpdateGlobalConfigInput): Promise<ApiResult<GlobalConfigDto>>;
  validateYgoProPath(input: ValidateYgoProPathInput): Promise<ApiResult<YgoProPathCheckResultDto>>;
}
```

## 12.3 `shared/api/workspaceApi.ts`

```ts
export interface WorkspaceApi {
  listRecentWorkspaces(): Promise<ApiResult<WorkspaceListItemDto[]>>;
  createWorkspace(input: CreateWorkspaceInput): Promise<ApiResult<WorkspaceOpenedDto>>;
  openWorkspace(input: OpenWorkspaceInput): Promise<ApiResult<WorkspaceOpenedDto>>;
  closeWorkspace(): Promise<ApiResult<void>>;
  reorderPacks(input: ReorderPacksInput): Promise<ApiResult<WorkspaceMetaDto>>;
}
```

## 12.4 `shared/api/packApi.ts`

```ts
export interface PackApi {
  createPack(input: CreatePackInput): Promise<ApiResult<PackOverviewDto>>;
  updatePackMetadata(input: UpdatePackMetadataInput): Promise<ApiResult<PackMetadataDto>>;
  deletePack(input: DeletePackInput): Promise<ApiResult<void>>;
  openPack(input: OpenPackInput): Promise<ApiResult<PackSnapshotDto>>;
  closePack(input: ClosePackInput): Promise<ApiResult<WorkspaceOpenedDto>>;
  setActivePack(input: SetActivePackInput): Promise<ApiResult<void>>;
  getPackOverview(input: GetPackInput): Promise<ApiResult<PackOverviewDto>>;
}
```

## 12.5 `shared/api/cardApi.ts`

```ts
export interface CardApi {
  listCards(input: ListCardsInput): Promise<ApiResult<CardListPageDto>>;
  getCard(input: GetCardInput): Promise<ApiResult<CardDetailDto>>;
  suggestNextCode(input: SuggestCodeInput): Promise<ApiResult<CodeSuggestionDto>>;
  createCard(input: CreateCardInput): Promise<ApiResult<WriteResult<CardDetailDto>>>;
  updateCard(input: UpdateCardInput): Promise<ApiResult<WriteResult<CardDetailDto>>>;
  deleteCards(input: DeleteCardsInput): Promise<ApiResult<WriteResult<DeleteCardsResultDto>>>;
  batchPatchCards(input: BatchPatchCardsInput): Promise<ApiResult<WriteResult<BatchPatchResultDto>>>;
  moveCards(input: MoveCardsInput): Promise<ApiResult<WriteResult<MoveCardsResultDto>>>;
  confirmCardWrite(input: ConfirmCardWriteInput): Promise<ApiResult<unknown>>;
}
```

## 12.6 `shared/api/stringsApi.ts`

```ts
export interface StringsApi {
  listPackStrings(input: ListPackStringsInput): Promise<ApiResult<PackStringsPageDto>>;
  getPackString(input: GetPackStringInput): Promise<ApiResult<PackStringRecordDetailDto>>;
  upsertPackString(input: UpsertPackStringInput): Promise<ApiResult<WriteResult<PackStringsPageDto>>>;
  upsertPackStringRecord(input: UpsertPackStringRecordInput): Promise<ApiResult<WriteResult<PackStringRecordDetailDto>>>;
  deletePackStrings(input: DeletePackStringsInput): Promise<ApiResult<WriteResult<DeletePackStringsResultDto>>>;
  removePackStringTranslation(input: RemovePackStringTranslationInput): Promise<ApiResult<WriteResult<PackStringRecordDetailDto>>>;
  confirmPackStringsWrite(input: ConfirmPackStringsWriteInput): Promise<ApiResult<PackStringsPageDto>>;
}
```

## 12.7 `shared/api/resourceApi.ts`

```ts
export interface ResourceApi {
  importMainImage(input: ImportMainImageInput): Promise<ApiResult<WriteResult<CardAssetStateDto>>>;
  deleteMainImage(input: DeleteMainImageInput): Promise<ApiResult<WriteResult<CardAssetStateDto>>>;
  importFieldImage(input: ImportFieldImageInput): Promise<ApiResult<WriteResult<CardAssetStateDto>>>;
  deleteFieldImage(input: DeleteFieldImageInput): Promise<ApiResult<WriteResult<CardAssetStateDto>>>;
  createEmptyScript(input: CreateEmptyScriptInput): Promise<ApiResult<WriteResult<CardAssetStateDto>>>;
  importScript(input: ImportScriptInput): Promise<ApiResult<WriteResult<CardAssetStateDto>>>;
  deleteScript(input: DeleteScriptInput): Promise<ApiResult<WriteResult<CardAssetStateDto>>>;
  openScriptExternal(input: OpenScriptExternalInput): Promise<ApiResult<void>>;
}
```

## 12.8 `shared/api/standardPackApi.ts`

```ts
export interface StandardPackApi {
  getStandardPackStatus(): Promise<ApiResult<StandardPackStatusDto>>;
  rebuildStandardPackIndex(): Promise<ApiResult<JobAcceptedDto>>;
  searchStandardCards(input: SearchStandardCardsInput): Promise<ApiResult<StandardCardPageDto>>;
}
```

## 12.9 `shared/api/importApi.ts`

```ts
export interface ImportApi {
  previewImportPack(input: PreviewImportPackInput): Promise<ApiResult<PreviewResult<ImportPreviewDto>>>;
  executeImportPack(input: ExecuteImportPackInput): Promise<ApiResult<JobAcceptedDto>>;
}
```

## 12.10 `shared/api/exportApi.ts`

```ts
export interface ExportApi {
  previewExportBundle(input: PreviewExportBundleInput): Promise<ApiResult<PreviewResult<ExportPreviewDto>>>;
  executeExportBundle(input: ExecuteExportBundleInput): Promise<ApiResult<JobAcceptedDto>>;
}
```

## 12.11 `shared/api/jobApi.ts`

```ts
export interface JobApi {
  getJobStatus(input: GetJobStatusInput): Promise<ApiResult<JobSnapshotDto>>;
  listActiveJobs(): Promise<ApiResult<JobSnapshotDto[]>>;
}
```

## 12.12 `shared/api/events.ts`

```ts
export interface AppEventsApi {
  onJobProgress(handler: (event: JobProgressEvent) => void): Promise<() => void>;
  onJobFinished(handler: (event: JobFinishedEvent) => void): Promise<() => void>;
  onWorkspaceChanged(handler: (event: WorkspaceChangedEvent) => void): Promise<() => void>;
  onPackChanged(handler: (event: PackChangedEvent) => void): Promise<() => void>;
  onStandardPackIndexUpdated(handler: (event: StandardPackIndexUpdatedEvent) => void): Promise<() => void>;
}
```

## 13. JSON 文件协议接口

## 13.1 `global_config.json`

```ts
interface GlobalConfigFile {
  schema_version: 1;
  data: GlobalConfig;
}
```

## 13.2 `workspace_registry.json`

```ts
interface WorkspaceRegistryFile {
  schema_version: 1;
  workspaces: WorkspaceRegistryEntry[];
}
```

## 13.3 `workspace.json`

```ts
interface WorkspaceFile {
  schema_version: 1;
  data: WorkspaceMeta;
}
```

## 13.4 `metadata.json`

```ts
interface PackMetadataFile {
  schema_version: 1;
  data: PackMetadata;
}
```

## 13.5 `cards.json`

```ts
interface CardsFile {
  schema_version: 1;
  cards: CardEntity[];
}
```

## 13.6 `strings.json`

```ts
interface PackStringsFile {
  schema_version: 2;
  entries: PackStringRecord[];
}
```

## 14. 明确保留为实现细节的部分

以下内容在本文件中故意不冻结：

1. `infrastructure/sqlite_cdb` 内部如何拆 reader/writer/helper
2. 图片缩放内部使用哪个图像库
3. `runtime/index` 内部使用何种搜索索引结构
4. `confirm_card_write` 最终是否维持 `serde_json::Value`，还是收敛为强类型 enum
5. 导入导出 Job 的内部执行器文件组织
6. `PackSession` 的内部重建是整体替换还是局部增量更新

本文件已冻结的状态模型约束：

1. 真相源在磁盘文件与资源文件
2. `WorkspaceSession` / `PackSession` 是运行时快照
3. 缓存必须可丢弃、可重建
4. 写接口必须依赖显式输入定位目标对象
5. `PackSession` 只为已打开 `pack` 存在，不要求 `workspace` 打开时全量构建
6. 单 `pack` 内写操作的逻辑编排统一归 `PackWriteService`

本文件已冻结的分层类型约束：

1. `application dto` 不直接暴露完整 `domain entity`
2. 真相源仓储允许直接读写 `domain model`
3. `application ports` 不直接返回前端 DTO
4. `application event bus` 不直接依赖 `runtime` 对外事件模型

这些内容应该在真正开始写代码时再按实现复杂度细化，但不能反过来破坏本文档定义的公开边界。

## 15. 最终建议

如果按首版工程推进顺序，建议先把以下接口真正落为代码骨架：

1. `domain/common`
2. `domain/card`
3. `application/dto`
4. `application/ports`
5. `application/card/query_service.rs`
6. `application/card/write_service.rs`
7. `presentation/commands/card_commands.rs`
8. `shared/contracts/card.ts`
9. `shared/api/cardApi.ts`

原因是卡片模块是整个首版最密集的中心点：

1. 它连接模型、列表、校验、warning、资源、安全写入
2. 它最能检验文档定义是否足够稳定
3. 它一旦跑通，其他模块就可以按同一模式推进

这份接口设计文档可以视为首版编码阶段的“主合同”。后续如果要继续细化，最合适的下一步不是再泛泛写实现文档，而是：

1. 直接按本文档创建代码骨架
2. 给每个文件补 `mod.rs` 和空实现
3. 先从 `card/workspace/pack` 三个模块开始落首批可编译接口
