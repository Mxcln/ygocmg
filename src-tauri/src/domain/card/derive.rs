use std::collections::BTreeMap;

use crate::domain::card::model::{
    CardEntity, CardListRow, CardTexts, MonsterFlag, PrimaryType, SpellSubtype, TrapSubtype,
};
use crate::domain::common::ids::LanguageCode;
use crate::domain::resource::model::CardAssetState;

pub fn resolve_display_texts<'a>(
    texts: &'a BTreeMap<LanguageCode, CardTexts>,
    display_language_order: &[LanguageCode],
) -> Option<&'a CardTexts> {
    for language in display_language_order {
        if let Some(texts) = texts.get(language) {
            return Some(texts);
        }
    }

    texts.values().next()
}

fn format_monster_flag(flag: &MonsterFlag) -> &'static str {
    match flag {
        MonsterFlag::Normal => "Normal",
        MonsterFlag::Effect => "Effect",
        MonsterFlag::Fusion => "Fusion",
        MonsterFlag::Ritual => "Ritual",
        MonsterFlag::Synchro => "Synchro",
        MonsterFlag::Xyz => "Xyz",
        MonsterFlag::Pendulum => "Pendulum",
        MonsterFlag::Link => "Link",
        MonsterFlag::Tuner => "Tuner",
        MonsterFlag::Token => "Token",
        MonsterFlag::Gemini => "Gemini",
        MonsterFlag::Spirit => "Spirit",
        MonsterFlag::Union => "Union",
        MonsterFlag::Flip => "Flip",
        MonsterFlag::Toon => "Toon",
    }
}

fn format_spell_subtype(subtype: &Option<SpellSubtype>) -> &'static str {
    match subtype {
        Some(SpellSubtype::Normal) | None => "Normal",
        Some(SpellSubtype::Continuous) => "Continuous",
        Some(SpellSubtype::QuickPlay) => "Quick-Play",
        Some(SpellSubtype::Ritual) => "Ritual",
        Some(SpellSubtype::Field) => "Field",
        Some(SpellSubtype::Equip) => "Equip",
    }
}

fn format_trap_subtype(subtype: &Option<TrapSubtype>) -> &'static str {
    match subtype {
        Some(TrapSubtype::Normal) | None => "Normal",
        Some(TrapSubtype::Continuous) => "Continuous",
        Some(TrapSubtype::Counter) => "Counter",
    }
}

fn derive_subtype_display(card: &CardEntity) -> String {
    match &card.primary_type {
        PrimaryType::Monster => match &card.monster_flags {
            Some(flags) if !flags.is_empty() => flags
                .iter()
                .map(format_monster_flag)
                .collect::<Vec<_>>()
                .join(" / "),
            _ => "Normal".to_string(),
        },
        PrimaryType::Spell => format_spell_subtype(&card.spell_subtype).to_string(),
        PrimaryType::Trap => format_trap_subtype(&card.trap_subtype).to_string(),
    }
}

pub fn derive_card_list_row(
    card: &CardEntity,
    asset_state: &CardAssetState,
    display_language_order: &[LanguageCode],
) -> CardListRow {
    let texts = resolve_display_texts(&card.texts, display_language_order);

    CardListRow {
        id: card.id.clone(),
        code: card.code,
        name: texts.map(|value| value.name.clone()).unwrap_or_default(),
        desc: texts.map(|value| value.desc.clone()).unwrap_or_default(),
        primary_type: card.primary_type.clone(),
        subtype_display: derive_subtype_display(card),
        atk: card.atk,
        def: card.def,
        level: card.level,
        has_image: asset_state.has_image,
        has_script: asset_state.has_script,
        has_field_image: asset_state.has_field_image,
    }
}
