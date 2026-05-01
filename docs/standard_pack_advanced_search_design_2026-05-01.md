# 标准包高级搜索设计文档

状态：v3 首版已实现
日期：2026-05-01
实施更新：2026-05-01
范围：Standard Pack 卡片浏览与高级搜索

## 0. 实施状态

本设计已完成首版实现。实现范围覆盖：

1. `search_standard_cards` 输入 DTO 的兼容扩展。
2. Standard Pack SQLite schema v3。
3. rebuild 写入新的语义化查询列和规范化查询表。
4. 后端动态 SQL builder 和结构化筛选。
5. 标准包 Cards tab 的高级筛选 UI。
6. active filter chips、clear all、setname picker。
7. zh-CN / en-US / ja-JP 三语文案。
8. 后端集成测试和前端类型/构建验证。

### 0.1 本次落地的核心文件

后端：

1. `src-tauri/src/application/dto/standard_pack.rs`
   - 新增 `StandardCardSearchFiltersDto`。
   - 新增 `NumericRangeFilterDto`。
   - 新增 `CardFilterMatchModeDto`。
   - 新增 `SetcodeFilterModeDto`。
   - `SearchStandardCardsInput` 增加可选 `filters` 字段。

2. `src-tauri/src/infrastructure/standard_pack/sqlite_store.rs`
   - `STANDARD_SQLITE_SCHEMA_VERSION` 从 `2` 升到 `3`。
   - `standard_cards` 增加 `race`、`attribute`、`spell_subtype`、`trap_subtype`。
   - 新增 `standard_card_monster_flags`。
   - 新增 `standard_card_setcodes`。
   - 新增 `standard_card_pendulum`。
   - 新增 `standard_card_link_markers`。
   - rebuild 时写入上述新列和新表。
   - row count validation 覆盖新表。

3. `src-tauri/src/application/standard_pack/repository.rs`
   - 将原先 keyword 空 / 非空两个静态 SQL 分支重构为小型 SQL builder。
   - count query 和 page query 共用同一个 query object。
   - 所有用户输入通过 rusqlite 参数绑定进入 SQL。
   - `ORDER BY` 仍只来自后端枚举白名单。
   - 实现本阶段所有结构化筛选。

4. `src-tauri/tests/standard_pack.rs`
   - 标准包测试从 34 个扩展到 36 个。
   - 新增 schema v3 写入测试。
   - 新增高级筛选组合测试。

前端：

1. `src/shared/contracts/standardPack.ts`
   - 新增 TypeScript filter contract。
   - `SearchStandardCardsInput` 增加 `filters`。

2. `src/shared/constants/cardOptions.ts`
   - 从 `CardInfoForm` 抽出卡片枚举选项，供编辑表单和高级筛选 UI 共用。

3. `src/features/card/CardBrowserPanel.tsx`
   - keyword 增加 250ms debounce。
   - 增加 `queryKeyExtra`。
   - 增加 `resetKey`。
   - 增加 toolbar 扩展入口。
   - 保持自定义包卡片列表兼容。

4. `src/features/standardPack/StandardCardAdvancedSearchPanel.tsx`
   - 新增高级筛选 UI。
   - 筛选编辑以悬浮 modal 呈现。
   - modal 内按领域拆分为 tabs。
   - 支持 active chips、单项移除、clear all。
   - 支持 setname 搜索、hex 搜索、自定义 hex setcode。

5. `src/features/standardPack/StandardPackView.tsx`
   - 持有 `advancedFilters` state。
   - 将 filters 传入 `standardPackApi.searchCards`。
   - 筛选变化后通过 `resetKey` 重置列表分页。

6. `src/shared/i18n/messages/*.ts`
   - 增加高级筛选相关文案。

### 0.2 本次实现的筛选能力

后端已支持：

1. `keyword` 宽松搜索，并与结构化筛选 `AND` 组合。
2. code 精确值。
3. code range。
4. alias 精确值。
5. alias range。
6. OT。
7. name contains。
8. desc contains。
9. primary type。
10. race。
11. attribute。
12. monster flags `any/all`。
13. spell subtype。
14. trap subtype。
15. pendulum left/right scale range。
16. link markers `any/all`。
17. setcode `exact/base`。
18. setcode `any/all`。
19. category masks `any/all`。
20. ATK / DEF / Level range。

前端首版暴露了上述能力，并采用以下默认值：

1. `setcodeMode = base`。
2. `setcodeMatch = any`。
3. `categoryMatch = any`。
4. `monsterFlagMatch = any`。
5. `linkMarkerMatch = any`。

### 0.3 兼容和迁移行为

1. `filters` 是可选字段。
2. 旧前端或旧调用只传 keyword/sort/page/pageSize 仍可用。
3. 空数组、空字符串、空 range 在前后端都会被视为无筛选。
4. SQLite schema v3 不做 in-place migration。
5. 旧 schema v2 索引会触发现有 schema mismatch/rebuild required 流程。
6. 用户通过重建 Standard Pack 索引恢复。
7. 标准包仍然是只读 reference index。
8. 资源状态仍只用于列表展示，不参与高级筛选。

### 0.4 验证结果

本次实现后已通过：

```text
cargo fmt
cargo test --offline --test standard_pack
cargo test --offline
npm run typecheck
npm run build
```

验证结果：

1. `standard_pack` 集成测试：36 passed。
2. 全量 Rust 测试通过。
3. 前端 TypeScript 检查通过。
4. 前端生产构建通过。

`npm run build` 仍有 Vite chunk size warning，但构建成功；该 warning 不影响本功能。

## 1. 背景

对 custom card 作者来说，标准包高级搜索比自定义包高级搜索更实用。作者在设计新卡时，经常需要从官方卡中查找满足某些结构或文本条件的参考卡，例如：

