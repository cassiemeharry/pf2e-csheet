use serde::Deserialize;
use smartstring::alias::String;
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

mod armor;
mod weapon;

use crate::{
    bonuses::{Bonus, HasModifiers, Modifier, Modifies, Penalty},
    resources::{Character, Resource},
    stats::{Bulk, DamageType, Gold, Level},
};

pub use armor::*;
pub use weapon::*;

#[derive(Clone, Debug, Deserialize)]
pub struct ArmorInfo {
    pub category: ArmorCategory,
    #[serde(default)]
    pub dex_cap: Option<u8>,
    #[serde(default)]
    pub check_penalty: Option<Penalty>,
    pub min_strength: u8,
    #[serde(default)]
    pub traits: Vec<ArmorTrait>,
}

impl ArmorInfo {
    pub fn no_armor() -> Self {
        Self {
            category: ArmorCategory::Unarmored,
            dex_cap: None,
            check_penalty: None,
            min_strength: 0,
            traits: vec![],
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ShieldInfo {
    pub hp: u16,
    pub hardness: u16,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WeaponInfo {
    #[serde(default)]
    pub range: Option<u16>,
    pub hands: usize,
    pub damage_die: WeaponDie,
    pub damage_type: DamageType,
    pub category: WeaponCategory,
    pub group: WeaponGroup,
    #[serde(default)]
    pub traits: Vec<WeaponTrait>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    Armor(ArmorInfo),
    Shield(ShieldInfo),
    Weapon(WeaponInfo),
}

#[derive(Clone, Debug, Deserialize)]
pub struct Item {
    name: String,
    level: Level,
    #[serde(default)]
    bulk: Bulk,
    #[serde(default)]
    price: Option<Gold>,
    #[serde(flatten)]
    item_type: ItemType,
    #[serde(default)]
    bonuses: HashMap<Modifies, Bonus>,
    #[serde(default)]
    penalties: HashMap<Modifies, Penalty>,
    #[serde(default)]
    traits: HashSet<String>,
}

impl Item {
    // pub fn item_type(&self) -> &ItemType {
    //     &self.item_type
    // }

    pub fn armor_info(&self) -> Option<&ArmorInfo> {
        match &self.item_type {
            ItemType::Armor(info) => Some(info),
            _ => None,
        }
    }

    pub fn shield_info(&self) -> Option<&ShieldInfo> {
        match &self.item_type {
            ItemType::Shield(info) => Some(info),
            _ => None,
        }
    }

    pub fn weapon_info(&self) -> Option<&WeaponInfo> {
        match &self.item_type {
            ItemType::Weapon(info) => Some(info),
            _ => None,
        }
    }
}

impl Resource for Item {
    fn get_index_value(&self) -> String {
        self.name.clone()
    }
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl HasModifiers for Item {
    fn get_modifier(&self, _c: &Character, m: Modifies) -> Modifier {
        let bonus = self.bonuses.get(&m).cloned().unwrap_or_default();
        let penalty = self.penalties.get(&m).cloned().unwrap_or_default();
        // This is a decent enough place to hook modifiers
        bonus + penalty
    }
}
