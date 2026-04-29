use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::{Connection, OpenFlags, Row, params};

use crate::domain::card::model::{
    Attribute, CardEntity, CardTexts, LinkData, LinkMarker, MonsterFlag, Ot, Pendulum, PrimaryType,
    Race, SpellSubtype, TrapSubtype,
};
use crate::domain::common::error::{AppError, AppResult};
use crate::domain::common::time::now_utc;

const TYPE_MONSTER: u64 = 0x1;
const TYPE_SPELL: u64 = 0x2;
const TYPE_TRAP: u64 = 0x4;
const TYPE_NORMAL: u64 = 0x10;
const TYPE_EFFECT: u64 = 0x20;
const TYPE_FUSION: u64 = 0x40;
const TYPE_RITUAL: u64 = 0x80;
const TYPE_SPIRIT: u64 = 0x200;
const TYPE_UNION: u64 = 0x400;
const TYPE_DUAL: u64 = 0x800;
const TYPE_TUNER: u64 = 0x1000;
const TYPE_SYNCHRO: u64 = 0x2000;
const TYPE_TOKEN: u64 = 0x4000;
const TYPE_QUICKPLAY: u64 = 0x10000;
const TYPE_CONTINUOUS: u64 = 0x20000;
const TYPE_EQUIP: u64 = 0x40000;
const TYPE_FIELD: u64 = 0x80000;
const TYPE_COUNTER: u64 = 0x100000;
const TYPE_FLIP: u64 = 0x200000;
const TYPE_TOON: u64 = 0x400000;
const TYPE_XYZ: u64 = 0x800000;
const TYPE_PENDULUM: u64 = 0x1000000;
const TYPE_LINK: u64 = 0x4000000;

const LINK_MARKER_BOTTOM_LEFT: i32 = 0x001;
const LINK_MARKER_BOTTOM: i32 = 0x002;
const LINK_MARKER_BOTTOM_RIGHT: i32 = 0x004;
const LINK_MARKER_LEFT: i32 = 0x008;
const LINK_MARKER_RIGHT: i32 = 0x020;
const LINK_MARKER_TOP_LEFT: i32 = 0x040;
const LINK_MARKER_TOP: i32 = 0x080;
const LINK_MARKER_TOP_RIGHT: i32 = 0x100;

#[derive(Debug, Clone)]
pub struct YgoProCardRecord {
    pub card: CardEntity,
    pub raw_type: u64,
    pub raw_race: u64,
    pub raw_attribute: u64,
    pub raw_level: u64,
}

pub fn load_cards_from_cdb(cdb_path: &Path) -> AppResult<Vec<YgoProCardRecord>> {
    let connection = Connection::open_with_flags(cdb_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|source| {
            AppError::new("ygopro_cdb.open_failed", source.to_string())
                .with_detail("path", cdb_path.display().to_string())
        })?;
    validate_schema(&connection)?;

    let mut statement = connection
        .prepare(
            "select d.id, d.ot, d.alias, d.setcode, d.type, d.atk, d.def, d.level, d.race, d.attribute, d.category, \
                    t.name, t.desc, t.str1, t.str2, t.str3, t.str4, t.str5, t.str6, t.str7, t.str8, \
                    t.str9, t.str10, t.str11, t.str12, t.str13, t.str14, t.str15, t.str16 \
             from datas d left join texts t on t.id = d.id",
        )
        .map_err(|source| AppError::new("ygopro_cdb.query_prepare_failed", source.to_string()))?;

    let mapped = statement
        .query_map([], decode_card_row)
        .map_err(|source| AppError::new("ygopro_cdb.query_failed", source.to_string()))?;

    let mut records = Vec::new();
    for item in mapped {
        records.push(
            item.map_err(|source| {
                AppError::new("ygopro_cdb.row_decode_failed", source.to_string())
            })?,
        );
    }
    Ok(records)
}

