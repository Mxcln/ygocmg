use std::collections::BTreeMap;
use std::fs;
use std::thread;
use std::time::{Duration, Instant};

use tempfile::tempdir;
use ygocmg_core::application::dto::card::{CreateCardInput, UpdateCardInput};
use ygocmg_core::application::dto::common::WriteResultDto;
use ygocmg_core::application::dto::export::{ExecuteExportBundleInput, PreviewExportBundleInput};
use ygocmg_core::application::dto::job::{GetJobStatusInput, JobSnapshotDto, JobStatusDto};
use ygocmg_core::application::dto::strings::{
    ConfirmPackStringsWriteInput, PackStringEntryDto, UpsertPackStringInput,
};
use ygocmg_core::bootstrap::wiring::build_app_state;
use ygocmg_core::domain::card::model::{
    Attribute, CardTexts, CardUpdateInput, LinkData, MonsterFlag, Ot, Pendulum, PrimaryType, Race,
    SpellSubtype,
};
use ygocmg_core::domain::common::issue::IssueLevel;
use ygocmg_core::domain::resource::path_rules::{card_image_path, field_image_path, script_path};
use ygocmg_core::domain::strings::model::PackStringKind;
use ygocmg_core::infrastructure::pack_locator;
use ygocmg_core::presentation::commands::app_commands;

#[test]
fn exports_selected_open_packs_to_runtime_bundle() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let output_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-export-success");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    app_commands::initialize(&state).unwrap();
    let workspace =
        app_commands::create_workspace(&state, workspace_path.clone(), "Workspace Export", None)
            .unwrap();
    app_commands::open_workspace(&state, workspace_path.clone()).unwrap();

    let pack_a = create_open_pack(&state, "Pack A");
    let card_a = create_card(
        &state,
        &workspace.id,
        &pack_a.id,
        CardSpec::monster(100_000_100, "Export Monster"),
    );
    upsert_setname(&state, &workspace.id, &pack_a.id, 0x345, "Export Set");

    let pack_b = create_open_pack(&state, "Pack B");
    let card_b = create_card(
        &state,
        &workspace.id,
        &pack_b.id,
        CardSpec::field_spell(100_000_110, "Export Field"),
    );

    let inventory = pack_locator::load_workspace_pack_inventory(&workspace_path).unwrap();
    let pack_a_path = pack_locator::resolve_pack_path(&inventory, &pack_a.id).unwrap();
    let pack_b_path = pack_locator::resolve_pack_path(&inventory, &pack_b.id).unwrap();
    fs::write(card_image_path(&pack_a_path, card_a.code), b"main-a").unwrap();
    fs::write(script_path(&pack_a_path, card_a.code), b"-- script a").unwrap();
    fs::write(card_image_path(&pack_b_path, card_b.code), b"main-b").unwrap();
    fs::write(field_image_path(&pack_b_path, card_b.code), b"field-b").unwrap();
    fs::write(script_path(&pack_b_path, card_b.code), b"-- script b").unwrap();

    app_commands::close_pack(&state, &pack_a.id).unwrap();
    app_commands::close_pack(&state, &pack_b.id).unwrap();
    app_commands::open_pack(&state, &pack_a.id).unwrap();
    app_commands::open_pack(&state, &pack_b.id).unwrap();

    let preview = app_commands::preview_export_bundle(
        &state,
        PreviewExportBundleInput {
            workspace_id: workspace.id.clone(),
            pack_ids: vec![pack_a.id.clone(), pack_b.id.clone()],
            export_language: "zh-CN".to_string(),
            output_dir: output_root.path().to_path_buf(),
            output_name: "bundle".to_string(),
        },
    )
    .unwrap();
    assert_eq!(preview.data.error_count, 0);
    assert_eq!(preview.data.pack_count, 2);
    assert_eq!(preview.data.card_count, 2);
    assert_eq!(preview.data.main_image_count, 2);
    assert_eq!(preview.data.field_image_count, 1);
    assert_eq!(preview.data.script_count, 2);

    let accepted = app_commands::execute_export_bundle(
        &state,
        ExecuteExportBundleInput {
            preview_token: preview.preview_token,
        },
    )
    .unwrap();
    wait_for_job_success(&state, &accepted.job_id);

    let bundle_dir = output_root.path().join("bundle");
    assert!(bundle_dir.join("bundle.cdb").exists());
    assert_eq!(
        fs::read(bundle_dir.join("pics").join("100000100.jpg")).unwrap(),
        b"main-a"
    );
    assert_eq!(
        fs::read(bundle_dir.join("pics").join("field").join("100000110.jpg")).unwrap(),
        b"field-b"
    );
    assert_eq!(
        fs::read_to_string(bundle_dir.join("script").join("c100000110.lua")).unwrap(),
        "-- script b"
    );
    let strings_conf = fs::read_to_string(bundle_dir.join("strings.conf")).unwrap();
    assert!(strings_conf.contains("!setname"));
    assert!(strings_conf.contains("0x345 Export Set"));

    let exported_cards = ygocmg_core::infrastructure::ygopro_cdb::load_cards_from_cdb(
        &bundle_dir.join("bundle.cdb"),
    )
    .unwrap();
    assert_eq!(exported_cards.len(), 2);
    let monster = exported_cards
        .iter()
        .find(|record| record.card.code == 100_000_100)
        .unwrap();
    assert_eq!(monster.card.texts["default"].name, "Export Monster");
    assert_eq!(monster.card.level, Some(4));
}

