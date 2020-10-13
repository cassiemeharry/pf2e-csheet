#[cfg(test)]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(test, derive(Arbitrary))]
pub enum ItemType {
    Any,
    Armor,
    Shield,
    Weapon,
    Other,
}

impl Default for ItemType {
    fn default() -> Self {
        Self::Any
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum ArmorCategory {
    #[serde(rename = "unarmored")]
    Unarmored,
    #[serde(rename = "light armor")]
    LightArmor,
    #[serde(rename = "medium armor")]
    MediumArmor,
    #[serde(rename = "heavy armor")]
    HeavyArmor,
}
