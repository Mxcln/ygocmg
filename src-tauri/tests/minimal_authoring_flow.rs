use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use tempfile::tempdir;
use ygocmg_core::application::dto::card::{
    CardSortFieldDto, ConfirmCardWriteInput, CreateCardInput, DeleteCardInput, GetCardInput,
    ListCardsInput, SortDirectionDto, SuggestCodeInput, UpdateCardInput,
};
use ygocmg_core::application::dto::common::WriteResultDto;
use ygocmg_core::application::dto::resource::{
    CreateEmptyScriptInput, DeleteFieldImageInput, DeleteMainImageInput, DeleteScriptInput,
    ImportFieldImageInput, ImportMainImageInput, ImportScriptInput, OpenScriptExternalInput,
};
use ygocmg_core::application::dto::strings::{
    ConfirmPackStringsWriteInput, DeletePackStringsInput, ListPackStringsInput, PackStringEntryDto,
    PackStringKeyDto, UpsertPackStringInput,
};
use ygocmg_core::bootstrap::wiring::build_app_state;
use ygocmg_core::domain::card::model::{
    Attribute, CardTexts, CardUpdateInput, MonsterFlag, Ot, PrimaryType, Race, SpellSubtype,
};
use ygocmg_core::domain::config::rules::default_global_config;
use ygocmg_core::domain::language::model::{TextLanguageKind, TextLanguageProfile};
use ygocmg_core::domain::resource::path_rules::{card_image_path, field_image_path, script_path};
use ygocmg_core::domain::strings::model::PackStringKind;
use ygocmg_core::infrastructure::json_store;
use ygocmg_core::infrastructure::pack_locator;
use ygocmg_core::presentation::commands::app_commands;

fn custom_language(id: &str, label: &str) -> TextLanguageProfile {
    TextLanguageProfile {
        id: id.to_string(),
        label: label.to_string(),
        kind: TextLanguageKind::Custom,
        hidden: false,
        last_used_at: None,
    }
}

#[test]
fn minimal_authoring_flow_persists_and_renames_assets() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-a");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    let _config = app_commands::initialize(&state).unwrap();
    let workspace = app_commands::create_workspace(
        &state,
        workspace_path.clone(),
        "Workspace A",
        Some("authoring workspace".to_string()),
    )
    .unwrap();
    assert_eq!(workspace.name, "Workspace A");

    let opened_workspace = app_commands::open_workspace(&state, workspace_path.clone()).unwrap();
    assert_eq!(opened_workspace.id, workspace.id);

    let pack = app_commands::create_pack(
        &state,
        "Pack One",
        "Max",
        "0.1.0",
        Some("test pack".to_string()),
        vec!["zh-CN".to_string(), "en-US".to_string()],
        Some("zh-CN".to_string()),
    )
    .unwrap();
    let pack = app_commands::open_pack(&state, &pack.id).unwrap();
    let workspace_id = workspace.id.clone();
    let pack_path = pack_locator::resolve_pack_path(
        &pack_locator::load_workspace_pack_inventory(&workspace_path).unwrap(),
        &pack.id,
    )
    .unwrap();

    let mut texts = BTreeMap::new();
    texts.insert(
        "zh-CN".to_string(),
        CardTexts {
            name: "测试怪兽".to_string(),
            desc: "一张用于测试的卡片".to_string(),
            strings: vec![],
        },
    );

    let card = match app_commands::create_card(
        &state,
        CreateCardInput {
            workspace_id: workspace_id.clone(),
            pack_id: pack.id.clone(),
            card: CardUpdateInput {
                code: 201_000_000,
                alias: 0,
                setcodes: vec![],
                ot: Ot::Custom,
                category: 0,
                primary_type: PrimaryType::Monster,
                texts,
                monster_flags: Some(vec![MonsterFlag::Effect]),
                atk: Some(1500),
                def: Some(1200),
                race: Some(Race::Warrior),
                attribute: Some(Attribute::Light),
                level: Some(4),
                pendulum: None,
                link: None,
                spell_subtype: None,
                trap_subtype: None,
            },
        },
    )
    .unwrap()
    {
        WriteResultDto::NeedsConfirmation {
            confirmation_token,
            warnings,
            ..
        } => {
            assert!(
                !warnings.is_empty(),
                "code outside recommended range should produce warnings"
            );
            app_commands::confirm_card_write(&state, ConfirmCardWriteInput { confirmation_token })
                .unwrap()
                .card
        }
        WriteResultDto::Ok { .. } => panic!("expected confirmation result"),
    };

    let original_script = script_path(&pack_path, card.code);
    fs::write(&original_script, "-- test script").unwrap();

    let mut texts = BTreeMap::new();
    texts.insert(
        "zh-CN".to_string(),
        CardTexts {
            name: "测试怪兽改".to_string(),
            desc: "改号后的测试卡片".to_string(),
            strings: vec![],
        },
    );

    let updated = match app_commands::update_card(
        &state,
        UpdateCardInput {
            workspace_id: workspace_id.clone(),
            pack_id: pack.id.clone(),
            card_id: card.id.clone(),
            card: CardUpdateInput {
                code: 201_000_010,
                alias: 0,
                setcodes: vec![],
                ot: Ot::Custom,
                category: 0,
                primary_type: PrimaryType::Monster,
                texts,
                monster_flags: Some(vec![MonsterFlag::Effect]),
                atk: Some(1600),
                def: Some(1200),
                race: Some(Race::Warrior),
                attribute: Some(Attribute::Light),
                level: Some(4),
                pendulum: None,
                link: None,
                spell_subtype: None,
                trap_subtype: None,
            },
        },
    )
    .unwrap()
    {
        WriteResultDto::NeedsConfirmation {
            confirmation_token,
            warnings,
            ..
        } => {
            assert!(
                !warnings.is_empty(),
                "code outside recommended range should produce warnings"
            );
            app_commands::confirm_card_write(&state, ConfirmCardWriteInput { confirmation_token })
                .unwrap()
                .card
        }
        WriteResultDto::Ok { .. } => panic!("expected confirmation result"),
    };

    assert_eq!(updated.code, 201_000_010);
    assert!(!original_script.exists());
    assert!(script_path(&pack_path, updated.code).exists());

    let reopened_state = build_app_state(app_dir.path().to_path_buf()).unwrap();
    let reopened_workspace =
        app_commands::open_workspace(&reopened_state, workspace_path.clone()).unwrap();
    app_commands::open_pack(&reopened_state, &pack.id).unwrap();
    let sessions = reopened_state.sessions.read().unwrap();
    let reopened_pack = sessions.open_packs.get(&pack.id).unwrap();
    assert_eq!(reopened_pack.revision, 0);
    assert!(!reopened_pack.source_stamp.is_empty());

    let rows = app_commands::list_cards(
        &reopened_state,
        ListCardsInput {
            workspace_id: reopened_workspace.id.clone(),
            pack_id: pack.id.clone(),
            keyword: None,
            sort_by: CardSortFieldDto::Code,
            sort_direction: SortDirectionDto::Asc,
            page: 1,
            page_size: 50,
        },
    )
    .unwrap();
    assert_eq!(rows.items.len(), 1);
    assert_eq!(rows.total, 1);
    assert_eq!(rows.items[0].code, 201_000_010);
    assert!(rows.items[0].has_script);

    let detail = app_commands::get_card(
        &reopened_state,
        GetCardInput {
            workspace_id: reopened_workspace.id.clone(),
            pack_id: pack.id.clone(),
            card_id: updated.id.clone(),
        },
    )
    .unwrap();
    assert_eq!(detail.card.code, 201_000_010);
    assert!(detail.asset_state.has_script);

    let suggestion = app_commands::suggest_card_code(
        &reopened_state,
        SuggestCodeInput {
            workspace_id: reopened_workspace.id,
            pack_id: pack.id.clone(),
            preferred_start: Some(90_000_000),
        },
    )
    .unwrap();
    assert!(suggestion.suggested_code.is_some());
}

