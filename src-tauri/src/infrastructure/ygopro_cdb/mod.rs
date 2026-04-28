use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::{Connection, OpenFlags, Row};

use crate::domain::card::model::{
    Attribute, CardEntity, CardTexts, LinkData, LinkMarker, MonsterFlag, Ot, Pendulum,
    PrimaryType, Race, SpellSubtype, TrapSubtype,
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
        records.push(item.map_err(|source| {
            AppError::new("ygopro_cdb.row_decode_failed", source.to_string())
        })?);
    }
    Ok(records)
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
            "id", "name", "desc", "str1", "str2", "str3", "str4", "str5", "str6", "str7",
            "str8", "str9", "str10", "str11", "str12", "str13", "str14", "str15", "str16",
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
        return Err(AppError::new(
            "ygopro_cdb.schema_missing_table",
            "CDB table is missing",
        )
        .with_detail("table", table));
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
            .map(|idx| row.get::<_, Option<String>>(idx).map(|value| value.unwrap_or_default()))
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