pub fn write_cards_to_cdb(
    cdb_path: &Path,
    cards: &[CardEntity],
    export_language: &str,
) -> AppResult<()> {
    if let Some(parent) = cdb_path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| {
            AppError::from_io("ygopro_cdb.create_parent_failed", source)
                .with_detail("path", parent.display().to_string())
        })?;
    }
    if cdb_path.exists() {
        std::fs::remove_file(cdb_path).map_err(|source| {
            AppError::from_io("ygopro_cdb.remove_existing_failed", source)
                .with_detail("path", cdb_path.display().to_string())
        })?;
    }

    let mut connection = Connection::open(cdb_path).map_err(|source| {
        AppError::new("ygopro_cdb.create_failed", source.to_string())
            .with_detail("path", cdb_path.display().to_string())
    })?;
    create_schema(&connection)?;
    let transaction = connection
        .transaction()
        .map_err(|source| AppError::new("ygopro_cdb.transaction_failed", source.to_string()))?;
    {
        let mut datas_statement = transaction
            .prepare(
                "insert into datas(id, ot, alias, setcode, type, atk, def, level, race, attribute, category) \
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )
            .map_err(|source| {
                AppError::new("ygopro_cdb.insert_prepare_failed", source.to_string())
            })?;
        let mut texts_statement = transaction
            .prepare(
                "insert into texts(id, name, desc, str1, str2, str3, str4, str5, str6, str7, str8, \
                 str9, str10, str11, str12, str13, str14, str15, str16) \
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            )
            .map_err(|source| {
                AppError::new("ygopro_cdb.insert_prepare_failed", source.to_string())
            })?;

        for card in cards {
            let encoded = encode_card(card)?;
            datas_statement
                .execute(params![
                    card.code as i64,
                    encode_ot(&card.ot) as i64,
                    card.alias as i64,
                    card.setcode as i64,
                    encoded.raw_type as i64,
                    encoded.atk as i64,
                    encoded.def as i64,
                    encoded.raw_level as i64,
                    encoded.raw_race as i64,
                    encoded.raw_attribute as i64,
                    card.category as i64,
                ])
                .map_err(|source| {
                    AppError::new("ygopro_cdb.insert_datas_failed", source.to_string())
                        .with_detail("code", card.code)
                })?;

            let texts = card.texts.get(export_language).ok_or_else(|| {
                AppError::new(
                    "ygopro_cdb.missing_export_language",
                    "card is missing export language text",
                )
                .with_detail("code", card.code)
                .with_detail("language", export_language)
            })?;
            let strings = padded_card_strings(&texts.strings);
            texts_statement
                .execute(params![
                    card.code as i64,
                    texts.name.as_str(),
                    texts.desc.as_str(),
                    strings[0].as_str(),
                    strings[1].as_str(),
                    strings[2].as_str(),
                    strings[3].as_str(),
                    strings[4].as_str(),
                    strings[5].as_str(),
                    strings[6].as_str(),
                    strings[7].as_str(),
                    strings[8].as_str(),
                    strings[9].as_str(),
                    strings[10].as_str(),
                    strings[11].as_str(),
                    strings[12].as_str(),
                    strings[13].as_str(),
                    strings[14].as_str(),
                    strings[15].as_str(),
                ])
                .map_err(|source| {
                    AppError::new("ygopro_cdb.insert_texts_failed", source.to_string())
                        .with_detail("code", card.code)
                })?;
        }
    }
    transaction
        .commit()
        .map_err(|source| AppError::new("ygopro_cdb.commit_failed", source.to_string()))?;
    Ok(())
}

fn create_schema(connection: &Connection) -> AppResult<()> {
    connection
        .execute_batch(
            "create table datas(id integer primary key, ot integer, alias integer, setcode integer, type integer, atk integer, def integer, level integer, race integer, attribute integer, category integer);
             create table texts(id integer primary key, name text, desc text, str1 text, str2 text, str3 text, str4 text, str5 text, str6 text, str7 text, str8 text, str9 text, str10 text, str11 text, str12 text, str13 text, str14 text, str15 text, str16 text);",
        )
        .map_err(|source| AppError::new("ygopro_cdb.schema_create_failed", source.to_string()))
}

fn validate_schema(connection: &Connection) -> AppResult<()> {
    ensure_columns(
        connection,
        "datas",
        &[
            "id",
            "ot",
            "alias",
            "setcode",
            "type",
            "atk",
            "def",
            "level",
            "race",
            "attribute",
            "category",
        ],
    )?;
    ensure_columns(
        connection,
        "texts",
        &[
            "id", "name", "desc", "str1", "str2", "str3", "str4", "str5", "str6", "str7", "str8",
            "str9", "str10", "str11", "str12", "str13", "str14", "str15", "str16",
        ],
    )
}

fn ensure_columns(connection: &Connection, table: &str, required: &[&str]) -> AppResult<()> {
    let mut statement = connection
        .prepare(&format!("pragma table_info({table})"))
        .map_err(|source| AppError::new("ygopro_cdb.schema_query_failed", source.to_string()))?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|source| AppError::new("ygopro_cdb.schema_query_failed", source.to_string()))?;
    let mut actual = std::collections::BTreeSet::new();
    for column in columns {
        actual.insert(column.map_err(|source| {
            AppError::new("ygopro_cdb.schema_query_failed", source.to_string())
        })?);
    }
    if actual.is_empty() {
        return Err(
            AppError::new("ygopro_cdb.schema_missing_table", "CDB table is missing")
                .with_detail("table", table),
        );
    }
    let missing = required
        .iter()
        .filter(|column| !actual.contains(**column))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(AppError::new(
            "ygopro_cdb.schema_missing_columns",
            "CDB table is missing required columns",
        )
        .with_detail("table", table)
        .with_detail("columns", missing));
    }
    Ok(())
}

