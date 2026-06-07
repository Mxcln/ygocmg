# YGOCMG UI 主题系统设计文档

日期：2026-05-06  
状态：Draft  
范围：全局 UI 主题、深浅色模式、主题配置、主题 token 分层、主题启动与持久化策略

相关文档：
- [YGOCMG v1 UI Specification](./ygocmg_v1_ui_spec_2026-04-26.md)
- [YGOCMG v1 Functional Specification](./ygocmg_v1_functional_spec_2026-04-25.md)
- [YGOCMG v1 Interface Design](./ygocmg_v1_interface_design_2026-04-25.md)
- [Card Text Language Design](./card_text_language_design.md)

## 0. 结论摘要

本方案建议 YGOCMG 的主题系统采用以下结构：

1. `GlobalConfig` 增加 `theme_mode` 字段，支持 `system`、`light`、`dark` 三态。
2. 运行时单独计算 `resolved_theme`，仅有 `light` 与 `dark` 两态，不写回配置。
3. 主题应用由前端负责，Tauri / Rust 负责配置持久化、默认值、校验与兼容加载。
4. 主题是全局壳层能力，不区分 Standard Pack 与 Custom Pack 两套主题。
5. 现有 CSS 语义 token 体系继续沿用，`tokens.css` 扩展为 light / dark 两套 token 定义。
6. 第一阶段不做完整“自定义皮肤编辑器”，只做“跟随系统 / 浅色 / 深色”。
7. 启动防闪采用“双层策略”：
   1. CSS 默认跟随系统。
   2. 前端用一个轻量 `localStorage` 镜像在 React 启动前尽早应用用户上次显式主题选择。

一句话概括：

```text
theme_mode 是用户选择
resolved_theme 是当前实际显示结果
CSS variables 是唯一主题入口
```

## 1. 背景与当前状态

### 1.1 当前项目已有的良好基础

当前项目已经具备实现主题系统的关键前提：

1. 已有统一的全局配置模型。
2. 已有前端与 Rust 对齐的 `GlobalConfig` 合同。
3. 已有全局样式入口与 token 文件。
4. 当前 UI 已普遍使用语义化 CSS 变量，而不是把颜色全写死在组件里。

当前相关文件位置如下：

```text
src/shared/contracts/config.ts
src-tauri/src/domain/config/model.rs
src-tauri/src/domain/config/rules.rs
src/app/App.tsx
src/main.tsx
src/app/styles.css
src/shared/styles/tokens.css
src/shared/styles/reset.css
src/features/settings/SettingsModal.tsx
```

### 1.2 当前实现状态

截至本设计编写时：

1. [src/shared/styles/tokens.css](../src/shared/styles/tokens.css) 只有一套浅色 token。
2. [src/app/styles.css](../src/app/styles.css) 负责导入 `tokens.css` 与 `reset.css`。
3. [src/shared/styles/reset.css](../src/shared/styles/reset.css) 已经通过 `var(--text-0)` 与 `var(--bg-0)` 驱动 `body` 的基础前景色和背景色。
4. [src/features/settings/SettingsModal.tsx](../src/features/settings/SettingsModal.tsx) 已有 General 标签页，适合作为主题设置入口。
5. [src/shared/contracts/config.ts](../src/shared/contracts/config.ts) 当前尚无 `theme_mode` 字段。
6. 当前标题栏是自绘 Web UI，而不是依赖原生系统标题栏主题，因此前端主导主题切换是可行的。

### 1.3 当前存在的问题

虽然项目已经有 token 基础，但要支持 dark/light 仍有几个现实问题：

1. token 目前只有浅色方案，没有 dark 主题块。
2. 一些共享样式仍存在硬编码颜色，深色模式下会失真或不可读。
3. 配置层没有“主题模式”的正式字段。
4. App 启动时配置通过异步 Tauri 初始化返回，如果没有提前应用主题，容易发生首帧闪白或错误主题闪现。
5. Settings 目前没有“外观”配置分组，用户没有可见入口管理主题。

### 1.4 已发现的硬编码颜色样例

以下只是目前已确认的代表性问题，不是完整清单：

1. [src/shared/styles/shared.module.css](../src/shared/styles/shared.module.css) 中 `input` / `textarea` 仍直接使用 `background: white;`
2. 同文件中 `primaryButton:hover` 使用了固定值 `#0e5051`
3. 多个模块内存在重复的 `rgba(...)` 阴影、边框强调色与品牌色透明度写法

