use std::collections::BTreeSet;

use crate::domain::common::issue::{ValidationIssue, ValidationTarget};

#[derive(Debug, Clone)]
pub struct CodePolicy {
    pub reserved_max: u32,
    pub recommended_min: u32,
    pub recommended_max: u32,
    pub hard_max: u32,
    pub min_gap: u32,
}

#[derive(Debug, Clone)]
pub struct CodeValidationContext {
    pub policy: CodePolicy,
    pub current_pack_codes: BTreeSet<u32>,
    pub other_custom_codes: BTreeSet<u32>,
    pub standard_codes: BTreeSet<u32>,
}

pub fn validate_card_code(code: u32, ctx: &CodeValidationContext) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let target = ValidationTarget::new("card").with_field("code");

    if code == 0 {
        issues.push(ValidationIssue::error("card.code_required", target.clone()));
        return issues;
    }

    if code <= ctx.policy.reserved_max {
        issues.push(
            ValidationIssue::error("card.code_reserved_range", target.clone())
                .with_param("reserved_max", ctx.policy.reserved_max),
        );
    }

    if code > ctx.policy.hard_max {
        issues.push(
            ValidationIssue::error("card.code_hard_max_exceeded", target.clone())
                .with_param("hard_max", ctx.policy.hard_max),
        );
    }

    if ctx.current_pack_codes.contains(&code) {
        issues.push(ValidationIssue::error(
            "card.code_conflicts_with_current_pack_card",
            target.clone(),
        ));
    }

    if ctx.other_custom_codes.contains(&code) {
        issues.push(ValidationIssue::warning(
            "card.code_conflicts_with_workspace_custom_card",
            target.clone(),
        ));
    }

    if ctx.standard_codes.contains(&code) {
        issues.push(ValidationIssue::warning(
            "card.code_conflicts_with_standard_card",
            target.clone(),
        ));
    }

    if code < ctx.policy.recommended_min || code > ctx.policy.recommended_max {
        issues.push(
            ValidationIssue::warning("card.code_outside_recommended_range", target.clone())
                .with_param("recommended_min", ctx.policy.recommended_min)
                .with_param("recommended_max", ctx.policy.recommended_max),
        );
    }

    let nearest_gap = ctx
        .current_pack_codes
        .iter()
        .chain(ctx.other_custom_codes.iter())
        .chain(ctx.standard_codes.iter())
        .filter_map(|used| used.abs_diff(code).checked_sub(0))
        .min();

    if let Some(nearest_gap) = nearest_gap {
        if nearest_gap < ctx.policy.min_gap {
            issues.push(
                ValidationIssue::warning("card.code_gap_too_small", target)
                    .with_param("nearest_gap", nearest_gap)
                    .with_param("min_gap", ctx.policy.min_gap),
            );
        }
    }

    issues
}

pub fn suggest_next_code(
    ctx: &CodeValidationContext,
    preferred_start: Option<u32>,
) -> Option<u32> {
    let start = preferred_start
        .unwrap_or(ctx.policy.recommended_min)
        .max(ctx.policy.recommended_min);

    (start..=ctx.policy.recommended_max).find(|candidate| {
        if ctx.current_pack_codes.contains(candidate)
            || ctx.other_custom_codes.contains(candidate)
            || ctx.standard_codes.contains(candidate)
        {
            return false;
        }

        ctx.current_pack_codes
            .iter()
            .chain(ctx.other_custom_codes.iter())
            .chain(ctx.standard_codes.iter())
            .all(|used| used.abs_diff(*candidate) >= ctx.policy.min_gap)
    })
}
