use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use rusqlite::Connection;
use tempfile::tempdir;
use ygocmg_core::application::dto::card::{CreateCardInput, SortDirectionDto};
use ygocmg_core::application::dto::export::PreviewExportBundleInput;
use ygocmg_core::application::dto::job::{GetJobStatusInput, JobStatusDto};
use ygocmg_core::application::dto::standard_pack::{
    SearchStandardCardsInput, SearchStandardStringsInput, StandardCardSortFieldDto,
    StandardStringSortFieldDto,
};
use ygocmg_core::application::dto::strings::{
    PackStringRecordDto, PackStringValueDto, UpsertPackStringRecordInput,
};
use ygocmg_core::application::standard_pack::service::StandardPackService;
use ygocmg_core::bootstrap::AppState;
use ygocmg_core::domain::card::model::{
    Attribute, CardTexts, CardUpdateInput, MonsterFlag, Ot, PrimaryType, Race,
};
use ygocmg_core::domain::common::issue::IssueLevel;
use ygocmg_core::domain::strings::model::PackStringKind;
use ygocmg_core::presentation::commands::app_commands;

#[test]
fn discover_source_requires_exactly_one_root_cdb() {
    let root = tempdir().unwrap();

    let missing =
        ygocmg_core::infrastructure::standard_pack::discover_source(root.path()).unwrap_err();
    assert_eq!(missing.code, "standard_pack.cdb_missing");

    create_test_cdb(
        &root.path().join("cards.cdb"),
        &[(100, "Alpha", 0x1 | 0x10)],
    )
    .unwrap();
    let source = ygocmg_core::infrastructure::standard_pack::discover_source(root.path()).unwrap();
    assert!(source.cdb_path.ends_with("cards.cdb"));

    create_test_cdb(&root.path().join("other.cdb"), &[(101, "Beta", 0x2)]).unwrap();
    let multiple =
        ygocmg_core::infrastructure::standard_pack::discover_source(root.path()).unwrap_err();
    assert_eq!(multiple.code, "standard_pack.multiple_cdb_files");
}

#[test]
fn rebuild_index_reads_cdb_and_supports_search_and_detail() {
    let app = tempdir().unwrap();
    let root = tempdir().unwrap();
    create_test_cdb(
        &root.path().join("cards.cdb"),
        &[(100, "Alpha Dragon", 0x1 | 0x20), (200, "Beta Spell", 0x2)],
    )
    .unwrap();
    fs::create_dir_all(root.path().join("script")).unwrap();
    fs::write(root.path().join("script").join("c100.lua"), "-- test").unwrap();

    let index =
        ygocmg_core::infrastructure::standard_pack::rebuild_index(root.path(), "zh-CN").unwrap();
    ygocmg_core::infrastructure::standard_pack::save_index(app.path(), &index).unwrap();

    let state = AppState::new(app.path().to_path_buf()).unwrap();
    let service = StandardPackService::new(&state);
    let page = service
        .search_cards(SearchStandardCardsInput {
            keyword: Some("dragon".to_string()),
            sort_by: StandardCardSortFieldDto::Name,
            sort_direction: SortDirectionDto::Asc,
            page: 1,
            page_size: 20,
        })
        .unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].code, 100);

    let detail = service
        .get_card(ygocmg_core::application::dto::standard_pack::GetStandardCardInput { code: 100 })
        .unwrap();
    assert_eq!(detail.card.texts["zh-CN"].name, "Alpha Dragon");
    assert!(detail.asset_state.has_script);
}

#[test]
fn rebuild_index_accepts_signed_32_bit_cdb_bitfields() {
    let root = tempdir().unwrap();
    create_test_cdb_with_category(
        &root.path().join("cards.cdb"),
        &[(300, "Signed Category", 0x1 | 0x20, -2_147_483_384)],
    )
    .unwrap();

    let index =
        ygocmg_core::infrastructure::standard_pack::rebuild_index(root.path(), "zh-CN").unwrap();
    let card = &index.cards[0].card;
    assert_eq!(card.code, 300);
    assert_eq!(card.category, (-2_147_483_384i64 as u32) as u64);
}

