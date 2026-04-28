use crate::domain::card::code::{CodeValidationContext, validate_card_code};
use crate::domain::card::model::{CardEntity, CardUpdateInput, MonsterFlag, PrimaryType, QMARK};
use crate::domain::common::issue::{IssueLevel, ValidationIssue, ValidationTarget};

pub fn validate_card_structure(card: &CardEntity) -> Vec<ValidationIssue> {
    validate_card_update_input(&CardUpdateInput {
        code: card.code,
        alias: card.alias,
        setcode: card.setcode,
        ot: card.ot.clone(),
        category: card.category,
        primary_type: card.primary_type.clone(),
        texts: card.texts.clone(),
        monster_flags: card.monster_flags.clone(),
        atk: card.atk,
        def: card.def,
        race: card.race.clone(),
        attribute: card.attribute.clone(),
        level: card.level,
        pendulum: card.pendulum.clone(),
        link: card.link.clone(),
        spell_subtype: card.spell_subtype.clone(),
        trap_subtype: card.trap_subtype.clone(),
    })
}

pub fn validate_card_update_input(input: &CardUpdateInput) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let target = ValidationTarget::new("card");

    if input.code == 0 {
        issues.push(ValidationIssue::error(
            "card.code_required",
            target.clone().with_field("code"),
        ));
    }

    if input.texts.is_empty() {
        issues.push(ValidationIssue::error(
            "card.texts_required",
            target.clone().with_field("texts"),
        ));
    }

    let has_name = input
        .texts
        .values()
        .any(|texts| !texts.name.trim().is_empty());
    if !has_name {
        issues.push(ValidationIssue::error(
            "card.name_required",
            target.clone().with_field("texts"),
        ));
    }

    let stats_valid =
        |value: Option<i32>| value.is_none_or(|number| number >= 0 || number == QMARK);
    if !stats_valid(input.atk) {
        issues.push(ValidationIssue::error(
            "card.atk_invalid",
            target.clone().with_field("atk"),
        ));
    }
    if !stats_valid(input.def) {
        issues.push(ValidationIssue::error(
            "card.def_invalid",
            target.clone().with_field("def"),
        ));
    }

    match input.primary_type {
        PrimaryType::Monster => {
            if input.monster_flags.as_ref().is_none_or(Vec::is_empty) {
                issues.push(ValidationIssue::error(
                    "card.monster_flags_required",
                    target.clone().with_field("monster_flags"),
                ));
            }

            let flags = input.monster_flags.clone().unwrap_or_default();
            let is_link = flags.contains(&MonsterFlag::Link);
            let is_pendulum = flags.contains(&MonsterFlag::Pendulum);

            if input.atk.is_none() {
                issues.push(ValidationIssue::error(
                    "card.monster_atk_required",
                    target.clone().with_field("atk"),
                ));
            }

            if input.race.is_none() {
                issues.push(ValidationIssue::error(
                    "card.monster_race_required",
                    target.clone().with_field("race"),
                ));
            }

            if input.attribute.is_none() {
                issues.push(ValidationIssue::error(
                    "card.monster_attribute_required",
                    target.clone().with_field("attribute"),
                ));
            }

            if is_link {
                if input.link.is_none() {
                    issues.push(ValidationIssue::error(
                        "card.link_markers_required",
                        target.clone().with_field("link"),
                    ));
                }
                if input.def.is_some() {
                    issues.push(ValidationIssue::error(
                        "card.link_monster_def_forbidden",
                        target.clone().with_field("def"),
                    ));
                }
            } else {
                if input.level.is_none() {
                    issues.push(ValidationIssue::error(
                        "card.level_required",
                        target.clone().with_field("level"),
                    ));
                }
            }

            if !is_pendulum && input.pendulum.is_some() {
                issues.push(ValidationIssue::error(
                    "card.pendulum_forbidden_without_flag",
                    target.clone().with_field("pendulum"),
                ));
            }
        }
        PrimaryType::Spell => {
            if input.spell_subtype.is_none() {
                issues.push(ValidationIssue::error(
                    "card.spell_subtype_required",
                    target.clone().with_field("spell_subtype"),
                ));
            }
        }
        PrimaryType::Trap => {
            if input.trap_subtype.is_none() {
                issues.push(ValidationIssue::error(
                    "card.trap_subtype_required",
                    target.clone().with_field("trap_subtype"),
                ));
            }
        }
    }

    issues
}

pub fn collect_card_warnings(
    card: &CardEntity,
    code_ctx: &CodeValidationContext,
) -> Vec<ValidationIssue> {
    validate_card_code(card.code, code_ctx)
        .into_iter()
        .filter(|issue| issue.level == IssueLevel::Warning)
        .collect()
}