这些值在浅色下问题不大，但在深色主题中会造成：

1. 输入框背景突兀发白
2. hover 状态亮度不自然
3. 阴影与描边层级失衡

## 2. 设计目标

本方案的第一阶段目标如下：

1. 支持 `system / light / dark` 三种主题模式选择。
2. 在配置中持久化用户主题偏好。
3. 在系统主题变化时自动响应，仅当用户选择 `system` 时生效。
4. 让现有主要界面在 dark 模式下保持可读、可用、层级清晰。
5. 尽量复用现有 token 与全局配置架构，避免大面积重构组件逻辑。
6. 保证主题切换后 Standard Pack 与 Custom Pack 体验统一，不产生“两个产品”的割裂感。
7. 保证后续可以平滑扩展品牌色、自定义主题或高对比度模式。

## 3. 非目标

第一阶段明确不做以下内容：

1. 不做“用户自由编辑全部颜色”的完整主题编辑器。
2. 不做 Standard Pack 与 Custom Pack 两套独立主题。
3. 不做按 workspace、按 pack、按弹窗分别保存主题。
4. 不把深浅色判断散落到每个组件的业务逻辑中。
5. 不依赖 Rust 端直接驱动前端组件颜色切换。
6. 不优先处理 macOS / Linux 平台的原生窗口材质主题联动。

## 4. 关键产品决策

### 4.1 主题是全局能力，不是 pack 级能力

本设计明确选择：

1. 主题以整个应用 shell 为作用范围。
2. Standard Pack 与 Custom Pack 使用同一套主题系统。
3. 不额外提供“标准包主题”和“自定义包主题”两个设置项。

原因如下：

1. 用户心智更简单。主题是“应用外观”，不是“数据源外观”。
2. 当前主壳层、标题栏、侧边栏、模态框、表单组件都在两个视图间共享。
3. 如果按 pack 类型拆两套主题，会显著增加 token 数量、样式分支和测试复杂度。
4. Standard 与 Custom 的视觉差异可以通过局部组件语言、数据标识与交互重点体现，而不必上升为两套主题。

### 4.2 主题模式与实际主题分离

必须区分两个概念：

1. `theme_mode`
2. `resolved_theme`

定义如下：

```text
theme_mode: 用户保存的偏好
- system
- light
- dark

resolved_theme: 当前真正显示的主题
- light
- dark
```

解析规则如下：

1. `theme_mode = light` 时，`resolved_theme = light`
2. `theme_mode = dark` 时，`resolved_theme = dark`
3. `theme_mode = system` 时，`resolved_theme` 由系统偏好决定

这个分离非常重要，因为：

1. 设置页需要显示“用户选了什么”
2. DOM 与 CSS 需要知道“当前正在应用什么”
3. 如果两者混在一起，后续监听系统变化与展示设置状态时会变得混乱

### 4.3 第一阶段主题切换遵循“保存后生效”

本设计建议第一阶段主题设置遵循当前 Settings 的一致交互模型：

1. 用户在 Settings 中修改 `theme_mode`
2. 该修改先保存在 `draft`
3. 点击 `Save Settings` 后写入配置并正式生效
4. 关闭或放弃未保存更改时，不改变当前应用主题

理由如下：

1. 现有 SettingsModal 已采用“草稿编辑 -> 显式保存”的事务式交互。
2. 这样可以避免为主题额外引入“预览并回滚”的全局临时状态。
3. 第一阶段复杂度更低，也更符合当前代码结构。

后续如果产品希望“切换主题时立即预览”，可以在第二阶段引入临时预览状态，但这不是本方案的首期要求。

## 5. 配置模型设计

### 5.1 TypeScript 合同

建议在前端配置合同中新增：

```ts
export type ThemeMode = "system" | "light" | "dark";

export interface GlobalConfig {
  app_language: LanguageCode;
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
  standard_pack_source_language: LanguageCode | null;
  theme_mode: ThemeMode;
}
```

### 5.2 Rust 配置模型

