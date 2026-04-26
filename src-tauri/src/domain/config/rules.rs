use crate::domain::common::issue::{IssueLevel, ValidationIssue, ValidationTarget};
use crate::domain::config::model::GlobalConfig;

pub fn default_global_config() -> GlobalConfig {
    GlobalConfig {
        app_language: "zh-CN".to_string(),
        ygopro_path: None,
        external_text_editor_path: None,
        custom_code_recommended_min: 90_000_000,
        custom_code_recommended_max: 99_999_999,
        custom_code_min_gap: 10,
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
