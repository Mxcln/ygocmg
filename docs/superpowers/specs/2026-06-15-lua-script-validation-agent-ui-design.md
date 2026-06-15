# Lua Script Validation Agent/UI Design

## Goal

Expose the existing Lua script validation backend through two minimal user-facing entry points:

- a read-only AI Agent tool named `validate_lua_script`;
- a card asset bar button that opens an explicit validation report dialog.

Both entry points call the existing `scriptApi.validateLuaScript` wrapper. Neither entry point reads script files directly, rewrites validation rules, edits scripts, or attempts automatic repair.

## Scope

This phase covers only phases 7 and 8 from `docs/history/lua_script_validation_phase_7_8_handoff_2026-06-15.md`.

Included:

- Agent tool registration and prompt guidance.
- Selected-card fallback for the Agent tool.
- Optional `levels` argument validation for the Agent tool.
- A compact "validate" action in `CardAssetBar`.
- A modal dialog that displays the returned `LuaValidationReport`.
- Tests for tool behavior, registry exposure, and report rendering behavior.
- Current-fact documentation updates.

Excluded:

- Lua script editor or workbench.
- Unsaved UI script draft validation.
- Automatic fixes or script mutation.
- Smoke/scenario implementation.
- Tauri sidecar packaging for the helper.
- Semantic correctness claims beyond the backend report.

## Agent Tool

Create `src/features/agent/tools/scriptTools.ts` with a read-only `validateLuaScriptTool`.

The tool accepts:

- `cardId?: string`
- `levels?: LuaValidationLevel[]`

Execution rules:

- Require an active workspace and custom pack through `requirePack(ctx)`.
- If `cardId` is a non-empty string, validate that card.
- If `cardId` is omitted, use `ctx.selectedCardId`.
- If neither is available, throw `ToolError` with a clear message asking the user to specify a card or open one in the editor.
- If `levels` is omitted, do not include it in the API input, allowing the backend default `static + ocgcore_init`.
- If `levels` is provided, every value must be one of `static`, `ocgcore_init`, `smoke`, or `scenario`; otherwise throw `ToolError`.
- Call `scriptApi.validateLuaScript({ workspaceId, packId, cardId, levels? })`.
- Return the `LuaValidationReport` as structured JSON.

Extend `ToolContext` with:

```ts
selectedCardId: string | null;
```

`useAgentLoop.ts` will populate this from `useShellStore.getState().selectedCard?.id ?? null`.

Register the tool in `src/features/agent/tools/registry.ts`. The exported `TOOL_DEFINITIONS` should then include `validate_lua_script` automatically.

Update `src/features/agent/systemPrompt.ts` so the model knows:

- use `validate_lua_script` when the user asks to validate/check/test a card's Lua script;
- prefer the selected card when the user says "this card";
- the tool is read-only and cannot fix scripts;
- `ocgcore_init` only proves load/init, not effect semantics.

## Validation Report Dialog

`CardAssetBar` stays a compact resource-operation surface. It gains a "validate" button in the script button group when:

- the card is already saved;
- `cardId` is present;
- `assetState.has_script` is true.

Clicking the button:

- sets a local validating state;
- calls `scriptApi.validateLuaScript({ workspaceId, packId, cardId })`;
- stores the returned `LuaValidationReport`;
- opens a local report dialog;
- disables resource buttons while validating, following the existing `busy` pattern;
- sends thrown API/program errors through the existing `onError(formatError(err))` path.

Backend business reports such as `inconclusive`, `helper_not_found`, or `script_not_found` are not treated as UI exceptions. They are displayed in the dialog.

Create `src/features/card/ScriptValidationReportDialog.tsx`.

Dialog behavior:

- Renders only when a report is present and the parent says it is open.
- Uses a modal layer/backdrop/dialog pattern consistent with existing app modal styles.
- Uses `role="dialog"`, `aria-modal="true"`, and a labelled title.
- Backdrop click and close button close the dialog.
- The body scrolls when content is long.
- It is read-only and has only a close action.

Dialog content:

- Title from i18n.
- Overall status (`pass`, `warning`, `fail`, `inconclusive`) and confidence.
- Summary.
- Stage rows with stage name, status, duration, and issue count.
- Issue rows with severity, stage, code, message, optional line/column, and optional suggestion.
- Limitations list.
- Clear empty states for no stage issues and no limitations.

Styling:

- Use CSS modules.
- Follow existing modal/dialog tokens (`var(--line)`, `var(--bg-2)`, `var(--panel)`, `var(--text-*)`, status colors).
- Do not introduce a new visual system.
- Ensure long issue messages wrap and do not overflow.

## i18n

Add user-visible messages to the existing locale files:

- action label for validating scripts;
- validating/loading label;
- report dialog title and close label if no shared close/action label exists;
- labels for status, confidence, summary, stages, issues, limitations, line, column, suggestion, no issues, and no limitations.

Do not hard-code Chinese or English user-facing strings in components.

## Testing

Agent tool tests:

- no active pack throws `ToolError`;
- omitted `cardId` with no selected card throws a clear `ToolError`;
- omitted `cardId` with selected card calls `scriptApi.validateLuaScript` using the selected card id;
- explicit `cardId` takes precedence over selected card;
- omitted `levels` does not pass a `levels` property;
- valid explicit `levels` are forwarded;
- invalid `levels` throws `ToolError`;
- registry includes `validate_lua_script`;
- `TOOL_DEFINITIONS` contains the tool definition sent to the model.

UI/report tests:

- clicking validate calls `scriptApi.validateLuaScript` for the current card;
- validating state disables the button;
- returned reports open the dialog;
- `pass`, `warning`, `fail`, and `inconclusive` statuses render;
- issue line/column and suggestion render when present;
- limitations render;
- the close action hides the dialog.

If the current test stack cannot mount React components cleanly without new dependencies, keep the dialog presentation separated enough to test the report rendering helpers or add the smallest needed test dependency in a focused way.

## Documentation Updates

Update current-fact docs after implementation:

- `docs/functional_spec.md`: user-visible script validation via card UI and Agent.
- `docs/system_architecture.md`: Agent/UI are now entry points, not future work.
- `docs/code_structure_api.md`: new agent tool file and report dialog component.
- `docs/ui_design.md`: Card asset bar validation button and report dialog convention.
- `docs/agent.md`: new read-only `validate_lua_script` tool and selected-card behavior.

After docs are updated, search for stale statements:

```powershell
rg "尚未调用 helper|不运行 ocgcore|当前阶段返回静态检查报告|Agent/UI 入口仍属于后续阶段" docs
```
