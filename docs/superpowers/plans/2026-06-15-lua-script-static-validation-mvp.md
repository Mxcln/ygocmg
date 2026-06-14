# Lua Script Static Validation MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first Lua script validation platform slice: a `validate_lua_script` command with frontend contract/API wrapper, backend source resolution, deterministic static checks, structured reports, tests, and current docs.

**Architecture:** The frontend exposes only typed contracts and `scriptApi.validateLuaScript()`. The Tauri command delegates to `application::script::ScriptValidationService`, which resolves the card script from an open custom pack or draft `scriptText`, runs static checks, and assembles a `LuaValidationReport`. Unsupported non-static levels return structured `inconclusive` stage results; ocgcore, Agent, and UI entry points stay outside this slice.

**Tech Stack:** React/TypeScript/Vitest frontend, Tauri 2 IPC, Rust 2024 backend, serde DTOs, existing `AppState` session APIs, `cargo test`, `cargo check`, `npm run typecheck`.

---

## File Structure

- Create: `src/shared/contracts/script.ts`  
  Frontend boundary types for validation input, issue, stage result, and report.
- Create: `src/shared/api/scriptApi.ts`  
  Frontend wrapper around `invokeApi("validate_lua_script", { input })`.
- Create: `src/shared/api/scriptApi.test.ts`  
  Vitest coverage that the wrapper calls the right command with the right payload.
- Create: `src-tauri/src/application/script/mod.rs`  
  Rust script validation module exports.
- Create: `src-tauri/src/application/script/dto.rs`  
  Rust serde DTOs matching `src/shared/contracts/script.ts`.
- Create: `src-tauri/src/application/script/report.rs`  
  Stage/report status aggregation and standard report constructors.
- Create: `src-tauri/src/application/script/static_checker.rs`  
  Conservative static scanner and unit tests.
- Create: `src-tauri/src/application/script/source_resolver.rs`  
  Resolves card context and script text from draft input or saved script path.
- Create: `src-tauri/src/application/script/service.rs`  
  Orchestrates source resolution, requested levels, static checker, and report assembly.
- Modify: `src-tauri/src/application/mod.rs`  
  Expose `application::script`.
- Modify: `src-tauri/src/presentation/commands/app_commands.rs`  
  Add presentation function for `validate_lua_script`.
- Modify: `src-tauri/src/tauri_commands.rs`  
  Add Tauri command wrapper.
- Modify: `src-tauri/src/main.rs`  
  Register command in `generate_handler!`.
- Modify: `docs/functional_spec.md`  
  Document static Lua script validation as implemented.
- Modify: `docs/system_architecture.md`  
  Document backend script validation service boundary.
- Modify: `docs/code_structure_api.md`  
  Document new contract/API wrapper/command.

---

### Task 1: Frontend Contract And API Wrapper

**Files:**
- Create: `src/shared/contracts/script.ts`
- Create: `src/shared/api/scriptApi.ts`
- Create: `src/shared/api/scriptApi.test.ts`

- [ ] **Step 1: Write the failing API wrapper test**

Create `src/shared/api/scriptApi.test.ts`:

```ts
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeApi } from "./invoke";
import { scriptApi } from "./scriptApi";
import type { LuaValidationReport } from "../contracts/script";

vi.mock("./invoke", () => ({
  invokeApi: vi.fn(),
}));

beforeEach(() => {
  vi.clearAllMocks();
});

describe("scriptApi", () => {
  it("calls validate_lua_script with a typed input payload", async () => {
    const report = {
      status: "pass",
      confidence: "high",
      summary: "Static validation passed.",
      issues: [],
      stages: [
        {
          stage: "static",
          status: "pass",
          durationMs: 2,
          issues: [],
          log: [],
        },
      ],
      limitations: [
        "Static validation does not prove ocgcore load/init success or effect semantics.",
      ],
    } satisfies LuaValidationReport;
    vi.mocked(invokeApi).mockResolvedValue(report);

    await expect(
      scriptApi.validateLuaScript({
        workspaceId: "workspace-1",
        packId: "pack-1",
        cardId: "card-1",
        scriptText: "local s,id,o=GetID()\nfunction s.initial_effect(c)\nend",
        levels: ["static"],
      }),
    ).resolves.toBe(report);

    expect(invokeApi).toHaveBeenCalledWith("validate_lua_script", {
      input: {
        workspaceId: "workspace-1",
        packId: "pack-1",
        cardId: "card-1",
        scriptText: "local s,id,o=GetID()\nfunction s.initial_effect(c)\nend",
        levels: ["static"],
      },
    });
  });
});
```

- [ ] **Step 2: Run the frontend test and verify it fails**

Run:

```powershell
npm test -- src/shared/api/scriptApi.test.ts
```

Expected: FAIL because `src/shared/api/scriptApi.ts` and `src/shared/contracts/script.ts` do not exist.

- [ ] **Step 3: Add the frontend contract**

Create `src/shared/contracts/script.ts`:

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

- [ ] **Step 4: Add the API wrapper**

Create `src/shared/api/scriptApi.ts`:

```ts
import { invokeApi } from "./invoke";
import type {
  LuaValidationReport,
  ValidateLuaScriptInput,
} from "../contracts/script";

export const scriptApi = {
  validateLuaScript(input: ValidateLuaScriptInput) {
    return invokeApi<LuaValidationReport>("validate_lua_script", { input });
  },
};
```

- [ ] **Step 5: Run the frontend test and typecheck**

Run:

```powershell
npm test -- src/shared/api/scriptApi.test.ts
npm run typecheck
```

Expected: PASS for `scriptApi.test.ts`; `npm run typecheck` should complete without TypeScript errors.

- [ ] **Step 6: Commit frontend contract and wrapper**

Run:

```powershell
git add src/shared/contracts/script.ts src/shared/api/scriptApi.ts src/shared/api/scriptApi.test.ts
git commit -m "feat: add lua script validation api wrapper"
```

---

### Task 2: Rust DTOs And Report Assembly

**Files:**
- Create: `src-tauri/src/application/script/mod.rs`
- Create: `src-tauri/src/application/script/dto.rs`
- Create: `src-tauri/src/application/script/report.rs`
- Modify: `src-tauri/src/application/mod.rs`

- [ ] **Step 1: Write failing report tests**

Create `src-tauri/src/application/script/mod.rs`:

```rust
pub mod dto;
pub mod report;
```