#[test]
fn workspace_id_mismatch_rejected_for_card_commands() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-mismatch");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    let _config = app_commands::initialize(&state).unwrap();
    let workspace = app_commands::create_workspace(
        &state,
        workspace_path.clone(),
        "Workspace A",
        Some("authoring workspace".to_string()),
    )
    .unwrap();
    app_commands::open_workspace(&state, workspace_path.clone()).unwrap();

    let pack = app_commands::create_pack(
        &state,
        "Pack One",
        "Max",
        "0.1.0",
        Some("test pack".to_string()),
        vec!["zh-CN".to_string(), "en-US".to_string()],
        Some("zh-CN".to_string()),
    )
    .unwrap();
    app_commands::open_pack(&state, &pack.id).unwrap();

    let mut texts = BTreeMap::new();
    texts.insert(
        "zh-CN".to_string(),
        CardTexts {
            name: "测试怪兽".to_string(),
            desc: "一张用于测试的卡片".to_string(),
            strings: vec![],
        },
    );

    let error = app_commands::create_card(
        &state,
        CreateCardInput {
            workspace_id: format!("{}-wrong", workspace.id),
            pack_id: pack.id,
            card: CardUpdateInput {
                code: 100_000_100,
                alias: 0,
                setcodes: vec![],
                ot: Ot::Custom,
                category: 0,
                primary_type: PrimaryType::Monster,
                texts,
                monster_flags: Some(vec![MonsterFlag::Effect]),
                atk: Some(1500),
                def: Some(1200),
                race: Some(Race::Warrior),
                attribute: Some(Attribute::Light),
                level: Some(4),
                pendulum: None,
                link: None,
                spell_subtype: None,
                trap_subtype: None,
            },
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "workspace.mismatch");
}

#[test]
fn confirmation_token_can_only_be_used_once_and_expires_on_revision_change() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-confirmation");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    let _config = app_commands::initialize(&state).unwrap();
    let workspace =
        app_commands::create_workspace(&state, workspace_path.clone(), "Workspace A", None)
            .unwrap();
    app_commands::open_workspace(&state, workspace_path.clone()).unwrap();
    let pack = app_commands::create_pack(
        &state,
        "Pack One",
        "Max",
        "0.1.0",
        None,
        vec!["zh-CN".to_string()],
        Some("zh-CN".to_string()),
    )
    .unwrap();
    let pack = app_commands::open_pack(&state, &pack.id).unwrap();

    let mut texts = BTreeMap::new();
    texts.insert(
        "zh-CN".to_string(),
        CardTexts {
            name: "测试怪兽".to_string(),
            desc: "需要确认".to_string(),
            strings: vec![],
        },
    );

    let confirmation_token = match app_commands::create_card(
        &state,
        CreateCardInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card: CardUpdateInput {
                code: 201_000_000,
                alias: 0,
                setcodes: vec![],
                ot: Ot::Custom,
                category: 0,
                primary_type: PrimaryType::Monster,
                texts: texts.clone(),
                monster_flags: Some(vec![MonsterFlag::Effect]),
                atk: Some(1500),
                def: Some(1200),
                race: Some(Race::Warrior),
                attribute: Some(Attribute::Light),
                level: Some(4),
                pendulum: None,
                link: None,
                spell_subtype: None,
                trap_subtype: None,
            },
        },
    )
    .unwrap()
    {
        WriteResultDto::NeedsConfirmation {
            confirmation_token, ..
        } => confirmation_token,
        WriteResultDto::Ok { .. } => panic!("expected confirmation result"),
    };

    let created = app_commands::confirm_card_write(
        &state,
        ConfirmCardWriteInput {
            confirmation_token: confirmation_token.clone(),
        },
    )
    .unwrap();
    assert_eq!(created.card.code, 201_000_000);

    let consumed_error = app_commands::confirm_card_write(
        &state,
        ConfirmCardWriteInput {
            confirmation_token: confirmation_token.clone(),
        },
    )
    .unwrap_err();
    assert_eq!(consumed_error.code, "confirmation.invalid_token");

    let stale_token = match app_commands::create_card(
        &state,
        CreateCardInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card: CardUpdateInput {
                code: 201_000_020,
                alias: 0,
                setcodes: vec![],
                ot: Ot::Custom,
                category: 0,
                primary_type: PrimaryType::Monster,
                texts,
                monster_flags: Some(vec![MonsterFlag::Effect]),
                atk: Some(1700),
                def: Some(1200),
                race: Some(Race::Warrior),
                attribute: Some(Attribute::Light),
                level: Some(4),
                pendulum: None,
                link: None,
                spell_subtype: None,
                trap_subtype: None,
            },
        },
    )
    .unwrap()
    {
        WriteResultDto::NeedsConfirmation {
            confirmation_token, ..
        } => confirmation_token,
        WriteResultDto::Ok { .. } => panic!("expected confirmation result"),
    };

    let mut clean_texts = BTreeMap::new();
    clean_texts.insert(
        "zh-CN".to_string(),
        CardTexts {
            name: "正常卡".to_string(),
            desc: "无 warning".to_string(),
            strings: vec![],
        },
    );
    let create_ok = app_commands::create_card(
        &state,
        CreateCardInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card: CardUpdateInput {
                code: 100_000_200,
                alias: 0,
                setcodes: vec![],
                ot: Ot::Custom,
                category: 0,
                primary_type: PrimaryType::Monster,
                texts: clean_texts,
                monster_flags: Some(vec![MonsterFlag::Effect]),
                atk: Some(1800),
                def: Some(1000),
                race: Some(Race::Warrior),
                attribute: Some(Attribute::Light),
                level: Some(4),
                pendulum: None,
                link: None,
                spell_subtype: None,
                trap_subtype: None,
            },
        },
    )
    .unwrap();
    match create_ok {
        WriteResultDto::Ok { .. } => {}
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    }

    let stale_error = app_commands::confirm_card_write(
        &state,
        ConfirmCardWriteInput {
            confirmation_token: stale_token,
        },
    )
    .unwrap_err();
    assert_eq!(stale_error.code, "confirmation.invalid_token");
}

