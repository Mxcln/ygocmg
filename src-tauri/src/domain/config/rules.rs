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
        shell_sidebar_width: 150,
        shell_window_width: 960,
        shell_window_height: 640,
        shell_window_is_maximized: false,
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
