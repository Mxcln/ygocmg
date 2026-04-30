use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};

use crate::domain::common::error::{AppError, AppResult};
use crate::domain::namespace::model::StandardNamespaceBaseline;
use crate::infrastructure::standard_pack::sqlite_store::{
    StandardPackSqliteManifest, StandardSetnameRecord, standard_pack_sqlite_path,
};

#[derive(Clone, Debug, Default)]
pub struct StandardPackRuntimeCache {
    inner: Arc<RwLock<CachedStandardPackRuntime>>,
}

#[derive(Debug, Default)]
struct CachedStandardPackRuntime {
    manifest: Option<CachedValue<StandardPackSqliteManifest>>,
    namespace_baseline: Option<CachedValue<StandardNamespaceBaseline>>,
    setnames_by_language: BTreeMap<String, CachedValue<Vec<StandardSetnameRecord>>>,
}

#[derive(Debug, Clone)]
struct CachedValue<T> {
    file_stamp: IndexFileStamp,
    value: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IndexFileStamp {
    len: u64,
    modified: Option<std::time::SystemTime>,
}

impl StandardPackRuntimeCache {
    pub fn manifest<F>(&self, app_data_dir: &Path, load: F) -> AppResult<StandardPackSqliteManifest>
    where
        F: FnOnce() -> AppResult<StandardPackSqliteManifest>,
    {
        let current_stamp = sqlite_file_stamp(app_data_dir)?;
        {
            let guard = self.read_inner()?;
            if let Some(cached) = guard
                .manifest
                .as_ref()
                .filter(|cached| cached.file_stamp == current_stamp)
            {
                return Ok(cached.value.clone());
            }
        }

        let value = load()?;
        let mut guard = self.write_inner()?;
        guard.manifest = Some(CachedValue {
            file_stamp: current_stamp,
            value: value.clone(),
        });
        Ok(value)
    }

    pub fn namespace_baseline<F>(
        &self,
        app_data_dir: &Path,
        load: F,
    ) -> AppResult<StandardNamespaceBaseline>
    where
        F: FnOnce() -> AppResult<StandardNamespaceBaseline>,
    {
        let current_stamp = sqlite_file_stamp(app_data_dir)?;
        {
            let guard = self.read_inner()?;
            if let Some(cached) = guard
                .namespace_baseline
                .as_ref()
                .filter(|cached| cached.file_stamp == current_stamp)
            {
                return Ok(cached.value.clone());
            }
        }

        let value = load()?;
        let mut guard = self.write_inner()?;
        guard.namespace_baseline = Some(CachedValue {
            file_stamp: current_stamp,
            value: value.clone(),
        });
        Ok(value)
    }

    pub fn setnames<F>(
        &self,
        app_data_dir: &Path,
        language: &str,
        load: F,
    ) -> AppResult<Vec<StandardSetnameRecord>>
    where
        F: FnOnce() -> AppResult<Vec<StandardSetnameRecord>>,
    {
        let current_stamp = sqlite_file_stamp(app_data_dir)?;
        {
            let guard = self.read_inner()?;
            if let Some(cached) = guard
                .setnames_by_language
                .get(language)
                .filter(|cached| cached.file_stamp == current_stamp)
            {
                return Ok(cached.value.clone());
            }
        }

        let value = load()?;
        let mut guard = self.write_inner()?;
        guard.setnames_by_language.insert(
            language.to_string(),
            CachedValue {
                file_stamp: current_stamp,
                value: value.clone(),
            },
        );
        Ok(value)
    }

    pub fn clear(&self) -> AppResult<()> {
        let mut guard = self.write_inner()?;
        *guard = CachedStandardPackRuntime::default();
        Ok(())
    }

    fn read_inner(&self) -> AppResult<std::sync::RwLockReadGuard<'_, CachedStandardPackRuntime>> {
        self.inner.read().map_err(|_| {
            AppError::new(
                "standard_pack.cache_lock_poisoned",
                "standard pack runtime cache lock poisoned",
            )
        })
    }

    fn write_inner(&self) -> AppResult<std::sync::RwLockWriteGuard<'_, CachedStandardPackRuntime>> {
        self.inner.write().map_err(|_| {
            AppError::new(
                "standard_pack.cache_lock_poisoned",
                "standard pack runtime cache lock poisoned",
            )
        })
    }
}

fn sqlite_file_stamp(app_data_dir: &Path) -> AppResult<IndexFileStamp> {
    let path = standard_pack_sqlite_path(app_data_dir);
    let metadata = fs::metadata(&path).map_err(|source| {
        AppError::from_io("standard_pack.sqlite_missing", source)
            .with_detail("path", path.display().to_string())
    })?;
    Ok(IndexFileStamp {
        len: metadata.len(),
        modified: metadata.modified().ok(),
    })
}
