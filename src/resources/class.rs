use lazy_static::lazy_static;
use serde::Deserialize;
use smartstring::alias::String;
use std::{collections::HashMap, fmt};

use crate::{
    bonuses::{Bonus, HasModifiers, Modifier, Modifies},
    resources::{traits::Resource, ArmorCategory, Character, WeaponCategory},
    stats::{Ability, Proficiency, ProficiencyCategory as PC, ProvidesProficiency, Skill},
};

#[derive(Clone, Debug, Deserialize)]
pub struct Class {
    name: String,
    key_ability: Vec<Ability>,
    hp_per_level: u16,
    perception: Proficiency,
    fort_save: Proficiency,
    reflex_save: Proficiency,
    will_save: Proficiency,
    #[serde(default)]
    trained_skill_options: Vec<Option<Skill>>,
    #[serde(default)]
    free_skill_trained: u8,
    #[serde(default)]
    weapon_proficiencies: HashMap<WeaponCategory, Proficiency>,
    #[serde(default)]
    armor_proficiencies: HashMap<ArmorCategory, Proficiency>,
}

impl Resource for Class {
    fn get_index_value(&self, _extra: &()) -> String {
        self.name.clone()
    }
}

impl fmt::Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.name.fmt(f)
    }
}

impl HasModifiers for Class {
    fn get_modifier(&self, c: &Character, m: Modifies) -> Modifier {
        let level = c.class_level(&self.name);
        let bonus = match m {
            Modifies::ClassDC => {
                let p = match level.get() {
                    1..=10 => Proficiency::Trained,
                    11..=18 => Proficiency::Expert,
                    19..=20 => Proficiency::Master,
                    _ => panic!("Level should be between 1 and 20, found {}", level.get()),
                };
                p.bonus(level)
            }
            Modifies::FortitudeSave => self.fort_save.bonus(level),
            Modifies::HP => {
                let per_level = self.hp_per_level as i16;
                let level = level.get() as i16;
                Bonus::untyped(level * per_level)
            }
            Modifies::Perception => self.perception.bonus(level),
            Modifies::ReflexSave => self.reflex_save.bonus(level),
            Modifies::WillSave => self.will_save.bonus(level),
            Modifies::Speed => {
                // should handle Monk speed boost here
                Bonus::none()
            }
            _ => Bonus::none(),
        };
        Modifier::new() + bonus
    }
}

lazy_static! {
    pub static ref FIGHTER: Class = Class {
        name: "Fighter".into(),
        key_ability: vec![Ability::STR, Ability::DEX],
        hp_per_level: 10,
        perception: Proficiency::Expert,
        fort_save: Proficiency::Expert,
        reflex_save: Proficiency::Expert,
        will_save: Proficiency::Trained,
        trained_skill_options: vec![Some(Skill::Acrobatics), Some(Skill::Athletics)],
        free_skill_trained: 3,
        weapon_proficiencies: [
            (WeaponCategory::Simple, Proficiency::Expert),
            (WeaponCategory::Martial, Proficiency::Expert),
            (WeaponCategory::Advanced, Proficiency::Trained),
            (WeaponCategory::Unarmed, Proficiency::Expert),
        ]
        .iter()
        .cloned()
        .collect(),
        armor_proficiencies: [
            (ArmorCategory::Light, Proficiency::Trained),
            (ArmorCategory::Medium, Proficiency::Trained),
            (ArmorCategory::Heavy, Proficiency::Trained),
            (ArmorCategory::Unarmored, Proficiency::Trained),
        ]
        .iter()
        .cloned()
        .collect(),
    };
    pub static ref CHAMPION: Class = Class {
        name: "Champion".into(),
        key_ability: vec![Ability::STR, Ability::DEX],
        hp_per_level: 10,
        perception: Proficiency::Trained,
        fort_save: Proficiency::Expert,
        reflex_save: Proficiency::Trained,
        will_save: Proficiency::Expert,
        trained_skill_options: vec![Some(Skill::Religion), None],
        free_skill_trained: 3,
        weapon_proficiencies: [
            (WeaponCategory::Simple, Proficiency::Trained),
            (WeaponCategory::Martial, Proficiency::Trained),
            (WeaponCategory::Unarmed, Proficiency::Trained),
        ]
        .iter()
        .cloned()
        .collect(),
        armor_proficiencies: [
            (ArmorCategory::Light, Proficiency::Trained),
            (ArmorCategory::Medium, Proficiency::Trained),
            (ArmorCategory::Heavy, Proficiency::Trained),
            (ArmorCategory::Unarmored, Proficiency::Trained),
        ]
        .iter()
        .cloned()
        .collect(),
    };
}

impl ProvidesProficiency for Class {
    fn get_proficiency_level(&self, character: &Character, p: &PC) -> Proficiency {
        let level = character.class_level(&self.name);
        match (self.name.as_ref(), p, level) {
            ("Champion", PC::Perception, _) => Proficiency::Trained,
            (_, PC::Armor(category), _) => self
                .armor_proficiencies
                .get(category)
                .cloned()
                .unwrap_or_default(),
            (_, PC::Weapon(category), _) => self
                .weapon_proficiencies
                .get(category)
                .cloned()
                .unwrap_or_default(),
            _ => Proficiency::Untrained,
        }
    }
}
