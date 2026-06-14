# Lua Script Static Validation MVP Design

## Purpose

This spec defines the first implementation slice of the Lua script validation platform. It turns the historical validation-platform report into a current, testable MVP that establishes the shared backend command, frontend contract/API wrapper, saved-script loading path, draft-script override path, and deterministic static diagnostics.

The goal of this slice is not to prove script semantics. It is to create the stable validation boundary that later ocgcore helper, Agent tool, and UI report work can reuse.

## Scope

This MVP includes:

- A new `validate_lua_script` Tauri command.
- A new frontend contract in `src/shared/contracts/script.ts`.
- A new frontend API wrapper in `src/shared/api/scriptApi.ts`.
- A backend `application/script` module with DTOs, service orchestration, source resolution, static checking, and report assembly.
- Validation of one card in an open custom pack.
- Optional `scriptText` input for validating an unsaved draft without writing it to disk.
- Saved-script resolution from the current pack's `scripts/c{code}.lua` path when `scriptText` is omitted.
- Structured `LuaValidationReport` output with stage results, issues, summary, confidence, and limitations.
- Conservative static checks for obvious, deterministic Lua script problems.
- Rust and TypeScript tests covering the contract wrapper and static-checking behavior where the existing test harness allows it.

This MVP excludes:

- ocgcore sidecar/helper execution.
- Lua runtime/load/init validation.
- Agent tool registration.
- Card UI validation button or report panel.
- Automatic repair or generation.
- Batch validation.
- Scenario, smoke, or puzzle runner support.

## Architecture

The new flow follows the existing YGOCMG frontend/backend boundary:

```text
React/Agent future caller
  -> src/shared/api/scriptApi.ts
  -> invokeApi("validate_lua_script")
  -> tauri_commands::validate_lua_script
  -> presentation::commands::app_commands::validate_lua_script
  -> application::script::ScriptValidationService
  -> SourceResolver + StaticChecker + report assembly
```

The validation platform belongs to the backend application layer. Frontend callers do not read script files, compute card context, or own validation rules.

## Data Contract

Create `src/shared/contracts/script.ts` with these exported types:

```ts
export type LuaValidationLevel = "static" | "ocgcore_init" | "smoke" | "scenario";

export type LuaValidationStatus = "pass" | "warning" | "fail" | "inconclusive";

export type LuaValidationConfidence = "low" | "medium" | "high";

export type LuaValidationStage = LuaValidationLevel;

export interface ValidateLuaScriptInput {
  workspaceId: string;
  packId: string;
  cardId: string;
  scriptText?: string;
  levels?: LuaValidationLevel[];
}

export interface LuaValidationIssue {
  severity: "error" | "warning" | "info";
  stage: LuaValidationStage;
  code: string;
  message: string;
  line?: number;
  column?: number;
  suggestion?: string;
}

export interface LuaValidationStageResult {
  stage: LuaValidationStage;
  status: LuaValidationStatus;
  durationMs: number;
  issues: LuaValidationIssue[];
  log?: string[];
}

export interface LuaValidationReport {
  status: LuaValidationStatus;
  confidence: LuaValidationConfidence;
  summary: string;
  issues: LuaValidationIssue[];
  stages: LuaValidationStageResult[];
  limitations: string[];
}
```

Rules:

- `workspaceId`, `packId`, and `cardId` are required.
- `scriptText` is optional. When present, the backend validates this text and does not write it to disk.
- `levels` is optional. For this MVP, omitted levels default to `["static"]`.
- Unsupported requested levels are represented as `inconclusive` stage results with clear `level_not_implemented` issues instead of silently pretending they ran.
- TypeScript uses camelCase fields; Rust DTOs use `serde(rename_all = "camelCase")`.

## Backend Modules

Create:

```text
src-tauri/src/application/script/
  mod.rs
  dto.rs
  service.rs
  source_resolver.rs
  static_checker.rs
  report.rs
```

Update:

```text
src-tauri/src/application/mod.rs
src-tauri/src/presentation/commands/app_commands.rs
src-tauri/src/tauri_commands.rs
src-tauri/src/main.rs
```

Responsibilities:

- `dto.rs`: serializable Rust input/report DTOs matching `src/shared/contracts/script.ts`.
- `service.rs`: validates workspace/pack/card, resolves source text, runs requested stages, and returns a report.
- `source_resolver.rs`: loads card context and script text from either `scriptText` or `scripts/c{code}.lua`.
- `static_checker.rs`: implements conservative deterministic checks over script text.
- `report.rs`: merges stage results, computes final status/confidence, and produces summary/limitations.

