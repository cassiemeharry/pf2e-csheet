use anyhow::{anyhow, Error, Result};
use lazy_static::lazy_static;
use serde::Deserialize;
use smartstring::alias::String;
use std::{fmt, str::FromStr};

use crate::{bonuses::Bonus, character::Character, try_from_str};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Ord, PartialOrd, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub enum Ability {
    STR,
    DEX,
    CON,
    INT,
    WIS,
    CHA,
}

impl Ability {
    pub fn iter_all() -> impl Iterator<Item = Self> {
        let mut n = 0;
        std::iter::from_fn(move || match n {
            0 => {
                n += 1;
                Some(Self::STR)
            }
            1 => {
                n += 1;
                Some(Self::DEX)
            }
            2 => {
                n += 1;
                Some(Self::CON)
            }
            3 => {
                n += 1;
                Some(Self::INT)
            }
            4 => {
                n += 1;
                Some(Self::WIS)
            }
            5 => {
                n += 1;
                Some(Self::CHA)
            }
            _ => None,
        })
    }
}

try_from_str!(Ability);

impl FromStr for Ability {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "STR" => Ok(Self::STR),
            "DEX" => Ok(Self::DEX),
            "CON" => Ok(Self::CON),
            "INT" => Ok(Self::INT),
            "WIS" => Ok(Self::WIS),
            "CHA" => Ok(Self::CHA),
            _ => Err(anyhow!(
                r#"Unexpected ability value {:?}, expected one of "STR", "DEX", "CON", "INT", "WIS", or "CHA""#,
                s
            )),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub enum AbilityBoost {
    Choice(smallvec::SmallVec<[Ability; 2]>),
    Fixed(Ability),
    Free,
}

try_from_str!(AbilityBoost);

impl FromStr for AbilityBoost {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(crate::parsers::ability_boost(s)?)
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub enum Alignment {
    LawfulGood,
    LawfulNeutral,
    LawfulEvil,
    NeutralGood,
    Neutral,
    NeutralEvil,
    ChaoticGood,
    ChaoticNeutral,
    ChaoticEvil,
}

try_from_str!(Alignment);

impl FromStr for Alignment {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Alignment> {
        match s {
            "LG" | "lawful good" | "Lawful Good" => Ok(Alignment::LawfulGood),
            "LN" | "lawfuln eutral" | "Lawful Neutral" => Ok(Alignment::LawfulNeutral),
            "LE" | "lawful evil" | "Lawful Evil" => Ok(Alignment::LawfulEvil),
            "NG" | "neutral good" | "Neutral Good" => Ok(Alignment::NeutralGood),
            "N" | "TN" | "neutral" | "true neutral" | "Neutral" | "True Neutral" => {
                Ok(Alignment::Neutral)
            }
            "NE" | "neutral evil" | "Neutral Evil" => Ok(Alignment::NeutralEvil),
            "CG" | "chaotic good" | "Chaotic Good" => Ok(Alignment::ChaoticGood),
            "CN" | "chaotic neutral" | "Chaotic Neutral" => Ok(Alignment::ChaoticNeutral),
            "CE" | "chaotic evil" | "Chaotic Evil" => Ok(Alignment::ChaoticEvil),
            other => Err(anyhow!("Unknown alignment string {:?}", other)),
        }
    }
}

impl fmt::Display for Alignment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self, f.alternate()) {
            (Self::LawfulGood, false) => write!(f, "lawful good"),
            (Self::LawfulGood, true) => write!(f, "LG"),
            (Self::LawfulNeutral, false) => write!(f, "lawful neutral"),
            (Self::LawfulNeutral, true) => write!(f, "LN"),
            (Self::LawfulEvil, false) => write!(f, "lawful evil"),
            (Self::LawfulEvil, true) => write!(f, "LE"),
            (Self::NeutralGood, false) => write!(f, "neutral good"),
            (Self::NeutralGood, true) => write!(f, "NG"),
            (Self::Neutral, false) => write!(f, "true neutral"),
            (Self::Neutral, true) => write!(f, "N"),
            (Self::NeutralEvil, false) => write!(f, "neutral evil"),
            (Self::NeutralEvil, true) => write!(f, "NE"),
            (Self::ChaoticGood, false) => write!(f, "chaotic good"),
            (Self::ChaoticGood, true) => write!(f, "CG"),
            (Self::ChaoticNeutral, false) => write!(f, "chaotic neutral"),
            (Self::ChaoticNeutral, true) => write!(f, "CN"),
            (Self::ChaoticEvil, false) => write!(f, "chaotic evil"),
            (Self::ChaoticEvil, true) => write!(f, "CE"),
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub enum Bulk {
    Negligable,
    Light,
    Heavy(u16),
}

impl Default for Bulk {
    fn default() -> Self {
        Self::Negligable
    }
}

try_from_str!(Bulk);

impl FromStr for Bulk {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "-" => Ok(Self::Negligable),
            "L" => Ok(Self::Light),
            n => Ok(Self::Heavy(n.parse()?)),
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub struct Gold {
    total_copper: u32,
}

try_from_str!(Gold);

impl FromStr for Gold {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        lazy_static! {
            static ref GOLD_REGEX: regex::Regex =
                regex::Regex::new("^(?P<n>[1-9][0-9]*) ?(?P<type>[csg])p$").unwrap();
        }
        let s = s.trim();
        if s == "-" {
            return Ok(Gold::zero());
        }
        match GOLD_REGEX.captures(s) {
            Some(caps) => match &caps["type"] {
                "c" => Ok(Self::cp(caps["n"].parse()?)),
                "s" => Ok(Self::sp(caps["n"].parse()?)),
                "g" => Ok(Self::gp(caps["n"].parse()?)),
                _ => unreachable!(),
            },
            None => Err(anyhow!(
                "Invalid gold value, expected either \"-\" or a number followed by gp, sp, or cp."
            )),
        }
    }
}

impl Gold {
    pub fn zero() -> Self {
        Self { total_copper: 0 }
    }