#[test]
fn create_confirmation_reuses_staged_card_identity() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let workspace_path = workspace_root
        .path()
        .join("workspace-confirmation-identity");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    let _config = app_commands::initialize(&state).unwrap();
    let workspace =
        app_commands::create_workspace(&state, workspace_path.clone(), "Workspace A", None)
            .unwrap();
    app_commands::open_workspace(&state, workspace_path.clone()).unwrap();
    let pack = app_commands::create_pack(
        &state,
        "Pack One",
        "Max",
        "0.1.0",
        None,
        vec!["zh-CN".to_string()],
        Some("zh-CN".to_string()),
    )
    .unwrap();
    let pack = app_commands::open_pack(&state, &pack.id).unwrap();

    let mut texts = BTreeMap::new();
    texts.insert(
        "zh-CN".to_string(),
        CardTexts {
            name: "确认身份复用".to_string(),
            desc: "warning create should keep staged identity".to_string(),
            strings: vec![],
        },
    );

    let confirmation_token = match app_commands::create_card(
        &state,
        CreateCardInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card: CardUpdateInput {
                code: 201_000_100,
                alias: 0,
                setcodes: vec![],
                ot: Ot::Custom,
                category: 0,
                primary_type: PrimaryType::Monster,
                texts,
                monster_flags: Some(vec![MonsterFlag::Effect]),
                atk: Some(1500),
                def: Some(1200),
                race: Some(Race::Warrior),
                attribute: Some(Attribute::Light),
                level: Some(4),
                pendulum: None,
                link: None,
                spell_subtype: None,
                trap_subtype: None,
            },
        },
    )
    .unwrap()
    {
        WriteResultDto::NeedsConfirmation {
            confirmation_token, ..
        } => confirmation_token,
        WriteResultDto::Ok { .. } => panic!("expected confirmation result"),
    };

    let staged_entry = state
        .confirmation_cache
        .read()
        .unwrap()
        .debug_get_card_entry(&confirmation_token)
        .cloned()
        .expect("confirmation entry should exist");
    let staged_card = staged_entry
        .input_snapshot
        .create_card_seed
        .expect("staged create entry should carry card seed");

    let confirmed =
        app_commands::confirm_card_write(&state, ConfirmCardWriteInput { confirmation_token })
            .unwrap();

    assert_eq!(confirmed.card.id, staged_card.id);
    assert_eq!(confirmed.card.created_at, staged_card.created_at);
}

#[test]
fn delete_card_returns_write_result_and_rejects_workspace_mismatch() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-delete-card");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    let _config = app_commands::initialize(&state).unwrap();
    let workspace =
        app_commands::create_workspace(&state, workspace_path.clone(), "Workspace A", None)
            .unwrap();
    app_commands::open_workspace(&state, workspace_path.clone()).unwrap();
    let pack = app_commands::create_pack(
        &state,
        "Pack One",
        "Max",
        "0.1.0",
        None,
        vec!["zh-CN".to_string()],
        Some("zh-CN".to_string()),
    )
    .unwrap();
    let pack = app_commands::open_pack(&state, &pack.id).unwrap();

    let mut texts = BTreeMap::new();
    texts.insert(
        "zh-CN".to_string(),
        CardTexts {
            name: "正常卡".to_string(),
            desc: "删除测试".to_string(),
            strings: vec![],
        },
    );

    let created = match app_commands::create_card(
        &state,
        CreateCardInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card: CardUpdateInput {
                code: 100_000_300,
                alias: 0,
                setcodes: vec![],
                ot: Ot::Custom,
                category: 0,
                primary_type: PrimaryType::Monster,
                texts,
                monster_flags: Some(vec![MonsterFlag::Effect]),
                atk: Some(1900),
                def: Some(1000),
                race: Some(Race::Warrior),
                attribute: Some(Attribute::Light),
                level: Some(4),
                pendulum: None,
                link: None,
                spell_subtype: None,
                trap_subtype: None,
            },
        },
    )
    .unwrap()
    {
        WriteResultDto::Ok { data, .. } => data.card,
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    };

    let delete_result = app_commands::delete_card(
        &state,
        DeleteCardInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card_id: created.id.clone(),
        },
    )
    .unwrap();
    match delete_result {
        WriteResultDto::Ok { data, warnings } => {
            assert!(warnings.is_empty());
            assert_eq!(data.deleted_card_id, created.id);
        }
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    }

    let rows = app_commands::list_cards(
        &state,
        ListCardsInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            keyword: None,
            sort_by: CardSortFieldDto::Code,
            sort_direction: SortDirectionDto::Asc,
            page: 1,
            page_size: 50,
        },
    )
    .unwrap();
    assert_eq!(rows.total, 0);

    let mismatch_error = app_commands::delete_card(
        &state,
        DeleteCardInput {
            workspace_id: format!("{}-wrong", workspace.id),
            pack_id: pack.id,
            card_id: created.id,
        },
    )
    .unwrap_err();
    assert_eq!(mismatch_error.code, "workspace.mismatch");
}