#[test]
fn preview_blocks_missing_language_and_full_setname_conflict() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let output_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-export-errors");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    app_commands::initialize(&state).unwrap();
    let workspace =
        app_commands::create_workspace(&state, workspace_path, "Workspace Export Errors", None)
            .unwrap();
    app_commands::open_workspace(
        &state,
        workspace_root.path().join("workspace-export-errors"),
    )
    .unwrap();

    let pack_a = create_open_pack(&state, "Pack A");
    create_card(
        &state,
        &workspace.id,
        &pack_a.id,
        CardSpec::monster(100_000_120, "Has Language"),
    );
    upsert_setname(&state, &workspace.id, &pack_a.id, 0x456, "Set A");

    let pack_b = create_open_pack(&state, "Pack B");
    create_card(
        &state,
        &workspace.id,
        &pack_b.id,
        CardSpec::monster_missing_language(100_000_130, "Only English"),
    );
    upsert_setname(&state, &workspace.id, &pack_b.id, 0x456, "Set B");

    let preview = app_commands::preview_export_bundle(
        &state,
        PreviewExportBundleInput {
            workspace_id: workspace.id,
            pack_ids: vec![pack_a.id, pack_b.id],
            export_language: "zh-CN".to_string(),
            output_dir: output_root.path().to_path_buf(),
            output_name: "bundle".to_string(),
        },
    )
    .unwrap();

    assert!(preview.data.error_count >= 2);
    assert_issue(
        &preview.data.issues,
        "export.card_text_missing_target_language",
        IssueLevel::Error,
    );
    assert_issue(
        &preview.data.issues,
        "export.setname_key_conflicts_between_selected_packs",
        IssueLevel::Error,
    );
}

#[test]
fn setname_base_overlap_warns_without_blocking_execute() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let output_root = tempdir().unwrap();
    let workspace_path = workspace_root
        .path()
        .join("workspace-export-setname-warning");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    app_commands::initialize(&state).unwrap();
    let workspace =
        app_commands::create_workspace(&state, workspace_path, "Workspace Setname", None).unwrap();
    app_commands::open_workspace(
        &state,
        workspace_root
            .path()
            .join("workspace-export-setname-warning"),
    )
    .unwrap();

    let pack_a = create_open_pack(&state, "Pack A");
    create_card(
        &state,
        &workspace.id,
        &pack_a.id,
        CardSpec::monster(100_000_140, "Parent Set Card"),
    );
    upsert_setname(&state, &workspace.id, &pack_a.id, 0x345, "Parent Set");

    let pack_b = create_open_pack(&state, "Pack B");
    create_card(
        &state,
        &workspace.id,
        &pack_b.id,
        CardSpec::monster(100_000_150, "Child Set Card"),
    );
    upsert_setname(&state, &workspace.id, &pack_b.id, 0x1345, "Child Set");

    let preview = app_commands::preview_export_bundle(
        &state,
        PreviewExportBundleInput {
            workspace_id: workspace.id,
            pack_ids: vec![pack_a.id, pack_b.id],
            export_language: "zh-CN".to_string(),
            output_dir: output_root.path().to_path_buf(),
            output_name: "bundle".to_string(),
        },
    )
    .unwrap();

    assert_eq!(preview.data.error_count, 0);
    assert_issue(
        &preview.data.issues,
        "export.setname_base_overlaps_between_selected_packs",
        IssueLevel::Warning,
    );

    let accepted = app_commands::execute_export_bundle(
        &state,
        ExecuteExportBundleInput {
            preview_token: preview.preview_token,
        },
    )
    .unwrap();
    wait_for_job_success(&state, &accepted.job_id);
}