Create `src-tauri/src/application/script/report.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::script::dto::{
        LuaValidationIssueDto, LuaValidationIssueSeverityDto, LuaValidationLevelDto,
        LuaValidationStageResultDto, LuaValidationStatusDto,
    };

    fn stage(
        status: LuaValidationStatusDto,
        issues: Vec<LuaValidationIssueDto>,
    ) -> LuaValidationStageResultDto {
        LuaValidationStageResultDto {
            stage: LuaValidationLevelDto::Static,
            status,
            duration_ms: 1,
            issues,
            log: Vec::new(),
        }
    }

    #[test]
    fn assemble_report_fails_when_any_issue_is_error() {
        let issue = LuaValidationIssueDto {
            severity: LuaValidationIssueSeverityDto::Error,
            stage: LuaValidationLevelDto::Static,
            code: "missing_initial_effect".to_string(),
            message: "Lua script does not define initial_effect(c).".to_string(),
            line: None,
            column: None,
            suggestion: Some("Define function s.initial_effect(c).".to_string()),
        };

        let report = assemble_report(vec![stage(
            LuaValidationStatusDto::Fail,
            vec![issue.clone()],
        )]);

        assert_eq!(report.status, LuaValidationStatusDto::Fail);
        assert_eq!(report.confidence, crate::application::script::dto::LuaValidationConfidenceDto::High);
        assert_eq!(report.issues, vec![issue]);
        assert!(report.summary.contains("failed"));
    }

    #[test]
    fn assemble_report_warns_without_errors() {
        let issue = LuaValidationIssueDto {
            severity: LuaValidationIssueSeverityDto::Warning,
            stage: LuaValidationLevelDto::Static,
            code: "dangerous_lua_api".to_string(),
            message: "Script uses os.execute.".to_string(),
            line: Some(4),
            column: None,
            suggestion: Some("Remove shell execution from card scripts.".to_string()),
        };

        let report = assemble_report(vec![stage(
            LuaValidationStatusDto::Warning,
            vec![issue.clone()],
        )]);

        assert_eq!(report.status, LuaValidationStatusDto::Warning);
        assert_eq!(report.confidence, crate::application::script::dto::LuaValidationConfidenceDto::Medium);
        assert_eq!(report.issues, vec![issue]);
    }

    #[test]
    fn unsupported_stage_result_is_inconclusive() {
        let result = unsupported_stage_result(LuaValidationLevelDto::OcgcoreInit, 0);

        assert_eq!(result.status, LuaValidationStatusDto::Inconclusive);
        assert_eq!(result.issues[0].code, "level_not_implemented");
        assert_eq!(result.issues[0].stage, LuaValidationLevelDto::OcgcoreInit);
    }
}
```

Modify `src-tauri/src/application/mod.rs` by adding:

```rust
pub mod script;
```

- [ ] **Step 2: Run the Rust test and verify it fails**

Run:

```powershell
Push-Location src-tauri
cargo test application::script::report
Pop-Location
```

Expected: FAIL because `dto.rs` and report functions/types are missing.

- [ ] **Step 3: Add Rust DTOs**

Create `src-tauri/src/application/script/dto.rs`:

```rust
use serde::{Deserialize, Serialize};

use crate::domain::common::ids::{CardId, PackId, WorkspaceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LuaValidationLevelDto {
    Static,
    OcgcoreInit,
    Smoke,
    Scenario,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LuaValidationStatusDto {
    Pass,
    Warning,
    Fail,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LuaValidationConfidenceDto {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LuaValidationIssueSeverityDto {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateLuaScriptInput {
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub card_id: CardId,
    pub script_text: Option<String>,
    #[serde(default)]
    pub levels: Option<Vec<LuaValidationLevelDto>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LuaValidationIssueDto {
    pub severity: LuaValidationIssueSeverityDto,
    pub stage: LuaValidationLevelDto,
    pub code: String,
    pub message: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LuaValidationStageResultDto {
    pub stage: LuaValidationLevelDto,
    pub status: LuaValidationStatusDto,
    pub duration_ms: u64,
    pub issues: Vec<LuaValidationIssueDto>,
    #[serde(default)]
    pub log: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LuaValidationReportDto {
    pub status: LuaValidationStatusDto,
    pub confidence: LuaValidationConfidenceDto,
    pub summary: String,
    pub issues: Vec<LuaValidationIssueDto>,
    pub stages: Vec<LuaValidationStageResultDto>,
    pub limitations: Vec<String>,
}
```

- [ ] **Step 4: Implement report assembly**

Replace `src-tauri/src/application/script/report.rs` with the tests from Step 1 plus this implementation above the test module:

```rust
use crate::application::script::dto::{
    LuaValidationConfidenceDto, LuaValidationIssueDto, LuaValidationIssueSeverityDto,
    LuaValidationLevelDto, LuaValidationReportDto, LuaValidationStageResultDto,
    LuaValidationStatusDto,
};

const STATIC_LIMITATION: &str =
    "Static validation does not prove ocgcore load/init success or effect semantics.";

pub fn assemble_report(stages: Vec<LuaValidationStageResultDto>) -> LuaValidationReportDto {
    let issues = stages
        .iter()
        .flat_map(|stage| stage.issues.iter().cloned())
        .collect::<Vec<_>>();
    let status = final_status(&stages, &issues);
    let confidence = confidence_for(status, &stages, &issues);
    let mut limitations = vec![STATIC_LIMITATION.to_string()];

    let unsupported = stages
        .iter()
        .filter(|stage| {
            stage
                .issues
                .iter()
                .any(|issue| issue.code == "level_not_implemented")
        })
        .map(|stage| format!("{:?}", stage.stage))
        .collect::<Vec<_>>();
    if !unsupported.is_empty() {
        limitations.push(format!(
            "Requested validation levels are not available in this MVP: {}.",
            unsupported.join(", ")
        ));
    }

    LuaValidationReportDto {
        status,
        confidence,
        summary: summary_for(status, issues.len()),
        issues,
        stages,
        limitations,
    }
}

pub fn stage_status_from_issues(issues: &[LuaValidationIssueDto]) -> LuaValidationStatusDto {
    if issues
        .iter()
        .any(|issue| issue.severity == LuaValidationIssueSeverityDto::Error)
    {
        LuaValidationStatusDto::Fail
    } else if issues
        .iter()
        .any(|issue| issue.severity == LuaValidationIssueSeverityDto::Warning)
    {
        LuaValidationStatusDto::Warning
    } else {
        LuaValidationStatusDto::Pass
    }
}

pub fn unsupported_stage_result(
    stage: LuaValidationLevelDto,
    duration_ms: u64,
) -> LuaValidationStageResultDto {
    LuaValidationStageResultDto {
        stage,
        status: LuaValidationStatusDto::Inconclusive,
        duration_ms,
        issues: vec![LuaValidationIssueDto {
            severity: LuaValidationIssueSeverityDto::Info,
            stage,
            code: "level_not_implemented".to_string(),
            message: "This validation level is not implemented in the static validation MVP."
                .to_string(),
            line: None,
            column: None,
            suggestion: Some("Run static validation now; ocgcore and scenario stages will be added in later slices.".to_string()),
        }],
        log: Vec::new(),
    }
}

pub fn single_issue_report(issue: LuaValidationIssueDto) -> LuaValidationReportDto {
    assemble_report(vec![LuaValidationStageResultDto {
        stage: issue.stage,
        status: match issue.severity {
            LuaValidationIssueSeverityDto::Error => LuaValidationStatusDto::Fail,
            LuaValidationIssueSeverityDto::Warning => LuaValidationStatusDto::Warning,
            LuaValidationIssueSeverityDto::Info => LuaValidationStatusDto::Inconclusive,
        },
        duration_ms: 0,
        issues: vec![issue],
        log: Vec::new(),
    }])
}

fn final_status(
    stages: &[LuaValidationStageResultDto],
    issues: &[LuaValidationIssueDto],
) -> LuaValidationStatusDto {
    if issues
        .iter()
        .any(|issue| issue.severity == LuaValidationIssueSeverityDto::Error)
        || stages
            .iter()
            .any(|stage| stage.status == LuaValidationStatusDto::Fail)
    {
        LuaValidationStatusDto::Fail
    } else if issues
        .iter()
        .any(|issue| issue.severity == LuaValidationIssueSeverityDto::Warning)
        || stages
            .iter()
            .any(|stage| stage.status == LuaValidationStatusDto::Warning)
    {
        LuaValidationStatusDto::Warning
    } else if stages.is_empty()
        || stages
            .iter()
            .all(|stage| stage.status == LuaValidationStatusDto::Inconclusive)
    {
        LuaValidationStatusDto::Inconclusive
    } else {
        LuaValidationStatusDto::Pass
    }
}

fn confidence_for(
    status: LuaValidationStatusDto,
    stages: &[LuaValidationStageResultDto],
    _issues: &[LuaValidationIssueDto],
) -> LuaValidationConfidenceDto {
    if status == LuaValidationStatusDto::Inconclusive
        || stages
            .iter()
            .all(|stage| stage.status == LuaValidationStatusDto::Inconclusive)
    {
        LuaValidationConfidenceDto::Low
    } else if status == LuaValidationStatusDto::Warning {
        LuaValidationConfidenceDto::Medium
    } else {
        LuaValidationConfidenceDto::High
    }
}

fn summary_for(status: LuaValidationStatusDto, issue_count: usize) -> String {
    match status {
        LuaValidationStatusDto::Pass => "Static validation passed.".to_string(),
        LuaValidationStatusDto::Warning => {
            format!("Static validation completed with {issue_count} warning or informational issue(s).")
        }
        LuaValidationStatusDto::Fail => {
            format!("Static validation failed with {issue_count} issue(s).")
        }
        LuaValidationStatusDto::Inconclusive => {
            "Lua script validation is inconclusive for the requested input.".to_string()
        }
    }
}
```

- [ ] **Step 5: Run Rust report tests**

Run:

```powershell
Push-Location src-tauri
cargo test application::script::report
Pop-Location
```

Expected: PASS for report tests.

- [ ] **Step 6: Commit Rust DTO and report assembly**

Run:

```powershell
git add src-tauri/src/application/mod.rs src-tauri/src/application/script/mod.rs src-tauri/src/application/script/dto.rs src-tauri/src/application/script/report.rs
git commit -m "feat: add lua validation report dto"
```

---

### Task 3: Static Checker

**Files:**
- Create: `src-tauri/src/application/script/static_checker.rs`
- Modify: `src-tauri/src/application/script/mod.rs`

- [ ] **Step 1: Register the module and write failing static checker tests**

Modify `src-tauri/src/application/script/mod.rs`:

```rust
pub mod dto;
pub mod report;
pub mod static_checker;
```

Create `src-tauri/src/application/script/static_checker.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::script::dto::LuaValidationIssueSeverityDto;

    #[test]
    fn valid_get_id_script_has_no_static_errors() {
        let script = r#"
local s,id,o=GetID()
function s.initial_effect(c)
  local e1=Effect.CreateEffect(c)
  e1:SetType(EFFECT_TYPE_SINGLE)
  e1:SetOperation(s.thop)
  c:RegisterEffect(e1)
end
function s.thop(e,tp,eg,ep,ev,re,r,rp)
end
"#;

        let issues = StaticChecker::check(script, 99999999);

        assert!(issues.is_empty(), "{issues:#?}");
    }

    #[test]
    fn missing_initial_effect_is_an_error() {
        let issues = StaticChecker::check("local s,id,o=GetID()\nfunction s.thop(e,tp)\nend", 99999999);

        assert!(issues.iter().any(|issue| {
            issue.code == "missing_initial_effect"
                && issue.severity == LuaValidationIssueSeverityDto::Error
        }));
    }

    #[test]
    fn undefined_set_operation_callback_is_an_error() {
        let script = r#"
local s,id,o=GetID()
function s.initial_effect(c)
  local e1=Effect.CreateEffect(c)
  e1:SetOperation(s.thop)
  c:RegisterEffect(e1)
end
"#;

        let issues = StaticChecker::check(script, 99999999);

        assert!(issues.iter().any(|issue| {
            issue.code == "undefined_effect_callback"
                && issue.message.contains("s.thop")
                && issue.line == Some(5)
        }));
    }

    #[test]
    fn target_without_chk_zero_branch_is_a_warning() {
        let script = r#"
local s,id,o=GetID()
function s.initial_effect(c)
  local e1=Effect.CreateEffect(c)
  e1:SetTarget(s.target)
  c:RegisterEffect(e1)
end
function s.target(e,tp,eg,ep,ev,re,r,rp,chk)
  Duel.SelectMatchingCard(tp,Card.IsAbleToHand,tp,LOCATION_DECK,0,1,1,nil)
end
"#;

        let issues = StaticChecker::check(script, 99999999);

        assert!(issues.iter().any(|issue| {
            issue.code == "target_missing_chk_branch"
                && issue.severity == LuaValidationIssueSeverityDto::Warning
        }));
    }

    #[test]
    fn direct_callback_like_reference_without_definition_is_an_error() {
        let script = r#"
local s,id,o=GetID()
function s.initial_effect(c)
end
function s.target(e,tp,eg,ep,ev,re,r,rp,chk)
  if chk==0 then return Duel.IsExistingMatchingCard(s.filter,tp,LOCATION_DECK,0,1,nil) end
end
"#;

        let issues = StaticChecker::check(script, 99999999);

        assert!(issues.iter().any(|issue| {
            issue.code == "undefined_script_function_reference"
                && issue.severity == LuaValidationIssueSeverityDto::Error
                && issue.message.contains("s.filter")
                && issue.line == Some(6)
        }));
    }

    #[test]
    fn dangerous_lua_api_is_a_warning() {
        let script = r#"
local s,id,o=GetID()
function s.initial_effect(c)
  os.execute("calc")
end
"#;

        let issues = StaticChecker::check(script, 99999999);

        assert!(issues.iter().any(|issue| {
            issue.code == "dangerous_lua_api"
                && issue.severity == LuaValidationIssueSeverityDto::Warning
                && issue.line == Some(4)
        }));
    }
}
```