#[test]
fn card_code_context_uses_standard_index_before_fallback_baseline() {
    let app = tempdir().unwrap();
    let root = tempdir().unwrap();
    create_test_cdb(
        &root.path().join("cards.cdb"),
        &[(12345678, "Indexed", 0x1 | 0x20)],
    )
    .unwrap();

    let index =
        ygocmg_core::infrastructure::standard_pack::rebuild_index(root.path(), "zh-CN").unwrap();
    ygocmg_core::infrastructure::standard_pack::save_index(app.path(), &index).unwrap();

    let workspace = tempdir().unwrap();
    let state = AppState::new(app.path().to_path_buf()).unwrap();
    let _meta = ygocmg_core::application::workspace::service::WorkspaceService::new(&state)
        .create_workspace(workspace.path(), "ws", None)
        .unwrap();
    ygocmg_core::application::workspace::service::WorkspaceService::new(&state)
        .open_workspace(workspace.path())
        .unwrap();
    let pack = ygocmg_core::application::pack::service::PackService::new(&state)
        .create_pack("pack", "me", "1", None, vec!["zh-CN".to_string()], None)
        .unwrap();
    let context = ygocmg_core::application::card::service::CardService::new(&state)
        .build_code_context(&pack.id, None)
        .unwrap();

    assert!(context.standard_codes.contains(&12345678));
}

#[test]
fn rebuild_index_job_success_and_failure_are_queryable() {
    let app = tempdir().unwrap();
    let root = tempdir().unwrap();
    create_test_cdb(
        &root.path().join("cards.cdb"),
        &[(123, "Indexed", 0x1 | 0x20)],
    )
    .unwrap();

    let state = AppState::new(app.path().to_path_buf()).unwrap();
    let mut config = ygocmg_core::domain::config::rules::default_global_config();
    config.ygopro_path = Some(root.path().to_path_buf());
    config.standard_pack_source_language = Some("zh-CN".to_string());
    app_commands::save_config(&state, &config).unwrap();

    let accepted = app_commands::rebuild_standard_pack_index(&state).unwrap();
    wait_for_status(&state, &accepted.job_id, JobStatusDto::Succeeded);
    let status = app_commands::get_standard_pack_status(&state);
    assert!(status.index_exists);
    assert_eq!(status.card_count, 1);

    fs::remove_file(root.path().join("cards.cdb")).unwrap();
    let failed = app_commands::rebuild_standard_pack_index(&state).unwrap();
    let snapshot = wait_for_status(&state, &failed.job_id, JobStatusDto::Failed);
    assert_eq!(snapshot.error.unwrap().code, "standard_pack.cdb_missing");
}

#[test]
fn custom_card_write_rejects_exact_standard_code_but_warns_for_reserved_gap() {
    let app = tempdir().unwrap();
    let root = tempdir().unwrap();
    create_test_cdb(
        &root.path().join("cards.cdb"),
        &[(12345678, "Indexed", 0x1 | 0x20)],
    )
    .unwrap();

    let index =
        ygocmg_core::infrastructure::standard_pack::rebuild_index(root.path(), "zh-CN").unwrap();
    ygocmg_core::infrastructure::standard_pack::save_index(app.path(), &index).unwrap();

    let workspace = tempdir().unwrap();
    let state = AppState::new(app.path().to_path_buf()).unwrap();
    let workspace_meta =
        app_commands::create_workspace(&state, workspace.path().to_path_buf(), "ws", None).unwrap();
    app_commands::open_workspace(&state, workspace.path().to_path_buf()).unwrap();
    let pack = app_commands::create_pack(
        &state,
        "pack",
        "me",
        "1",
        None,
        vec!["zh-CN".to_string()],
        None,
    )
    .unwrap();
    app_commands::open_pack(&state, &pack.id).unwrap();

    let conflict = app_commands::create_card(
        &state,
        CreateCardInput {
            workspace_id: workspace_meta.id.clone(),
            pack_id: pack.id.clone(),
            card: test_monster_input(12345678, "Conflict"),
        },
    )
    .unwrap_err();
    assert_eq!(conflict.code, "card.code_validation_failed");

    let warning_result = app_commands::create_card(
        &state,
        CreateCardInput {
            workspace_id: workspace_meta.id,
            pack_id: pack.id,
            card: test_monster_input(12_345_679, "Reserved Gap"),
        },
    )
    .unwrap();
    match warning_result {
        ygocmg_core::application::dto::common::WriteResultDto::NeedsConfirmation {
            warnings,
            ..
        } => {
            assert!(
                warnings
                    .iter()
                    .any(|issue| issue.code == "card.code_reserved_range")
            );
            assert!(
                warnings
                    .iter()
                    .all(|issue| matches!(issue.level, IssueLevel::Warning))
            );
        }
        ygocmg_core::application::dto::common::WriteResultDto::Ok { .. } => {
            panic!("reserved standard range should require confirmation")
        }
    }
}

