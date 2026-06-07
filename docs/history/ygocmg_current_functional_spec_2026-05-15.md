# YGOCMG 当前功能规范

日期：2026-05-15  
状态：Current Implementation Spec  
适用范围：当前 `main` 分支实现，至提交 `ae80e2d feat: add light/dark theme system with high contrast and custom brand color`

关联历史文档：

1. [YGOCMG 首版功能规范 v1](./ygocmg_v1_functional_spec_2026-04-25.md)
2. [YGOCMG v1 UI Specification](./ygocmg_v1_ui_spec_2026-04-26.md)
3. [Card Text Language Design](./card_text_language_design.md)
4. [Standard Pack Queryable Index Refactor Plan](./standard_pack_queryable_index_refactor_plan_2026-05-01.md)
5. [标准包高级搜索设计文档](./standard_pack_advanced_search_design_2026-05-01.md)
6. [YGOCMG UI 主题系统设计文档](./ui_theme_system_design_2026-05-06.md)

## 1. 文档目标

本文档用于重新描述 YGOCMG 的当前功能规范。它不是 2026-04-25 旧文档的“计划版补丁”，而是以当前代码、命令接口、数据模型和近期提交历史为依据的实现态说明。

本文档回答以下问题：

1. YGOCMG 当前是什么产品。
2. 当前支持哪些用户流程。
3. `workspace / pack / card / strings / resource / standard pack / import / export / config` 的功能边界是什么。
4. 当前数据模型和校验规则如何约束用户输入。
5. 哪些原首版设想已经实现、改变、扩展或尚未实现。

本文档不展开：

1. 前端组件拆分细节。
2. 后端分层架构细节。
3. SQLite 内部 schema 的完整字段。
4. UI 视觉 token 的具体配色值。
5. 未来路线图。

## 2. 当前产品定位

YGOCMG 是一个面向 YGOPro 自定义卡作者的桌面端卡包管理与编辑工具。当前实现已经从“最小作者态卡包编辑器”扩展为一个包含标准包参考、语言目录、导入导出、资源管理、高级搜索和主题配置的作者工具。

当前核心目标是：

1. 管理多个本地作者工作区。
2. 在工作区内管理多个自定义卡包。
3. 使用语义化卡片模型编辑自定义卡。
4. 管理卡片多语言文本、卡包级字符串、卡图、场地图和脚本资源。
5. 只读接入标准包，提供浏览、字符串参考、setname 参考、编号和字符串命名空间校验。
6. 从 YGOPro 运行时资源导入为作者态卡包。
7. 将一个或多个已打开的作者态卡包导出为 YGOPro 可用的运行时资源目录。
8. 提供多语言 UI、文本语言目录、深浅色主题、高对比度和品牌色设置。

## 3. 功能演进依据

本规范参考了当前代码和近期提交历史。关键功能演进包括：

1. `bbcbeed`：实现 P3.5 统一确认流。
2. `ea1afeb`：实现 P4 卡包字符串和卡片资源管理。
3. `4af4299`：实现 P7 标准包只读接入。
4. `3d07241`、`19b8f1c`：实现 P8 导入后端和前端向导。
5. `867a075`：实现 P9 导出流程。
6. `5135701`：实现受管卡片文本语言。
7. `c3a91d8`：实现语义化 category 编辑器。
8. `a26a69e`：将 `setcode` 从 opaque `u64` 重构为结构化 `Vec<u16>` 多槽模型。
9. `6149a5a` 到 `7d1d4c2`：完善通知、i18n 基础和三语 UI 文案。
10. `881ebf1` 到 `bdba7c6`：标准包 repository、SQLite 查询索引和缓存重构。
11. `015b2fd`、`9cc4257`：标准包和自定义包高级搜索。
12. `9530f78`：为 Pack 元数据增加 `pack_code`。
13. `10e4747`：层级返回导航和未保存关闭确认。
14. `970cb03`：可折叠 pack sidebar。
15. `ae80e2d`：深浅色主题、高对比度和自定义品牌色。

## 4. 核心术语

### 4.1 Workspace

作者态工作区。一个工作区是磁盘上的目录，负责承载多个自定义卡包，并记录打开会话状态。

### 4.2 Custom Pack

工作区内的作者态卡包。自定义卡包可编辑，拥有元数据、卡片、字符串、图片和脚本资源。

### 4.3 Standard Pack

由全局 YGOPro 路径和标准包源语言配置生成的只读参考索引。标准包不属于任何工作区，不可编辑，通过应用数据目录下的 SQLite 索引查询。

### 4.4 CardEntity

作者态卡片的持久化实体。`id` 是内部稳定身份，`code` 是 YGOPro 业务编号。

### 4.5 Pack Strings

卡包级 `strings.conf` 字符串集合，支持 `setname`、`counter`、`victory` 和兼容读取 `system`。自定义包正常作者流程禁止新增或修改 `system` 字符串。

### 4.6 Text Language Catalog

全局配置中的文本语言目录。卡片文本、卡包显示语言、默认导出语言、导入源语言和标准包源语言都通过该目录受管。