- [ ] **Step 2: Run the static checker tests and verify they fail**

Run:

```powershell
Push-Location src-tauri
cargo test application::script::static_checker
Pop-Location
```

Expected: FAIL because `StaticChecker::check` and implementation helpers do not exist.

- [ ] **Step 3: Implement the static checker**

Add this implementation above the test module in `src-tauri/src/application/script/static_checker.rs`:

```rust
use std::collections::{BTreeMap, BTreeSet};

use crate::application::script::dto::{
    LuaValidationIssueDto, LuaValidationIssueSeverityDto, LuaValidationLevelDto,
};

pub struct StaticChecker;

#[derive(Debug, Clone)]
struct FunctionDef {
    qualified_name: String,
    line: u32,
    body: String,
}

impl StaticChecker {
    pub fn check(script: &str, card_code: u32) -> Vec<LuaValidationIssueDto> {
        let lines = script
            .lines()
            .enumerate()
            .map(|(index, raw)| {
                let without_comment = raw.split("--").next().unwrap_or_default().to_string();
                (index as u32 + 1, raw.to_string(), without_comment)
            })
            .collect::<Vec<_>>();
        let functions = collect_functions(&lines);
        let mut issues = Vec::new();

        if !has_initial_effect(&functions, card_code) {
            issues.push(issue(
                LuaValidationIssueSeverityDto::Error,
                "missing_initial_effect",
                "Lua script does not define initial_effect(c).",
                None,
                Some("Define function s.initial_effect(c) for GetID-style scripts or c{code}.initial_effect(c) for legacy scripts."),
            ));
        }

        let legacy_prefix = format!("c{card_code}.");
        let has_get_id = script.contains("GetID()");
        let has_legacy = lines
            .iter()
            .any(|(_, _, line)| line.contains(&legacy_prefix));
        if has_get_id && has_legacy {
            issues.push(issue(
                LuaValidationIssueSeverityDto::Warning,
                "mixed_script_style",
                "Script mixes GetID() style with legacy c{code}. function references.",
                first_line_containing(&lines, &legacy_prefix),
                Some("Prefer one script style in a single card script."),
            ));
        }

        issues.extend(check_callback_references(
            &lines,
            &functions,
            card_code,
            "SetCondition",
        ));
        issues.extend(check_callback_references(
            &lines,
            &functions,
            card_code,
            "SetCost",
        ));
        issues.extend(check_callback_references(
            &lines,
            &functions,
            card_code,
            "SetTarget",
        ));
        issues.extend(check_callback_references(
            &lines,
            &functions,
            card_code,
            "SetOperation",
        ));
        issues.extend(check_target_bodies(&lines, &functions, card_code));
        issues.extend(check_direct_callback_like_references(
            &lines,
            &functions,
            card_code,
        ));
        issues.extend(check_dangerous_api(&lines));
        issues.extend(check_common_typos(&lines));

        dedupe_issues(issues)
    }
}

fn collect_functions(lines: &[(u32, String, String)]) -> BTreeMap<String, FunctionDef> {
    let mut defs = BTreeMap::new();
    for (index, (line_no, _raw, line)) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let Some(name) = function_name_from_line(trimmed) else {
            continue;
        };
        let body = collect_body(lines, index);
        defs.insert(
            name.clone(),
            FunctionDef {
                qualified_name: name,
                line: *line_no,
                body,
            },
        );
    }
    defs
}

fn function_name_from_line(line: &str) -> Option<String> {
    if let Some(rest) = line.strip_prefix("function ") {
        return rest
            .split('(')
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
    }

    let compact = line.replace(' ', "");
    let marker = "=function";
    compact
        .find(marker)
        .map(|position| compact[..position].to_string())
        .filter(|value| !value.is_empty())
}

fn collect_body(lines: &[(u32, String, String)], start: usize) -> String {
    let mut body = String::new();
    for (_, _raw, line) in lines.iter().skip(start) {
        let trimmed = line.trim();
        if !body.is_empty()
            && (trimmed.starts_with("function ")
                || trimmed.contains("=function")
                || trimmed.contains("= function"))
        {
            break;
        }
        body.push_str(line);
        body.push('\n');
    }
    body
}

fn has_initial_effect(functions: &BTreeMap<String, FunctionDef>, card_code: u32) -> bool {
    let legacy = format!("c{card_code}.initial_effect");
    functions.contains_key("s.initial_effect") || functions.contains_key(&legacy)
}

fn check_callback_references(
    lines: &[(u32, String, String)],
    functions: &BTreeMap<String, FunctionDef>,
    card_code: u32,
    setter: &str,
) -> Vec<LuaValidationIssueDto> {
    let mut issues = Vec::new();
    for (line_no, _raw, line) in lines {
        if !line.contains(setter) {
            continue;
        }
        for reference in callback_references(line, card_code) {
            if !functions.contains_key(&reference) {
                issues.push(issue(
                    LuaValidationIssueSeverityDto::Error,
                    "undefined_effect_callback",
                    format!("{setter} references {reference}, but no matching function definition was found."),
                    Some(*line_no),
                    Some("Define the referenced function or update the callback reference."),
                ));
            }
        }
    }
    issues
}

fn callback_references(line: &str, card_code: u32) -> Vec<String> {
    let mut references = Vec::new();
    collect_prefix_references(line, "s.", &mut references);
    collect_prefix_references(line, &format!("c{card_code}."), &mut references);
    references
}

fn collect_prefix_references(line: &str, prefix: &str, references: &mut Vec<String>) {
    let mut start = 0;
    while let Some(offset) = line[start..].find(prefix) {
        let absolute = start + offset;
        let after_prefix = absolute + prefix.len();
        let name = line[after_prefix..]
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .collect::<String>();
        if !name.is_empty() {
            references.push(format!("{prefix}{name}"));
        }
        start = after_prefix + name.len();
    }
}

fn check_direct_callback_like_references(
    lines: &[(u32, String, String)],
    functions: &BTreeMap<String, FunctionDef>,
    card_code: u32,
) -> Vec<LuaValidationIssueDto> {
    let mut issues = Vec::new();
    for (line_no, _raw, line) in lines {
        if line.contains("SetCondition")
            || line.contains("SetCost")
            || line.contains("SetTarget")
            || line.contains("SetOperation")
            || !(line.contains("Duel.") || line.contains("aux."))
        {
            continue;
        }
        for reference in callback_references(line, card_code) {
            if !functions.contains_key(&reference) {
                issues.push(issue(
                    LuaValidationIssueSeverityDto::Error,
                    "undefined_script_function_reference",
                    format!("Script references {reference}, but no matching function definition was found."),
                    Some(*line_no),
                    Some("Define the referenced function or remove the callback reference."),
                ));
            }
        }
    }
    issues
}

fn check_target_bodies(
    lines: &[(u32, String, String)],
    functions: &BTreeMap<String, FunctionDef>,
    card_code: u32,
) -> Vec<LuaValidationIssueDto> {
    let mut issues = Vec::new();
    let mut target_refs = BTreeSet::new();
    for (_, _raw, line) in lines {
        if line.contains("SetTarget") {
            target_refs.extend(callback_references(line, card_code));
        }
    }

    for reference in target_refs {
        let Some(definition) = functions.get(&reference) else {
            continue;
        };
        let compact = definition.body.replace([' ', '\t'], "");
        if !compact.contains("chk==0") {
            issues.push(issue(
                LuaValidationIssueSeverityDto::Warning,
                "target_missing_chk_branch",
                format!("Target callback {} does not contain a recognizable chk==0 branch.", definition.qualified_name),
                Some(definition.line),
                Some("Most YGOPro target functions should return availability from an if chk==0 then ... end branch."),
            ));
        }
    }
    issues
}

fn check_dangerous_api(lines: &[(u32, String, String)]) -> Vec<LuaValidationIssueDto> {
    const PATTERNS: [&str; 6] = [
        "os.execute",
        "io.popen",
        "package.loadlib",
        "loadfile",
        "dofile",
        "require",
    ];

    let mut issues = Vec::new();
    for (line_no, _raw, line) in lines {
        for pattern in PATTERNS {
            if line.contains(pattern) {
                issues.push(issue(
                    LuaValidationIssueSeverityDto::Warning,
                    "dangerous_lua_api",
                    format!("Script uses {pattern}, which is unsafe or unsupported for card validation."),
                    Some(*line_no),
                    Some("Remove filesystem, module-loading, and shell APIs from card scripts."),
                ));
            }
        }
    }
    issues
}

fn check_common_typos(lines: &[(u32, String, String)]) -> Vec<LuaValidationIssueDto> {
    const TYPOS: [(&str, &str); 4] = [
        ("Duel.SpecialSummom", "Duel.SpecialSummon"),
        ("Duel.SelectMathchingCard", "Duel.SelectMatchingCard"),
        ("Effect.CreateEffct", "Effect.CreateEffect"),
        ("Card.IsRelateToEffct", "Card.IsRelateToEffect"),
    ];

    let mut issues = Vec::new();
    for (line_no, _raw, line) in lines {
        for (typo, correction) in TYPOS {
            if line.contains(typo) {
                issues.push(issue(
                    LuaValidationIssueSeverityDto::Error,
                    "common_api_typo",
                    format!("Possible API typo `{typo}`; did you mean `{correction}`?"),
                    Some(*line_no),
                    Some(format!("Replace `{typo}` with `{correction}`.")),
                ));
            }
        }
    }
    issues
}

fn first_line_containing(lines: &[(u32, String, String)], needle: &str) -> Option<u32> {
    lines
        .iter()
        .find_map(|(line_no, _raw, line)| line.contains(needle).then_some(*line_no))
}

fn issue(
    severity: LuaValidationIssueSeverityDto,
    code: impl Into<String>,
    message: impl Into<String>,
    line: Option<u32>,
    suggestion: Option<impl Into<String>>,
) -> LuaValidationIssueDto {
    LuaValidationIssueDto {
        severity,
        stage: LuaValidationLevelDto::Static,
        code: code.into(),
        message: message.into(),
        line,
        column: None,
        suggestion: suggestion.map(Into::into),
    }
}

fn dedupe_issues(issues: Vec<LuaValidationIssueDto>) -> Vec<LuaValidationIssueDto> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for issue in issues {
        let key = (
            issue.code.clone(),
            issue.message.clone(),
            issue.line,
            issue.column,
        );
        if seen.insert(key) {
            deduped.push(issue);
        }
    }
    deduped
}
```

