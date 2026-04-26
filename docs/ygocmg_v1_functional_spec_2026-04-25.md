# YGOCMG 首版功能规范 v1

日期：2026-04-25  
状态：Draft

关联文档：
- [项目粗略设计方案](./ygocmg.md)
- [卡片数据模型语义化重构方案 v1](./card_data_model_refactor_plan_2026-04-17.md)
- [卡片数据模型语义化重构方案 v2](./card_data_model_refactor_v2_2026-04-23.md)

## 1. 文档目标

本文档用于定义 YGOCMG 首版的：

1. 功能范围
2. `workspace / pack / card` 及相关文本、资源模型
3. 首版边界
4. 导入、导出、校验的产品规则

本文档不讨论：

1. 项目整体架构
2. 前后端模块拆分
3. 存储技术选型
4. 缓存、事务、任务调度等实现细节

换句话说，本文档回答的是“首版做什么、数据是什么、用户能怎么用”，而不是“代码将如何组织实现”。

## 2. 产品目标

YGOCMG 首版的目标是做成一个作者态的自定义卡包管理与编辑工具，重点完成以下四件事：

1. 管理多个工作区与多个作者态卡包
2. 用语义化卡片模型编辑卡片
3. 管理多语言文本、卡图、脚本和场地图片资源
4. 把一个或多个作者态 `pack` 导出为 YGOPro 可用的运行时资源

## 3. 首版边界

### 3.1 首版必须支持

1. 全局配置
2. 工作区创建、打开、切换
3. 标准包只读接入
4. 自定义 `pack` 的创建、编辑、删除、导入
5. 单卡创建、查看、编辑、删除
6. 卡片列表搜索、排序、基础批量操作
7. 多语言卡片文本编辑
8. 多语言 `pack strings` 编辑
9. 单卡主卡图管理
10. 单卡 Lua 脚本管理
11. 场地魔法的场地图片管理
12. 多 `pack` 融合导出
13. 编号唯一性与冲突检测
14. 程序内保存的安全写入行为

### 3.2 当前规范不展开

1. 项目整体架构设计
2. `setcode / category / alias` 的深度语义化
3. 更复杂的批量编辑能力，例如批量编辑 `pendulum` 和 `link.markers`

### 3.3 明确不纳入当前产品范围

1. AI 生成脚本
2. AI 制图或图片编辑
3. 内置 Lua IDE
4. `package/` 共享脚本体系

## 4. 核心术语

### 4.1 Standard Pack

通过全局配置指定的 YGOPro 标准包数据来源。标准包：

1. 不属于任何 `workspace`
2. 全局唯一
3. 只读
4. 用于浏览、搜索、参考和编号冲突检测

### 4.2 Workspace

作者态工作区。一个 `workspace` 是一组自定义 `pack` 的容器，也是首版的主要管理边界。

### 4.3 Pack

作者态卡包。`pack` 是编辑、组织、导入、导出选择的基本单位，不等同于 YGOPro 运行时中的单个 `.cdb` 文件。

### 4.4 Runtime Export Bundle

导出产物。由一个或多个作者态 `pack` 融合而成，输出为 YGOPro 可用的运行时资源目录。目录中至少包含：

1. 一个 `.cdb`
2. 一个 `pics/`
3. 一个 `script/`
4. 一个 `strings.conf`

若存在场地图片，还包含：

1. `pics/field/`

### 4.5 Card Text

卡片自身多语言文本，逻辑上对应 CDB `texts` 表中的：

1. 卡名
2. 效果描述
3. 16 条提示文本

### 4.6 Pack Strings

`pack` 级别的多语言字符串集合，逻辑上对应 `strings.conf` 中的条目，例如：

1. `system`
2. `victory`
3. `counter`
4. `setname`

### 4.7 Error / Warning

`Error`：阻断当前操作。  
`Warning`：允许用户确认后继续。

## 5. 全局配置

全局配置不属于任何 `workspace`。首版必须支持以下字段：

```ts
type LanguageCode = string;

interface GlobalConfig {
  app_language: LanguageCode;
  ygopro_path: string | null;
  external_text_editor_path: string | null;

  custom_code_recommended_min: number; // 默认 90000000
  custom_code_recommended_max: number; // 默认 99999999
  custom_code_min_gap: number;         // 默认 10
  shell_sidebar_width: number;         // 默认 150
  shell_window_width: number;          // 默认 960
  shell_window_height: number;         // 默认 640
  shell_window_is_maximized: boolean;  // 默认 false
}
```

除此之外，程序必须维护独立于任何 `workspace` 的程序级工作区注册表，存放在应用数据目录中，用于 recent workspaces：

