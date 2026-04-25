use std::collections::BTreeSet;

use crate::domain::common::issue::{ValidationIssue, ValidationTarget};
use crate::domain::strings::model::PackStringsFile;

pub fn validate_pack_strings(file: &PackStringsFile) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    for (language, entries) in &file.entries {
        let mut seen = BTreeSet::new();
        for entry in entries {
            let key = (entry.kind.clone(), entry.key);
            if !seen.insert(key) {
                issues.push(
                    ValidationIssue::error(
                        "pack_strings.duplicate_kind_key",
                        ValidationTarget::new("pack_strings")
                            .with_entity_id(language.clone())
                            .with_field("entries"),
                    )
                    .with_param("language", language),
                );
            }
        }
    }

    issues
}
