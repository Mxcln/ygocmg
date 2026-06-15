# Ocgcore Helper Client Integration Design

## Purpose

This spec defines phases 5 and 6 from `docs/history/lua_script_validation_platform_report_2026-06-15.md`: Rust-side helper client integration and `ocgcore_init` validation wiring inside `ScriptValidationService`.

The goal is to make the existing Tauri `validate_lua_script` command run both static validation and ocgcore card load/init validation through the standalone helper built under `tools/script-validator-helper`, while keeping ocgcore isolated outside the Tauri process.

## Scope

This slice includes:

- A Rust infrastructure client that invokes the standalone helper executable as a child process.
- Helper input JSON generation from the resolved card/script context.
- Helper stdout JSON parsing and conversion to `LuaValidationStageResultDto`.
- Timeout, missing executable, non-zero exit, stderr, and invalid JSON handling as structured `inconclusive` stage results.
- `ScriptValidationService` integration for `LuaValidationLevelDto::OcgcoreInit`.
- Default validation levels changed from static-only to `static + ocgcore_init`.
- Report limitations updated so users understand ocgcore init pass only proves load/init, not effect semantics.
- Rust unit tests around helper client behavior and service orchestration.
- Current documentation updates after behavior is verified.

This slice excludes:

- Agent tool registration.
- UI validation buttons or report panels.
- Tauri sidecar packaging/release bundling.
- Automatic helper build from `cargo` or Tauri dev commands.
- Smoke, scenario, puzzle, or semantic validation.
- Automatic Lua script generation or repair.
- Directly linking ocgcore into the Rust/Tauri process.

## Current Repository Context

The repository already contains:

- Static validation contracts and command wiring:
  - `src/shared/contracts/script.ts`
  - `src/shared/api/scriptApi.ts`
  - `src-tauri/src/application/script/*`
  - `src-tauri/src/tauri_commands.rs`
- A standalone helper source tree:
  - `tools/script-validator-helper/CMakeLists.txt`
  - `tools/script-validator-helper/scripts/build.ps1`
  - `tools/script-validator-helper/scripts/run-fixtures.ps1`
  - `tools/script-validator-helper/src/main.cpp`
- Helper fixtures that prove:
  - valid `GetID()` script returns `pass`
  - missing `end` returns `fail` with `lua_syntax_error`
  - missing core scripts return `fail` with `missing_core_script`
- Core docs currently say `ocgcore_init` is not called by `validate_lua_script`.

This slice changes that current fact after tests pass.

## Architecture

The runtime flow becomes:

```text
React / Agent future callers
  -> src/shared/api/scriptApi.validateLuaScript
  -> Tauri validate_lua_script command
  -> ScriptValidationService
      -> ScriptSourceResolver
      -> StaticChecker
      -> OcgcoreInitValidator
          -> OcgcoreValidatorHelperClient
              -> write helper input JSON to temp file
              -> launch tools/script-validator-helper/build/bin/.../script-validator-helper.exe
              -> parse stdout JSON
              -> convert helper failure modes into inconclusive stage result
      -> assemble_report
```

ocgcore remains in a helper process. A helper crash, timeout, or bad output must not crash the Tauri app or fail the Tauri command. It becomes a normal `LuaValidationStageResultDto` with `status: "inconclusive"`.

## File Layout

Create:

```text
src-tauri/src/infrastructure/ocgcore_validator/
  mod.rs
  input.rs
  output.rs
  helper_client.rs

src-tauri/src/application/script/ocgcore_init.rs
```

Modify:

```text
src-tauri/src/infrastructure/mod.rs
src-tauri/src/application/script/mod.rs
src-tauri/src/application/script/service.rs
src-tauri/src/application/script/report.rs
docs/functional_spec.md
docs/system_architecture.md
docs/code_structure_api.md
```

The infrastructure module owns process execution and helper JSON contracts. The application module owns conversion from resolved card/script domain data into validation stages.

## Helper Discovery

The first integration slice uses a development-time helper path, not release sidecar packaging.

Discovery order:

1. If `YGOCMG_SCRIPT_VALIDATOR_HELPER` is set, use that exact executable path.
2. Otherwise, resolve from the Rust crate working directory to:
   - `../tools/script-validator-helper/build/bin/Release/script-validator-helper.exe`
   - `../tools/script-validator-helper/build/bin/script-validator-helper.exe`
   - `tools/script-validator-helper/build/bin/Release/script-validator-helper.exe`
   - `tools/script-validator-helper/build/bin/script-validator-helper.exe`

The first two paths cover tests and commands run from `src-tauri`; the latter two cover commands run from the repo root.

