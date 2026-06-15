# Ocgcore Helper Client Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire `validate_lua_script` to the standalone ocgcore helper through a Rust helper client so default validation runs static + ocgcore load/init.

**Architecture:** Add a focused `infrastructure::ocgcore_validator` module that owns helper JSON contracts, process execution, timeout, and failure-to-stage conversion. Add an application `ocgcore_init` adapter that builds helper input from resolved `CardEntity` + script text, then update `ScriptValidationService` and report aggregation to include real `ocgcore_init` stages while leaving UI and Agent entry points untouched.

**Tech Stack:** Rust 2024, serde/serde_json, std::process, std::thread timeout supervision, existing `ygopro_cdb` card encoding rules, existing Lua validation DTOs, PowerShell helper build/fixture scripts.

---

## File Structure

- Modify: `src-tauri/src/infrastructure/ygopro_cdb/mod.rs`  
  Expose existing CDB raw card encoding as a reusable `encode_card_data_for_ocgcore` function.
- Create: `src-tauri/src/infrastructure/ocgcore_validator/mod.rs`  
  Module exports and helper client configuration.
- Create: `src-tauri/src/infrastructure/ocgcore_validator/input.rs`  
  Helper input DTOs and helper input construction.
- Create: `src-tauri/src/infrastructure/ocgcore_validator/output.rs`  
  Helper output DTOs and conversion to `LuaValidationStageResultDto`.
- Create: `src-tauri/src/infrastructure/ocgcore_validator/helper_client.rs`  
  Helper discovery, temp input writing, child process execution, timeout, and failure stages.
- Modify: `src-tauri/src/infrastructure/mod.rs`  
  Register `ocgcore_validator`.
- Create: `src-tauri/src/application/script/ocgcore_init.rs`  
  Application-layer conversion from resolved source to helper validation stage.
- Modify: `src-tauri/src/application/script/mod.rs`  
  Register `ocgcore_init`.
- Modify: `src-tauri/src/application/script/service.rs`  
  Default levels, stage orchestration, static-error skip behavior, and helper-client injection seam for tests.
- Modify: `src-tauri/src/application/script/report.rs`  
  Stage-aware limitations and generic summary text.
- Modify: `docs/functional_spec.md`, `docs/system_architecture.md`, `docs/code_structure_api.md`  
  Current facts after verification.

---

### Task 1: Reuse YGOPro Card Encoding

**Files:**
- Modify: `src-tauri/src/infrastructure/ygopro_cdb/mod.rs`

- [ ] **Step 1: Write the failing encoder visibility test**

Add this test inside the existing `#[cfg(test)] mod tests` in `src-tauri/src/infrastructure/ygopro_cdb/mod.rs`. If the module has no tests block at the bottom, create one.

```rust
#[test]
fn encode_card_data_for_ocgcore_maps_effect_monster_fields() {
    use std::collections::BTreeMap;

    use crate::domain::card::model::{
        Attribute, CardEntity, CardTexts, MonsterFlag, Ot, PrimaryType, Race,
    };
    use crate::domain::common::time::now_utc;

    let now = now_utc();
    let card = CardEntity {
        id: "card-1".to_string(),
        code: 99999999,
        alias: 123,
        setcodes: vec![0x1234, 0x5678],
        ot: Ot::Custom,
        category: 0,
        primary_type: PrimaryType::Monster,
        texts: BTreeMap::from([(
            "en".to_string(),
            CardTexts {
                name: "Validator".to_string(),
                desc: String::new(),
                strings: Vec::new(),
            },
        )]),
        monster_flags: Some(vec![MonsterFlag::Effect]),
        atk: Some(1500),
        def: Some(1200),
        race: Some(Race::Warrior),
        attribute: Some(Attribute::Light),
        level: Some(4),
        pendulum: None,
        link: None,
        spell_subtype: None,
        trap_subtype: None,
        created_at: now,
        updated_at: now,
    };

    let encoded = encode_card_data_for_ocgcore(&card).unwrap();

    assert_eq!(encoded.raw_type, TYPE_MONSTER | TYPE_EFFECT);
    assert_eq!(encoded.attack, 1500);
    assert_eq!(encoded.defense, 1200);
    assert_eq!(encoded.level, 4);
    assert_eq!(encoded.race, 0x1);
    assert_eq!(encoded.attribute, 0x10);
    assert_eq!(encoded.link_marker, 0);
    assert_eq!(encoded.lscale, 0);
    assert_eq!(encoded.rscale, 0);
}
```

- [ ] **Step 2: Run the encoder test to verify RED**

Run:

```powershell
Push-Location src-tauri
cargo test infrastructure::ygopro_cdb::tests::encode_card_data_for_ocgcore_maps_effect_monster_fields
Pop-Location
```

Expected: FAIL to compile because `encode_card_data_for_ocgcore` does not exist.

- [ ] **Step 3: Add the reusable encoder output**