#[test]
fn export_preview_token_is_consumed_and_cleared_on_workspace_switch() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let output_root = tempdir().unwrap();
    let workspace_a = workspace_root.path().join("workspace-token-a");
    let workspace_b = workspace_root.path().join("workspace-token-b");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    app_commands::initialize(&state).unwrap();
    let workspace =
        app_commands::create_workspace(&state, workspace_a.clone(), "Workspace Token A", None)
            .unwrap();
    app_commands::create_workspace(&state, workspace_b.clone(), "Workspace Token B", None).unwrap();
    app_commands::open_workspace(&state, workspace_a).unwrap();
    let pack = create_open_pack(&state, "Token Pack");
    create_card(
        &state,
        &workspace.id,
        &pack.id,
        CardSpec::monster(100_000_160, "Token Card"),
    );

    let preview = app_commands::preview_export_bundle(
        &state,
        PreviewExportBundleInput {
            workspace_id: workspace.id,
            pack_ids: vec![pack.id],
            export_language: "zh-CN".to_string(),
            output_dir: output_root.path().to_path_buf(),
            output_name: "bundle".to_string(),
        },
    )
    .unwrap();

    app_commands::open_workspace(&state, workspace_b).unwrap();
    let error = app_commands::execute_export_bundle(
        &state,
        ExecuteExportBundleInput {
            preview_token: preview.preview_token,
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "export.preview_token_invalid");
}

#[test]
fn export_execute_fails_when_preview_is_stale() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let output_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-export-stale");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    app_commands::initialize(&state).unwrap();
    let workspace =
        app_commands::create_workspace(&state, workspace_path, "Workspace Stale", None).unwrap();
    app_commands::open_workspace(&state, workspace_root.path().join("workspace-export-stale"))
        .unwrap();
    let pack = create_open_pack(&state, "Stale Pack");
    let card = create_card(
        &state,
        &workspace.id,
        &pack.id,
        CardSpec::monster(100_000_170, "Before"),
    );

    let preview = app_commands::preview_export_bundle(
        &state,
        PreviewExportBundleInput {
            workspace_id: workspace.id.clone(),
            pack_ids: vec![pack.id.clone()],
            export_language: "zh-CN".to_string(),
            output_dir: output_root.path().to_path_buf(),
            output_name: "bundle".to_string(),
        },
    )
    .unwrap();

    match app_commands::update_card(
        &state,
        UpdateCardInput {
            workspace_id: workspace.id,
            pack_id: pack.id,
            card_id: card.id,
            card: CardSpec::monster(100_000_170, "After").into_update(),
        },
    )
    .unwrap()
    {
        WriteResultDto::Ok { .. } => {}
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation"),
    }

    let accepted = app_commands::execute_export_bundle(
        &state,
        ExecuteExportBundleInput {
            preview_token: preview.preview_token,
        },
    )
    .unwrap();
    let failed = wait_for_job_failed(&state, &accepted.job_id);
    assert_eq!(failed.error.unwrap().code, "export.preview_stale");
}

#[test]
fn preview_rejects_unsafe_output_names() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let output_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-export-output-name");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    app_commands::initialize(&state).unwrap();
    let workspace =
        app_commands::create_workspace(&state, workspace_path.clone(), "Workspace Output", None)
            .unwrap();
    app_commands::open_workspace(&state, workspace_path).unwrap();
    let pack = create_open_pack(&state, "Output Pack");

    for output_name in ["../bundle", "nested/bundle", "bundle\\nested", "CON"] {
        let error = app_commands::preview_export_bundle(
            &state,
            PreviewExportBundleInput {
                workspace_id: workspace.id.clone(),
                pack_ids: vec![pack.id.clone()],
                export_language: "zh-CN".to_string(),
                output_dir: output_root.path().to_path_buf(),
                output_name: output_name.to_string(),
            },
        )
        .unwrap_err();
        assert_eq!(
            error.code, "export.output_name_invalid",
            "unexpected error for output name {output_name:?}: {error:?}"
        );
    }
}

