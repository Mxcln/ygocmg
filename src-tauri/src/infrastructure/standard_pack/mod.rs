use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::domain::card::derive::derive_card_list_row;
use crate::domain::card::model::CardEntity;
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::ids::LanguageCode;
use crate::domain::common::time::{AppTimestamp, now_utc};
use crate::domain::namespace::model::StandardStringNamespaceBaseline;
use crate::domain::resource::model::CardAssetState;
use crate::domain::strings::model::PackStringRecord;

pub mod manifest;
pub mod sqlite_store;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardPackIndexFile {
    pub source: StandardPackSourceSnapshot,
    pub source_language: LanguageCode,
    pub indexed_at: AppTimestamp,
    pub cards: Vec<StandardCardIndexRecord>,
    pub strings: StandardStringsIndex,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StandardStringsIndex {
    pub baseline: StandardStringNamespaceBaseline,
    pub records: Vec<PackStringRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StandardPackSourceSnapshot {
    pub ygopro_path: String,
    pub cdb_path: String,
    pub cdb_modified: Option<i64>,
    pub cdb_len: u64,
    pub strings_modified: Option<i64>,
    pub strings_len: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardCardIndexRecord {
    pub card: CardEntity,
    pub row: crate::domain::card::model::CardListRow,
    pub asset_state: CardAssetState,
    pub raw_type: u64,
    pub raw_race: u64,
    pub raw_attribute: u64,
    pub raw_level: u64,
}

#[derive(Debug, Clone)]
pub struct StandardPackSource {
    pub ygopro_path: PathBuf,
    pub cdb_path: PathBuf,
    pub snapshot: StandardPackSourceSnapshot,
}

#[derive(Debug, Clone)]
pub struct StandardPackStatus {
    pub configured: bool,
    pub source_language_configured: bool,
    pub ygopro_path: Option<PathBuf>,
    pub cdb_path: Option<PathBuf>,
    pub index_exists: bool,
    pub schema_mismatch: bool,
    pub stale: bool,
    pub source_language: Option<LanguageCode>,
    pub indexed_at: Option<AppTimestamp>,
    pub card_count: usize,
    pub message: Option<String>,
}

pub fn standard_pack_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("standard_pack")
}

pub fn save_index(app_data_dir: &Path, index: &StandardPackIndexFile) -> AppResult<()> {
    sqlite_store::save_sqlite_index(app_data_dir, index)?;
    manifest::save_manifest(app_data_dir, index)
}

pub fn status(
    app_data_dir: &Path,
    ygopro_path: Option<&Path>,
    configured_source_language: Option<&str>,
) -> StandardPackStatus {
    let configured = ygopro_path.is_some();
    let source_result = ygopro_path.map(discover_source);
    let source = source_result
        .as_ref()
        .and_then(|result| result.as_ref().ok());
    let source_error = source_result
        .as_ref()
        .and_then(|result| result.as_ref().err())
        .map(|error| error.message.clone());
    let index_result = sqlite_store::load_sqlite_manifest_from_app_data(app_data_dir);
    let schema_mismatch = index_result
        .as_ref()
        .err()
        .is_some_and(|error| error.code == "standard_pack.sqlite_schema_mismatch");
    let sqlite_missing = index_result
        .as_ref()
        .err()
        .is_some_and(|error| error.code == "standard_pack.sqlite_missing");
    let index_error = index_result.as_ref().err().cloned();
    let index = index_result.ok();
    let index_exists = index.is_some();
    let stale = match (&index, source) {
        (Some(index), Some(source)) => {
            index.source != source.snapshot
                || configured_source_language
                    .is_some_and(|language| language != index.source_language)
        }
        _ => false,
    };
    let message = if configured && configured_source_language.is_none() {
        Some("standard pack source language is not configured".to_string())
    } else if schema_mismatch {
        Some("standard pack sqlite schema is outdated; rebuild required".to_string())
    } else if sqlite_missing {
        Some("standard pack sqlite index is missing; rebuild required".to_string())
    } else if let Some(error) = index_error {
        Some(error.message)
    } else {
        source_error
    };

    StandardPackStatus {
        configured,
        source_language_configured: configured_source_language.is_some(),
        ygopro_path: ygopro_path.map(Path::to_path_buf),
        cdb_path: source.map(|source| source.cdb_path.clone()),
        index_exists,
        schema_mismatch,
        stale,
        source_language: index.as_ref().map(|index| index.source_language.clone()),
        indexed_at: index.as_ref().map(|index| index.indexed_at),
        card_count: index.as_ref().map(|index| index.card_count).unwrap_or(0),
        message,
    }
}

pub fn discover_source(ygopro_path: &Path) -> AppResult<StandardPackSource> {
    if !ygopro_path.exists() {
        return Err(AppError::new(
            "standard_pack.ygopro_path_missing",
            "YGOPro path does not exist",
        )
        .with_detail("path", ygopro_path.display().to_string()));
    }
    if !ygopro_path.is_dir() {
        return Err(AppError::new(
            "standard_pack.ygopro_path_not_directory",
            "YGOPro path is not a directory",
        )
        .with_detail("path", ygopro_path.display().to_string()));
    }

    let mut cdb_paths = Vec::new();
    let entries = fs::read_dir(ygopro_path).map_err(|source| {
        AppError::from_io("standard_pack.read_dir_failed", source)
            .with_detail("path", ygopro_path.display().to_string())
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| {
            AppError::from_io("standard_pack.read_dir_entry_failed", source)
                .with_detail("path", ygopro_path.display().to_string())
        })?;
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("cdb"))
        {
            cdb_paths.push(path);
        }
    }

    match cdb_paths.len() {
        0 => Err(AppError::new(
            "standard_pack.cdb_missing",
            "no root .cdb file found in YGOPro path",
        )
        .with_detail("path", ygopro_path.display().to_string())),
        1 => {
            let cdb_path = cdb_paths.remove(0);
            let cdb_meta = metadata_stamp(&cdb_path)?;
            let strings_path = ygopro_path.join("strings.conf");
            let strings_meta = metadata_stamp_optional(&strings_path)?;
            Ok(StandardPackSource {
                ygopro_path: ygopro_path.to_path_buf(),
                cdb_path: cdb_path.clone(),
                snapshot: StandardPackSourceSnapshot {
                    ygopro_path: ygopro_path.to_string_lossy().to_string(),
                    cdb_path: cdb_path.to_string_lossy().to_string(),
                    cdb_modified: cdb_meta.modified,
                    cdb_len: cdb_meta.len.unwrap_or(0),
                    strings_modified: strings_meta.modified,
                    strings_len: strings_meta.len,
                },
            })
        }
        _ => Err(AppError::new(
            "standard_pack.multiple_cdb_files",
            "multiple root .cdb files found in YGOPro path",
        )
        .with_detail("path", ygopro_path.display().to_string())
        .with_detail(
            "cdb_paths",
            cdb_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>(),
        )),
    }
}

pub fn rebuild_index(
    ygopro_path: &Path,
    source_language: &str,
) -> AppResult<StandardPackIndexFile> {
    let source = discover_source(ygopro_path)?;
    let mut records = load_cards_from_cdb(&source, source_language)?;
    records.sort_by(|left, right| left.card.code.cmp(&right.card.code));

    Ok(StandardPackIndexFile {
        source: source.snapshot,
        source_language: source_language.to_string(),
        indexed_at: now_utc(),
        cards: records,
        strings: load_standard_strings(&source.ygopro_path.join("strings.conf"), source_language),
    })
}

fn load_cards_from_cdb(
    source: &StandardPackSource,
    source_language: &str,
) -> AppResult<Vec<StandardCardIndexRecord>> {
    let assets = StandardAssetIndex::scan(&source.ygopro_path);
    let records = crate::infrastructure::ygopro_cdb::load_cards_from_cdb(&source.cdb_path)?
        .into_iter()
        .map(|mut record| {
            remap_card_language(&mut record.card, source_language);
            let asset_state = assets.asset_state(record.card.code);
            let row =
                derive_card_list_row(&record.card, &asset_state, &[source_language.to_string()]);
            StandardCardIndexRecord {
                card: record.card,
                row,
                asset_state,
                raw_type: record.raw_type,
                raw_race: record.raw_race,
                raw_attribute: record.raw_attribute,
                raw_level: record.raw_level,
            }
        })
        .collect::<Vec<_>>();
    Ok(records)
}

#[derive(Debug, Clone, Default)]
struct StandardAssetIndex {
    images: BTreeSet<u32>,
    field_images: BTreeSet<u32>,
    scripts: BTreeSet<u32>,
}

impl StandardAssetIndex {
    fn scan(ygopro_path: &Path) -> Self {
        Self {
            images: scan_numeric_assets(&ygopro_path.join("pics"), "", "jpg"),
            field_images: scan_numeric_assets(&ygopro_path.join("pics").join("field"), "", "jpg"),
            scripts: scan_numeric_assets(&ygopro_path.join("script"), "c", "lua"),
        }
    }

