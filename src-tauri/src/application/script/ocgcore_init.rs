use std::path::PathBuf;

use crate::application::script::dto::{
    LuaValidationIssueDto, LuaValidationIssueSeverityDto, LuaValidationLevelDto,
    LuaValidationStageResultDto, LuaValidationStatusDto,
};
use crate::application::script::source_resolver::ResolvedScriptSource;
use crate::domain::common::error::AppResult;
use crate::infrastructure::ocgcore_validator::OcgcoreValidatorHelperClient;
use crate::infrastructure::ocgcore_validator::input::HelperInput;

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
            message: "ocgcore_init was skipped because static validation found errors."
                .to_string(),
            line: None,
            column: None,
            suggestion: Some("Fix static validation errors, then run validation again.".to_string()),
        }],
        log: Vec::new(),
    }
}

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