In `src-tauri/src/infrastructure/ygopro_cdb/mod.rs`, add this public struct near `YgoProCardRecord`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcgcoreCardData {
    pub raw_type: u32,
    pub attack: i32,
    pub defense: i32,
    pub level: u32,
    pub race: u32,
    pub attribute: u32,
    pub lscale: u32,
    pub rscale: u32,
    pub link_marker: u32,
}
```

Then add this public function near `encode_card`:

```rust
pub fn encode_card_data_for_ocgcore(card: &CardEntity) -> AppResult<OcgcoreCardData> {
    let encoded = encode_card(card)?;
    let is_link = encoded.raw_type & TYPE_LINK != 0;
    Ok(OcgcoreCardData {
        raw_type: encoded.raw_type as u32,
        attack: encoded.atk,
        defense: if is_link { 0 } else { encoded.def },
        level: (encoded.raw_level & 0xff) as u32,
        race: encoded.raw_race as u32,
        attribute: encoded.raw_attribute as u32,
        lscale: ((encoded.raw_level >> 24) & 0xff) as u32,
        rscale: ((encoded.raw_level >> 16) & 0xff) as u32,
        link_marker: if is_link { encoded.def as u32 } else { 0 },
    })
}
```

- [ ] **Step 4: Run the encoder test to verify GREEN**

Run:

```powershell
Push-Location src-tauri
cargo test infrastructure::ygopro_cdb::tests::encode_card_data_for_ocgcore_maps_effect_monster_fields
Pop-Location
```

Expected: PASS.

- [ ] **Step 5: Commit encoder reuse**

Run:

```powershell
git add src-tauri/src/infrastructure/ygopro_cdb/mod.rs
git commit -m "feat: expose ocgcore card data encoding"
```

---

### Task 2: Helper Input And Output Contracts

**Files:**
- Create: `src-tauri/src/infrastructure/ocgcore_validator/mod.rs`
- Create: `src-tauri/src/infrastructure/ocgcore_validator/input.rs`
- Create: `src-tauri/src/infrastructure/ocgcore_validator/output.rs`
- Modify: `src-tauri/src/infrastructure/mod.rs`

- [ ] **Step 1: Write failing helper input serialization test**

Create `src-tauri/src/infrastructure/ocgcore_validator/input.rs` with this initial test module:

```rust
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use crate::domain::card::model::{
        Attribute, CardEntity, CardTexts, MonsterFlag, Ot, PrimaryType, Race,
    };
    use crate::domain::common::time::now_utc;

    use super::*;

    #[test]
    fn helper_input_serializes_expected_script_and_card_shape() {
        let now = now_utc();
        let card = CardEntity {
            id: "card-1".to_string(),
            code: 99999999,
            alias: 0,
            setcodes: vec![0x1234],
            ot: Ot::Custom,
            category: 0,
            primary_type: PrimaryType::Monster,
            texts: BTreeMap::from([(
                "en".to_string(),
                CardTexts {
                    name: "Validator".to_string(),
                    desc: String::new(),
                    strings: Vec::new(),
                },
            )]),
            monster_flags: Some(vec![MonsterFlag::Effect]),
            atk: Some(0),
            def: Some(0),
            race: Some(Race::Warrior),
            attribute: Some(Attribute::Light),
            level: Some(4),
            pendulum: None,
            link: None,
            spell_subtype: None,
            trap_subtype: None,
            created_at: now,
            updated_at: now,
        };

        let input = HelperInput::from_card_script(
            "req-1".to_string(),
            &card,
            "local s,id,o=GetID()\nfunction s.initial_effect(c)\nend\n",
            PathBuf::from("D:/Game/YGODIY/ygocmg/third_party/ygopro-scripts"),
            3000,
        )
        .unwrap();
        let value = serde_json::to_value(&input).unwrap();

        assert_eq!(value["requestId"], "req-1");
        assert_eq!(value["level"], "ocgcore_init");
        assert_eq!(value["card"]["code"], 99999999);
        assert_eq!(value["card"]["type"], 33);
        assert_eq!(value["card"]["level"], 4);
        assert_eq!(
            value["scripts"]["./script/c99999999.lua"],
            "local s,id,o=GetID()\nfunction s.initial_effect(c)\nend\n"
        );
        assert_eq!(value["timeoutMs"], 3000);
    }
}
```

- [ ] **Step 2: Add module declarations only**

Create `src-tauri/src/infrastructure/ocgcore_validator/mod.rs`:

```rust
pub mod input;
```

Modify `src-tauri/src/infrastructure/mod.rs`:

```rust
pub mod ocgcore_validator;
```

- [ ] **Step 3: Run helper input test to verify RED**

Run:

```powershell
Push-Location src-tauri
cargo test infrastructure::ocgcore_validator::input::tests::helper_input_serializes_expected_script_and_card_shape
Pop-Location
```

Expected: FAIL to compile because `HelperInput` does not exist.

- [ ] **Step 4: Implement helper input DTOs**

Replace `src-tauri/src/infrastructure/ocgcore_validator/input.rs` with:

```rust
use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;

use crate::domain::card::model::CardEntity;
use crate::domain::common::error::AppResult;
use crate::infrastructure::ygopro_cdb::encode_card_data_for_ocgcore;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperInput {
    pub request_id: String,
    pub level: String,
    pub card: HelperCardInput,
    pub scripts: BTreeMap<String, String>,
    pub core_script_root: String,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperCardInput {
    pub code: u32,
    pub alias: u32,
    pub setcodes: Vec<u16>,
    #[serde(rename = "type")]
    pub raw_type: u32,
    pub level: u32,
    pub attribute: u32,
    pub race: u32,
    pub attack: i32,
    pub defense: i32,
    pub lscale: u32,
    pub rscale: u32,
    pub link_marker: u32,
    pub rule_code: u32,
}

impl HelperInput {
    pub fn from_card_script(
        request_id: String,
        card: &CardEntity,
        script_text: &str,
        core_script_root: PathBuf,
        timeout_ms: u64,
    ) -> AppResult<Self> {
        let encoded = encode_card_data_for_ocgcore(card)?;
        let mut scripts = BTreeMap::new();
        scripts.insert(format!("./script/c{}.lua", card.code), script_text.to_string());

        Ok(Self {
            request_id,
            level: "ocgcore_init".to_string(),
            card: HelperCardInput {
                code: card.code,
                alias: card.alias,
                setcodes: card.setcodes.clone(),
                raw_type: encoded.raw_type,
                level: encoded.level,
                attribute: encoded.attribute,
                race: encoded.race,
                attack: encoded.attack,
                defense: encoded.defense,
                lscale: encoded.lscale,
                rscale: encoded.rscale,
                link_marker: encoded.link_marker,
                rule_code: 0,
            },
            scripts,
            core_script_root: core_script_root.display().to_string(),
            timeout_ms,
        })
    }
}
```

Keep the test module from Step 1 at the bottom of the file.

- [ ] **Step 5: Write failing helper output conversion test**

Create `src-tauri/src/infrastructure/ocgcore_validator/output.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::application::script::dto::{
        LuaValidationIssueSeverityDto, LuaValidationLevelDto, LuaValidationStatusDto,
    };

    use super::*;

    #[test]
    fn helper_output_converts_to_stage_result() {
        let raw = r#"{
          "requestId": "req-1",
          "stage": "ocgcore_init",
          "status": "fail",
          "durationMs": 12,
          "issues": [{
            "severity": "error",
            "stage": "ocgcore_init",
            "code": "lua_syntax_error",
            "message": "[string \"./script/c99999999.lua\"]:7: 'end' expected",
            "line": 7,
            "column": null,
            "suggestion": "Fix the Lua syntax error reported by ocgcore."
          }],
          "log": ["syntax log"],
          "helperVersion": "0.1.0"
        }"#;

        let output: HelperOutput = serde_json::from_str(raw).unwrap();
        let stage = output.into_stage_result().unwrap();

        assert_eq!(stage.stage, LuaValidationLevelDto::OcgcoreInit);
        assert_eq!(stage.status, LuaValidationStatusDto::Fail);
        assert_eq!(stage.duration_ms, 12);
        assert_eq!(stage.issues[0].severity, LuaValidationIssueSeverityDto::Error);
        assert_eq!(stage.issues[0].code, "lua_syntax_error");
        assert_eq!(stage.issues[0].line, Some(7));
        assert_eq!(stage.log, vec!["syntax log".to_string()]);
    }
}
```

- [ ] **Step 6: Register output module and run output test to verify RED**

Modify `src-tauri/src/infrastructure/ocgcore_validator/mod.rs`:

```rust
pub mod input;
pub mod output;
```

Run:

```powershell
Push-Location src-tauri
cargo test infrastructure::ocgcore_validator::output::tests::helper_output_converts_to_stage_result
Pop-Location
```

Expected: FAIL to compile because `HelperOutput` does not exist.

- [ ] **Step 7: Implement helper output DTOs**

Replace `src-tauri/src/infrastructure/ocgcore_validator/output.rs` with:

```rust
use serde::Deserialize;

