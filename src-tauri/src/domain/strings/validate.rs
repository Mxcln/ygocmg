use std::collections::BTreeSet;

use crate::domain::common::issue::{ValidationIssue, ValidationTarget};
use crate::domain::strings::model::{PackStringKind, PackStringsFile};

pub fn validate_pack_strings(file: &PackStringsFile) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let mut seen = BTreeSet::new();

    for record in &file.entries {
        let identity = (record.kind.clone(), record.key);
        if !seen.insert(identity) {
            issues.push(
                ValidationIssue::error(
                    "pack_strings.duplicate_kind_key",
                    ValidationTarget::new("pack_strings").with_field("entries"),
                )
                .with_param("kind", &record.kind)
                .with_param("key", record.key),
            );
        }

        if record.values.is_empty() {
            issues.push(
                ValidationIssue::error(
                    "pack_strings.values_required",
                    ValidationTarget::new("pack_strings").with_field("values"),
                )
                .with_param("kind", &record.kind)
                .with_param("key", record.key),
            );
        }

        for (language, value) in &record.values {
            if value.trim().is_empty() {
                issues.push(
                    ValidationIssue::error(
                        "pack_strings.value_required",
                        ValidationTarget::new("pack_strings")
                            .with_entity_id(language.clone())
                            .with_field("value"),
                    )
                    .with_param("language", language)
                    .with_param("kind", &record.kind)
                    .with_param("key", record.key),
                );
            }
        }

        match record.kind {
            PackStringKind::System => {
                if record.key > 0x07ff {
                    issues.push(
                        ValidationIssue::error(
                            "pack_strings.system_key_hard_max_exceeded",
                            ValidationTarget::new("pack_strings").with_field("key"),
                        )
                        .with_param("kind", &record.kind)
                        .with_param("key", record.key)
                        .with_param("hard_max", 0x07ff),
                    );
                }
            }
            PackStringKind::Victory | PackStringKind::Counter | PackStringKind::Setname => {
                if record.key > 0xffff {
                    issues.push(
                        ValidationIssue::error(
                            "pack_strings.key_hard_max_exceeded",
                            ValidationTarget::new("pack_strings").with_field("key"),
                        )
                        .with_param("kind", &record.kind)
                        .with_param("key", record.key)
                        .with_param("hard_max", 0xffff),
                    );
                }
            }
        }
    }

    issues
}