```ts
interface WorkspaceRegistryFile {
  schema_version: 1;
  workspaces: WorkspaceRegistryEntry[];
}

interface WorkspaceRegistryEntry {
  workspace_id: string;
  path: string;
  name_cache: string | null;
  last_opened_at: string | null;
}
```

说明：

1. `workspace_registry.json` 不属于任何 `workspace`
2. 它只记录程序已知工作区及最近打开信息，不保存 `pack` 数据
3. 首版 UI 只消费 recent workspaces 信息，不提供删除或注销工作区入口

固定产品规则：

1. 标准包保留编号范围：`0 - 99999999`
2. YGOPro 最大支持编号：`268435455`

全局配置功能要求：

1. 设置程序 UI 语言
2. 设置 YGOPro 路径
3. 设置外部文本编辑器路径
4. 持久化侧边栏宽度
5. 持久化窗口普通尺寸
6. 持久化窗口最大化状态
7. 设置自定义卡推荐编号范围
8. 设置自定义卡最小编号间距阈值

## 6. Workspace 规范

### 6.1 Workspace 职责

`workspace` 负责：

1. 承载多个自定义 `pack`
2. 记录包顺序与当前打开状态
3. 提供工作区级编号冲突校验上下文
4. 提供多 `pack` 导出的选择范围

标准包不存放在 `workspace` 内。

### 6.2 Workspace 数据模型

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
```

说明：

1. `pack_order` 是工作区内自定义包的显式顺序
2. `open_pack_ids` 记录当前已打开的 pack tab 列表，用于会话恢复
3. `last_opened_pack_id` 目前承担“最后查看的活跃 pack”语义，用于恢复上次聚焦的 pack 视图
4. 工作区不保存卡片显示语言偏好
5. 工作区不保存默认导出语言

这样可以避免 `workspace` 和 `pack` 同时持有语言偏好而产生冲突。

### 6.3 Workspace 磁盘组织

首版要求 `workspace` 在磁盘上表现为一个目录。逻辑上至少包含：

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

说明：

1. `workspace.json` 记录工作区元数据
2. 每个 `pack` 拥有独立目录
3. 标准包资源不复制到 `workspace`
4. `pics/field/` 仅在该 `pack` 有场地图片时需要存在
5. `workspace.json`、`metadata.json`、`cards.json`、`strings.json` 都必须包含 `schema_version`
6. 首版所有作者态 JSON 文件的 `schema_version` 固定为 `1`
7. 程序级的 `global_config.json` 和 `workspace_registry.json` 存放在应用数据目录，不放进任何 `workspace`
8. 作者态 `pack` 内部目录固定使用 `scripts/`；运行时导入导出目录固定使用 `script/`；两者映射由导入导出层负责

### 6.4 Workspace 功能要求

首版必须支持：

1. 新建工作区
2. 打开已有工作区
3. 切换当前工作区
4. 展示当前工作区下的所有自定义 `pack`
5. 展示程序已记录的最近工作区
6. 显式打开一个或多个 `pack`
7. 显式关闭已打开的 `pack`
8. 在多个已打开 `pack` 之间切换当前激活 tab

运行时约束：

1. `workspace` 内可以存在多个自定义 `pack`，不以总数量作为首版产品限制
2. 首版限制“同时打开的自定义 `pack` 数量”而不是限制 `workspace` 中 `pack` 的总数
3. 同时打开上限建议固定为 `8`
4. 达到上限后，再打开新的 `pack` 必须被阻断并给出明确错误提示
5. 打开或关闭 `pack` 只影响运行时会话，不改变 `workspace` 中 `pack` 的存在性

补充约束：

1. 程序级 recent workspaces 注册表允许保留失效路径记录
2. 当注册表中的工作区路径已经不存在时，UI 应把它展示为不可打开或打开失败的 recent 项
3. 首版不提供“从程序中移除记录”或“删除磁盘目录”的用户入口

## 7. Pack 规范

### 7.1 Pack 职责

作者态 `pack` 负责：

1. 存放卡片数据
2. 存放 `pack strings`
3. 存放图片与脚本资源
4. 提供列表、搜索、排序、批量操作边界
5. 作为导入目标与导出选择单位

### 7.2 Pack 分类

界面层面可以存在两类 `pack`：

```ts
type PackKind = "standard" | "custom";
```

其中：

1. `standard`：全局只读参考包，不属于工作区
2. `custom`：工作区内作者态包，可编辑

首版持久化到 `workspace` 内的只有 `custom` 包。

### 7.3 Pack 元数据模型

```ts
interface PackMetadataFile {
  schema_version: 1;
  data: PackMetadata;
}