use crate::application::script::dto::{
    LuaValidationIssueDto, LuaValidationIssueSeverityDto, LuaValidationLevelDto,
    LuaValidationStageResultDto, LuaValidationStatusDto,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperOutput {
    pub request_id: String,
    pub stage: String,
    pub status: String,
    pub duration_ms: u64,
    #[serde(default)]
    pub issues: Vec<HelperIssue>,
    #[serde(default)]
    pub log: Vec<String>,
    pub helper_version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperIssue {
    pub severity: String,
    pub stage: String,
    pub code: String,
    pub message: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub suggestion: Option<String>,
}

impl HelperOutput {
    pub fn into_stage_result(self) -> Result<LuaValidationStageResultDto, String> {
        if self.stage != "ocgcore_init" {
            return Err(format!("unexpected helper stage: {}", self.stage));
        }

        let issues = self
            .issues
            .into_iter()
            .map(HelperIssue::into_issue)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(LuaValidationStageResultDto {
            stage: LuaValidationLevelDto::OcgcoreInit,
            status: parse_status(&self.status)?,
            duration_ms: self.duration_ms,
            issues,
            log: self.log,
        })
    }
}

impl HelperIssue {
    fn into_issue(self) -> Result<LuaValidationIssueDto, String> {
        if self.stage != "ocgcore_init" {
            return Err(format!("unexpected helper issue stage: {}", self.stage));
        }

        Ok(LuaValidationIssueDto {
            severity: parse_severity(&self.severity)?,
            stage: LuaValidationLevelDto::OcgcoreInit,
            code: self.code,
            message: self.message,
            line: self.line,
            column: self.column,
            suggestion: self.suggestion,
        })
    }
}

fn parse_status(value: &str) -> Result<LuaValidationStatusDto, String> {
    match value {
        "pass" => Ok(LuaValidationStatusDto::Pass),
        "warning" => Ok(LuaValidationStatusDto::Warning),
        "fail" => Ok(LuaValidationStatusDto::Fail),
        "inconclusive" => Ok(LuaValidationStatusDto::Inconclusive),
        other => Err(format!("unexpected helper status: {other}")),
    }
}

fn parse_severity(value: &str) -> Result<LuaValidationIssueSeverityDto, String> {
    match value {
        "error" => Ok(LuaValidationIssueSeverityDto::Error),
        "warning" => Ok(LuaValidationIssueSeverityDto::Warning),
        "info" => Ok(LuaValidationIssueSeverityDto::Info),
        other => Err(format!("unexpected helper issue severity: {other}")),
    }
}
```

Keep the test module from Step 5 at the bottom of the file.

- [ ] **Step 8: Run input/output tests to verify GREEN**

Run:

```powershell
Push-Location src-tauri
cargo test infrastructure::ocgcore_validator::input
cargo test infrastructure::ocgcore_validator::output
Pop-Location
```

Expected: PASS.

- [ ] **Step 9: Commit helper contracts**

Run:

```powershell
git add src-tauri/src/infrastructure/mod.rs src-tauri/src/infrastructure/ocgcore_validator
git commit -m "feat: add ocgcore helper contracts"
```

---

### Task 3: Helper Client Process Boundary

**Files:**
- Create/Modify: `src-tauri/src/infrastructure/ocgcore_validator/helper_client.rs`
- Modify: `src-tauri/src/infrastructure/ocgcore_validator/mod.rs`

- [ ] **Step 1: Write failing helper client tests**

Create `src-tauri/src/infrastructure/ocgcore_validator/helper_client.rs` with this test module:

```rust
#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use tempfile::tempdir;

    use crate::application::script::dto::LuaValidationStatusDto;
    use crate::infrastructure::ocgcore_validator::input::{HelperCardInput, HelperInput};

    use super::*;

    fn minimal_input() -> HelperInput {
        HelperInput {
            request_id: "req-1".to_string(),
            level: "ocgcore_init".to_string(),
            card: HelperCardInput {
                code: 99999999,
                alias: 0,
                setcodes: Vec::new(),
                raw_type: 33,
                level: 4,
                attribute: 16,
                race: 1,
                attack: 0,
                defense: 0,
                lscale: 0,
                rscale: 0,
                link_marker: 0,
                rule_code: 0,
            },
            scripts: std::collections::BTreeMap::from([(
                "./script/c99999999.lua".to_string(),
                "local s,id,o=GetID()\nfunction s.initial_effect(c)\nend\n".to_string(),
            )]),
            core_script_root: "D:/Game/YGODIY/ygocmg/third_party/ygopro-scripts".to_string(),
            timeout_ms: 3000,
        }
    }

    fn write_fake_helper(dir: &Path, body: &str) -> PathBuf {
        let script = dir.join("fake-helper.ps1");
        fs::write(&script, body).unwrap();
        script
    }

    fn powershell_client(script: PathBuf, timeout: Duration) -> OcgcoreValidatorHelperClient {
        OcgcoreValidatorHelperClient::for_test(
            "powershell".into(),
            vec![
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-File".into(),
                script.display().to_string(),
            ],
            timeout,
        )
    }

    #[test]
    fn run_returns_helper_stage_for_valid_stdout() {
        let dir = tempdir().unwrap();
        let script = write_fake_helper(
            dir.path(),
            r#"
param([string]$flag, [string]$inputPath)
Write-Output '{"requestId":"req-1","stage":"ocgcore_init","status":"pass","durationMs":3,"issues":[],"log":[],"helperVersion":"test"}'
"#,
        );
        let client = powershell_client(script, Duration::from_secs(2));

        let stage = client.run(&minimal_input()).unwrap();

        assert_eq!(stage.status, LuaValidationStatusDto::Pass);
        assert_eq!(stage.issues.len(), 0);
    }

    #[test]
    fn missing_helper_returns_inconclusive_stage() {
        let client = OcgcoreValidatorHelperClient::for_test(
            "Z:/missing/script-validator-helper.exe".into(),
            Vec::new(),
            Duration::from_millis(100),
        );

        let stage = client.run(&minimal_input()).unwrap();

        assert_eq!(stage.status, LuaValidationStatusDto::Inconclusive);
        assert_eq!(stage.issues[0].code, "helper_not_found");
    }

    #[test]
    fn invalid_stdout_returns_inconclusive_stage() {
        let dir = tempdir().unwrap();
        let script = write_fake_helper(
            dir.path(),
            r#"
param([string]$flag, [string]$inputPath)
Write-Output 'not json'
"#,
        );
        let client = powershell_client(script, Duration::from_secs(2));

        let stage = client.run(&minimal_input()).unwrap();

        assert_eq!(stage.status, LuaValidationStatusDto::Inconclusive);
        assert_eq!(stage.issues[0].code, "helper_invalid_output");
    }

    #[test]
    fn non_zero_exit_returns_inconclusive_stage_with_log() {
        let dir = tempdir().unwrap();
        let script = write_fake_helper(
            dir.path(),
            r#"
param([string]$flag, [string]$inputPath)
Write-Error 'boom'
exit 9
"#,
        );
        let client = powershell_client(script, Duration::from_secs(2));

        let stage = client.run(&minimal_input()).unwrap();

        assert_eq!(stage.status, LuaValidationStatusDto::Inconclusive);
        assert_eq!(stage.issues[0].code, "helper_failed");
        assert!(stage.log.iter().any(|entry| entry.contains("boom")));
    }

    #[test]
    fn timeout_returns_inconclusive_stage() {
        let dir = tempdir().unwrap();
        let script = write_fake_helper(
            dir.path(),
            r#"
param([string]$flag, [string]$inputPath)
Start-Sleep -Seconds 2
Write-Output '{"requestId":"req-1","stage":"ocgcore_init","status":"pass","durationMs":3,"issues":[],"log":[],"helperVersion":"test"}'
"#,
        );
        let client = powershell_client(script, Duration::from_millis(100));

        let stage = client.run(&minimal_input()).unwrap();

        assert_eq!(stage.status, LuaValidationStatusDto::Inconclusive);
        assert_eq!(stage.issues[0].code, "helper_timeout");
    }
}
```

- [ ] **Step 2: Run helper client tests to verify RED**

Run:

```powershell
Push-Location src-tauri
cargo test infrastructure::ocgcore_validator::helper_client
Pop-Location
```

Expected: FAIL to compile because `OcgcoreValidatorHelperClient` does not exist.

- [ ] **Step 3: Implement helper client**

Add this implementation above the test module in `src-tauri/src/infrastructure/ocgcore_validator/helper_client.rs`:

```rust
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::application::script::dto::{
    LuaValidationIssueDto, LuaValidationIssueSeverityDto, LuaValidationLevelDto,
    LuaValidationStageResultDto, LuaValidationStatusDto,
};
use crate::domain::common::error::AppResult;
use crate::infrastructure::ocgcore_validator::input::HelperInput;
use crate::infrastructure::ocgcore_validator::output::HelperOutput;

const DEFAULT_TIMEOUT: Duration = Duration::from_millis(3000);

#[derive(Debug, Clone)]
pub struct OcgcoreValidatorHelperClient {
    executable: PathBuf,
    prefix_args: Vec<String>,
    timeout: Duration,
}

impl OcgcoreValidatorHelperClient {
    pub fn discover() -> Self {
        if let Some(path) = std::env::var_os("YGOCMG_SCRIPT_VALIDATOR_HELPER") {
            return Self {
                executable: PathBuf::from(path),
                prefix_args: Vec::new(),
                timeout: DEFAULT_TIMEOUT,
            };
        }

        let candidates = [
            "../tools/script-validator-helper/build/bin/Release/script-validator-helper.exe",
            "../tools/script-validator-helper/build/bin/script-validator-helper.exe",
            "tools/script-validator-helper/build/bin/Release/script-validator-helper.exe",
            "tools/script-validator-helper/build/bin/script-validator-helper.exe",
        ];
        let executable = candidates
            .iter()
            .map(PathBuf::from)
            .find(|path| path.exists())
            .unwrap_or_else(|| PathBuf::from(candidates[0]));

        Self {
            executable,
            prefix_args: Vec::new(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    #[cfg(test)]
    pub fn for_test(executable: PathBuf, prefix_args: Vec<String>, timeout: Duration) -> Self {
        Self {
            executable,
            prefix_args,
            timeout,
        }
    }

    pub fn run(&self, input: &HelperInput) -> AppResult<LuaValidationStageResultDto> {
        if self.prefix_args.is_empty() && !self.executable.exists() {
            return Ok(failure_stage(
                "helper_not_found",
                "The ocgcore script validator helper executable was not found.",
                Some("Run powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/build.ps1 before requesting ocgcore_init validation."),
                Vec::new(),
            ));
        }

        let temp_dir = match create_temp_dir() {
            Ok(path) => path,
            Err(message) => {
                return Ok(failure_stage(
                    "helper_temp_io_failed",
                    &format!("Could not create helper input directory: {message}"),
                    Some("Check temporary directory permissions and try again."),
                    Vec::new(),
                ));
            }
        };
        let input_path = temp_dir.join("input.json");
        let encoded = match serde_json::to_vec_pretty(input) {
            Ok(value) => value,
            Err(source) => {
                cleanup_temp_dir(&temp_dir);
                return Ok(failure_stage(
                    "helper_input_encode_failed",
                    &format!("Could not encode helper input JSON: {source}"),
                    None,
                    Vec::new(),
                ));
            }
        };
        if let Err(source) = fs::write(&input_path, encoded) {
            cleanup_temp_dir(&temp_dir);
            return Ok(failure_stage(
                "helper_temp_io_failed",
                &format!("Could not write helper input JSON: {source}"),
                Some("Check temporary directory permissions and try again."),
                Vec::new(),
            ));
        }

        let result = self.run_process(&input_path);
        cleanup_temp_dir(&temp_dir);
        Ok(result)
    }

    fn run_process(&self, input_path: &Path) -> LuaValidationStageResultDto {
        let mut command = Command::new(&self.executable);
        command
            .args(&self.prefix_args)
            .arg("--input")
            .arg(input_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(source) => {
                let code = if source.kind() == std::io::ErrorKind::NotFound {
                    "helper_not_found"
                } else {
                    "helper_spawn_failed"
                };
                return failure_stage(
                    code,
                    &format!("Could not start ocgcore script validator helper: {source}"),
                    Some("Build the helper and verify YGOCMG_SCRIPT_VALIDATOR_HELPER if it is set."),
                    Vec::new(),
                );
            }
        };

        let started = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_status)) => break,
                Ok(None) if started.elapsed() >= self.timeout => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return failure_stage(
                        "helper_timeout",
                        "The ocgcore script validator helper timed out.",
                        Some("Try again after reducing script complexity or rebuilding the helper."),
                        Vec::new(),
                    );
                }
                Ok(None) => thread::sleep(Duration::from_millis(10)),
                Err(source) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return failure_stage(
                        "helper_process_failed",
                        &format!("Could not observe helper process status: {source}"),
                        None,
                        Vec::new(),
                    );
                }
            }
        }

        let output = match child.wait_with_output() {
            Ok(output) => output,
            Err(source) => {
                return failure_stage(
                    "helper_process_failed",
                    &format!("Could not collect helper process output: {source}"),
                    None,
                    Vec::new(),
                );
            }
        };
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let mut log = Vec::new();
        if !stderr.is_empty() {
            log.push(stderr.clone());
        }
        if !stdout.is_empty() && !output.status.success() {
            log.push(stdout.clone());
        }

        if !output.status.success() {
            return failure_stage(
                "helper_failed",
                &format!("The ocgcore script validator helper exited with status {}.", output.status),
                Some("Inspect helper stderr and rebuild the helper if needed."),
                log,
            );
        }
        if stdout.is_empty() {
            return failure_stage(
                "helper_invalid_output",
                "The ocgcore script validator helper wrote no stdout JSON.",
                Some("Rebuild the helper and try validation again."),
                log,
            );
        }

        match serde_json::from_str::<HelperOutput>(&stdout)
            .map_err(|source| source.to_string())
            .and_then(HelperOutput::into_stage_result)
        {
            Ok(stage) => stage,
            Err(message) => failure_stage(
                "helper_invalid_output",
                &format!("Could not parse helper stdout JSON: {message}"),
                Some("Rebuild the helper and try validation again."),
                vec![stdout],
            ),
        }
    }
}

