# YGOCMG 当前功能描述

本文档描述当前实现中的功能事实。它不是未来路线图，也不继承历史设计稿中的未实现承诺。

## 产品定位

YGOCMG 是一个本地桌面应用，用于维护自定义 Yu-Gi-Oh 卡包。用户可以创建或打开 workspace，在其中管理自定义 pack、卡片数据、Pack Strings、卡图、场地图、脚本，并可以从 YGOPro 兼容资源中导入、导出或只读浏览标准卡数据。

## Workspace

- Workspace 是用户工作的顶层容器，包含 pack 顺序、最近打开 pack、当前打开 pack 会话等元数据。
- 应用启动时会加载全局配置和最近 workspace 注册表，并尝试恢复最近打开的 workspace 和 pack 会话。
- Workspace 支持创建、按路径打开和从最近列表打开。
- 侧边栏在没有 workspace 时禁用 pack 创建/打开和导出入口。

## Pack

- 当前实现区分 `custom` 和 `standard` 两类 pack。
- Custom Pack 是用户可编辑卡包，包含名称、`pack_code`、作者、版本、描述、显示语言顺序、默认导出语言等元数据。
- 用户可以创建、打开、关闭、更新和删除 custom pack。
- 打开的 custom pack 会显示在侧边栏，并可在多个 pack 之间切换。
- Standard Pack 是只读参考入口，不作为用户可编辑 pack 写入。

## Card

- Card 使用 `CardEntity` 表示，核心字段包括 `code`、`alias`、`setcodes`、`ot`、`category`、`primary_type`、多语言 `texts`、怪兽/魔法/陷阱相关字段、Link 和 Pendulum 数据、创建/更新时间。
- Card 列表支持关键字、分页、排序、高级筛选和批量选择；切换搜索、排序或分页时保留已选卡片，列表刷新后会清理当前 pack 中已不存在的选择。
- Card 编辑通过抽屉完成，支持创建、读取、更新和删除。
- Custom Pack 中支持批量删除选中卡片；同时删除对应主卡图、场地图和脚本资源（不再提供保留孤儿资源的选项）。
- Custom Pack 中支持把选中卡片批量移动到当前 workspace 内另一个已打开 custom pack；移动会一并搬运对应主卡图、场地图和脚本资源，并保留卡片 `id`、`code` 和创建时间。
- 批量移动不支持 standard pack、未打开 pack、同一 pack 或跨 workspace；目标 pack 已有相同 card id 时失败，目标 pack 同 code 和目标资源路径已存在会先以 warning 进入确认流程。
- 写入可能返回 `needs_confirmation`，前端通过统一确认对话框展示 warning，并使用 confirmation token 完成确认写入。
- 编号推荐通过后端提供，使用全局配置中的推荐范围和间隔规则。卡片 code 与 setname base 各有独立的推荐区段配置。

## Card Texts 与多语言

- Card 文本以语言为 key 保存，每个语言包含 `name`、`desc` 和 `strings`。
- 全局配置包含 `text_language_catalog`，用于描述内置/自定义语言、隐藏状态和最近使用信息。
- Pack 元数据包含显示语言顺序和默认导出语言。
- 标准包索引使用 `standard_pack_source_language` 作为源语言配置。

## Pack Strings

- Pack Strings 支持 `system`、`victory`、`counter`、`setname` 类型。
- 用户可以按语言、类型、key 和关键字浏览 Pack Strings。
- Pack Strings 支持单条 upsert、整条记录 upsert、删除、删除某个语言翻译。
- Pack Strings 写入同样可能进入 warning + confirmation token 流程。
- 新建 setname 时后端可按配置的推荐 base 区段建议下一个空闲的顶级 base key；strings 面板新建行切到 setname 且 key 为空时自动预填，agent 新建系列时也复用同一建议。

## 资源管理