### 4.7 Preview Token / Confirmation Token

`preview_token` 用于导入导出两阶段流程，预检后执行。  
`confirmation_token` 用于带 warning 的写入确认，例如卡片保存或字符串覆盖。

### 4.8 Job

后台任务。当前用于标准包索引重建、导入执行、导出执行，并可查询任务状态。

## 5. 全局配置规范

全局配置存储在应用数据目录，不属于任何工作区。

当前配置模型为：

```ts
type ThemeMode = "system" | "light" | "dark";
type TextLanguageKind = "builtin" | "custom";

interface TextLanguageProfile {
  id: string;
  label: string;
  kind: TextLanguageKind;
  hidden: boolean;
  last_used_at: string | null;
}

interface GlobalConfig {
  app_language: string;
  ygopro_path: string | null;
  external_text_editor_path: string | null;

  custom_code_recommended_min: number;
  custom_code_recommended_max: number;
  custom_code_min_gap: number;

  shell_sidebar_width: number;
  shell_sidebar_collapsed: boolean;
  shell_window_width: number;
  shell_window_height: number;
  shell_window_is_maximized: boolean;

  text_language_catalog: TextLanguageProfile[];
  standard_pack_source_language: string | null;

  theme_mode: ThemeMode;
  high_contrast: boolean;
  custom_brand_color: string | null;
}
```

### 5.1 UI 语言

当前应用 UI 支持：

1. `en-US`
2. `ja-JP`
3. `zh-CN`

保存配置时，`app_language` 必须是上述三者之一。

### 5.2 YGOPro 路径

`ygopro_path` 用于标准包索引源发现。未配置时，标准包状态为 `not_configured`。路径不存在时，配置保存可给出 warning，标准包状态会进入不可用或缺失源状态。

### 5.3 外部文本编辑器

`external_text_editor_path` 用于打开自定义包脚本和标准包脚本。外部编辑器修改脚本文件不纳入 YGOCMG 程序内回滚范围。

### 5.4 编号策略配置

默认配置：

1. 推荐自定义卡编号下限：`100000000`
2. 推荐自定义卡编号上限：`200000000`
3. 最小编号间距：`5`
4. 标准包保留上限：`99999999`
5. YGOPro 硬上限：`268435455`

配置要求：

1. 推荐下限不得大于推荐上限。
2. 最小编号间距不得为 `0`。
3. 自动推荐编号只从推荐区间中寻找可用编号。

### 5.5 Shell 状态

当前持久化：

1. 侧边栏宽度。
2. 侧边栏折叠状态。
3. 普通窗口宽度和高度。
4. 窗口最大化状态。

约束：

1. 侧边栏宽度保存范围为 `140 - 280`。
2. 窗口宽度最小为 `960`。
3. 窗口高度最小为 `640`。

### 5.6 文本语言目录

默认内置文本语言包括：

1. `zh-CN`
2. `en-US`
3. `ja-JP`
4. `ko-KR`
5. `es-ES`

语言目录规则：

1. `LanguageCode` 在持久化文件和 API 中仍是开放字符串。
2. 正常新建和编辑流程只能使用全局语言目录中的可见语言。
3. `default` 是 legacy/parser 兼容语言，不允许新作者流程创建。
4. 已存在包中的未知非 `default` 语言可以被读取并在无关保存中保留。
5. 自定义语言推荐使用 `x-*` 形式。
6. 语言 id 不得包含文件系统不友好的字符，例如 `/ \ : * ? " < > |`。

### 5.7 标准包源语言

`standard_pack_source_language` 是标准包索引重建的必需配置。未配置时标准包状态为 `missing_language`，重建会失败。

标准包解析出的 legacy `default` 文本会在重建时映射为该源语言。

### 5.8 主题与外观

当前支持：

1. `theme_mode = system | light | dark`
2. `high_contrast`
3. `custom_brand_color`

主题由前端应用到根节点。`theme_mode` 是用户配置，实际显示主题由系统偏好和配置共同解析。`custom_brand_color` 必须是合法 `#RRGGBB` 格式。

## 6. Workspace 规范

### 6.1 职责

Workspace 负责：

1. 作为多个自定义卡包的磁盘容器。
2. 持久化卡包顺序。
3. 持久化上次打开的自定义卡包列表。
4. 持久化上次激活的自定义卡包。
5. 提供工作区级编号和字符串命名空间校验上下文。

标准包不存放在 workspace 内。

### 6.2 数据模型

```ts
interface WorkspaceMeta {
  id: string;
  name: string;
  description: string | null;
  created_at: string;
  updated_at: string;
  pack_order: string[];
  last_opened_pack_id: string | null;
  open_pack_ids: string[];
}

interface WorkspaceFile {
  schema_version: 1;
  data: WorkspaceMeta;
}
```

### 6.3 磁盘结构

```text
<workspace>/
  workspace.json
  packs/
    <storage-name>/
      metadata.json
      cards.json
      strings.json
      pics/
        <code>.jpg
        field/
          <code>.jpg
      scripts/
        c<code>.lua
```

说明：

