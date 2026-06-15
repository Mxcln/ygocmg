use std::path::PathBuf;
use std::time::Instant;

use crate::application::script::dto::{
    LuaValidationIssueDto, LuaValidationIssueSeverityDto, LuaValidationLevelDto,
    LuaValidationReportDto, LuaValidationStageResultDto, ValidateLuaScriptInput,
};
use crate::application::script::ocgcore_init::{
    OcgcoreInitHelper, skipped_due_to_static_errors, validate_ocgcore_init,
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
        _ => vec![
            LuaValidationLevelDto::Static,
            LuaValidationLevelDto::OcgcoreInit,
        ],
    }
}

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

fn default_core_script_root() -> PathBuf {
    let from_src_tauri = PathBuf::from("../third_party/ygopro-scripts");
    if from_src_tauri.exists() {
        return std::fs::canonicalize(&from_src_tauri).unwrap_or(from_src_tauri);
    }
    let from_repo_root = PathBuf::from("third_party/ygopro-scripts");
    std::fs::canonicalize(&from_repo_root).unwrap_or(from_repo_root)
}

#[cfg(test)]
pub(crate) fn run_requested_levels_for_test(
    script_text: &str,
    card_code: u32,
    levels: &[LuaValidationLevelDto],
) -> Vec<LuaValidationStageResultDto> {
    use crate::application::script::source_resolver::ScriptSourceKind;
    use crate::domain::card::model::{CardEntity, Ot, PrimaryType};
    use crate::domain::common::time::now_utc;

    struct InconclusiveHelper;

    impl OcgcoreInitHelper for InconclusiveHelper {
        fn run(
            &self,
            _input: &crate::infrastructure::ocgcore_validator::input::HelperInput,
        ) -> AppResult<LuaValidationStageResultDto> {
            Ok(
                crate::infrastructure::ocgcore_validator::helper_client::failure_stage(
                    "helper_not_found",
                    "test helper is not available",
                    None,
                    Vec::new(),
                ),
            )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::script::dto::{LuaValidationLevelDto, LuaValidationStatusDto};

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

    #[test]
    fn static_stage_fails_for_missing_initial_effect() {
        let stages = run_requested_levels_for_test(
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