#[test]
fn create_pack_uses_readable_storage_name_and_handles_collisions() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-readable-pack");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    let _config = app_commands::initialize(&state).unwrap();
    app_commands::create_workspace(&state, workspace_path.clone(), "Workspace A", None).unwrap();
    app_commands::open_workspace(&state, workspace_path.clone()).unwrap();

    let pack_a = app_commands::create_pack(
        &state,
        "龙族卡组",
        "Max",
        "1.0.0",
        None,
        vec!["zh-CN".to_string()],
        None,
    )
    .unwrap();
    let pack_b = app_commands::create_pack(
        &state,
        "龙族卡组",
        "Max",
        "1.0.1",
        None,
        vec!["zh-CN".to_string()],
        None,
    )
    .unwrap();

    let inventory = pack_locator::load_workspace_pack_inventory(&workspace_path).unwrap();
    let path_a = pack_locator::resolve_pack_path(&inventory, &pack_a.id).unwrap();
    let path_b = pack_locator::resolve_pack_path(&inventory, &pack_b.id).unwrap();

    let dir_a = path_a.file_name().unwrap().to_string_lossy().to_string();
    let dir_b = path_b.file_name().unwrap().to_string_lossy().to_string();

    assert!(dir_a.starts_with("龙族卡组--"));
    assert!(dir_b.starts_with("龙族卡组--"));
    assert_ne!(dir_a, dir_b);
}

#[test]
fn invalid_pack_directory_without_metadata_fails_workspace_open() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-invalid-pack");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    let _config = app_commands::initialize(&state).unwrap();
    app_commands::create_workspace(&state, workspace_path.clone(), "Workspace A", None).unwrap();

    let broken_dir = workspace_path.join("packs").join("broken-pack--deadbeef");
    fs::create_dir_all(&broken_dir).unwrap();

    let error = app_commands::open_workspace(&state, workspace_path.clone()).unwrap_err();
    assert_eq!(error.code, "pack.metadata_missing");
}

#[test]
fn duplicate_pack_ids_fail_workspace_open() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-duplicate-pack");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    let _config = app_commands::initialize(&state).unwrap();
    app_commands::create_workspace(&state, workspace_path.clone(), "Workspace A", None).unwrap();
    app_commands::open_workspace(&state, workspace_path.clone()).unwrap();

    let pack = app_commands::create_pack(
        &state,
        "Pack One",
        "Max",
        "1.0.0",
        None,
        vec!["en-US".to_string()],
        None,
    )
    .unwrap();
    let original_pack_path = pack_locator::resolve_pack_path(
        &pack_locator::load_workspace_pack_inventory(&workspace_path).unwrap(),
        &pack.id,
    )
    .unwrap();
    let metadata = json_store::load_pack_metadata(&original_pack_path).unwrap();

    let duplicate_dir = workspace_path
        .join("packs")
        .join("duplicate-pack--cafebabe");
    json_store::ensure_pack_layout(&duplicate_dir).unwrap();
    json_store::save_pack_metadata(&duplicate_dir, &metadata).unwrap();
    json_store::save_cards(&duplicate_dir, &[]).unwrap();
    json_store::save_pack_strings(&duplicate_dir, &Default::default()).unwrap();

    let reopened_state = build_app_state(app_dir.path().to_path_buf()).unwrap();
    let error = app_commands::open_workspace(&reopened_state, workspace_path.clone()).unwrap_err();
    assert_eq!(error.code, "pack.duplicate_id");
}

#[test]
fn removing_workspace_record_keeps_workspace_directory() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-record-only");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    let _config = app_commands::initialize(&state).unwrap();
    let workspace = app_commands::create_workspace(
        &state,
        workspace_path.clone(),
        "Workspace Record Only",
        None,
    )
    .unwrap();

    let recent_before = app_commands::list_recent_workspaces(&state).unwrap();
    assert_eq!(recent_before.workspaces.len(), 1);

    app_commands::delete_workspace(&state, &workspace.id, workspace_path.clone(), false).unwrap();

    assert!(workspace_path.exists());
    let recent_after = app_commands::list_recent_workspaces(&state).unwrap();
    assert!(recent_after.workspaces.is_empty());
}

#[test]
fn deleting_workspace_directory_removes_directory_and_clears_session() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-delete-dir");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    let _config = app_commands::initialize(&state).unwrap();
    let workspace = app_commands::create_workspace(
        &state,
        workspace_path.clone(),
        "Workspace Delete Dir",
        None,
    )
    .unwrap();

    app_commands::open_workspace(&state, workspace_path.clone()).unwrap();
    app_commands::delete_workspace(&state, &workspace.id, workspace_path.clone(), true).unwrap();

    assert!(!workspace_path.exists());
    let recent_after = app_commands::list_recent_workspaces(&state).unwrap();
    assert!(recent_after.workspaces.is_empty());

    let sessions = state.sessions.read().unwrap();
    assert!(sessions.current_workspace.is_none());
}

