use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::domain::common::ids::LanguageCode;

pub const PACK_STRINGS_SCHEMA_VERSION: u32 = 2;

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
pub struct PackStringRecord {
    pub kind: PackStringKind,
    pub key: u32,
    pub values: BTreeMap<LanguageCode, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackStringsFile {
    pub schema_version: u32,
    pub entries: Vec<PackStringRecord>,
}

impl Default for PackStringsFile {
    fn default() -> Self {
        Self {
            schema_version: PACK_STRINGS_SCHEMA_VERSION,
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpsertPackStringTranslationOutcome {
    NoChange,
    Inserted,
    Updated { previous_value: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpsertPackStringRecordOutcome {
    NoChange,
    Inserted,
    Replaced { previous: PackStringRecord },
}

impl PackStringsFile {
    pub fn sort_entries(&mut self) {
        self.entries
            .sort_by(|left, right| left.kind.cmp(&right.kind).then(left.key.cmp(&right.key)));
    }

    pub fn language_entry_count(&self, language: &str) -> usize {
        self.entries
            .iter()
            .filter(|record| record.values.contains_key(language))
            .count()
    }

    pub fn project_language_entries(&self, language: &str) -> Vec<PackStringEntry> {
        let mut items = self
            .entries
            .iter()
            .filter_map(|record| {
                record.values.get(language).map(|value| PackStringEntry {
                    kind: record.kind.clone(),
                    key: record.key,
                    value: value.clone(),
                })
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.kind.cmp(&right.kind).then(left.key.cmp(&right.key)));
        items
    }

    pub fn get_record(&self, kind: &PackStringKind, key: u32) -> Option<&PackStringRecord> {
        self.entries
            .iter()
            .find(|record| &record.kind == kind && record.key == key)
    }

    pub fn get_record_mut(
        &mut self,
        kind: &PackStringKind,
        key: u32,
    ) -> Option<&mut PackStringRecord> {
        self.entries
            .iter_mut()
            .find(|record| &record.kind == kind && record.key == key)
    }

    pub fn upsert_translation(
        &mut self,
        language: &str,
        entry: &PackStringEntry,
    ) -> UpsertPackStringTranslationOutcome {
        if let Some(record) = self.get_record_mut(&entry.kind, entry.key) {
            if let Some(existing) = record.values.get(language) {
                if existing == &entry.value {
                    return UpsertPackStringTranslationOutcome::NoChange;
                }
                let previous_value = existing.clone();
                record.values.insert(language.to_string(), entry.value.clone());
                return UpsertPackStringTranslationOutcome::Updated { previous_value };
            }
            record.values.insert(language.to_string(), entry.value.clone());
            return UpsertPackStringTranslationOutcome::Inserted;
        }

        let mut values = BTreeMap::new();
        values.insert(language.to_string(), entry.value.clone());
        self.entries.push(PackStringRecord {
            kind: entry.kind.clone(),
            key: entry.key,
            values,
        });
        self.sort_entries();
        UpsertPackStringTranslationOutcome::Inserted
    }

    pub fn upsert_record(
        &mut self,
        record: PackStringRecord,
    ) -> UpsertPackStringRecordOutcome {
        if let Some(existing) = self.get_record_mut(&record.kind, record.key) {
            if existing == &record {
                return UpsertPackStringRecordOutcome::NoChange;
            }
            let previous = existing.clone();
            *existing = record;
            return UpsertPackStringRecordOutcome::Replaced { previous };
        }

        self.entries.push(record);
        self.sort_entries();
        UpsertPackStringRecordOutcome::Inserted
    }

    pub fn delete_records(&mut self, keys: &[(PackStringKind, u32)]) -> usize {
        let original_len = self.entries.len();
        self.entries.retain(|record| {
            !keys
                .iter()
                .any(|(kind, key)| &record.kind == kind && record.key == *key)
        });
        original_len.saturating_sub(self.entries.len())
    }

    pub fn remove_translation(
        &mut self,
        kind: &PackStringKind,
        key: u32,
        language: &str,
    ) -> RemovePackStringTranslationOutcome {
        let Some(index) = self
            .entries
            .iter()
            .position(|record| &record.kind == kind && record.key == key)
        else {
            return RemovePackStringTranslationOutcome::NoChange;
        };

        let record = &mut self.entries[index];
        if record.values.remove(language).is_none() {
            return RemovePackStringTranslationOutcome::NoChange;
        }

        if record.values.is_empty() {
            self.entries.remove(index);
            return RemovePackStringTranslationOutcome::DeletedRecord;
        }

        RemovePackStringTranslationOutcome::Updated(record.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemovePackStringTranslationOutcome {
    NoChange,
    Updated(PackStringRecord),
    DeletedRecord,
}