- 每张卡可以关联主卡图、场地图和脚本。
- 资源操作包括导入/删除主卡图、导入/删除场地图、创建空脚本、导入/删除脚本、用外部编辑器打开脚本。
- 资源状态通过 `has_image`、`has_field_image`、`has_script` 暴露给前端。
- 资源写入、批量删除资源、批量移动资源和编号一致性由后端负责，前端只通过 API wrapper 发起操作。
- 自定义包中的卡片脚本支持验证：后端可读取当前保存的 `scripts/c{code}.lua`，或验证调用方传入的未保存 `scriptText` 草稿，并返回结构化状态、阶段结果、issues 和 limitations。默认验证包含 deterministic static check 和独立 helper 进程中的 ocgcore load/init；`ocgcore_init` 只证明脚本能加载并执行 `initial_effect`，不证明效果语义正确。helper 未构建、超时、崩溃或输出异常时，报告以 `inconclusive` 表达而不是让 Tauri command 失败。

## 导入

- 导入入口在添加 pack 弹窗内。
- 导入以 preview + execute 两步完成。
- Preview 输入包括目标 pack 元数据、CDB 路径、图片目录、场地图目录、脚本目录、`strings.conf` 路径和源语言。
- Preview 返回卡片数、缺失资源统计、warning/error 数量和 issues。
- Execute 使用 preview token，返回后台 job。

## 导出

- 导出入口在侧边栏。
- 导出以 preview + execute 两步完成。
- Preview 输入包括 workspace、pack 列表、导出语言、输出目录和输出名称。
- Preview 返回 pack/card/resource 统计以及 warning/error issues。
- Execute 使用 preview token，返回后台 job。

## 标准包只读浏览

- 标准包入口位于侧边栏底部。
- 标准包依赖全局 YGOPro 路径和标准包源语言。
- 标准包状态包括未配置、缺语言、缺源、缺索引、过期、ready、error 等状态。
- 用户可以重建标准包索引；重建过程以 job 形式轮询展示。
- ready 后可浏览标准卡列表、标准 strings，并打开卡片只读详情。
- 标准卡浏览支持关键字、分页、排序、高级筛选和 setname 查询。
- 标准脚本可通过外部编辑器打开，但标准数据本身不在 YGOCMG 中编辑。

## 设置与全局配置

- 设置包括 UI 语言、YGOPro 路径、外部文本编辑器路径、编号推荐范围（卡片 code 与 setname base）、shell 侧边栏状态、文本语言目录、标准包源语言、主题模式、高对比度、自定义品牌色，以及 AI Agent 的 DeepSeek API key 和回复语言。
- 主题模式支持 `system`、`light`、`dark`。
- AI Agent 设置集中在独立的"AI 助手"设置 tab。
- 保存配置后会更新主题，并刷新标准包相关查询。

## Job

- 导入、导出和标准包索引重建会返回 job。
- 前端可查询单个 job 状态，也可列出 active jobs。
- UI 当前会对标准包重建 job 做轮询，并展示状态、阶段、进度和错误。

## AI Agent

- AI Agent 是对话式助手，用户用自然语言对当前激活的 custom pack 完成卡片查询、创建、修改、移动和删除，由 DeepSeek 模型驱动。
- agent 通过工具调用复用后端业务规则：只读工具（列卡、读卡、搜索标准卡、读配置、列 pack、建议 code、列 setname）和写工具（建卡、改卡、移动卡、删卡、建/改/删系列名）；改卡覆盖全部可编辑字段，移动和删除均使用批量工具形态，单卡操作传一个 card id；建系列名复用后端 setname key 建议，删系列名经用户确认；写操作经统一确认流程。
- agent 以右侧边栏形式呈现，可收缩；助手回复以 Markdown 渲染，写操作确认卡片内联展示。
- 回复语言由配置决定，可跟随程序 UI 语言或显式指定。
- 未配置 DeepSeek API key 时边栏提示前往设置；对话历史不持久化。
- 详细功能、架构与边界见 `agent.md`。
