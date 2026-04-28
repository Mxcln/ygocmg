use crate::domain::common::issue::{ValidationIssue, ValidationTarget};
use crate::domain::namespace::model::{PackStringsNamespaceContext, counter_low12, setname_base};
use crate::domain::strings::model::{PackStringKind, PackStringRecord};

pub const RECOMMENDED_SETNAME_BASE_MIN: u16 = 0x0300;
pub const RECOMMENDED_SETNAME_BASE_MAX: u16 = 0x0fff;
pub const RECOMMENDED_COUNTER_LOW12_MIN: u16 = 0x0100;
pub const RECOMMENDED_COUNTER_LOW12_MAX: u16 = 0x0fff;
pub const RECOMMENDED_VICTORY_MIN: u32 = 0x0100;
pub const RECOMMENDED_VICTORY_MAX: u32 = 0xffff;

pub fn validate_pack_string_record_namespace(
    record: &PackStringRecord,
    ctx: &PackStringsNamespaceContext,
) -> Vec<ValidationIssue> {
    let target = ValidationTarget::new("pack_strings")
        .with_entity_id(format!("{:?}:{}", record.kind, record.key))
        .with_field("key");
    let mut issues = Vec::new();

    match record.kind {
        PackStringKind::System => {
            if ctx.other_custom.system_keys.contains(&record.key) {
                issues.push(
                    ValidationIssue::warning(
                        "pack_strings.system_key_conflicts_with_workspace_custom_pack",
                        target.clone(),
                    )
                    .with_param("key", record.key),
                );
            }
            if ctx.standard.system_keys.contains(&record.key) {
                issues.push(
                    ValidationIssue::warning(
                        "pack_strings.system_key_conflicts_with_standard_pack",
                        target.clone(),
                    )
                    .with_param("key", record.key),
                );
            }
        }
        PackStringKind::Victory => {
            if record.key < RECOMMENDED_VICTORY_MIN || record.key > RECOMMENDED_VICTORY_MAX {
                issues.push(
                    ValidationIssue::warning(
                        "pack_strings.victory_key_outside_recommended_range",
                        target.clone(),
                    )
                    .with_param("recommended_min", RECOMMENDED_VICTORY_MIN)
                    .with_param("recommended_max", RECOMMENDED_VICTORY_MAX)
                    .with_param("key", record.key),
                );
            }
            if ctx.other_custom.victory_keys.contains(&record.key) {
                issues.push(
                    ValidationIssue::warning(
                        "pack_strings.victory_key_conflicts_with_workspace_custom_pack",
                        target.clone(),
                    )
                    .with_param("key", record.key),
                );
            }
            if ctx.standard.victory_keys.contains(&record.key) {
                issues.push(
                    ValidationIssue::warning(
                        "pack_strings.victory_key_conflicts_with_standard_pack",
                        target.clone(),
                    )
                    .with_param("key", record.key),
                );
            }
        }
        PackStringKind::Counter => {
            let low12 = counter_low12(record.key);
            if low12 < RECOMMENDED_COUNTER_LOW12_MIN || low12 > RECOMMENDED_COUNTER_LOW12_MAX {
                issues.push(
                    ValidationIssue::warning(
                        "pack_strings.counter_key_outside_recommended_range",
                        target.clone(),
                    )
                    .with_param("recommended_low12_min", RECOMMENDED_COUNTER_LOW12_MIN)
                    .with_param("recommended_low12_max", RECOMMENDED_COUNTER_LOW12_MAX)
                    .with_param("key", record.key),
                );
            }
            if ctx.other_custom.counter_keys.contains(&record.key) {
                issues.push(
                    ValidationIssue::warning(
                        "pack_strings.counter_key_conflicts_with_workspace_custom_pack",
                        target.clone(),
                    )
                    .with_param("key", record.key),
                );
            }
            if ctx.standard.counter_keys.contains(&record.key) {
                issues.push(
                    ValidationIssue::warning(
                        "pack_strings.counter_key_conflicts_with_standard_pack",
                        target.clone(),
                    )
                    .with_param("key", record.key),
                );
            }
        }
        PackStringKind::Setname => {
            let base = setname_base(record.key);
            if base < RECOMMENDED_SETNAME_BASE_MIN || base > RECOMMENDED_SETNAME_BASE_MAX {
                issues.push(
                    ValidationIssue::warning(
                        "pack_strings.setname_base_outside_recommended_range",
                        target.clone(),
                    )
                    .with_param("recommended_base_min", RECOMMENDED_SETNAME_BASE_MIN)
                    .with_param("recommended_base_max", RECOMMENDED_SETNAME_BASE_MAX)
                    .with_param("base", base)
                    .with_param("key", record.key),
                );
            }
            if ctx.other_custom.setname_bases.contains(&base) {
                issues.push(
                    ValidationIssue::warning(
                        "pack_strings.setname_base_conflicts_with_workspace_custom_pack",
                        target.clone(),
                    )
                    .with_param("base", base)
                    .with_param("key", record.key),
                );
            }
            if ctx.standard.setname_bases.contains(&base) {
                issues.push(
                    ValidationIssue::warning(
                        "pack_strings.setname_base_conflicts_with_standard_pack",
                        target.clone(),
                    )
                    .with_param("base", base)
                    .with_param("key", record.key),
                );
            }
        }
    }

    issues
}