- [ ] **Step 4: Run static checker tests**

Run:

```powershell
Push-Location src-tauri
cargo test application::script::static_checker
Pop-Location
```

Expected: PASS for static checker tests.

- [ ] **Step 5: Commit static checker**

Run:

```powershell
git add src-tauri/src/application/script/mod.rs src-tauri/src/application/script/static_checker.rs
git commit -m "feat: add lua static checker"
```

---

### Task 4: Source Resolver And Service

**Files:**
- Create: `src-tauri/src/application/script/source_resolver.rs`
- Create: `src-tauri/src/application/script/service.rs`
- Modify: `src-tauri/src/application/script/mod.rs`

- [ ] **Step 1: Register modules and write failing source resolver tests**

Modify `src-tauri/src/application/script/mod.rs`:

```rust
pub mod dto;
pub mod report;
pub mod source_resolver;
pub mod static_checker;
```

Create `src-tauri/src/application/script/source_resolver.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tempfile::tempdir;

    use super::*;
    use crate::application::script::dto::ValidateLuaScriptInput;
    use crate::bootstrap::AppState;
    use crate::domain::card::model::{CardEntity, Ot, PrimaryType};
    use crate::domain::common::time::now_utc;
    use crate::domain::pack::model::{PackKind, PackMetadata, PackOverview};
    use crate::domain::strings::model::PackStringsFile;
    use crate::domain::workspace::model::WorkspaceMeta;
    use crate::runtime::sessions::{PackSession, WorkspaceSession};

    #[test]
    fn draft_script_text_wins_without_saved_file() {
        let (_temp, state) = state_with_open_pack(false);
        let input = ValidateLuaScriptInput {
            workspace_id: "workspace-1".to_string(),
            pack_id: "pack-1".to_string(),
            card_id: "card-1".to_string(),
            script_text: Some("draft script".to_string()),
            levels: None,
        };

        let resolved = ScriptSourceResolver::new(&state).resolve(&input).unwrap();

        match resolved {
            ScriptSourceResolution::Ready(source) => {
                assert_eq!(source.card.code, 99999999);
                assert_eq!(source.script_text, "draft script");
                assert_eq!(source.source_kind, ScriptSourceKind::Draft);
                assert!(source.script_path.is_none());
            }
            other => panic!("expected ready source, got {other:#?}"),
        }
    }

    #[test]
    fn saved_script_is_read_from_pack_scripts_dir() {
        let (_temp, state) = state_with_open_pack(true);
        let input = ValidateLuaScriptInput {
            workspace_id: "workspace-1".to_string(),
            pack_id: "pack-1".to_string(),
            card_id: "card-1".to_string(),
            script_text: None,
            levels: None,
        };

        let resolved = ScriptSourceResolver::new(&state).resolve(&input).unwrap();

        match resolved {
            ScriptSourceResolution::Ready(source) => {
                assert!(source.script_text.contains("function s.initial_effect"));
                assert_eq!(source.source_kind, ScriptSourceKind::Saved);
                assert!(source.script_path.unwrap().ends_with("scripts/c99999999.lua"));
            }
            other => panic!("expected ready source, got {other:#?}"),
        }
    }

    #[test]
    fn missing_saved_script_returns_missing_resolution() {
        let (_temp, state) = state_with_open_pack(false);
        let input = ValidateLuaScriptInput {
            workspace_id: "workspace-1".to_string(),
            pack_id: "pack-1".to_string(),
            card_id: "card-1".to_string(),
            script_text: None,
            levels: None,
        };

        let resolved = ScriptSourceResolver::new(&state).resolve(&input).unwrap();

        match resolved {
            ScriptSourceResolution::MissingScript { card_code, script_path } => {
                assert_eq!(card_code, 99999999);
                assert!(script_path.ends_with("scripts/c99999999.lua"));
            }
            other => panic!("expected missing script, got {other:#?}"),
        }
    }

    fn state_with_open_pack(write_script: bool) -> (tempfile::TempDir, AppState) {
        let temp = tempdir().unwrap();
        let app_dir = temp.path().join("app-data");
        let workspace_path = temp.path().join("workspace");
        let pack_path = workspace_path.join("packs").join("pack-one");
        std::fs::create_dir_all(pack_path.join("scripts")).unwrap();
        if write_script {
            std::fs::write(
                pack_path.join("scripts").join("c99999999.lua"),
                "local s,id,o=GetID()\nfunction s.initial_effect(c)\nend\n",
            )
            .unwrap();
        }

        let state = AppState::new(app_dir).unwrap();
        let now = now_utc();
        let workspace_meta = WorkspaceMeta {
            id: "workspace-1".to_string(),
            name: "Workspace".to_string(),
            description: None,
            created_at: now,
            updated_at: now,
            pack_order: vec!["pack-1".to_string()],
            last_opened_pack_id: Some("pack-1".to_string()),
            open_pack_ids: vec!["pack-1".to_string()],
        };
        let pack_metadata = PackMetadata {
            id: "pack-1".to_string(),
            kind: PackKind::Custom,
            name: "Pack".to_string(),
            pack_code: None,
            author: "author".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            created_at: now,
            updated_at: now,
            display_language_order: vec!["en-US".to_string()],
            default_export_language: Some("en-US".to_string()),
        };
        let card = CardEntity {
            id: "card-1".to_string(),
            code: 99999999,
            alias: 0,
            setcodes: Vec::new(),
            ot: Ot::Custom,
            category: 0,
            primary_type: PrimaryType::Monster,
            texts: BTreeMap::new(),
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
        };
        let pack_session = PackSession {
            pack_id: "pack-1".to_string(),
            pack_path: pack_path.clone(),
            revision: 0,
            source_stamp: "test".to_string(),
            metadata: pack_metadata.clone(),
            cards: vec![card],
            strings: PackStringsFile::default(),
            asset_index: BTreeMap::new(),
            card_list_cache: Vec::new(),
        };
        let workspace_session = WorkspaceSession {
            workspace_path,
            meta: workspace_meta,
            pack_paths: BTreeMap::from([("pack-1".to_string(), pack_path)]),
            pack_overviews: BTreeMap::from([(
                "pack-1".to_string(),
                PackOverview {
                    id: "pack-1".to_string(),
                    kind: PackKind::Custom,
                    name: "Pack".to_string(),
                    author: "author".to_string(),
                    version: "1.0.0".to_string(),
                    card_count: 1,
                    updated_at: now,
                },
            )]),
            open_pack_ids: vec!["pack-1".to_string()],
            active_pack_id: Some("pack-1".to_string()),
        };
        {
            let mut sessions = state.sessions.write().unwrap();
            sessions.set_workspace(workspace_session);
            sessions.put_pack(pack_session);
        }
        (temp, state)
    }
}
```

