use std::time::Instant;

use crate::application::script::dto::{
    LuaValidationIssueDto, LuaValidationIssueSeverityDto, LuaValidationLevelDto,
    LuaValidationReportDto, LuaValidationStageResultDto, ValidateLuaScriptInput,
};
use crate::application::script::report::{
    assemble_report, single_issue_report, stage_status_from_issues, unsupported_stage_result,
};
use crate::application::script::source_resolver::{ScriptSourceResolution, ScriptSourceResolver};
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
                card_code, message, ..
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
        suggestion: Some(
            "Create or import a script for this card before validating the saved script."
                .to_string(),
        ),
    })
}

fn read_failed_report(card_code: u32, message: String) -> LuaValidationReportDto {
    single_issue_report(LuaValidationIssueDto {
        severity: LuaValidationIssueSeverityDto::Info,
        stage: LuaValidationLevelDto::Static,
        code: "script_read_failed".to_string(),
        message: format!(
            "The saved Lua script for card code {card_code} could not be read: {message}"
        ),
        line: None,
        column: None,
        suggestion: Some("Check the script file permissions and try again.".to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::script::dto::{LuaValidationLevelDto, LuaValidationStatusDto};

    #[test]
    fn normalize_levels_defaults_empty_and_missing_to_static() {
        assert_eq!(normalize_levels(None), vec![LuaValidationLevelDto::Static]);
        assert_eq!(
            normalize_levels(Some(Vec::new())),
            vec![LuaValidationLevelDto::Static]
        );
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
        assert!(
            stages[0]
                .issues
                .iter()
                .any(|issue| issue.code == "missing_initial_effect")
        );
    }

    #[test]
    fn missing_script_report_is_inconclusive() {
        let report = missing_script_report(99999999);

        assert_eq!(report.status, LuaValidationStatusDto::Inconclusive);
        assert_eq!(report.issues[0].code, "script_not_found");
    }

    #[test]
    fn read_failed_report_is_inconclusive() {
        let report = read_failed_report(99999999, "access denied".to_string());

        assert_eq!(report.status, LuaValidationStatusDto::Inconclusive);
        assert_eq!(report.issues[0].code, "script_read_failed");
    }
}