fn decode_card_row(row: &Row<'_>) -> rusqlite::Result<YgoProCardRecord> {
    let code = read_u32_bits(row, 0)?;
    let raw_type = read_u32_bits(row, 4)? as u64;
    let raw_level = read_u32_bits(row, 7)? as u64;
    let raw_race = read_u32_bits(row, 8)? as u64;
    let raw_attribute = read_u32_bits(row, 9)? as u64;
    let def = row.get::<_, i32>(6)?;
    let texts = CardTexts {
        name: row.get::<_, Option<String>>(11)?.unwrap_or_default(),
        desc: row.get::<_, Option<String>>(12)?.unwrap_or_default(),
        strings: (13..=28)
            .map(|idx| {
                row.get::<_, Option<String>>(idx)
                    .map(|value| value.unwrap_or_default())
            })
            .collect::<Result<Vec<_>, _>>()?,
    };
    let now = now_utc();
    let card = CardEntity {
        id: code.to_string(),
        code,
        ot: parse_ot(read_u32_bits(row, 1)?),
        alias: read_u32_bits(row, 2)?,
        setcode: read_i64_bits(row, 3)?,
        category: read_u32_bits(row, 10)? as u64,
        primary_type: parse_primary_type(raw_type),
        texts: BTreeMap::from([("default".to_string(), texts)]),
        monster_flags: parse_monster_flags(raw_type),
        atk: parse_atk(raw_type, row.get::<_, i32>(5)?),
        def: parse_def(raw_type, def),
        race: parse_race(raw_race),
        attribute: parse_attribute(raw_attribute),
        level: parse_level(raw_type, raw_level),
        pendulum: parse_pendulum(raw_type, raw_level),
        link: parse_link(raw_type, def),
        spell_subtype: parse_spell_subtype(raw_type),
        trap_subtype: parse_trap_subtype(raw_type),
        created_at: now,
        updated_at: now,
    };

    Ok(YgoProCardRecord {
        card,
        raw_type,
        raw_race,
        raw_attribute,
        raw_level,
    })
}

fn read_u32_bits(row: &Row<'_>, index: usize) -> rusqlite::Result<u32> {
    row.get::<_, i64>(index).map(|value| value as u32)
}

fn read_i64_bits(row: &Row<'_>, index: usize) -> rusqlite::Result<u64> {
    row.get::<_, i64>(index).map(|value| value as u64)
}

fn parse_primary_type(raw_type: u64) -> PrimaryType {
    if raw_type & TYPE_SPELL != 0 {
        PrimaryType::Spell
    } else if raw_type & TYPE_TRAP != 0 {
        PrimaryType::Trap
    } else {
        PrimaryType::Monster
    }
}

fn parse_ot(value: u32) -> Ot {
    match value {
        1 => Ot::Ocg,
        2 => Ot::Tcg,
        _ => Ot::Custom,
    }
}

fn encode_ot(value: &Ot) -> u32 {
    match value {
        Ot::Ocg => 1,
        Ot::Tcg => 2,
        Ot::Custom => 3,
    }
}

