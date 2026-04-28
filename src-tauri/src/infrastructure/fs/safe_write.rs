use std::fs;
use std::io::Write;
use std::path::Path;

use uuid::Uuid;

use crate::domain::common::error::{AppError, AppResult};

pub fn safe_write_bytes(path: &Path, contents: &[u8]) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| {
            AppError::from_io("fs.parent_create_failed", source)
                .with_detail("path", parent.display().to_string())
        })?;
    }

    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("file");

    let temp_path = path.with_file_name(format!("{file_name}.tmp-{}", Uuid::now_v7()));
    let backup_path = path.with_file_name(format!("{file_name}.bak-{}", Uuid::now_v7()));

    {
        let mut file = fs::File::create(&temp_path).map_err(|source| {
            AppError::from_io("fs.temp_create_failed", source)
                .with_detail("path", temp_path.display().to_string())
        })?;
        file.write_all(contents).map_err(|source| {
            AppError::from_io("fs.temp_write_failed", source)
                .with_detail("path", temp_path.display().to_string())
        })?;
        file.sync_all().map_err(|source| {
            AppError::from_io("fs.temp_sync_failed", source)
                .with_detail("path", temp_path.display().to_string())
        })?;
    }

    let target_exists = path.exists();
    if target_exists {
        fs::rename(path, &backup_path).map_err(|source| {
            let _ = fs::remove_file(&temp_path);
            AppError::from_io("fs.backup_rename_failed", source)
                .with_detail("path", path.display().to_string())
                .with_detail("backup_path", backup_path.display().to_string())
        })?;
    }

    if let Err(source) = fs::rename(&temp_path, path) {
        if target_exists && backup_path.exists() {
            let _ = fs::rename(&backup_path, path);
        }
        let _ = fs::remove_file(&temp_path);
        return Err(AppError::from_io("fs.commit_rename_failed", source)
            .with_detail("path", path.display().to_string())
            .with_detail("temp_path", temp_path.display().to_string()));
    }

    if target_exists {
        fs::remove_file(&backup_path).map_err(|source| {
            AppError::from_io("fs.backup_cleanup_failed", source)
                .with_detail("path", backup_path.display().to_string())
        })?;
    }

    Ok(())
}

pub fn safe_write_string(path: &Path, contents: &str) -> AppResult<()> {
    safe_write_bytes(path, contents.as_bytes())
}
