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