fn create_temp_dir() -> Result<PathBuf, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|source| source.to_string())?
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "ygocmg-script-validator-{}-{now}",
        std::process::id()
    ));
    fs::create_dir_all(&path).map_err(|source| source.to_string())?;
    Ok(path)
}

fn cleanup_temp_dir(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

pub fn failure_stage(
    code: &str,
    message: &str,
    suggestion: Option<&str>,
    log: Vec<String>,
) -> LuaValidationStageResultDto {
    LuaValidationStageResultDto {
        stage: LuaValidationLevelDto::OcgcoreInit,
        status: LuaValidationStatusDto::Inconclusive,
        duration_ms: 0,
        issues: vec![LuaValidationIssueDto {
            severity: LuaValidationIssueSeverityDto::Info,
            stage: LuaValidationLevelDto::OcgcoreInit,
            code: code.to_string(),
            message: message.to_string(),
            line: None,
            column: None,
            suggestion: suggestion.map(str::to_string),
        }],
        log,
    }
}
```

- [ ] **Step 4: Export helper client types**

Modify `src-tauri/src/infrastructure/ocgcore_validator/mod.rs`:

```rust
pub mod helper_client;
pub mod input;
pub mod output;

pub use helper_client::OcgcoreValidatorHelperClient;
```

- [ ] **Step 5: Run helper client tests to verify GREEN**

Run:

```powershell
Push-Location src-tauri
cargo test infrastructure::ocgcore_validator::helper_client
Pop-Location
```

Expected: PASS.

- [ ] **Step 6: Commit helper client**

Run:

```powershell
git add src-tauri/src/infrastructure/ocgcore_validator/helper_client.rs src-tauri/src/infrastructure/ocgcore_validator/mod.rs
git commit -m "feat: add ocgcore helper process client"
```

---

### Task 4: Ocgcore Init Application Adapter

**Files:**
- Create: `src-tauri/src/application/script/ocgcore_init.rs`
- Modify: `src-tauri/src/application/script/mod.rs`

- [ ] **Step 1: Write failing adapter test**

Create `src-tauri/src/application/script/ocgcore_init.rs` with this test module:

```rust
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use crate::application::script::dto::{LuaValidationLevelDto, LuaValidationStatusDto};
    use crate::application::script::source_resolver::{ResolvedScriptSource, ScriptSourceKind};
    use crate::domain::card::model::{
        Attribute, CardEntity, CardTexts, MonsterFlag, Ot, PrimaryType, Race,
    };
    use crate::domain::common::time::now_utc;
    use crate::infrastructure::ocgcore_validator::helper_client::failure_stage;

    use super::*;

    struct RecordingClient;

    impl OcgcoreInitHelper for RecordingClient {
        fn run(
            &self,
            input: &crate::infrastructure::ocgcore_validator::input::HelperInput,
        ) -> crate::domain::common::error::AppResult<
            crate::application::script::dto::LuaValidationStageResultDto,
        > {
            assert_eq!(input.level, "ocgcore_init");
            assert_eq!(input.card.code, 99999999);
            assert!(input.scripts.contains_key("./script/c99999999.lua"));
            Ok(crate::application::script::dto::LuaValidationStageResultDto {
                stage: LuaValidationLevelDto::OcgcoreInit,
                status: LuaValidationStatusDto::Pass,
                duration_ms: 4,
                issues: Vec::new(),
                log: Vec::new(),
            })
        }
    }

    #[test]
    fn validate_ocgcore_init_builds_helper_input_from_resolved_source() {
        let now = now_utc();
        let source = ResolvedScriptSource {
            card: CardEntity {
                id: "card-1".to_string(),
                code: 99999999,
                alias: 0,
                setcodes: Vec::new(),
                ot: Ot::Custom,
                category: 0,
                primary_type: PrimaryType::Monster,
                texts: BTreeMap::from([(
                    "en".to_string(),
                    CardTexts {
                        name: "Validator".to_string(),
                        desc: String::new(),
                        strings: Vec::new(),
                    },
                )]),
                monster_flags: Some(vec![MonsterFlag::Effect]),
                atk: Some(0),
                def: Some(0),
                race: Some(Race::Warrior),
                attribute: Some(Attribute::Light),
                level: Some(4),
                pendulum: None,
                link: None,
                spell_subtype: None,
                trap_subtype: None,
                created_at: now,
                updated_at: now,
            },
            script_text: "local s,id,o=GetID()\nfunction s.initial_effect(c)\nend\n".to_string(),
            source_kind: ScriptSourceKind::Draft,
            script_path: None,
        };

        let stage = validate_ocgcore_init(
            &source,
            "workspace-1:pack-1:card-1",
            PathBuf::from("D:/Game/YGODIY/ygocmg/third_party/ygopro-scripts"),
            3000,
            &RecordingClient,
        )
        .unwrap();

        assert_eq!(stage.stage, LuaValidationLevelDto::OcgcoreInit);
        assert_eq!(stage.status, LuaValidationStatusDto::Pass);
    }

    #[test]
    fn skipped_due_to_static_errors_uses_ocgcore_stage() {
        let stage = skipped_due_to_static_errors();

        assert_eq!(stage.stage, LuaValidationLevelDto::OcgcoreInit);
        assert_eq!(stage.status, LuaValidationStatusDto::Inconclusive);
        assert_eq!(stage.issues[0].code, "skipped_due_to_static_errors");
    }
}
```

- [ ] **Step 2: Register module and run adapter test to verify RED**

Modify `src-tauri/src/application/script/mod.rs`:

```rust
pub mod ocgcore_init;
```

Run:

```powershell
Push-Location src-tauri
cargo test application::script::ocgcore_init
Pop-Location
```

Expected: FAIL to compile because adapter types/functions do not exist.

- [ ] **Step 3: Implement adapter**

Add this implementation above the test module in `src-tauri/src/application/script/ocgcore_init.rs`:

```rust
use std::path::PathBuf;