interface PackMetadata {
  id: string;
  kind: "custom";

  name: string;
  author: string;
  version: string;
  description: string | null;

  created_at: string;
  updated_at: string;

  display_language_order: LanguageCode[];
  default_export_language: LanguageCode | null;
}
```

说明：

1. `display_language_order` 用于该包卡片显示时的语言回退顺序
2. `default_export_language` 用于该包导出界面的默认语言建议值
3. `updated_at` 在包内卡片、字符串、资源或元数据发生修改时更新

语言规则：

1. 卡片显示语言只由当前 `pack.display_language_order` 决定
2. 单包导出时，导出界面默认语言由 `pack.default_export_language` 决定
3. 多包导出时，必须由用户显式选择导出语言
4. 多包导出时，不使用任何单个 `pack` 的默认导出语言去覆盖其他 `pack`

### 7.4 Pack 数据内容

每个自定义 `pack` 至少包含：

1. `metadata`
2. `cards`
3. `pack strings`
4. `pics/`
5. `scripts/`

### 7.5 Pack 功能要求

首版必须支持：

1. 新建空白 `pack`
2. 编辑 `pack` 元数据
3. 删除 `pack`
4. 从运行时资源导入为 `pack`
5. 在 `pack` 页面以 `CardList` 和 `Strings` 两个 tab 管理内容
6. 从 `pack` 列表显式打开 `pack`
7. 在侧边栏 tab 中同时保留多个已打开 `pack`
8. 关闭任意一个已打开 `pack`

首版不要求：

1. 一键把多个 `pack` 合并成一个新的作者态 `pack`
2. `pack` 内部的子分组、标签树或文件夹系统

### 7.6 Pack 列表与摘要

首版 `pack` 列表至少显示：

1. 包名
2. 作者
3. 版本
4. 卡片数量
5. 更新时间
6. 当前是否已打开

## 8. Card 模型规范

### 8.1 设计原则

`Card` 模型必须遵守以下原则：

1. 内部模型服务于编辑，而不是服务于 CDB 编码
2. 一个字段只表达一个语义
3. `primary_type` 作为顶层真相源显式保留
4. 使用扁平结构
5. 不适用字段使用 `null`

### 8.2 Card 枚举模型

```ts
type PrimaryType = "monster" | "spell" | "trap";

type Attribute =
  | "earth"
  | "water"
  | "fire"
  | "wind"
  | "light"
  | "dark"
  | "divine"
  | { kind: "unknown"; raw: number };

type Race =
  | "warrior"
  | "spellcaster"
  | "fairy"
  | "fiend"
  | "zombie"
  | "machine"
  | "aqua"
  | "pyro"
  | "rock"
  | "winged_beast"
  | "plant"
  | "insect"
  | "thunder"
  | "dragon"
  | "beast"
  | "beast_warrior"
  | "dinosaur"
  | "fish"
  | "sea_serpent"
  | "reptile"
  | "psychic"
  | "divine_beast"
  | "creator_god"
  | "wyrm"
  | "cyberse"
  | "illusion"
  | { kind: "unknown"; raw: number };

type Ot =
  | "none"
  | "ocg"
  | "tcg"
  | "ocg_tcg"
  | "custom"
  | "speed"
  | { kind: "unknown"; raw: number };

type MonsterFlag =
  | "normal"
  | "effect"
  | "fusion"
  | "ritual"
  | "synchro"
  | "xyz"
  | "pendulum"
  | "link"
  | "tuner"
  | "flip"
  | "toon"
  | "spirit"
  | "union"
  | "gemini"
  | "spsummon"
  | "token";

type SpellSubtype =
  | "normal"
  | "quickplay"
  | "continuous"
  | "equip"
  | "field"
  | "ritual";

type TrapSubtype = "normal" | "continuous" | "counter";

type LinkMarker =
  | "top_left"
  | "top"
  | "top_right"
  | "left"
  | "right"
  | "bottom_left"
  | "bottom"
  | "bottom_right";
```

### 8.3 Card 文本子结构

```ts
interface CardTexts {
  name: string;
  desc: string;
  strings: string[]; // 规范化后固定 16 项
}
```

### 8.4 Card 资源相关子结构

```ts
interface Pendulum {
  left_scale: number;
  right_scale: number;
}

interface LinkData {
  markers: LinkMarker[];
}
```

### 8.5 CardEntity 主结构

```ts
type CardId = string;

