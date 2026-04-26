use std::collections::BTreeMap;
use std::fs;

use tempfile::tempdir;
use ygocmg_core::bootstrap::wiring::build_app_state;
use ygocmg_core::application::dto::card::{
    CardSortFieldDto, CreateCardInput, GetCardInput, ListCardsInput, SortDirectionDto,
    SuggestCodeInput, UpdateCardInput,
};
use ygocmg_core::application::dto::common::WriteResultDto;
use ygocmg_core::domain::card::model::{
    Attribute, CardTexts, CardUpdateInput, MonsterFlag, Ot, PrimaryType, Race,
};
use ygocmg_core::domain::resource::path_rules::script_path;
use ygocmg_core::infrastructure::json_store;
use ygocmg_core::infrastructure::pack_locator;
use ygocmg_core::presentation::commands::app_commands;

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
                setcode: 0,
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
        WriteResultDto::Ok { data, warnings } => {
            assert!(!warnings.is_empty(), "code outside recommended range should produce warnings");
            data.card
        }
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
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
                setcode: 0,
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
        WriteResultDto::Ok { data, warnings } => {
            assert!(!warnings.is_empty(), "code outside recommended range should produce warnings");
            data.card
        }
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation result"),
    };

    assert_eq!(updated.code, 201_000_010);
    assert!(!original_script.exists());
    assert!(script_path(&pack_path, updated.code).exists());

    let reopened_state = build_app_state(app_dir.path().to_path_buf()).unwrap();
    let reopened_workspace = app_commands::open_workspace(&reopened_state, workspace_path.clone()).unwrap();
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
                setcode: 0,
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

    let duplicate_dir = workspace_path.join("packs").join("duplicate-pack--cafebabe");
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
