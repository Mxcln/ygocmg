# YGOCMG Agent Guide

YGOCMG 是一个基于 Tauri 2、React 和 TypeScript 的本地卡包管理工具，用于管理自定义 Yu-Gi-Oh 卡片、卡包、文本、资源、导入导出和标准卡包只读参考数据。

## 当前事实来源

新对话或新任务开始时优先阅读：

1. `docs/README.md`：文档入口和阅读顺序。
2. `docs/functional_spec.md`：当前功能事实。
3. `docs/system_architecture.md`：当前架构和运行边界。
4. `docs/code_structure_api.md`：代码结构、Tauri commands、frontend API wrappers 和 contracts。
5. `docs/ui_design.md`：当前 UI 布局与交互约定。

`docs/history/*` 是历史资料，只能作为背景参考。不要把历史设计稿、评审稿或旧规范直接当成当前实现事实。

## 项目边界

- 前端入口在 `src/main.tsx`，主 shell 在 `src/app/App.tsx`。
- 前端业务 UI 按领域放在 `src/features/*`。
- 前端到后端的调用集中在 `src/shared/api/*`。
- 前后端边界类型放在 `src/shared/contracts/*`。
- Tauri 后端入口在 `src-tauri/src/main.rs`，命令表面在 `src-tauri/src/tauri_commands.rs`。
- 后端分层主要是 `application`、`domain`、`infrastructure`、`runtime`、`presentation`。

## 工作规则

- 先查当前代码和新权威文档，再参考历史文档。
- 优先沿用已有 feature、API wrapper、contract、CSS module、React Query、Zustand、Tauri command patterns。
- 不要绕过 `src/shared/api` 直接在 UI 中拼后端调用。
- 涉及文件系统、资源写入、导入导出、编号变更和验证规则时，让后端拥有最终业务规则。
- 标准卡包是只读参考数据，不要当作用户可编辑包处理。
- 文档变更应同步保持 `docs/README.md` 和相关核心文档准确。

## 常用命令

- `npm run dev`：启动 Vite 开发服务。
- `npm run typecheck`：运行 TypeScript 类型检查。
- `npm run build`：运行类型检查并构建前端。
- `npm run tauri`：调用 Tauri CLI。
- 后端 Rust 检查在 `src-tauri` 下运行 `cargo check` 或 `cargo test`。

## 文档维护规则

- 当功能事实、架构边界、API 调用面、UI 约定或验证方式发生稳定变化时，更新对应核心文档。
- 一次性 bug、临时实验和未稳定设计不必进入核心文档。
- 新文档应该描述当前实现，不复述历史计划。

