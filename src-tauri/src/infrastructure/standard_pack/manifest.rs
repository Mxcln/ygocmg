use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::ids::LanguageCode;
use crate::domain::common::time::AppTimestamp;

use super::{StandardPackIndexFile, StandardPackSourceSnapshot, sqlite_store, standard_pack_dir};

pub const STANDARD_MANIFEST_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardPackManifestFile {
    pub schema_version: u32,
    pub sqlite_schema_version: u32,
    pub source: StandardPackSourceSnapshot,
    pub source_language: LanguageCode,
    pub indexed_at: AppTimestamp,
    pub card_count: usize,
    pub string_count: usize,
}

pub fn standard_pack_manifest_path(app_data_dir: &Path) -> PathBuf {
    standard_pack_dir(app_data_dir).join("manifest.json")
}

pub fn load_manifest(app_data_dir: &Path) -> AppResult<StandardPackManifestFile> {
    let path = standard_pack_manifest_path(app_data_dir);
    let manifest: StandardPackManifestFile = crate::infrastructure::json_store::read_json(&path)
        .map_err(|error| {
            AppError::new("standard_pack.manifest_read_failed", error.message)
                .with_detail("path", path.display().to_string())
        })?;
    if manifest.schema_version != STANDARD_MANIFEST_SCHEMA_VERSION {
        return Err(AppError::new(
            "standard_pack.manifest_schema_mismatch",
            "standard pack manifest schema mismatch",
        )
        .with_detail("path", path.display().to_string())
        .with_detail("expected", STANDARD_MANIFEST_SCHEMA_VERSION)
        .with_detail("actual", manifest.schema_version));
    }
    if manifest.sqlite_schema_version != sqlite_store::STANDARD_SQLITE_SCHEMA_VERSION {
        return Err(AppError::new(
            "standard_pack.manifest_sqlite_schema_mismatch",
            "standard pack manifest sqlite schema mismatch",
        )
        .with_detail("path", path.display().to_string())
        .with_detail("expected", sqlite_store::STANDARD_SQLITE_SCHEMA_VERSION)
        .with_detail("actual", manifest.sqlite_schema_version));
    }
    Ok(manifest)
}

pub fn save_manifest(app_data_dir: &Path, index: &StandardPackIndexFile) -> AppResult<()> {
    crate::infrastructure::json_store::write_json(
        &standard_pack_manifest_path(app_data_dir),
        &manifest_from_index(index),
    )
}

pub fn manifest_from_index(index: &StandardPackIndexFile) -> StandardPackManifestFile {
    StandardPackManifestFile {
        schema_version: STANDARD_MANIFEST_SCHEMA_VERSION,
        sqlite_schema_version: sqlite_store::STANDARD_SQLITE_SCHEMA_VERSION,
        source: index.source.clone(),
        source_language: index.source_language.clone(),
        indexed_at: index.indexed_at,
        card_count: index.cards.len(),
        string_count: index.strings.records.len(),
    }
}
