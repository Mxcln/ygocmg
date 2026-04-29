use std::collections::BTreeSet;

use crate::domain::common::ids::LanguageCode;
use crate::domain::common::issue::{ValidationIssue, ValidationTarget};
use crate::domain::language::model::{
    LEGACY_DEFAULT_LANGUAGE, TextLanguageKind, TextLanguageProfile,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageValidationContext {
    UserAuthored,
    LegacyCompatibility,
}

pub fn normalize_language_id(input: &str) -> LanguageCode {
    input.trim().to_string()
}

pub fn is_legacy_default_language_id(id: &str) -> bool {
    id == LEGACY_DEFAULT_LANGUAGE
}

pub fn builtin_text_language_catalog() -> Vec<TextLanguageProfile> {
    [
        ("zh-CN", "Simplified Chinese"),
        ("en-US", "English"),
        ("ja-JP", "Japanese"),
        ("ko-KR", "Korean"),
        ("es-ES", "Spanish"),
    ]
    .into_iter()
    .map(|(id, label)| TextLanguageProfile {
        id: id.to_string(),
        label: label.to_string(),
        kind: TextLanguageKind::Builtin,
        hidden: false,
        last_used_at: None,
    })
    .collect()
}

pub fn default_text_language_catalog() -> Vec<TextLanguageProfile> {
    builtin_text_language_catalog()
}

pub fn merge_missing_builtin_languages(
    catalog: &[TextLanguageProfile],
) -> Vec<TextLanguageProfile> {
    let mut merged = Vec::new();
    let builtins = builtin_text_language_catalog();
    let builtin_ids = builtins
        .iter()
        .map(|profile| profile.id.clone())
        .collect::<BTreeSet<_>>();

    for builtin in builtins {
        match catalog.iter().find(|profile| profile.id == builtin.id) {
            Some(existing) => {
                let mut next = existing.clone();
                next.kind = TextLanguageKind::Builtin;
                next.hidden = false;
                if next.label.trim().is_empty() {
                    next.label = builtin.label;
                }
                merged.push(next);
            }
            None => merged.push(builtin),
        }
    }

    for profile in catalog {
        if builtin_ids.contains(&profile.id) && matches!(profile.kind, TextLanguageKind::Builtin) {
            continue;
        }
        merged.push(profile.clone());
    }

    merged
}

pub fn normalize_text_language_catalog(
    catalog: &[TextLanguageProfile],
) -> Vec<TextLanguageProfile> {
    let mut normalized = Vec::new();
    for profile in catalog {
        let id = normalize_language_id(&profile.id);
        if id.is_empty() {
            normalized.push(profile.clone());
            continue;
        }
        let mut next = profile.clone();
        next.id = canonical_builtin_id(&id).unwrap_or(id);
        next.label = next.label.trim().to_string();
        normalized.push(next);
    }
    merge_missing_builtin_languages(&normalized)
}

pub fn validate_language_id(
    id: &str,
    context: LanguageValidationContext,
    scope: &str,
    field: &str,
    code_prefix: &str,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let normalized = normalize_language_id(id);
    let target = ValidationTarget::new(scope).with_field(field);

    if normalized.is_empty() {
        issues.push(ValidationIssue::error(
            format!("{code_prefix}_required"),
            target.clone(),
        ));
        return issues;
    }

    if normalized != id {
        issues.push(
            ValidationIssue::error(format!("{code_prefix}_invalid"), target.clone())
                .with_param("language", id)
                .with_param("reason", "leading_or_trailing_whitespace"),
        );
    }

    if normalized.chars().any(char::is_control) {
        issues.push(
            ValidationIssue::error(format!("{code_prefix}_invalid"), target.clone())
                .with_param("language", id)
                .with_param("reason", "control_character"),
        );
    }

    if normalized.len() > 64 {
        issues.push(
            ValidationIssue::error(format!("{code_prefix}_invalid"), target.clone())
                .with_param("language", id)
                .with_param("reason", "too_long"),
        );
    }

    if !is_accepted_language_id(&normalized) {
        issues.push(
            ValidationIssue::error(format!("{code_prefix}_invalid"), target.clone())
                .with_param("language", id)
                .with_param("reason", "unsupported_language_id"),
        );
    }

    if normalized
        .chars()
        .any(|ch| matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
    {
        issues.push(
            ValidationIssue::error(format!("{code_prefix}_invalid"), target.clone())
                .with_param("language", id)
                .with_param("reason", "forbidden_character"),
        );
    }

    if is_legacy_default_language_id(&normalized)
        && matches!(context, LanguageValidationContext::UserAuthored)
    {
        issues.push(
            ValidationIssue::error(format!("{code_prefix}_default_reserved"), target)
                .with_param("language", id),
        );
    }

    issues
}

pub fn is_catalog_language(catalog: &[TextLanguageProfile], id: &str) -> bool {
    let normalized = normalize_language_id(id);
    catalog
        .iter()
        .any(|profile| !profile.hidden && profile.id == normalized)
}

pub fn visible_catalog_ids(catalog: &[TextLanguageProfile]) -> BTreeSet<LanguageCode> {
    catalog
        .iter()
        .filter(|profile| !profile.hidden)
        .map(|profile| profile.id.clone())
        .collect()
}

pub fn validate_catalog_membership(
    language: &str,
    catalog: &[TextLanguageProfile],
    existing_languages: &BTreeSet<LanguageCode>,
    scope: &str,
    field: &str,
    code: &str,
) -> Vec<ValidationIssue> {
    let mut issues = validate_language_id(
        language,
        LanguageValidationContext::UserAuthored,
        scope,
        field,
        code,
    );
    let normalized = normalize_language_id(language);
    if !normalized.is_empty()
        && !is_legacy_default_language_id(&normalized)
        && !is_catalog_language(catalog, &normalized)
        && !existing_languages.contains(&normalized)
    {
        issues.push(
            ValidationIssue::error(
                format!("{code}_not_in_catalog"),
                ValidationTarget::new(scope).with_field(field),
            )
            .with_param("language", language),
        );
    }
    issues
}

pub fn canonical_builtin_id(id: &str) -> Option<LanguageCode> {
    builtin_text_language_catalog()
        .into_iter()
        .find(|profile| profile.id.eq_ignore_ascii_case(id))
        .map(|profile| profile.id)
}

fn is_accepted_language_id(id: &str) -> bool {
    if id.starts_with("x-") {
        return is_custom_language_id(id);
    }
    is_bcpish_language_id(id)
}

fn is_bcpish_language_id(id: &str) -> bool {
    let segments = id.split('-').collect::<Vec<_>>();
    if segments.is_empty() || segments.iter().any(|segment| segment.is_empty()) {
        return false;
    }

    let primary = segments[0];
    if !(2..=3).contains(&primary.len()) || !primary.chars().all(|ch| ch.is_ascii_lowercase()) {
        return false;
    }

    segments.iter().skip(1).all(|segment| {
        (2..=8).contains(&segment.len()) && segment.chars().all(|ch| ch.is_ascii_alphanumeric())
    })
}

fn is_custom_language_id(id: &str) -> bool {
    let rest = &id[2..];
    !rest.is_empty()
        && rest.split('-').all(|segment| {
            !segment.is_empty()
                && segment.len() <= 16
                && segment.chars().all(|ch| ch.is_ascii_alphanumeric())
        })
}
