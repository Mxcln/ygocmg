# 自定义卡包高级搜索扩展计划

状态：已实现
计划日期：2026-05-02
实施日期：2026-05-02

## Implementation Update

本计划已完成首版落地：

- 已抽象通用 `CardSearchFilters` / `CardSearchFiltersDto`，标准包保留兼容别名。
- 已抽象通用 `CardAdvancedSearchPanel`，标准包与自定义卡包共用高级筛选 UI。
- 自定义卡包 `list_cards` 已支持可选 `filters`，并在 `PackSession.cards + card_list_cache` 内存快照上完成筛选、排序、分页与 total count。
- 自定义卡包 setname picker 已复用 CardEdit 语义，合并当前 pack setname 与 Standard Pack setname，pack 来源优先显示。
- 未引入自定义包 SQLite，未修改 `cards.json` / `strings.json` 存储格式。
- 已通过 `cargo test --offline`、`npm run typecheck`、`npm run build`。

## Summary

本计划把已完成的“标准包高级搜索”扩展到自定义卡包，并保持两者高级搜索 UI 一致。实现策略是：抽出通用筛选类型与通用高级搜索面板；标准包继续走 SQLite SQL builder；自定义卡包基于已打开的 `PackSession` 内存快照过滤，不引入自定义包 SQLite 存储。

核心目标：

- 自定义卡包 Cards tab 增加与标准包一致的高级筛选入口、modal tabs、active chips、clear all、setname picker。
- 自定义卡包后端 `list_cards` 支持结构化 `filters`，并与 keyword 使用 `AND` 组合。
- 自定义卡包 setname picker 使用“当前包 setname + 标准包 setname”的合并列表，沿用 CardEdit 当前语义：pack 来源优先显示，standard 来源作为补充。
- 保持标准包现有行为、API 兼容和 SQLite 查询路径不回退。

## Key Changes

### Shared Filter Contract

- 将前端 `StandardCardSearchFilters` 泛化为 `CardSearchFilters`：
  - 移到 `src/shared/contracts/card.ts`。
  - 包含现有 code/alias/OT/name/desc/type/race/attribute/monster flags/spell/trap/pendulum/link/setcode/category/atk/def/level 筛选字段。
  - 保留 `src/shared/contracts/standardPack.ts` 中的兼容导出：`StandardCardSearchFilters = CardSearchFilters`。
- 将 Rust 侧筛选 DTO 泛化：
  - 在 `application/dto/card.rs` 中定义 `CardSearchFiltersDto`、`NumericRangeFilterDto`、`CardFilterMatchModeDto`、`SetcodeFilterModeDto`。
  - 在 `application/dto/standard_pack.rs` 中 re-export 旧名称，避免标准包测试和调用大范围改名。
- 给自定义包 `ListCardsInput` 增加可选字段：
  - TypeScript：`filters?: CardSearchFilters | null`。
  - Rust：`#[serde(default)] pub filters: Option<CardSearchFiltersDto>`。
  - 旧调用不传 `filters` 时保持当前 keyword/sort/page 行为。

### Shared Advanced Search UI

- 将 `StandardCardAdvancedSearchPanel` 抽成通用 `CardAdvancedSearchPanel`：
  - 保留相同 tabs、chips、clear all、match mode、setcode exact/base、custom hex setcode 输入。
  - 面板不再自己调用标准包 API，而是通过 props 接收 `setnameEntries`。
  - `compactFilters`、`countCardFilters`、`cardFiltersKey` 作为通用工具导出；标准包保留旧 wrapper 名称或 alias。
- 标准包页面改为 adapter：
  - `StandardPackView` 自己加载标准 setnames，映射为 `{ key, name, source: "standard" }[]` 后传给通用面板。
  - 标准包 `search_standard_cards` 请求参数和行为保持不变。
- 自定义包页面接入同一面板：
  - `CardListPanel` 持有 `advancedFilters` 和 `advancedSearchOpen`。
  - 通过 `CardBrowserPanel` 的 `queryKeyExtra`、`resetKey`、`toolbarExtra`、`toolbarPanel` 接入筛选按钮和 modal。
  - `loadPage` 调用 `cardApi.listCards` 时传入 `filters: advancedFilters`。
- 第一版不重命名现有 `standard.search.*` i18n key，通用面板继续复用这些文案，避免三语文案大规模迁移；后续可单独做 `card.search.*` 命名整理。

### Merged Setname Source

- 抽出共享 hook/utility，例如 `useMergedSetnameEntries`，供 CardEdit 和自定义包高级搜索共用。
- 自定义包 setname 合并规则：
  - pack setnames 来自 `stringsApi.listPackStrings({ kindFilter: "setname", language: defaultDisplayLanguage, pageSize: 10000 })`。
  - standard setnames 来自 `standardPackApi.listSetnames({ language: config.standard_pack_source_language ?? null })`。
  - 合并结果保留 `source: "pack" | "standard"`。
  - 同 key 冲突时 UI 显示 pack 来源优先。
  - picker 排序 pack 来源在前，standard 来源在后，同来源内按名称排序。
  - 标准包索引缺失或查询失败时，自定义包高级搜索不阻断，只显示 pack setnames 和 custom hex 输入。
- 更新 CardEdit：
  - 复用同一个 hook/utility，避免 CardEdit 与 AdvancedSearch 未来出现 setname 行为漂移。
  - pack setname query key 包含 `packId` 与当前默认显示语言，避免语言变化后使用旧缓存。
- 字符串编辑后的缓存刷新：
  - `StringsListPanel` 在 setname 相关写入/删除后，除 invalidate `["strings"]` 外，也 invalidate `["pack-setnames", activePackId]`。
  - 如果统一 hook 使用更具体 query key，则同时按该 key 前缀失效。

