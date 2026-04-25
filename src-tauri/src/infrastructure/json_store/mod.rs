use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::domain::card::model::{CardEntity, CardsFile};
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::config::model::{GlobalConfig, GlobalConfigFile};
use crate::domain::config::rules::default_global_config;
use crate::domain::pack::model::{PackMetadata, PackMetadataFile};
use crate::domain::resource::path_rules::{pack_field_pics_dir, pack_pics_dir, pack_scripts_dir};
use crate::domain::strings::model::PackStringsFile;
use crate::domain::workspace::model::{WorkspaceFile, WorkspaceMeta, WorkspaceRegistryFile};
use crate::infrastructure::fs::safe_write::safe_write_bytes;

pub const SCHEMA_VERSION: u32 = 1;

pub fn global_config_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("global_config.json")
}

pub fn workspace_registry_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("workspace_registry.json")
}

pub fn workspace_file_path(workspace_path: &Path) -> PathBuf {
    workspace_path.join("workspace.json")
}

pub fn packs_root_path(workspace_path: &Path) -> PathBuf {
    workspace_path.join("packs")
}

pub fn pack_path(workspace_path: &Path, pack_id: &str) -> PathBuf {
    packs_root_path(workspace_path).join(pack_id)
}

pub fn pack_metadata_path(pack_path: &Path) -> PathBuf {
    pack_path.join("metadata.json")
}

pub fn cards_path(pack_path: &Path) -> PathBuf {
    pack_path.join("cards.json")
}

pub fn pack_strings_path(pack_path: &Path) -> PathBuf {
    pack_path.join("strings.json")
}

pub fn ensure_workspace_layout(workspace_path: &Path) -> AppResult<()> {
    fs::create_dir_all(packs_root_path(workspace_path)).map_err(|source| {
        AppError::from_io("json_store.workspace_layout_create_failed", source)
            .with_detail("path", workspace_path.display().to_string())
    })?;
    Ok(())
}

pub fn ensure_pack_layout(pack_path: &Path) -> AppResult<()> {
    for directory in [
        pack_path.to_path_buf(),
        pack_pics_dir(pack_path),
        pack_field_pics_dir(pack_path),
        pack_scripts_dir(pack_path),
    ] {
        fs::create_dir_all(&directory).map_err(|source| {
            AppError::from_io("json_store.pack_layout_create_failed", source)
                .with_detail("path", directory.display().to_string())
        })?;
    }
    Ok(())
}

pub fn load_global_config(app_data_dir: &Path) -> AppResult<GlobalConfig> {
    let path = global_config_path(app_data_dir);
    if !path.exists() {
        return Ok(default_global_config());
    }

    let file: GlobalConfigFile = read_json(&path)?;
    ensure_schema(path.as_path(), file.schema_version)?;
    Ok(file.data)
}

pub fn save_global_config(app_data_dir: &Path, config: &GlobalConfig) -> AppResult<()> {
    write_json(
        &global_config_path(app_data_dir),
        &GlobalConfigFile {
            schema_version: SCHEMA_VERSION,
            data: config.clone(),
        },
    )
}

pub fn load_workspace_registry(app_data_dir: &Path) -> AppResult<WorkspaceRegistryFile> {
    let path = workspace_registry_path(app_data_dir);
    if !path.exists() {
        return Ok(WorkspaceRegistryFile::default());
    }

    let file: WorkspaceRegistryFile = read_json(&path)?;
    ensure_schema(path.as_path(), file.schema_version)?;
    Ok(file)
}

pub fn save_workspace_registry(app_data_dir: &Path, registry: &WorkspaceRegistryFile) -> AppResult<()> {
    write_json(&workspace_registry_path(app_data_dir), registry)
}

pub fn load_workspace_meta(workspace_path: &Path) -> AppResult<WorkspaceMeta> {
    let path = workspace_file_path(workspace_path);
    let file: WorkspaceFile = read_json(&path)?;
    ensure_schema(path.as_path(), file.schema_version)?;
    Ok(file.data)
}

pub fn save_workspace_meta(workspace_path: &Path, meta: &WorkspaceMeta) -> AppResult<()> {
    write_json(
        &workspace_file_path(workspace_path),
        &WorkspaceFile {
            schema_version: SCHEMA_VERSION,
            data: meta.clone(),
        },
    )
}

pub fn load_pack_metadata(pack_path: &Path) -> AppResult<PackMetadata> {
    let path = pack_metadata_path(pack_path);
    let file: PackMetadataFile = read_json(&path)?;
    ensure_schema(path.as_path(), file.schema_version)?;
    Ok(file.data)
}

pub fn save_pack_metadata(pack_path: &Path, metadata: &PackMetadata) -> AppResult<()> {
    write_json(
        &pack_metadata_path(pack_path),
        &PackMetadataFile {
            schema_version: SCHEMA_VERSION,
            data: metadata.clone(),
        },
    )
}

pub fn load_cards(pack_path: &Path) -> AppResult<Vec<CardEntity>> {
    let path = cards_path(pack_path);
    let file: CardsFile = read_json(&path)?;
    ensure_schema(path.as_path(), file.schema_version)?;
    Ok(file.cards)
}

pub fn save_cards(pack_path: &Path, cards: &[CardEntity]) -> AppResult<()> {
    write_json(
        &cards_path(pack_path),
        &CardsFile {
            schema_version: SCHEMA_VERSION,
            cards: cards.to_vec(),
        },
    )
}

pub fn load_pack_strings(pack_path: &Path) -> AppResult<PackStringsFile> {
    let path = pack_strings_path(pack_path);
    if !path.exists() {
        return Ok(PackStringsFile::default());
    }
    let file: PackStringsFile = read_json(&path)?;
    ensure_schema(path.as_path(), file.schema_version)?;
    Ok(file)
}

pub fn save_pack_strings(pack_path: &Path, strings: &PackStringsFile) -> AppResult<()> {
    write_json(&pack_strings_path(pack_path), strings)
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    let encoded = serde_json::to_vec_pretty(value).map_err(|source| {
        AppError::new("json_store.serialize_failed", source.to_string())
            .with_detail("path", path.display().to_string())
    })?;
    safe_write_bytes(path, &encoded)
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> AppResult<T> {
    let raw = fs::read(path).map_err(|source| {
        AppError::from_io("json_store.read_failed", source)
            .with_detail("path", path.display().to_string())
    })?;
    serde_json::from_slice(&raw).map_err(|source| {
        AppError::new("json_store.deserialize_failed", source.to_string())
            .with_detail("path", path.display().to_string())
    })
}

fn ensure_schema(path: &Path, schema_version: u32) -> AppResult<()> {
    if schema_version == SCHEMA_VERSION {
        return Ok(());
    }

    Err(
        AppError::new("json_store.schema_mismatch", "schema version mismatch")
            .with_detail("path", path.display().to_string())
            .with_detail("expected", SCHEMA_VERSION)
            .with_detail("actual", schema_version),
    )
}
