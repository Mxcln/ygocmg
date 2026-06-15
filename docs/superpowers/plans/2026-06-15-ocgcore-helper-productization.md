# Ocgcore Helper Productization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Productize the historical ocgcore load/init spike into a standalone helper executable with pinned Lua bootstrap, fixtures, build scripts, and direct fixture verification.

**Architecture:** The helper lives under `tools/script-validator-helper` and is invoked independently from Tauri in this slice. It reads a fixed JSON input, registers ocgcore callbacks, validates card load/init through `new_card`, and emits a structured JSON result that a later Rust helper client can convert into `LuaValidationStageResultDto`.

**Tech Stack:** C++17, CMake, PowerShell scripts, `third_party/ocgcore`, `third_party/ygopro-scripts`, pinned Lua 5.4.7 cache, existing Rust/TypeScript validation checks.

---

## File Structure

- Create: `tools/script-validator-helper/README.md`  
  Documents purpose, build commands, fixture commands, and current non-integration boundary.
- Create: `tools/script-validator-helper/CMakeLists.txt`  
  Builds the helper executable from local source, ocgcore sources, and extracted Lua sources.
- Create: `tools/script-validator-helper/src/json.hpp`  
  Small fixed-contract JSON parser/writer used by the helper.
- Create: `tools/script-validator-helper/src/main.cpp`  
  Helper CLI, JSON contract handling, ocgcore callbacks, validation flow, and JSON output.
- Create: `tools/script-validator-helper/fixtures/valid_getid.input.json`  
  Valid `GetID()` script fixture.
- Create: `tools/script-validator-helper/fixtures/missing_end.input.json`  
  Invalid Lua syntax fixture.
- Create: `tools/script-validator-helper/fixtures/missing_core.input.json`  
  Missing core script root fixture.
- Create: `tools/script-validator-helper/scripts/bootstrap.ps1`  
  Verifies Lua cache checksum and extracts Lua into helper build deps.
- Create: `tools/script-validator-helper/scripts/build.ps1`  
  Runs bootstrap, configures CMake, and builds the helper.
- Create: `tools/script-validator-helper/scripts/run-fixtures.ps1`  
  Runs helper against fixtures and asserts output status/issue shape.
- Modify: `.gitignore`  
  Ignores `tools/script-validator-helper/build/`.
- Modify later: `docs/system_architecture.md`, `docs/code_structure_api.md`  
  Documents helper source/build boundary only after fixture verification passes.

---

### Task 1: Helper Scaffolding And Lua Bootstrap

**Files:**
- Create: `tools/script-validator-helper/README.md`
- Create: `tools/script-validator-helper/scripts/bootstrap.ps1`
- Create: `tools/script-validator-helper/fixtures/valid_getid.input.json`
- Create: `tools/script-validator-helper/fixtures/missing_end.input.json`
- Create: `tools/script-validator-helper/fixtures/missing_core.input.json`
- Modify: `.gitignore`

- [ ] **Step 1: Write the failing bootstrap verification command**

Run before creating files:

```powershell
Test-Path tools/script-validator-helper/scripts/bootstrap.ps1
```

Expected: `False`.

- [ ] **Step 2: Add `.gitignore` entry**

Add this line near the existing `third_party/cache/` ignore:

```gitignore
tools/script-validator-helper/build/
```

- [ ] **Step 3: Create helper README**

Create `tools/script-validator-helper/README.md`:

```markdown
# Script Validator Helper

This helper productizes the ocgcore load/init validation spike. It is a standalone executable used by future Rust backend integration; this slice does not add UI, Agent tools, or Tauri command behavior.

The helper validates card scripts through ocgcore's card load path:

```text
create_duel -> set_player_info -> new_card -> load_card_script -> initial_effect
```

Do not use bare `preload_script("./script/c{id}.lua")` for card script validation. `GetID()` depends on card context set by ocgcore during `load_card_script`.

## Dependencies

- `third_party/ocgcore`
- `third_party/ygopro-scripts`
- `third_party/cache/lua-5.4.7.tar.gz`
- `third_party/cache/lua-5.4.7.sha256`
- CMake
- Visual Studio 2022 C++ build tools on Windows

## Bootstrap

```powershell
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/bootstrap.ps1
```

The bootstrap script verifies the pinned Lua archive checksum and extracts it into `tools/script-validator-helper/build/deps/lua-5.4.7`.

## Build

```powershell
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/build.ps1
```

## Run Fixtures

```powershell
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/run-fixtures.ps1
```

## Contract

The helper accepts:

```text
script-validator-helper --input <input.json>
```

It prints exactly one JSON object to stdout. Validation failures such as Lua syntax errors are represented in stdout JSON with exit code `0`; process/setup failures may return a non-zero exit code.
```

- [ ] **Step 4: Create bootstrap script**

Create `tools/script-validator-helper/scripts/bootstrap.ps1`:

```powershell
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$helperRoot = Resolve-Path (Join-Path $scriptDir "..")
$repoRoot = Resolve-Path (Join-Path $helperRoot "../..")
$cacheDir = Join-Path $repoRoot "third_party/cache"
$archive = Join-Path $cacheDir "lua-5.4.7.tar.gz"
$checksumFile = Join-Path $cacheDir "lua-5.4.7.sha256"
$depsDir = Join-Path $helperRoot "build/deps"
$extractRoot = Join-Path $depsDir "lua-5.4.7"

if (-not (Test-Path $archive)) {
  throw "Missing Lua archive: $archive"
}
if (-not (Test-Path $checksumFile)) {
  throw "Missing Lua checksum file: $checksumFile"
}

$expected = (Get-Content $checksumFile -Raw).Trim().Split(" ")[0].ToLowerInvariant()
$actual = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant()
if ($actual -ne $expected) {
  throw "Lua archive checksum mismatch. Expected $expected but got $actual."
}

New-Item -ItemType Directory -Force -Path $depsDir | Out-Null
if (Test-Path $extractRoot) {
  Remove-Item -Recurse -Force $extractRoot
}

tar -xzf $archive -C $depsDir
$expanded = Join-Path $depsDir "lua-5.4.7"
if (-not (Test-Path (Join-Path $expanded "src/lua.h"))) {
  throw "Lua extraction did not produce expected src/lua.h under $expanded"
}

Write-Output "Lua 5.4.7 ready at $expanded"
```

- [ ] **Step 5: Create valid fixture**

Create `tools/script-validator-helper/fixtures/valid_getid.input.json`:

```json
{
  "requestId": "fixture-valid-getid",
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
    "./script/c99999999.lua": "local s,id,o=GetID()\nfunction s.initial_effect(c)\n  local e1=Effect.CreateEffect(c)\n  e1:SetType(EFFECT_TYPE_SINGLE)\n  e1:SetCode(EFFECT_CANNOT_ATTACK)\n  c:RegisterEffect(e1)\nend\n"
  },
  "coreScriptRoot": "../../third_party/ygopro-scripts",
  "timeoutMs": 3000
}
```

- [ ] **Step 6: Create missing-end fixture**

Create `tools/script-validator-helper/fixtures/missing_end.input.json`:

```json
{
  "requestId": "fixture-missing-end",
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
    "./script/c99999999.lua": "local s,id,o=GetID()\nfunction s.initial_effect(c)\n  local e1=Effect.CreateEffect(c)\n  e1:SetType(EFFECT_TYPE_SINGLE)\n  e1:SetCode(EFFECT_CANNOT_ATTACK)\n  c:RegisterEffect(e1)\n-- missing end on purpose\n"
  },
  "coreScriptRoot": "../../third_party/ygopro-scripts",
  "timeoutMs": 3000
}
```

- [ ] **Step 7: Create missing-core fixture**

Create `tools/script-validator-helper/fixtures/missing_core.input.json`:

```json
{
  "requestId": "fixture-missing-core",
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
  "coreScriptRoot": "./does-not-exist",
  "timeoutMs": 3000
}
```