## Backend Behavior

### Standard Pack

- 标准包继续使用现有 SQLite schema v3 和 SQL builder。
- 只调整 DTO 引用来源，不改变以下行为：
  - `filters` 可选。
  - keyword + filters 使用 `AND`。
  - count/page 共用同一 query object。
  - setcode exact/base、any/all、category any/all 等语义保持现状。
  - 标准包列表不反序列化 `detail_json`。

### Custom Pack

- `CardService::list_cards` 改为基于 `PackSession` 的内存过滤：
  - 从 `pack.card_list_cache` 遍历列表行，按 `row.id` 找对应 `CardEntity`。
  - 对 keyword 和 `filters` 都匹配后再排序、分页。
  - `total` 是筛选后的总数，分页仍由后端完成。
- 自定义包 keyword 建议对齐标准包宽松搜索：
  - 匹配 `row.code`、`row.name`、`row.desc`、`row.subtype_display`、`row.primary_type`。
  - 这是对当前自定义包 keyword 的兼容增强，不会减少结果。
- 自定义包结构化筛选语义：
  - `codes/codeRange/aliases/aliasRange/ots`：直接匹配 `CardEntity`。
  - `nameContains/descContains`：匹配当前列表显示文本，即 `CardListRow.name/desc`。
  - `primaryTypes/races/attributes/spellSubtypes/trapSubtypes`：直接匹配 enum，字段为空时不匹配非空筛选。
  - `monsterFlags/linkMarkers`：支持 `any/all`；`all` 表示卡片包含所有选中项。
  - `setcodes`：支持 `exact/base` 与 `any/all`；`base` 使用 `setcode & 0x0fff`，忽略 0。
  - `categoryMasks`：`any` 为 `(category & combined) != 0`，`all` 为 `(category & combined) == combined`。
  - `atk/def/level/pendulumLeftScale/pendulumRightScale`：字段为空时不匹配 active range。
- 避免性能陷阱：
  - 不在每行过滤时线性查找卡片；先构建 `card_id -> CardEntity` map，或遍历前构建等价索引。
  - 不读取磁盘 JSON；只使用当前 `PackSession`。
  - 不新增自定义包 SQLite 或持久化索引。

## Cache & Performance Notes

- 标准包缓存架构不变：
  - 标准 setnames 继续由 `StandardPackRuntimeCache.setnames_by_language` 按 SQLite 文件 stamp 缓存。
  - 高级搜索仍通过 SQLite 查询，不加载完整标准包。
- 自定义包列表缓存影响：
  - 当前 `card_list_cache` 仍保留，用于列表展示字段和显示语言回退。
  - 高级筛选额外读取 `PackSession.cards`，但仍是内存操作。
  - 自定义包写卡后现有 `["cards"]` invalidation 会清掉不同筛选组合的 React Query cache。
- 前端 query cache：
  - `cardFiltersKey` 必须 canonical 化，至少保证数组字段排序/去重后再 stringify，避免同一筛选因选择顺序不同生成多个缓存条目。
  - 筛选变化通过 `resetKey` 重置到第一页。
  - 文本字段保持 250ms debounce，避免每个键入都触发后端查询。
- 后续性能升级条件：
  - 若真实自定义包规模达到数千至上万张且筛选明显卡顿，再添加 revision-bound 的内存 search projection。
  - 该 projection 只存在于 `PackSession` 生命周期内，不落盘，不引入 schema migration。

## Test Plan

### Rust Tests

- 保持现有标准包测试通过，确认 DTO 泛化不改变 `search_standard_cards` 行为。
- 为自定义包 `list_cards` 增加后端测试：
  - `filters = None` 与旧行为一致。
  - 空数组、空字符串、空 range 被视为无筛选。
  - keyword + filters 使用 `AND`。
  - code/alias/OT/name/desc/type/race/attribute 基础筛选。
  - monster flags `any/all`。
  - link markers `any/all`。
  - setcode `exact/base` 与 `any/all`。
  - category `any/all`。
  - ATK/DEF/Level/pendulum scale range。
  - 筛选后 `total` 与分页结果正确。
  - null 字段不匹配 active range 或 enum 筛选。

### Frontend Checks

- `npm run typecheck` 和 `npm run build` 必须通过。
- 手动检查标准包：
  - 高级搜索 UI、chips、clear all、setname picker 行为不回归。
  - 标准 setname 查询失败时 UI 不崩溃。
- 手动检查自定义包：
  - Cards tab 出现与标准包一致的筛选按钮和 modal。
  - 选择 pack setname 能筛到当前包卡片。
  - 选择 standard setname 能筛到使用标准系列码的自定义卡片。
  - 输入 custom hex setcode 能筛选对应卡片。
  - 编辑卡片后，当前筛选列表刷新。
  - 编辑 pack setname 后，CardEdit 与高级搜索 picker 都刷新显示。
  - 切换打开的 pack 后，筛选状态不串包；建议每个 `CardListPanel` 生命周期内重置 filters。

### Recommended Commands

```text
cargo test --offline
npm run typecheck
npm run build
```

## Assumptions & Defaults

- 自定义卡包高级搜索只针对“当前打开的单个自定义包”，不做跨包搜索。
- 自定义包高级搜索文本筛选使用当前列表显示语言回退结果，不搜索所有语言。
- 自定义包不引入 SQLite、不改变 `cards.json`、`strings.json` 持久化格式。
- 自定义包排序第一版保持现状：`code/name`；高级搜索 UI 一致不要求排序选项完全一致。
- 标准包缺失时，自定义包高级搜索仍可用；仅标准 setname 来源降级不可用。
- 资源状态仍只用于列表展示，不纳入本次高级筛选。
- 第一版不保存筛选 presets，也不持久化最近筛选条件。