#[test]
fn pack_strings_support_list_upsert_confirm_delete_and_filtering() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-pack-strings");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    let _config = app_commands::initialize(&state).unwrap();
    let workspace =
        app_commands::create_workspace(&state, workspace_path.clone(), "Workspace A", None)
            .unwrap();
    app_commands::open_workspace(&state, workspace_path.clone()).unwrap();
    let pack = app_commands::create_pack(
        &state,
        "Pack Strings",
        "Max",
        "1.0.0",
        None,
        vec!["zh-CN".to_string(), "en-US".to_string()],
        Some("zh-CN".to_string()),
    )
    .unwrap();
    let pack = app_commands::open_pack(&state, &pack.id).unwrap();

    let empty = app_commands::list_pack_strings(
        &state,
        ListPackStringsInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            language: "zh-CN".to_string(),
            kind_filter: None,
            key_filter: None,
            keyword: None,
            page: 1,
            page_size: 20,
        },
    )
    .unwrap();
    assert_eq!(empty.total, 0);

    let before_updated_at = json_store::load_pack_metadata(
        &pack_locator::resolve_pack_path(
            &pack_locator::load_workspace_pack_inventory(&workspace_path).unwrap(),
            &pack.id,
        )
        .unwrap(),
    )
    .unwrap()
    .updated_at;

    let inserted = app_commands::upsert_pack_string(
        &state,
        UpsertPackStringInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            language: "zh-CN".to_string(),
            entry: PackStringEntryDto {
                kind: PackStringKind::Setname,
                key: 0x345,
                value: "Alpha".to_string(),
            },
        },
    )
    .unwrap();
    match inserted {
        WriteResultDto::Ok { data, warnings } => {
            assert!(warnings.is_empty());
            assert_eq!(data.total, 1);
            assert_eq!(data.items[0].value, "Alpha");
        }
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    }

    let after_updated_at = json_store::load_pack_metadata(
        &pack_locator::resolve_pack_path(
            &pack_locator::load_workspace_pack_inventory(&workspace_path).unwrap(),
            &pack.id,
        )
        .unwrap(),
    )
    .unwrap()
    .updated_at;
    assert!(after_updated_at >= before_updated_at);

    for (kind, key, value) in [
        (PackStringKind::Setname, 0x346, "Beta"),
        (PackStringKind::Counter, 0x222, "Gamma"),
        (PackStringKind::Victory, 0x155, "Delta"),
    ] {
        let result = app_commands::upsert_pack_string(
            &state,
            UpsertPackStringInput {
                workspace_id: workspace.id.clone(),
                pack_id: pack.id.clone(),
                language: "zh-CN".to_string(),
                entry: PackStringEntryDto {
                    kind,
                    key,
                    value: value.to_string(),
                },
            },
        )
        .unwrap();
        match result {
            WriteResultDto::Ok { .. } => {}
            WriteResultDto::NeedsConfirmation {
                confirmation_token, ..
            } => {
                app_commands::confirm_pack_strings_write(
                    &state,
                    ConfirmPackStringsWriteInput { confirmation_token },
                )
                .unwrap();
            }
        }
    }

    let filtered = app_commands::list_pack_strings(
        &state,
        ListPackStringsInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            language: "zh-CN".to_string(),
            kind_filter: Some(PackStringKind::Setname),
            key_filter: None,
            keyword: Some("a".to_string()),
            page: 1,
            page_size: 10,
        },
    )
    .unwrap();
    assert_eq!(filtered.total, 2);
    assert_eq!(filtered.items[0].key, 0x345);
    assert_eq!(filtered.items[1].key, 0x346);

    let key_filtered = app_commands::list_pack_strings(
        &state,
        ListPackStringsInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            language: "zh-CN".to_string(),
            kind_filter: None,
            key_filter: Some(0x222),
            keyword: None,
            page: 1,
            page_size: 10,
        },
    )
    .unwrap();
    assert_eq!(key_filtered.total, 1);
    assert_eq!(key_filtered.items[0].kind, PackStringKind::Counter);

    let paged = app_commands::list_pack_strings(
        &state,
        ListPackStringsInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            language: "zh-CN".to_string(),
            kind_filter: None,
            key_filter: None,
            keyword: None,
            page: 2,
            page_size: 2,
        },
    )
    .unwrap();
    assert_eq!(paged.total, 4);
    assert_eq!(paged.items.len(), 2);

    let untranslated_language = app_commands::list_pack_strings(
        &state,
        ListPackStringsInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            language: "en-US".to_string(),
            kind_filter: None,
            key_filter: None,
            keyword: None,
            page: 1,
            page_size: 10,
        },
    )
    .unwrap();
    assert_eq!(untranslated_language.total, 4);
    assert!(
        untranslated_language
            .items
            .iter()
            .all(|entry| entry.value.is_empty())
    );

    let confirmation_token = match app_commands::upsert_pack_string(
        &state,
        UpsertPackStringInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            language: "zh-CN".to_string(),
            entry: PackStringEntryDto {
                kind: PackStringKind::Setname,
                key: 0x345,
                value: "Alpha Updated".to_string(),
            },
        },
    )
    .unwrap()
    {
        WriteResultDto::NeedsConfirmation {
            confirmation_token,
            warnings,
            ..
        } => {
            assert_eq!(warnings.len(), 1);
            confirmation_token
        }
        WriteResultDto::Ok { .. } => panic!("expected confirmation result"),
    };

    let confirmed = app_commands::confirm_pack_strings_write(
        &state,
        ConfirmPackStringsWriteInput { confirmation_token },
    )
    .unwrap();
    assert_eq!(confirmed.total, 4);
    let updated_entry = confirmed
        .items
        .iter()
        .find(|entry| entry.kind == PackStringKind::Setname && entry.key == 0x345)
        .expect("updated setname string should be present");
    assert_eq!(updated_entry.value, "Alpha Updated");

    let stale_token = match app_commands::upsert_pack_string(
        &state,
        UpsertPackStringInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            language: "zh-CN".to_string(),
            entry: PackStringEntryDto {
                kind: PackStringKind::Setname,
                key: 0x346,
                value: "Beta Updated".to_string(),
            },
        },
    )
    .unwrap()
    {
        WriteResultDto::NeedsConfirmation {
            confirmation_token, ..
        } => confirmation_token,
        WriteResultDto::Ok { .. } => panic!("expected confirmation result"),
    };

    let intervening_delete = app_commands::delete_pack_strings(
        &state,
        DeletePackStringsInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            entries: vec![PackStringKeyDto {
                kind: PackStringKind::Counter,
                key: 0x222,
            }],
        },
    )
    .unwrap();
    match intervening_delete {
        WriteResultDto::Ok { data, warnings } => {
            assert_eq!(data.deleted_count, 1);
            assert!(warnings.is_empty());
        }
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    }

    let stale_error = app_commands::confirm_pack_strings_write(
        &state,
        ConfirmPackStringsWriteInput {
            confirmation_token: stale_token,
        },
    )
    .unwrap_err();
    assert_eq!(stale_error.code, "confirmation.invalid_token");

    let noop_delete = app_commands::delete_pack_strings(
        &state,
        DeletePackStringsInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            entries: vec![PackStringKeyDto {
                kind: PackStringKind::Setname,
                key: 9999,
            }],
        },
    )
    .unwrap();
    match noop_delete {
        WriteResultDto::Ok { data, warnings } => {
            assert_eq!(data.deleted_count, 0);
            assert!(warnings.is_empty());
        }
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    }

    let deleted = app_commands::delete_pack_strings(
        &state,
        DeletePackStringsInput {
            workspace_id: workspace.id,
            pack_id: pack.id,
            entries: vec![PackStringKeyDto {
                kind: PackStringKind::Victory,
                key: 0x155,
            }],
        },
    )
    .unwrap();
    match deleted {
        WriteResultDto::Ok { data, warnings } => {
            assert_eq!(data.deleted_count, 1);
            assert!(warnings.is_empty());
        }
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    }
}

