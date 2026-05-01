use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags, Transaction, params};
use serde::Serialize;
use uuid::Uuid;

use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::ids::LanguageCode;
use crate::domain::common::time::AppTimestamp;
use crate::domain::strings::model::PackStringKind;

use super::{StandardPackIndexFile, StandardPackSourceSnapshot, standard_pack_dir};

pub const STANDARD_SQLITE_SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Clone)]
pub struct StandardPackSqliteManifest {
    pub source: StandardPackSourceSnapshot,
    pub source_language: LanguageCode,
    pub indexed_at: AppTimestamp,
    pub card_count: usize,
    pub string_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandardSetnameRecord {
    pub key: u32,
    pub value: String,
}

const CREATE_SCHEMA_SQL: &str = r#"
create table standard_manifest (
  id integer primary key check (id = 1),
  schema_version integer not null,
  source_language text not null,
  indexed_at text not null,
  ygopro_path text not null,
  cdb_path text not null,
  cdb_modified integer,
  cdb_len integer not null,
  strings_modified integer,
  strings_len integer,
  card_count integer not null,
  string_count integer not null
);

create table standard_cards (
  code integer primary key,
  alias integer not null,
  ot text not null,
  category integer not null,
  primary_type text not null,
  subtype_display text not null,
  race text,
  attribute text,
  spell_subtype text,
  trap_subtype text,
  atk integer,
  def integer,
  level integer,
  raw_type integer not null,
  raw_race integer not null,
  raw_attribute integer not null,
  raw_level integer not null,
  detail_json text not null
);

create index idx_standard_cards_primary_type
  on standard_cards(primary_type, subtype_display, code);
create index idx_standard_cards_subtype
  on standard_cards(subtype_display, code);
create index idx_standard_cards_alias
  on standard_cards(alias, code);
create index idx_standard_cards_ot
  on standard_cards(ot, code);
create index idx_standard_cards_race
  on standard_cards(race, code);
create index idx_standard_cards_attribute
  on standard_cards(attribute, code);
create index idx_standard_cards_race_attribute
  on standard_cards(race, attribute, code);
create index idx_standard_cards_stats
  on standard_cards(atk, def, level, code);

create table standard_card_monster_flags (
  code integer not null,
  flag text not null,
  primary key (code, flag)
);

create index idx_standard_card_monster_flags_flag
  on standard_card_monster_flags(flag, code);

create table standard_card_setcodes (
  code integer not null,
  setcode integer not null,
  base integer not null,
  primary key (code, setcode)
);

create index idx_standard_card_setcodes_setcode
  on standard_card_setcodes(setcode, code);
create index idx_standard_card_setcodes_base
  on standard_card_setcodes(base, code);

create table standard_card_pendulum (
  code integer primary key,
  left_scale integer not null,
  right_scale integer not null
);

create index idx_standard_card_pendulum_left
  on standard_card_pendulum(left_scale, code);
create index idx_standard_card_pendulum_right
  on standard_card_pendulum(right_scale, code);

create table standard_card_link_markers (
  code integer not null,
  marker text not null,
  primary key (code, marker)
);

create index idx_standard_card_link_markers_marker
  on standard_card_link_markers(marker, code);

create table standard_card_texts (
  code integer not null,
  language text not null,
  name text not null,
  desc text not null,
  strings_json text not null,
  primary key (code, language)
);

create index idx_standard_card_texts_language_name
  on standard_card_texts(language, name, code);

create table standard_card_list_rows (
  code integer not null,
  language text not null,
  name text not null,
  desc text not null,
  primary_type text not null,
  subtype_display text not null,
  atk integer,
  def integer,
  level integer,
  has_image integer not null,
  has_script integer not null,
  has_field_image integer not null,
  primary key (code, language)
);

create index idx_standard_card_rows_code
  on standard_card_list_rows(code);
create index idx_standard_card_rows_name
  on standard_card_list_rows(name, code);
create index idx_standard_card_rows_type
  on standard_card_list_rows(primary_type, subtype_display, code);

create virtual table standard_card_search_fts using fts5(
  code unindexed,
  language unindexed,
  name,
  card_desc,
  primary_type,
  subtype_display
);

create table standard_assets (
  code integer primary key,
  has_image integer not null,
  has_script integer not null,
  has_field_image integer not null
);

create table standard_strings (
  kind text not null,
  key integer not null,
  language text not null,
  value text not null,
  primary key (kind, key, language)
);

create index idx_standard_strings_kind_key
  on standard_strings(kind, key);
create index idx_standard_strings_kind_value
  on standard_strings(kind, value, key);

create table standard_code_baseline (
  code integer primary key
);

create table standard_string_baseline (
  kind text not null,
  key integer not null,
  primary key (kind, key)
);

create table standard_setname_base_baseline (
  base integer primary key
);
"#;

pub fn standard_pack_sqlite_path(app_data_dir: &Path) -> PathBuf {
    standard_pack_dir(app_data_dir).join("index.sqlite")
}

pub fn open_readonly(app_data_dir: &Path) -> AppResult<Connection> {
    let path = standard_pack_sqlite_path(app_data_dir);
    if !path.exists() {
        return Err(AppError::new(
            "standard_pack.sqlite_missing",
            "standard pack sqlite index is missing",
        )
        .with_detail("path", path.display().to_string()));
    }
    Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|source| {
        AppError::new("standard_pack.sqlite_open_failed", source.to_string())
            .with_detail("path", path.display().to_string())
    })
}