## Source Resolution

`ScriptSourceResolver` uses existing pack/session boundaries:

1. Ensure the input workspace matches the current workspace.
2. Require the pack to be an open custom pack.
3. Find the card in the pack snapshot by `cardId`.
4. If `scriptText` is present, use it as the validation source and mark the source kind as draft.
5. If `scriptText` is absent, read the saved script path from `domain::resource::path_rules::script_path(&snapshot.pack_path, card.code)`.
6. If the script is absent, return a valid report with an issue code `script_not_found`.
7. If the script cannot be read, return an `inconclusive` report with issue code `script_read_failed`.

The resolver does not mutate pack data or resource files.

## Static Checks

The static checker is intentionally conservative. It should emit `error` only for findings that are highly likely to be real problems, and use `warning` for suspicious patterns.

MVP checks:

- `missing_initial_effect`: no `initial_effect` function definition is found.
- `mixed_script_style`: script appears to mix `local s,id,o=GetID()` style with legacy `c{code}.xxx` style.
- `undefined_effect_callback`: `SetCondition`, `SetCost`, `SetTarget`, or `SetOperation` references `s.foo` or `c{code}.foo` but no matching function definition exists.
- `undefined_script_function_reference`: direct `s.foo` or `c{code}.foo` callback references appear without definitions, limited to callback-like contexts to reduce false positives.
- `target_missing_chk_branch`: a function referenced by `SetTarget` does not contain a recognizable `chk==0` branch.
- `dangerous_lua_api`: use of `os.execute`, `io.popen`, `package.loadlib`, `loadfile`, `dofile`, or `require` produces a warning.
- `common_api_typo`: obvious typo table for a small set of high-confidence mistakes such as `Duel.SpecialSummom`, `Duel.SelectMathchingCard`, `Effect.CreateEffct`, and `Card.IsRelateToEffct`.

Line numbers should be reported when the scanner can identify them cheaply. The checker should avoid full Lua parsing in this slice.

## Report Semantics

Final status:

- `fail`: any requested stage has an error issue or a failed stage.
- `warning`: no error exists, but one or more warning issues exist.
- `pass`: requested implemented stages ran and produced no warning or error.
- `inconclusive`: validation could not run meaningfully because the script is missing, unreadable, or only unsupported levels were requested.

Confidence:

- `high`: static stage ran and status is `pass` or `fail` based on deterministic static findings.
- `medium`: static stage ran with warnings only.
- `low`: result is `inconclusive` or only unsupported levels were requested.

Limitations always include a message that static validation does not prove ocgcore load/init success or effect semantics. If unsupported levels were requested, include a limitation naming the unimplemented levels.

## Error Handling

The command returns a normal `LuaValidationReport` for expected validation outcomes:

- Missing script.
- Empty script.
- Static errors.
- Unsupported validation levels.
- Script read failure where the path is known.

The command returns `AppError` only for boundary failures that match existing application behavior:

- Workspace mismatch.
- Pack is not open.
- Card is not found.
- Pack type is not eligible for custom script validation.
- Session lock or storage access failures outside the validation-result domain.

## Testing

Rust unit tests should cover `static_checker` and `report` behavior:

- A correct `GetID()` script with `initial_effect` and a defined operation returns no error.
- Missing `initial_effect` returns `missing_initial_effect`.
- A callback reference such as `e1:SetOperation(s.thop)` without `function s.thop` returns `undefined_effect_callback`.
- A target callback without `chk==0` returns `target_missing_chk_branch` warning.
- Dangerous Lua API usage returns `dangerous_lua_api` warning.
- Unsupported requested level returns `level_not_implemented` and an `inconclusive` stage.

Frontend tests should cover `scriptApi.validateLuaScript` if the existing API wrapper test pattern supports invoke mocking. Otherwise, TypeScript coverage is through `npm run typecheck`.

Manual verification commands:

```powershell
npm run typecheck
cd src-tauri
cargo test
cargo check
```

## Documentation

After implementation is complete, update current authoritative docs:

- `docs/functional_spec.md`: describe saved/draft Lua static validation as implemented.
- `docs/system_architecture.md`: describe the backend script validation service boundary.
- `docs/code_structure_api.md`: list `scriptApi`, `script.ts`, and `validate_lua_script`.

Do not update docs to claim ocgcore helper, Agent tool, or UI validation panel until those slices are implemented.