- [ ] **Step 2: Run source resolver tests and verify they fail**

Run:

```powershell
Push-Location src-tauri
cargo test application::script::source_resolver
Pop-Location
```

Expected: FAIL because `ScriptSourceResolver`, `ScriptSourceResolution`, and `ScriptSourceKind` are missing.

- [ ] **Step 3: Implement source resolution**

Add this implementation above the test module in `src-tauri/src/application/script/source_resolver.rs`:

```rust
use std::path::PathBuf;

use crate::application::script::dto::ValidateLuaScriptInput;
use crate::bootstrap::AppState;
use crate::domain::card::model::CardEntity;
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::pack::model::PackKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptSourceKind {
    Draft,
    Saved,
}

#[derive(Debug, Clone)]
pub struct ResolvedScriptSource {
    pub card: CardEntity,
    pub script_text: String,
    pub source_kind: ScriptSourceKind,
    pub script_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum ScriptSourceResolution {
    Ready(ResolvedScriptSource),
    MissingScript {
        card_code: u32,
        script_path: PathBuf,
    },
    ReadFailed {
        card_code: u32,
        script_path: PathBuf,
        message: String,
    },
}

pub struct ScriptSourceResolver<'a> {
    state: &'a AppState,
}

impl<'a> ScriptSourceResolver<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub fn resolve(&self, input: &ValidateLuaScriptInput) -> AppResult<ScriptSourceResolution> {
        let snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            &input.workspace_id,
            &input.pack_id,
        )?;
        if snapshot.metadata.kind != PackKind::Custom {
            return Err(AppError::new(
                "script_validation.pack_not_custom",
                "Lua script validation is only available for custom packs",
            ));
        }
        let card = snapshot
            .cards
            .iter()
            .find(|card| card.id == input.card_id)
            .cloned()
            .ok_or_else(|| AppError::new("card.not_found", "card was not found"))?;

        if let Some(script_text) = &input.script_text {
            return Ok(ScriptSourceResolution::Ready(ResolvedScriptSource {
                card,
                script_text: script_text.clone(),
                source_kind: ScriptSourceKind::Draft,
                script_path: None,
            }));
        }

        let script_path =
            crate::domain::resource::path_rules::script_path(&snapshot.pack_path, card.code);
        if !script_path.exists() {
            return Ok(ScriptSourceResolution::MissingScript {
                card_code: card.code,
                script_path,
            });
        }

        match std::fs::read_to_string(&script_path) {
            Ok(script_text) => Ok(ScriptSourceResolution::Ready(ResolvedScriptSource {
                card,
                script_text,
                source_kind: ScriptSourceKind::Saved,
                script_path: Some(script_path),
            })),
            Err(source) => Ok(ScriptSourceResolution::ReadFailed {
                card_code: card.code,
                script_path,
                message: source.to_string(),
            }),
        }
    }
}
```