#[test]
fn strings_conf_namespace_enters_standard_baseline() {
    let app = tempdir().unwrap();
    let root = tempdir().unwrap();
    create_test_cdb(
        &root.path().join("cards.cdb"),
        &[(123, "Indexed", 0x1 | 0x20)],
    )
    .unwrap();
    fs::write(
        root.path().join("strings.conf"),
        "!victory\n10 Test victory\n!counter\n101 Test counter\n!setname\n20 Test set\n",
    )
    .unwrap();

    let index =
        ygocmg_core::infrastructure::standard_pack::rebuild_index(root.path(), "zh-CN").unwrap();
    ygocmg_core::infrastructure::standard_pack::save_index(app.path(), &index).unwrap();

    let workspace = tempdir().unwrap();
    let state = AppState::new(app.path().to_path_buf()).unwrap();
    let workspace_meta =
        app_commands::create_workspace(&state, workspace.path().to_path_buf(), "ws", None).unwrap();
    app_commands::open_workspace(&state, workspace.path().to_path_buf()).unwrap();
    let pack = app_commands::create_pack(
        &state,
        "pack",
        "me",
        "1",
        None,
        vec!["zh-CN".to_string()],
        None,
    )
    .unwrap();
    app_commands::open_pack(&state, &pack.id).unwrap();

    let result = app_commands::upsert_pack_string_record(
        &state,
        UpsertPackStringRecordInput {
            workspace_id: workspace_meta.id,
            pack_id: pack.id,
            record: PackStringRecordDto {
                kind: PackStringKind::Victory,
                key: 0x10,
                values: vec![PackStringValueDto {
                    language: "zh-CN".to_string(),
                    value: "custom".to_string(),
                }],
            },
        },
    )
    .unwrap();

    match result {
        ygocmg_core::application::dto::common::WriteResultDto::NeedsConfirmation {
            warnings,
            ..
        } => assert!(warnings.iter().any(|issue| {
            issue.code == "pack_strings.victory_key_conflicts_with_standard_pack"
        })),
        ygocmg_core::application::dto::common::WriteResultDto::Ok { .. } => {
            panic!("standard string namespace conflict should require confirmation")
        }
    }
}