#[test]
fn preview_rejects_duplicate_pack_ids() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let output_root = tempdir().unwrap();
    let workspace_path = workspace_root
        .path()
        .join("workspace-export-duplicate-pack");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    app_commands::initialize(&state).unwrap();
    let workspace =
        app_commands::create_workspace(&state, workspace_path.clone(), "Workspace Duplicate", None)
            .unwrap();
    app_commands::open_workspace(&state, workspace_path).unwrap();
    let pack = create_open_pack(&state, "Duplicate Pack");

    let error = app_commands::preview_export_bundle(
        &state,
        PreviewExportBundleInput {
            workspace_id: workspace.id,
            pack_ids: vec![pack.id.clone(), pack.id],
            export_language: "zh-CN".to_string(),
            output_dir: output_root.path().to_path_buf(),
            output_name: "bundle".to_string(),
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "export.pack_ids_duplicate");
}

#[test]
fn export_execute_fails_when_output_dir_becomes_non_empty_after_preview() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let output_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-export-output-race");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    app_commands::initialize(&state).unwrap();
    let workspace = app_commands::create_workspace(
        &state,
        workspace_path.clone(),
        "Workspace Output Race",
        None,
    )
    .unwrap();
    app_commands::open_workspace(&state, workspace_path).unwrap();
    let pack = create_open_pack(&state, "Output Race Pack");
    create_card(
        &state,
        &workspace.id,
        &pack.id,
        CardSpec::monster(100_000_180, "Output Race Card"),
    );

    let preview = app_commands::preview_export_bundle(
        &state,
        PreviewExportBundleInput {
            workspace_id: workspace.id,
            pack_ids: vec![pack.id],
            export_language: "zh-CN".to_string(),
            output_dir: output_root.path().to_path_buf(),
            output_name: "bundle".to_string(),
        },
    )
    .unwrap();
    assert_eq!(preview.data.error_count, 0);

    let bundle_dir = output_root.path().join("bundle");
    fs::create_dir_all(&bundle_dir).unwrap();
    fs::write(bundle_dir.join("existing.txt"), b"keep me").unwrap();

    let accepted = app_commands::execute_export_bundle(
        &state,
        ExecuteExportBundleInput {
            preview_token: preview.preview_token,
        },
    )
    .unwrap();
    let failed = wait_for_job_failed(&state, &accepted.job_id);
    assert_eq!(failed.error.unwrap().code, "export.preview_has_errors");
    assert_eq!(
        fs::read(bundle_dir.join("existing.txt")).unwrap(),
        b"keep me"
    );
    assert!(!bundle_dir.join("bundle.cdb").exists());
}

fn create_open_pack(
    state: &ygocmg_core::bootstrap::app_state::AppState,
    name: &str,
) -> ygocmg_core::domain::pack::model::PackMetadata {
    let pack = app_commands::create_pack(
        state,
        name,
        None,
        "Tester",
        "1.0.0",
        None,
        vec!["zh-CN".to_string()],
        Some("zh-CN".to_string()),
    )
    .unwrap();
    app_commands::open_pack(state, &pack.id).unwrap()
}

fn create_card(
    state: &ygocmg_core::bootstrap::app_state::AppState,
    workspace_id: &str,
    pack_id: &str,
    spec: CardSpec,
) -> ygocmg_core::application::dto::card::EditableCardDto {
    match app_commands::create_card(
        state,
        CreateCardInput {
            workspace_id: workspace_id.to_string(),
            pack_id: pack_id.to_string(),
            card: spec.into_update(),
        },
    )
    .unwrap()
    {
        WriteResultDto::Ok { data, .. } => data.card,
        WriteResultDto::NeedsConfirmation { .. } => panic!("unexpected confirmation"),
    }
}