    pub fn cp(n: u32) -> Self {
        Self { total_copper: n }
    }

    pub fn sp(n: u32) -> Self {
        Self {
            total_copper: n * 10,
        }
    }

    pub fn gp(n: u32) -> Self {
        Self {
            total_copper: n * 100,
        }
    }

    pub fn pp(n: u32) -> Self {
        Self {
            total_copper: n * 1000,
        }
    }

    #[inline]
    pub fn copper_part(&self) -> u32 {
        self.total_copper % 10
    }

    #[inline]
    pub fn silver_part(&self) -> u32 {
        (self.total_copper / 10) % 10
    }

    #[inline]
    pub fn gold_part(&self) -> u32 {
        (self.total_copper / 100) % 10
    }

    #[inline]
    pub fn platinum_part(&self) -> u32 {
        self.total_copper / 1000
    }
}

impl fmt::Display for Gold {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut platinum = self.platinum_part();
        let mut gold = self.gold_part();
        let copper = self.copper_part();
        let silver = self.silver_part();

        if platinum > 0 && platinum <= 10 && gold != 0 {
            gold += 10 * (platinum % 10);
            platinum /= 10;
        }

        match (platinum, gold, silver, copper) {
            (0, 0, 0, 0) => write!(f, "-"),
            (0, 0, 0, c) => write!(f, "{} cp", c),
            (0, 0, s, 0) => write!(f, "{} sp", s),
            (0, 0, s, c) => write!(f, "{} cp", (s * 10) + c),
            (0, g, 0, 0) => write!(f, "{} gp", g),
            (0, g, s, 0) => write!(f, "{} sp", (g * 10) + s),
            (0, g, s, c) => write!(f, "{} gp {} cp", g, (10 * s) + c),
            (p, 0, 0, 0) => write!(f, "{} pp", p),
            (p, 0, 0, c) => write!(f, "{} pp {} cp", p, c),
            (p, 0, s, 0) => write!(f, "{} pp {} sp", p, s),
            (p, 0, s, c) => write!(f, "{} pp {} cp", p, (s * 10) + c),
            (p, g, 0, 0) => write!(f, "{} gp", (p * 10) + g),
            (p, g, s, 0) => write!(f, "{} gp {} sp", (p * 10) + g, s),
            (p, g, s, c) => write!(f, "{} gp {} cp", (p * 10) + g, (10 * s) + c),
        }
    }
}

impl std::ops::Add for Gold {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            total_copper: self.total_copper + other.total_copper,
        }
    }
}

