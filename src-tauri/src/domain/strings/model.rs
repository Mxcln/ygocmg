use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::domain::common::ids::LanguageCode;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PackStringKind {
    System,
    Victory,
    Counter,
    Setname,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackStringEntry {
    pub kind: PackStringKind,
    pub key: u32,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackStringsFile {
    pub schema_version: u32,
    pub entries: BTreeMap<LanguageCode, Vec<PackStringEntry>>,
}

impl Default for PackStringsFile {
    fn default() -> Self {
        Self {
            schema_version: 1,
            entries: BTreeMap::new(),
        }
    }
}
