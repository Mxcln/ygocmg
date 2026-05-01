use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use image::{Rgb, RgbImage};
use rusqlite::Connection;
use tempfile::tempdir;
use ygocmg_core::application::dto::import::{ExecuteImportPackInput, PreviewImportPackInput};
use ygocmg_core::application::dto::job::{GetJobStatusInput, JobStatusDto};
use ygocmg_core::bootstrap::wiring::build_app_state;
use ygocmg_core::domain::resource::path_rules::{card_image_path, field_image_path, script_path};
use ygocmg_core::domain::strings::model::PackStringKind;
use ygocmg_core::infrastructure::{json_store, pack_locator};
use ygocmg_core::presentation::commands::app_commands;

#[test]
fn imports_runtime_resources_into_author_pack() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let source_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-import");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    app_commands::initialize(&state).unwrap();
    let workspace =
        app_commands::create_workspace(&state, workspace_path.clone(), "Workspace Import", None)
            .unwrap();
    app_commands::open_workspace(&state, workspace_path.clone()).unwrap();

    let cdb_path = source_root.path().join("cards.cdb");
    write_test_cdb(
        &cdb_path,
        &[(201_100_001, 0x1 | 0x20, "导入怪兽", "怪兽效果")],
    );
    let pics_dir = source_root.path().join("pics");
    let field_dir = pics_dir.join("field");
    let script_dir = source_root.path().join("script");
    fs::create_dir_all(&field_dir).unwrap();
    fs::create_dir_all(&script_dir).unwrap();
    write_test_image(&pics_dir.join("201100001.jpg"), 32, 32);
    write_test_image(&field_dir.join("201100001.jpg"), 32, 24);
    fs::write(script_dir.join("c201100001.lua"), "-- imported script").unwrap();
    let strings_path = source_root.path().join("strings.conf");
    fs::write(&strings_path, "!setname 0x123 Imported Set\n").unwrap();

    let preview = app_commands::preview_import_pack(
        &state,
        PreviewImportPackInput {
            workspace_id: workspace.id.clone(),
            new_pack_name: "Imported Pack".to_string(),
            new_pack_code: None,
            new_pack_author: "Importer".to_string(),
            new_pack_version: "1.0.0".to_string(),
            new_pack_description: Some("from cdb".to_string()),
            display_language_order: vec!["en-US".to_string()],
            default_export_language: Some("zh-CN".to_string()),
            cdb_path: cdb_path.clone(),
            pics_dir: Some(pics_dir.clone()),
            field_pics_dir: Some(field_dir.clone()),
            script_dir: Some(script_dir.clone()),
            strings_conf_path: Some(strings_path),
            source_language: "zh-CN".to_string(),
        },
    )
    .unwrap();

    assert_eq!(preview.data.card_count, 1);
    assert_eq!(preview.data.error_count, 0);
    assert_eq!(preview.data.missing_main_image_count, 0);
    assert_eq!(preview.data.missing_script_count, 0);
    assert!(
        preview
            .data
            .issues
            .iter()
            .any(|issue| issue.code == "import.source_language_not_in_display_order")
    );
    let target_pack_id = preview.data.target_pack_id.clone();

    let accepted = app_commands::execute_import_pack(
        &state,
        ExecuteImportPackInput {
            preview_token: preview.preview_token,
        },
    )
    .unwrap();
    wait_for_job_success(&state, &accepted.job_id);

    let pack_path = pack_locator::resolve_pack_path(
        &pack_locator::load_workspace_pack_inventory(&workspace_path).unwrap(),
        &target_pack_id,
    )
    .unwrap();
    let metadata = json_store::load_pack_metadata(&pack_path).unwrap();
    assert_eq!(metadata.name, "Imported Pack");
    assert_eq!(metadata.display_language_order[0], "zh-CN");

    let cards = json_store::load_cards(&pack_path).unwrap();
    assert_eq!(cards.len(), 1);
    assert!(cards[0].texts.contains_key("zh-CN"));
    assert!(!cards[0].texts.contains_key("default"));
    assert_eq!(cards[0].texts["zh-CN"].name, "导入怪兽");

    let strings = json_store::load_pack_strings(&pack_path).unwrap();
    assert_eq!(strings.entries.len(), 1);
    assert_eq!(strings.entries[0].kind, PackStringKind::Setname);
    assert!(strings.entries[0].values.contains_key("zh-CN"));
    assert!(!strings.entries[0].values.contains_key("default"));

    assert!(card_image_path(&pack_path, 201_100_001).exists());
    assert!(field_image_path(&pack_path, 201_100_001).exists());
    assert!(script_path(&pack_path, 201_100_001).exists());

    let opened = app_commands::open_pack(&state, &target_pack_id).unwrap();
    assert_eq!(opened.id, target_pack_id);
}

