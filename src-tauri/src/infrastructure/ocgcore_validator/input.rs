use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;

use crate::domain::card::model::CardEntity;
use crate::domain::common::error::AppResult;
use crate::infrastructure::ygopro_cdb::encode_card_data_for_ocgcore;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperInput {
    pub request_id: String,
    pub level: String,
    pub card: HelperCardInput,
    pub scripts: BTreeMap<String, String>,
    pub core_script_root: String,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperCardInput {
    pub code: u32,
    pub alias: u32,
    pub setcodes: Vec<u16>,
    #[serde(rename = "type")]
    pub raw_type: u32,
    pub level: u32,
    pub attribute: u32,
    pub race: u32,
    pub attack: i32,
    pub defense: i32,
    pub lscale: u32,
    pub rscale: u32,
    pub link_marker: u32,
    pub rule_code: u32,
}

impl HelperInput {
    pub fn from_card_script(
        request_id: String,
        card: &CardEntity,
        script_text: &str,
        core_script_root: PathBuf,
        timeout_ms: u64,
    ) -> AppResult<Self> {
        let encoded = encode_card_data_for_ocgcore(card)?;
        let mut scripts = BTreeMap::new();
        scripts.insert(format!("./script/c{}.lua", card.code), script_text.to_string());

        Ok(Self {
            request_id,
            level: "ocgcore_init".to_string(),
            card: HelperCardInput {
                code: card.code,
                alias: card.alias,
                setcodes: card.setcodes.clone(),
                raw_type: encoded.raw_type,
                level: encoded.level,
                attribute: encoded.attribute,
                race: encoded.race,
                attack: encoded.attack,
                defense: encoded.defense,
                lscale: encoded.lscale,
                rscale: encoded.rscale,
                link_marker: encoded.link_marker,
                rule_code: 0,
            },
            scripts,
            core_script_root: core_script_root.display().to_string(),
            timeout_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use crate::domain::card::model::{
        Attribute, CardEntity, CardTexts, MonsterFlag, Ot, PrimaryType, Race,
    };
    use crate::domain::common::time::now_utc;

    use super::*;

    #[test]
    fn helper_input_serializes_expected_script_and_card_shape() {
        let now = now_utc();
        let card = CardEntity {
            id: "card-1".to_string(),
            code: 99999999,
            alias: 0,
            setcodes: vec![0x1234],
            ot: Ot::Custom,
            category: 0,
            primary_type: PrimaryType::Monster,
            texts: BTreeMap::from([(
                "en".to_string(),
                CardTexts {
                    name: "Validator".to_string(),
                    desc: String::new(),
                    strings: Vec::new(),
                },
            )]),
            monster_flags: Some(vec![MonsterFlag::Effect]),
            atk: Some(0),
            def: Some(0),
            race: Some(Race::Warrior),
            attribute: Some(Attribute::Light),
            level: Some(4),
            pendulum: None,
            link: None,
            spell_subtype: None,
            trap_subtype: None,
            created_at: now,
            updated_at: now,
        };

        let input = HelperInput::from_card_script(
            "req-1".to_string(),
            &card,
            "local s,id,o=GetID()\nfunction s.initial_effect(c)\nend\n",
            PathBuf::from("D:/Game/YGODIY/ygocmg/third_party/ygopro-scripts"),
            3000,
        )
        .unwrap();
        let value = serde_json::to_value(&input).unwrap();

        assert_eq!(value["requestId"], "req-1");
        assert_eq!(value["level"], "ocgcore_init");
        assert_eq!(value["card"]["code"], 99999999);
        assert_eq!(value["card"]["type"], 33);
        assert_eq!(value["card"]["level"], 4);
        assert_eq!(
            value["scripts"]["./script/c99999999.lua"],
            "local s,id,o=GetID()\nfunction s.initial_effect(c)\nend\n"
        );
        assert_eq!(value["timeoutMs"], 3000);
    }
}
