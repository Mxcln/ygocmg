use crate::domain::card::model::{CardEntity, CardUpdateInput, MonsterFlag, PrimaryType};
use crate::domain::common::ids::CardId;
use crate::domain::common::time::AppTimestamp;

pub fn normalize_card_input(mut input: CardUpdateInput) -> CardUpdateInput {
    for texts in input.texts.values_mut() {
        texts.name = texts.name.trim().to_string();
        texts.desc = texts.desc.trim().to_string();
        texts.strings = texts
            .strings
            .iter()
            .map(|value| value.trim().to_string())
            .collect();
    }

    if let Some(flags) = &mut input.monster_flags {
        flags.sort();
        flags.dedup();
    }

    if let Some(link) = &mut input.link {
        link.markers.sort();
        link.markers.dedup();
    }

    match input.primary_type {
        PrimaryType::Monster => {
            input.spell_subtype = None;
            input.trap_subtype = None;

            let flags = input.monster_flags.clone().unwrap_or_default();
            let is_link = flags.contains(&MonsterFlag::Link);
            let is_pendulum = flags.contains(&MonsterFlag::Pendulum);

            if is_link {
                input.def = None;
                input.level = None;
            } else {
                input.link = None;
            }

            if !is_pendulum {
                input.pendulum = None;
            }
        }
        PrimaryType::Spell => {
            input.monster_flags = None;
            input.atk = None;
            input.def = None;
            input.race = None;
            input.attribute = None;
            input.level = None;
            input.pendulum = None;
            input.link = None;
            input.trap_subtype = None;
        }
        PrimaryType::Trap => {
            input.monster_flags = None;
            input.atk = None;
            input.def = None;
            input.race = None;
            input.attribute = None;
            input.level = None;
            input.pendulum = None;
            input.link = None;
            input.spell_subtype = None;
        }
    }

    input
}

pub fn create_card_entity(new_id: CardId, input: CardUpdateInput, now: AppTimestamp) -> CardEntity {
    let normalized = normalize_card_input(input);
    CardEntity {
        id: new_id,
        code: normalized.code,
        alias: normalized.alias,
        setcode: normalized.setcode,
        ot: normalized.ot,
        category: normalized.category,
        primary_type: normalized.primary_type,
        texts: normalized.texts,
        monster_flags: normalized.monster_flags,
        atk: normalized.atk,
        def: normalized.def,
        race: normalized.race,
        attribute: normalized.attribute,
        level: normalized.level,
        pendulum: normalized.pendulum,
        link: normalized.link,
        spell_subtype: normalized.spell_subtype,
        trap_subtype: normalized.trap_subtype,
        created_at: now,
        updated_at: now,
    }
}

pub fn apply_card_update(
    existing: &CardEntity,
    input: CardUpdateInput,
    now: AppTimestamp,
) -> CardEntity {
    let normalized = normalize_card_input(input);
    CardEntity {
        id: existing.id.clone(),
        code: normalized.code,
        alias: normalized.alias,
        setcode: normalized.setcode,
        ot: normalized.ot,
        category: normalized.category,
        primary_type: normalized.primary_type,
        texts: normalized.texts,
        monster_flags: normalized.monster_flags,
        atk: normalized.atk,
        def: normalized.def,
        race: normalized.race,
        attribute: normalized.attribute,
        level: normalized.level,
        pendulum: normalized.pendulum,
        link: normalized.link,
        spell_subtype: normalized.spell_subtype,
        trap_subtype: normalized.trap_subtype,
        created_at: existing.created_at,
        updated_at: now,
    }
}
