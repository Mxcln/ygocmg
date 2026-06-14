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
            &lines, &functions, card_code, "SetCost",
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
            &lines, &functions, card_code,
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
        for reference in direct_setter_callback_references(line, setter, card_code) {
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

fn direct_setter_callback_references(line: &str, setter: &str, card_code: u32) -> Vec<String> {
    let marker = format!("{setter}(");
    let mut references = Vec::new();
    let mut start = 0;

    while let Some(offset) = line[start..].find(&marker) {
        let argument_start = start + offset + marker.len();
        let argument = line[argument_start..].trim_start();
        if let Some(reference) = direct_callback_reference_at_start(argument, card_code) {
            references.push(reference);
        }
        start = argument_start;
    }

    references
}

fn direct_callback_reference_at_start(value: &str, card_code: u32) -> Option<String> {
    direct_prefix_reference_at_start(value, "s.")
        .or_else(|| direct_prefix_reference_at_start(value, &format!("c{card_code}.")))
}

fn direct_prefix_reference_at_start(value: &str, prefix: &str) -> Option<String> {
    let rest = value.strip_prefix(prefix)?;
    let name = rest
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>();
    if name.is_empty() {
        return None;
    }

    let after_name = &rest[name.len()..];
    let next = after_name.chars().next();
    if matches!(next, None | Some(')') | Some(','))
        || next.is_some_and(|ch| ch.is_ascii_whitespace())
    {
        Some(format!("{prefix}{name}"))
    } else {
        None
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
            || !contains_callback_taking_call(line)
        {
            continue;
        }
        for reference in unindexed_callback_references(line, card_code) {
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

fn contains_callback_taking_call(line: &str) -> bool {
    const CALLS: [&str; 7] = [
        "MatchingCard(",
        "MatchingGroup(",
        "SelectMatchingCard(",
        "IsExistingMatchingCard(",
        "CheckReleaseGroup(",
        "ReleaseGroupCost(",
        "Filter(",
    ];

    CALLS.iter().any(|call| line.contains(call))
}

fn unindexed_callback_references(line: &str, card_code: u32) -> Vec<String> {
    let mut references = Vec::new();
    collect_unindexed_prefix_references(line, "s.", &mut references);
    collect_unindexed_prefix_references(line, &format!("c{card_code}."), &mut references);
    references
}

fn collect_unindexed_prefix_references(line: &str, prefix: &str, references: &mut Vec<String>) {
    let mut start = 0;
    while let Some(offset) = line[start..].find(prefix) {
        let absolute = start + offset;
        let after_prefix = absolute + prefix.len();
        let name = line[after_prefix..]
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .collect::<String>();
        if !name.is_empty() {
            let after_name = after_prefix + name.len();
            if !line[after_name..].starts_with('[') {
                references.push(format!("{prefix}{name}"));
            }
        }
        start = after_prefix + name.len();
    }
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
            target_refs.extend(direct_setter_callback_references(
                line,
                "SetTarget",
                card_code,
            ));
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
        let issues =
            StaticChecker::check("local s,id,o=GetID()\nfunction s.thop(e,tp)\nend", 99999999);

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
    fn aux_target_bool_function_data_field_is_not_an_undefined_callback() {
        let script = r#"
local s,id,o=GetID()
s.listed_names={12345678}
function s.initial_effect(c)
  local e1=Effect.CreateEffect(c)
  e1:SetTarget(aux.TargetBoolFunction(Card.IsCode,s.listed_names[1]))
  c:RegisterEffect(e1)
end
"#;

        let issues = StaticChecker::check(script, 99999999);

        assert!(
            !issues.iter().any(|issue| {
                issue.code == "undefined_effect_callback"
                    || issue.code == "undefined_script_function_reference"
            }),
            "{issues:#?}"
        );
    }

    #[test]
    fn duel_hint_data_field_is_not_an_undefined_script_function_reference() {
        let script = r#"
local s,id,o=GetID()
s.listed_names={12345678}
function s.initial_effect(c)
  Duel.Hint(HINT_CARD,0,s.listed_names[1])
end
"#;

        let issues = StaticChecker::check(script, 99999999);

        assert!(
            !issues
                .iter()
                .any(|issue| issue.code == "undefined_script_function_reference"),
            "{issues:#?}"
        );
    }

    #[test]
    fn assignment_style_functions_are_collected_for_setter_callbacks() {
        let script = r#"
local s,id,o=GetID()
s.initial_effect=function(c)
  local e1=Effect.CreateEffect(c)
  e1:SetOperation(s.thop)
  c:RegisterEffect(e1)
end
s.thop=function(e,tp,eg,ep,ev,re,r,rp)
end
"#;

        let issues = StaticChecker::check(script, 99999999);

        assert!(
            !issues
                .iter()
                .any(|issue| issue.code == "undefined_effect_callback"),
            "{issues:#?}"
        );
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

    #[test]
    fn common_api_typo_is_an_error() {
        let script = r#"
local s,id,o=GetID()
function s.initial_effect(c)
  Duel.SpecialSummom(tp,12345678,0,tp,tp,false,false,POS_FACEUP)
end
"#;

        let issues = StaticChecker::check(script, 99999999);

        assert!(issues.iter().any(|issue| {
            issue.code == "common_api_typo"
                && issue.severity == LuaValidationIssueSeverityDto::Error
                && issue.message.contains("Duel.SpecialSummom")
                && issue.line == Some(4)
        }));
    }
}