    fn asset_state(&self, code: u32) -> CardAssetState {
        CardAssetState {
            has_image: self.images.contains(&code),
            has_field_image: self.field_images.contains(&code),
            has_script: self.scripts.contains(&code),
        }
    }
}

fn scan_numeric_assets(dir: &Path, prefix: &str, extension: &str) -> BTreeSet<u32> {
    let mut values = BTreeSet::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return values;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case(extension))
        {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(digits) = stem.strip_prefix(prefix) else {
            continue;
        };
        if digits.is_empty() || !digits.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        if let Ok(code) = digits.parse::<u32>() {
            values.insert(code);
        }
    }
    values
}

fn load_standard_strings(path: &Path, source_language: &str) -> StandardStringsIndex {
    let Ok(mut records) = crate::infrastructure::strings_conf::load_records(path) else {
        return StandardStringsIndex::default();
    };
    for record in &mut records {
        remap_string_language(record, source_language);
    }
    records.sort_by(|left, right| left.kind.cmp(&right.kind).then(left.key.cmp(&right.key)));
    let baseline = crate::infrastructure::strings_conf::baseline_from_records(&records);

    StandardStringsIndex { baseline, records }
}

fn remap_card_language(card: &mut CardEntity, source_language: &str) {
    if card.texts.contains_key(source_language) {
        return;
    }
    if let Some(texts) = card
        .texts
        .remove(crate::domain::language::model::LEGACY_DEFAULT_LANGUAGE)
    {
        card.texts.insert(source_language.to_string(), texts);
    }
}

fn remap_string_language(record: &mut PackStringRecord, source_language: &str) {
    if record.values.contains_key(source_language) {
        return;
    }
    if let Some(value) = record
        .values
        .remove(crate::domain::language::model::LEGACY_DEFAULT_LANGUAGE)
    {
        record.values.insert(source_language.to_string(), value);
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct FileStamp {
    modified: Option<i64>,
    len: Option<u64>,
}

fn metadata_stamp(path: &Path) -> AppResult<FileStamp> {
    let metadata = fs::metadata(path).map_err(|source| {
        AppError::from_io("standard_pack.file_metadata_failed", source)
            .with_detail("path", path.display().to_string())
    })?;
    Ok(FileStamp {
        modified: metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .map(|value| value.as_secs() as i64),
        len: Some(metadata.len()),
    })
}

fn metadata_stamp_optional(path: &Path) -> AppResult<FileStamp> {
    if !path.exists() {
        return Ok(FileStamp::default());
    }
    metadata_stamp(path)
}