1. `workspace.json` 的 `schema_version` 当前为 `1`。
2. `metadata.json` 和 `cards.json` 的 `schema_version` 当前为 `1`。
3. `strings.json` 当前使用 `schema_version = 2`。
4. 自定义包作者态脚本目录为 `scripts/`。
5. 导出运行时目录使用 YGOPro 风格 `script/`。

### 6.4 程序级工作区注册表

应用数据目录中保存：

```ts
interface WorkspaceRegistryEntry {
  workspace_id: string;
  path: string;
  name_cache: string | null;
  last_opened_at: string | null;
}

interface WorkspaceRegistryFile {
  schema_version: 1;
  workspaces: WorkspaceRegistryEntry[];
}
```

注册表用于最近工作区和启动恢复。打开或创建工作区会 upsert 注册表并按 `last_opened_at` 倒序排列。

### 6.5 当前支持的工作区操作

后端命令支持：

1. 初始化并加载全局配置。
2. 列出最近工作区。
3. 创建工作区。
4. 打开工作区。
5. 删除工作区记录。
6. 删除工作区磁盘目录。

前端主流程支持：

1. 启动时尝试恢复最近打开的工作区。
2. 恢复 `open_pack_ids` 中的自定义包。
3. 恢复 `last_opened_pack_id` 对应的激活包。
4. 若路径或卡包已失效，则跳过恢复。

创建工作区要求目标目录不存在，或存在但为空。

## 7. Pack 规范

### 7.1 Pack 分类

```ts
type PackKind = "standard" | "custom";
```

当前持久化在 workspace 内的只有 `custom` 包。`standard` 由标准包只读视图提供。

### 7.2 自定义包元数据

```ts
interface PackMetadata {
  id: string;
  kind: "custom";
  name: string;
  pack_code: string | null;
  author: string;
  version: string;
  description: string | null;
  created_at: string;
  updated_at: string;
  display_language_order: string[];
  default_export_language: string | null;
}

interface PackMetadataFile {
  schema_version: 1;
  data: PackMetadata;
}
```

### 7.3 `pack_code`

`pack_code` 是当前新增的卡包短代码字段，用于 UI 识别和未来扩展。规则：

1. 可为空。
2. 保存时 trim 后转大写。
3. 最大长度为 `12`。
4. 只能包含 ASCII 大写字母和数字。

### 7.4 Pack 元数据校验

硬性错误：

1. `name` 为空。
2. `author` 为空。
3. `version` 为空。
4. `pack_code` 非法。
5. 语言字段引用了目录不允许的新语言。

Warning：

1. `display_language_order` 为空。
2. `default_export_language` 不在显示语言顺序中。

### 7.5 当前支持的 Pack 操作

1. 创建空白自定义包。
2. 打开自定义包。
3. 关闭已打开自定义包。
4. 切换激活自定义包。
5. 编辑自定义包元数据。
6. 删除自定义包磁盘目录。
7. 列出当前工作区内所有自定义包摘要。
8. 从运行时资源导入新自定义包。
9. 将一个或多个已打开自定义包导出为运行时资源。

说明：

1. 当前导出要求目标包已经打开，因为导出预检从 `PackSession` 读取包快照。
2. 创建包后前端会组合调用 `create_pack` 和 `open_pack`，让新包进入打开状态。
3. 当前没有命令级“批量移动卡片到另一个包”接口。
4. 当前没有一键合并多个作者态包为新作者态包的功能。

### 7.6 Pack 会话

打开自定义包时会构建 `PackSession`：

```text
PackSession
  pack_id
  pack_path
  revision
  source_stamp
  metadata
  cards
  strings
  asset_index
  card_list_cache
```

会话规则：

1. 写入成功后 `revision` 递增。
2. `source_stamp` 由元数据更新时间、`cards.json` 和 `strings.json` 文件状态生成。
3. `confirmation_token` 和 `preview_token` 使用 revision/source stamp 识别过期。
4. 切换 workspace 会清空已打开包会话。

## 8. Card 模型规范

### 8.1 当前 CardEntity

```ts
type PrimaryType = "monster" | "spell" | "trap";
type Ot = "ocg" | "tcg" | "custom";

interface CardEntity {
  id: string;
  code: number;
  alias: number;
  setcodes: number[];
  ot: Ot;
  category: number;
  primary_type: PrimaryType;
  texts: Record<string, CardTexts>;

  monster_flags: MonsterFlag[] | null;
  atk: number | null;
  def: number | null;
  race: Race | null;
  attribute: Attribute | null;
  level: number | null;
  pendulum: Pendulum | null;
  link: LinkData | null;

  spell_subtype: SpellSubtype | null;
  trap_subtype: TrapSubtype | null;

  created_at: string;
  updated_at: string;
}
```

### 8.2 与旧文档的主要差异

当前实现与 2026-04-25 旧规范相比有以下关键变化：