use crate::application::script::dto::{
    LuaValidationIssueDto, LuaValidationIssueSeverityDto, LuaValidationLevelDto,
    LuaValidationStageResultDto, LuaValidationStatusDto,
};
use crate::application::script::source_resolver::ResolvedScriptSource;
use crate::domain::common::error::AppResult;
use crate::infrastructure::ocgcore_validator::input::HelperInput;
use crate::infrastructure::ocgcore_validator::OcgcoreValidatorHelperClient;

pub trait OcgcoreInitHelper {
    fn run(&self, input: &HelperInput) -> AppResult<LuaValidationStageResultDto>;
}

impl OcgcoreInitHelper for OcgcoreValidatorHelperClient {
    fn run(&self, input: &HelperInput) -> AppResult<LuaValidationStageResultDto> {
        OcgcoreValidatorHelperClient::run(self, input)
    }
}

pub fn validate_ocgcore_init(
    source: &ResolvedScriptSource,
    request_id: &str,
    core_script_root: PathBuf,
    timeout_ms: u64,
    helper: &dyn OcgcoreInitHelper,
) -> AppResult<LuaValidationStageResultDto> {
    let input = HelperInput::from_card_script(
        request_id.to_string(),
        &source.card,
        &source.script_text,
        core_script_root,
        timeout_ms,
    )?;
    helper.run(&input)
}