#[test]
fn standard_strings_are_indexed_and_searchable() {
    let app = tempdir().unwrap();
    let root = tempdir().unwrap();
    create_test_cdb(
        &root.path().join("cards.cdb"),
        &[(123, "Indexed", 0x1 | 0x20)],
    )
    .unwrap();
    fs::write(
        root.path().join("strings.conf"),
        "!system\n123 System value\n!victory\n10 Victory value\n!counter\n101 Counter value\n!setname\n20 Set value\n",
    )
    .unwrap();

    let index =
        ygocmg_core::infrastructure::standard_pack::rebuild_index(root.path(), "zh-CN").unwrap();
    assert_eq!(index.strings.records.len(), 4);
    assert!(index.strings.baseline.system_keys.contains(&123));
    assert!(index.strings.baseline.victory_keys.contains(&0x10));
    ygocmg_core::infrastructure::standard_pack::save_index(app.path(), &index).unwrap();

    let state = AppState::new(app.path().to_path_buf()).unwrap();
    let service = StandardPackService::new(&state);
    let page = service
        .search_strings(SearchStandardStringsInput {
            kind_filter: Some(PackStringKind::Victory),
            key_filter: None,
            keyword: Some("victory".to_string()),
            sort_by: StandardStringSortFieldDto::Value,
            sort_direction: SortDirectionDto::Asc,
            page: 1,
            page_size: 20,
        })
        .unwrap();
    assert_eq!(page.language, "zh-CN");
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].key, 0x10);
    assert_eq!(page.items[0].value, "Victory value");

    let key_page = service
        .search_strings(SearchStandardStringsInput {
            kind_filter: None,
            key_filter: Some(0x101),
            keyword: None,
            sort_by: StandardStringSortFieldDto::Key,
            sort_direction: SortDirectionDto::Asc,
            page: 1,
            page_size: 20,
        })
        .unwrap();
    assert_eq!(key_page.total, 1);
    assert_eq!(key_page.items[0].kind, PackStringKind::Counter);
}

#[test]
fn standard_strings_parse_ygopro_inline_format() {
    let root = tempdir().unwrap();
    create_test_cdb(
        &root.path().join("cards.cdb"),
        &[(123, "Indexed", 0x1 | 0x20)],
    )
    .unwrap();
    fs::write(
        root.path().join("strings.conf"),
        "!system 1 通常召唤\n!victory 0x10 特殊胜利\n!counter 0x101 指示物\n!setname 0x20 系列名\n",
    )
    .unwrap();

    let index =
        ygocmg_core::infrastructure::standard_pack::rebuild_index(root.path(), "zh-CN").unwrap();
    assert_eq!(index.strings.records.len(), 4);
    assert!(index.strings.baseline.system_keys.contains(&1));
    assert!(index.strings.baseline.victory_keys.contains(&0x10));
    assert!(index.strings.baseline.counter_keys.contains(&0x101));
    assert!(index.strings.baseline.setname_bases.contains(&0x20));
    assert!(index.strings.records.iter().any(|record| {
        record
            .values
            .get("zh-CN")
            .is_some_and(|value| value == "通常召唤")
    }));
}

#[test]
fn schema_mismatch_requires_rebuild() {
    let app = tempdir().unwrap();
    fs::create_dir_all(ygocmg_core::infrastructure::standard_pack::standard_pack_dir(app.path()))
        .unwrap();
    fs::write(
        ygocmg_core::infrastructure::standard_pack::standard_pack_index_path(app.path()),
        r#"{"schema_version":1,"source":{"ygopro_path":"x","cdb_path":"x","cdb_modified":null,"cdb_len":0,"strings_modified":null,"strings_len":null},"indexed_at":"2026-01-01T00:00:00Z","cards":[],"strings":{"system_keys":[],"victory_keys":[],"counter_keys":[],"setname_bases":[]}}"#,
    )
    .unwrap();

    let err = ygocmg_core::infrastructure::standard_pack::load_index(app.path()).unwrap_err();
    assert_eq!(err.code, "standard_pack.index_schema_mismatch");

    let status = ygocmg_core::infrastructure::standard_pack::status(app.path(), None, None);
    assert!(status.schema_mismatch);
    assert!(!status.index_exists);
}