1. 指定编号、别名编号、OT。
2. 指定种族或属性。
3. 指定主类型、怪兽类型、魔法/陷阱子类型。
4. 指定是否是融合、同调、超量、连接、调整、灵摆等。
5. 指定灵摆刻度、连接箭头等卡片数据。
6. 指定包含某个卡片系列 / setcode。
7. 指定包含某些 CDB 效果分类标签。
8. 卡名或效果文本包含某段文字。
9. ATK / DEF / Level 范围。

当前标准包搜索已经有 SQLite 索引和 FTS 基础，但它仍然主要是“关键词浏览”。高级搜索需要：

1. 一个结构化查询 DTO。
2. 更丰富的 SQLite 可查询读模型。
3. 安全的动态 SQL 查询构造。
4. 前端高级筛选面板。

## 2. 设计前实现

本节记录设计开始时的实现状态。首版实现完成后，实际代码已升级到本文 `0. 实施状态` 所述的 v3 形态。

设计前标准包搜索输入：

```rust
pub struct SearchStandardCardsInput {
    pub keyword: Option<String>,
    pub sort_by: StandardCardSortFieldDto,
    pub sort_direction: SortDirectionDto,
    pub page: u32,
    pub page_size: u32,
}
```

后端通过 `SqliteStandardPackRepository::search_cards` 打开 `index.sqlite`，从 `standard_card_list_rows` 查询列表页。非空关键词会同时走：

1. `instr(lower(...), keyword)` 兼容子串匹配。
2. `standard_card_search_fts` 的 FTS 查询路径。

当前 SQLite schema version：

```rust
pub const STANDARD_SQLITE_SCHEMA_VERSION: u32 = 2;
```

当前与卡片搜索相关的表：

```sql
create table standard_cards (
  code integer primary key,
  alias integer not null,
  ot text not null,
  category integer not null,
  primary_type text not null,
  subtype_display text not null,
  atk integer,
  def integer,
  level integer,
  raw_type integer not null,
  raw_race integer not null,
  raw_attribute integer not null,
  raw_level integer not null,
  detail_json text not null
);

create table standard_card_texts (
  code integer not null,
  language text not null,
  name text not null,
  desc text not null,
  strings_json text not null,
  primary key (code, language)
);

create table standard_card_list_rows (
  code integer not null,
  language text not null,
  name text not null,
  desc text not null,
  primary_type text not null,
  subtype_display text not null,
  atk integer,
  def integer,
  level integer,
  has_image integer not null,
  has_script integer not null,
  has_field_image integer not null,
  primary key (code, language)
);

create virtual table standard_card_search_fts using fts5(
  code unindexed,
  language unindexed,
  name,
  card_desc,
  primary_type,
  subtype_display
);

create table standard_assets (
  code integer primary key,
  has_image integer not null,
  has_script integer not null,
  has_field_image integer not null
);
```

`standard_assets` 仍用于列表展示和详情读取，但本次高级搜索不提供资源状态筛选。

这套 schema 已经可以支持一部分高级筛选，但还不能直接支持：

1. setcode / 系列搜索。
2. 灵摆刻度和连接箭头的语义化查询。
3. 完全语义化的怪兽 flags 查询。

如果搜索时去读取 `detail_json`，就会破坏标准包 SQLite 化的初衷。因此这些能力应通过 schema v3 增加规范化查询表。

## 3. 目标

1. 为标准包卡片提供结构化高级搜索。
2. 搜索、排序、分页、total count 继续在 SQLite 中完成。
3. 保留现有关键词搜索行为，尤其是中文/日文子串匹配。
4. 列表搜索不反序列化 `detail_json`。
5. 除提示文本和资源状态以外，覆盖卡片编辑表单中的可编辑卡片数据字段。
6. 查询 DTO 尽量设计成未来可复用于自定义包。
7. SQLite 索引仍然是可丢弃、可重建缓存。

## 4. 非目标

1. 不解析 Lua 脚本来推断真实效果语义。
2. 不把 `datas.category` 误认为 Lua `CATEGORY_*` 常量。
3. 不在本阶段实现标准包多语言全文索引。
4. 不把自定义包主存储改成 SQLite。
5. 不做相关性排序；排序仍由用户选择 code/name/type 等字段。
6. 不让标准包变成可编辑。
7. 不搜索提示文本 `str1` - `str16`。
8. 不按资源状态筛选，例如是否有卡图、场地图、脚本。

## 5. 搜索语义

### 5.1 Keyword

`keyword` 保持当前宽松搜索语义：

1. 匹配十进制 code 子串。
2. 匹配 source language 的卡名。
3. 匹配 source language 的效果文本 `desc`。
4. 匹配主类型和 subtype 展示文本。
5. 额外走 FTS 查询路径。

`keyword` 是“泛搜索”，高级筛选是“结构化约束”。两者同时存在时使用 `AND` 组合。

### 5.2 结构化筛选组合

不同筛选组之间默认使用 `AND`。

示例：

```text
keyword = "special summon"
race = dragon
attribute = dark
category contains search
setcode base = 0x345
```

语义是：

```text
keyword 条件
AND 种族条件
AND 属性条件
AND 分类条件
AND 系列条件
```

同一筛选组内部，如果存在多个值，应按筛选组自己的 `any` / `all` 模式处理。

### 5.3 编号、别名和 OT

编号支持：

1. `codes`：指定一组精确编号。
2. `codeRange`：指定编号范围。
3. `keyword`：继续支持编号子串搜索。

别名编号支持：

1. `aliases`：指定一组精确 alias 值。
2. `aliasRange`：指定 alias 范围。

OT 支持：

```text
ocg | tcg | custom
```

标准包来自官方 CDB，通常主要是 OCG/TCG，但 DTO 应与 `CardEntity.ot` 保持一致。

