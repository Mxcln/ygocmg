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
        .map(|stage| level_label(stage.stage))
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
            .any(|stage| stage.status == LuaValidationStatusDto::Inconclusive)
    {
        LuaValidationStatusDto::Inconclusive
    } else {
        LuaValidationStatusDto::Pass
    }
}

fn confidence_for(
    status: LuaValidationStatusDto,
    _stages: &[LuaValidationStageResultDto],
    _issues: &[LuaValidationIssueDto],
) -> LuaValidationConfidenceDto {
    match status {
        LuaValidationStatusDto::Fail => LuaValidationConfidenceDto::High,
        LuaValidationStatusDto::Warning => LuaValidationConfidenceDto::Medium,
        LuaValidationStatusDto::Pass => LuaValidationConfidenceDto::High,
        LuaValidationStatusDto::Inconclusive => LuaValidationConfidenceDto::Low,
    }
}

fn summary_for(status: LuaValidationStatusDto, issue_count: usize) -> String {
    match status {
        LuaValidationStatusDto::Pass => "Static validation passed.".to_string(),
        LuaValidationStatusDto::Warning => {
            format!(
                "Static validation completed with {issue_count} warning or informational issue(s)."
            )
        }
        LuaValidationStatusDto::Fail => {
            format!("Static validation failed with {issue_count} issue(s).")
        }
        LuaValidationStatusDto::Inconclusive => {
            "Lua script validation is inconclusive for the requested input.".to_string()
        }
    }
}

fn level_label(level: LuaValidationLevelDto) -> &'static str {
    match level {
        LuaValidationLevelDto::Static => "static",
        LuaValidationLevelDto::OcgcoreInit => "ocgcore_init",
        LuaValidationLevelDto::Smoke => "smoke",
        LuaValidationLevelDto::Scenario => "scenario",
    }
}

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
        assert_eq!(
            report.confidence,
            crate::application::script::dto::LuaValidationConfidenceDto::High
        );
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
        assert_eq!(
            report.confidence,
            crate::application::script::dto::LuaValidationConfidenceDto::Medium
        );
        assert_eq!(report.issues, vec![issue]);
    }

    #[test]
    fn assemble_report_keeps_high_confidence_when_static_failure_dominates_unsupported_stage() {
        let issue = LuaValidationIssueDto {
            severity: LuaValidationIssueSeverityDto::Error,
            stage: LuaValidationLevelDto::Static,
            code: "missing_initial_effect".to_string(),
            message: "Lua script does not define initial_effect(c).".to_string(),
            line: None,
            column: None,
            suggestion: Some("Define function s.initial_effect(c).".to_string()),
        };

        let report = assemble_report(vec![
            stage(LuaValidationStatusDto::Fail, vec![issue]),
            unsupported_stage_result(LuaValidationLevelDto::OcgcoreInit, 1),
        ]);

        assert_eq!(report.status, LuaValidationStatusDto::Fail);
        assert_eq!(
            report.confidence,
            crate::application::script::dto::LuaValidationConfidenceDto::High
        );
    }

    #[test]
    fn assemble_report_keeps_medium_confidence_when_static_warning_dominates_unsupported_stage() {
        let issue = LuaValidationIssueDto {
            severity: LuaValidationIssueSeverityDto::Warning,
            stage: LuaValidationLevelDto::Static,
            code: "dangerous_lua_api".to_string(),
            message: "Script uses os.execute.".to_string(),
            line: Some(4),
            column: None,
            suggestion: Some("Remove shell execution from card scripts.".to_string()),
        };

        let report = assemble_report(vec![
            stage(LuaValidationStatusDto::Warning, vec![issue]),
            unsupported_stage_result(LuaValidationLevelDto::OcgcoreInit, 1),
        ]);

        assert_eq!(report.status, LuaValidationStatusDto::Warning);
        assert_eq!(
            report.confidence,
            crate::application::script::dto::LuaValidationConfidenceDto::Medium
        );
    }

    #[test]
    fn unsupported_stage_result_is_inconclusive() {
        let result = unsupported_stage_result(LuaValidationLevelDto::OcgcoreInit, 0);

        assert_eq!(result.status, LuaValidationStatusDto::Inconclusive);
        assert_eq!(result.issues[0].code, "level_not_implemented");
        assert_eq!(result.issues[0].stage, LuaValidationLevelDto::OcgcoreInit);
    }

    #[test]
    fn assemble_report_is_inconclusive_when_any_requested_stage_is_inconclusive() {
        let report = assemble_report(vec![
            stage(LuaValidationStatusDto::Pass, Vec::new()),
            unsupported_stage_result(LuaValidationLevelDto::OcgcoreInit, 1),
        ]);

        assert_eq!(report.status, LuaValidationStatusDto::Inconclusive);
        assert_eq!(
            report.confidence,
            crate::application::script::dto::LuaValidationConfidenceDto::Low
        );
        assert!(report
            .limitations
            .iter()
            .any(|limitation| limitation.contains("ocgcore_init")));
        assert!(!report
            .limitations
            .iter()
            .any(|limitation| limitation.contains("OcgcoreInit")));
    }
}