pub fn load_sqlite_manifest(connection: &Connection) -> AppResult<StandardPackSqliteManifest> {
    let row = connection
        .query_row(
            "select schema_version, source_language, indexed_at, ygopro_path, cdb_path,
                    cdb_modified, cdb_len, strings_modified, strings_len, card_count, string_count
             from standard_manifest where id = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                    row.get::<_, Option<i64>>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                ))
            },
        )
        .map_err(map_manifest_read_error)?;

    let (
        schema_version,
        source_language,
        indexed_at,
        ygopro_path,
        cdb_path,
        cdb_modified,
        cdb_len,
        strings_modified,
        strings_len,
        card_count,
        string_count,
    ) = row;

    if schema_version != STANDARD_SQLITE_SCHEMA_VERSION as i64 {
        return Err(AppError::new(
            "standard_pack.sqlite_schema_mismatch",
            "standard pack sqlite schema mismatch",
        )
        .with_detail("expected", STANDARD_SQLITE_SCHEMA_VERSION)
        .with_detail("actual", schema_version));
    }

    Ok(StandardPackSqliteManifest {
        source: StandardPackSourceSnapshot {
            ygopro_path,
            cdb_path,
            cdb_modified,
            cdb_len: i64_to_u64("cdb_len", cdb_len)?,
            strings_modified,
            strings_len: optional_i64_to_u64("strings_len", strings_len)?,
        },
        source_language,
        indexed_at: parse_timestamp("indexed_at", &indexed_at)?,
        card_count: i64_to_usize("card_count", card_count)?,
        string_count: i64_to_usize("string_count", string_count)?,
    })
}

pub fn load_sqlite_manifest_from_app_data(
    app_data_dir: &Path,
) -> AppResult<StandardPackSqliteManifest> {
    let connection = open_readonly(app_data_dir)?;
    load_sqlite_manifest(&connection)
}

pub fn save_sqlite_index(app_data_dir: &Path, index: &StandardPackIndexFile) -> AppResult<()> {
    let index_dir = standard_pack_dir(app_data_dir);
    fs::create_dir_all(&index_dir).map_err(|source| {
        AppError::from_io("standard_pack.sqlite_dir_create_failed", source)
            .with_detail("path", index_dir.display().to_string())
    })?;

    let target_path = standard_pack_sqlite_path(app_data_dir);
    let temp_path = index_dir.join(format!("index.sqlite.tmp-{}", Uuid::now_v7()));

    let result = write_temp_sqlite_index(&temp_path, index)
        .and_then(|_| replace_sqlite_index(&temp_path, &target_path));

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }

    result
}