fn parse_monster_flags(raw_type: u64) -> Option<Vec<MonsterFlag>> {
    if raw_type & TYPE_MONSTER == 0 {
        return None;
    }

    let mut flags = Vec::new();
    for (mask, flag) in [
        (TYPE_NORMAL, MonsterFlag::Normal),
        (TYPE_EFFECT, MonsterFlag::Effect),
        (TYPE_FUSION, MonsterFlag::Fusion),
        (TYPE_RITUAL, MonsterFlag::Ritual),
        (TYPE_SYNCHRO, MonsterFlag::Synchro),
        (TYPE_XYZ, MonsterFlag::Xyz),
        (TYPE_PENDULUM, MonsterFlag::Pendulum),
        (TYPE_LINK, MonsterFlag::Link),
        (TYPE_TUNER, MonsterFlag::Tuner),
        (TYPE_TOKEN, MonsterFlag::Token),
        (TYPE_DUAL, MonsterFlag::Gemini),
        (TYPE_SPIRIT, MonsterFlag::Spirit),
        (TYPE_UNION, MonsterFlag::Union),
        (TYPE_FLIP, MonsterFlag::Flip),
        (TYPE_TOON, MonsterFlag::Toon),
    ] {
        if raw_type & mask != 0 {
            flags.push(flag);
        }
    }

    if flags.is_empty() {
        flags.push(MonsterFlag::Normal);
    }
    Some(flags)
}

fn parse_atk(raw_type: u64, atk: i32) -> Option<i32> {
    if raw_type & TYPE_MONSTER != 0 {
        Some(atk)
    } else {
        None
    }
}

fn parse_def(raw_type: u64, def: i32) -> Option<i32> {
    if raw_type & TYPE_MONSTER == 0 || raw_type & TYPE_LINK != 0 {
        None
    } else {
        Some(def)
    }
}

fn parse_level(raw_type: u64, raw_level: u64) -> Option<i32> {
    if raw_type & TYPE_MONSTER == 0 || raw_type & TYPE_LINK != 0 {
        None
    } else {
        Some((raw_level & 0xff) as i32)
    }
}

fn parse_pendulum(raw_type: u64, raw_level: u64) -> Option<Pendulum> {
    if raw_type & TYPE_PENDULUM == 0 {
        return None;
    }

    Some(Pendulum {
        left_scale: ((raw_level >> 24) & 0xff) as i32,
        right_scale: ((raw_level >> 16) & 0xff) as i32,
    })
}

fn parse_link(raw_type: u64, def: i32) -> Option<LinkData> {
    if raw_type & TYPE_LINK == 0 {
        return None;
    }

    let mut markers = Vec::new();
    for (mask, marker) in [
        (LINK_MARKER_TOP_LEFT, LinkMarker::TopLeft),
        (LINK_MARKER_TOP, LinkMarker::Top),
        (LINK_MARKER_TOP_RIGHT, LinkMarker::TopRight),
        (LINK_MARKER_LEFT, LinkMarker::Left),
        (LINK_MARKER_RIGHT, LinkMarker::Right),
        (LINK_MARKER_BOTTOM_LEFT, LinkMarker::BottomLeft),
        (LINK_MARKER_BOTTOM, LinkMarker::Bottom),
        (LINK_MARKER_BOTTOM_RIGHT, LinkMarker::BottomRight),
    ] {
        if def & mask != 0 {
            markers.push(marker);
        }
    }

    Some(LinkData { markers })
}

fn parse_spell_subtype(raw_type: u64) -> Option<SpellSubtype> {
    if raw_type & TYPE_SPELL == 0 {
        return None;
    }
    if raw_type & TYPE_QUICKPLAY != 0 {
        Some(SpellSubtype::QuickPlay)
    } else if raw_type & TYPE_CONTINUOUS != 0 {
        Some(SpellSubtype::Continuous)
    } else if raw_type & TYPE_RITUAL != 0 {
        Some(SpellSubtype::Ritual)
    } else if raw_type & TYPE_FIELD != 0 {
        Some(SpellSubtype::Field)
    } else if raw_type & TYPE_EQUIP != 0 {
        Some(SpellSubtype::Equip)
    } else {
        Some(SpellSubtype::Normal)
    }
}

fn parse_trap_subtype(raw_type: u64) -> Option<TrapSubtype> {
    if raw_type & TYPE_TRAP == 0 {
        return None;
    }
    if raw_type & TYPE_CONTINUOUS != 0 {
        Some(TrapSubtype::Continuous)
    } else if raw_type & TYPE_COUNTER != 0 {
        Some(TrapSubtype::Counter)
    } else {
        Some(TrapSubtype::Normal)
    }
}