interface CardEntity {
  id: CardId;
  code: number;
  alias: number;
  setcode: number;
  ot: Ot;
  category: number;
  primary_type: PrimaryType;
  texts: Record<LanguageCode, CardTexts>;

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

说明：

1. `CardEntity` 是作者态卡片的唯一持久化真相源
2. `id` 是内部稳定标识，由后端在新建、导入或复制时生成，推荐使用 `UUIDv7` 或 `ULID` 字符串
3. `id` 必须持久化写入 `cards.json`
4. `id` 在同一 `workspace` 内必须唯一
5. `id` 创建后不可修改；修改 `code` 时 `id` 不变
6. 卡片移动到别的 `pack` 时保留 `id`
7. 从运行时资源导入为作者态卡片时，必须为每张卡分配新的 `id`
8. `code` 是业务编号，可编辑，但不充当内部身份标识
9. 首版不把图片、脚本、场地图片作为 `CardEntity` 字段持久化
10. 资源绑定由 `pack` 中的文件资源和卡片 `code` 共同决定
11. 场地图片只对场地魔法有意义

### 8.6 Card 语义约束

#### 8.6.1 通用说明

1. `level` 只表示纯数值
2. 是否显示为 Level 还是 Rank，由 `monster_flags` 是否包含 `"xyz"` 决定
3. Link Rating 不直接存储，由 `link.markers.length` 推导
4. `alias`、`setcode`、`category` 在首版中保持 raw 数值

#### 8.6.2 Monster 适用字段

当 `primary_type === "monster"` 时：

1. `monster_flags` 可用
2. `atk` 可用
3. `race` 可用
4. `attribute` 可用
5. `spell_subtype` 不适用
6. `trap_subtype` 不适用

若 `monster_flags` 包含 `"link"`：

1. `link` 适用
2. `def` 不适用
3. `level` 不适用

若 `monster_flags` 不包含 `"link"`：

1. `link` 不适用
2. `def` 适用
3. `level` 适用

若 `monster_flags` 包含 `"pendulum"`：

1. `pendulum` 适用

若 `monster_flags` 不包含 `"pendulum"`：

1. `pendulum` 不适用

#### 8.6.3 Spell 适用字段

当 `primary_type === "spell"` 时：

1. `spell_subtype` 适用
2. `monster_flags`、`atk`、`def`、`race`、`attribute`、`level`、`pendulum`、`link`、`trap_subtype` 不适用
3. 若 `spell_subtype === "field"`，则该卡允许绑定一张场地图片资源

#### 8.6.4 Trap 适用字段

当 `primary_type === "trap"` 时：

1. `trap_subtype` 适用
2. `monster_flags`、`atk`、`def`、`race`、`attribute`、`level`、`pendulum`、`link`、`spell_subtype` 不适用

### 8.7 QMARK 常量

```ts
const QMARK = -2;
```

说明：

1. `atk === QMARK` 表示 `?`
2. `def === QMARK` 表示 `?`
3. `QMARK` 只用于数值语义字段，不用于 `level`

### 8.8 CardListRow

`CardListRow` 是运行时派生出来的列表行模型，不单独持久化。它只保留卡片列表真正需要的字段：

```ts
interface CardListRow {
  id: CardId;
  code: number;
  name: string;
  desc: string;
  primary_type: PrimaryType;
  atk: number | null;
  def: number | null;
  level: number | null;
  has_image: boolean;
  has_script: boolean;
  has_field_image: boolean;
}
```

说明：

1. `name` 取自当前 `pack.display_language_order` 的回退结果
2. `desc` 也取自 `pack.display_language_order` 的回退结果，用于搜索，不要求完整展示在列表中
3. `CardListRow` 由 `CardEntity + 当前语言 + 当前资源状态` 派生
4. `CardListRow` 不包含 `monster_flags`
5. `CardListRow` 不包含 `spell_subtype`
6. `CardListRow` 不包含 `trap_subtype`
7. `CardListRow` 不包含 `link_rating`
8. 列表选择、多选和局部刷新应以 `id` 为准，而不是以 `code` 为准

### 8.9 CardUpdateInput

单卡新建与单卡编辑使用完整输入模型：

```ts
interface CardUpdateInput {
  code: number;
  alias: number;
  setcode: number;
  ot: Ot;
  category: number;
  primary_type: PrimaryType;
  texts: Record<LanguageCode, CardTexts>;

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
}
```

说明：

1. `CardUpdateInput` 用于单卡创建和单卡保存
2. 保存时先执行结构校验，再执行规范化，再写回为 `CardEntity`
3. 当用户修改 `code` 时，该次保存操作必须连同资源改名一起视为同一程序内操作，并采用最佳努力方式保证整体一致性

### 8.10 BulkCardPatch

首版批量编辑使用独立的批量补丁模型，而不是复用单卡输入模型：

```ts
type PatchValue<T> =
  | { op: "set"; value: T }
  | { op: "clear" };

interface BulkCardPatch {
  primary_type?: PatchValue<PrimaryType>;
  monster_flags?: PatchValue<MonsterFlag[]>;
  spell_subtype?: PatchValue<SpellSubtype>;
  trap_subtype?: PatchValue<TrapSubtype>;
  atk?: PatchValue<number>;
  def?: PatchValue<number>;
  race?: PatchValue<Race>;
  attribute?: PatchValue<Attribute>;
  level?: PatchValue<number>;
  setcode?: PatchValue<number>;
  ot?: PatchValue<Ot>;
}
```

补丁语义：

1. 字段缺席：不修改
2. `{ op: "set", value }`：显式设值
3. `{ op: "clear" }`：显式清空
4. 首版批量补丁不覆盖 `code`、`alias`、`category`、`texts`

### 8.11 CardsFile

`cards.json` 的持久化结构如下：

```ts
interface CardsFile {
  schema_version: 1;
  cards: CardEntity[];
}
```

说明：

1. `cards.json` 只持久化 `CardEntity`
2. `CardListRow` 不写入磁盘
3. 运行时应允许建立 `id -> entity` 和 `code -> id` 的索引以支撑列表、搜索和校验

## 9. 文本模型规范

### 9.1 三类文本模型

首版必须明确区分三类文本：

1. `CardTexts`：卡片自身文本
2. `PackStrings`：`pack` 级字符串
3. `AppI18n`：程序 UI 文案

三者不能混用。

### 9.2 CardTexts 多语言要求

`Card.texts` 必须支持多语言，例如：

```ts
texts: {
  "zh-CN": { ... },
  "ja-JP": { ... },
  "en-US": { ... }
}
```

规则：

1. `LanguageCode` 首版使用字符串，不限制死枚举
2. 列表展示按当前 `pack.display_language_order` 选择 `name` 和 `desc`
3. 详情页可以切换 `current_edit_language`
4. `current_edit_language` 只是页面级临时状态，不持久化到 `workspace` 或 `pack`
5. 若当前编辑语言缺失，则按回退顺序选择第一个可用语言作为初始显示

### 9.3 PackStrings 模型

```ts
type PackStringKind = "system" | "victory" | "counter" | "setname";

interface PackStringEntry {
  kind: PackStringKind;
  key: number;
  value: string;
}

interface PackStringsFile {
  schema_version: 1;
  entries: Record<LanguageCode, PackStringEntry[]>;
}
```

约束：

1. 同一语言下，`(kind, key)` 组合必须唯一
2. 导入、编辑、保存、导出都必须检查该唯一性
3. 若存在相同 `(kind, key)` 且 `value` 不同，属于冲突而不是覆盖

### 9.4 Strings 编辑要求

`Strings` tab 必须支持：

1. 按语言查看字符串列表
2. 新增字符串条目
3. 修改字符串条目
4. 删除字符串条目
5. 按 `kind` 和 `key` 搜索

## 10. 资源模型规范

### 10.1 单卡资源边界

首版采用收敛后的资源模型：

1. 一张卡最多对应一张主卡图
2. 一张卡最多对应一个 Lua 脚本
3. 场地魔法卡可以额外绑定一张场地图片
4. 不支持 `package/` 共享脚本
5. 不支持一张卡多个主卡图版本
6. 不支持一张卡多个场地图片版本

### 10.2 主卡图资源要求

主卡图资源要求如下：

1. 目标格式为 `.jpg`
2. 导入源图的原始尺寸不要求固定
3. 导入时程序应自动缩放并落盘为 `400 x 580`
4. 支持导入、替换、删除、预览
5. 首版不提供用户可交互的内置裁剪、缩放、修图功能

### 10.3 脚本资源要求

脚本资源要求如下：

1. 以单卡 `.lua` 文件为唯一脚本资源
2. 支持新建空白脚本
3. 支持导入脚本文件
4. 支持删除脚本
5. 支持在外部编辑器中打开脚本

### 10.4 场地图片资源要求

场地图片资源要求如下：

1. 只有 `primary_type === "spell"` 且 `spell_subtype === "field"` 的卡片允许绑定场地图片
2. 每张场地魔法最多绑定一张场地图片
3. 场地图片与主卡图是两种不同资源
4. 首版必须支持导入、替换、删除、预览场地图片
5. 导出时场地图片输出到 `pics/field/<code>.jpg`
6. 导入时从 `pics/field/<code>.jpg` 读取

## 11. 标准包只读接入

首版必须允许用户在全局配置中指定 YGOPro 路径，并以只读方式接入标准包。

标准包功能要求：

1. 只读浏览卡片
2. 只读搜索标准卡名和基础信息
3. 只读查看标准字符串参考
4. 用于编号冲突检测

标准包不支持：

1. 编辑
2. 删除
3. 移入工作区

## 12. Pack 页面功能规范

### 12.1 页面结构

自定义 `pack` 工作区应采用“左侧 `pack` 列表 + 右侧已打开 `pack` 的侧边栏 tab”结构。

其中单个自定义 `pack` 页面应包含两个主要内容 tab：

1. `CardList`
2. `Strings`

交互要求：

1. `pack` 列表展示当前 `workspace` 下全部自定义 `pack`
2. 用户点击或显式操作后，`pack` 进入“已打开”状态，并在侧边栏 tab 中出现
3. 同一时刻允许多个 `pack` 处于已打开状态
4. 任一时刻只能有一个当前激活的 `pack`
5. 关闭 tab 只释放该 `pack` 的运行时会话，不删除 `pack`

### 12.2 CardList 显示要求

`CardList` 只显示摘要信息，不显示完整 `CardEntity`。显示字段与 `CardListRow` 保持一致：

1. 缩略卡图或占位图
2. `code`
3. `name`
4. `primary_type`
5. `atk`
6. `def`
7. `level`

说明：

1. Spell/Trap 的 `atk / def / level` 显示为空
2. CardList 不额外显示 `link_rating`
3. CardList 不额外显示 `spell_subtype / trap_subtype / monster_flags`
4. CardList 的选中、多选、局部刷新、批量操作目标统一以 `CardEntity.id` 标识

### 12.3 搜索与排序

`CardList` 必须支持：

1. 关键字搜索
2. 按编号排序
3. 按名称排序
4. 按类型排序
5. 按 ATK 排序
6. 按 DEF 排序
7. 按 Level 排序

搜索至少覆盖：

1. `code`
2. 按 `pack.display_language_order` 回退得到的列表展示 `name`
3. 按 `pack.display_language_order` 回退得到的列表展示 `desc`

### 12.4 批量操作

`CardList` 必须支持多选，并提供以下基础批量操作：

1. 批量删除
2. 批量移动到已有 `pack`
3. 批量移动到新建 `pack`
4. 批量修改基础字段

批量修改应基于 `BulkCardPatch` 执行，而不是基于完整 `CardUpdateInput` 执行。

首版批量修改字段范围：

1. `primary_type`
2. `monster_flags`
3. `spell_subtype`
4. `trap_subtype`
5. `atk`
6. `def`
7. `race`
8. `attribute`
9. `level`
10. `setcode`
11. `ot`

首版不要求批量编辑：

1. 多语言文本
2. `pendulum`
3. `link.markers`
4. 主卡图
5. 场地图片
6. 脚本

卡片在批量移动到已有或新建 `pack` 时，必须保留原 `CardEntity.id`，不得因为移动操作重新分配身份标识。

### 12.5 新建卡片

首版必须支持从 `pack` 页面新建卡片。新建卡片时：

1. 默认自动分配推荐范围内的可用编号
2. 用户可以手动修改编号
3. 保存前执行编号与模型校验

## 13. 单卡详情与编辑规范

### 13.1 详情区块

单卡详情页至少包含四个部分：

1. 主卡图
2. 卡片数据
3. 卡片文本
4. 脚本资源

若当前卡片是场地魔法，还必须包含：

1. 场地图片资源区块

### 13.2 卡片数据编辑要求

首版必须允许编辑：

1. `code`
2. `alias`
3. `setcode`
4. `ot`
5. `category`
6. `primary_type`
7. 与类型适用性对应的语义字段

字段显示逻辑必须按“是否适用”决定，而不是按当前值是否为 `null` 决定。

### 13.3 文本编辑要求

首版必须支持：

1. 切换当前编辑语言
2. 编辑 `name`
3. 编辑 `desc`
4. 编辑 16 条提示文本
5. 新增语言
6. 删除语言

### 13.4 资源编辑要求

首版必须支持：

1. 导入主卡图
2. 删除主卡图
3. 新建空白脚本
4. 导入脚本
5. 删除脚本
6. 用外部编辑器打开脚本
7. 若当前卡片是场地魔法，支持导入场地图片
8. 若当前卡片是场地魔法，支持删除场地图片
9. 若当前卡片是场地魔法，支持预览场地图片

## 14. 编号策略与冲突规则

### 14.1 编号区间

首版采用以下编号策略：

1. 标准包保留范围：`0 - 99999999`
2. 自定义卡推荐范围：`100000000 - 200000000`
3. 最大支持上限：`268435455`

### 14.2 编号唯一性

以下规则为硬规则：

1. 同一 `workspace` 内，自定义卡 `code` 不得重复
2. 自定义卡 `code` 不得与标准包已存在卡号完全相同
3. `code` 不得超过最大支持上限

实现约束：

1. `code` 的同 `workspace` 唯一性校验不能依赖“当前已打开 `pack`”
2. 即使某个 `pack` 未处于已打开状态，也必须参与编号冲突检测

### 14.3 编号间距

首版支持全局配置 `custom_code_min_gap`，默认值为 `5`。

规则：

1. 自动分配编号时，必须避开间距不满足阈值的编号
2. 用户手动输入或导入已有编号时，若与已有编号距离小于等于阈值，应给出 `warning`

说明：

1. “间距过近”是 `warning`
2. “完全重复”是 `error`

### 14.4 推荐范围使用规则

1. 自动分配编号只从推荐自定义范围内取值
2. 用户手动输入标准包保留范围内的编号时允许继续，但必须给出强警告
3. 若该编号与标准包已存在卡号完全相同，则阻断保存

## 15. 导入规范

### 15.1 导入目标

首版支持把 YGOPro 风格运行时资源导入为一个作者态 `pack`。

导入输入至少允许用户提供：

1. 一个源 `.cdb`
2. 可选的源 `pics/`
3. 可选的源 `pics/field/`
4. 可选的源 `script/`
5. 可选的源 `strings.conf`
6. 一个必选的源语言 `source_language`

### 15.2 导入内容

导入时处理：

1. CDB `datas/texts` -> `CardEntity.texts[source_language]`
2. `strings.conf` -> `PackStrings.entries[source_language]`
3. `pics/<code>.jpg` -> 主卡图资源
4. `pics/field/<code>.jpg` -> 场地图片资源
5. `script/c<code>.lua` -> 单卡脚本资源

说明：

1. 首版不根据源文件内容自动猜测语言
2. 导入界面必须要求用户显式选择 `source_language`
3. 同一次导入中的卡片文本和 `PackStrings` 使用同一个 `source_language`

### 15.3 导入校验

导入时至少检查：

1. 结构可读性
2. 卡号完全冲突
3. 编号间距警告
4. 模型结构一致性
5. 领域建议类 warning
6. 主卡图缺失
7. 脚本缺失
8. 场地魔法的场地图片缺失
9. `source_language` 是否已明确指定

导入必须采用“先预检、后执行”的两阶段流程：

1. 预检返回可展示的错误、警告、缺失资源信息和一次性 `preview_token`
2. 真正执行导入时只能提交该 `preview_token`
3. 若源文件、目标 `workspace / pack` 状态、相关 `pack revision`、磁盘状态摘要或 `preview_token` 有效期发生变化，必须重新预检

### 15.4 导入结果要求

导入结果必须可向用户明确展示：

1. 成功导入的卡片数量
2. 阻断导入的错误列表
3. 可确认继续的警告列表
4. 缺失的主卡图数量
5. 缺失的脚本数量
6. 缺失的场地图片数量

## 16. 导出规范

### 16.1 导出目标

首版导出目标是一个 YGOPro 风格运行时目录，包含：

1. 一个 `.cdb`
2. 一个 `pics/`
3. 一个 `script/`
4. 一个 `strings.conf`

若有场地图片，还包含：

1. `pics/field/`

### 16.2 导出输入

导出时用户必须至少选择：

1. 一个或多个 `pack`
2. 一个导出语言
3. 一个输出目录

### 16.3 导出语言规则

1. 导出时必须显式选择一个目标语言
2. 导出时只写出该语言的 `CardTexts`
3. 导出时只写出该语言的 `PackStrings`
4. 若某张卡缺失目标语言文本，阻断导出
5. 若 `PackStrings` 缺失目标语言条目，阻断导出

### 16.4 导出冲突检查

导出前必须检查：

1. 多个 `pack` 之间是否存在相同 `code`
2. 多个 `pack` 之间是否存在同一资源目标路径冲突
3. 多个 `pack` 之间是否存在相同 `PackStrings(kind, key)` 且值不同
4. 所有卡片是否满足结构一致性要求
5. 所选导出语言是否完整

导出必须采用“先预检、后执行”的两阶段流程：

1. 预检返回可展示的冲突、警告和一次性 `preview_token`
2. 真正执行导出时只能提交该 `preview_token`
3. 若所选 `pack`、导出语言、输出目录、相关 `pack revision` 或磁盘状态摘要发生变化，必须重新预检

### 16.5 导出结果要求

导出完成后，程序至少应反馈：

1. 导出目标目录
2. 导出的卡片数量
3. 导出的 `pack` 数量
4. 导出的主卡图数量
5. 导出的场地图片数量
6. 导出的脚本数量
7. 若失败，给出明确失败原因

## 17. 校验分层

### 17.1 结构一致性错误

以下问题属于 `error`，必须阻断保存、导入或导出：

1. `primary_type` 缺失或非法
2. 必填顶层字段结构损坏
3. 同一 `workspace` 内自定义卡号完全重复
4. 自定义卡号与标准包卡号完全重复
5. `code` 超过最大上限
6. 导出目标语言缺失
7. 多个导出 `pack` 之间存在不可解析的资源路径冲突

### 17.2 领域建议 Warning

以下问题属于 `warning`，允许确认后继续：

1. 自定义卡号位于标准包保留范围
2. 自定义卡号距离已有卡号过近
3. 怪兽缺少 `race`
4. 怪兽缺少 `attribute`
5. 怪兽缺少 `atk`
6. 非 Link 怪兽缺少 `def`
7. 非 Link 怪兽缺少 `level`
8. Link 怪兽缺少 `link.markers`
9. Pendulum 怪兽缺少刻度
10. 主卡图缺失
11. 脚本缺失
12. 场地魔法缺少场地图片

### 17.3 保存前规范化

首版保存前必须做规范化，保证模型内部不自相矛盾。

规范化要求：

1. `spell` 卡自动清空所有怪兽字段
2. `trap` 卡自动清空所有怪兽字段
3. `monster` 卡自动清空 `spell_subtype / trap_subtype`
4. Link 怪兽自动清空 `def / level`
5. 非 Link 怪兽自动清空 `link`
6. 非 Pendulum 怪兽自动清空 `pendulum`
7. `texts.strings` 规范化为固定 16 项
8. `monster_flags`、`link.markers` 去重并按固定顺序保存

## 18. 程序内安全写入要求

虽然本文档不讨论具体事务实现，但首版必须满足以下产品行为要求：

1. 程序内触发的 `workspace / pack / cards / strings` 写入不能轻易把磁盘数据写坏
2. 单文件写入应体现为“完整成功”或“保持旧内容”
3. 涉及多个文件的程序内操作，失败后应尽量保持可恢复状态，不承诺完整的崩溃可恢复原子事务
4. 程序启动时应清理残留临时文件，并对受影响 `pack` 做基本一致性检查
5. 外部编辑器修改脚本不纳入程序内回滚范围
6. 当单卡保存涉及 `code` 变更时，`CardEntity` 更新与相关资源重命名必须视为同一程序内操作
7. 若 `code` 变更涉及 `pics/<old_code>.jpg`、`pics/field/<old_code>.jpg`、`scripts/c<old_code>.lua`，程序必须尽量同步迁移到新路径；若中途失败，应优先保证作者态主 JSON 仍可恢复
8. 程序内多文件操作的计划与目标对象定位应以 `CardEntity.id` 为准，而不是以旧 `code` 或新 `code` 直接充当操作标识
8. 批量移动卡片时，实体和资源可以迁移到目标 `pack`，但不得因为移动操作重建 `CardEntity.id`

与 token 相关的运行时要求：

1. 首版应为已打开 `pack` 维护一个运行时 `revision`
2. 首版应为已打开 `pack` 维护一个磁盘状态摘要 `source_stamp`
3. 程序内成功写入后递增 `revision`
4. 手动刷新、重载 pack 或发现外部修改后，应更新 `source_stamp`
5. `confirmation_token` 与 `preview_token` 都应在 `revision` 或 `source_stamp` 变化后失效

## 19. 首版用户流程

首版应覆盖以下最小可用流程：

1. 首次启动配置 `ygopro_path`
2. 创建 `workspace`
3. 新建一个或多个 `pack`
4. 从 `pack` 列表打开一个或多个 `pack`
5. 在已打开 `pack` 的 tab 之间切换
6. 在 `pack` 中新建卡片
7. 编辑卡片数据、多语言文本、主卡图、脚本
8. 若是场地魔法，编辑场地图片
9. 编辑 `pack strings`
10. 通过搜索、排序、批量操作整理卡片
11. 关闭不再需要的 `pack` tab
12. 选择多个 `pack` 导出运行时资源

## 20. 最终结论

YGOCMG 首版的核心交付不是完整的“自定义卡生态系统”，而是一个作者态卡包管理与编辑工具。首版的重点稳定落在以下四件事上：

1. 工作区与卡包管理
2. 语义化卡片编辑
3. 多语言文本与基础资源管理
4. 从作者态到 YGOPro 运行态的可靠导出

只要这四件事定义清楚并稳定交付，后续再讨论更深的模型语义化和架构设计才有意义。