fn write_temp_sqlite_index(path: &Path, index: &StandardPackIndexFile) -> AppResult<()> {
    let mut connection = Connection::open(path).map_err(|source| {
        AppError::new("standard_pack.sqlite_create_failed", source.to_string())
            .with_detail("path", path.display().to_string())
    })?;
    connection
        .execute_batch(CREATE_SCHEMA_SQL)
        .map_err(|source| {
            AppError::new("standard_pack.sqlite_schema_failed", source.to_string())
        })?;

    let transaction = connection.transaction().map_err(|source| {
        AppError::new(
            "standard_pack.sqlite_transaction_failed",
            source.to_string(),
        )
    })?;
    insert_index(&transaction, index)?;
    transaction.commit().map_err(|source| {
        AppError::new("standard_pack.sqlite_commit_failed", source.to_string())
    })?;

    validate_index(&connection, index)?;
    Ok(())
}

fn insert_index(transaction: &Transaction<'_>, index: &StandardPackIndexFile) -> AppResult<()> {
    insert_manifest(transaction, index)?;
    insert_cards(transaction, index)?;
    insert_strings(transaction, index)?;
    insert_baselines(transaction, index)
}

fn insert_manifest(transaction: &Transaction<'_>, index: &StandardPackIndexFile) -> AppResult<()> {
    transaction
        .execute(
            "insert into standard_manifest(
                id, schema_version, source_language, indexed_at, ygopro_path, cdb_path,
                cdb_modified, cdb_len, strings_modified, strings_len, card_count, string_count
             ) values (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                STANDARD_SQLITE_SCHEMA_VERSION as i64,
                index.source_language.as_str(),
                index.indexed_at.to_rfc3339(),
                index.source.ygopro_path.as_str(),
                index.source.cdb_path.as_str(),
                index.source.cdb_modified,
                u64_to_i64("cdb_len", index.source.cdb_len)?,
                index.source.strings_modified,
                optional_u64_to_i64("strings_len", index.source.strings_len)?,
                usize_to_i64("card_count", index.cards.len())?,
                usize_to_i64("string_count", index.strings.records.len())?,
            ],
        )
        .map_err(|source| {
            AppError::new(
                "standard_pack.sqlite_insert_manifest_failed",
                source.to_string(),
            )
        })?;
    Ok(())
}