pub fn skipped_due_to_static_errors() -> LuaValidationStageResultDto {
    LuaValidationStageResultDto {
        stage: LuaValidationLevelDto::OcgcoreInit,
        status: LuaValidationStatusDto::Inconclusive,
        duration_ms: 0,
        issues: vec![LuaValidationIssueDto {
            severity: LuaValidationIssueSeverityDto::Info,
            stage: LuaValidationLevelDto::OcgcoreInit,
            code: "skipped_due_to_static_errors".to_string(),
            message: "ocgcore_init was skipped because static validation found errors.".to_string(),
            line: None,
            column: None,
            suggestion: Some("Fix static validation errors, then run validation again.".to_string()),
        }],
        log: Vec::new(),
    }
}
```

- [ ] **Step 4: Run adapter test to verify GREEN**

Run:

```powershell
Push-Location src-tauri
cargo test application::script::ocgcore_init
Pop-Location
```

Expected: PASS.

- [ ] **Step 5: Commit adapter**

Run:

```powershell
git add src-tauri/src/application/script/mod.rs src-tauri/src/application/script/ocgcore_init.rs
git commit -m "feat: add ocgcore init validation adapter"
```

---

### Task 5: ScriptValidationService Integration

**Files:**
- Modify: `src-tauri/src/application/script/service.rs`

- [ ] **Step 1: Replace service tests with failing orchestration expectations**

In `src-tauri/src/application/script/service.rs`, update the tests module:

- Change `normalize_levels_defaults_empty_and_missing_to_static` to:

```rust
#[test]
fn normalize_levels_defaults_empty_and_missing_to_static_and_ocgcore() {
    assert_eq!(
        normalize_levels(None),
        vec![LuaValidationLevelDto::Static, LuaValidationLevelDto::OcgcoreInit]
    );
    assert_eq!(
        normalize_levels(Some(Vec::new())),
        vec![LuaValidationLevelDto::Static, LuaValidationLevelDto::OcgcoreInit]
    );
}
```

- Replace `unsupported_stage_returns_inconclusive_report_stage` with:

```rust
#[test]
fn smoke_and_scenario_stages_remain_unsupported() {
    let stages = run_requested_levels_for_test(
        "local s,id,o=GetID()\nfunction s.initial_effect(c)\nend",
        99999999,
        &[LuaValidationLevelDto::Smoke, LuaValidationLevelDto::Scenario],
    );

    assert_eq!(stages.len(), 2);
    assert_eq!(stages[0].status, LuaValidationStatusDto::Inconclusive);
    assert_eq!(stages[0].issues[0].code, "level_not_implemented");
    assert_eq!(stages[1].issues[0].code, "level_not_implemented");
}
```

- Add:

```rust
#[test]
fn static_error_skips_ocgcore_stage() {
    let stages = run_requested_levels_for_test(
        "local s,id,o=GetID()\nfunction s.thop(e,tp)\nend",
        99999999,
        &[LuaValidationLevelDto::Static, LuaValidationLevelDto::OcgcoreInit],
    );

    assert_eq!(stages.len(), 2);
    assert_eq!(stages[0].status, LuaValidationStatusDto::Fail);
    assert_eq!(stages[1].status, LuaValidationStatusDto::Inconclusive);
    assert_eq!(stages[1].issues[0].code, "skipped_due_to_static_errors");
}
```

- [ ] **Step 2: Run service tests to verify RED**

Run:

```powershell
Push-Location src-tauri
cargo test application::script::service
Pop-Location
```

Expected: FAIL because defaults are still static-only and `run_requested_levels_for_test` does not exist.

- [ ] **Step 3: Refactor service orchestration**

Modify imports at top of `src-tauri/src/application/script/service.rs`:

```rust
use std::path::PathBuf;
use std::time::Instant;