1. `setcode: number` 已变更为 `setcodes: number[]`。
2. `category` 当前允许 `u32` 范围内的 bit mask，DTO 使用 `u64` 承载但校验不得超过 `u32::MAX`。
3. `Ot` 当前只有 `ocg | tcg | custom`，没有 `none`、`ocg_tcg`、`speed` 和 unknown raw 变体。
4. `Race` 和 `Attribute` 当前是封闭枚举，不保留 unknown raw 变体。
5. `CardListRow` 增加 `subtype_display`。
6. 当前没有 `BulkCardPatch` 命令模型。

### 8.3 CardTexts

```ts
interface CardTexts {
  name: string;
  desc: string;
  strings: string[];
}
```

规则：

1. `texts` 至少需要一个语言。
2. 至少一个语言的 `name` 非空。
3. `strings` 在规范化时按 CDB 提示文本语义处理。
4. 新建和正常编辑只能使用文本语言目录允许的语言。

### 8.4 怪兽字段

`monster_flags` 支持：

```text
normal, effect, fusion, ritual, synchro, xyz, pendulum, link,
tuner, token, gemini, spirit, union, flip, toon
```

`race` 支持：

```text
warrior, spellcaster, dragon, zombie, machine, aqua, pyro, rock,
winged_beast, plant, insect, thunder, fish, sea_serpent, reptile,
psychic, divine_beast, beast, beast_warrior, dinosaur, fairy, fiend,
illusion, cyberse, creator_god, wyrm
```

`attribute` 支持：

```text
light, dark, earth, water, fire, wind, divine
```

### 8.5 魔法与陷阱字段

`spell_subtype` 支持：

```text
normal, continuous, quick_play, ritual, field, equip
```

`trap_subtype` 支持：

```text
normal, continuous, counter
```

### 8.6 Link 与 Pendulum

```ts
interface Pendulum {
  left_scale: number;
  right_scale: number;
}

interface LinkData {
  markers: LinkMarker[];
}
```

`LinkMarker` 支持：

```text
top, bottom, left, right, top_left, top_right, bottom_left, bottom_right
```

### 8.7 Card 结构校验

硬性错误包括：

1. `code == 0`。
2. `category > u32::MAX`。
3. `setcodes.length > 4`。
4. `texts` 为空。
5. 所有文本语言的 `name` 都为空。
6. `atk` 或 `def` 小于 `0` 且不等于 `-2`。
7. 怪兽缺少 `monster_flags`。
8. 怪兽缺少 `atk`。
9. 怪兽缺少 `race`。
10. 怪兽缺少 `attribute`。
11. Link 怪兽缺少 `link`。
12. Link 怪兽设置了 `def`。
13. 非 Link 怪兽缺少 `level`。
14. 非 Pendulum 怪兽设置了 `pendulum`。
15. 魔法卡缺少 `spell_subtype`。
16. 陷阱卡缺少 `trap_subtype`。

### 8.8 QMARK

```ts
const QMARK = -2;
```

`atk` 和 `def` 可用 `-2` 表示 `?`。`level` 不使用 QMARK。

### 8.9 Card 规范化

保存前会对输入做规范化。规范化目标是移除不适用字段、去重并保持内部结构一致。

规范化包括：

1. 魔法/陷阱清空怪兽字段。
2. 怪兽清空魔法/陷阱 subtype。
3. Link 怪兽清空 `def`。
4. 非 Link 怪兽清空 `link`。
5. 非 Pendulum 怪兽清空 `pendulum`。
6. flags 和 markers 按固定顺序去重。

### 8.10 Card 列表行

```ts
interface CardListRow {
  id: string;
  code: number;
  name: string;
  desc: string;
  primary_type: PrimaryType;
  subtype_display: string;
  atk: number | null;
  def: number | null;
  level: number | null;
  has_image: boolean;
  has_script: boolean;
  has_field_image: boolean;
}
```

列表行由 `CardEntity + Pack 显示语言顺序 + 资源状态` 派生，不持久化。

## 9. 卡片功能规范

### 9.1 当前支持的卡片操作

1. 列出已打开自定义包卡片。
2. 按关键字搜索。
3. 按结构化高级筛选搜索。
4. 按 `code / name / type / atk / def / level` 排序。
5. 分页。
6. 获取卡片详情。
7. 新建卡片。
8. 编辑卡片。
9. 删除卡片。
10. 推荐下一个可用编号。
11. 带 warning 的写入确认。
12. 编辑卡片文本语言。
13. 管理卡图、场地图和脚本资源。

### 9.2 当前不支持的卡片操作

1. 批量删除。
2. 批量移动到已有包。
3. 批量移动到新建包。
4. 批量字段修改。
5. 跨包复制卡片。
6. Lua 脚本内置 IDE。

这些能力在旧功能文档中被描述为首版必须支持，但当前命令面和主要 UI 尚未实现。

### 9.3 高级搜索

自定义包和标准包共享结构化筛选合同：

1. code 精确值和范围。
2. alias 精确值和范围。
3. OT。
4. 卡名包含。
5. 效果文本包含。
6. 主类型。
7. 种族。
8. 属性。
9. 怪兽 flags，支持 `any/all`。
10. 魔法 subtype。
11. 陷阱 subtype。
12. 灵摆左右刻度范围。
13. Link markers，支持 `any/all`。
14. setcodes，支持 `exact/base` 和 `any/all`。
15. category masks，支持 `any/all`。
16. ATK / DEF / Level 范围。