- [ ] **Step 4: Write failing service tests**

Modify `src-tauri/src/application/script/mod.rs`:

```rust
pub mod dto;
pub mod report;
pub mod service;
pub mod source_resolver;
pub mod static_checker;
```

Create `src-tauri/src/application/script/service.rs` with tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::script::dto::{
        LuaValidationLevelDto, LuaValidationStatusDto, ValidateLuaScriptInput,
    };

    #[test]
    fn normalize_levels_defaults_empty_and_missing_to_static() {
        assert_eq!(normalize_levels(None), vec![LuaValidationLevelDto::Static]);
        assert_eq!(normalize_levels(Some(Vec::new())), vec![LuaValidationLevelDto::Static]);
    }

    #[test]
    fn unsupported_stage_returns_inconclusive_report_stage() {
        let stages = run_requested_levels(
            "local s,id,o=GetID()\nfunction s.initial_effect(c)\nend",
            99999999,
            &[LuaValidationLevelDto::OcgcoreInit],
        );

        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0].status, LuaValidationStatusDto::Inconclusive);
        assert_eq!(stages[0].issues[0].code, "level_not_implemented");
    }

    #[test]
    fn static_stage_fails_for_missing_initial_effect() {
        let stages = run_requested_levels(
            "local s,id,o=GetID()\nfunction s.thop(e,tp)\nend",
            99999999,
            &[LuaValidationLevelDto::Static],
        );

        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0].status, LuaValidationStatusDto::Fail);
        assert!(stages[0]
            .issues
            .iter()
            .any(|issue| issue.code == "missing_initial_effect"));
    }

    #[test]
    fn missing_script_report_is_inconclusive() {
        let report = missing_script_report(99999999);

        assert_eq!(report.status, LuaValidationStatusDto::Inconclusive);
        assert_eq!(report.issues[0].code, "script_not_found");
    }
}
```

- [ ] **Step 5: Run service tests and verify they fail**

Run:

```powershell
Push-Location src-tauri
cargo test application::script::service
Pop-Location
```

Expected: FAIL because service functions are missing.

- [ ] **Step 6: Implement service orchestration**

Add this implementation above the test module in `src-tauri/src/application/script/service.rs`:

```rust
use std::time::Instant;

use crate::application::script::dto::{
    LuaValidationIssueDto, LuaValidationIssueSeverityDto, LuaValidationLevelDto,
    LuaValidationReportDto, LuaValidationStageResultDto, LuaValidationStatusDto,
    ValidateLuaScriptInput,
};
use crate::application::script::report::{
    assemble_report, single_issue_report, stage_status_from_issues, unsupported_stage_result,
};
use crate::application::script::source_resolver::{
    ScriptSourceResolution, ScriptSourceResolver,
};
use crate::application::script::static_checker::StaticChecker;
use crate::bootstrap::AppState;
use crate::domain::common::error::AppResult;

pub struct ScriptValidationService<'a> {
    state: &'a AppState,
}

impl<'a> ScriptValidationService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub fn validate(&self, input: ValidateLuaScriptInput) -> AppResult<LuaValidationReportDto> {
        let levels = normalize_levels(input.levels.clone());
        let resolved = ScriptSourceResolver::new(self.state).resolve(&input)?;
        match resolved {
            ScriptSourceResolution::Ready(source) => Ok(assemble_report(run_requested_levels(
                &source.script_text,
                source.card.code,
                &levels,
            ))),
            ScriptSourceResolution::MissingScript { card_code, .. } => {
                Ok(missing_script_report(card_code))
            }
            ScriptSourceResolution::ReadFailed {
                card_code,
                message,
                ..
            } => Ok(read_failed_report(card_code, message)),
        }
    }
}

pub(crate) fn normalize_levels(
    levels: Option<Vec<LuaValidationLevelDto>>,
) -> Vec<LuaValidationLevelDto> {
    match levels {
        Some(values) if !values.is_empty() => values,
        _ => vec![LuaValidationLevelDto::Static],
    }
}

pub(crate) fn run_requested_levels(
    script_text: &str,
    card_code: u32,
    levels: &[LuaValidationLevelDto],
) -> Vec<LuaValidationStageResultDto> {
    levels
        .iter()
        .copied()
        .map(|level| match level {
            LuaValidationLevelDto::Static => run_static_stage(script_text, card_code),
            LuaValidationLevelDto::OcgcoreInit
            | LuaValidationLevelDto::Smoke
            | LuaValidationLevelDto::Scenario => unsupported_stage_result(level, 0),
        })
        .collect()
}

fn run_static_stage(script_text: &str, card_code: u32) -> LuaValidationStageResultDto {
    let started = Instant::now();
    let issues = StaticChecker::check(script_text, card_code);
    LuaValidationStageResultDto {
        stage: LuaValidationLevelDto::Static,
        status: stage_status_from_issues(&issues),
        duration_ms: started.elapsed().as_millis() as u64,
        issues,
        log: Vec::new(),
    }
}

pub(crate) fn missing_script_report(card_code: u32) -> LuaValidationReportDto {
    single_issue_report(LuaValidationIssueDto {
        severity: LuaValidationIssueSeverityDto::Info,
        stage: LuaValidationLevelDto::Static,
        code: "script_not_found".to_string(),
        message: format!("No saved Lua script was found for card code {card_code}."),
        line: None,
        column: None,
        suggestion: Some("Create or import a script for this card before validating the saved script.".to_string()),
    })
}