#[test]
fn missing_resources_are_warnings_and_do_not_block_import() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let source_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-missing-resources");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    app_commands::initialize(&state).unwrap();
    let workspace =
        app_commands::create_workspace(&state, workspace_path, "Workspace Missing", None).unwrap();
    app_commands::open_workspace(
        &state,
        workspace_root.path().join("workspace-missing-resources"),
    )
    .unwrap();

    let cdb_path = source_root.path().join("cards.cdb");
    write_test_cdb(
        &cdb_path,
        &[(201_100_010, 0x2 | 0x80000, "场地魔法", "场地效果")],
    );

    let preview = app_commands::preview_import_pack(
        &state,
        PreviewImportPackInput {
            workspace_id: workspace.id,
            new_pack_name: "Missing Resources".to_string(),
            new_pack_code: None,
            new_pack_author: "Importer".to_string(),
            new_pack_version: "1.0.0".to_string(),
            new_pack_description: None,
            display_language_order: vec!["zh-CN".to_string()],
            default_export_language: None,
            cdb_path,
            pics_dir: None,
            field_pics_dir: None,
            script_dir: None,
            strings_conf_path: None,
            source_language: "zh-CN".to_string(),
        },
    )
    .unwrap();

    assert_eq!(preview.data.error_count, 0);
    assert_eq!(preview.data.missing_main_image_count, 1);
    assert_eq!(preview.data.missing_script_count, 1);
    assert_eq!(preview.data.missing_field_image_count, 1);
    assert!(preview.data.warning_count >= 3);

    let accepted = app_commands::execute_import_pack(
        &state,
        ExecuteImportPackInput {
            preview_token: preview.preview_token,
        },
    )
    .unwrap();
    wait_for_job_success(&state, &accepted.job_id);
}

#[test]
fn duplicate_codes_block_import_execute() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let source_root = tempdir().unwrap();
    let workspace_path = workspace_root.path().join("workspace-duplicate-codes");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    app_commands::initialize(&state).unwrap();
    let workspace =
        app_commands::create_workspace(&state, workspace_path.clone(), "Workspace Dup", None)
            .unwrap();
    app_commands::open_workspace(&state, workspace_path).unwrap();

    let cdb_path = source_root.path().join("cards.cdb");
    write_test_cdb(
        &cdb_path,
        &[
            (201_100_020, 0x1 | 0x20, "重复一", "效果一"),
            (201_100_020, 0x1 | 0x20, "重复二", "效果二"),
        ],
    );

    let preview = app_commands::preview_import_pack(
        &state,
        PreviewImportPackInput {
            workspace_id: workspace.id,
            new_pack_name: "Duplicate Codes".to_string(),
            new_pack_code: None,
            new_pack_author: "Importer".to_string(),
            new_pack_version: "1.0.0".to_string(),
            new_pack_description: None,
            display_language_order: vec!["zh-CN".to_string()],
            default_export_language: None,
            cdb_path,
            pics_dir: None,
            field_pics_dir: None,
            script_dir: None,
            strings_conf_path: None,
            source_language: "zh-CN".to_string(),
        },
    )
    .unwrap();

    assert!(preview.data.error_count > 0);
    assert!(
        preview
            .data
            .issues
            .iter()
            .any(|issue| issue.code == "import.cdb_duplicate_code")
    );

    let accepted = app_commands::execute_import_pack(
        &state,
        ExecuteImportPackInput {
            preview_token: preview.preview_token,
        },
    )
    .unwrap();
    let failed = wait_for_job_failed(&state, &accepted.job_id);
    assert_eq!(failed.error.unwrap().code, "import.preview_has_errors");
}

