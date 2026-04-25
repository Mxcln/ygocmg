use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::domain::common::ids::{CardId, LanguageCode};
use crate::domain::common::time::AppTimestamp;

pub const QMARK: i32 = -2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrimaryType {
    Monster,
    Spell,
    Trap,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Ot {
    Ocg,
    Tcg,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MonsterFlag {
    Normal,
    Effect,
    Fusion,
    Ritual,
    Synchro,
    Xyz,
    Pendulum,
    Link,
    Tuner,
    Token,
    Gemini,
    Spirit,
    Union,
    Flip,
    Toon,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Race {
    Warrior,
    Spellcaster,
    Dragon,
    Zombie,
    Machine,
    Aqua,
    Pyro,
    Rock,
    WingedBeast,
    Plant,
    Insect,
    Thunder,
    Fish,
    SeaSerpent,
    Reptile,
    Psychic,
    DivineBeast,
    Beast,
    BeastWarrior,
    Dinosaur,
    Fairy,
    Fiend,
    Illusion,
    Cyberse,
    CreatorGod,
    Wyrm,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Attribute {
    Light,
    Dark,
    Earth,
    Water,
    Fire,
    Wind,
    Divine,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpellSubtype {
    Normal,
    Continuous,
    QuickPlay,
    Ritual,
    Field,
    Equip,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrapSubtype {
    Normal,
    Continuous,
    Counter,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum LinkMarker {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CardTexts {
    pub name: String,
    pub desc: String,
    pub strings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Pendulum {
    pub left_scale: i32,
    pub right_scale: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LinkData {
    pub markers: Vec<LinkMarker>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CardEntity {
    pub id: CardId,
    pub code: u32,
    pub alias: u32,
    pub setcode: u64,
    pub ot: Ot,
    pub category: u64,
    pub primary_type: PrimaryType,
    pub texts: BTreeMap<LanguageCode, CardTexts>,
    pub monster_flags: Option<Vec<MonsterFlag>>,
    pub atk: Option<i32>,
    pub def: Option<i32>,
    pub race: Option<Race>,
    pub attribute: Option<Attribute>,
    pub level: Option<i32>,
    pub pendulum: Option<Pendulum>,
    pub link: Option<LinkData>,
    pub spell_subtype: Option<SpellSubtype>,
    pub trap_subtype: Option<TrapSubtype>,
    pub created_at: AppTimestamp,
    pub updated_at: AppTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CardUpdateInput {
    pub code: u32,
    pub alias: u32,
    pub setcode: u64,
    pub ot: Ot,
    pub category: u64,
    pub primary_type: PrimaryType,
    pub texts: BTreeMap<LanguageCode, CardTexts>,
    pub monster_flags: Option<Vec<MonsterFlag>>,
    pub atk: Option<i32>,
    pub def: Option<i32>,
    pub race: Option<Race>,
    pub attribute: Option<Attribute>,
    pub level: Option<i32>,
    pub pendulum: Option<Pendulum>,
    pub link: Option<LinkData>,
    pub spell_subtype: Option<SpellSubtype>,
    pub trap_subtype: Option<TrapSubtype>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CardsFile {
    pub schema_version: u32,
    pub cards: Vec<CardEntity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CardListRow {
    pub id: CardId,
    pub code: u32,
    pub name: String,
    pub desc: String,
    pub primary_type: PrimaryType,
    pub atk: Option<i32>,
    pub def: Option<i32>,
    pub level: Option<i32>,
    pub has_image: bool,
    pub has_script: bool,
    pub has_field_image: bool,
}
