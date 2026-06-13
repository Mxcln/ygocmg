use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::domain::common::ids::PackId;
use crate::domain::strings::model::{PackStringKind, PackStringRecord, PackStringsFile};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StandardNamespaceBaseline {
    pub standard_codes: BTreeSet<u32>,
    pub strings: StandardStringNamespaceBaseline,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StandardStringNamespaceBaseline {
    pub system_keys: BTreeSet<u32>,
    pub victory_keys: BTreeSet<u32>,
    pub counter_keys: BTreeSet<u32>,
    #[serde(default)]
    pub setname_keys: BTreeSet<u32>,
    pub setname_bases: BTreeSet<u16>,
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceNamespaceIndex {
    pub codes_by_pack: BTreeMap<PackId, BTreeSet<u32>>,
    pub strings_by_pack: BTreeMap<PackId, PackStringNamespaceIndex>,
}

#[derive(Debug, Clone, Default)]
pub struct PackStringNamespaceIndex {
    pub system_keys: BTreeSet<u32>,
    pub victory_keys: BTreeSet<u32>,
    pub counter_keys: BTreeSet<u32>,
    pub setname_keys: BTreeSet<u32>,
    pub setname_bases: BTreeSet<u16>,
}

#[derive(Debug, Clone, Default)]
pub struct PackStringsNamespaceContext {
    pub other_custom: PackStringNamespaceIndex,
    pub standard: StandardStringNamespaceBaseline,
    /// Recommended custom setname base range (from global config), driving the
    /// out-of-range warning. Defaults via with_setname_base_range below.
    pub setname_base_recommended_min: u16,
    pub setname_base_recommended_max: u16,
}

impl PackStringNamespaceIndex {
    pub fn insert_record(&mut self, record: &PackStringRecord) {
        match record.kind {
            PackStringKind::System => {
                self.system_keys.insert(record.key);
            }
            PackStringKind::Victory => {
                self.victory_keys.insert(record.key);
            }
            PackStringKind::Counter => {
                self.counter_keys.insert(record.key);
            }
            PackStringKind::Setname => {
                self.setname_keys.insert(record.key);
                self.setname_bases.insert(setname_base(record.key));
            }
        }
    }

    pub fn extend(&mut self, other: &Self) {
        self.system_keys.extend(other.system_keys.iter().copied());
        self.victory_keys.extend(other.victory_keys.iter().copied());
        self.counter_keys.extend(other.counter_keys.iter().copied());
        self.setname_keys.extend(other.setname_keys.iter().copied());
        self.setname_bases
            .extend(other.setname_bases.iter().copied());
    }
}

pub fn setname_base(key: u32) -> u16 {
    (key & 0x0fff) as u16
}

/// Pick the next free top-level setname base (child = 0) within the recommended
/// range `[min, max]`, skipping any base already in `used`. Returns `None` when
/// the range is exhausted or invalid.
pub fn suggest_next_setname_base(used: &BTreeSet<u16>, min: u16, max: u16) -> Option<u16> {
    if min > max {
        return None;
    }
    (min..=max).find(|base| !used.contains(base))
}

pub fn counter_low12(key: u32) -> u16 {
    (key & 0x0fff) as u16
}

pub fn build_pack_strings_namespace_index(strings: &PackStringsFile) -> PackStringNamespaceIndex {
    let mut index = PackStringNamespaceIndex::default();
    for record in &strings.entries {
        index.insert_record(record);
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    fn used(values: &[u16]) -> BTreeSet<u16> {
        values.iter().copied().collect()
    }

    #[test]
    fn empty_used_returns_min() {
        assert_eq!(
            suggest_next_setname_base(&BTreeSet::new(), 0x0300, 0x0fff),
            Some(0x0300)
        );
    }

    #[test]
    fn skips_contiguous_used_bases() {
        let used = used(&[0x0300, 0x0301, 0x0302]);
        assert_eq!(
            suggest_next_setname_base(&used, 0x0300, 0x0fff),
            Some(0x0303)
        );
    }

    #[test]
    fn skips_standard_and_other_pack_bases() {
        // Official bases (<= 0x1DE) won't appear in range, but a custom base from
        // another pack inside the range must be skipped.
        let used = used(&[0x0300, 0x0305]);
        assert_eq!(
            suggest_next_setname_base(&used, 0x0300, 0x0fff),
            Some(0x0301)
        );
    }

    #[test]
    fn exhausted_range_returns_none() {
        let used = used(&[0x0300, 0x0301, 0x0302]);
        assert_eq!(suggest_next_setname_base(&used, 0x0300, 0x0302), None);
    }

    #[test]
    fn invalid_range_returns_none() {
        assert_eq!(suggest_next_setname_base(&BTreeSet::new(), 0x0fff, 0x0300), None);
    }
}