#[derive(Debug, Clone)]
struct EncodedCardData {
    raw_type: u64,
    atk: i32,
    def: i32,
    raw_level: u64,
    raw_race: u64,
    raw_attribute: u64,
}

fn encode_card(card: &CardEntity) -> AppResult<EncodedCardData> {
    let mut raw_type = match card.primary_type {
        PrimaryType::Monster => TYPE_MONSTER,
        PrimaryType::Spell => TYPE_SPELL,
        PrimaryType::Trap => TYPE_TRAP,
    };

    let mut atk = 0;
    let mut def = 0;
    let mut raw_level = 0;
    let mut raw_race = 0;
    let mut raw_attribute = 0;

    match card.primary_type {
        PrimaryType::Monster => {
            for flag in card.monster_flags.as_deref().unwrap_or_default() {
                raw_type |= encode_monster_flag(flag);
            }
            atk = card.atk.unwrap_or(0);
            if raw_type & TYPE_LINK != 0 {
                def = encode_link_markers(card.link.as_ref());
            } else {
                def = card.def.unwrap_or(0);
                raw_level = encode_level(card.level.unwrap_or(0), card.pendulum.as_ref());
            }
            raw_race = card.race.as_ref().map(encode_race).unwrap_or(0);
            raw_attribute = card.attribute.as_ref().map(encode_attribute).unwrap_or(0);
        }
        PrimaryType::Spell => {
            raw_type |= encode_spell_subtype(card.spell_subtype.as_ref());
        }
        PrimaryType::Trap => {
            raw_type |= encode_trap_subtype(card.trap_subtype.as_ref());
        }
    }

    Ok(EncodedCardData {
        raw_type,
        atk,
        def,
        raw_level,
        raw_race,
        raw_attribute,
    })
}

fn encode_monster_flag(flag: &MonsterFlag) -> u64 {
    match flag {
        MonsterFlag::Normal => TYPE_NORMAL,
        MonsterFlag::Effect => TYPE_EFFECT,
        MonsterFlag::Fusion => TYPE_FUSION,
        MonsterFlag::Ritual => TYPE_RITUAL,
        MonsterFlag::Synchro => TYPE_SYNCHRO,
        MonsterFlag::Xyz => TYPE_XYZ,
        MonsterFlag::Pendulum => TYPE_PENDULUM,
        MonsterFlag::Link => TYPE_LINK,
        MonsterFlag::Tuner => TYPE_TUNER,
        MonsterFlag::Token => TYPE_TOKEN,
        MonsterFlag::Gemini => TYPE_DUAL,
        MonsterFlag::Spirit => TYPE_SPIRIT,
        MonsterFlag::Union => TYPE_UNION,
        MonsterFlag::Flip => TYPE_FLIP,
        MonsterFlag::Toon => TYPE_TOON,
    }
}

fn encode_spell_subtype(value: Option<&SpellSubtype>) -> u64 {
    match value {
        Some(SpellSubtype::QuickPlay) => TYPE_QUICKPLAY,
        Some(SpellSubtype::Continuous) => TYPE_CONTINUOUS,
        Some(SpellSubtype::Ritual) => TYPE_RITUAL,
        Some(SpellSubtype::Field) => TYPE_FIELD,
        Some(SpellSubtype::Equip) => TYPE_EQUIP,
        Some(SpellSubtype::Normal) | None => 0,
    }
}

fn encode_trap_subtype(value: Option<&TrapSubtype>) -> u64 {
    match value {
        Some(TrapSubtype::Continuous) => TYPE_CONTINUOUS,
        Some(TrapSubtype::Counter) => TYPE_COUNTER,
        Some(TrapSubtype::Normal) | None => 0,
    }
}