Rust 端建议引入封闭枚举，而不是自由字符串：

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    System,
    Light,
    Dark,
}
```

`GlobalConfig` 增加：

```rust
#[serde(default = "default_theme_mode")]
pub theme_mode: ThemeMode,
```

### 5.3 默认值

默认值建议为：

```text
theme_mode = system
```

原因：

1. 首次安装时最符合桌面应用用户预期。
2. 不会强行把用户已有系统深色工作环境改成浅色。
3. 不会为未来多平台适配制造额外分支。

### 5.4 兼容与容错要求

旧配置文件没有 `theme_mode` 字段时，必须平滑加载为 `system`。

建议后端遵循以下兼容原则：

1. 缺失字段时，自动回落到默认值。
2. 无效值不应导致应用启动失败。
3. 无效值应在加载后被规范化为 `system`，或在保存时被覆盖为合法值。

实现上可以使用：

1. 自定义反序列化容错
2. 或加载后 normalize 回 `system`

本设计不强制具体代码写法，但强制结果语义：

```text
旧配置不会因新增主题字段而损坏启动链路
```

## 6. 前端主题状态模型

### 6.1 建议的数据结构

前端建议显式定义：

```ts
export type ThemeMode = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";
```

并集中实现以下工具：

1. `resolveTheme(mode, systemPrefersDark)`
2. `applyResolvedThemeToDocument(theme, mode)`
3. `readBootThemeModeMirror()`
4. `writeBootThemeModeMirror(mode)`
5. `watchSystemTheme(listener)`

建议放置于：

```text
src/shared/theme/theme.ts
```

### 6.2 不建议把主题状态塞进业务 store

本方案不建议把主题模式放进 `shellStore`，原因如下：

1. 主题属于全局配置，而不是临时页面状态。
2. App 顶层已经持有 `config`，复用当前配置流更自然。
3. 把主题状态拆到另一个 store 会制造“双数据源”问题。

推荐做法：

1. `config.theme_mode` 是唯一权威配置值。
2. App 顶层根据它计算并应用 `resolved_theme`。
3. Settings 只修改 `draft.theme_mode`，保存后经配置流回到 App 顶层。

### 6.3 系统主题监听

当且仅当 `theme_mode === "system"` 时，前端应监听：

```text
window.matchMedia("(prefers-color-scheme: dark)")
```

行为要求：

1. 初始化时读取当前 `matches`
2. 系统切换深浅色时重新解析并应用 `resolved_theme`
3. 用户切到 `light` 或 `dark` 后，系统变化不再影响界面

## 7. DOM 应用约定

### 7.1 根节点属性

建议统一把主题应用到 `document.documentElement`，即 `html` 节点，而不是某个局部容器。

推荐约定：

```html
<html data-theme="dark" data-theme-mode="system">
```

说明：

1. `data-theme` 只放最终生效值 `light` 或 `dark`
2. `data-theme-mode` 放用户选择值，主要用于调试、排查和未来可能的选择器扩展

### 7.2 color-scheme 同步

每次应用主题时，应同步设置：

```ts
document.documentElement.style.colorScheme = resolvedTheme;
```

这样可以帮助浏览器 / WebView 正确渲染：

1. 原生表单控件
2. 滚动条
3. 其他依赖 `color-scheme` 的内建表现

## 8. Token 体系设计

### 8.1 保留现有语义命名

现有 token 命名已经较合理，建议保留，不做破坏性重命名：

1. `--bg-0`
2. `--bg-1`
3. `--bg-2`
4. `--panel`
5. `--panel-strong`
6. `--panel-muted`
7. `--line`
8. `--line-strong`
9. `--text-0`
10. `--text-1`
11. `--text-2`
12. `--brand`
13. `--brand-soft`
14. `--accent`
15. `--accent-soft`
16. `--danger`
17. `--danger-soft`
18. `--success`
19. `--warning`

### 8.2 建议新增 token

为支持深色主题，建议新增以下通用 token：

1. `--overlay`
2. `--shadow-1`
3. `--shadow-2`
4. `--focus-ring`
5. `--text-on-brand`
6. `--surface-input`
7. `--surface-input-disabled`
8. `--brand-hover`

新增这些 token 的意义：

1. 阴影不应在组件里写死 `rgba(0, 0, 0, 0.08)` 这类值
2. 输入框背景不能直接假设永远是白色
3. 品牌色 hover 不应固定为单个浅色主题专用 hex
4. focus ring 在 dark 下通常需要更清晰的对比

### 8.3 推荐的 `tokens.css` 结构

推荐把 [src/shared/styles/tokens.css](../src/shared/styles/tokens.css) 改造成：

```css
:root {
  color-scheme: light;
}

