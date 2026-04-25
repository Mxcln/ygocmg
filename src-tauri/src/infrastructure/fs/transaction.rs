use std::fs;
use std::path::PathBuf;

use crate::domain::common::error::{AppError, AppResult};
use crate::infrastructure::fs::safe_write::safe_write_bytes;

#[derive(Debug, Clone)]
pub enum FsOperation {
    CreateDir { path: PathBuf },
    WriteFile { path: PathBuf, contents: Vec<u8> },
    Rename { from: PathBuf, to: PathBuf },
}

#[derive(Debug)]
enum AppliedOperation {
    CreatedDir { path: PathBuf, created: bool },
    WroteFile { path: PathBuf, original: Option<Vec<u8>> },
    Renamed { from: PathBuf, to: PathBuf },
}

pub fn execute_plan(operations: Vec<FsOperation>) -> AppResult<()> {
    let mut applied = Vec::new();

    for operation in operations {
        let applied_operation = match operation {
            FsOperation::CreateDir { path } => {
                let created = !path.exists();
                fs::create_dir_all(&path).map_err(|source| {
                    AppError::from_io("fs.plan_create_dir_failed", source)
                        .with_detail("path", path.display().to_string())
                })?;
                AppliedOperation::CreatedDir { path, created }
            }
            FsOperation::WriteFile { path, contents } => {
                let original = if path.exists() {
                    Some(fs::read(&path).map_err(|source| {
                        AppError::from_io("fs.plan_read_original_failed", source)
                            .with_detail("path", path.display().to_string())
                    })?)
                } else {
                    None
                };
                safe_write_bytes(&path, &contents)?;
                AppliedOperation::WroteFile { path, original }
            }
            FsOperation::Rename { from, to } => {
                if !from.exists() {
                    continue;
                }
                if to.exists() {
                    rollback(&mut applied);
                    return Err(
                        AppError::new(
                            "fs.plan_rename_target_exists",
                            "rename target already exists",
                        )
                        .with_detail("from", from.display().to_string())
                        .with_detail("to", to.display().to_string()),
                    );
                }
                if let Some(parent) = to.parent() {
                    fs::create_dir_all(parent).map_err(|source| {
                        AppError::from_io("fs.plan_create_rename_parent_failed", source)
                            .with_detail("path", parent.display().to_string())
                    })?;
                }
                fs::rename(&from, &to).map_err(|source| {
                    rollback(&mut applied);
                    AppError::from_io("fs.plan_rename_failed", source)
                        .with_detail("from", from.display().to_string())
                        .with_detail("to", to.display().to_string())
                })?;
                AppliedOperation::Renamed { from, to }
            }
        };

        applied.push(applied_operation);
    }

    Ok(())
}

fn rollback(applied: &mut Vec<AppliedOperation>) {
    while let Some(operation) = applied.pop() {
        match operation {
            AppliedOperation::CreatedDir { path, created } => {
                if created {
                    let _ = fs::remove_dir(&path);
                }
            }
            AppliedOperation::WroteFile { path, original } => {
                if let Some(original) = original {
                    let _ = safe_write_bytes(&path, &original);
                } else if path.exists() {
                    let _ = fs::remove_file(&path);
                }
            }
            AppliedOperation::Renamed { from, to } => {
                if to.exists() && !from.exists() {
                    let _ = fs::rename(&to, &from);
                }
            }
        }
    }
}
