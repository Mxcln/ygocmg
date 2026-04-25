use std::collections::BTreeMap;
use std::fs;

use tempfile::tempdir;
use ygocmg_core::bootstrap::wiring::build_app_state;
use ygocmg_core::domain::card::model::{
    Attribute, CardTexts, CardUpdateInput, MonsterFlag, Ot, PrimaryType, Race,
};
use ygocmg_core::domain::resource::path_rules::script_path;
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

    let mut texts = BTreeMap::new();
    texts.insert(
        "zh-CN".to_string(),
        CardTexts {
            name: "测试怪兽".to_string(),
            desc: "一张用于测试的卡片".to_string(),
            strings: vec![],
        },
    );

    let card = app_commands::create_card(
        &state,
        &pack.id,
        CardUpdateInput {
            code: 100_000_000,
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
    )
    .unwrap();

    let pack_path = workspace_path.join("packs").join(&pack.id);
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

    let updated = app_commands::update_card(
        &state,
        &pack.id,
        &card.id,
        CardUpdateInput {
            code: 100_000_010,
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
    )
    .unwrap();

    assert_eq!(updated.code, 100_000_010);
    assert!(!original_script.exists());
    assert!(script_path(&pack_path, updated.code).exists());

    let reopened_state = build_app_state(app_dir.path().to_path_buf()).unwrap();
    app_commands::open_workspace(&reopened_state, workspace_path.clone()).unwrap();
    app_commands::open_pack(&reopened_state, &pack.id).unwrap();
    let rows = app_commands::list_cards(&reopened_state, &pack.id).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].code, 100_000_010);
    assert!(rows[0].has_script);
}