#[test]
fn resource_management_supports_images_scripts_and_external_editor_validation()
-> Result<(), Box<dyn std::error::Error>> {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-resources");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    let _config = app_commands::initialize(&state).unwrap();
    let workspace =
        app_commands::create_workspace(&state, workspace_path.clone(), "Workspace A", None)
            .unwrap();
    app_commands::open_workspace(&state, workspace_path.clone()).unwrap();
    let pack = app_commands::create_pack(
        &state,
        "Pack Resources",
        "Max",
        "1.0.0",
        None,
        vec!["zh-CN".to_string()],
        Some("zh-CN".to_string()),
    )
    .unwrap();
    let pack = app_commands::open_pack(&state, &pack.id).unwrap();
    let pack_path = pack_locator::resolve_pack_path(
        &pack_locator::load_workspace_pack_inventory(&workspace_path).unwrap(),
        &pack.id,
    )
    .unwrap();

    let monster = create_card_direct(
        &state,
        &workspace.id,
        &pack.id,
        100_000_100,
        PrimaryType::Monster,
        None,
        "Monster",
    );
    let field_spell = create_card_direct(
        &state,
        &workspace.id,
        &pack.id,
        100_000_110,
        PrimaryType::Spell,
        Some(SpellSubtype::Field),
        "Field Spell",
    );

    let source_dir = tempdir().unwrap();
    let main_png = source_dir.path().join("main.png");
    create_png(&main_png, 123, 200)?;
    let field_png = source_dir.path().join("field.png");
    create_png(&field_png, 321, 100)?;
    let script_file = source_dir.path().join("script.lua");
    fs::write(&script_file, "-- imported script").unwrap();

    let main_result = app_commands::import_main_image(
        &state,
        ImportMainImageInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card_id: monster.id.clone(),
            source_path: main_png.clone(),
        },
    )
    .unwrap();
    match main_result {
        WriteResultDto::Ok { data, warnings } => {
            assert!(data.has_image);
            assert!(warnings.is_empty());
        }
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    }
    let saved_main = card_image_path(&pack_path, 100_000_100);
    assert!(saved_main.exists());
    let main_image = image::ImageReader::open(&saved_main)
        .unwrap()
        .decode()
        .unwrap();
    assert_eq!(main_image.width(), 400);
    assert_eq!(main_image.height(), 580);

    let not_field_error = app_commands::import_field_image(
        &state,
        ImportFieldImageInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card_id: monster.id.clone(),
            source_path: field_png.clone(),
        },
    )
    .unwrap_err();
    assert_eq!(
        not_field_error.code,
        "resource.field_image_requires_field_spell"
    );

    let field_result = app_commands::import_field_image(
        &state,
        ImportFieldImageInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card_id: field_spell.id.clone(),
            source_path: field_png.clone(),
        },
    )
    .unwrap();
    match field_result {
        WriteResultDto::Ok { data, .. } => assert!(data.has_field_image),
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    }
    let saved_field = field_image_path(&pack_path, 100_000_110);
    assert!(saved_field.exists());
    let field_image = image::ImageReader::open(&saved_field)
        .unwrap()
        .decode()
        .unwrap();
    assert_eq!(field_image.width(), 321);
    assert_eq!(field_image.height(), 100);

    let empty_script = app_commands::create_empty_script(
        &state,
        CreateEmptyScriptInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card_id: monster.id.clone(),
        },
    )
    .unwrap();
    match empty_script {
        WriteResultDto::Ok { data, .. } => assert!(data.has_script),
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    }

    let duplicate_script_error = app_commands::create_empty_script(
        &state,
        CreateEmptyScriptInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card_id: monster.id.clone(),
        },
    )
    .unwrap_err();
    assert_eq!(duplicate_script_error.code, "resource.script_exists");

    let imported_script = app_commands::import_script(
        &state,
        ImportScriptInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card_id: monster.id.clone(),
            source_path: script_file.clone(),
        },
    )
    .unwrap();
    match imported_script {
        WriteResultDto::Ok { data, .. } => assert!(data.has_script),
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    }
    let script_path_on_disk = script_path(&pack_path, 100_000_100);
    assert_eq!(
        fs::read_to_string(&script_path_on_disk).unwrap(),
        "-- imported script"
    );

    let no_editor_error = app_commands::open_script_external(
        &state,
        OpenScriptExternalInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card_id: monster.id.clone(),
        },
    )
    .unwrap_err();
    assert_eq!(
        no_editor_error.code,
        "resource.external_editor_not_configured"
    );

    let mut config = default_global_config();
    config.external_text_editor_path = Some(source_dir.path().join("missing-editor.exe"));
    app_commands::save_config(&state, &config).unwrap();
    let missing_editor_error = app_commands::open_script_external(
        &state,
        OpenScriptExternalInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card_id: monster.id.clone(),
        },
    )
    .unwrap_err();
    assert_eq!(
        missing_editor_error.code,
        "resource.external_editor_missing"
    );

    let delete_script = app_commands::delete_script(
        &state,
        DeleteScriptInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card_id: monster.id.clone(),
        },
    )
    .unwrap();
    match delete_script {
        WriteResultDto::Ok { data, .. } => assert!(!data.has_script),
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    }

    let mut config = default_global_config();
    config.external_text_editor_path = Some(std::env::current_exe()?);
    app_commands::save_config(&state, &config).unwrap();
    let missing_script_error = app_commands::open_script_external(
        &state,
        OpenScriptExternalInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card_id: monster.id.clone(),
        },
    )
    .unwrap_err();
    assert_eq!(missing_script_error.code, "resource.script_missing");

    let delete_main = app_commands::delete_main_image(
        &state,
        DeleteMainImageInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card_id: monster.id.clone(),
        },
    )
    .unwrap();
    match delete_main {
        WriteResultDto::Ok { data, .. } => assert!(!data.has_image),
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    }

    let delete_field = app_commands::delete_field_image(
        &state,
        DeleteFieldImageInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card_id: field_spell.id.clone(),
        },
    )
    .unwrap();
    match delete_field {
        WriteResultDto::Ok { data, .. } => assert!(!data.has_field_image),
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    }

    let imported_again_main = app_commands::import_main_image(
        &state,
        ImportMainImageInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card_id: monster.id.clone(),
            source_path: main_png,
        },
    )
    .unwrap();
    match imported_again_main {
        WriteResultDto::Ok { data, .. } => assert!(data.has_image),
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    }
    app_commands::import_field_image(
        &state,
        ImportFieldImageInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card_id: field_spell.id.clone(),
            source_path: field_png,
        },
    )
    .unwrap();
    app_commands::import_script(
        &state,
        ImportScriptInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card_id: monster.id.clone(),
            source_path: script_file,
        },
    )
    .unwrap();

    let updated = update_card_direct(
        &state,
        &workspace.id,
        &pack.id,
        &monster.id,
        100_000_120,
        PrimaryType::Monster,
        None,
        "Monster Updated",
    );
    assert_eq!(updated.code, 100_000_120);
    assert!(!card_image_path(&pack_path, 100_000_100).exists());
    assert!(!script_path(&pack_path, 100_000_100).exists());
    assert!(card_image_path(&pack_path, 100_000_120).exists());
    assert!(script_path(&pack_path, 100_000_120).exists());

    let updated_field = update_card_direct(
        &state,
        &workspace.id,
        &pack.id,
        &field_spell.id,
        100_000_130,
        PrimaryType::Spell,
        Some(SpellSubtype::Field),
        "Field Updated",
    );
    assert_eq!(updated_field.code, 100_000_130);
    assert!(!field_image_path(&pack_path, 100_000_110).exists());
    assert!(field_image_path(&pack_path, 100_000_130).exists());
    Ok(())
}

