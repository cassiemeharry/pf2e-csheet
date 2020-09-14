use anyhow::{anyhow, Error, Result};
use serde::Deserialize;
use std::{fmt, str::FromStr};

use crate::{
    stats::{DamageType, Range},
    try_from_str,
};

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub enum WeaponCategory {
    Unarmed,
    Simple,
    Martial,
    Advanced,
    // Other(String),
}

try_from_str!(WeaponCategory);

impl FromStr for WeaponCategory {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "unarmed" => Ok(Self::Unarmed),
            "simple" => Ok(Self::Simple),
            "martial" => Ok(Self::Martial),
            "advanced" => Ok(Self::Advanced),
            _ => Err(anyhow!("Unknown weapon category {:?}", s)),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub enum WeaponGroup {
    Axe,
    Bomb,
    Bow,
    Brawling,
    Club,
    Dart,
    Flail,
    Hammer,
    Knife,
    Natural,
    Pick,
    Polearm,
    Shield,
    Sling,
    Spear,
    Sword,
}

try_from_str!(WeaponGroup);

impl FromStr for WeaponGroup {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim() {
            "axe" => Ok(Self::Axe),
            "bomb" => Ok(Self::Bomb),
            "bow" => Ok(Self::Bow),
            "brawling" => Ok(Self::Brawling),
            "club" => Ok(Self::Club),
            "dart" => Ok(Self::Dart),
            "flail" => Ok(Self::Flail),
            "hammer" => Ok(Self::Hammer),
            "knife" => Ok(Self::Knife),
            "natural" => Ok(Self::Natural),
            "pick" => Ok(Self::Pick),
            "polearm" => Ok(Self::Polearm),
            "shield" => Ok(Self::Shield),
            "sling" => Ok(Self::Sling),
            "spear" => Ok(Self::Spear),
            "sword" => Ok(Self::Sword),
            other => Err(anyhow!("Unknown weapon group {:?}", other)),
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub enum WeaponTrait {
    Agile,
    Attached,
    Backstabber,
    Backswing,
    Deadly(WeaponDie),
    Disarm,
    Dwarf,
    Elf,
    Fatal(WeaponDie),
    Finesse,
    Forceful,
    FreeHand,
    Gnome,
    Goblin,
    Grapple,
    Halfling,
    Jousting,
    Monk,
    Nonleathal,
    Orc,
    Parry,
    Propulsive,
    Reach,
    Shove,
    Sweep,
    Thrown,
    Trip,
    Twin,
    TwoHand(WeaponDie),
    Unarmed,
    Versatile(DamageType),
    Volley(Range),
}

try_from_str!(WeaponTrait);

impl FromStr for WeaponTrait {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(crate::parsers::weapon_trait(s)?)
    }
}

impl fmt::Display for WeaponTrait {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Agile => write!(f, "agile"),
            Self::Attached => write!(f, "attached"),
            Self::Backstabber => write!(f, "backstabber"),
            Self::Backswing => write!(f, "backswing"),
            Self::Deadly(die) => write!(f, "deadly {}", die),
            Self::Disarm => write!(f, "disarm"),
            Self::Dwarf => write!(f, "dwarf"),
            Self::Elf => write!(f, "elf"),
            Self::Fatal(die) => write!(f, "fatal {}", die),
            Self::Finesse => write!(f, "finesse"),
            Self::Forceful => write!(f, "forceful"),
            Self::FreeHand => write!(f, "free-hand"),
            Self::Gnome => write!(f, "gnome"),
            Self::Goblin => write!(f, "goblin"),
            Self::Grapple => write!(f, "grapple"),
            Self::Halfling => write!(f, "halfling"),
            Self::Jousting => write!(f, "jousting"),
            Self::Monk => write!(f, "monk"),
            Self::Nonleathal => write!(f, "nonleathal"),
            Self::Orc => write!(f, "orc"),
            Self::Parry => write!(f, "parry"),
            Self::Propulsive => write!(f, "propulsive"),
            Self::Reach => write!(f, "reach"),
            Self::Shove => write!(f, "shove"),
            Self::Sweep => write!(f, "sweep"),
            Self::Thrown => write!(f, "thrown"),
            Self::Trip => write!(f, "trip"),
            Self::Twin => write!(f, "twin"),
            Self::TwoHand(die) => write!(f, "two-hand {}", die),
            Self::Unarmed => write!(f, "unarmed"),
            Self::Versatile(dt) => write!(f, "versatile {}", dt),
            Self::Volley(range) => write!(f, "volley {:#}", range),
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub enum WeaponDie {
    D4,
    D6,
    D8,
    D10,
    D12,
}

try_from_str!(WeaponDie);

impl FromStr for WeaponDie {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(crate::parsers::weapon_die(s)?)
    }
}

impl fmt::Display for WeaponDie {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::D4 => write!(f, "d4"),
            Self::D6 => write!(f, "d6"),
            Self::D8 => write!(f, "d8"),
            Self::D10 => write!(f, "d10"),
            Self::D12 => write!(f, "d12"),
        }
    }
}
