use std::collections::BTreeMap;

use crate::domain::card::model::{CardEntity, CardUpdateInput};
use crate::domain::common::ids::{CardId, ConfirmationToken, PackId, WorkspaceId};
use crate::domain::common::issue::ValidationIssue;

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

#[derive(Debug, Default)]
pub struct ConfirmationCache {
    card_entries: BTreeMap<ConfirmationToken, CardConfirmationEntry>,
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
    pub fn invalidate_pack(&mut self, workspace_id: &str, pack_id: &str) {
        self.card_entries.retain(|_, entry| {
            !(entry.workspace_id == workspace_id && entry.pack_id == pack_id)
        });
    }

    pub fn invalidate_workspace(&mut self, workspace_id: &str) {
        self.card_entries
            .retain(|_, entry| entry.workspace_id != workspace_id);
    }

    pub fn clear(&mut self) {
        self.card_entries.clear();
    }

    pub fn debug_get_card_entry(
        &self,
        token: &ConfirmationToken,
    ) -> Option<&CardConfirmationEntry> {
        self.card_entries.get(token)
    }
}