use crate::application::script::dto::{
    LuaValidationIssueDto, LuaValidationIssueSeverityDto, LuaValidationLevelDto,
    LuaValidationReportDto, LuaValidationStageResultDto, ValidateLuaScriptInput,
};
use crate::application::script::ocgcore_init::{
    skipped_due_to_static_errors, validate_ocgcore_init, OcgcoreInitHelper,
};
use crate::application::script::report::{
    assemble_report, single_issue_report, stage_status_from_issues, unsupported_stage_result,
};
use crate::application::script::source_resolver::{
    ResolvedScriptSource, ScriptSourceResolution, ScriptSourceResolver,
};
use crate::application::script::static_checker::StaticChecker;
use crate::bootstrap::AppState;
use crate::domain::common::error::AppResult;
use crate::infrastructure::ocgcore_validator::OcgcoreValidatorHelperClient;
```

Update `validate` body:

```rust
pub fn validate(&self, input: ValidateLuaScriptInput) -> AppResult<LuaValidationReportDto> {
    let levels = normalize_levels(input.levels.clone());
    let resolved = ScriptSourceResolver::new(self.state).resolve(&input)?;
    match resolved {
        ScriptSourceResolution::Ready(source) => {
            let helper = OcgcoreValidatorHelperClient::discover();
            let core_script_root = default_core_script_root();
            Ok(assemble_report(run_requested_levels(
                &source,
                &levels,
                &format!("{}:{}:{}", input.workspace_id, input.pack_id, input.card_id),
                core_script_root,
                3000,
                &helper,
            )?))
        }
        ScriptSourceResolution::MissingScript { card_code, .. } => Ok(missing_script_report(card_code)),
        ScriptSourceResolution::ReadFailed { card_code, message, .. } => {
            Ok(read_failed_report(card_code, message))
        }
    }
}
```

Replace `normalize_levels` implementation:

```rust
match levels {
    Some(values) if !values.is_empty() => values,
    _ => vec![LuaValidationLevelDto::Static, LuaValidationLevelDto::OcgcoreInit],
}
```

Replace `run_requested_levels` with:

```rust
pub(crate) fn run_requested_levels(
    source: &ResolvedScriptSource,
    levels: &[LuaValidationLevelDto],
    request_id: &str,
    core_script_root: PathBuf,
    timeout_ms: u64,
    helper: &dyn OcgcoreInitHelper,
) -> AppResult<Vec<LuaValidationStageResultDto>> {
    let mut stages = Vec::new();
    let mut static_has_errors = false;

    for level in levels.iter().copied() {
        match level {
            LuaValidationLevelDto::Static => {
                let stage = run_static_stage(&source.script_text, source.card.code);
                static_has_errors = stage
                    .issues
                    .iter()
                    .any(|issue| issue.severity == LuaValidationIssueSeverityDto::Error);
                stages.push(stage);
            }
            LuaValidationLevelDto::OcgcoreInit => {
                if static_has_errors {
                    stages.push(skipped_due_to_static_errors());
                } else {
                    stages.push(validate_ocgcore_init(
                        source,
                        request_id,
                        core_script_root.clone(),
                        timeout_ms,
                        helper,
                    )?);
                }
            }
            LuaValidationLevelDto::Smoke | LuaValidationLevelDto::Scenario => {
                stages.push(unsupported_stage_result(level, 0));
            }
        }
    }

    Ok(stages)
}
```

Add test-only helper:

```rust
#[cfg(test)]
pub(crate) fn run_requested_levels_for_test(
    script_text: &str,
    card_code: u32,
    levels: &[LuaValidationLevelDto],
) -> Vec<LuaValidationStageResultDto> {
    use crate::application::script::source_resolver::{ResolvedScriptSource, ScriptSourceKind};
    use crate::domain::card::model::{CardEntity, Ot, PrimaryType};
    use crate::domain::common::time::now_utc;

    struct InconclusiveHelper;
    impl OcgcoreInitHelper for InconclusiveHelper {
        fn run(
            &self,
            _input: &crate::infrastructure::ocgcore_validator::input::HelperInput,
        ) -> AppResult<LuaValidationStageResultDto> {
            Ok(crate::infrastructure::ocgcore_validator::helper_client::failure_stage(
                "helper_not_found",
                "test helper is not available",
                None,
                Vec::new(),
            ))
        }
    }

    let now = now_utc();
    let source = ResolvedScriptSource {
        card: CardEntity {
            id: "card-1".to_string(),
            code: card_code,
            alias: 0,
            setcodes: Vec::new(),
            ot: Ot::Custom,
            category: 0,
            primary_type: PrimaryType::Monster,
            texts: std::collections::BTreeMap::new(),
            monster_flags: None,
            atk: Some(0),
            def: Some(0),
            race: None,
            attribute: None,
            level: Some(4),
            pendulum: None,
            link: None,
            spell_subtype: None,
            trap_subtype: None,
            created_at: now,
            updated_at: now,
        },
        script_text: script_text.to_string(),
        source_kind: ScriptSourceKind::Draft,
        script_path: None,
    };

    run_requested_levels(
        &source,
        levels,
        "test-request",
        PathBuf::from("third_party/ygopro-scripts"),
        3000,
        &InconclusiveHelper,
    )
    .unwrap()
}
```

Add:

```rust
fn default_core_script_root() -> PathBuf {
    let from_src_tauri = PathBuf::from("../third_party/ygopro-scripts");
    if from_src_tauri.exists() {
        return std::fs::canonicalize(&from_src_tauri).unwrap_or(from_src_tauri);
    }
    let from_repo_root = PathBuf::from("third_party/ygopro-scripts");
    std::fs::canonicalize(&from_repo_root).unwrap_or(from_repo_root)
}
```

- [ ] **Step 4: Run service tests to verify GREEN**

Run:

```powershell
Push-Location src-tauri
cargo test application::script::service
Pop-Location
```

Expected: PASS.

- [ ] **Step 5: Commit service integration**

Run:

```powershell
git add src-tauri/src/application/script/service.rs
git commit -m "feat: run ocgcore init validation stage"
```

---

### Task 6: Report Limitations And Summary

**Files:**
- Modify: `src-tauri/src/application/script/report.rs`

- [ ] **Step 1: Write failing report behavior tests**

In `src-tauri/src/application/script/report.rs`, add these tests to the existing test module:

```rust
#[test]
fn assemble_report_adds_ocgcore_limitation_when_stage_runs() {
    let report = assemble_report(vec![
        stage(LuaValidationStatusDto::Pass, Vec::new()),
        LuaValidationStageResultDto {
            stage: LuaValidationLevelDto::OcgcoreInit,
            status: LuaValidationStatusDto::Pass,
            duration_ms: 2,
            issues: Vec::new(),
            log: Vec::new(),
        },
    ]);

    assert_eq!(report.status, LuaValidationStatusDto::Pass);
    assert_eq!(
        report.summary,
        "Lua script validation passed for the requested stages."
    );
    assert!(report
        .limitations
        .iter()
        .any(|item| item.contains("ocgcore_init only proves")));
}

#[test]
fn assemble_report_does_not_add_unsupported_limitation_for_ocgcore_init() {
    let report = assemble_report(vec![LuaValidationStageResultDto {
        stage: LuaValidationLevelDto::OcgcoreInit,
        status: LuaValidationStatusDto::Inconclusive,
        duration_ms: 0,
        issues: vec![LuaValidationIssueDto {
            severity: LuaValidationIssueSeverityDto::Info,
            stage: LuaValidationLevelDto::OcgcoreInit,
            code: "helper_not_found".to_string(),
            message: "helper missing".to_string(),
            line: None,
            column: None,
            suggestion: None,
        }],
        log: Vec::new(),
    }]);

    assert!(!report
        .limitations
        .iter()
        .any(|item| item.contains("not available") && item.contains("ocgcore_init")));
}
```

- [ ] **Step 2: Run report tests to verify RED**

Run:

```powershell
Push-Location src-tauri
cargo test application::script::report
Pop-Location
```

Expected: FAIL because pass summary is still static-specific and ocgcore limitation is missing.

- [ ] **Step 3: Update report logic**

In `src-tauri/src/application/script/report.rs`:

- Add:

```rust
const OCGCORE_INIT_LIMITATION: &str =
    "ocgcore_init only proves the script loads and initial_effect runs; it does not prove effect semantics.";