fn upsert_setname(
    state: &ygocmg_core::bootstrap::app_state::AppState,
    workspace_id: &str,
    pack_id: &str,
    key: u32,
    value: &str,
) {
    match app_commands::upsert_pack_string(
        state,
        UpsertPackStringInput {
            workspace_id: workspace_id.to_string(),
            pack_id: pack_id.to_string(),
            language: "zh-CN".to_string(),
            entry: PackStringEntryDto {
                kind: PackStringKind::Setname,
                key,
                value: value.to_string(),
            },
        },
    )
    .unwrap()
    {
        WriteResultDto::Ok { .. } => {}
        WriteResultDto::NeedsConfirmation {
            confirmation_token, ..
        } => {
            app_commands::confirm_pack_strings_write(
                state,
                ConfirmPackStringsWriteInput { confirmation_token },
            )
            .unwrap();
        }
    }
}

#[derive(Debug)]
struct CardSpec {
    code: u32,
    name: String,
    language: String,
    primary_type: PrimaryType,
    spell_subtype: Option<SpellSubtype>,
}

impl CardSpec {
    fn monster(code: u32, name: &str) -> Self {
        Self {
            code,
            name: name.to_string(),
            language: "zh-CN".to_string(),
            primary_type: PrimaryType::Monster,
            spell_subtype: None,
        }
    }

    fn monster_missing_language(code: u32, name: &str) -> Self {
        Self {
            code,
            name: name.to_string(),
            language: "en-US".to_string(),
            primary_type: PrimaryType::Monster,
            spell_subtype: None,
        }
    }

    fn field_spell(code: u32, name: &str) -> Self {
        Self {
            code,
            name: name.to_string(),
            language: "zh-CN".to_string(),
            primary_type: PrimaryType::Spell,
            spell_subtype: Some(SpellSubtype::Field),
        }
    }

    fn into_update(self) -> CardUpdateInput {
        let is_monster = matches!(self.primary_type, PrimaryType::Monster);
        let mut texts = BTreeMap::new();
        texts.insert(
            self.language,
            CardTexts {
                name: self.name.clone(),
                desc: format!("{} desc", self.name),
                strings: vec!["hint one".to_string()],
            },
        );

        CardUpdateInput {
            code: self.code,
            alias: 0,
            setcodes: if is_monster { vec![0x345] } else { vec![] },
            ot: Ot::Custom,
            category: 0,
            primary_type: self.primary_type,
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
            pendulum: None::<Pendulum>,
            link: None::<LinkData>,
            spell_subtype: self.spell_subtype,
            trap_subtype: None,
        }
    }
}

fn assert_issue(
    issues: &[ygocmg_core::domain::common::issue::ValidationIssue],
    code: &str,
    level: IssueLevel,
) {
    assert!(
        issues
            .iter()
            .any(|issue| issue.code == code && issue.level == level),
        "missing issue {code} at level {level:?}; issues: {issues:?}"
    );
}

fn wait_for_job_success(state: &ygocmg_core::bootstrap::AppState, job_id: &str) {
    let snapshot = wait_for_job_terminal(state, job_id);
    if snapshot.status != JobStatusDto::Succeeded {
        panic!("job did not succeed: {:?}", snapshot);
    }
}

fn wait_for_job_failed(state: &ygocmg_core::bootstrap::AppState, job_id: &str) -> JobSnapshotDto {
    let snapshot = wait_for_job_terminal(state, job_id);
    if snapshot.status != JobStatusDto::Failed {
        panic!("job did not fail: {:?}", snapshot);
    }
    snapshot
}

fn wait_for_job_terminal(state: &ygocmg_core::bootstrap::AppState, job_id: &str) -> JobSnapshotDto {
    let started = Instant::now();
    loop {
        let snapshot = app_commands::get_job_status(
            state,
            GetJobStatusInput {
                job_id: job_id.to_string(),
            },
        )
        .unwrap();
        match snapshot.status {
            JobStatusDto::Succeeded | JobStatusDto::Failed | JobStatusDto::Cancelled => {
                return snapshot;
            }
            _ => {}
        }
        if started.elapsed() > Duration::from_secs(5) {
            panic!("job timed out: {:?}", snapshot);
        }
        thread::sleep(Duration::from_millis(10));
    }
}
