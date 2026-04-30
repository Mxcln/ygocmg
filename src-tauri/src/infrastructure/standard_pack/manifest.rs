use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::ids::LanguageCode;
use crate::domain::common::time::AppTimestamp;

use super::{
    STANDARD_INDEX_SCHEMA_VERSION, StandardPackIndexFile, StandardPackSourceSnapshot,
    standard_pack_dir, standard_pack_index_path,
};

pub const STANDARD_MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct StandardPackIndexFileStamp {
    pub len: u64,
    pub modified: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardPackManifestFile {
    pub schema_version: u32,
    pub index_schema_version: u32,
    pub source: StandardPackSourceSnapshot,
    pub source_language: LanguageCode,
    pub indexed_at: AppTimestamp,
    pub card_count: usize,
    pub string_count: usize,
    pub index_file: StandardPackIndexFileStamp,
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
    if manifest.index_schema_version != STANDARD_INDEX_SCHEMA_VERSION {
        return Err(AppError::new(
            "standard_pack.manifest_index_schema_mismatch",
            "standard pack manifest index schema mismatch",
        )
        .with_detail("path", path.display().to_string())
        .with_detail("expected", STANDARD_INDEX_SCHEMA_VERSION)
        .with_detail("actual", manifest.index_schema_version));
    }
    Ok(manifest)
}

pub fn save_manifest(app_data_dir: &Path, index: &StandardPackIndexFile) -> AppResult<()> {
    let stamp = index_file_stamp(app_data_dir)?;
    let manifest = manifest_from_index(index, stamp);
    crate::infrastructure::json_store::write_json(
        &standard_pack_manifest_path(app_data_dir),
        &manifest,
    )
}

pub fn load_matching_manifest(app_data_dir: &Path) -> AppResult<Option<StandardPackManifestFile>> {
    let current_stamp = match index_file_stamp(app_data_dir) {
        Ok(stamp) => stamp,
        Err(error) if error.code == "standard_pack.index_metadata_failed" => return Ok(None),
        Err(error) => return Err(error),
    };
    let manifest = match load_manifest(app_data_dir) {
        Ok(manifest) => manifest,
        Err(_) => return Ok(None),
    };
    if manifest.index_file == current_stamp {
        Ok(Some(manifest))
    } else {
        Ok(None)
    }
}

pub fn index_file_stamp(app_data_dir: &Path) -> AppResult<StandardPackIndexFileStamp> {
    let path = standard_pack_index_path(app_data_dir);
    let metadata = fs::metadata(&path).map_err(|source| {
        AppError::from_io("standard_pack.index_metadata_failed", source)
            .with_detail("path", path.display().to_string())
    })?;
    Ok(StandardPackIndexFileStamp {
        len: metadata.len(),
        modified: metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .map(|value| value.as_secs() as i64),
    })
}

pub fn manifest_from_index(
    index: &StandardPackIndexFile,
    index_file: StandardPackIndexFileStamp,
) -> StandardPackManifestFile {
    StandardPackManifestFile {
        schema_version: STANDARD_MANIFEST_SCHEMA_VERSION,
        index_schema_version: STANDARD_INDEX_SCHEMA_VERSION,
        source: index.source.clone(),
        source_language: index.source_language.clone(),
        indexed_at: index.indexed_at,
        card_count: index.cards.len(),
        string_count: index.strings.records.len(),
        index_file,
    }
}