#[test]
fn test_gold_display() {
    assert_eq!(&format!("{}", Gold::cp(0)), "-");
    assert_eq!(&format!("{}", Gold::cp(1)), "1 cp");
    assert_eq!(&format!("{}", Gold::cp(10)), "1 sp");
    assert_eq!(&format!("{}", Gold::sp(1)), "1 sp");
    assert_eq!(&format!("{}", Gold::cp(12)), "12 cp");
    assert_eq!(&format!("{}", Gold::cp(100)), "1 gp");
    assert_eq!(&format!("{}", Gold::sp(10)), "1 gp");
    assert_eq!(&format!("{}", Gold::gp(1)), "1 gp");
    assert_eq!(&format!("{}", Gold::cp(102)), "1 gp 2 cp");
    assert_eq!(&format!("{}", Gold::gp(10)), "1 pp");
    assert_eq!(&format!("{}", Gold::pp(1)), "1 pp");
    assert_eq!(&format!("{}", Gold::gp(12)), "12 gp");
    assert_eq!(&format!("{}", Gold::gp(12) + Gold::cp(3)), "12 gp 3 cp");
    assert_eq!(&format!("{}", Gold::gp(120) + Gold::cp(3)), "12 pp 3 cp");
    assert_eq!(&format!("{}", Gold::gp(120) + Gold::cp(34)), "12 pp 34 cp");
    assert_eq!(&format!("{}", Gold::gp(123) + Gold::cp(45)), "123 gp 45 cp");
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, Hash, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
#[serde(transparent)]
pub struct Level(u8);

impl Level {
    #[inline]
    pub fn get(self) -> u8 {
        self.0
    }
}

impl From<u8> for Level {
    #[inline]
    fn from(l: u8) -> Level {
        Level(l)
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub enum Proficiency {
    Untrained,
    Trained,
    Expert,
    Master,
    Legendary,
}

try_from_str!(Proficiency);

impl FromStr for Proficiency {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "u" | "untrained" => Ok(Self::Untrained),
            "t" | "trained" => Ok(Self::Untrained),
            "e" | "expert" => Ok(Self::Untrained),
            "m" | "master" => Ok(Self::Untrained),
            "l" | "legendary" => Ok(Self::Untrained),
            _ => Err(anyhow!("Unknown proficiency level {:?}", s)),
        }
    }
}

impl Default for Proficiency {
    fn default() -> Self {
        Proficiency::Untrained
    }
}

impl Proficiency {
    #[inline]
    pub fn bonus(self, level: Level) -> Bonus {
        Bonus::proficiency(self, level)
    }
}

pub trait ProvidesProficiency {
    fn get_proficiency_level(&self, character: &Character, p: &str) -> Proficiency;
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub enum Skill {
    Acrobatics,
    Arcana,
    Athletics,
    Crafting,
    Deception,
    Diplomacy,
    Intimidation,
    Lore(String),
    Medicine,
    Nature,
    Occultism,
    Performance,
    Religion,
    Society,
    Stealth,
    Survival,
    Thievery,
}

try_from_str!(Skill);

impl FromStr for Skill {
    type Err = Error;

    fn from_str(s: &str) -> Result<Skill> {
        Ok(crate::parsers::skill(s)?)
    }
}

impl Skill {
    pub fn base_ability(&self) -> Ability {
        use Ability::*;

        match self {
            Self::Acrobatics => DEX,
            Self::Arcana => INT,
            Self::Athletics => STR,
            Self::Crafting => INT,
            Self::Deception => CHA,
            Self::Diplomacy => CHA,
            Self::Intimidation => CHA,
            Self::Lore(_) => INT,
            Self::Medicine => WIS,
            Self::Nature => WIS,
            Self::Occultism => INT,
            Self::Performance => CHA,
            Self::Religion => WIS,
            Self::Society => INT,
            Self::Stealth => DEX,
            Self::Survival => WIS,
            Self::Thievery => DEX,
        }
    }
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub enum Size {
    Tiny,
    Small,
    Medium,
    Large,
}

try_from_str!(Size);

impl FromStr for Size {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "tiny" => Ok(Self::Tiny),
            "small" => Ok(Self::Small),
            "medium" => Ok(Self::Medium),
            "large" => Ok(Self::Large),
            other => Err(anyhow!("Unknown size {:?}", other)),
        }
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(transparent)]
pub struct Range(pub u16);

impl FromStr for Range {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let x = s.parse()?;
        Ok(Range(x))
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ft.", self.0)
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub enum Die {
    D4,
    D6,
    D8,
    D10,
    D12,
    D20,
}

impl fmt::Display for Die {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::D4 => write!(f, "d4"),
            Self::D6 => write!(f, "d6"),
            Self::D8 => write!(f, "d8"),
            Self::D10 => write!(f, "d10"),
            Self::D12 => write!(f, "d12"),
            Self::D20 => write!(f, "d20"),
        }
    }
}

try_from_str!(Die);

impl FromStr for Die {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "d4" => Ok(Self::D4),
            "d6" => Ok(Self::D6),
            "d8" => Ok(Self::D8),
            "d10" => Ok(Self::D10),
            "d12" => Ok(Self::D12),
            "d20" => Ok(Self::D20),
            _ => Err(anyhow!("Unknown die size {:?}", s)),
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub struct DieRoll {
    pub count: u16,
    pub size: Die,
}

impl fmt::Display for DieRoll {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.count, self.size)
    }
}