```

- Build limitations as:

```rust
let mut limitations = Vec::new();
if stages.iter().any(|stage| stage.stage == LuaValidationLevelDto::Static) {
    limitations.push(STATIC_LIMITATION.to_string());
}
if stages.iter().any(|stage| {
    stage.stage == LuaValidationLevelDto::OcgcoreInit
        && !stage.issues.iter().any(|issue| {
            matches!(
                issue.code.as_str(),
                "helper_not_found"
                    | "helper_timeout"
                    | "helper_failed"
                    | "helper_invalid_output"
                    | "helper_spawn_failed"
                    | "helper_process_failed"
                    | "helper_temp_io_failed"
                    | "helper_input_encode_failed"
                    | "skipped_due_to_static_errors"
            )
        })
}) {
    limitations.push(OCGCORE_INIT_LIMITATION.to_string());
}
```

- Change `summary_for`:

```rust
fn summary_for(status: LuaValidationStatusDto, issue_count: usize) -> String {
    match status {
        LuaValidationStatusDto::Pass => {
            "Lua script validation passed for the requested stages.".to_string()
        }
        LuaValidationStatusDto::Warning => {
            format!(
                "Lua script validation completed with {issue_count} warning or informational issue(s)."
            )
        }
        LuaValidationStatusDto::Fail => {
            format!("Lua script validation failed with {issue_count} issue(s).")
        }
        LuaValidationStatusDto::Inconclusive => {
            "Lua script validation is inconclusive for the requested input.".to_string()
        }
    }
}
```

- [ ] **Step 4: Run report tests to verify GREEN**

Run:

```powershell
Push-Location src-tauri
cargo test application::script::report
Pop-Location
```

Expected: PASS.

- [ ] **Step 5: Commit report updates**

Run:

```powershell
git add src-tauri/src/application/script/report.rs
git commit -m "feat: summarize ocgcore validation reports"
```

---

### Task 7: Real Helper Integration Verification

**Files:**
- No new source files.

- [ ] **Step 1: Build the real helper**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/build.ps1
```

Expected: output contains `Helper built at`.

- [ ] **Step 2: Run helper fixtures**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/run-fixtures.ps1
```

Expected:

```text
valid_getid.input.json => pass
missing_end.input.json => fail
missing_core.input.json => fail
```

- [ ] **Step 3: Run Rust script and helper client tests**

Run:

```powershell
Push-Location src-tauri
cargo test application::script
cargo test infrastructure::ocgcore_validator
cargo check
Pop-Location
```

Expected: all pass.

- [ ] **Step 4: Run frontend validation contract checks**

Run:

```powershell
npm test -- src/shared/api/scriptApi.test.ts
npm run typecheck
```

Expected: all pass.

- [ ] **Step 5: Inspect verification state**

Run:

```powershell
git status --short
```

Expected: no output. If there are modified source files, stop and inspect them before continuing to the docs task; do not make a broad verification commit.

---

### Task 8: Current Documentation Updates

**Files:**
- Modify: `docs/functional_spec.md`
- Modify: `docs/system_architecture.md`
- Modify: `docs/code_structure_api.md`

- [ ] **Step 1: Update functional spec**

In `docs/functional_spec.md`, replace the resource script validation sentence:

```markdown
- 自定义包中的卡片脚本支持静态验证：后端可读取当前保存的 `scripts/c{code}.lua`，或验证调用方传入的未保存 `scriptText` 草稿，并返回结构化状态、阶段结果、issues 和 limitations。当前实现只做 deterministic static check，不运行 ocgcore，也不证明效果语义正确。
```

with:

```markdown
- 自定义包中的卡片脚本支持验证：后端可读取当前保存的 `scripts/c{code}.lua`，或验证调用方传入的未保存 `scriptText` 草稿，并返回结构化状态、阶段结果、issues 和 limitations。默认验证包含 deterministic static check 和独立 helper 进程中的 ocgcore load/init；`ocgcore_init` 只证明脚本能加载并执行 `initial_effect`，不证明效果语义正确。helper 未构建、超时、崩溃或输出异常时，报告以 `inconclusive` 表达而不是让 Tauri command 失败。
```

- [ ] **Step 2: Update architecture docs**

In `docs/system_architecture.md`, replace the two paragraphs under `## Lua 脚本验证架构` that say static-only and helper not yet wired with:

```markdown
当前实现包含 `ScriptValidationService`、`ScriptSourceResolver`、`StaticChecker`、`OcgcoreInitValidator` 和报告聚合。默认验证阶段为 `static + ocgcore_init`：静态检查可发现缺少 `initial_effect`、未定义 callback、危险 Lua API 和少量高置信拼写错误；`ocgcore_init` 通过独立 helper 进程验证 `new_card -> load_card_script -> initial_effect`，确认脚本可被真实 ocgcore 加载并完成初始化。

`tools/script-validator-helper` 包含独立的 ocgcore load/init helper 源码、构建脚本和 fixture runner。Tauri 主进程不直接链接 ocgcore；`src-tauri/src/infrastructure/ocgcore_validator` 负责写入 helper input JSON、启动 helper、设置 timeout、解析 stdout JSON，并把 helper 缺失、超时、崩溃或非法输出转换为 `ocgcore_init` 的 `inconclusive` stage。`ocgcore_init` 通过只代表脚本 load/init 成功，不代表效果语义正确；smoke、scenario 和 Agent/UI 入口仍属于后续阶段。
```

- [ ] **Step 3: Update code structure docs**

In `docs/code_structure_api.md`, under backend directory list, add:

```markdown
- `src-tauri/src/infrastructure/ocgcore_validator`：Rust helper client，负责调用独立 ocgcore 脚本验证 helper、处理 timeout 和 stdout JSON 转换。
```

Update the existing `tools/script-validator-helper` bullet to:

```markdown
- `tools/script-validator-helper`：独立 ocgcore 脚本验证 helper 源码、构建脚本和 fixtures；由 Rust `ocgcore_validator` client 在 `ocgcore_init` 阶段调用。
```

- [ ] **Step 4: Run docs verification commands**

Run:

```powershell
rg "静态验证|不运行 ocgcore|尚未调用 helper|当前尚未接入 Tauri command" docs/functional_spec.md docs/system_architecture.md docs/code_structure_api.md
```

Expected: no stale claim that script validation is static-only or that helper is not wired into `validate_lua_script`.

- [ ] **Step 5: Run core verification**

Run:

```powershell
Push-Location src-tauri
cargo test application::script
cargo test infrastructure::ocgcore_validator
cargo check
Pop-Location
npm run typecheck
```

Expected: all pass.

- [ ] **Step 6: Commit docs**

Run:

```powershell
git add docs/functional_spec.md docs/system_architecture.md docs/code_structure_api.md
git commit -m "docs: document ocgcore validation integration"
```

---

### Task 9: Final Verification

**Files:**
- No new files.

- [ ] **Step 1: Run helper verification**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/build.ps1
powershell -ExecutionPolicy Bypass -File tools/script-validator-helper/scripts/run-fixtures.ps1
```

Expected: helper builds and fixture runner passes.

- [ ] **Step 2: Run Rust verification**

Run:

```powershell
Push-Location src-tauri
cargo test application::script
cargo test infrastructure::ocgcore_validator
cargo check
Pop-Location
```

Expected: all pass.

- [ ] **Step 3: Run frontend verification**

Run:

```powershell
npm test -- src/shared/api/scriptApi.test.ts
npm run typecheck
```

Expected: all pass.

- [ ] **Step 4: Inspect git state and recent commits**

Run:

```powershell
git status --short
git log --oneline -10
```

Expected:

- `git status --short` has no output.
- recent commits include spec, plan, encoder reuse, helper contracts, helper client, app adapter, service integration, report updates, and docs.
