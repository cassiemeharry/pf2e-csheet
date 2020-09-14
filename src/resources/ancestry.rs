use lazy_static::lazy_static;
use serde::Deserialize;
use smartstring::alias::String;
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use crate::{
    bonuses::{Bonus, HasModifiers, Modifier, Modifies, Penalty},
    resources::{traits::Resource, Character},
    stats::{Ability, AbilityBoost, Size},
};

#[derive(Clone, Debug, Deserialize)]
pub struct Ancestry {
    name: String,
    size: Size,
    #[serde(default)]
    ability_boosts: Vec<AbilityBoost>,
    #[serde(default)]
    ability_flaws: Vec<Ability>,
    #[serde(default)]
    starting_languages: Vec<String>,
    #[serde(default)]
    flat_bonuses: HashMap<Modifies, Bonus>,
    #[serde(default)]
    flat_penalties: HashMap<Modifies, Penalty>,
    #[serde(default)]
    per_level_bonuses: HashMap<Modifies, Bonus>,
    #[serde(default)]
    per_level_penalties: HashMap<Modifies, Penalty>,
}

impl fmt::Display for Ancestry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Resource for Ancestry {
    fn get_index_value(&self, _extra: &()) -> String {
        self.name.clone()
    }
}

impl HasModifiers for Ancestry {
    fn get_modifier(&self, c: &Character, m: Modifies) -> Modifier {
        let mut bonus = Bonus::none();
        let level = c.character_level();
        if let Some(flat) = self.flat_bonuses.get(&m) {
            bonus += *flat;
        }
        if let Some(per_level) = self.per_level_bonuses.get(&m) {
            bonus += *per_level * level;
        }

        let mut penalty = Penalty::none();
        if let Some(flat) = self.flat_penalties.get(&m) {
            penalty += *flat;
        }
        if let Some(per_level) = self.per_level_penalties.get(&m) {
            penalty += *per_level * level;
        }

        if let Modifies::Resistance(s) = m.clone() {
            match (s.as_ref(), self.name.as_ref(), level.get()) {
                ("silver", "Werebear", 0) => (),
                ("silver", "Werebear", 1..=4) => {
                    penalty += Penalty::untyped(5);
                }
                ("silver", "Werebear", 5..=7) => {
                    penalty += Penalty::untyped(7);
                }
                ("silver", "Werebear", 8..=14) => {
                    penalty += Penalty::untyped(10);
                }
                ("silver", "Werebear", _) => {
                    penalty += Penalty::untyped(15);
                }
                _ => (),
            }
        }

        Modifier::new() + bonus + penalty
    }

    fn get_modified_resistances(&self, _c: &Character) -> HashSet<String> {
        match self.name.as_ref() {
            "Werebear" => vec!["silver".into()].into_iter().collect(),
            _ => HashSet::new(),
        }
    }
}

lazy_static! {
    pub static ref HUMAN: Ancestry = Ancestry {
        name: "Human".into(),
        size: Size::Medium,
        ability_boosts: vec![AbilityBoost::Free, AbilityBoost::Free],
        ability_flaws: vec![],
        starting_languages: vec!["Common".into()],
        flat_bonuses: vec![
            (Modifies::HP, Bonus::untyped(8)),
            (Modifies::Speed, Bonus::untyped(25))
        ]
        .into_iter()
        .collect(),
        flat_penalties: HashMap::new(),
        per_level_bonuses: HashMap::new(),
        per_level_penalties: HashMap::new(),
    };
    pub static ref WEREBEAR: Ancestry = Ancestry {
        name: "Werebear".into(),
        size: Size::Large,
        ability_boosts: vec![
            AbilityBoost::Fixed(Ability::WIS),
            AbilityBoost::Fixed(Ability::STR),
            AbilityBoost::Fixed(Ability::CON)
        ],
        ability_flaws: vec![Ability::CHA],
        starting_languages: vec!["Common".into(), "bear empathy".into()],
        flat_bonuses: vec![
            (Modifies::HP, Bonus::untyped(5)),
            (Modifies::Speed, Bonus::untyped(25)),
            (Modifies::Attack, Bonus::untyped(1)),
            (Modifies::AC, Bonus::untyped(1)),
            (Modifies::FortitudeSave, Bonus::untyped(1)),
            (Modifies::ReflexSave, Bonus::untyped(1)),
            (Modifies::WillSave, Bonus::untyped(1)),
        ]
        .into_iter()
        .collect(),
        flat_penalties: HashMap::new(),
        per_level_bonuses: vec![(Modifies::HP, Bonus::untyped(5))]
            .into_iter()
            .collect(),
        per_level_penalties: HashMap::new(),
    };
}