### 5.4 卡名和效果文本

文本包含只覆盖：

1. `nameContains`：卡名。
2. `descContains`：效果文本 `desc`。

不搜索 `CardTexts.strings`，也就是 CDB 的 `str1` - `str16` 提示文本。

文本匹配必须保留子串匹配：

```sql
instr(lower(value), :needle) > 0
```

FTS 可以作为额外路径，但不能作为唯一路径，否则中文/日文搜索容易漏结果。

### 5.5 种族和属性

种族和属性是精确结构匹配。

第一版可以直接查询 `raw_race` 和 `raw_attribute`，但长期更推荐在 `standard_cards` 中保存语义字段：

```sql
race text null,
attribute text null
```

原因是 repository 层不应该长期理解 CDB bit 编码细节。

### 5.6 主类型和子类型

主类型：

```text
monster | spell | trap
```

怪兽 flags：

```text
normal | effect | fusion | ritual | synchro | xyz | pendulum | link |
tuner | token | gemini | spirit | union | flip | toon
```

魔法子类型：

```text
normal | continuous | quick_play | ritual | field | equip
```

陷阱子类型：

```text
normal | continuous | counter
```

为了避免查询层直接依赖 `raw_type` bitmask，schema v3 建议新增 `standard_card_monster_flags` 表，并在 `standard_cards` 增加 `spell_subtype` / `trap_subtype` 语义列。

### 5.7 灵摆刻度和连接箭头

灵摆刻度筛选：

1. `leftScale` 范围。
2. `rightScale` 范围。

连接箭头筛选：

1. `linkMarkers`：选择一组连接箭头。
2. `linkMarkerMatch = any`：包含任意一个所选箭头。
3. `linkMarkerMatch = all`：包含所有所选箭头。

连接箭头值与 `CardEntity.link.markers` 保持一致：

```text
top | bottom | left | right | top_left | top_right | bottom_left | bottom_right
```

### 5.8 系列 / Setcode

setcode 搜索需要两种模式：

1. `exact`：匹配完整 setcode 槽位值。
2. `base`：匹配低 12 位 setname base，用于查找主系列及其子系列。

base 规则应与现有 namespace 逻辑一致：

```rust
base = setcode & 0x0fff
```

setcode 搜索还需要支持：

1. `any`：卡片包含任意一个指定 setcode/base。
2. `all`：卡片同时包含所有指定 setcode/base。

UI 上如果使用“系列”这个词，默认更适合 `base`。如果使用“系列码”这个词，可以让用户显式选择 exact/base。

### 5.9 效果分类 Category

`category` 是 YGOPro CDB `datas.category` 搜索标签，对应 `strings.conf` system strings `1100` - `1131`。它不是 Lua 脚本里的 `CATEGORY_*` 常量。

category 搜索支持：

1. `any`：包含任意一个所选 bit。
2. `all`：包含所有所选 bit。

SQL：

```sql
-- any
(c.category & :mask) != 0

-- all
(c.category & :mask) = :mask
```

### 5.10 数值范围

支持：

1. `atkMin` / `atkMax`。
2. `defMin` / `defMax`。
3. `levelMin` / `levelMax`。

注意：

1. `atk` / `def` 可能为 `-2`，代表 `?`。
2. 连接怪兽没有正常 DEF 和 Level。
3. 魔法/陷阱没有 ATK/DEF/Level。
4. 第一版范围筛选可以让 `NULL` 自然不匹配。
5. 是否把 `?` 作为特殊可选项，可以后续单独加 `includeUnknownStats`。

## 6. API 设计

### 6.1 TypeScript Contract

```ts
export type CardFilterMatchMode = "any" | "all";
export type SetcodeFilterMode = "exact" | "base";

export interface NumericRangeFilter {
  min: number | null;
  max: number | null;
}

export interface StandardCardSearchFilters {
  codes?: number[];
  codeRange?: NumericRangeFilter | null;
  aliases?: number[];
  aliasRange?: NumericRangeFilter | null;
  ots?: Ot[];

  nameContains?: string | null;
  descContains?: string | null;

  primaryTypes?: PrimaryType[];
  races?: Race[];
  attributes?: Attribute[];

  monsterFlags?: MonsterFlag[];
  monsterFlagMatch?: CardFilterMatchMode;

  spellSubtypes?: SpellSubtype[];
  trapSubtypes?: TrapSubtype[];

  pendulumLeftScale?: NumericRangeFilter | null;
  pendulumRightScale?: NumericRangeFilter | null;

  linkMarkers?: LinkMarker[];
  linkMarkerMatch?: CardFilterMatchMode;

  setcodes?: number[];
  setcodeMode?: SetcodeFilterMode;
  setcodeMatch?: CardFilterMatchMode;

  categoryMasks?: number[];
  categoryMatch?: CardFilterMatchMode;

  atk?: NumericRangeFilter | null;
  def?: NumericRangeFilter | null;
  level?: NumericRangeFilter | null;
}

export interface SearchStandardCardsInput {
  keyword: string | null;
  filters: StandardCardSearchFilters | null;
  sortBy: StandardCardSortField;
  sortDirection: SortDirection;
  page: number;
  pageSize: number;
}
```

