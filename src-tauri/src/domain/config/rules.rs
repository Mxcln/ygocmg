use std::collections::BTreeSet;

use crate::domain::common::issue::{IssueLevel, ValidationIssue, ValidationTarget};
use crate::domain::config::model::GlobalConfig;
use crate::domain::language::rules::{
    LanguageValidationContext, default_text_language_catalog, is_catalog_language,
    normalize_language_id, normalize_text_language_catalog, validate_language_id,
};

pub const DEFAULT_APP_LANGUAGE: &str = "en-US";
pub const SUPPORTED_APP_LANGUAGES: [&str; 3] = ["en-US", "ja-JP", "zh-CN"];

pub fn default_global_config() -> GlobalConfig {
    GlobalConfig {
        app_language: DEFAULT_APP_LANGUAGE.to_string(),
        ygopro_path: None,
        external_text_editor_path: None,
        custom_code_recommended_min: 100_000_000,
        custom_code_recommended_max: 200_000_000,
        custom_code_min_gap: 5,
        shell_sidebar_width: 150,
        shell_window_width: 960,
        shell_window_height: 640,
        shell_window_is_maximized: false,
        text_language_catalog: default_text_language_catalog(),
        standard_pack_source_language: None,
    }
}

pub fn validate_code_policy(config: &GlobalConfig) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let target = ValidationTarget::new("global_config");

    if config.custom_code_recommended_min > config.custom_code_recommended_max {
        issues.push(
            ValidationIssue::error(
                "config.invalid_code_range",
                target.clone().with_field("custom_code_recommended_min"),
            )
            .with_param("min", config.custom_code_recommended_min)
            .with_param("max", config.custom_code_recommended_max),
        );
    }

    if config.custom_code_min_gap == 0 {
        issues.push(
            ValidationIssue::error(
                "config.invalid_code_min_gap",
                target.clone().with_field("custom_code_min_gap"),
            )
            .with_param("value", config.custom_code_min_gap),
        );
    }

    issues
}

pub fn validate_global_config(config: &GlobalConfig) -> Vec<ValidationIssue> {
    let mut issues = validate_code_policy(config);
    let target = ValidationTarget::new("global_config");

    if config.app_language.trim().is_empty() {
        issues.push(ValidationIssue::error(
            "config.app_language_required",
            target.clone().with_field("app_language"),
        ));
    } else if !SUPPORTED_APP_LANGUAGES.contains(&config.app_language.as_str()) {
        issues.push(
            ValidationIssue::error(
                "config.app_language_unsupported",
                target.clone().with_field("app_language"),
            )
            .with_param("language", &config.app_language),
        );
    }

    issues.extend(validate_text_language_catalog(config));

    if let Some(language) = &config.standard_pack_source_language {
        issues.extend(validate_language_id(
            language,
            LanguageValidationContext::UserAuthored,
            "global_config",
            "standard_pack_source_language",
            "config.standard_pack_source_language",
        ));
        if !is_catalog_language(&config.text_language_catalog, language) {
            issues.push(
                ValidationIssue::error(
                    "config.standard_pack_source_language_not_in_catalog",
                    target.clone().with_field("standard_pack_source_language"),
                )
                .with_param("language", language),
            );
        }
    }

    if !(140..=280).contains(&config.shell_sidebar_width) {
        issues.push(
            ValidationIssue::error(
                "config.invalid_shell_sidebar_width",
                target.clone().with_field("shell_sidebar_width"),
            )
            .with_param("value", config.shell_sidebar_width)
            .with_param("min", 140)
            .with_param("max", 280),
        );
    }

    if config.shell_window_width < 960 {
        issues.push(
            ValidationIssue::error(
                "config.invalid_shell_window_width",
                target.clone().with_field("shell_window_width"),
            )
            .with_param("value", config.shell_window_width)
            .with_param("min", 960),
        );
    }

    if config.shell_window_height < 640 {
        issues.push(
            ValidationIssue::error(
                "config.invalid_shell_window_height",
                target.clone().with_field("shell_window_height"),
            )
            .with_param("value", config.shell_window_height)
            .with_param("min", 640),
        );
    }

    for (field, path) in [
        ("ygopro_path", config.ygopro_path.as_ref()),
        (
            "external_text_editor_path",
            config.external_text_editor_path.as_ref(),
        ),
    ] {
        if let Some(path) = path {
            if !path.exists() {
                issues.push(
                    ValidationIssue {
                        code: format!("config.{}_missing", field),
                        level: IssueLevel::Warning,
                        target: target.clone().with_field(field),
                        params: Default::default(),
                    }
                    .with_param("path", path.display().to_string()),
                );
            }
        }
    }

    issues
}

pub fn normalize_global_config(config: &GlobalConfig) -> GlobalConfig {
    let mut next = config.clone();
    next.app_language = normalize_app_language(&next.app_language);
    next.text_language_catalog = normalize_text_language_catalog(&next.text_language_catalog);
    next.standard_pack_source_language = next
        .standard_pack_source_language
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    next
}

fn normalize_app_language(language: &str) -> String {
    let normalized = language.trim();
    if SUPPORTED_APP_LANGUAGES.contains(&normalized) {
        normalized.to_string()
    } else {
        DEFAULT_APP_LANGUAGE.to_string()
    }
}

fn validate_text_language_catalog(config: &GlobalConfig) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let target = ValidationTarget::new("global_config").with_field("text_language_catalog");
    let mut seen = BTreeSet::new();

    for profile in &config.text_language_catalog {
        issues.extend(validate_language_id(
            &profile.id,
            LanguageValidationContext::UserAuthored,
            "global_config",
            "text_language_catalog",
            "config.text_language_id",
        ));

        let normalized_id = normalize_language_id(&profile.id);
        if !seen.insert(normalized_id) {
            issues.push(
                ValidationIssue::error("config.text_language_id_duplicate", target.clone())
                    .with_param("language", &profile.id),
            );
        }

        if !profile.hidden && profile.label.trim().is_empty() {
            issues.push(
                ValidationIssue::error("config.text_language_label_required", target.clone())
                    .with_param("language", &profile.id),
            );
        }
    }

    issues
}