#[test]
fn source_missing_keeps_existing_index_browsable() {
    let app = tempdir().unwrap();
    let root = tempdir().unwrap();
    create_test_cdb(
        &root.path().join("cards.cdb"),
        &[(123, "Indexed", 0x1 | 0x20)],
    )
    .unwrap();
    fs::write(root.path().join("strings.conf"), "!system\n1 Hello\n").unwrap();

    let index =
        ygocmg_core::infrastructure::standard_pack::rebuild_index(root.path(), "zh-CN").unwrap();
    ygocmg_core::infrastructure::standard_pack::save_index(app.path(), &index).unwrap();
    let missing_path = root.path().join("missing");
    let status = ygocmg_core::infrastructure::standard_pack::status(
        app.path(),
        Some(&missing_path),
        Some("zh-CN"),
    );
    assert!(status.index_exists);
    assert!(status.message.is_some());

    let state = AppState::new(app.path().to_path_buf()).unwrap();
    let service = StandardPackService::new(&state);
    let cards = service
        .search_cards(SearchStandardCardsInput {
            keyword: None,
            sort_by: StandardCardSortFieldDto::Code,
            sort_direction: SortDirectionDto::Asc,
            page: 1,
            page_size: 20,
        })
        .unwrap();
    assert_eq!(cards.total, 1);
    let strings = service
        .search_strings(SearchStandardStringsInput {
            kind_filter: None,
            key_filter: None,
            keyword: None,
            sort_by: StandardStringSortFieldDto::Kind,
            sort_direction: SortDirectionDto::Asc,
            page: 1,
            page_size: 20,
        })
        .unwrap();
    assert_eq!(strings.total, 1);
}

#[test]
fn source_change_marks_stale_without_auto_rebuild() {
    let app = tempdir().unwrap();
    let root = tempdir().unwrap();
    create_test_cdb(
        &root.path().join("cards.cdb"),
        &[(123, "Original", 0x1 | 0x20)],
    )
    .unwrap();

    let index =
        ygocmg_core::infrastructure::standard_pack::rebuild_index(root.path(), "zh-CN").unwrap();
    ygocmg_core::infrastructure::standard_pack::save_index(app.path(), &index).unwrap();

    std::thread::sleep(Duration::from_secs(1));
    fs::write(root.path().join("strings.conf"), "!system 1 Updated\n").unwrap();
    let status = ygocmg_core::infrastructure::standard_pack::status(
        app.path(),
        Some(root.path()),
        Some("zh-CN"),
    );
    assert!(status.index_exists);
    assert!(status.stale);

    let state = AppState::new(app.path().to_path_buf()).unwrap();
    let service = StandardPackService::new(&state);
    let cards = service
        .search_cards(SearchStandardCardsInput {
            keyword: None,
            sort_by: StandardCardSortFieldDto::Code,
            sort_direction: SortDirectionDto::Asc,
            page: 1,
            page_size: 20,
        })
        .unwrap();
    assert_eq!(cards.total, 1);
    assert_eq!(cards.items[0].name, "Original");

    let strings = service
        .search_strings(SearchStandardStringsInput {
            kind_filter: None,
            key_filter: None,
            keyword: None,
            sort_by: StandardStringSortFieldDto::Kind,
            sort_direction: SortDirectionDto::Asc,
            page: 1,
            page_size: 20,
        })
        .unwrap();
    assert_eq!(strings.total, 0);
}

#[test]
fn rebuild_index_uses_prescanned_asset_state() {
    let root = tempdir().unwrap();
    create_test_cdb(
        &root.path().join("cards.cdb"),
        &[
            (123, "Has Assets", 0x1 | 0x20),
            (124, "No Assets", 0x1 | 0x20),
        ],
    )
    .unwrap();
    fs::create_dir_all(root.path().join("pics").join("field")).unwrap();
    fs::create_dir_all(root.path().join("script")).unwrap();
    fs::write(root.path().join("pics").join("123.jpg"), "image").unwrap();
    fs::write(
        root.path().join("pics").join("field").join("123.jpg"),
        "field",
    )
    .unwrap();
    fs::write(root.path().join("script").join("c123.lua"), "-- script").unwrap();
    fs::write(
        root.path().join("script").join("not-a-card.lua"),
        "-- ignored",
    )
    .unwrap();

    let index =
        ygocmg_core::infrastructure::standard_pack::rebuild_index(root.path(), "zh-CN").unwrap();
    let with_assets = index
        .cards
        .iter()
        .find(|record| record.card.code == 123)
        .unwrap();
    assert!(with_assets.asset_state.has_image);
    assert!(with_assets.asset_state.has_field_image);
    assert!(with_assets.asset_state.has_script);

    let without_assets = index
        .cards
        .iter()
        .find(|record| record.card.code == 124)
        .unwrap();
    assert!(!without_assets.asset_state.has_image);
    assert!(!without_assets.asset_state.has_field_image);
    assert!(!without_assets.asset_state.has_script);
}