If no executable exists, return an `ocgcore_init` stage:

```text
status: inconclusive
issue code: helper_not_found
```

with a suggestion to run:

```powershell
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/build.ps1
```

## Temp Input Files

The helper accepts `--input <json>`, so the Rust client writes a temporary JSON file and then launches the helper.

Rules:

- Use `std::env::temp_dir()` plus a unique directory name such as `ygocmg-script-validator-{process-id}-{timestamp}`.
- Write `input.json` inside that directory.
- Remove the directory after the helper returns, times out, or errors.
- If cleanup fails, do not fail validation; the result can still be returned.
- The helper input can include the full script text because the temporary file is local to the machine.

No new dependency is required for temporary directories in this slice.

## Helper Input Mapping

Rust sends the helper input contract already implemented by `tools/script-validator-helper`.

Required fields:

```json
{
  "requestId": "workspace:pack:card",
  "level": "ocgcore_init",
  "card": {
    "code": 99999999,
    "alias": 0,
    "setcodes": [],
    "type": 33,
    "level": 4,
    "attribute": 16,
    "race": 1,
    "attack": 0,
    "defense": 0,
    "lscale": 0,
    "rscale": 0,
    "linkMarker": 0,
    "ruleCode": 0
  },
  "scripts": {
    "./script/c99999999.lua": "local s,id,o=GetID()\nfunction s.initial_effect(c)\nend\n"
  },
  "coreScriptRoot": "../../third_party/ygopro-scripts",
  "timeoutMs": 3000
}
```

Card mapping in this slice should use the existing `CardEntity` fields already loaded by `ScriptSourceResolver`. The repository already has YGOPro raw-value encoding logic in `src-tauri/src/infrastructure/ygopro_cdb/mod.rs`; the implementation should extract or expose a small reusable encoder instead of copying a second constant table into the validator.

The helper expects ocgcore/YGOPro numeric values:

- `code`: `CardEntity.code`
- `alias`: card alias or `0`
- `setcodes`: card setcodes as numeric values
- `type`: encoded raw type from the same rules used by CDB export
- `level`: low 8 bits of encoded raw level
- `attribute`: encoded raw attribute
- `race`: encoded raw race
- `attack`: encoded attack
- `defense`: encoded defense for non-Link cards, otherwise `0`
- `lscale`: pendulum left scale if present, otherwise `0`
- `rscale`: pendulum right scale if present, otherwise `0`
- `linkMarker`: encoded Link marker bitmask for Link cards, otherwise `0`
- `ruleCode`: `0`

If the current card model stores these values under different field names, the implementation must follow the current model rather than inventing new fields.

The script map contains exactly:

```text
./script/c{code}.lua -> resolved script text
```

The helper reads core scripts from:

```text
third_party/ygopro-scripts
```

The Rust input should pass an absolute `coreScriptRoot` path so helper invocation is independent of current working directory.

## Helper Output Mapping

The helper output maps directly to `LuaValidationStageResultDto`:

- `stage`: must be `ocgcore_init`; otherwise return `helper_invalid_output`.
- `status`: `pass`, `warning`, `fail`, or `inconclusive`.
- `durationMs`: `duration_ms`.
- `issues`: map severity/stage/code/message/line/column/suggestion field-for-field.
- `log`: copy as stage log.

Unknown issue codes are allowed; they are diagnostic codes from the helper boundary and should pass through unchanged.

Invalid helper stdout JSON returns:

```text
status: inconclusive
issue code: helper_invalid_output
```

Non-zero helper exit returns:

```text
status: inconclusive
issue code: helper_failed
log: stderr/stdout excerpts
```

Timeout returns:

```text
status: inconclusive
issue code: helper_timeout
```

The timeout is 3000 ms for this slice. It should be configurable inside the helper client constructor for tests.

## Service Orchestration

`normalize_levels(None)` and `normalize_levels(Some(vec![]))` should now return:

```text
[Static, OcgcoreInit]
```

`run_requested_levels` should no longer own all stage execution because `ocgcore_init` needs resolved source data and a helper client. The service should orchestrate in this order:

1. Resolve card/script source.
2. For each requested level:
   - `static`: run `StaticChecker`.
   - `ocgcore_init`: run `OcgcoreInitValidator` unless static errors already exist.
   - `smoke` / `scenario`: keep `unsupported_stage_result`.
3. Assemble report.

Static errors should skip ocgcore in this slice. The ocgcore stage should be:

```text
status: inconclusive
issue code: skipped_due_to_static_errors
severity: info
```

Rationale: when static validation already found deterministic syntax/shape errors, running ocgcore usually produces duplicate or lower-signal messages. Future callers can add an explicit "run despite static errors" option if needed.

