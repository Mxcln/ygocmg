# YGOCMG 系统架构

本文档描述当前实现中的系统结构和职责边界。

## 技术栈

- 桌面壳：Tauri 2。
- 前端：React 19、TypeScript、Vite。
- 前端数据：React Query、Zustand。
- 国际化：React Intl。
- 后端：Rust 2024 edition。
- 持久化和资源：文件系统、JSON store、YGOPro CDB/SQLite 适配、图片和脚本资源。

## 运行入口

- `src/main.tsx` 创建 React 应用。
- `src/app/App.tsx` 初始化全局配置、最近 workspace、主题、shell 状态，并恢复最近会话。
- `src-tauri/src/main.rs` 创建 Tauri app，初始化 app state，注册插件和 command handler。
- `src-tauri/src/tauri_commands.rs` 是前端可调用的命令边界。

## 前端职责

- `src/app` 管理 shell、窗口控制、侧边栏、顶层视图、弹窗层、通知和会话恢复。
- `src/features` 按业务区域实现 UI：workspace、settings、pack、card、strings、export、standardPack、dialogs、language。
- `src/shared/api` 封装所有 Tauri command 调用。
- `src/shared/contracts` 定义前后端边界 DTO。
- `src/shared/stores` 保存 shell 级 UI 状态，例如当前 workspace、打开 pack、活动视图、modal 和 dialog。
- `src/shared/theme` 和 `src/shared/styles` 提供主题与共享样式。

## 后端职责

- `domain`：领域模型、值对象、校验规则和业务不变量。
- `application`：用例服务和业务流程编排，例如 card、pack、workspace、resource、import、export、standard_pack、jobs。
- `infrastructure`：文件系统、JSON store、YGOPro CDB、标准包索引、`strings.conf`、资源和外部编辑器适配。
- `runtime`：运行期 session、job、event 等状态。
- `presentation` 与 `tauri_commands.rs`：把后端服务适配成 Tauri commands。

## 数据流

典型调用链：

`React component -> src/shared/api -> invokeApi -> Tauri command -> application service -> domain/infrastructure -> DTO -> React Query/UI`

设计含义：

- UI 负责交互和展示，不直接拥有文件系统和写入规则。
- 后端拥有校验、资源写入、导入导出、标准包索引和持久化边界。
- Contracts 是前后端边界，不等同于后端内部存储模型。
- React Query 用于缓存和刷新后端读取结果。

## Shell 与运行时状态

- App 启动后加载 config 和最近 workspace 注册表。
- 如果存在最近打开 workspace，会尝试恢复 workspace、打开 pack 列表和上次激活 pack。
- Shell store 保存当前 workspace、打开 pack、活动 view、pack metadata map、pack overviews、modal 和 dialog。
- `activeView` 支持 custom pack 和 standard pack 两种顶层工作区视图。

## 写入与确认

- Card 和 Pack Strings 写入可能先返回 warning 和 confirmation token。
- 前端展示确认对话框；用户确认后调用对应 confirm command。
- 这种模式避免前端绕过后端 warning 和业务校验。
- Card 批量删除和批量移动也使用同一写入/确认边界：前端只提交一次批量请求，后端负责校验、生成 warning、持久化和资源操作。
- 批量移动会同时读取并更新源 pack 与目标 pack 的打开 session；提交时一次事务性计划写入两边 `cards.json`、pack metadata，并按选项移动主卡图、场地图和脚本资源。
- 批量确认 token 会保存涉及 pack 的 revision/source stamp；相关 pack 发生写入、关闭或重新打开时，对应确认项会失效或被清理。

## 导入导出架构

- 导入和导出采用 preview + execute 两阶段。
- Preview 负责解析输入、计算影响、返回 issues 和 preview token。
- Execute 使用 preview token 创建后台 job。
- Job 状态通过 job API 查询。

## 标准包架构

- 标准包是只读参考能力，依赖全局 YGOPro 路径和标准包源语言。
- 后端负责检测标准包状态、重建索引、搜索标准卡、搜索标准 strings、查询只读卡详情和 setname。
- 前端标准包视图负责展示状态、触发重建、轮询 job、浏览数据和打开只读 inspector。

## AI Agent 架构

- AI Agent 是对话式卡片管理能力，前端驱动工具调用循环，后端仅做 DeepSeek HTTP 转发（注入明文 API key，非流式），不引入新业务规则。
- agent 工具体复用 `src/shared/api/*`，写操作经后端两段式确认门；标准卡只暴露只读工具。
- 详见 `agent.md`。

## 主题与配置

- 全局配置由后端持久化，前端在启动时读取。
- 主题应用由前端负责，根据 `theme_mode`、`high_contrast`、`custom_brand_color` 更新 DOM/CSS token。
- 系统主题变化在 `theme_mode = system` 时触发重新应用。