自定义包在已打开 `PackSession` 内存快照上过滤。标准包在 SQLite 查询索引上过滤。

## 10. 编号和命名空间规范

### 10.1 卡片编号

卡片编号校验上下文包括：

1. 当前包内其他卡号。
2. 工作区内其他自定义包卡号。
3. 标准包卡号基线。

硬性错误：

1. `code == 0`。
2. `code > 268435455`。
3. 与当前包内其他卡号重复。
4. 与标准包卡号重复。

Warning：

1. 位于标准包保留范围 `0 - 99999999`。
2. 超出推荐自定义范围。
3. 与已有卡号距离小于 `custom_code_min_gap`。
4. 与工作区内其他自定义包卡号重复。

注意：当前实现将“与其他自定义包卡号重复”作为 warning，而不是硬性错误。导出时，所选导出包之间的 code 重复会升级为 error。

### 10.2 自动推荐编号

推荐编号从 `preferred_start` 和全局推荐下限二者较大值开始，在推荐范围内寻找：

1. 不与当前包重复。
2. 不与其他自定义包重复。
3. 不与标准包重复。
4. 与所有已用编号距离满足 `custom_code_min_gap`。

### 10.3 Pack Strings 命名空间

自定义包字符串写入时会检查：

1. 与工作区内其他自定义包字符串 key/base 的冲突。
2. 与标准包字符串 key/base 的冲突。
3. 推荐 key 范围。

编辑阶段多数命名空间问题为 warning，需要确认后继续。导出阶段部分冲突会成为 error。

## 11. 文本语言和多语言规范

### 11.1 三类文本

YGOCMG 明确区分：

1. App UI 文案。
2. Card Texts。
3. Pack Strings。

三者不混用。

### 11.2 Card Texts 规则

1. 卡片可保存多个文本语言。
2. 卡片列表使用 `pack.display_language_order` 做显示回退。
3. 卡片编辑器展示 pack 期望语言和卡片实际语言。
4. 缺失的期望语言可在编辑器中创建空草稿。
5. 额外语言只能从全局语言目录中选择。
6. 删除非空语言文本需要确认。
7. 导出不使用显示回退，必须存在目标语言文本。

### 11.3 Pack 显示语言

`display_language_order` 决定：

1. 自定义包卡片列表的 `name` 和 `desc` 回退。
2. 卡片编辑器中期望语言标签。
3. Pack strings UI 的语言选择默认语义。

### 11.4 默认导出语言

`default_export_language` 用于导出 UI 预选：

1. 单包导出可使用该包默认导出语言。
2. 多包导出只有在所有选中包共享同一个有效默认导出语言时才可预选。
3. 最终导出仍必须显式指定 `export_language`。

## 12. Pack Strings 规范

### 12.1 当前模型

```ts
type PackStringKind = "system" | "victory" | "counter" | "setname";

interface PackStringRecord {
  kind: PackStringKind;
  key: number;
  values: Record<string, string>;
}

interface PackStringsFile {
  schema_version: 2;
  entries: PackStringRecord[];
}
```

### 12.2 兼容迁移

读取 `strings.json` 时支持旧版 schema：

```ts
interface LegacyPackStringsFile {
  schema_version: 1;
  entries: Record<string, PackStringEntry[]>;
}
```

旧版按语言分组的数据会在读取时转换为当前的 `(kind, key) -> values` 聚合模型。

### 12.3 唯一性

同一包内 `(kind, key)` 必须唯一。

### 12.4 自定义包作者限制

当前自定义包作者态禁止新增或修改 `system` 字符串。`system` 主要作为标准包和 legacy 数据兼容项存在。

### 12.5 当前支持的字符串操作

1. 按语言列出字符串。
2. 按 kind 过滤。
3. 按 key 过滤。
4. 按 keyword 搜索 value。
5. 新增或更新单语言字符串值。
6. 新增或替换多语言字符串记录。
7. 删除整个 `(kind, key)` 记录。
8. 删除某个语言翻译。
9. 覆盖已有值时进入确认流。

## 13. 资源规范

### 13.1 资源类型

每张卡按 `code` 绑定以下作者态资源：

1. 主卡图：`pics/<code>.jpg`
2. 场地图片：`pics/field/<code>.jpg`
3. 脚本：`scripts/c<code>.lua`

资源状态不写入 `CardEntity`，由文件存在性派生。

### 13.2 主卡图

规则：

1. 导入任意可解码图片。
2. 保存为 JPEG。
3. 强制缩放为 `400 x 580`。
4. JPEG 质量为 `90`。
5. 支持导入/替换和删除。

### 13.3 场地图片

规则：

1. 只有场地魔法卡允许导入场地图片。
2. 场地魔法条件为 `primary_type = spell` 且 `spell_subtype = field`。
3. 保存为 JPEG。
4. 不强制缩放，保留源图尺寸。
5. 支持导入/替换和删除。

### 13.4 脚本

