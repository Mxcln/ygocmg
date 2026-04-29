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