#[test]
fn config_injects_and_validates_text_language_catalog() {
    let app_dir = tempdir().unwrap();
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    let initialized = app_commands::initialize(&state).unwrap();
    assert!(
        initialized
            .text_language_catalog
            .iter()
            .any(|language| language.id == "zh-CN")
    );
    assert!(initialized.standard_pack_source_language.is_none());

    let mut duplicate = initialized.clone();
    duplicate
        .text_language_catalog
        .push(custom_language("zh-CN", "Duplicate"));
    assert_eq!(
        app_commands::save_config(&state, &duplicate)
            .unwrap_err()
            .code,
        "config.validation_failed"
    );

    let mut invalid_id = initialized.clone();
    invalid_id
        .text_language_catalog
        .push(custom_language("default", "Default"));
    assert_eq!(
        app_commands::save_config(&state, &invalid_id)
            .unwrap_err()
            .code,
        "config.validation_failed"
    );

    let mut missing_label = initialized.clone();
    missing_label
        .text_language_catalog
        .push(custom_language("x-test", ""));
    assert_eq!(
        app_commands::save_config(&state, &missing_label)
            .unwrap_err()
            .code,
        "config.validation_failed"
    );

    let mut accepted = initialized;
    accepted
        .text_language_catalog
        .push(custom_language("x-test", "Test Language"));
    accepted.standard_pack_source_language = Some("x-test".to_string());
    let saved = app_commands::save_config(&state, &accepted).unwrap();
    assert!(
        saved
            .text_language_catalog
            .iter()
            .any(|language| language.id == "x-test")
    );
    assert_eq!(
        saved.standard_pack_source_language.as_deref(),
        Some("x-test")
    );
}

#[test]
fn language_catalog_is_enforced_on_pack_and_card_writes_but_preserves_legacy_unknowns() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-language");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    app_commands::initialize(&state).unwrap();
    let workspace =
        app_commands::create_workspace(&state, workspace_path.clone(), "ws", None).unwrap();
    app_commands::open_workspace(&state, workspace_path.clone()).unwrap();

    let bad_pack = app_commands::create_pack(
        &state,
        "bad",
        "me",
        "1",
        None,
        vec!["fr-FR".to_string()],
        None,
    )
    .unwrap_err();
    assert_eq!(bad_pack.code, "pack.validation_failed");

    let default_pack = app_commands::create_pack(
        &state,
        "default",
        "me",
        "1",
        None,
        vec!["default".to_string()],
        None,
    )
    .unwrap_err();
    assert_eq!(default_pack.code, "pack.validation_failed");

    let mut config = app_commands::load_config(&state).unwrap();
    config
        .text_language_catalog
        .push(custom_language("x-fan", "Fan Translation"));
    app_commands::save_config(&state, &config).unwrap();

    let pack = app_commands::create_pack(
        &state,
        "good",
        "me",
        "1",
        None,
        vec!["zh-CN".to_string(), "x-fan".to_string()],
        Some("x-fan".to_string()),
    )
    .unwrap();
    app_commands::open_pack(&state, &pack.id).unwrap();

    let unknown_card = app_commands::create_card(
        &state,
        CreateCardInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card: monster_input_with_texts(
                150_000_001,
                BTreeMap::from([(
                    "fr-FR".to_string(),
                    CardTexts {
                        name: "Bonjour".to_string(),
                        desc: "desc".to_string(),
                        strings: vec![],
                    },
                )]),
            ),
        },
    )
    .unwrap_err();
    assert_eq!(unknown_card.code, "card.language_validation_failed");

    let created = match app_commands::create_card(
        &state,
        CreateCardInput {
            workspace_id: workspace.id.clone(),
            pack_id: pack.id.clone(),
            card: monster_input_with_texts(
                150_000_002,
                BTreeMap::from([(
                    "x-fan".to_string(),
                    CardTexts {
                        name: "Fan".to_string(),
                        desc: "desc".to_string(),
                        strings: vec![],
                    },
                )]),
            ),
        },
    )
    .unwrap()
    {
        WriteResultDto::Ok { data, .. } => data,
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    };
    assert!(created.card.texts.contains_key("x-fan"));

    let pack_path = pack_locator::resolve_pack_path(
        &pack_locator::load_workspace_pack_inventory(&workspace_path).unwrap(),
        &pack.id,
    )
    .unwrap();
    let mut cards = json_store::load_cards(&pack_path).unwrap();
    cards[0].texts.insert(
        "fr-FR".to_string(),
        CardTexts {
            name: "Legacy".to_string(),
            desc: "legacy".to_string(),
            strings: vec![],
        },
    );
    json_store::save_cards(&pack_path, &cards).unwrap();
    app_commands::close_pack(&state, &pack.id).unwrap();
    app_commands::open_pack(&state, &pack.id).unwrap();

    let preserved = match app_commands::update_card(
        &state,
        UpdateCardInput {
            workspace_id: workspace.id,
            pack_id: pack.id.clone(),
            card_id: created.card.id,
            card: monster_input_with_texts(
                150_000_002,
                BTreeMap::from([
                    (
                        "x-fan".to_string(),
                        CardTexts {
                            name: "Fan 2".to_string(),
                            desc: "desc".to_string(),
                            strings: vec![],
                        },
                    ),
                    (
                        "fr-FR".to_string(),
                        CardTexts {
                            name: "Legacy".to_string(),
                            desc: "legacy".to_string(),
                            strings: vec![],
                        },
                    ),
                ]),
            ),
        },
    )
    .unwrap()
    {
        WriteResultDto::Ok { data, .. } => data,
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    };
    assert!(preserved.card.texts.contains_key("fr-FR"));
}