规则：

1. 脚本文件名为 `c<code>.lua`。
2. 支持创建空白脚本。
3. 支持导入脚本文件。
4. 支持删除脚本。
5. 支持通过外部文本编辑器打开脚本。

### 13.5 编号变更时的资源迁移

当卡片 `code` 变更时，程序会计划重命名：

1. `pics/<old>.jpg` -> `pics/<new>.jpg`
2. `pics/field/<old>.jpg` -> `pics/field/<new>.jpg`
3. `scripts/c<old>.lua` -> `scripts/c<new>.lua`

资源迁移与 `cards.json` 写入一起作为程序内文件操作计划执行。

## 14. 标准包只读规范

### 14.1 定位

标准包是只读参考索引，不是 workspace 中的 pack。它用于：

1. 浏览官方/标准卡。
2. 搜索官方/标准卡。
3. 查看标准包字符串。
4. 提供 setname 选择参考。
5. 打开标准包脚本文件。
6. 提供编号和字符串命名空间基线。

### 14.2 标准包状态

当前状态枚举：

```ts
type StandardPackIndexState =
  | "not_configured"
  | "missing_language"
  | "missing_source"
  | "missing_index"
  | "stale"
  | "ready"
  | "error";
```

状态 DTO 包含：

1. 是否配置 YGOPro 路径。
2. YGOPro 路径。
3. CDB 路径。
4. 索引是否存在。
5. schema 是否不匹配。
6. 是否 stale。
7. 源语言。
8. 索引时间。
9. 卡片数量。
10. 状态消息。

### 14.3 索引来源

标准包索引来源于：

1. YGOPro 根目录下唯一 `.cdb`。
2. 根目录 `strings.conf`。
3. 根目录 `pics/`。
4. 根目录 `pics/field/`。
5. 根目录 `script/`。

当前规范不扫描或合并 `expansions/`。

### 14.4 索引存储

当前标准包索引写入应用数据目录：

```text
<app_data>/standard_pack/index.sqlite
<app_data>/standard_pack/manifest.json
```

说明：

1. `index.sqlite` 是运行时查询的权威缓存。
2. `manifest.json` 是轻量 sidecar。
3. 不再写入旧的完整 `index.json`。
4. 索引是可丢弃缓存，源数据仍是用户配置的 YGOPro 目录。
5. schema mismatch 或缺失时，用户通过重建索引恢复。

### 14.5 标准包功能

当前支持：

1. 查询状态。
2. 后台重建索引。
3. 搜索标准卡列表。
4. 标准卡高级筛选。
5. 查看标准卡详情。
6. 搜索标准包字符串。
7. 列出标准包 setnames。
8. 打开标准包脚本。

标准包不支持：

1. 编辑卡片。
2. 编辑字符串。
3. 导入资源。
4. 删除资源。
5. 移入工作区。

## 15. 导入规范

### 15.1 目标

导入功能将一组 YGOPro 运行时资源转换为新的作者态自定义包。

### 15.2 输入

导入预检输入：

```ts
interface PreviewImportPackInput {
  workspace_id: string;
  new_pack_name: string;
  new_pack_code: string | null;
  new_pack_author: string;
  new_pack_version: string;
  new_pack_description: string | null;
  display_language_order: string[];
  default_export_language: string | null;
  cdb_path: string;
  pics_dir: string | null;
  field_pics_dir: string | null;
  script_dir: string | null;
  strings_conf_path: string | null;
  source_language: string;
}
```

### 15.3 语言规则

1. `source_language` 必须来自全局文本语言目录。
2. `display_language_order` 必须来自全局文本语言目录。
3. `default_export_language` 必须来自全局文本语言目录。
4. 如果 `display_language_order` 不包含源语言，会产生 warning，并在内部规范化时把源语言插入。
5. CDB 解析出的 `default` 语言会映射到 `source_language`。
6. `strings.conf` 解析出的 `default` 语言会映射到 `source_language`。

### 15.4 预检

导入采用两阶段流程：

1. `preview_import_pack`
2. `execute_import_pack`

预检返回：

1. 目标 pack id。
2. 目标 pack 名称。
3. 卡片数量。
4. warning 数量。
5. error 数量。
6. 缺失主卡图数量。
7. 缺失脚本数量。
8. 缺失场地图片数量。
9. issues 列表。
10. `preview_token`。
11. `snapshot_hash`。
12. 过期时间。

`preview_token` 默认 10 分钟过期。

### 15.5 导入校验

导入预检检查：

1. workspace 是否匹配当前会话。
2. 目标 pack 元数据是否合法。
3. 目标 pack 目录是否已存在。
4. CDB 是否可读。
5. CDB 内 code 是否重复。
6. 卡片结构是否合法。
7. 卡号是否与当前 workspace 和标准包冲突。
8. 编号是否在推荐范围和间距规则内。
9. strings 是否可读。
10. 主卡图是否缺失。
11. 脚本是否缺失。
12. 场地魔法的场地图片是否缺失。

缺失资源是 warning，不阻断导入。

### 15.6 执行

执行导入时：

