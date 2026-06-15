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