fn encode_race(value: &Race) -> u64 {
    match value {
        Race::Warrior => 0x1,
        Race::Spellcaster => 0x2,
        Race::Fairy => 0x4,
        Race::Fiend => 0x8,
        Race::Zombie => 0x10,
        Race::Machine => 0x20,
        Race::Aqua => 0x40,
        Race::Pyro => 0x80,
        Race::Rock => 0x100,
        Race::WingedBeast => 0x200,
        Race::Plant => 0x400,
        Race::Insect => 0x800,
        Race::Thunder => 0x1000,
        Race::Dragon => 0x2000,
        Race::Beast => 0x4000,
        Race::BeastWarrior => 0x8000,
        Race::Dinosaur => 0x10000,
        Race::Fish => 0x20000,
        Race::SeaSerpent => 0x40000,
        Race::Reptile => 0x80000,
        Race::Psychic => 0x100000,
        Race::DivineBeast => 0x200000,
        Race::CreatorGod => 0x400000,
        Race::Wyrm => 0x800000,
        Race::Cyberse => 0x1000000,
        Race::Illusion => 0x2000000,
    }
}

fn encode_attribute(value: &Attribute) -> u64 {
    match value {
        Attribute::Earth => 0x01,
        Attribute::Water => 0x02,
        Attribute::Fire => 0x04,
        Attribute::Wind => 0x08,
        Attribute::Light => 0x10,
        Attribute::Dark => 0x20,
        Attribute::Divine => 0x40,
    }
}

fn encode_level(level: i32, pendulum: Option<&Pendulum>) -> u64 {
    let level_bits = (level as u64) & 0xff;
    if let Some(pendulum) = pendulum {
        (((pendulum.left_scale as u64) & 0xff) << 24)
            | (((pendulum.right_scale as u64) & 0xff) << 16)
            | level_bits
    } else {
        level_bits
    }
}

fn encode_link_markers(link: Option<&LinkData>) -> i32 {
    let mut raw = 0;
    if let Some(link) = link {
        for marker in &link.markers {
            raw |= match marker {
                LinkMarker::TopLeft => LINK_MARKER_TOP_LEFT,
                LinkMarker::Top => LINK_MARKER_TOP,
                LinkMarker::TopRight => LINK_MARKER_TOP_RIGHT,
                LinkMarker::Left => LINK_MARKER_LEFT,
                LinkMarker::Right => LINK_MARKER_RIGHT,
                LinkMarker::BottomLeft => LINK_MARKER_BOTTOM_LEFT,
                LinkMarker::Bottom => LINK_MARKER_BOTTOM,
                LinkMarker::BottomRight => LINK_MARKER_BOTTOM_RIGHT,
            };
        }
    }
    raw
}

fn padded_card_strings(strings: &[String]) -> Vec<String> {
    let mut padded = strings.iter().take(16).cloned().collect::<Vec<_>>();
    padded.resize(16, String::new());
    padded
}

fn parse_attribute(value: u64) -> Option<Attribute> {
    match value {
        0x01 => Some(Attribute::Earth),
        0x02 => Some(Attribute::Water),
        0x04 => Some(Attribute::Fire),
        0x08 => Some(Attribute::Wind),
        0x10 => Some(Attribute::Light),
        0x20 => Some(Attribute::Dark),
        0x40 => Some(Attribute::Divine),
        _ => None,
    }
}

fn parse_race(value: u64) -> Option<Race> {
    match value {
        0x1 => Some(Race::Warrior),
        0x2 => Some(Race::Spellcaster),
        0x4 => Some(Race::Fairy),
        0x8 => Some(Race::Fiend),
        0x10 => Some(Race::Zombie),
        0x20 => Some(Race::Machine),
        0x40 => Some(Race::Aqua),
        0x80 => Some(Race::Pyro),
        0x100 => Some(Race::Rock),
        0x200 => Some(Race::WingedBeast),
        0x400 => Some(Race::Plant),
        0x800 => Some(Race::Insect),
        0x1000 => Some(Race::Thunder),
        0x2000 => Some(Race::Dragon),
        0x4000 => Some(Race::Beast),
        0x8000 => Some(Race::BeastWarrior),
        0x10000 => Some(Race::Dinosaur),
        0x20000 => Some(Race::Fish),
        0x40000 => Some(Race::SeaSerpent),
        0x80000 => Some(Race::Reptile),
        0x100000 => Some(Race::Psychic),
        0x200000 => Some(Race::DivineBeast),
        0x400000 => Some(Race::CreatorGod),
        0x800000 => Some(Race::Wyrm),
        0x1000000 => Some(Race::Cyberse),
        0x2000000 => Some(Race::Illusion),
        _ => None,
    }
}
