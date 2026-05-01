use crate::domain::common::issue::{ValidationIssue, ValidationTarget};
use crate::domain::common::time::AppTimestamp;
use crate::domain::pack::model::{PackMetadata, PackOverview};

pub fn validate_pack_metadata(metadata: &PackMetadata) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let target = ValidationTarget::new("pack").with_entity_id(metadata.id.clone());

    if metadata.name.trim().is_empty() {
        issues.push(ValidationIssue::error(
            "pack.name_required",
            target.clone().with_field("name"),
        ));
    }

    if metadata.author.trim().is_empty() {
        issues.push(ValidationIssue::error(
            "pack.author_required",
            target.clone().with_field("author"),
        ));
    }

    if metadata.version.trim().is_empty() {
        issues.push(ValidationIssue::error(
            "pack.version_required",
            target.clone().with_field("version"),
        ));
    }

    if let Some(code) = metadata.pack_code.as_ref() {
        let trimmed = code.trim();
        if trimmed.is_empty() {
            issues.push(ValidationIssue::error(
                "pack.pack_code_invalid",
                target.clone().with_field("pack_code"),
            ));
        } else if trimmed.len() > 12 {
            issues.push(ValidationIssue::error(
                "pack.pack_code_invalid",
                target.clone().with_field("pack_code"),
            ));
        } else if !trimmed
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            issues.push(ValidationIssue::error(
                "pack.pack_code_invalid",
                target.clone().with_field("pack_code"),
            ));
        }
    }

    if metadata.display_language_order.is_empty() {
        issues.push(ValidationIssue::warning(
            "pack.display_language_order_empty",
            target.with_field("display_language_order"),
        ));
    }

    issues
}

pub fn derive_pack_overview(metadata: &PackMetadata, card_count: usize) -> PackOverview {
    PackOverview {
        id: metadata.id.clone(),
        kind: metadata.kind.clone(),
        name: metadata.name.clone(),
        author: metadata.author.clone(),
        version: metadata.version.clone(),
        card_count,
        updated_at: metadata.updated_at,
    }
}

pub fn touch_pack_metadata(metadata: &PackMetadata, now: AppTimestamp) -> PackMetadata {
    let mut next = metadata.clone();
    next.updated_at = now;
    next
}