#[test]
fn malformed_cdb_schema_returns_clear_error() {
    let root = tempdir().unwrap();
    let connection = Connection::open(root.path().join("cards.cdb")).unwrap();
    connection
        .execute_batch("create table datas(id integer primary key);")
        .unwrap();

    let err = ygocmg_core::infrastructure::standard_pack::rebuild_index(root.path(), "zh-CN")
        .unwrap_err();
    assert_eq!(err.code, "ygopro_cdb.schema_missing_columns");
}

#[test]
fn export_preflight_uses_standard_index_for_code_conflicts_and_reserved_warning() {
    let app = tempdir().unwrap();
    let root = tempdir().unwrap();
    create_test_cdb(
        &root.path().join("cards.cdb"),
        &[(12345678, "Indexed", 0x1 | 0x20)],
    )
    .unwrap();

    let index =
        ygocmg_core::infrastructure::standard_pack::rebuild_index(root.path(), "zh-CN").unwrap();
    ygocmg_core::infrastructure::standard_pack::save_index(app.path(), &index).unwrap();

    let workspace = tempdir().unwrap();
    let state = AppState::new(app.path().to_path_buf()).unwrap();
    let workspace_meta =
        app_commands::create_workspace(&state, workspace.path().to_path_buf(), "ws", None).unwrap();
    app_commands::open_workspace(&state, workspace.path().to_path_buf()).unwrap();

    let conflict_pack = app_commands::create_pack(
        &state,
        "conflict",
        "me",
        "1",
        None,
        vec!["zh-CN".to_string()],
        None,
    )
    .unwrap();
    let conflict_pack = app_commands::open_pack(&state, &conflict_pack.id).unwrap();
    let warning_pack = app_commands::create_pack(
        &state,
        "warning",
        "me",
        "1",
        None,
        vec!["zh-CN".to_string()],
        None,
    )
    .unwrap();
    let warning_pack = app_commands::open_pack(&state, &warning_pack.id).unwrap();

    let inventory =
        ygocmg_core::infrastructure::pack_locator::load_workspace_pack_inventory(workspace.path())
            .unwrap();
    let conflict_path =
        ygocmg_core::infrastructure::pack_locator::resolve_pack_path(&inventory, &conflict_pack.id)
            .unwrap();
    let warning_path =
        ygocmg_core::infrastructure::pack_locator::resolve_pack_path(&inventory, &warning_pack.id)
            .unwrap();
    save_direct_cards(&conflict_path, vec![test_card_entity(12345678, "Conflict")]);
    save_direct_cards(
        &warning_path,
        vec![test_card_entity(12_345_679, "Reserved")],
    );

    app_commands::close_pack(&state, &conflict_pack.id).unwrap();
    app_commands::close_pack(&state, &warning_pack.id).unwrap();
    app_commands::open_pack(&state, &conflict_pack.id).unwrap();
    app_commands::open_pack(&state, &warning_pack.id).unwrap();

    let preview = app_commands::preview_export_bundle(
        &state,
        PreviewExportBundleInput {
            workspace_id: workspace_meta.id,
            pack_ids: vec![conflict_pack.id, warning_pack.id],
            export_language: "zh-CN".to_string(),
            output_dir: workspace.path().join("out"),
            output_name: "bundle".to_string(),
        },
    )
    .unwrap()
    .data;

    assert!(preview.issues.iter().any(|issue| {
        issue.code == "export.code_conflicts_with_standard_pack"
            && matches!(issue.level, IssueLevel::Error)
    }));
    assert!(preview.issues.iter().any(|issue| {
        issue.code == "export.code_in_standard_reserved_range"
            && matches!(issue.level, IssueLevel::Warning)
    }));
}