If the request only asks for `ocgcore_init`, the service should run it even without a static stage.

Missing script and read failure behavior remains unchanged: these return structured inconclusive reports without invoking the helper.

## Report Limitations And Summary

Replace the static-only limitation with stage-aware limitations:

- If static ran:
  - `Static validation does not prove ocgcore load/init success or effect semantics.`
- If ocgcore init ran and passed or failed deterministically:
  - `ocgcore_init only proves the script loads and initial_effect runs; it does not prove effect semantics.`
- If helper is unavailable or inconclusive:
  - Include the helper issue in stage issues; no special limitation is required beyond the summary and issue.

Summaries should no longer always say "Static validation ...". They should say "Lua script validation ..." when more than static is involved:

- pass: `Lua script validation passed for the requested stages.`
- warning: `Lua script validation completed with N warning or informational issue(s).`
- fail: `Lua script validation failed with N issue(s).`
- inconclusive: existing inconclusive summary is acceptable.

## Error Isolation

The helper client must never panic for normal helper failures.

These cases become `LuaValidationStageResultDto`:

- executable not found
- temp directory/file creation failure
- process spawn failure
- timeout
- non-zero exit
- stdout is empty
- stdout is invalid JSON
- output stage/status/severity contains unknown strings

Only programming errors in Rust tests may panic. The Tauri command should return `Ok(LuaValidationReportDto)` for helper boundary problems.

## Testing Strategy

Use TDD for this slice.

Infrastructure helper client tests should use tiny fake helper scripts/executables instead of the real C++ helper, so failure modes are deterministic and fast:

- fake helper writes valid pass JSON to stdout.
- fake helper sleeps longer than timeout.
- fake helper writes invalid JSON.
- fake helper exits with non-zero status and stderr.
- missing helper path.

On Windows, fake helpers can be PowerShell scripts invoked through an executable command wrapper in the test-only client configuration. The production client still invokes the real helper executable directly.

Application/service tests should cover:

- default levels include static and ocgcore init.
- static pass + helper pass -> report includes both stages and final `pass`.
- static error + requested static/ocgcore -> ocgcore is skipped and final `fail`.
- helper invalid output -> final `inconclusive` when static passes.
- explicit `ocgcore_init` only can run without static.
- `smoke` and `scenario` remain unsupported.

End-to-end local verification should include the real helper:

```powershell
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/build.ps1
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/run-fixtures.ps1
Push-Location src-tauri
cargo test application::script
cargo test infrastructure::ocgcore_validator
cargo check
Pop-Location
npm test -- src/shared/api/scriptApi.test.ts
npm run typecheck
```

## Documentation Updates

After implementation and verification:

- `docs/functional_spec.md`: update resource/script validation fact from static-only to static + ocgcore load/init, with limitations.
- `docs/system_architecture.md`: update Lua validation architecture to state `validate_lua_script` calls the helper for `ocgcore_init`.
- `docs/code_structure_api.md`: list the new `src-tauri/src/infrastructure/ocgcore_validator` module.
- Do not update `docs/ui_design.md` for a UI entry because this slice does not add UI.
- Do not update `docs/agent.md` for Agent tools because this slice does not add Agent tools.

## Risks

- **Helper not built:** Development machines may not have run the helper build script. The command must return `helper_not_found` inconclusive with a clear suggestion.
- **Path differences:** Commands may run from repo root or `src-tauri`. Helper discovery must support both.
- **Timeout handling on Windows:** Killing a timed-out child process must be explicit. If child termination fails, return `helper_timeout` and continue.
- **Card field mapping mismatch:** The first mapping should follow current `CardEntity`. If a field is not available, use `0` and keep tests focused on the valid minimal fixture shape.
- **Report status changes:** Defaulting to static + ocgcore means environments without helper become inconclusive instead of static pass. This is intentional for phase 6 because missing ocgcore is now a real unavailable stage, not a hidden non-feature.

## Acceptance Criteria

- `validate_lua_script` defaults to static + ocgcore init.
- Requesting `ocgcore_init` no longer returns `level_not_implemented`.
- A valid saved or draft `GetID()` script can produce an `ocgcore_init` pass stage when the helper is built.
- Helper missing, timeout, non-zero exit, and invalid JSON produce `inconclusive` reports instead of command errors.
- Static errors skip ocgcore with a structured `skipped_due_to_static_errors` stage.
- `smoke` and `scenario` remain unsupported with `level_not_implemented`.
- Existing frontend contract/API tests continue to pass.
- Rust script validation and helper client tests pass.
- Current docs describe the new backend capability without claiming UI or Agent entry points exist.
