use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};

use crate::domain::common::error::{AppError, AppResult};
use crate::infrastructure::standard_pack::{
    StandardCardIndexRecord, StandardPackIndexFile, load_index, standard_pack_index_path,
};

#[derive(Clone, Debug, Default)]
pub struct StandardPackIndexCache {
    inner: Arc<RwLock<Option<CachedStandardPackIndex>>>,
}

#[derive(Debug, Clone)]
pub struct StandardPackIndexSnapshot {
    index: Arc<StandardPackIndexFile>,
    cards_by_code: Arc<BTreeMap<u32, usize>>,
}

impl StandardPackIndexSnapshot {
    pub fn index(&self) -> &StandardPackIndexFile {
        &self.index
    }

    pub fn card_by_code(&self, code: u32) -> Option<&StandardCardIndexRecord> {
        self.cards_by_code
            .get(&code)
            .and_then(|index| self.index.cards.get(*index))
    }
}

#[derive(Debug, Clone)]
struct CachedStandardPackIndex {
    file_stamp: IndexFileStamp,
    snapshot: StandardPackIndexSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IndexFileStamp {
    len: u64,
    modified: Option<std::time::SystemTime>,
}

impl StandardPackIndexCache {
    pub fn get_or_load(&self, app_data_dir: &Path) -> AppResult<StandardPackIndexSnapshot> {
        let current_stamp = index_file_stamp(app_data_dir)?;
        if let Some(snapshot) = self.cached_snapshot(current_stamp)? {
            return Ok(snapshot);
        }

        let index = load_index(app_data_dir)?;
        let snapshot = build_snapshot(index);
        let mut guard = self.write_inner()?;
        *guard = Some(CachedStandardPackIndex {
            file_stamp: current_stamp,
            snapshot: snapshot.clone(),
        });
        Ok(snapshot)
    }

    pub fn replace(
        &self,
        app_data_dir: &Path,
        index: StandardPackIndexFile,
    ) -> AppResult<StandardPackIndexSnapshot> {
        let file_stamp = index_file_stamp(app_data_dir)?;
        let snapshot = build_snapshot(index);
        let mut guard = self.write_inner()?;
        *guard = Some(CachedStandardPackIndex {
            file_stamp,
            snapshot: snapshot.clone(),
        });
        Ok(snapshot)
    }

    pub fn clear(&self) -> AppResult<()> {
        let mut guard = self.write_inner()?;
        *guard = None;
        Ok(())
    }

    fn cached_snapshot(
        &self,
        current_stamp: IndexFileStamp,
    ) -> AppResult<Option<StandardPackIndexSnapshot>> {
        let guard = self.read_inner()?;
        Ok(guard
            .as_ref()
            .filter(|cached| cached.file_stamp == current_stamp)
            .map(|cached| cached.snapshot.clone()))
    }

    fn read_inner(
        &self,
    ) -> AppResult<std::sync::RwLockReadGuard<'_, Option<CachedStandardPackIndex>>> {
        self.inner.read().map_err(|_| {
            AppError::new(
                "standard_pack.cache_lock_poisoned",
                "standard pack index cache lock poisoned",
            )
        })
    }

    fn write_inner(
        &self,
    ) -> AppResult<std::sync::RwLockWriteGuard<'_, Option<CachedStandardPackIndex>>> {
        self.inner.write().map_err(|_| {
            AppError::new(
                "standard_pack.cache_lock_poisoned",
                "standard pack index cache lock poisoned",
            )
        })
    }
}

fn build_snapshot(index: StandardPackIndexFile) -> StandardPackIndexSnapshot {
    let cards_by_code = index
        .cards
        .iter()
        .enumerate()
        .map(|(record_index, record)| (record.card.code, record_index))
        .collect::<BTreeMap<_, _>>();
    StandardPackIndexSnapshot {
        index: Arc::new(index),
        cards_by_code: Arc::new(cards_by_code),
    }
}

fn index_file_stamp(app_data_dir: &Path) -> AppResult<IndexFileStamp> {
    let path = standard_pack_index_path(app_data_dir);
    let metadata = fs::metadata(&path).map_err(|source| {
        AppError::from_io("standard_pack.index_metadata_failed", source)
            .with_detail("path", path.display().to_string())
    })?;
    Ok(IndexFileStamp {
        len: metadata.len(),
        modified: metadata.modified().ok(),
    })
}
