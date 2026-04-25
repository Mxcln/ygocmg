use std::collections::BTreeSet;

use crate::domain::common::error::AppError;
use crate::domain::common::issue::{ValidationIssue, ValidationTarget};
use crate::domain::common::time::AppTimestamp;
use crate::domain::workspace::model::WorkspaceMeta;

pub fn validate_workspace_meta(meta: &WorkspaceMeta) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let target = ValidationTarget::new("workspace").with_entity_id(meta.id.clone());

    if meta.name.trim().is_empty() {
        issues.push(ValidationIssue::error(
            "workspace.name_required",
            target.clone().with_field("name"),
        ));
    }

    let unique_count = meta.pack_order.iter().collect::<BTreeSet<_>>().len();
    if unique_count != meta.pack_order.len() {
        issues.push(ValidationIssue::error(
            "workspace.pack_order_duplicated",
            target.with_field("pack_order"),
        ));
    }

    issues
}

pub fn reorder_pack_ids(
    current: &[String],
    target_order: &[String],
) -> Result<Vec<String>, AppError> {
    let current_set = current.iter().collect::<BTreeSet<_>>();
    let target_set = target_order.iter().collect::<BTreeSet<_>>();

    if current_set != target_set {
        return Err(AppError::new(
            "workspace.pack_order_mismatch",
            "target order does not match current pack set",
        ));
    }

    Ok(target_order.to_vec())
}

pub fn touch_workspace(meta: &WorkspaceMeta, now: AppTimestamp) -> WorkspaceMeta {
    let mut next = meta.clone();
    next.updated_at = now;
    next
}