1. 消耗 `preview_token`。
2. 检查 token 是否过期。
3. 重新 prepare import。
4. 若预检包含 error，阻断执行。
5. 对比当前 source snapshot 和预检 snapshot。
6. 若源文件或目标状态变化，报 `import.preview_stale`。
7. 以后台 Job 写入 pack 目录、JSON 和资源。
8. 更新 workspace 的 `pack_order`。
9. 刷新 workspace 摘要。

## 16. 导出规范

### 16.1 目标

导出功能将一个或多个已打开自定义包融合为 YGOPro 运行时资源目录。

输出结构：

```text
<output_dir>/<output_name>/
  <output_name>.cdb
  strings.conf
  pics/
    <code>.jpg
    field/
      <code>.jpg
  script/
    c<code>.lua
```

### 16.2 输入

```ts
interface PreviewExportBundleInput {
  workspace_id: string;
  pack_ids: string[];
  export_language: string;
  output_dir: string;
  output_name: string;
}
```

### 16.3 输出名称规则

`output_name` 必须是安全的单个文件名：

1. 不为空。
2. 不是 `.` 或 `..`。
3. 不是绝对路径。
4. 不以空格或 `.` 结尾。
5. 不含 `/ \ < > : " | ? *` 或控制字符。
6. 不是 Windows 保留设备名，如 `CON`、`PRN`、`AUX`、`NUL`、`COM1`、`LPT1`。

### 16.4 预检

导出采用两阶段流程：

1. `preview_export_bundle`
2. `execute_export_bundle`

预检返回：

1. pack 数量。
2. 卡片数量。
3. 主卡图数量。
4. 场地图数量。
5. 脚本数量。
6. warning 数量。
7. error 数量。
8. issues 列表。
9. `preview_token`。
10. `snapshot_hash`。
11. 过期时间。

`preview_token` 默认 10 分钟过期。

### 16.5 导出校验

硬性错误包括：

1. 未选择 pack。
2. 选择了重复 pack id。
3. output name 非法。
4. 输出目标目录存在且非空。
5. 所选 pack 不是 custom。
6. 卡片结构不合法。
7. 卡片缺少目标语言文本。
8. Pack string 缺少目标语言值。
9. 所选 pack 之间 code 重复。
10. code 与标准包冲突。
11. `system` key 与标准包冲突。
12. `setname` key 与标准包冲突。
13. `counter` key 与标准包冲突。
14. `victory` key 与标准包冲突。
15. 所选 pack 之间 `setname` full key 冲突。
16. 所选 pack 之间 `counter` key 冲突。
17. 所选 pack 之间 `victory` key 冲突。

Warning 包括：

1. code 位于标准包保留范围但未与标准包实际 code 冲突。
2. `setname` base 与标准包重叠。
3. 所选 pack 之间 `setname` base 重叠。

### 16.6 执行

执行导出时：

1. 消耗 `preview_token`。
2. 检查 token 是否过期。
3. 重新 prepare export。
4. 若预检包含 error，阻断执行。
5. 检查 pack ids 是否仍匹配。
6. 对比当前 pack snapshot 与预检 snapshot。
7. 若 pack revision 或 source stamp 变化，报 `export.preview_stale`。
8. 以后台 Job 写入 CDB、`strings.conf` 和资源文件。

## 17. 统一确认与写入规范

### 17.1 WriteResult

带 warning 的写入使用：

```ts
type WriteResult<T> =
  | { status: "ok"; data: T; warnings: ValidationIssue[] }
  | {
      status: "needs_confirmation";
      confirmation_token: string;
      warnings: ValidationIssue[];
      preview: unknown | null;
    };
```

当前使用确认流的场景：

1. 创建卡片时存在 warning。
2. 更新卡片时存在 warning。
3. 覆盖 pack string 翻译。
4. 覆盖 pack string record。
5. pack string 命名空间 warning。

### 17.2 Confirmation Token

规则：

1. token 只能使用一次。
2. pack revision/source stamp 变化后 token 失效。
3. workspace 切换后 token 清空。

### 17.3 文件写入

当前 JSON 写入使用 safe write。多文件操作通过文件操作计划执行，常见操作包括：

1. 写入文件。
2. 创建目录。
3. 删除文件。
4. 重命名文件。

目标行为：

1. 单文件写入尽量表现为完整成功或保持旧内容。
2. 多文件操作失败时尽量保持可恢复。
3. 不承诺跨多个文件的崩溃原子事务。

## 18. Job 规范

后台 Job 当前用于：

1. 标准包索引重建。
2. 导入执行。
3. 导出执行。

命令支持：

1. 获取单个 job 状态。
2. 列出活动 job。

Job 返回 `JobAccepted` 后，前端通过 job 状态轮询或事件反馈进度。

## 19. UI Shell 当前功能规范

### 19.1 主壳层

当前主界面由以下部分组成：

1. 自绘标题栏。
2. 左侧可调整、可折叠 sidebar。
3. 右侧工作区。
4. 中央 modal 层。
5. 右侧 card editor drawer。
6. 全局 notice banner。
7. 统一确认 dialog。

