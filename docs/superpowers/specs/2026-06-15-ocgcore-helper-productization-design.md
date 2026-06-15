# Ocgcore Helper Productization Design

## Purpose

This spec defines the next Lua script validation slice after the static validation MVP. It turns the historical ocgcore spike in `docs/history/lua_script_validation_platform_report_2026-06-15.md` into a current, buildable helper/sidecar direction.

The goal is to productize the validated ocgcore load/init path without adding Agent tools, UI entry points, script generation, automatic repair, smoke tests, or scenario runners in this slice.

## Scope

This slice includes:

- A standalone script validator helper executable source tree.
- Helper input and output JSON contracts documented for Rust/backend integration.
- A deterministic ocgcore load/init routine based on `new_card`, not bare `preload_script` for card scripts.
- A script reader that can serve core YGOPro scripts and the tested `c{code}.lua` script.
- A card reader backed by helper input card data.
- A message/log capture path that can report Lua syntax/runtime errors as diagnostics.
- Local build/bootstrap scripts for Windows development using the existing `third_party/ocgcore`, `third_party/ygopro-scripts`, and pinned Lua cache.
- Standalone helper fixtures and tests that can run without Tauri UI.

This slice excludes:

- Tauri command behavior changes.
- Rust helper client integration into `ScriptValidationService`.
- Agent tool registration.
- Card UI validation button or report panel.
- Batch validation.
- Smoke/scenario/puzzle validation.
- Automatic script repair or generation.
- Full cross-platform release packaging.

## Current Repository Context

The repository already contains:

- `third_party/ocgcore` submodule at commit `19026e316c6de1773886a6b742081b5130c1a413`.
- `third_party/ygopro-scripts` submodule at commit `9c1dceb9890d3f526b3b19859454260b459b457f`.
- `third_party/cache/lua-5.4.7.tar.gz`, ignored by git.
- `third_party/cache/lua-5.4.7.sha256` with checksum `9fbf5e28ef86c69858f6d3d34eccc32e911c1a28b4120ff3e84aaa70cfbf1e30`.
- Static validation command infrastructure in `src-tauri/src/application/script`.

The helper should depend on those existing third-party locations instead of adding new vendored Lua source or duplicating ocgcore.

## Architecture

The helper is an independent executable, not code loaded into the Tauri process.

```text
future ScriptValidationService ocgcore stage
  -> helper client writes input JSON / launches helper with timeout
  -> helper executable
      -> loads input JSON
      -> registers ocgcore script_reader/card_reader/message_handler
      -> create_duel + set_player_info + new_card
      -> captures log messages
      -> emits output JSON
  -> helper client converts output to LuaValidationStageResultDto
```

The first productization slice stops at the helper boundary. It must prove that the executable can be built and exercised directly. A later slice will add the Rust helper client and connect `LuaValidationLevelDto::OcgcoreInit` to the helper output.

## File Layout

Create:

```text
tools/script-validator-helper/
  README.md
  CMakeLists.txt
  src/main.cpp
  src/json.hpp
  fixtures/valid_getid.input.json
  fixtures/missing_end.input.json
  fixtures/missing_helper.input.json
  scripts/bootstrap.ps1
  scripts/build.ps1
  scripts/run-fixtures.ps1
```

Rationale:

- `tools/` keeps sidecar source outside Tauri application code until packaging integration is added.
- `src/json.hpp` is a small local JSON parser/writer tailored to the helper contract, avoiding an additional third-party dependency in this slice.
- PowerShell scripts target the current Windows development environment first, matching the spike.

## Helper Invocation

The helper should support:

```text
script-validator-helper --input <path-to-input.json>
```

It writes exactly one JSON object to stdout. Diagnostic logs may go to stderr, but fixture tests should pass with empty stderr for expected validation failures because those failures are represented in stdout JSON.

Exit code behavior:

- `0`: helper ran and emitted valid output JSON, even if validation status is `fail` or `inconclusive`.
- non-zero: helper could not parse input, could not initialize dependencies, or crashed before producing output.

## Input Contract

Helper input JSON:

```json
{
  "requestId": "fixture-valid",
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

Rules:

- `level` must be `ocgcore_init` in this slice.
- `card.code` is required and is the card passed to `new_card`.
- `scripts` maps ocgcore script names to exact content. It must include `./script/c{code}.lua` for the tested card unless the test intentionally validates missing script behavior.
- `coreScriptRoot` points at `third_party/ygopro-scripts` or an equivalent directory containing at least `constant.lua`, `utility.lua`, and `procedure.lua`.
- Helper input uses raw ocgcore/YGOPro numeric fields. Rust-side conversion from `CardEntity` is a later integration slice.
- `timeoutMs` is informational in the helper productization slice. The future Rust helper client owns process timeout enforcement.

## Output Contract

Helper output JSON:

```json
{
  "requestId": "fixture-valid",
  "stage": "ocgcore_init",
  "status": "pass",
  "durationMs": 12,
  "issues": [],
  "log": [],
  "helperVersion": "0.1.0"
}
```

For a Lua syntax/runtime error:

```json
{
  "requestId": "fixture-missing-end",
  "stage": "ocgcore_init",
  "status": "fail",
  "durationMs": 12,
  "issues": [
    {
      "severity": "error",
      "stage": "ocgcore_init",
      "code": "lua_syntax_error",
      "message": "[string \"./script/c99999999.lua\"]:7: 'end' expected ...",
      "line": 7,
      "column": null,
      "suggestion": "Fix the Lua error reported by ocgcore and validate again."
    }
  ],
  "log": ["..."],
  "helperVersion": "0.1.0"
}
```

Issue mapping:

- `lua_syntax_error`: message matches a Lua parser error and contains a script line.
- `lua_runtime_error`: message comes from an `initial_effect` or related runtime call.
- `missing_core_script`: `constant.lua`, `utility.lua`, or `procedure.lua` cannot be read.
- `missing_card_script`: `./script/c{code}.lua` is not supplied or cannot be read.
- `missing_card_data`: card reader receives an unknown code.
- `ocgcore_message`: fallback for messages that do not fit the above categories.
- `helper_error`: helper setup error that still permits JSON output.

Status mapping:

- `pass`: `new_card` completed, the card is present in the field, and no error issues were captured.
- `fail`: Lua syntax/runtime or missing required validation data makes load/init fail deterministically.
- `inconclusive`: helper cannot make a reliable determination while still emitting JSON.

## Ocgcore Load/Init Flow

The helper validates card scripts via:

```text
set_script_reader
set_card_reader
set_message_handler
create_duel(seed)
set_player_info(pduel, 0, 8000, 0, 1)
set_player_info(pduel, 1, 8000, 0, 1)
new_card(pduel, card.code, 0, 0, LOCATION_MZONE, 0, POS_FACEUP_ATTACK)
query_field_count(pduel, 0, LOCATION_MZONE)
get_log_message loop/flush
end_duel(pduel)
```

Card scripts must not be validated with bare `preload_script("./script/c{id}.lua")` because `GetID()` depends on ocgcore card context set by `load_card_script`.

## Script Reader

The helper script reader resolves:

1. Exact names in `input.scripts`, such as `./script/c99999999.lua`.
2. Core script names from `coreScriptRoot`:
   - `./script/constant.lua`
   - `./script/utility.lua`
   - `./script/procedure.lua`
3. Future helper/package names are out of scope unless present in `input.scripts`.

The callback must keep script buffers alive long enough for ocgcore to consume them. The simplest first implementation stores script content in a process-wide map and returns `byte*` to stable `std::string` data with the length set.

## Card Reader

The helper card reader supports the single tested card in this slice.

Rules:

- If ocgcore requests `input.card.code`, fill `card_data` from input.
- If ocgcore requests another code, clear the struct, record `missing_card_data`, and return `0`.
- `setcodes` should be copied into the 16-slot `card_data::setcode` array.

Auxiliary card data for scenarios is out of scope for this slice.

## Build Strategy

Windows development build:

1. Verify `third_party/cache/lua-5.4.7.tar.gz` exists.
2. Verify SHA-256 equals `9fbf5e28ef86c69858f6d3d34eccc32e911c1a28b4120ff3e84aaa70cfbf1e30`.
3. Extract Lua into a generated build directory under `tools/script-validator-helper/build/deps/lua-5.4.7`.
4. Build helper with CMake or MSBuild-compatible tooling using:
   - `third_party/ocgcore`
   - extracted Lua source
   - helper source
5. Emit helper binary under `tools/script-validator-helper/build/bin/`.

Generated build output must remain untracked. Add ignore rules for:

```text
tools/script-validator-helper/build/
```

The first implementation may build only on Windows if Linux/macOS are documented as future packaging work.

## Testing Strategy

Standalone fixture tests:

- `valid_getid.input.json`: returns `pass`, no issues.
- `missing_end.input.json`: returns `fail`, includes a line-numbered Lua syntax issue.
- `missing_helper.input.json`: returns `fail` or `inconclusive` with `missing_core_script` or related issue.

Script checks:

- `scripts/bootstrap.ps1` verifies Lua cache checksum and prepares dependency build files.
- `scripts/build.ps1` builds helper.
- `scripts/run-fixtures.ps1` runs helper against fixtures and asserts status/issue codes.

Tauri/Rust integration tests are out of scope until the helper client slice.

## Documentation Updates

After helper productization is implemented:

- `docs/system_architecture.md`: describe helper/sidecar boundary as available but not yet wired to the Tauri validation command unless the Rust client is also implemented.
- `docs/code_structure_api.md`: list helper source/build location.
- Do not update `docs/functional_spec.md` to claim UI or Agent access.
- Do not claim `ocgcore_init` is available through `validate_lua_script` until `ScriptValidationService` calls the helper client.

## Risks

- **Build tool availability:** Windows helper build may require CMake/MSBuild/Visual Studio tooling. Scripts must fail with clear messages.
- **ocgcore crash:** Product code must keep ocgcore outside the Tauri process. This slice maintains that boundary.
- **JSON parser limits:** A small local parser is acceptable only for the fixed helper contract. If it becomes complex, replace it with an explicit third-party dependency in a later design.
- **Card data mismatch:** This slice accepts raw ocgcore fields. Mapping from YGOCMG `CardEntity` remains a later Rust integration risk.
- **Core script drift:** `ygopro-scripts` is a submodule and should stay pinned by the parent repo.

## Acceptance Criteria

- Helper source tree exists and documents how to build/run it.
- Bootstrap verifies pinned Lua cache checksum.
- Helper builds in the current Windows development environment or fails with a precise missing-tool message.
- Running the valid fixture returns JSON with `stage: "ocgcore_init"` and `status: "pass"`.
- Running the missing-`end` fixture returns JSON with `status: "fail"` and an issue containing a Lua line number.
- Build output is ignored by git.
- No Agent/UI entry is added in this slice.
- Existing static validation tests and `cargo check` continue to pass.