fn read_failed_report(card_code: u32, message: String) -> LuaValidationReportDto {
    single_issue_report(LuaValidationIssueDto {
        severity: LuaValidationIssueSeverityDto::Info,
        stage: LuaValidationLevelDto::Static,
        code: "script_read_failed".to_string(),
        message: format!("The saved Lua script for card code {card_code} could not be read: {message}"),
        line: None,
        column: None,
        suggestion: Some("Check the script file permissions and try again.".to_string()),
    })
}
```

- [ ] **Step 7: Run resolver and service tests**

Run:

```powershell
Push-Location src-tauri
cargo test application::script::source_resolver
cargo test application::script::service
Pop-Location
```

Expected: PASS for resolver and service tests.

- [ ] **Step 8: Commit resolver and service**

Run:

```powershell
git add src-tauri/src/application/script/mod.rs src-tauri/src/application/script/source_resolver.rs src-tauri/src/application/script/service.rs
git commit -m "feat: resolve lua script validation sources"
```

---

### Task 5: Tauri Command Registration

**Files:**
- Modify: `src-tauri/src/presentation/commands/app_commands.rs`
- Modify: `src-tauri/src/tauri_commands.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Add the presentation command function**

In `src-tauri/src/presentation/commands/app_commands.rs`, add imports near the other application DTO imports:

```rust
use crate::application::script::dto::{LuaValidationReportDto, ValidateLuaScriptInput};
```

Add this function near the other command adapter functions:

```rust
pub fn validate_lua_script(
    state: &AppState,
    input: ValidateLuaScriptInput,
) -> AppResult<LuaValidationReportDto> {
    crate::application::script::service::ScriptValidationService::new(state).validate(input)
}
```

- [ ] **Step 2: Add the Tauri command wrapper**

In `src-tauri/src/tauri_commands.rs`, add an import near the other application DTO imports:

```rust
use crate::application::script::dto::{LuaValidationReportDto, ValidateLuaScriptInput};
```

Add this command near the resource/script commands:

```rust
#[tauri::command]
pub fn validate_lua_script(
    state: State<'_, AppState>,
    input: ValidateLuaScriptInput,
) -> CommandResult<LuaValidationReportDto> {
    crate::presentation::commands::app_commands::validate_lua_script(&state, input)
}
```

- [ ] **Step 3: Register the command in Tauri main**

In `src-tauri/src/main.rs`, add this entry to the `tauri::generate_handler![...]` list near `open_script_external`:

```rust
tauri_commands::validate_lua_script,
```

- [ ] **Step 4: Run backend check**

Run:

```powershell
Push-Location src-tauri
cargo check
Pop-Location
```

Expected: PASS.

- [ ] **Step 5: Commit command registration**

Run:

```powershell
git add src-tauri/src/presentation/commands/app_commands.rs src-tauri/src/tauri_commands.rs src-tauri/src/main.rs
git commit -m "feat: register lua script validation command"
```

---

### Task 6: Current Documentation Updates

**Files:**
- Modify: `docs/functional_spec.md`
- Modify: `docs/system_architecture.md`
- Modify: `docs/code_structure_api.md`

- [ ] **Step 1: Update functional spec**

In `docs/functional_spec.md`, under `## 资源管理`, add this bullet:

```markdown
- 自定义包中的卡片脚本支持静态验证：后端可读取当前保存的 `scripts/c{code}.lua`，或验证调用方传入的未保存 `scriptText` 草稿，并返回结构化状态、阶段结果、issues 和 limitations。当前实现只做 deterministic static check，不运行 ocgcore，也不证明效果语义正确。
```

- [ ] **Step 2: Update system architecture**

In `docs/system_architecture.md`, after `## 数据流`, add this subsection:

```markdown
## Lua 脚本验证架构

Lua 脚本验证是后端 application 层能力，入口为 `validate_lua_script`。前端只通过 `src/shared/api/scriptApi.ts` 提交 workspace、pack、card 和可选 `scriptText`，不直接读取脚本文件或实现验证规则。

当前实现的第一阶段包含 `ScriptValidationService`、`ScriptSourceResolver`、`StaticChecker` 和报告聚合。它只做静态检查：可发现缺少 `initial_effect`、未定义 callback、危险 Lua API 和少量高置信拼写错误；不会启动 ocgcore，也不会证明脚本能在真实 duel 中加载或效果语义正确。
```

- [ ] **Step 3: Update code structure/API docs**

In `docs/code_structure_api.md`, update the frontend API wrapper list with:

```markdown
- `scriptApi`：验证 custom pack 中单张卡的 Lua 脚本，支持保存脚本和未保存 `scriptText` 草稿，当前阶段返回静态检查报告。
```

Update the Tauri command surface list with:

```markdown
- Script Validation：`validate_lua_script`
```

Update the contracts section with:

```markdown
- Lua script validation input、issue、stage result 和 report types live in `script.ts`.
```

- [ ] **Step 4: Review docs do not overclaim future slices**

Run:

```powershell
rg "ocgcore.*(运行|执行|helper|sidecar)|Agent.*validate_lua_script|验证入口|UI.*验证" docs/functional_spec.md docs/system_architecture.md docs/code_structure_api.md
```

Expected: Any matches must describe them as not implemented or not be present. The docs must not claim ocgcore helper, Agent tool, or UI validation panel exists.

- [ ] **Step 5: Commit docs**

Run:

```powershell
git add docs/functional_spec.md docs/system_architecture.md docs/code_structure_api.md
git commit -m "docs: document lua static validation"
```

---

### Task 7: Full Verification

**Files:**
- No new files.

- [ ] **Step 1: Run frontend tests**

Run:

```powershell
npm test -- src/shared/api/scriptApi.test.ts
```

Expected: PASS.

- [ ] **Step 2: Run frontend typecheck**

Run:

```powershell
npm run typecheck
```

Expected: PASS.

- [ ] **Step 3: Run backend tests**

Run:

```powershell
Push-Location src-tauri
cargo test application::script
Pop-Location
```

Expected: PASS.

- [ ] **Step 4: Run backend check**

Run:

```powershell
Push-Location src-tauri
cargo check
Pop-Location
```

Expected: PASS.

- [ ] **Step 5: Inspect git status**

Run:

```powershell
git status --short
```

Expected: No uncommitted implementation changes. Existing unrelated user changes, if any, must remain untouched.

- [ ] **Step 6: Final implementation review**

Review these items before reporting completion:

- `scriptApi.validateLuaScript` is the only frontend command entry for this feature.
- `validate_lua_script` returns normal `LuaValidationReportDto` for missing scripts, read failures, static errors, and unsupported levels.
- `AppError` is used only for boundary/session failures such as workspace mismatch, pack not open, non-custom pack, or card not found.
- Static checker errors are conservative; uncertain patterns are warnings.
- Docs describe only the implemented static validation slice.

Run:

```powershell
git log --oneline -5
```

Expected: Recent commits should show the frontend wrapper, Rust report/DTO, static checker, source resolver/service, command registration, and docs commits.