#[test]
fn opening_workspace_clears_import_preview_tokens() {
    let app_dir = tempdir().unwrap();
    let workspace_root = tempdir().unwrap();
    let source_root = tempdir().unwrap();
    let workspace_a = workspace_root.path().join("workspace-token-a");
    let workspace_b = workspace_root.path().join("workspace-token-b");
    let state = build_app_state(app_dir.path().to_path_buf()).unwrap();

    app_commands::initialize(&state).unwrap();
    let workspace =
        app_commands::create_workspace(&state, workspace_a.clone(), "Workspace Token A", None)
            .unwrap();
    app_commands::create_workspace(&state, workspace_b.clone(), "Workspace Token B", None).unwrap();
    app_commands::open_workspace(&state, workspace_a).unwrap();

    let cdb_path = source_root.path().join("cards.cdb");
    write_test_cdb(&cdb_path, &[(201_100_030, 0x1 | 0x20, "Token", "Effect")]);
    let preview = app_commands::preview_import_pack(
        &state,
        PreviewImportPackInput {
            workspace_id: workspace.id,
            new_pack_name: "Token Pack".to_string(),
            new_pack_code: None,
            new_pack_author: "Importer".to_string(),
            new_pack_version: "1.0.0".to_string(),
            new_pack_description: None,
            display_language_order: vec!["zh-CN".to_string()],
            default_export_language: None,
            cdb_path,
            pics_dir: None,
            field_pics_dir: None,
            script_dir: None,
            strings_conf_path: None,
            source_language: "zh-CN".to_string(),
        },
    )
    .unwrap();

    app_commands::open_workspace(&state, workspace_b).unwrap();
    let error = app_commands::execute_import_pack(
        &state,
        ExecuteImportPackInput {
            preview_token: preview.preview_token,
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "import.preview_token_invalid");
}

fn write_test_cdb(path: &Path, rows: &[(u32, i64, &str, &str)]) {
    let connection = Connection::open(path).unwrap();
    connection
        .execute(
            "create table datas(id integer, ot integer, alias integer, setcode integer, type integer, atk integer, def integer, level integer, race integer, attribute integer, category integer)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "create table texts(id integer, name text, desc text, str1 text, str2 text, str3 text, str4 text, str5 text, str6 text, str7 text, str8 text, str9 text, str10 text, str11 text, str12 text, str13 text, str14 text, str15 text, str16 text)",
            [],
        )
        .unwrap();
    for (code, raw_type, name, desc) in rows {
        connection
            .execute(
                "insert into datas values (?1, 3, 0, 0, ?2, 1000, 1000, 4, 1, 16, 0)",
                (*code as i64, *raw_type),
            )
            .unwrap();
        connection
            .execute(
                "insert into texts values (?1, ?2, ?3, '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '')",
                (*code as i64, *name, *desc),
            )
            .unwrap();
    }
}

fn write_test_image(path: &Path, width: u32, height: u32) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let image = RgbImage::from_pixel(width, height, Rgb([120, 40, 200]));
    image.save(path).unwrap();
}

fn wait_for_job_success(state: &ygocmg_core::bootstrap::AppState, job_id: &str) {
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
            JobStatusDto::Succeeded => return,
            JobStatusDto::Failed => panic!("job failed: {:?}", snapshot.error),
            _ => {}
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "timed out waiting for import job"
        );
        thread::sleep(Duration::from_millis(25));
    }
}

fn wait_for_job_failed(
    state: &ygocmg_core::bootstrap::AppState,
    job_id: &str,
) -> ygocmg_core::application::dto::job::JobSnapshotDto {
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
            JobStatusDto::Failed => return snapshot,
            JobStatusDto::Succeeded => panic!("job unexpectedly succeeded"),
            _ => {}
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "timed out waiting for import job failure"
        );
        thread::sleep(Duration::from_millis(25));
    }
}