fn create_test_cdb(path: &Path, rows: &[(u32, &str, u64)]) -> rusqlite::Result<()> {
    create_test_cdb_with_category(
        path,
        &rows
            .iter()
            .map(|(code, name, card_type)| (*code, *name, *card_type, 0))
            .collect::<Vec<_>>(),
    )
}

fn create_test_cdb_with_category(
    path: &Path,
    rows: &[(u32, &str, u64, i64)],
) -> rusqlite::Result<()> {
    let connection = Connection::open(path)?;
    connection.execute_batch(
        "create table datas(id integer primary key, ot integer, alias integer, setcode integer, type integer, atk integer, def integer, level integer, race integer, attribute integer, category integer);
         create table texts(id integer primary key, name text, desc text, str1 text, str2 text, str3 text, str4 text, str5 text, str6 text, str7 text, str8 text, str9 text, str10 text, str11 text, str12 text, str13 text, str14 text, str15 text, str16 text);",
    )?;
    for (code, name, card_type, category) in rows {
        connection.execute(
            "insert into datas(id, ot, alias, setcode, type, atk, def, level, race, attribute, category) values (?1, 1, 0, 0, ?2, 1000, 1000, 4, 8192, 16, ?3)",
            rusqlite::params![code, card_type, category],
        )?;
        connection.execute(
            "insert into texts(id, name, desc, str1, str2, str3, str4, str5, str6, str7, str8, str9, str10, str11, str12, str13, str14, str15, str16) values (?1, ?2, 'desc', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '', '')",
            rusqlite::params![code, name],
        )?;
    }
    Ok(())
}

fn test_monster_input(code: u32, name: &str) -> CardUpdateInput {
    CardUpdateInput {
        code,
        alias: 0,
        setcodes: vec![],
        ot: Ot::Custom,
        category: 0,
        primary_type: PrimaryType::Monster,
        texts: std::collections::BTreeMap::from([(
            "zh-CN".to_string(),
            CardTexts {
                name: name.to_string(),
                desc: "desc".to_string(),
                strings: Vec::new(),
            },
        )]),
        monster_flags: Some(vec![MonsterFlag::Effect]),
        atk: Some(1000),
        def: Some(1000),
        race: Some(Race::Warrior),
        attribute: Some(Attribute::Light),
        level: Some(4),
        pendulum: None,
        link: None,
        spell_subtype: None,
        trap_subtype: None,
    }
}

fn test_card_entity(code: u32, name: &str) -> ygocmg_core::domain::card::model::CardEntity {
    ygocmg_core::domain::card::normalize::create_card_entity(
        format!("card-{code}"),
        test_monster_input(code, name),
        ygocmg_core::domain::common::time::now_utc(),
    )
}

fn save_direct_cards(pack_path: &Path, cards: Vec<ygocmg_core::domain::card::model::CardEntity>) {
    let cards_file = ygocmg_core::domain::card::model::CardsFile {
        schema_version: ygocmg_core::infrastructure::json_store::SCHEMA_VERSION,
        cards,
    };
    let contents = serde_json::to_vec_pretty(&cards_file).unwrap();
    fs::write(
        ygocmg_core::infrastructure::json_store::cards_path(pack_path),
        contents,
    )
    .unwrap();
}

fn wait_for_status(
    state: &AppState,
    job_id: &str,
    expected: JobStatusDto,
) -> ygocmg_core::application::dto::job::JobSnapshotDto {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let snapshot = app_commands::get_job_status(
            state,
            GetJobStatusInput {
                job_id: job_id.to_string(),
            },
        )
        .unwrap();
        if snapshot.status == expected {
            return snapshot;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for job status"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
