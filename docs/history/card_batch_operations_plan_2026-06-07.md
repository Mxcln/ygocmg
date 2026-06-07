# 卡片批量移动与批量删除实施计划

## Summary

本次改动分四部分推进：后端设计、前端设计、测试、i18n 与文档更新。第一步创建计划文件 `docs/history/card_batch_operations_plan_2026-06-07.md`，内容即本计划；随后按本计划实现。

默认采用 v1 范围：支持当前 workspace 内 **已打开 custom pack 之间** 的批量移动；支持当前 custom pack 内批量删除；移动默认搬运资源；批量删除默认提供“同时删除资源”选项；所有批量写入由后端一次性提交，避免前端循环调用单卡 API。

## Backend Design

- 新增 card 批量写入接口：
  - `bulk_delete_cards(input) -> WriteResult<BulkDeleteCardsResult>`
  - `move_cards(input) -> WriteResult<MoveCardsResult>`
  - `confirm_card_batch_write(input) -> CardBatchWriteResult`
- 新增前后端 contract / DTO：
  - `BulkDeleteCardsInput`: `workspaceId`, `packId`, `cardIds`, `deleteAssets`
  - `MoveCardsInput`: `workspaceId`, `sourcePackId`, `targetPackId`, `cardIds`, `moveAssets`
  - `BulkDeleteCardsResult`: `deleted_card_ids`, `deleted_asset_count`
  - `MoveCardsResult`: `moved_card_ids`, `moved_asset_count`, `source_pack_revision`, `target_pack_revision`
- 在 `PackWriteService` 增加批量 prepare/commit：
  - 批量删除：校验 workspace、pack 已打开、cardIds 非空且全部存在；一次性更新 `cards.json` 和 pack metadata；按 `deleteAssets` 删除主卡图、场地图、脚本。
  - 批量移动：校验 source/target pack 都已打开、不能相同、cardIds 非空且全部存在；目标 pack 不能已有相同 `CardId`；目标 pack 同 code 作为 warning 进入 confirmation；standard code 冲突保持 error；workspace 其他 custom/推荐范围/min gap 沿用现有 warning 规则。
  - 移动提交必须 all-or-nothing：一次 `execute_plan` 写源/目标 `cards.json`、源/目标 metadata，并 rename 资源到目标 pack。
- confirmation cache 增加批量 card entry：
  - 保存 operation kind、输入快照、源/目标 pack revision/source_stamp、warnings。
  - 确认时重新校验源和目标 pack revision/source_stamp；任一变化则返回 stale error。
  - `invalidate_pack` 要能清理涉及该 pack 的批量确认 token。
- 资源行为：
  - 移动默认 `moveAssets = true`，主卡图、场地图、脚本随 card code 从源 pack 移到目标 pack。
  - 如果目标资源路径已存在，作为 warning 返回 confirmation；确认后仍不覆盖，提交前若目标仍存在则 error，避免误删用户资源。
  - 批量删除 `deleteAssets = true` 时删除对应三类资源；`false` 时保留孤儿资源以兼容当前单卡删除行为。
- Tauri command surface 与 frontend API wrapper 同步新增 command；UI 不直接 invoke raw command。

## Frontend Design

- 在 `CardBrowserPanel` 增加可选选择模式：
  - 列表左侧显示 checkbox；普通模式点击行仍打开编辑抽屉。
  - 支持单选、多选、当前页全选、清除选择。
  - 搜索、排序、翻页后保留已选 id；列表刷新后自动移除当前 pack 中已不存在的选择。
- 在 `CardListPanel` 增加批量工具栏：
  - 显示“已选择 N 张”、移动、删除、清除。
  - 未选择时不显示批量命令。
  - 批量删除使用统一 confirm dialog；确认后调用 `cardApi.bulkDeleteCards`。
  - 批量移动打开目标 pack 选择 modal/dialog；目标列表来自 `openPackIds + packMetadataMap`，过滤当前源 pack，只显示 custom pack。
- 写入结果处理：
  - `ok`：清空选择，刷新当前 pack card list；移动成功时也刷新目标 pack 相关 query/overview。
  - `needs_confirmation`：复用 warning dialog，用户继续后调用 `confirmCardBatchWrite`。
  - error：显示在 dialog 或 notice，不静默吞掉。
- UI 约束：
  - 不支持从 standard pack 批量移动/删除。
  - v1 只允许移动到已打开 custom pack；未打开 pack 不出现在目标列表。
  - 批量删除默认勾选“同时删除卡图/脚本资源”，用户可取消。

## Test Plan

- Rust 集成测试：
  - 批量删除多张卡，确认 `cards.json`、metadata、revision/card_count 更新。
  - 批量删除开启 `deleteAssets` 时删除主卡图、场地图、脚本；关闭时保留资源。
  - 批量删除 unknown card id、空 cardIds、workspace mismatch 返回 error。
  - 批量移动多张卡，源 pack 移除、目标 pack 增加，两个 pack metadata/revision/card_count 更新。
  - 批量移动资源随动：主卡图、场地图、脚本从源 pack 路径消失并出现在目标 pack。
  - 目标 pack 已有相同 `CardId` 返回 error。
  - 目标 pack code 冲突返回 `needs_confirmation`；确认 token stale 时返回 stale error。
  - 目标资源路径已存在时提交不覆盖，返回可理解 error。
- 前端验证：
  - `npm run typecheck`
  - 选择模式、全选、清除、搜索/分页后选择状态行为正确。
  - 批量删除、批量移动、warning confirmation、error 展示流程可手动验证。
- 后端验证：
  - 在 `src-tauri` 下运行 `cargo test`，至少覆盖 `minimal_authoring_flow` 新增用例。

## i18n And Docs

- i18n：
  - 在 `en-US.ts`、`zh-CN.ts`、`ja-JP.ts` 增加批量选择、移动、删除、资源删除选项、warning、成功/失败提示文案。
  - 新增 issue code 文案：批量空选择、目标 pack 相同、目标 pack 未打开、目标 card id 冲突、目标资源已存在、批量确认 stale。
- 文档：
  - 创建 `docs/history/card_batch_operations_plan_2026-06-07.md` 保存本计划。
  - 实现完成后更新核心文档：
    - `functional_spec.md`：Card 增加批量选择、批量移动、批量删除。
    - `system_architecture.md`：说明批量卡片写入由后端负责，移动跨两个 pack session。
    - `code_structure_api.md`：补新增 card API wrapper、contracts、Tauri commands。
    - `ui_design.md`：补 Cards tab 选择模式、批量工具栏、移动目标选择和危险确认。

## Assumptions

- v1 只支持已打开 custom pack 之间移动，不支持移动到未打开 pack、standard pack 或跨 workspace。
- 批量操作 all-or-nothing，不做部分成功。
- 移动保留 `CardId`、`code`、`created_at`；移动时可刷新 `updated_at`。
- 移动默认搬运资源；批量删除默认勾选删除资源，但允许用户取消。
- 目标 code 冲突走 warning confirmation；standard code 冲突仍为 error。