fn create_card_direct(
    state: &ygocmg_core::bootstrap::app_state::AppState,
    workspace_id: &str,
    pack_id: &str,
    code: u32,
    primary_type: PrimaryType,
    spell_subtype: Option<SpellSubtype>,
    name: &str,
) -> ygocmg_core::application::dto::card::EditableCardDto {
    let is_monster = matches!(primary_type, PrimaryType::Monster);
    let mut texts = BTreeMap::new();
    texts.insert(
        "zh-CN".to_string(),
        CardTexts {
            name: name.to_string(),
            desc: format!("{name} desc"),
            strings: vec![],
        },
    );

    match app_commands::create_card(
        state,
        CreateCardInput {
            workspace_id: workspace_id.to_string(),
            pack_id: pack_id.to_string(),
            card: CardUpdateInput {
                code,
                alias: 0,
                setcodes: vec![],
                ot: Ot::Custom,
                category: 0,
                primary_type,
                texts,
                monster_flags: if is_monster {
                    Some(vec![MonsterFlag::Effect])
                } else {
                    None
                },
                atk: if is_monster { Some(1500) } else { None },
                def: if is_monster { Some(1200) } else { None },
                race: if is_monster {
                    Some(Race::Warrior)
                } else {
                    None
                },
                attribute: if is_monster {
                    Some(Attribute::Light)
                } else {
                    None
                },
                level: if is_monster { Some(4) } else { None },
                pendulum: None,
                link: None,
                spell_subtype,
                trap_subtype: None,
            },
        },
    )
    .unwrap()
    {
        WriteResultDto::Ok { data, .. } => data.card,
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    }
}

fn update_card_direct(
    state: &ygocmg_core::bootstrap::app_state::AppState,
    workspace_id: &str,
    pack_id: &str,
    card_id: &str,
    code: u32,
    primary_type: PrimaryType,
    spell_subtype: Option<SpellSubtype>,
    name: &str,
) -> ygocmg_core::application::dto::card::EditableCardDto {
    let is_monster = matches!(primary_type, PrimaryType::Monster);
    let mut texts = BTreeMap::new();
    texts.insert(
        "zh-CN".to_string(),
        CardTexts {
            name: name.to_string(),
            desc: format!("{name} desc"),
            strings: vec![],
        },
    );

    match app_commands::update_card(
        state,
        UpdateCardInput {
            workspace_id: workspace_id.to_string(),
            pack_id: pack_id.to_string(),
            card_id: card_id.to_string(),
            card: CardUpdateInput {
                code,
                alias: 0,
                setcodes: vec![],
                ot: Ot::Custom,
                category: 0,
                primary_type,
                texts,
                monster_flags: if is_monster {
                    Some(vec![MonsterFlag::Effect])
                } else {
                    None
                },
                atk: if is_monster { Some(1600) } else { None },
                def: if is_monster { Some(1200) } else { None },
                race: if is_monster {
                    Some(Race::Warrior)
                } else {
                    None
                },
                attribute: if is_monster {
                    Some(Attribute::Light)
                } else {
                    None
                },
                level: if is_monster { Some(4) } else { None },
                pendulum: None,
                link: None,
                spell_subtype,
                trap_subtype: None,
            },
        },
    )
    .unwrap()
    {
        WriteResultDto::Ok { data, .. } => data.card,
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    }
}

fn monster_input_with_texts(code: u32, texts: BTreeMap<String, CardTexts>) -> CardUpdateInput {
    CardUpdateInput {
        code,
        alias: 0,
        setcodes: vec![],
        ot: Ot::Custom,
        category: 0,
        primary_type: PrimaryType::Monster,
        texts,
        monster_flags: Some(vec![MonsterFlag::Effect]),
        atk: Some(1500),
        def: Some(1200),
        race: Some(Race::Warrior),
        attribute: Some(Attribute::Light),
        level: Some(4),
        pendulum: None,
        link: None,
        spell_subtype: None,
        trap_subtype: None,
    }
}

fn create_png(path: &Path, width: u32, height: u32) -> Result<(), Box<dyn std::error::Error>> {
    let image = image::RgbImage::from_fn(width, height, |x, y| {
        image::Rgb([(x % 255) as u8, (y % 255) as u8, ((x + y) % 255) as u8])
    });
    image.save(path)?;
    Ok(())
}
