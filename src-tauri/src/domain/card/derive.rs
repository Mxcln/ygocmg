use std::collections::BTreeMap;

use crate::domain::card::model::{CardEntity, CardListRow, CardTexts};
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
        atk: card.atk,
        def: card.def,
        level: card.level,
        has_image: asset_state.has_image,
        has_script: asset_state.has_script,
        has_field_image: asset_state.has_field_image,
    }
}