:root[data-theme="light"] {
  /* light tokens */
}

:root[data-theme="dark"] {
  /* dark tokens */
}
```

考虑到 React 启动前还可能没有设置 `data-theme`，建议同时保留基于系统的默认兜底：

```css
:root {
  /* light default */
}

@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    /* dark fallback when no explicit light override exists yet */
  }
}

:root[data-theme="light"] {
  /* explicit light */
}

:root[data-theme="dark"] {
  /* explicit dark */
}
```

这样可以得到两层保障：

1. 没有任何 JS 干预时，应用默认跟随系统
2. 用户若显式选择 light/dark，JS 可以通过 `data-theme` 覆盖默认系统兜底

### 8.4 dark 主题的基础调色原则

dark 主题不应只是把背景改黑，而应同时调整：

1. 文本层级
2. 边框对比度
3. 阴影强度
4. 品牌色亮度
5. panel 与背景之间的层级差

建议遵循这些原则：

1. `--bg-*` 与 `--panel-*` 的亮度差应小于浅色主题，但仍必须可区分。
2. `--line` 在 dark 中应偏低对比，避免界面“满屏格线”。
3. `--brand` 在 dark 中通常需要比浅色模式更亮一些，否则按钮和焦点不够醒目。
4. `--brand-soft` 在 dark 中不能直接沿用浅色模式的透明度，否则会过脏或过灰。
5. 危险色与成功色在 dark 中应保证足够对比，但避免刺眼。

### 8.5 示例 dark token 草案

以下值不是最终视觉稿，只是建议的首轮落点：

```css
:root[data-theme="dark"] {
  --bg-0: #171a1c;
  --bg-1: #1d2124;
  --bg-2: #23282b;
  --panel: #1f2427;
  --panel-strong: #252b2f;
  --panel-muted: #1a1e21;
  --line: rgba(236, 227, 215, 0.12);
  --line-strong: rgba(236, 227, 215, 0.2);
  --text-0: #f1eadf;
  --text-1: #cdbfae;
  --text-2: #9b8f82;
  --brand: #4fa3a4;
  --brand-soft: rgba(79, 163, 164, 0.18);
  --accent: #d4a15d;
  --accent-soft: rgba(212, 161, 93, 0.16);
  --danger: #d46b61;
  --danger-soft: rgba(212, 107, 97, 0.16);
  --success: #57a27e;
  --warning: #d3a04a;
  --overlay: rgba(8, 10, 11, 0.48);
  --shadow-1: 0 2px 8px rgba(0, 0, 0, 0.28);
  --shadow-2: 0 12px 28px rgba(0, 0, 0, 0.34);
  --focus-ring: rgba(79, 163, 164, 0.28);
  --text-on-brand: #081414;
  --surface-input: #15191b;
  --surface-input-disabled: #202528;
  --brand-hover: #5ab1b2;
  color-scheme: dark;
}
```

首版实现可以先从这个方向开始，再通过实际运行效果微调。

## 9. 启动防闪与主题引导策略

### 9.1 问题来源

当前应用的配置通过：

```text
configApi.initialize()
```

异步返回。  
这意味着在 React 真正拿到配置之前，页面已经可能先以默认样式绘制一帧。

如果直接把默认 token 定死为 light，则会出现：

1. 系统 dark 用户首帧闪白
2. 用户保存了 explicit dark 时启动仍先短暂显示浅色

### 9.2 推荐方案：CSS 系统兜底 + localStorage 镜像

建议采用双层方案：

第一层，CSS 系统兜底：

1. `tokens.css` 默认用 light
2. `@media (prefers-color-scheme: dark)` 提供 dark fallback

第二层，前端本地镜像：

1. 每次成功保存配置时，把 `theme_mode` 同步写入 `localStorage`
2. 在 React 挂载前读取 `localStorage`
3. 若用户上次明确选择 `light` 或 `dark`，则立即设置根节点 `data-theme`
4. 待 Tauri 配置初始化完成后，再用真实配置重新应用一次

### 9.3 镜像与权威源关系

必须明确：

1. `GlobalConfig.theme_mode` 才是权威数据源
2. `localStorage` 只是启动优化镜像
3. 如果两者不一致，以 `GlobalConfig.theme_mode` 为准

### 9.4 前端入口位置

推荐在 [src/main.tsx](../src/main.tsx) 中，在 `ReactDOM.createRoot(...).render(...)` 之前调用主题引导：

```ts
bootstrapThemeFromMirror();
```

该函数只做以下事情：

1. 读取镜像 `theme_mode`
2. 调用 `matchMedia`
3. 计算 `resolved_theme`
4. 设置 `document.documentElement.dataset.theme`
5. 设置 `document.documentElement.dataset.themeMode`
6. 设置 `document.documentElement.style.colorScheme`

这样即便 App 主体尚未初始化，也能尽快应用接近正确的主题。

## 10. 设置页交互设计

### 10.1 入口位置

主题入口建议放在：

```text
Settings -> General -> Appearance
```

理由：

1. 主题属于全局外观偏好
2. 与 `app_language` 同属顶层应用设置
3. 不应混入 Standard Pack 或 Code Policy 这类业务设置

### 10.2 控件形式

第一阶段推荐使用：

1. `select`
2. 或三段式 segmented control

可选值：

1. `Follow system`
2. `Light`
3. `Dark`

如果要与当前表单体系保持最小改动，首版使用 `select` 即可。

### 10.3 文案建议

建议新增以下 i18n key：

```text
settings.group.appearance
settings.themeMode
settings.themeMode.system
settings.themeMode.light
settings.themeMode.dark
settings.themeMode.help
```

示例文案语义：

1. `Appearance`
2. `Theme`
3. `Follow system`
4. `Light`
5. `Dark`
6. `Choose how the application matches your desktop appearance.`

## 11. 代码组织建议

### 11.1 推荐新增或改动的文件

后端配置层：

```text
src-tauri/src/domain/config/model.rs
src-tauri/src/domain/config/rules.rs
src-tauri/tests/minimal_authoring_flow.rs
```

前端合同与配置流：

```text
src/shared/contracts/config.ts
src/shared/api/configApi.ts
src/app/App.tsx
src/main.tsx
```

主题基础设施：

```text
src/shared/theme/theme.ts
```

样式层：

```text
src/shared/styles/tokens.css
src/shared/styles/reset.css
src/shared/styles/shared.module.css
src/app/styles.css
```

设置页与文案：

```text
src/features/settings/SettingsModal.tsx
src/shared/i18n/messages/en-US.ts
src/shared/i18n/messages/zh-CN.ts
src/shared/i18n/messages/ja-JP.ts
```

### 11.2 建议的 `theme.ts` 职责

`src/shared/theme/theme.ts` 建议集中提供：

1. `ThemeMode` 与 `ResolvedTheme` 类型
2. `resolveTheme`
3. `applyThemeToDocument`
4. `readThemeModeMirror`
5. `writeThemeModeMirror`
6. `watchSystemTheme`

这样可以避免：

1. `App.tsx` 自己处理全部底层细节
2. `SettingsModal` 直接操作 DOM
3. `main.tsx` 与运行时主题逻辑出现重复实现

## 12. 样式迁移策略

### 12.1 第一步：先让 token 双主题可用

先做：

1. `tokens.css` 增加 dark 块
2. `reset.css` 与根节点颜色应用验证
3. App 启动能正确切换 `data-theme`

在这一步之后，理论上所有已完全依赖 token 的组件都会初步具备 dark 能力。

### 12.2 第二步：审计硬编码颜色

接着清理硬编码颜色，优先顺序如下：

1. 输入框、文本域、select 背景
2. 共享按钮 hover / active 状态
3. modal 与 overlay 背景
4. sidebar / titlebar / drawer 的边框与阴影
5. 各模块中重复出现的品牌色透明度写法

### 12.3 第三步：做深色视觉微调

完成基础替换后，再针对 dark 单独调整：

1. panel 与背景对比
2. 文本层级
3. 焦点环可见性
4. 选中态品牌色强度
5. 阴影与分割线存在感

这一步不应和“新增主题模式配置”耦合到一起，否则排查问题会很困难。

## 13. Tauri 与前端的职责边界

### 13.1 Tauri / Rust 负责

1. 定义 `theme_mode` 配置字段
2. 提供默认值
3. 负责配置持久化
4. 负责兼容旧配置加载
5. 负责保存时的合法值约束

### 13.2 前端负责

1. 解析 `theme_mode`
2. 监听系统主题
3. 计算 `resolved_theme`
4. 把主题应用到 DOM
5. 驱动 Settings 中的主题选择 UI
6. 维护启动镜像以减少首帧闪烁

### 13.3 当前阶段不要求 Tauri 负责的事

1. 不要求 Rust 直接判断并推送系统主题
2. 不要求原生标题栏颜色同步
3. 不要求平台特有材质效果联动

因为当前项目的主界面是 Web UI 主导，这些能力不是第一阶段主题系统的必要条件。

## 14. 验证与验收标准

### 14.1 功能验收

实现完成后应满足：

1. 首次启动默认跟随系统主题。
2. 用户选择 `light` 后，重启应用仍保持 light。
3. 用户选择 `dark` 后，重启应用仍保持 dark。
4. 用户选择 `system` 后，系统切换深浅色时应用能同步变化。
5. Standard Pack 与 Custom Pack 不会因主题而出现两套不一致壳层。
6. 标题栏、侧边栏、模态框、表单输入、主按钮、列表、抽屉在 dark 下可读可用。
7. 没有明显“白底输入框”或“浅色 hover 残留”。

### 14.2 建议验证命令

前端验证：

```text
npm run typecheck
npm run build
```

后端验证：

```text
cargo test --offline --test minimal_authoring_flow
cargo test --offline
```

### 14.3 人工视觉检查点

至少人工检查这些界面：

1. 主壳层启动页
2. 侧边栏与 pack 列表
3. Settings Modal
4. Workspace Modal
5. Add Pack Modal
6. Standard Pack 浏览页
7. Card 编辑抽屉
8. Export Modal

## 15. 风险与注意事项

### 15.1 最大风险：表面支持主题，实则残留大量浅色硬编码

如果只加 `theme_mode` 和 dark token，不清理共享样式中的硬编码颜色，最终效果会非常割裂。  
因此本方案要求把“共享样式 audit”视为主题实施的一部分，而不是可无限推后的细节。

### 15.2 启动闪烁风险

如果不做启动镜像或至少不做系统兜底：

1. dark 用户会看到闪白
2. explicit light/dark 用户会看到错误首帧

这会显著拉低桌面应用质感。

### 15.3 过早扩展自定义皮肤风险

如果第一阶段就加入：

1. 自定义品牌色
2. 每种状态单独调色
3. 多套视觉风格模板

会显著拉长实施周期，并稀释最核心的 dark/light 交付目标。

## 16. 分阶段实施建议

### Phase 1：主题模式与基础切换

范围：

1. `theme_mode` 配置字段
2. light / dark 两套 token
3. `data-theme` 与 `color-scheme`
4. Settings 主题选择入口
5. 系统主题监听
6. 启动镜像

目标：

```text
功能可用，基础界面可读
```

### Phase 2：共享样式与关键界面修正

范围：

1. 清理共享样式中的硬编码颜色
2. 修正模态框、输入框、按钮、抽屉、侧边栏等关键界面
3. 微调 dark 下的阴影、边框、焦点环

目标：

```text
主要路径视觉一致，没有明显浅色残留
```

### Phase 3：增强能力

可选扩展：

1. live preview
2. 高对比度模式
3. 自定义品牌色
4. 更多预设主题

这些能力应建立在 Phase 1 与 Phase 2 已稳定的前提上。

## 17. 最终建议

对于当前 YGOCMG 项目，最稳妥、最符合现有代码结构的方案是：

1. 用 `theme_mode(system/light/dark)` 作为配置字段。
2. 用 `resolved_theme(light/dark)` 作为运行时结果。
3. 用根节点 `data-theme` 和 `color-scheme` 驱动全局样式。
4. 继续以 `tokens.css` 中的语义变量作为唯一主题入口。
5. 把主题视为全局应用外观，不为 Standard / Custom 拆两套体系。
6. 第一阶段只做三态主题模式，不做完整皮肤系统。
7. 主题落地时必须同步审计共享样式中的硬编码颜色，否则 dark 模式质量会明显不稳定。

如果按这个方案实施，后续无论是扩展品牌色、增加高对比模式，还是引入更完整的设计 token 分层，都不会推翻现有架构。