fn insert_cards(transaction: &Transaction<'_>, index: &StandardPackIndexFile) -> AppResult<()> {
    let mut card_statement = transaction
        .prepare(
            "insert into standard_cards(
                code, alias, ot, category, primary_type, subtype_display,
                race, attribute, spell_subtype, trap_subtype, atk, def, level,
                raw_type, raw_race, raw_attribute, raw_level, detail_json
             ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
        )
        .map_err(|source| {
            AppError::new(
                "standard_pack.sqlite_prepare_cards_failed",
                source.to_string(),
            )
        })?;
    let mut text_statement = transaction
        .prepare(
            "insert into standard_card_texts(code, language, name, desc, strings_json)
             values (?1, ?2, ?3, ?4, ?5)",
        )
        .map_err(|source| {
            AppError::new(
                "standard_pack.sqlite_prepare_card_texts_failed",
                source.to_string(),
            )
        })?;
    let mut list_statement = transaction
        .prepare(
            "insert into standard_card_list_rows(
                code, language, name, desc, primary_type, subtype_display, atk, def, level,
                has_image, has_script, has_field_image
             ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        )
        .map_err(|source| {
            AppError::new(
                "standard_pack.sqlite_prepare_card_rows_failed",
                source.to_string(),
            )
        })?;
    let mut fts_statement = transaction
        .prepare(
            "insert into standard_card_search_fts(
                code, language, name, card_desc, primary_type, subtype_display
             ) values (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .map_err(|source| {
            AppError::new(
                "standard_pack.sqlite_prepare_card_search_fts_failed",
                source.to_string(),
            )
        })?;
    let mut asset_statement = transaction
        .prepare(
            "insert into standard_assets(code, has_image, has_script, has_field_image)
             values (?1, ?2, ?3, ?4)",
        )
        .map_err(|source| {
            AppError::new(
                "standard_pack.sqlite_prepare_assets_failed",
                source.to_string(),
            )
        })?;
    let mut monster_flag_statement = transaction
        .prepare(
            "insert into standard_card_monster_flags(code, flag)
             values (?1, ?2)",
        )
        .map_err(|source| {
            AppError::new(
                "standard_pack.sqlite_prepare_monster_flags_failed",
                source.to_string(),
            )
        })?;
    let mut setcode_statement = transaction
        .prepare(
            "insert into standard_card_setcodes(code, setcode, base)
             values (?1, ?2, ?3)",
        )
        .map_err(|source| {
            AppError::new(
                "standard_pack.sqlite_prepare_setcodes_failed",
                source.to_string(),
            )
        })?;
    let mut pendulum_statement = transaction
        .prepare(
            "insert into standard_card_pendulum(code, left_scale, right_scale)
             values (?1, ?2, ?3)",
        )
        .map_err(|source| {
            AppError::new(
                "standard_pack.sqlite_prepare_pendulum_failed",
                source.to_string(),
            )
        })?;
    let mut link_marker_statement = transaction
        .prepare(
            "insert into standard_card_link_markers(code, marker)
             values (?1, ?2)",
        )
        .map_err(|source| {
            AppError::new(
                "standard_pack.sqlite_prepare_link_markers_failed",
                source.to_string(),
            )
        })?;

    for record in &index.cards {
        let detail_json = serialize_json("detail_json", &record.card)?;
        card_statement
            .execute(params![
                record.card.code as i64,
                record.card.alias as i64,
                serialize_enum_text("ot", &record.card.ot)?,
                u64_to_i64("category", record.card.category)?,
                serialize_enum_text("primary_type", &record.card.primary_type)?,
                record.row.subtype_display.as_str(),
                serialize_optional_enum_text("race", record.card.race.as_ref())?,
                serialize_optional_enum_text("attribute", record.card.attribute.as_ref())?,
                serialize_optional_enum_text("spell_subtype", record.card.spell_subtype.as_ref())?,
                serialize_optional_enum_text("trap_subtype", record.card.trap_subtype.as_ref())?,
                record.card.atk,
                record.card.def,
                record.card.level,
                u64_to_i64("raw_type", record.raw_type)?,
                u64_to_i64("raw_race", record.raw_race)?,
                u64_to_i64("raw_attribute", record.raw_attribute)?,
                u64_to_i64("raw_level", record.raw_level)?,
                detail_json,
            ])
            .map_err(|source| {
                AppError::new(
                    "standard_pack.sqlite_insert_card_failed",
                    source.to_string(),
                )
                .with_detail("code", record.card.code)
            })?;

        for (language, texts) in &record.card.texts {
            text_statement
                .execute(params![
                    record.card.code as i64,
                    language.as_str(),
                    texts.name.as_str(),
                    texts.desc.as_str(),
                    serialize_json("strings_json", &texts.strings)?,
                ])
                .map_err(|source| {
                    AppError::new(
                        "standard_pack.sqlite_insert_card_text_failed",
                        source.to_string(),
                    )
                    .with_detail("code", record.card.code)
                    .with_detail("language", language)
                })?;
        }

        list_statement
            .execute(params![
                record.row.code as i64,
                index.source_language.as_str(),
                record.row.name.as_str(),
                record.row.desc.as_str(),
                serialize_enum_text("row_primary_type", &record.row.primary_type)?,
                record.row.subtype_display.as_str(),
                record.row.atk,
                record.row.def,
                record.row.level,
                bool_int(record.row.has_image),
                bool_int(record.row.has_script),
                bool_int(record.row.has_field_image),
            ])
            .map_err(|source| {
                AppError::new(
                    "standard_pack.sqlite_insert_card_row_failed",
                    source.to_string(),
                )
                .with_detail("code", record.card.code)
            })?;

        fts_statement
            .execute(params![
                record.row.code.to_string(),
                index.source_language.as_str(),
                record.row.name.as_str(),
                record.row.desc.as_str(),
                serialize_enum_text("row_primary_type", &record.row.primary_type)?,
                record.row.subtype_display.as_str(),
            ])
            .map_err(|source| {
                AppError::new(
                    "standard_pack.sqlite_insert_card_search_fts_failed",
                    source.to_string(),
                )
                .with_detail("code", record.card.code)
            })?;

        asset_statement
            .execute(params![
                record.card.code as i64,
                bool_int(record.asset_state.has_image),
                bool_int(record.asset_state.has_script),
                bool_int(record.asset_state.has_field_image),
            ])
            .map_err(|source| {
                AppError::new(
                    "standard_pack.sqlite_insert_asset_failed",
                    source.to_string(),
                )
                .with_detail("code", record.card.code)
            })?;

        for flag in unique_serialized_values(
            "monster_flag",
            record.card.monster_flags.as_deref().unwrap_or_default(),
        )? {
            monster_flag_statement
                .execute(params![record.card.code as i64, flag.as_str()])
                .map_err(|source| {
                    AppError::new(
                        "standard_pack.sqlite_insert_monster_flag_failed",
                        source.to_string(),
                    )
                    .with_detail("code", record.card.code)
                    .with_detail("flag", flag)
                })?;
        }

        for setcode in unique_nonzero_setcodes(&record.card.setcodes) {
            setcode_statement
                .execute(params![
                    record.card.code as i64,
                    setcode as i64,
                    (setcode & 0x0fff) as i64,
                ])
                .map_err(|source| {
                    AppError::new(
                        "standard_pack.sqlite_insert_setcode_failed",
                        source.to_string(),
                    )
                    .with_detail("code", record.card.code)
                    .with_detail("setcode", setcode)
                })?;
        }

        if let Some(pendulum) = &record.card.pendulum {
            pendulum_statement
                .execute(params![
                    record.card.code as i64,
                    pendulum.left_scale as i64,
                    pendulum.right_scale as i64,
                ])
                .map_err(|source| {
                    AppError::new(
                        "standard_pack.sqlite_insert_pendulum_failed",
                        source.to_string(),
                    )
                    .with_detail("code", record.card.code)
                })?;
        }

        if let Some(link) = &record.card.link {
            for marker in unique_serialized_values("link_marker", &link.markers)? {
                link_marker_statement
                    .execute(params![record.card.code as i64, marker.as_str()])
                    .map_err(|source| {
                        AppError::new(
                            "standard_pack.sqlite_insert_link_marker_failed",
                            source.to_string(),
                        )
                        .with_detail("code", record.card.code)
                        .with_detail("marker", marker)
                    })?;
            }
        }
    }

    Ok(())
}

fn insert_strings(transaction: &Transaction<'_>, index: &StandardPackIndexFile) -> AppResult<()> {
    let mut statement = transaction
        .prepare(
            "insert into standard_strings(kind, key, language, value)
             values (?1, ?2, ?3, ?4)",
        )
        .map_err(|source| {
            AppError::new(
                "standard_pack.sqlite_prepare_strings_failed",
                source.to_string(),
            )
        })?;

    for record in &index.strings.records {
        let kind = serialize_enum_text("string_kind", &record.kind)?;
        for (language, value) in &record.values {
            statement
                .execute(params![
                    kind.as_str(),
                    record.key as i64,
                    language.as_str(),
                    value.as_str(),
                ])
                .map_err(|source| {
                    AppError::new(
                        "standard_pack.sqlite_insert_string_failed",
                        source.to_string(),
                    )
                    .with_detail("kind", &kind)
                    .with_detail("key", record.key)
                    .with_detail("language", language)
                })?;
        }
    }

    Ok(())
}

fn insert_baselines(transaction: &Transaction<'_>, index: &StandardPackIndexFile) -> AppResult<()> {
    let mut code_statement = transaction
        .prepare("insert into standard_code_baseline(code) values (?1)")
        .map_err(|source| {
            AppError::new(
                "standard_pack.sqlite_prepare_code_baseline_failed",
                source.to_string(),
            )
        })?;
    for record in &index.cards {
        code_statement
            .execute(params![record.card.code as i64])
            .map_err(|source| {
                AppError::new(
                    "standard_pack.sqlite_insert_code_baseline_failed",
                    source.to_string(),
                )
                .with_detail("code", record.card.code)
            })?;
    }
    drop(code_statement);

    let mut string_statement = transaction
        .prepare("insert into standard_string_baseline(kind, key) values (?1, ?2)")
        .map_err(|source| {
            AppError::new(
                "standard_pack.sqlite_prepare_string_baseline_failed",
                source.to_string(),
            )
        })?;
    insert_string_baseline_keys(
        &mut string_statement,
        PackStringKind::System,
        &index.strings.baseline.system_keys,
    )?;
    insert_string_baseline_keys(
        &mut string_statement,
        PackStringKind::Victory,
        &index.strings.baseline.victory_keys,
    )?;
    insert_string_baseline_keys(
        &mut string_statement,
        PackStringKind::Counter,
        &index.strings.baseline.counter_keys,
    )?;
    insert_string_baseline_keys(
        &mut string_statement,
        PackStringKind::Setname,
        &index.strings.baseline.setname_keys,
    )?;
    drop(string_statement);

    let mut setname_base_statement = transaction
        .prepare("insert into standard_setname_base_baseline(base) values (?1)")
        .map_err(|source| {
            AppError::new(
                "standard_pack.sqlite_prepare_setname_base_baseline_failed",
                source.to_string(),
            )
        })?;
    for base in &index.strings.baseline.setname_bases {
        setname_base_statement
            .execute(params![*base as i64])
            .map_err(|source| {
                AppError::new(
                    "standard_pack.sqlite_insert_setname_base_baseline_failed",
                    source.to_string(),
                )
                .with_detail("base", base)
            })?;
    }

    Ok(())
}

fn insert_string_baseline_keys(
    statement: &mut rusqlite::Statement<'_>,
    kind: PackStringKind,
    keys: &std::collections::BTreeSet<u32>,
) -> AppResult<()> {
    let kind_text = serialize_enum_text("baseline_kind", &kind)?;
    for key in keys {
        statement
            .execute(params![kind_text.as_str(), *key as i64])
            .map_err(|source| {
                AppError::new(
                    "standard_pack.sqlite_insert_string_baseline_failed",
                    source.to_string(),
                )
                .with_detail("kind", &kind_text)
                .with_detail("key", key)
            })?;
    }
    Ok(())
}

fn validate_index(connection: &Connection, index: &StandardPackIndexFile) -> AppResult<()> {
    assert_count(connection, "standard_manifest", 1)?;
    assert_count(connection, "standard_cards", index.cards.len())?;
    assert_count(connection, "standard_card_texts", card_text_count(index))?;
    assert_count(connection, "standard_card_list_rows", index.cards.len())?;
    assert_count(connection, "standard_card_search_fts", index.cards.len())?;
    assert_count(connection, "standard_assets", index.cards.len())?;
    assert_count(
        connection,
        "standard_card_monster_flags",
        monster_flag_count(index)?,
    )?;
    assert_count(connection, "standard_card_setcodes", setcode_count(index))?;
    assert_count(connection, "standard_card_pendulum", pendulum_count(index))?;
    assert_count(
        connection,
        "standard_card_link_markers",
        link_marker_count(index)?,
    )?;
    assert_count(connection, "standard_strings", string_value_count(index))?;
    assert_count(connection, "standard_code_baseline", index.cards.len())?;
    assert_count(
        connection,
        "standard_string_baseline",
        string_baseline_count(index),
    )?;
    assert_count(
        connection,
        "standard_setname_base_baseline",
        index.strings.baseline.setname_bases.len(),
    )
}

fn assert_count(connection: &Connection, table: &str, expected: usize) -> AppResult<()> {
    let actual = connection
        .query_row(&format!("select count(*) from {table}"), [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|source| {
            AppError::new(
                "standard_pack.sqlite_validate_count_failed",
                source.to_string(),
            )
            .with_detail("table", table)
        })?;
    let expected = usize_to_i64("expected_count", expected)?;
    if actual != expected {
        return Err(AppError::new(
            "standard_pack.sqlite_validate_count_mismatch",
            "standard pack sqlite row count mismatch",
        )
        .with_detail("table", table)
        .with_detail("expected", expected)
        .with_detail("actual", actual));
    }
    Ok(())
}

fn replace_sqlite_index(temp_path: &Path, target_path: &Path) -> AppResult<()> {
    let backup_path = target_path.with_file_name(format!("index.sqlite.bak-{}", Uuid::now_v7()));
    let target_exists = target_path.exists();

    if target_exists {
        fs::rename(target_path, &backup_path).map_err(|source| {
            AppError::from_io("standard_pack.sqlite_backup_rename_failed", source)
                .with_detail("path", target_path.display().to_string())
                .with_detail("backup_path", backup_path.display().to_string())
        })?;
    }

    if let Err(source) = fs::rename(temp_path, target_path) {
        if target_exists && backup_path.exists() {
            let _ = fs::rename(&backup_path, target_path);
        }
        return Err(
            AppError::from_io("standard_pack.sqlite_commit_rename_failed", source)
                .with_detail("path", target_path.display().to_string())
                .with_detail("temp_path", temp_path.display().to_string()),
        );
    }

    if target_exists {
        fs::remove_file(&backup_path).map_err(|source| {
            AppError::from_io("standard_pack.sqlite_backup_cleanup_failed", source)
                .with_detail("path", backup_path.display().to_string())
        })?;
    }

    Ok(())
}

fn card_text_count(index: &StandardPackIndexFile) -> usize {
    index
        .cards
        .iter()
        .map(|record| record.card.texts.len())
        .sum()
}

fn monster_flag_count(index: &StandardPackIndexFile) -> AppResult<usize> {
    index
        .cards
        .iter()
        .map(|record| {
            unique_serialized_values(
                "monster_flag",
                record.card.monster_flags.as_deref().unwrap_or_default(),
            )
            .map(|values| values.len())
        })
        .sum()
}

fn setcode_count(index: &StandardPackIndexFile) -> usize {
    index
        .cards
        .iter()
        .map(|record| unique_nonzero_setcodes(&record.card.setcodes).len())
        .sum()
}

fn pendulum_count(index: &StandardPackIndexFile) -> usize {
    index
        .cards
        .iter()
        .filter(|record| record.card.pendulum.is_some())
        .count()
}

fn link_marker_count(index: &StandardPackIndexFile) -> AppResult<usize> {
    index
        .cards
        .iter()
        .map(|record| {
            record
                .card
                .link
                .as_ref()
                .map(|link| unique_serialized_values("link_marker", &link.markers))
                .transpose()
                .map(|values| values.map(|values| values.len()).unwrap_or_default())
        })
        .sum()
}

fn unique_nonzero_setcodes(values: &[u16]) -> Vec<u16> {
    values
        .iter()
        .copied()
        .filter(|value| *value != 0)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn string_value_count(index: &StandardPackIndexFile) -> usize {
    index
        .strings
        .records
        .iter()
        .map(|record| record.values.len())
        .sum()
}

fn string_baseline_count(index: &StandardPackIndexFile) -> usize {
    index.strings.baseline.system_keys.len()
        + index.strings.baseline.victory_keys.len()
        + index.strings.baseline.counter_keys.len()
        + index.strings.baseline.setname_keys.len()
}

fn serialize_json<T: Serialize>(field: &str, value: &T) -> AppResult<String> {
    serde_json::to_string(value).map_err(|source| {
        AppError::new("standard_pack.sqlite_serialize_failed", source.to_string())
            .with_detail("field", field)
    })
}

fn serialize_optional_enum_text<T: Serialize>(
    field: &str,
    value: Option<&T>,
) -> AppResult<Option<String>> {
    value
        .map(|value| serialize_enum_text(field, value))
        .transpose()
}

fn serialize_enum_text<T: Serialize>(field: &str, value: &T) -> AppResult<String> {
    match serde_json::to_value(value).map_err(|source| {
        AppError::new("standard_pack.sqlite_serialize_failed", source.to_string())
            .with_detail("field", field)
    })? {
        serde_json::Value::String(value) => Ok(value),
        other => Err(AppError::new(
            "standard_pack.sqlite_serialize_enum_failed",
            "serialized enum did not produce a string",
        )
        .with_detail("field", field)
        .with_detail("value", other)),
    }
}

fn unique_serialized_values<T: Serialize>(field: &str, values: &[T]) -> AppResult<Vec<String>> {
    values
        .iter()
        .map(|value| serialize_enum_text(field, value))
        .collect::<AppResult<BTreeSet<_>>>()
        .map(|values| values.into_iter().collect())
}

fn bool_int(value: bool) -> i64 {
    if value { 1 } else { 0 }
}

fn usize_to_i64(field: &str, value: usize) -> AppResult<i64> {
    i64::try_from(value).map_err(|_| {
        AppError::new(
            "standard_pack.sqlite_integer_out_of_range",
            "standard pack sqlite integer is out of range",
        )
        .with_detail("field", field)
        .with_detail("value", value)
    })
}

fn u64_to_i64(field: &str, value: u64) -> AppResult<i64> {
    i64::try_from(value).map_err(|_| {
        AppError::new(
            "standard_pack.sqlite_integer_out_of_range",
            "standard pack sqlite integer is out of range",
        )
        .with_detail("field", field)
        .with_detail("value", value)
    })
}

fn optional_u64_to_i64(field: &str, value: Option<u64>) -> AppResult<Option<i64>> {
    value.map(|value| u64_to_i64(field, value)).transpose()
}

fn i64_to_usize(field: &str, value: i64) -> AppResult<usize> {
    usize::try_from(value).map_err(|_| {
        AppError::new(
            "standard_pack.sqlite_integer_out_of_range",
            "standard pack sqlite integer is out of range",
        )
        .with_detail("field", field)
        .with_detail("value", value)
    })
}

fn i64_to_u64(field: &str, value: i64) -> AppResult<u64> {
    u64::try_from(value).map_err(|_| {
        AppError::new(
            "standard_pack.sqlite_integer_out_of_range",
            "standard pack sqlite integer is out of range",
        )
        .with_detail("field", field)
        .with_detail("value", value)
    })
}

fn optional_i64_to_u64(field: &str, value: Option<i64>) -> AppResult<Option<u64>> {
    value.map(|value| i64_to_u64(field, value)).transpose()
}

fn parse_timestamp(field: &str, value: &str) -> AppResult<AppTimestamp> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|source| {
            AppError::new(
                "standard_pack.sqlite_timestamp_parse_failed",
                source.to_string(),
            )
            .with_detail("field", field)
            .with_detail("value", value)
        })
}

fn map_manifest_read_error(source: rusqlite::Error) -> AppError {
    if matches!(source, rusqlite::Error::QueryReturnedNoRows) {
        return AppError::new(
            "standard_pack.sqlite_schema_mismatch",
            "standard pack sqlite manifest is missing",
        );
    }

    let message = source.to_string();
    if message.contains("no such table")
        || message.contains("no such column")
        || message.contains("standard_manifest")
    {
        return AppError::new(
            "standard_pack.sqlite_schema_mismatch",
            "standard pack sqlite schema mismatch",
        )
        .with_detail("source", message);
    }

    AppError::new("standard_pack.sqlite_manifest_read_failed", message)
}