- [ ] **Step 8: Run bootstrap**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/bootstrap.ps1
```

Expected: output contains `Lua 5.4.7 ready`.

- [ ] **Step 9: Verify ignored build output**

Run:

```powershell
git status --short -- tools/script-validator-helper/build
```

Expected: no output.

- [ ] **Step 10: Commit scaffold**

Run:

```powershell
git add .gitignore tools/script-validator-helper/README.md tools/script-validator-helper/scripts/bootstrap.ps1 tools/script-validator-helper/fixtures/valid_getid.input.json tools/script-validator-helper/fixtures/missing_end.input.json tools/script-validator-helper/fixtures/missing_core.input.json
git commit -m "feat: scaffold ocgcore helper bootstrap"
```

---

### Task 2: Helper JSON Contract Parser

**Files:**
- Create: `tools/script-validator-helper/src/json.hpp`
- Create: `tools/script-validator-helper/src/main.cpp`
- Create: `tools/script-validator-helper/CMakeLists.txt`
- Create: `tools/script-validator-helper/scripts/build.ps1`

- [ ] **Step 1: Write minimal helper CLI without ocgcore**

Create `tools/script-validator-helper/src/json.hpp` with a fixed-contract parser that supports JSON objects, arrays, strings with escapes, numbers, booleans, and null. It must expose:

```cpp
namespace ygo_helper_json {
struct Value;
Value parse(const std::string& text);
std::string stringify(const Value& value);
}
```

Create `tools/script-validator-helper/src/main.cpp` with `--input <path>` parsing and input JSON loading. For this task only, return a JSON object:

```json
{
  "requestId": "<input requestId>",
  "stage": "ocgcore_init",
  "status": "inconclusive",
  "durationMs": 0,
  "issues": [
    {
      "severity": "info",
      "stage": "ocgcore_init",
      "code": "helper_not_linked",
      "message": "The helper CLI parsed input but ocgcore is not linked in this task.",
      "line": null,
      "column": null,
      "suggestion": "Build the ocgcore integration task."
    }
  ],
  "log": [],
  "helperVersion": "0.1.0"
}
```

- [ ] **Step 2: Add CMake skeleton**

Create `tools/script-validator-helper/CMakeLists.txt`:

```cmake
cmake_minimum_required(VERSION 3.20)
project(script_validator_helper LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_executable(script-validator-helper
  src/main.cpp
)

target_include_directories(script-validator-helper PRIVATE
  ${CMAKE_CURRENT_SOURCE_DIR}/src
)

if (MSVC)
  target_compile_options(script-validator-helper PRIVATE /W4 /permissive- /utf-8)
else()
  target_compile_options(script-validator-helper PRIVATE -Wall -Wextra -Wpedantic)
endif()
```

- [ ] **Step 3: Add build script**

Create `tools/script-validator-helper/scripts/build.ps1`:

```powershell
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$helperRoot = Resolve-Path (Join-Path $scriptDir "..")

& (Join-Path $scriptDir "bootstrap.ps1")

$buildDir = Join-Path $helperRoot "build/cmake"
$binDir = Join-Path $helperRoot "build/bin"
New-Item -ItemType Directory -Force -Path $binDir | Out-Null

$generatorArgs = @()
$vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio/Installer/vswhere.exe"
if (Test-Path $vswhere) {
  $vsPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
  if ($vsPath) {
    $generatorArgs = @("-G", "Visual Studio 17 2022", "-A", "x64")
  }
}

cmake -S $helperRoot -B $buildDir @generatorArgs -DCMAKE_RUNTIME_OUTPUT_DIRECTORY=$binDir
cmake --build $buildDir --config Release

$candidate = Join-Path $binDir "Release/script-validator-helper.exe"
if (-not (Test-Path $candidate)) {
  $candidate = Join-Path $binDir "script-validator-helper.exe"
}
if (-not (Test-Path $candidate)) {
  throw "Helper executable was not produced under $binDir"
}

Write-Output "Helper built at $candidate"
```

- [ ] **Step 4: Build helper CLI**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/build.ps1
```

Expected: output contains `Helper built at`.

- [ ] **Step 5: Run valid fixture through CLI parser**

Run:

```powershell
$helper = "tools/script-validator-helper/build/bin/Release/script-validator-helper.exe"
if (-not (Test-Path $helper)) { $helper = "tools/script-validator-helper/build/bin/script-validator-helper.exe" }
& $helper --input tools/script-validator-helper/fixtures/valid_getid.input.json | ConvertFrom-Json | Select-Object requestId,stage,status
```

Expected: `requestId` is `fixture-valid-getid`, `stage` is `ocgcore_init`, `status` is `inconclusive`.

- [ ] **Step 6: Commit parser/build skeleton**

Run:

```powershell
git add tools/script-validator-helper/CMakeLists.txt tools/script-validator-helper/src/json.hpp tools/script-validator-helper/src/main.cpp tools/script-validator-helper/scripts/build.ps1
git commit -m "feat: add script validator helper cli skeleton"
```

---

### Task 3: Ocgcore Build Integration

**Files:**
- Modify: `tools/script-validator-helper/CMakeLists.txt`
- Modify: `tools/script-validator-helper/src/main.cpp`
- Modify: `tools/script-validator-helper/scripts/build.ps1`

- [ ] **Step 1: Write failing fixture runner**

Create `tools/script-validator-helper/scripts/run-fixtures.ps1`:

```powershell
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$helperRoot = Resolve-Path (Join-Path $scriptDir "..")
$helper = Join-Path $helperRoot "build/bin/Release/script-validator-helper.exe"
if (-not (Test-Path $helper)) {
  $helper = Join-Path $helperRoot "build/bin/script-validator-helper.exe"
}
if (-not (Test-Path $helper)) {
  throw "Helper executable not found. Run scripts/build.ps1 first."
}

function Invoke-Fixture($name, $expectedStatus, $expectedIssueCode) {
  $path = Join-Path $helperRoot "fixtures/$name"
  $raw = & $helper --input $path
  if ($LASTEXITCODE -ne 0) {
    throw "Helper exited with $LASTEXITCODE for $name"
  }
  $json = $raw | ConvertFrom-Json
  if ($json.stage -ne "ocgcore_init") {
    throw "$name expected stage ocgcore_init but got $($json.stage)"
  }
  if ($json.status -ne $expectedStatus) {
    throw "$name expected status $expectedStatus but got $($json.status). Raw: $raw"
  }
  if ($expectedIssueCode) {
    $codes = @($json.issues | ForEach-Object { $_.code })
    if ($codes -notcontains $expectedIssueCode) {
      throw "$name expected issue code $expectedIssueCode but got [$($codes -join ', ')]"
    }
  }
  Write-Output "$name => $($json.status)"
}

Invoke-Fixture "valid_getid.input.json" "pass" $null
Invoke-Fixture "missing_end.input.json" "fail" "lua_syntax_error"
Invoke-Fixture "missing_core.input.json" "fail" "missing_core_script"
```

Run it before ocgcore integration:

```powershell
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/run-fixtures.ps1
```

Expected: FAIL because skeleton returns `inconclusive`.

- [ ] **Step 2: Update CMake to link Lua and ocgcore**

Modify `tools/script-validator-helper/CMakeLists.txt` to:

- Require `OCGCORE_DIR`.
- Require `LUA_SRC_DIR`.
- Build Lua static library from `src/*.c` except `lua.c`, `luac.c`, `linit.c`, `onelua.c`.
- Build ocgcore static library from `${OCGCORE_DIR}/*.cpp`.
- Link helper to ocgcore and Lua.
- Include helper `src`, ocgcore dir, and Lua `src`.

- [ ] **Step 3: Update build script to pass dependencies**

Modify `tools/script-validator-helper/scripts/build.ps1` to pass:

```powershell
-DOCGCORE_DIR=<repo>/third_party/ocgcore
-DLUA_SRC_DIR=<helper>/build/deps/lua-5.4.7
```

- [ ] **Step 4: Implement ocgcore validation**

Modify `tools/script-validator-helper/src/main.cpp` to:

- Parse input into request/card/scripts/core root.
- Register `set_script_reader`, `set_card_reader`, `set_message_handler`.
- Load core scripts from `coreScriptRoot`.
- Serve `./script/c{code}.lua` from the `scripts` map.
- Fill ocgcore `card_data`.
- Run `create_duel`, `set_player_info`, `new_card`, `query_field_count`, `end_duel`.
- Capture messages via `get_log_message` in `message_handler`.
- Convert Lua syntax messages with `:line:` into `lua_syntax_error`.
- Emit JSON status `pass` when field count is `1` and no error issues exist.

- [ ] **Step 5: Build and run fixtures**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/build.ps1
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/run-fixtures.ps1
```

Expected:

```text
valid_getid.input.json => pass
missing_end.input.json => fail
missing_core.input.json => fail
```

- [ ] **Step 6: Commit ocgcore integration**

Run:

```powershell
git add tools/script-validator-helper/CMakeLists.txt tools/script-validator-helper/src/main.cpp tools/script-validator-helper/scripts/build.ps1 tools/script-validator-helper/scripts/run-fixtures.ps1
git commit -m "feat: validate lua scripts with ocgcore helper"
```

---

### Task 4: Helper Documentation And Current Docs

**Files:**
- Modify: `tools/script-validator-helper/README.md`
- Modify: `docs/system_architecture.md`
- Modify: `docs/code_structure_api.md`

- [ ] **Step 1: Update helper README with verified fixture output**

Add a section:

```markdown
## Verified Fixture Behavior

- `valid_getid.input.json` returns `pass`.
- `missing_end.input.json` returns `fail` with `lua_syntax_error`.
- `missing_core.input.json` returns `fail` with `missing_core_script`.

The helper is not yet wired into `ScriptValidationService`; `validate_lua_script` still reports unsupported `ocgcore_init` until the Rust helper client slice lands.
```

- [ ] **Step 2: Update architecture docs**

In `docs/system_architecture.md`, under `## Lua 脚本验证架构`, append:

```markdown
`tools/script-validator-helper` contains the standalone ocgcore load/init helper source and fixture runner. The helper validates card script initialization through `new_card -> load_card_script -> initial_effect` in a separate process boundary. 当前 Tauri `validate_lua_script` command 尚未调用 helper；`ocgcore_init` 仍在 application 层报告为未实现阶段，直到 Rust helper client 集成完成。
```

- [ ] **Step 3: Update code structure docs**

In `docs/code_structure_api.md`, under backend directory list, add:

```markdown
- `tools/script-validator-helper`：独立 ocgcore 脚本验证 helper 源码、构建脚本和 fixtures；当前尚未接入 Tauri command。
```

- [ ] **Step 4: Run verification**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/run-fixtures.ps1
npm run typecheck
Push-Location src-tauri; cargo test application::script; cargo check; Pop-Location
```

Expected:

- Fixture runner passes.
- TypeScript typecheck passes.
- Rust script tests pass.
- Cargo check passes.

- [ ] **Step 5: Commit docs**

Run:

```powershell
git add tools/script-validator-helper/README.md docs/system_architecture.md docs/code_structure_api.md
git commit -m "docs: document ocgcore helper boundary"
```

---

### Task 5: Final Verification

**Files:**
- No new files.

- [ ] **Step 1: Run helper fixture verification**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/build.ps1
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/run-fixtures.ps1
```

Expected: all fixtures pass.

- [ ] **Step 2: Run frontend verification**

Run:

```powershell
npm test -- src/shared/api/scriptApi.test.ts
npm run typecheck
```

Expected: test and typecheck pass.

- [ ] **Step 3: Run backend verification**

Run:

```powershell
Push-Location src-tauri
cargo test application::script
cargo check
Pop-Location
```

Expected: Rust tests and check pass.

- [ ] **Step 4: Inspect git status**

Run:

```powershell
git status --short
```

Expected: clean working tree.

- [ ] **Step 5: Inspect recent commits**

Run:

```powershell
git log --oneline -8
```

Expected: recent commits include helper spec, helper plan, scaffold/bootstrap, CLI skeleton, ocgcore integration, and docs.
