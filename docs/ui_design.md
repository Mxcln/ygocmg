# UI 设计与交互约定

本文档描述当前 UI 实现和后续改动应遵守的交互方向。

## Shell 布局

- 应用使用自定义 title bar，显示 app icon、当前 workspace 名称和窗口控制按钮。
- 主体由左侧 sidebar 和右侧 work area 组成。
- Sidebar 支持折叠和拖拽调整宽度。
- 侧边栏顶部是 workspace、export、settings 操作；中部是打开的 custom packs；底部是 standard pack 入口。
- Modal layer 用于 workspace、settings、add pack、export；notice banner 用于短反馈。

## Workspace 与 Pack 入口

- 没有 workspace 时，work area 显示空状态，提示先打开 workspace。
- Workspace modal 支持最近列表、创建和按路径打开。
- Add Pack modal 支持打开已有 pack、创建 pack 和导入 pack。
- 打开的 custom pack 显示在侧边栏，可切换和关闭。

## Custom Pack 工作区

- Custom pack work area 顶部是 pack metadata panel。
- Pack 内容以 tabs 呈现：Cards 和 Strings。
- Cards tab 使用列表/搜索/分页/排序模式，并可打开 card edit drawer。
- Strings tab 使用 Pack Strings 浏览和编辑界面。
- Card edit drawer 用于创建和编辑卡片，保存后刷新 card 查询。

## Card UI

- Card 列表展示 code、名称、描述摘要、类型、ATK/DEF/level 和资源状态。
- Custom pack 的 Card 列表支持批量选择模式：默认隐藏 checkbox，工具栏提供「选择」按钮进入选择模式，再次点「完成」或切换 pack 退出。
- 选择模式下列表左侧显示 checkbox，支持当前页全选、跨搜索/排序/分页保留选择，整行点击切换选中而不打开编辑；工具栏隐藏「新建」「选择」，保留搜索/排序/每页数量/筛选，并显示已选数量、批量移动、批量删除、完成。
- 批量移动通过对话框选择目标 pack；目标只包含当前 workspace 内已打开的其他 custom pack，不显示当前 pack 或 standard pack。
- 批量删除使用危险确认对话框；批量删除/移动固定同时处理卡图、场地图和脚本资源，不再提供资源开关；出现 warning 时使用统一 warning 确认对话框。
- 高级搜索通过独立 panel 组织，不把所有筛选项塞进列表行。
- Card 表单拆分基础信息、文本、多语言和类型相关字段。
- 资源栏展示主卡图、场地图、脚本状态，并通过后端 API 完成导入、删除和打开。
- 写入 warning 使用统一确认对话框，而不是在表单中静默忽略。

## Pack Strings UI

- Pack Strings 支持按语言、类型、key 和关键字浏览。
- `setname` 和标准 setname 相关信息应通过现有 entries/merge 逻辑接入，不在 UI 中硬编码。
- 多语言编辑应尊重全局语言目录和 pack 显示语言顺序。

## Standard Pack UI

- Standard Pack 是独立顶层视图，不要求打开 custom pack。
- 顶部 meta bar 显示索引状态、卡片数、源语言、索引时间、YGOPro 路径和 CDB 路径。
- Meta 区域可展开，并提供设置入口和重建索引按钮。
- 重建索引时显示 job strip，包括状态、阶段、进度和错误。
- Cards 和 Strings 使用 tabs 切换。
- 标准卡列表支持关键字、排序、分页和高级筛选。
- 标准卡详情以只读 inspector 展示。

## 设置 UI

- Settings modal 编辑全局配置，包括 UI 语言、YGOPro 路径、外部文本编辑器、编号推荐、文本语言目录、标准包源语言、主题、高对比度和品牌色。
- 保存后应用主题，并刷新标准包状态和数据查询。

## 主题与视觉

- 样式主要使用 CSS modules 和共享样式模块。
- 主题通过 CSS tokens 和 `shared/theme` 应用。
- 支持 `system`、`light`、`dark` 主题模式，以及 high contrast 和 custom brand color。
- 新 UI 应优先使用现有 button、tab、empty、drawer、modal、notice 风格，避免引入新的视觉系统。

## i18n

- 用户可见文本应走现有 i18n message，而不是直接写死在组件中。
- 领域语言选择应使用 text language catalog 和已有语言工具函数。

## 交互约定

- 需要用户确认的危险或有 warning 的写入使用统一 dialog。
- 长任务使用 job 状态展示，不让用户以为操作卡死。
- 空状态要说明当前缺少什么条件，例如未打开 workspace、未配置标准包、未建立索引。
- 后续 UI 改动应保持 shell/sidebar/work area 的基本结构稳定。