### 6.2 Rust DTO

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardFilterMatchModeDto {
    Any,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SetcodeFilterModeDto {
    Exact,
    Base,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumericRangeFilterDto {
    pub min: Option<i64>,
    pub max: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StandardCardSearchFiltersDto {
    pub codes: Option<Vec<u32>>,
    pub code_range: Option<NumericRangeFilterDto>,
    pub aliases: Option<Vec<u32>>,
    pub alias_range: Option<NumericRangeFilterDto>,
    pub ots: Option<Vec<Ot>>,

    pub name_contains: Option<String>,
    pub desc_contains: Option<String>,

    pub primary_types: Option<Vec<PrimaryType>>,
    pub races: Option<Vec<Race>>,
    pub attributes: Option<Vec<Attribute>>,

    pub monster_flags: Option<Vec<MonsterFlag>>,
    pub monster_flag_match: Option<CardFilterMatchModeDto>,

    pub spell_subtypes: Option<Vec<SpellSubtype>>,
    pub trap_subtypes: Option<Vec<TrapSubtype>>,

    pub pendulum_left_scale: Option<NumericRangeFilterDto>,
    pub pendulum_right_scale: Option<NumericRangeFilterDto>,

    pub link_markers: Option<Vec<LinkMarker>>,
    pub link_marker_match: Option<CardFilterMatchModeDto>,

    pub setcodes: Option<Vec<u16>>,
    pub setcode_mode: Option<SetcodeFilterModeDto>,
    pub setcode_match: Option<CardFilterMatchModeDto>,

    pub category_masks: Option<Vec<u64>>,
    pub category_match: Option<CardFilterMatchModeDto>,

    pub atk: Option<NumericRangeFilterDto>,
    pub def: Option<NumericRangeFilterDto>,
    pub level: Option<NumericRangeFilterDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchStandardCardsInput {
    pub keyword: Option<String>,
    pub filters: Option<StandardCardSearchFiltersDto>,
    pub sort_by: StandardCardSortFieldDto,
    pub sort_direction: SortDirectionDto,
    pub page: u32,
    pub page_size: u32,
}
```

兼容性规则：

1. `filters` 是可选字段。
2. 旧前端只传 keyword/sort/page 仍然可用。
3. 空数组应在后端 normalize 成无筛选。
4. 空字符串应 trim 后视为无文本筛选。
5. code、alias、setcode 等无符号字段应在后端校验范围后再进入 SQL 参数。

## 7. SQLite Schema 设计

### 7.1 不改 schema 的最小阶段

当前 schema v2 已经可以支持：

1. code 精确值、code 范围、code 子串 keyword。
2. alias 精确值和范围。
3. OT。
4. source language 的 name / desc 文本包含。
5. 主类型。
6. `raw_race` 种族。
7. `raw_attribute` 属性。
8. `raw_type` 怪兽 flags / 魔陷子类型。
9. `category` 效果分类。
10. ATK / DEF / Level。
11. 通过 `raw_level` 解析灵摆刻度，作为短期 fallback。

这个阶段能快速给用户可用价值，但不能完整支持 setcode 和连接箭头。灵摆刻度虽然可以从 `raw_level` 解析，但长期仍应落成语义列或语义表。

### 7.2 推荐 schema v3

建议将 schema version 提升到 3：

```rust
pub const STANDARD_SQLITE_SCHEMA_VERSION: u32 = 3;
```

标准包索引是可重建缓存，不需要做复杂 in-place migration。旧 schema 命中 mismatch 后提示用户 rebuild 即可。

### 7.3 standard_cards 增加语义列

建议 v3 的 `standard_cards`：

```sql
create table standard_cards (
  code integer primary key,
  alias integer not null,
  ot text not null,
  category integer not null,
  primary_type text not null,
  subtype_display text not null,
  race text,
  attribute text,
  spell_subtype text,
  trap_subtype text,
  atk integer,
  def integer,
  level integer,
  raw_type integer not null,
  raw_race integer not null,
  raw_attribute integer not null,
  raw_level integer not null,
  detail_json text not null
);
```

推荐索引：

```sql
create index idx_standard_cards_primary_type
  on standard_cards(primary_type, subtype_display, code);

create index idx_standard_cards_alias
  on standard_cards(alias, code);

create index idx_standard_cards_ot
  on standard_cards(ot, code);

create index idx_standard_cards_race
  on standard_cards(race, code);

create index idx_standard_cards_attribute
  on standard_cards(attribute, code);

create index idx_standard_cards_race_attribute
  on standard_cards(race, attribute, code);

create index idx_standard_cards_stats
  on standard_cards(atk, def, level, code);
```

保留 `raw_*` 字段的原因：

1. 调试 CDB 导入。
2. 保持与现有实现兼容。
3. 允许未来做精确 raw bit 查询。

### 7.4 standard_card_monster_flags

```sql
create table standard_card_monster_flags (
  code integer not null,
  flag text not null,
  primary key (code, flag)
);

create index idx_standard_card_monster_flags_flag
  on standard_card_monster_flags(flag, code);
```

该表从 `CardEntity.monster_flags` 填充。

### 7.5 standard_card_setcodes

```sql
create table standard_card_setcodes (
  code integer not null,
  setcode integer not null,
  base integer not null,
  primary key (code, setcode)
);

create index idx_standard_card_setcodes_setcode
  on standard_card_setcodes(setcode, code);

create index idx_standard_card_setcodes_base
  on standard_card_setcodes(base, code);
```

该表从 `CardEntity.setcodes` 填充。

### 7.6 standard_card_pendulum

```sql
create table standard_card_pendulum (
  code integer primary key,
  left_scale integer not null,
  right_scale integer not null
);

create index idx_standard_card_pendulum_left
  on standard_card_pendulum(left_scale, code);

create index idx_standard_card_pendulum_right
  on standard_card_pendulum(right_scale, code);
```

该表从 `CardEntity.pendulum` 填充。没有灵摆数据的卡片不写入该表。

### 7.7 standard_card_link_markers

```sql
create table standard_card_link_markers (
  code integer not null,
  marker text not null,
  primary key (code, marker)
);

create index idx_standard_card_link_markers_marker
  on standard_card_link_markers(marker, code);
```

该表从 `CardEntity.link.markers` 填充。没有连接数据的卡片不写入该表。

### 7.8 FTS 保持现状

本次不搜索提示文本，因此 `standard_card_search_fts` 不需要加入 `prompt_text`。FTS 继续覆盖：

```sql
create virtual table standard_card_search_fts using fts5(
  code unindexed,
  language unindexed,
  name,
  card_desc,
  primary_type,
  subtype_display
);
```

仍然必须保留 `instr(lower(...), ?)` 子串路径。

## 8. Rebuild 写入流程变更

标准包 rebuild 时应按以下顺序构建索引：

1. 从 CDB 读取 `CardEntity`。
2. 将 `default` 文本映射到配置的 source language。
3. 派生 `CardListRow`。
4. 插入 `standard_cards`。
5. 插入 `standard_card_texts`。
6. 插入 `standard_card_list_rows`。
7. 插入 `standard_card_search_fts`。
8. 插入 `standard_assets`。
9. 插入 `standard_card_monster_flags`。
10. 插入 `standard_card_setcodes`。
11. 插入 `standard_card_pendulum`。
12. 插入 `standard_card_link_markers`。
13. 插入 strings/baseline/setname baseline 等现有表。

新增校验：

1. `standard_cards` 行数等于卡片数量。
2. `standard_card_list_rows` 行数等于 source language 下卡片数量。
3. `standard_card_search_fts` 行数等于卡片数量。
4. `standard_assets` 行数等于卡片数量。
5. `standard_card_setcodes` 行数等于所有卡片 setcode 槽位总数。
6. `standard_card_monster_flags` 行数等于所有怪兽 flag 总数。
7. `standard_card_pendulum` 行数等于所有灵摆卡数量。
8. `standard_card_link_markers` 行数等于所有连接箭头总数。

## 9. 查询架构

### 9.1 为什么需要 SQL Builder

当前 `load_card_list_page` 是两个静态查询分支：

1. keyword 为空。
2. keyword 非空。

高级搜索需要可选 join、可选 where、动态 `IN (...)`、`any/all` 子查询，因此应重构为小型 SQL builder。

### 9.2 Query Builder 建议结构

```rust
struct StandardCardSqlQuery {
    joins: Vec<String>,
    clauses: Vec<String>,
    params: Vec<rusqlite::types::Value>,
}
```

规则：

1. 用户输入只能进入参数绑定。
2. 表名、列名、排序表达式只能来自硬编码白名单。
3. count query 和 page query 必须来自同一个 query object。
4. 空筛选不生成 SQL。
5. 不生成非法 `IN ()`。
6. `ORDER BY` 只能使用后端枚举映射。

### 9.3 基础 SQL

```sql
from standard_card_list_rows r
join standard_cards c on c.code = r.code
where r.language = ?
```

列表页 projection 仍然返回列表行字段：

```sql
select r.code, r.name, r.desc, r.primary_type, r.subtype_display,
       r.atk, r.def, r.level,
       r.has_image, r.has_script, r.has_field_image
```

这里的 `has_*` 字段只是维持现有列表显示，不作为高级搜索筛选条件。

### 9.4 Keyword Clause

```sql
and (
  instr(cast(r.code as text), ?) > 0
  or instr(lower(r.name), ?) > 0
  or instr(lower(r.desc), ?) > 0
  or instr(lower(r.subtype_display), ?) > 0
  or instr(lower(r.primary_type), ?) > 0
  or exists (
    select 1
    from standard_card_search_fts f
    where f.language = r.language
      and f.code = cast(r.code as text)
      and f match ?
  )
)
```

### 9.5 Code / Alias / OT

精确 code：

```sql
and c.code in (?, ?, ...)
```

code 范围：

```sql
and c.code >= ?
and c.code <= ?
```

精确 alias：

```sql
and c.alias in (?, ?, ...)
```

alias 范围：

```sql
and c.alias >= ?
and c.alias <= ?
```

OT：

```sql
and c.ot in (?, ?, ...)
```

### 9.6 Name / Desc Contains

```sql
and instr(lower(r.name), ?) > 0
and instr(lower(r.desc), ?) > 0
```

`nameContains` 只匹配卡名，`descContains` 只匹配效果文本。它们比 `keyword` 更窄。

### 9.7 主类型

```sql
and c.primary_type in (?, ?, ...)
```

### 9.8 种族和属性

v3 推荐：

```sql
and c.race in (?, ?, ...)
and c.attribute in (?, ?, ...)
```

v2 fallback：

```sql
and c.raw_race in (?, ?, ...)
and c.raw_attribute in (?, ?, ...)
```

### 9.9 怪兽 Flags

`any`：

```sql
and exists (
  select 1
  from standard_card_monster_flags mf
  where mf.code = r.code
    and mf.flag in (?, ?, ...)
)
```

`all`：

```sql
and r.code in (
  select code
  from standard_card_monster_flags
  where flag in (?, ?, ...)
  group by code
  having count(distinct flag) = ?
)
```

### 9.10 魔法/陷阱子类型

v3 推荐：

```sql
and c.spell_subtype in (?, ?, ...)
and c.trap_subtype in (?, ?, ...)
```

前端可以根据主类型控制显示，但后端仍应把它当成普通筛选条件处理。

### 9.11 灵摆刻度

```sql
and exists (
  select 1
  from standard_card_pendulum p
  where p.code = r.code
    and p.left_scale >= ?
    and p.left_scale <= ?
    and p.right_scale >= ?
    and p.right_scale <= ?
)
```

实现时应按实际传入的范围只生成对应比较条件。只筛选左刻度时，不应要求右刻度条件存在；反之亦然。

### 9.12 连接箭头

`any`：

```sql
and exists (
  select 1
  from standard_card_link_markers lm
  where lm.code = r.code
    and lm.marker in (?, ?, ...)
)
```

`all`：

```sql
and r.code in (
  select code
  from standard_card_link_markers
  where marker in (?, ?, ...)
  group by code
  having count(distinct marker) = ?
)
```

### 9.13 Setcode

`exact + any`：

```sql
and exists (
  select 1
  from standard_card_setcodes sc
  where sc.code = r.code
    and sc.setcode in (?, ?, ...)
)
```

`base + any`：

```sql
and exists (
  select 1
  from standard_card_setcodes sc
  where sc.code = r.code
    and sc.base in (?, ?, ...)
)
```

`exact + all`：

```sql
and r.code in (
  select code
  from standard_card_setcodes
  where setcode in (?, ?, ...)
  group by code
  having count(distinct setcode) = ?
)
```

`base + all`：

```sql
and r.code in (
  select code
  from standard_card_setcodes
  where base in (?, ?, ...)
  group by code
  having count(distinct base) = ?
)
```

### 9.14 Category

后端先合并 mask：

```rust
let combined = masks.iter().fold(0u64, |acc, mask| acc | mask);
```

`any`：

```sql
and (c.category & ?) != 0
```

`all`：

```sql
and (c.category & ?) = ?
```

### 9.15 数值范围

```sql
and c.atk >= ?
and c.atk <= ?
and c.def >= ?
and c.def <= ?
and c.level >= ?
and c.level <= ?
```

如果字段为 `NULL`，SQL 比较会自然不匹配。

### 9.16 Count 和 Page 查询

Count：

```sql
select count(*)
from ...
where ...
```

Page：

```sql
select ...
from ...
where ...
order by {order_by}
limit ?
offset ?
```

`order_by` 白名单继续来自 `StandardCardSortFieldDto`：

1. code asc / desc。
2. name asc / desc。
3. type asc / desc。

ATK/DEF/Level 排序可以后续补。

## 10. 前端设计

### 10.1 入口

标准包 Cards tab 保留现有：

1. 搜索输入框。
2. 排序 select。

新增：

1. 筛选按钮。
2. 高级筛选面板。
3. 已启用筛选 chips。
4. 清空所有筛选按钮。

每次筛选变化应重置 page 到 `1`。

### 10.2 筛选面板分组

推荐分组：

1. 编号
   - code 精确值 / 范围。
   - alias 精确值 / 范围。
   - OT。

2. 文本
   - 卡名包含。
   - 效果文本包含。

3. 类型
   - 主类型。
   - 怪兽 flags。
   - 魔法子类型。
   - 陷阱子类型。

4. 怪兽数据
   - 种族。
   - 属性。
   - ATK / DEF / Level 范围。
   - 灵摆左/右刻度范围。
   - 连接箭头 any/all。

5. 系列
   - setname picker。
   - exact/base toggle。
   - any/all toggle。

6. 效果分类
   - 使用现有 `CARD_CATEGORY_OPTIONS`。
   - any/all toggle。

### 10.3 Setname Picker

复用现有标准包 setname API：

```ts
standardPackApi.listSetnames({ language })
```

能力：

1. 按 setname 显示名搜索。
2. 按 hex key 搜索。
3. 允许输入标准包未收录的自定义 hex setcode。
4. 显示名称和 hex。
5. 支持 exact/base 模式。

### 10.4 请求策略

高级搜索会增加请求频率。推荐：

1. enum/chip/toggle 变化后立即查询。
2. keyword、nameContains、descContains 做 250ms debounce。
3. 或者第一版使用“应用筛选”按钮，降低复杂度。

更推荐第一版：

1. 结构化筛选即时生效。
2. 文本输入 debounce。

### 10.5 空状态

空状态应区分：

1. 标准包未配置或无索引。
2. 标准包索引为空。
3. 当前搜索/筛选无结果。

第三种应提示用户调整或清空筛选。

## 11. 实施计划

### Phase 1：API 与 SQL Builder

任务：

1. 新增 `StandardCardSearchFiltersDto`。
2. 给 `SearchStandardCardsInput` 增加 `filters`。
3. 对空数组、空字符串做 normalize。
4. 将 `load_card_list_page` 重构为 SQL builder。
5. 实现 v2 可支持筛选：
   - code / alias / OT
   - name contains / desc contains
   - primary type
   - race
   - attribute
   - monster flags
   - spell/trap subtype
   - category
   - numeric ranges
   - pendulum scales via `raw_level` fallback
6. 保持现有 keyword 行为。
7. 增加后端测试。

结果：

不要求用户重建索引，也能先得到大部分结构化搜索能力。

### Phase 2：Schema v3，支持 setcode、灵摆和连接

任务：

1. bump `STANDARD_SQLITE_SCHEMA_VERSION` 到 `3`。
2. `standard_cards` 增加语义列。
3. 新增 `standard_card_monster_flags`。
4. 新增 `standard_card_setcodes`。
5. 新增 `standard_card_pendulum`。
6. 新增 `standard_card_link_markers`。
7. rebuild 时填充新表。
8. 增加行数校验。
9. 实现 setcode exact/base 搜索。
10. 实现灵摆刻度搜索。
11. 实现连接箭头 any/all 搜索。
12. 增加集成测试。

结果：

标准包高级搜索覆盖除提示文本和资源状态以外的卡片编辑数据字段。

### Phase 3：前端高级筛选 UI

任务：

1. 从 `CardInfoForm` 抽出共享卡片枚举选项。
2. 新增 `StandardCardAdvancedSearchPanel`。
3. 扩展 `CardBrowserPanel`，允许插入高级筛选控件或 toolbar slot。
4. 在 `StandardPackView.loadStandardPage` 中传入 filters。
5. 显示 active filter chips。
6. 实现 clear-all。
7. 补 i18n 文案。

结果：

作者可以在标准包 Cards tab 里通过结构化条件找官方参考卡。

### Phase 4：复用和增强

任务：

1. 增加 ATK/DEF/Level 排序。
2. 考虑保存最近筛选条件或 presets。
3. 将同一套 filter DTO 复用于 custom pack。
4. 如果标准包和 custom pack 能力不同，增加 capability metadata。

## 12. 测试计划

### 12.1 后端单元测试

覆盖：

1. `filters = None` 保持旧行为。
2. 空数组不产生筛选。
3. 主类型筛选。
4. 种族筛选。
5. 属性筛选。
6. 怪兽 flag any/all。
7. 魔法子类型筛选。
8. 陷阱子类型筛选。
9. category any/all。
10. ATK/DEF/Level 范围。
11. code / alias / OT。
12. name contains / desc contains。
13. 灵摆刻度范围。
14. 连接箭头 any/all。
15. SQL builder 不生成 `IN ()`。
16. 参数通过绑定传入，而不是字符串拼接。

### 12.2 SQLite rebuild 测试

schema v3 后覆盖：

1. rebuild 写入 `standard_card_setcodes`。
2. rebuild 写入 `standard_card_monster_flags`。
3. rebuild 写入 `standard_card_pendulum`。
4. rebuild 写入 `standard_card_link_markers`。
5. 行数校验能发现漏写。
6. schema mismatch 能提示 rebuild required。
7. setcode exact 搜索。
8. setcode base 搜索。
9. setcode all 搜索。
10. 灵摆刻度范围搜索。
11. 连接箭头 any/all 搜索。

### 12.3 集成测试数据建议

在 `src-tauri/tests/standard_pack.rs` 中构造一个小 CDB：

1. DARK Dragon Effect monster，带 Search category。
2. LIGHT Warrior Normal monster。
3. Quick-Play Spell。
4. Counter Trap。
5. 一张卡带 setcode `0x345`。
6. 一张卡带 setcode `0x1345`。
7. 一张灵摆怪兽，左右刻度不同。
8. 一张连接怪兽，包含多个连接箭头。
9. 一张 alias 指向其他编号的卡。

断言：

1. race + attribute 只返回目标卡。
2. category all 只返回同时包含所有 bit 的卡。
3. setcode exact `0x345` 不误匹配 `0x1345`。
4. setcode base `0x345` 同时匹配 `0x345` 和 `0x1345`。
5. 灵摆刻度范围只返回目标卡。
6. 连接箭头 all 只返回同时包含所有所选箭头的卡。
7. alias / OT 过滤返回正确卡。
8. keyword + structured filters 使用 `AND` 组合。
9. 筛选后的 total 和分页正确。

### 12.4 前端手动检查

第一版可以手动检查：

1. 筛选面板能打开/关闭。
2. active chips 和当前筛选一致。
3. 删除单个 chip 后结果刷新。
4. clear all 后恢复无筛选结果。
5. 筛选变化后 page 回到 1。
6. setname picker 能选择官方 setname。
7. 文本输入不会产生过量请求。
8. 筛选无结果时空状态合理。

## 13. 风险与缓解

### 13.1 CDB bit 编码泄漏

风险：

repository 层重复理解 `raw_type`、`raw_race`、`raw_attribute`。

缓解：

1. schema v3 增加语义列和规范化表。
2. `raw_*` 只作为兼容、调试、诊断字段。
3. 如果 Phase 1 需要 v2 fallback，把 raw bit 映射集中到一个 helper 中。

### 13.2 FTS 对中文/日文不稳定

风险：

FTS tokenization 不一定符合中文/日文子串搜索直觉。

缓解：

1. 保留 `instr(lower(...), needle)`。
2. FTS 只作为额外命中路径。
3. 增加中文/日文子串搜索测试。

### 13.3 Setcode 语义歧义

风险：

作者搜索“系列”时可能期望包含子系列，而 exact setcode 不会包含。

缓解：

1. UI 明确 exact/base。
2. 如果 label 是“系列”，默认使用 `base`。
3. 显示 setname 和 hex。

### 13.4 Category 数据质量

风险：

源 CDB 的 `datas.category` 可能不完整或不准确。

缓解：

1. UI 称为“效果分类标签”或沿用当前“效果分类”但避免暗示脚本语义分析。
2. 不从 Lua 推断效果。
3. 必要时在详情中显示 raw category hex。

### 13.5 动态 SQL 复杂度

风险：

count/page 条件不一致、参数顺序错、`OR` 括号错误。

缓解：

1. count/page 共用 query object。
2. 每个 filter clause builder 单独测试。
3. 排序表达式白名单。
4. 对 `any/all` 子查询分别做单元测试。

### 13.6 Rebuild 要求

风险：

schema v3 会让旧标准包索引进入 schema mismatch。

缓解：

1. 复用现有 missing/stale/schema-mismatch 状态。
2. UI 明确提示 rebuild required。
3. rebuild 继续作为后台 job。

## 14. 推荐第一里程碑

考虑到本次范围要求覆盖除提示文本和资源状态以外的全部卡片编辑数据字段，推荐第一里程碑包含：

1. `filters` API contract。
2. SQL builder 重构。
3. code / alias / OT / name / desc filters。
4. primary type / race / attribute / subtype / monster flag / category / numeric filters。
5. schema v3 的 `standard_card_setcodes`、`standard_card_pendulum`、`standard_card_link_markers`。
6. setcode exact/base 搜索。
7. 灵摆刻度和连接箭头搜索。
8. 标准包 Cards tab 基础高级筛选 UI。

本阶段明确不包含提示文本搜索和资源状态筛选。

## 15. 待定问题

本节是设计阶段提出的问题。首版实现已经确定其中一部分默认策略。

已确定：

1. setcode 默认 `base`。
2. category 默认 `any`。
3. monster flags 默认 `any`。
4. 连接箭头默认 `any`。
5. filter DTO 首版命名为 `StandardCardSearchFilters`，暂不提前抽成通用 `CardSearchFilters`。

仍待后续评估：

1. 数值范围是否需要包含 `?` 的单独开关。
2. 灵摆刻度筛选是否需要支持“左右刻度相等/不等”快捷条件。
3. 高级搜索是否需要保存 presets。
4. custom pack 是否复用同一套 filter DTO 和 UI。

## 16. 后续工作

首版已经完成“标准包高级搜索可用闭环”，但仍有一些值得继续推进的方向。

### 16.1 搜索能力增强

1. 增加 ATK / DEF / Level 排序。
   - 当前排序仍保持 code / name / type。
   - 如果用户在筛选数值字段后还想比较数值大小，ATK / DEF / Level 排序会很有帮助。

2. 增加 `includeUnknownStats`。
   - 当前范围比较让 `NULL` 自然不匹配。
   - `-2` 代表 `?`，首版不做特殊处理。
   - 后续可以让用户显式选择“包含 ? 攻/守”。

3. 增加灵摆刻度快捷条件。
   - 例如“左右刻度相等”。
   - 例如“左右刻度不同”。
   - 这类条件应作为额外 DTO 字段，而不是复用 min/max 范围表达。

4. 增加 raw category display。
   - 当前 UI 使用效果分类标签。
   - 后续可以在 chip 或 tooltip 中显示 raw hex mask，方便校验 CDB 数据质量。

5. 评估更多文本搜索能力。
   - 首版不搜索 `str1` - `str16`。
   - 如果作者确实需要提示文本搜索，应单独设计 prompt-text index 和 UI 分组。

### 16.2 UI 和交互增强

1. 保存最近筛选条件。
   - 可以保存最近一次 Standard Pack filters。
   - 也可以做用户命名 presets。
   - 需要注意 presets 应随 schema / DTO 版本演进。

2. 优化 active chips 的表达。
   - 当前 chips 只显示启用项。
   - 后续可以把 `any/all` 和 `exact/base` 作为 chip 的附加小标签展示。
   - 但不应在没有实际筛选值时显示默认 match mode。

3. 增加更强的 setname picker。
   - 当前支持名称、hex、自定义 hex。
   - 后续可以显示 base/exact 解释、官方 setname 来源、子系列提示。

4. 增加键盘体验。
   - modal 目前支持 Esc 关闭。
   - 后续可以增加 tab keyboard navigation、焦点循环和更细的 aria 标记。

5. 优化移动端布局。
   - 首版 modal 在窄屏下改为上方横向 tab。
   - 后续可以为小屏做更密集的两级折叠布局。

### 16.3 后端和性能增强

1. 为 SQL builder 增加更细粒度单元测试。
   - 首版通过 repository 集成测试覆盖行为。
   - 后续可以对单个 clause builder 做 snapshot-like SQL/params 测试。

2. 增加 query performance diagnostics。
   - 可以手动或测试环境输出查询耗时。
   - 对 setcode all、link marker all、monster flag all 这类 group by 查询特别有价值。

3. 评估 SQLite query plan。
   - 首版已经为新表建立 `(filter_value, code)` 索引。
   - 后续可使用 `EXPLAIN QUERY PLAN` 检查真实大库表现。
   - 如果 all 查询成为热点，可以考虑额外索引或 materialized bitmask。

4. 考虑 strings search SQLite 化。
   - 当前 strings browser 仍在 repository 中加载 rows 后在 Rust 侧过滤和排序。
   - 这不属于本次高级搜索范围，但长期可以沿用同一套 SQL builder 思路。

### 16.4 复用到 custom pack

1. 抽象通用 `CardSearchFilters`。
   - 首版保守命名为 `StandardCardSearchFilters`。
   - custom pack 若需要复用，应先确认能力差异。

2. 给不同来源增加 capability metadata。
   - Standard Pack 有 SQLite v3 查询表。
   - custom pack 当前是 JSON/session 模型。
   - 前端可根据 capabilities 显示或隐藏某些筛选项。

3. 避免把 custom pack 主存储改成 SQLite。
   - custom pack 是 author-state document。
   - Standard Pack 是 read-only reference index。
   - 即使复用 DTO，也不应强行统一存储模型。

### 16.5 测试和验收补充

1. 增加前端组件测试或轻量 UI 自动化。
   - 覆盖 modal 打开/关闭。
   - 覆盖 tab 切换。
   - 覆盖 chips 只显示启用筛选。
   - 覆盖 clear all。

2. 增加中文/日文子串搜索回归测试。
   - 设计中已经强调不能只依赖 FTS。
   - 后续可用 fixture 明确覆盖 CJK contains behavior。

3. 增加真实大库手动验收 checklist。
   - 重建 v3 索引。
   - 搜索常见系列。
   - 搜索 link marker all。
   - 搜索 category all。
   - 搜索 name/desc 中文或日文子串。

## 17. 首版接受标准完成情况

1. 结构化 DTO：已完成。
2. SQLite schema v3：已完成。
3. rebuild 写入新查询表：已完成。
4. 动态 SQL builder：已完成。
5. keyword 兼容子串搜索：已保留。
6. keyword + filters `AND` 组合：已完成。
7. setcode exact/base：已完成。
8. monster flags any/all：已完成。
9. link markers any/all：已完成。
10. category any/all：已完成。
11. 标准包高级筛选 UI：已完成。
12. 筛选 modal + tabs：已完成。
13. active chips：已完成。
14. clear all：已完成。
15. 三语文案：已完成。
16. 后端测试：已完成。
17. 前端 typecheck/build：已完成。
