use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use unicode_normalization::UnicodeNormalization;

use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::ids::PackId;
use crate::domain::pack::model::PackOverview;
use crate::domain::pack::summary::derive_pack_overview;
use crate::infrastructure::json_store;

#[derive(Debug, Clone, Default)]
pub struct WorkspacePackInventory {
    pub pack_paths: BTreeMap<PackId, PathBuf>,
    pub pack_overviews: BTreeMap<PackId, PackOverview>,
}

pub fn load_workspace_pack_inventory(workspace_path: &Path) -> AppResult<WorkspacePackInventory> {
    let packs_root = json_store::packs_root_path(workspace_path);
    if !packs_root.exists() {
        return Ok(WorkspacePackInventory::default());
    }

    let mut pack_paths: BTreeMap<PackId, PathBuf> = BTreeMap::new();
    let mut pack_overviews: BTreeMap<PackId, PackOverview> = BTreeMap::new();

    for entry in fs::read_dir(&packs_root).map_err(|source| {
        AppError::from_io("pack.read_dir_failed", source)
            .with_detail("path", packs_root.display().to_string())
    })? {
        let entry = entry.map_err(|source| {
            AppError::from_io("pack.read_dir_entry_failed", source)
                .with_detail("path", packs_root.display().to_string())
        })?;
        let pack_path = entry.path();
        if !pack_path.is_dir() {
            continue;
        }

        let metadata_path = json_store::pack_metadata_path(&pack_path);
        if !metadata_path.exists() {
            return Err(
                AppError::new("pack.metadata_missing", "pack metadata.json is missing")
                    .with_detail("path", metadata_path.display().to_string()),
            );
        }

        let metadata = json_store::load_pack_metadata(&pack_path)?;
        if let Some(existing_path) = pack_paths.get(&metadata.id) {
            return Err(AppError::new(
                "pack.duplicate_id",
                "duplicate pack id detected in workspace",
            )
            .with_detail("pack_id", metadata.id.clone())
            .with_detail("path", pack_path.display().to_string())
            .with_detail("existing_path", existing_path.display().to_string()));
        }

        let card_count = json_store::load_cards(&pack_path)
            .map(|cards| cards.len())
            .unwrap_or_default();
        pack_overviews.insert(
            metadata.id.clone(),
            derive_pack_overview(&metadata, card_count),
        );
        pack_paths.insert(metadata.id, pack_path);
    }

    Ok(WorkspacePackInventory {
        pack_paths,
        pack_overviews,
    })
}

pub fn resolve_pack_path(inventory: &WorkspacePackInventory, pack_id: &str) -> AppResult<PathBuf> {
    inventory.pack_paths.get(pack_id).cloned().ok_or_else(|| {
        AppError::new(
            "pack.path_not_indexed",
            "pack path is not indexed in workspace",
        )
    })
}

pub fn suggest_pack_storage_name(
    workspace_path: &Path,
    display_name: &str,
    pack_id: &str,
) -> AppResult<String> {
    let normalized_name = sanitize_pack_storage_label(display_name);
    let packs_root = json_store::packs_root_path(workspace_path);

    for suffix_len in [8_usize, 12_usize, pack_id.len()] {
        let suffix_len = suffix_len.min(pack_id.len());
        let candidate = format!("{normalized_name}--{}", &pack_id[..suffix_len]);
        let candidate_path = packs_root.join(&candidate);
        if !candidate_path.exists() {
            return Ok(candidate);
        }
    }

    Err(AppError::new(
        "pack.storage_name_conflict",
        "unable to allocate a unique pack storage name",
    )
    .with_detail("pack_id", pack_id)
    .with_detail("display_name", display_name))
}

pub fn sanitize_pack_storage_label(display_name: &str) -> String {
    let normalized: String = display_name.nfc().collect();
    let reserved_names = reserved_windows_names();
    let mut sanitized = String::new();
    let mut last_was_separator = false;

    for ch in normalized.chars() {
        let needs_separator =
            ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*');

        if needs_separator || ch.is_whitespace() || ch == '-' {
            if !last_was_separator && !sanitized.is_empty() {
                sanitized.push('-');
                last_was_separator = true;
            }
            continue;
        }

        sanitized.push(ch);
        last_was_separator = false;
    }

    let trimmed = sanitized
        .trim_matches(|ch| ch == ' ' || ch == '.' || ch == '-')
        .chars()
        .take(48)
        .collect::<String>();

    let mut result = if trimmed.is_empty() {
        "pack".to_string()
    } else {
        trimmed
    };

    if reserved_names.contains(&result.to_uppercase()) {
        result.push_str("-pack");
    }

    if result.is_empty() {
        "pack".to_string()
    } else {
        result
    }
}

fn reserved_windows_names() -> BTreeSet<String> {
    let mut names = BTreeSet::from([
        "CON".to_string(),
        "PRN".to_string(),
        "AUX".to_string(),
        "NUL".to_string(),
    ]);

    for index in 1..=9 {
        names.insert(format!("COM{index}"));
        names.insert(format!("LPT{index}"));
    }

    names
}

#[cfg(test)]
mod tests {
    use super::sanitize_pack_storage_label;

    #[test]
    fn sanitizes_unicode_names_without_ascii_fallback() {
        assert_eq!(sanitize_pack_storage_label("龙族卡组"), "龙族卡组");
    }

    #[test]
    fn sanitizes_reserved_and_invalid_names() {
        assert_eq!(sanitize_pack_storage_label("CON"), "CON-pack");
        assert_eq!(sanitize_pack_storage_label("My <Pack>?"), "My-Pack");
        assert_eq!(sanitize_pack_storage_label("   "), "pack");
    }
}
