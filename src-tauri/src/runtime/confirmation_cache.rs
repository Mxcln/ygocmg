use std::collections::BTreeMap;

use crate::domain::card::model::{CardEntity, CardUpdateInput};
use crate::domain::common::ids::{CardId, ConfirmationToken, PackId, WorkspaceId};
use crate::domain::common::issue::ValidationIssue;
use crate::domain::strings::model::{PackStringEntry, PackStringRecord};

#[derive(Debug, Clone)]
pub enum CardConfirmationOperationKind {
    CreateCard,
    UpdateCard,
}

#[derive(Debug, Clone)]
pub struct CardConfirmationInputSnapshot {
    pub card_id: Option<CardId>,
    pub card: CardUpdateInput,
    pub create_card_seed: Option<CardEntity>,
}

#[derive(Debug, Clone)]
pub struct CardConfirmationEntry {
    pub confirmation_token: ConfirmationToken,
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub pack_revision: u64,
    pub source_stamp: String,
    pub operation_kind: CardConfirmationOperationKind,
    pub input_snapshot: CardConfirmationInputSnapshot,
    pub warnings: Vec<ValidationIssue>,
}

#[derive(Debug, Clone)]
pub enum PackStringsConfirmationOperationKind {
    UpsertPackString,
}

#[derive(Debug, Clone)]
pub struct PackStringsConfirmationInputSnapshot {
    pub language: String,
    pub entry: PackStringEntry,
}

#[derive(Debug, Clone)]
pub struct PackStringRecordConfirmationInputSnapshot {
    pub record: PackStringRecord,
}

#[derive(Debug, Clone)]
pub struct PackStringsConfirmationEntry {
    pub confirmation_token: ConfirmationToken,
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub pack_revision: u64,
    pub source_stamp: String,
    pub operation_kind: PackStringsConfirmationOperationKind,
    pub input_snapshot: PackStringsConfirmationInputSnapshot,
    pub warnings: Vec<ValidationIssue>,
}

#[derive(Debug, Clone)]
pub struct PackStringRecordConfirmationEntry {
    pub confirmation_token: ConfirmationToken,
    pub workspace_id: WorkspaceId,
    pub pack_id: PackId,
    pub pack_revision: u64,
    pub source_stamp: String,
    pub input_snapshot: PackStringRecordConfirmationInputSnapshot,
    pub warnings: Vec<ValidationIssue>,
}

#[derive(Debug, Default)]
pub struct ConfirmationCache {
    card_entries: BTreeMap<ConfirmationToken, CardConfirmationEntry>,
    pack_strings_entries: BTreeMap<ConfirmationToken, PackStringsConfirmationEntry>,
    pack_string_record_entries: BTreeMap<ConfirmationToken, PackStringRecordConfirmationEntry>,
}

impl ConfirmationCache {
    pub fn insert_card_entry(&mut self, entry: CardConfirmationEntry) {
        self.card_entries
            .insert(entry.confirmation_token.clone(), entry);
    }

    pub fn remove_card_entry(
        &mut self,
        token: &ConfirmationToken,
    ) -> Option<CardConfirmationEntry> {
        self.card_entries.remove(token)
    }

    pub fn insert_pack_strings_entry(&mut self, entry: PackStringsConfirmationEntry) {
        self.pack_strings_entries
            .insert(entry.confirmation_token.clone(), entry);
    }

    pub fn remove_pack_strings_entry(
        &mut self,
        token: &ConfirmationToken,
    ) -> Option<PackStringsConfirmationEntry> {
        self.pack_strings_entries.remove(token)
    }

    pub fn insert_pack_string_record_entry(
        &mut self,
        entry: PackStringRecordConfirmationEntry,
    ) {
        self.pack_string_record_entries
            .insert(entry.confirmation_token.clone(), entry);
    }

    pub fn remove_pack_string_record_entry(
        &mut self,
        token: &ConfirmationToken,
    ) -> Option<PackStringRecordConfirmationEntry> {
        self.pack_string_record_entries.remove(token)
    }

    pub fn invalidate_pack(&mut self, workspace_id: &str, pack_id: &str) {
        self.card_entries.retain(|_, entry| {
            !(entry.workspace_id == workspace_id && entry.pack_id == pack_id)
        });
        self.pack_strings_entries.retain(|_, entry| {
            !(entry.workspace_id == workspace_id && entry.pack_id == pack_id)
        });
        self.pack_string_record_entries.retain(|_, entry| {
            !(entry.workspace_id == workspace_id && entry.pack_id == pack_id)
        });
    }

    pub fn invalidate_workspace(&mut self, workspace_id: &str) {
        self.card_entries
            .retain(|_, entry| entry.workspace_id != workspace_id);
        self.pack_strings_entries
            .retain(|_, entry| entry.workspace_id != workspace_id);
        self.pack_string_record_entries
            .retain(|_, entry| entry.workspace_id != workspace_id);
    }

    pub fn clear(&mut self) {
        self.card_entries.clear();
        self.pack_strings_entries.clear();
        self.pack_string_record_entries.clear();
    }

    pub fn debug_get_card_entry(
        &self,
        token: &ConfirmationToken,
    ) -> Option<&CardConfirmationEntry> {
        self.card_entries.get(token)
    }

    pub fn debug_get_pack_strings_entry(
        &self,
        token: &ConfirmationToken,
    ) -> Option<&PackStringsConfirmationEntry> {
        self.pack_strings_entries.get(token)
    }

    pub fn debug_get_pack_string_record_entry(
        &self,
        token: &ConfirmationToken,
    ) -> Option<&PackStringRecordConfirmationEntry> {
        self.pack_string_record_entries.get(token)
    }
}
