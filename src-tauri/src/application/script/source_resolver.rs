use std::path::PathBuf;

use crate::application::script::dto::ValidateLuaScriptInput;
use crate::bootstrap::AppState;
use crate::domain::card::model::CardEntity;
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::pack::model::PackKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptSourceKind {
    Draft,
    Saved,
}

#[derive(Debug, Clone)]
pub struct ResolvedScriptSource {
    pub card: CardEntity,
    pub script_text: String,
    pub source_kind: ScriptSourceKind,
    pub script_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum ScriptSourceResolution {
    Ready(ResolvedScriptSource),
    MissingScript {
        card_code: u32,
        script_path: PathBuf,
    },
    ReadFailed {
        card_code: u32,
        script_path: PathBuf,
        message: String,
    },
}

pub struct ScriptSourceResolver<'a> {
    state: &'a AppState,
}

impl<'a> ScriptSourceResolver<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub fn resolve(&self, input: &ValidateLuaScriptInput) -> AppResult<ScriptSourceResolution> {
        let snapshot = crate::application::pack::service::require_open_pack_snapshot(
            self.state,
            &input.workspace_id,
            &input.pack_id,
        )?;
        if snapshot.metadata.kind != PackKind::Custom {
            return Err(AppError::new(
                "script_validation.pack_not_custom",
                "Lua script validation is only available for custom packs",
            ));
        }
        let card = snapshot
            .cards
            .iter()
            .find(|card| card.id == input.card_id)
            .cloned()
            .ok_or_else(|| AppError::new("card.not_found", "card was not found"))?;

        if let Some(script_text) = &input.script_text {
            return Ok(ScriptSourceResolution::Ready(ResolvedScriptSource {
                card,
                script_text: script_text.clone(),
                source_kind: ScriptSourceKind::Draft,
                script_path: None,
            }));
        }

        let script_path =
            crate::domain::resource::path_rules::script_path(&snapshot.pack_path, card.code);
        if !script_path.exists() {
            return Ok(ScriptSourceResolution::MissingScript {
                card_code: card.code,
                script_path,
            });
        }

        match std::fs::read_to_string(&script_path) {
            Ok(script_text) => Ok(ScriptSourceResolution::Ready(ResolvedScriptSource {
                card,
                script_text,
                source_kind: ScriptSourceKind::Saved,
                script_path: Some(script_path),
            })),
            Err(source) => Ok(ScriptSourceResolution::ReadFailed {
                card_code: card.code,
                script_path,
                message: source.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tempfile::tempdir;

    use super::*;
    use crate::application::script::dto::ValidateLuaScriptInput;
    use crate::bootstrap::AppState;
    use crate::domain::card::model::{CardEntity, Ot, PrimaryType};
    use crate::domain::common::time::now_utc;
    use crate::domain::pack::model::{PackKind, PackMetadata, PackOverview};
    use crate::domain::strings::model::PackStringsFile;
    use crate::domain::workspace::model::WorkspaceMeta;
    use crate::runtime::sessions::{PackSession, WorkspaceSession};

    #[test]
    fn draft_script_text_wins_without_saved_file() {
        let (_temp, state) = state_with_open_pack(false);
        let input = ValidateLuaScriptInput {
            workspace_id: "workspace-1".to_string(),
            pack_id: "pack-1".to_string(),
            card_id: "card-1".to_string(),
            script_text: Some("draft script".to_string()),
            levels: None,
        };

        let resolved = ScriptSourceResolver::new(&state).resolve(&input).unwrap();

        match resolved {
            ScriptSourceResolution::Ready(source) => {
                assert_eq!(source.card.code, 99999999);
                assert_eq!(source.script_text, "draft script");
                assert_eq!(source.source_kind, ScriptSourceKind::Draft);
                assert!(source.script_path.is_none());
            }
            other => panic!("expected ready source, got {other:#?}"),
        }
    }

    #[test]
    fn saved_script_is_read_from_pack_scripts_dir() {
        let (_temp, state) = state_with_open_pack(true);
        let input = ValidateLuaScriptInput {
            workspace_id: "workspace-1".to_string(),
            pack_id: "pack-1".to_string(),
            card_id: "card-1".to_string(),
            script_text: None,
            levels: None,
        };

        let resolved = ScriptSourceResolver::new(&state).resolve(&input).unwrap();

        match resolved {
            ScriptSourceResolution::Ready(source) => {
                assert!(source.script_text.contains("function s.initial_effect"));
                assert_eq!(source.source_kind, ScriptSourceKind::Saved);
                assert!(
                    source
                        .script_path
                        .unwrap()
                        .ends_with("scripts/c99999999.lua")
                );
            }
            other => panic!("expected ready source, got {other:#?}"),
        }
    }

    #[test]
    fn missing_saved_script_returns_missing_resolution() {
        let (_temp, state) = state_with_open_pack(false);
        let input = ValidateLuaScriptInput {
            workspace_id: "workspace-1".to_string(),
            pack_id: "pack-1".to_string(),
            card_id: "card-1".to_string(),
            script_text: None,
            levels: None,
        };

        let resolved = ScriptSourceResolver::new(&state).resolve(&input).unwrap();

        match resolved {
            ScriptSourceResolution::MissingScript {
                card_code,
                script_path,
            } => {
                assert_eq!(card_code, 99999999);
                assert!(script_path.ends_with("scripts/c99999999.lua"));
            }
            other => panic!("expected missing script, got {other:#?}"),
        }
    }

    #[test]
    fn non_custom_pack_returns_pack_not_custom_error() {
        let (_temp, state) = state_with_open_pack_kind(PackKind::Standard, false);
        let input = ValidateLuaScriptInput {
            workspace_id: "workspace-1".to_string(),
            pack_id: "pack-1".to_string(),
            card_id: "card-1".to_string(),
            script_text: Some("draft script".to_string()),
            levels: None,
        };

        let error = ScriptSourceResolver::new(&state)
            .resolve(&input)
            .unwrap_err();

        assert_eq!(error.code, "script_validation.pack_not_custom");
    }

    #[test]
    fn missing_card_returns_card_not_found_error() {
        let (_temp, state) = state_with_open_pack(false);
        let input = ValidateLuaScriptInput {
            workspace_id: "workspace-1".to_string(),
            pack_id: "pack-1".to_string(),
            card_id: "missing-card".to_string(),
            script_text: Some("draft script".to_string()),
            levels: None,
        };

        let error = ScriptSourceResolver::new(&state)
            .resolve(&input)
            .unwrap_err();

        assert_eq!(error.code, "card.not_found");
    }

    #[test]
    fn saved_script_read_failure_returns_read_failed_resolution() {
        let (_temp, state) = state_with_unreadable_saved_script();
        let input = ValidateLuaScriptInput {
            workspace_id: "workspace-1".to_string(),
            pack_id: "pack-1".to_string(),
            card_id: "card-1".to_string(),
            script_text: None,
            levels: None,
        };

        let resolved = ScriptSourceResolver::new(&state).resolve(&input).unwrap();

        match resolved {
            ScriptSourceResolution::ReadFailed {
                card_code,
                script_path,
                message,
            } => {
                assert_eq!(card_code, 99999999);
                assert!(script_path.ends_with("scripts/c99999999.lua"));
                assert!(!message.is_empty());
            }
            other => panic!("expected read failed source, got {other:#?}"),
        }
    }

    fn state_with_open_pack(write_script: bool) -> (tempfile::TempDir, AppState) {
        state_with_open_pack_kind(PackKind::Custom, write_script)
    }

    fn state_with_unreadable_saved_script() -> (tempfile::TempDir, AppState) {
        let (temp, state) = state_with_open_pack(false);
        let script_path = temp
            .path()
            .join("workspace")
            .join("packs")
            .join("pack-one")
            .join("scripts")
            .join("c99999999.lua");
        std::fs::create_dir(script_path).unwrap();
        (temp, state)
    }

    fn state_with_open_pack_kind(
        pack_kind: PackKind,
        write_script: bool,
    ) -> (tempfile::TempDir, AppState) {
        let temp = tempdir().unwrap();
        let app_dir = temp.path().join("app-data");
        let workspace_path = temp.path().join("workspace");
        let pack_path = workspace_path.join("packs").join("pack-one");
        std::fs::create_dir_all(pack_path.join("scripts")).unwrap();
        if write_script {
            std::fs::write(
                pack_path.join("scripts").join("c99999999.lua"),
                "local s,id,o=GetID()\nfunction s.initial_effect(c)\nend\n",
            )
            .unwrap();
        }

        let state = AppState::new(app_dir).unwrap();
        let now = now_utc();
        let workspace_meta = WorkspaceMeta {
            id: "workspace-1".to_string(),
            name: "Workspace".to_string(),
            description: None,
            created_at: now,
            updated_at: now,
            pack_order: vec!["pack-1".to_string()],
            last_opened_pack_id: Some("pack-1".to_string()),
            open_pack_ids: vec!["pack-1".to_string()],
        };
        let pack_metadata = PackMetadata {
            id: "pack-1".to_string(),
            kind: pack_kind,
            name: "Pack".to_string(),
            pack_code: None,
            author: "author".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            created_at: now,
            updated_at: now,
            display_language_order: vec!["en-US".to_string()],
            default_export_language: Some("en-US".to_string()),
        };
        let card = CardEntity {
            id: "card-1".to_string(),
            code: 99999999,
            alias: 0,
            setcodes: Vec::new(),
            ot: Ot::Custom,
            category: 0,
            primary_type: PrimaryType::Monster,
            texts: BTreeMap::new(),
            monster_flags: None,
            atk: Some(0),
            def: Some(0),
            race: None,
            attribute: None,
            level: Some(4),
            pendulum: None,
            link: None,
            spell_subtype: None,
            trap_subtype: None,
            created_at: now,
            updated_at: now,
        };
        let pack_session = PackSession {
            pack_id: "pack-1".to_string(),
            pack_path: pack_path.clone(),
            revision: 0,
            source_stamp: "test".to_string(),
            metadata: pack_metadata.clone(),
            cards: vec![card],
            strings: PackStringsFile::default(),
            asset_index: BTreeMap::new(),
            card_list_cache: Vec::new(),
        };
        let workspace_session = WorkspaceSession {
            workspace_path,
            meta: workspace_meta,
            pack_paths: BTreeMap::from([("pack-1".to_string(), pack_path.clone())]),
            pack_overviews: BTreeMap::from([(
                "pack-1".to_string(),
                PackOverview {
                    id: "pack-1".to_string(),
                    kind: pack_metadata.kind.clone(),
                    name: "Pack".to_string(),
                    author: "author".to_string(),
                    version: "1.0.0".to_string(),
                    card_count: 1,
                    updated_at: now,
                },
            )]),
            open_pack_ids: vec!["pack-1".to_string()],
            active_pack_id: Some("pack-1".to_string()),
        };
        {
            let mut sessions = state.sessions.write().unwrap();
            sessions.set_workspace(workspace_session);
            sessions.put_pack(pack_session);
        }
        (temp, state)
    }
}