try_from_str!(DieRoll);

impl FromStr for DieRoll {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut count_end = 0;
        for (i, c) in s.char_indices() {
            count_end = i;
            if !c.is_digit(10) {
                break;
            }
        }
        if count_end == 0 {
            return Err(anyhow!("Expected a number when parsing a die roll"));
        }

        let count = s[..count_end].parse()?;
        let size = s[count_end..].parse()?;
        Ok(DieRoll { count, size })
    }
}

#[test]
fn test_parse_die_roll() {
    assert_eq!(
        "1d20".parse::<DieRoll>().unwrap(),
        DieRoll {
            count: 1,
            size: Die::D20
        }
    );
    assert_eq!(
        "10d6".parse::<DieRoll>().unwrap(),
        DieRoll {
            count: 10,
            size: Die::D6
        }
    );
    assert!("-10d6".parse::<DieRoll>().is_err());
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub enum DamageType {
    B,
    P,
    S,
}

try_from_str!(DamageType);

impl FromStr for DamageType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "bludgeoning" | "B" => Ok(Self::B),
            "piercing" | "P" => Ok(Self::P),
            "slashing" | "S" => Ok(Self::S),
            _ => Err(anyhow!("Unknown damage type {:?}", s)),
        }
    }
}

impl fmt::Display for DamageType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self, f.alternate()) {
            (Self::B, false) => write!(f, "bludgeoning"),
            (Self::B, true) => write!(f, "B"),
            (Self::P, false) => write!(f, "piercing"),
            (Self::P, true) => write!(f, "P"),
            (Self::S, false) => write!(f, "slashing"),
            (Self::S, true) => write!(f, "S"),
        }
    }
}