### 19.2 Sidebar

Sidebar 当前包含：

1. Workspace 按钮。
2. Export 按钮。
3. Settings 按钮。
4. 已打开 custom pack 列表。
5. `+` 打开/创建/导入 pack 按钮。
6. 固定 Standard Pack 按钮。
7. 折叠/展开按钮。
8. 宽度拖拽调节。

### 19.3 Custom Pack 工作区

自定义包工作区包含：

1. PackMetadataPanel。
2. `Cards` tab。
3. `Strings` tab。
4. CardEditDrawer。

### 19.4 Standard Pack 视图

标准包视图包含：

1. 状态和重建入口。
2. 只读 Cards 浏览。
3. 高级搜索。
4. 只读 Strings 浏览。
5. 标准卡详情 inspector。
6. 打开标准包脚本操作。

### 19.5 Modals

当前主要 modal：

1. WorkspaceModal。
2. SettingsModal。
3. AddPackModal。
4. ExportModal。

`AddPackModal` 包含：

1. Open Pack。
2. Create Pack。
3. Import Pack。

### 19.6 返回导航和未保存确认

当前支持：

1. Escape/back request 触发当前层级关闭。
2. Modal、drawer、导入向导、编辑表单等可以注册关闭拦截。
3. 有未保存更改时通过应用内 dialog 请求确认。
4. 不使用浏览器原生 `alert/confirm/prompt` 作为主要确认机制。

## 20. 当前不纳入或尚未实现的功能

以下功能不属于当前实现态规范：

1. AI 生成卡图。
2. AI 生成 Lua 脚本。
3. 内置 Lua IDE。
4. `package/` 共享脚本体系。
5. 自定义包批量移动卡片。
6. 自定义包批量字段编辑。
7. 自定义包多包合并成新作者态包。
8. 资源多版本管理。
9. 主卡图内置裁剪编辑器。
10. 自动扫描 YGOPro `expansions/` 并合并标准包。
11. 自定义包主存储 SQLite 化。
12. 标准包编辑。
13. 标准包自动监听并自动重建索引。
14. 完整主题皮肤编辑器。
15. 前端组件级自动化测试体系。

## 21. 与 2026-04-25 旧功能文档的主要偏移

### 21.1 已扩展

1. 增加 UI 三语 i18n。
2. 增加文本语言目录。
3. 增加标准包源语言。
4. 增加标准包 SQLite 查询索引。
5. 增加标准包高级搜索。
6. 增加自定义包高级搜索。
7. 增加 pack code。
8. 增加主题模式、高对比度和品牌色。
9. 增加 sidebar 折叠。
10. 增加标准包脚本外部打开。

### 21.2 已改变

1. `setcode` 从 raw 数字改为 `setcodes: Vec<u16>`。
2. `PackStringsFile` 从按语言分组改为 `(kind, key)` 聚合多语言 values。
3. 标准包索引从完整 JSON 快照改为 SQLite 查询缓存。
4. 标准包重建需要显式配置 `standard_pack_source_language`。
5. 与其他自定义包 code 重复在卡片保存阶段是 warning，导出阶段才对所选包之间重复升级为 error。
6. 怪兽缺少 race/attribute/atk 等在当前保存校验中是 error，不是 warning。

### 21.3 尚未落地

1. `BulkCardPatch`。
2. 批量删除。
3. 批量移动到已有 pack。
4. 批量移动到新建 pack。
5. 同时打开 pack 数量上限。
6. Workspace UI 中完整删除/注销流程是否暴露需要以当前 UI 为准，后端命令已支持删除记录和删除目录。

## 22. 当前最小完整用户流程

当前 YGOCMG 支持以下完整流程：

1. 首次启动应用。
2. 设置 UI 语言、主题、YGOPro 路径、标准包源语言和文本语言目录。
3. 重建标准包索引。
4. 创建或打开 workspace。
5. 创建自定义 pack，填写 pack code、作者、版本、显示语言和默认导出语言。
6. 打开一个或多个 custom pack。
7. 在 custom pack 中新建卡片。
8. 编辑卡片基础数据、setcodes、category、文本语言、主卡图、脚本和场地图。
9. 使用标准包浏览、高级搜索和 setname 参考辅助编辑。
10. 编辑 pack strings。
11. 从运行时 CDB、图片、脚本和 strings.conf 导入为新 pack。
12. 选择一个或多个已打开 custom pack，预检并导出为运行时资源目录。
13. 关闭 pack 或重启应用后恢复上次 workspace 和打开 pack 状态。

## 23. 最终结论

当前 YGOCMG 已经不只是旧文档中的“首版功能草案”。从代码实现看，它的产品形态已经稳定为：

```text
作者态自定义卡包编辑器
+ 标准包只读参考索引
+ 受管文本语言系统
+ 导入导出流水线
+ 资源管理
+ 高级搜索
+ 桌面壳层与主题配置
```

后续维护功能文档时，应以本文件描述的实现态为基线。旧的 2026-04-25 功能规范可以保留为历史设计资料，但不应继续作为当前功能事实来源。
