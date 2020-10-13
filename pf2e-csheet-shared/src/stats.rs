use lazy_static::lazy_static;
#[cfg(test)]
use proptest::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use std::{fmt, str::FromStr};
use thiserror::Error;

use crate::bonuses::Bonus;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
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

#[derive(Clone, Error, Debug, Deserialize, Serialize)]
pub enum AbilityFromStrError {
    #[error("Unknown ability value {0:?}, expected one of \"STR\", \"DEX\", \"CON\", \"INT\", \"WIS\", or \"CHA\"")]
    Unexpected(String),
}

impl FromStr for Ability {
    type Err = AbilityFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match crate::parsers::ability(s) {
            Ok(a) => Ok(a),
            Err(e) => {
                error!("Failed to parse ability value from {:?}: {}", s, e);
                Err(AbilityFromStrError::Unexpected(s.into()))
            }
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
#[serde(try_from = "smartstring::alias::String")]
pub enum AbilityBoost {
    Choice(
        #[cfg_attr(
            test,
            proptest(
                strategy = "proptest::collection::vec(any::<Ability>(), 0..=6).prop_map_into()"
            )
        )]
        smallvec::SmallVec<[Ability; 2]>,
    ),
    Fixed(Ability),
    Free,
}

try_from_str!(AbilityBoost);

#[derive(Clone, Error, Debug, Deserialize, Serialize)]
pub enum AbilityBoostFromStrError {
    #[error("Failed to parse ability boost")]
    Invalid,
}

impl FromStr for AbilityBoost {
    type Err = AbilityBoostFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(crate::parsers::ability_boost(s).map_err(|_| AbilityBoostFromStrError::Invalid)?)
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
#[serde(try_from = "smartstring::alias::String")]
pub enum Alignment {
    #[serde(rename = "LG")]
    #[serde(alias = "lawful good")]
    #[serde(alias = "Lawful Good")]
    LawfulGood,
    #[serde(rename = "LN")]
    #[serde(alias = "lawfuln eutral")]
    #[serde(alias = "Lawful Neutral")]
    LawfulNeutral,
    #[serde(rename = "LE")]
    #[serde(alias = "lawful evil")]
    #[serde(alias = "Lawful Evil")]
    LawfulEvil,
    #[serde(rename = "NG")]
    #[serde(alias = "neutral good")]
    #[serde(alias = "Neutral Good")]
    NeutralGood,
    #[serde(rename = "N")]
    #[serde(alias = "TN")]
    #[serde(alias = "neutral")]
    #[serde(alias = "true neutral")]
    #[serde(alias = "Neutral")]
    #[serde(alias = "True Neutral")]
    Neutral,
    #[serde(rename = "NE")]
    #[serde(alias = "neutral evil")]
    #[serde(alias = "Neutral Evil")]
    NeutralEvil,
    #[serde(rename = "CG")]
    #[serde(alias = "chaotic good")]
    #[serde(alias = "Chaotic Good")]
    ChaoticGood,
    #[serde(rename = "CN")]
    #[serde(alias = "chaotic neutral")]
    #[serde(alias = "Chaotic Neutral")]
    ChaoticNeutral,
    #[serde(rename = "CE")]
    #[serde(alias = "chaotic evil")]
    #[serde(alias = "Chaotic Evil")]
    ChaoticEvil,
}

try_from_str!(Alignment);

#[derive(Clone, Error, Debug, Deserialize, Serialize)]
pub enum AlignmentFromStrError {
    #[error("Failed to parse alignment")]
    Invalid,
}

impl FromStr for Alignment {
    type Err = AlignmentFromStrError;

    fn from_str(s: &str) -> Result<Alignment, Self::Err> {
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
            _ => Err(AlignmentFromStrError::Invalid),
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

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
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

#[derive(Clone, Error, Debug, Deserialize, Serialize)]
pub enum BulkFromStrError {
    #[error("Failed to parse bulk")]
    Invalid,
}

impl FromStr for Bulk {
    type Err = BulkFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "-" => Ok(Self::Negligable),
            "L" => Ok(Self::Light),
            n => Ok(Self::Heavy(
                n.parse().map_err(|_| BulkFromStrError::Invalid)?,
            )),
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
#[serde(try_from = "smartstring::alias::String")]
pub struct Gold {
    total_copper: u32,
}

try_from_str!(Gold);

#[derive(Clone, Error, Debug, Deserialize, Serialize)]
pub enum GoldFromStrError {
    #[error("Failed to parse gold value number")]
    BadNumber,
    #[error("Invalid gold value, expected either \"-\" or a number followed by gp, sp, or cp.")]
    Invalid,
}

impl FromStr for Gold {
    type Err = GoldFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
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
                "c" => Ok(Self::cp(
                    caps["n"].parse().map_err(|_| GoldFromStrError::BadNumber)?,
                )),
                "s" => Ok(Self::sp(
                    caps["n"].parse().map_err(|_| GoldFromStrError::BadNumber)?,
                )),
                "g" => Ok(Self::gp(
                    caps["n"].parse().map_err(|_| GoldFromStrError::BadNumber)?,
                )),
                _ => unreachable!(),
            },
            None => Err(GoldFromStrError::Invalid),
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
#[derive(
    Copy, Clone, Debug, Default, Hash, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize,
)]
#[cfg_attr(test, derive(Arbitrary))]
#[serde(transparent)]
pub struct Level(#[cfg_attr(test, proptest(strategy = "1u8..=20u8"))] u8);

impl FromStr for Level {
    type Err = <u8 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Level, Self::Err> {
        let inner = u8::from_str(s)?;
        Ok(Level(inner))
    }
}

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

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
#[serde(try_from = "smartstring::alias::String")]
pub enum Proficiency {
    Untrained,
    Trained,
    Expert,
    Master,
    Legendary,
}

try_from_str!(Proficiency);

#[derive(Clone, Error, Debug, Deserialize, Serialize)]
pub enum ProficiencyFromStrError {
    #[error("Invalid proficiency level")]
    Invalid,
}

impl FromStr for Proficiency {
    type Err = ProficiencyFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[cfg(test)]
        println!("Parsing a proficiency value from {:?}", s);
        match s {
            "u" | "U" | "untrained" | "Untrained" => Ok(Self::Untrained),
            "t" | "T" | "trained" | "Trained" => Ok(Self::Trained),
            "e" | "E" | "expert" | "Expert" => Ok(Self::Expert),
            "m" | "M" | "master" | "Master" => Ok(Self::Master),
            "l" | "L" | "legendary" | "Legendary" => Ok(Self::Legendary),
            _ => Err(ProficiencyFromStrError::Invalid),
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

// pub trait ProvidesProficiency {
//     fn get_proficiency_level(&self, character: &Character, p: &str) -> Proficiency;
// }

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

#[cfg(test)]
#[test]
fn lore_skill_roundtrip() {
    let lore_topics = &["a", "topic", "this is a rather long lore topic"];
    for topic in lore_topics.iter().copied() {
        let skill_1 = Skill::Lore(topic.into());
        println!("skill_1: {:?}", skill_1);
        let serialized_1 = skill_1.to_string();
        println!(
            "serialized_1:\n--------------------\n{}\n--------------------",
            serialized_1
        );
        let skill_2: Skill = serialized_1.parse().unwrap();
        println!("skill_2: {:?}", skill_2);
        assert_eq!(skill_1, skill_2);
        let serialized_2 = skill_2.to_string();
        assert_eq!(serialized_1, serialized_2);
    }
}

#[cfg(test)]
impl Arbitrary for Skill {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        prop_oneof![
            Just(Self::Acrobatics),
            Just(Self::Arcana),
            Just(Self::Athletics),
            Just(Self::Crafting),
            Just(Self::Deception),
            Just(Self::Diplomacy),
            Just(Self::Intimidation),
            "[a-zA-Z0-9 -]+".prop_map(|topic| Self::Lore(topic.into())),
            Just(Self::Medicine),
            Just(Self::Nature),
            Just(Self::Occultism),
            Just(Self::Performance),
            Just(Self::Religion),
            Just(Self::Society),
            Just(Self::Stealth),
            Just(Self::Survival),
            Just(Self::Thievery),
        ]
        .boxed()
    }
}

// #[cfg(test)]
// mod arb_lore_subject {
//     struct ArbLoreSubject(String);
//
//     #[cfg(test)]
//     impl Arbitrary for ArbLoreSubject {
//         type Parameters = ();
//         type Strategy = BoxedStrategy<Self>;
//         fn arbitrary_with(_args: ()) -> Self::Strategy {
//         }
//     }
//     impl From<ArbLoreSubject> for super::String {
//         fn from(als: ArbLoreSubject) -> super::String { als.0 }
//     }
// }

try_from_str!(Skill);

#[derive(Clone, Error, Debug, Deserialize, Serialize)]
pub enum SkillFromStrError {
    #[error("Failed to parse skill")]
    Invalid,
}

impl FromStr for Skill {
    type Err = SkillFromStrError;

    fn from_str(s: &str) -> Result<Skill, Self::Err> {
        Ok(crate::parsers::skill(s).map_err(|_| SkillFromStrError::Invalid)?)
    }
}

serialize_display!(Skill);

impl fmt::Display for Skill {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Acrobatics => write!(f, "acrobatics"),
            Self::Arcana => write!(f, "arcana"),
            Self::Athletics => write!(f, "athletics"),
            Self::Crafting => write!(f, "crafting"),
            Self::Deception => write!(f, "deception"),
            Self::Diplomacy => write!(f, "diplomacy"),
            Self::Intimidation => write!(f, "intimidation"),
            Self::Lore(s) => write!(f, "lore ({})", s),
            Self::Medicine => write!(f, "medicine"),
            Self::Nature => write!(f, "nature"),
            Self::Occultism => write!(f, "occultism"),
            Self::Performance => write!(f, "performance"),
            Self::Religion => write!(f, "religion"),
            Self::Society => write!(f, "society"),
            Self::Stealth => write!(f, "stealth"),
            Self::Survival => write!(f, "survival"),
            Self::Thievery => write!(f, "thievery"),
        }
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

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
#[serde(try_from = "smartstring::alias::String")]
pub enum Size {
    Tiny,
    Small,
    Medium,
    Large,
}

try_from_str!(Size);

#[derive(Clone, Error, Debug, Deserialize, Serialize)]
pub enum SizeFromStrError {
    #[error("Failed to parse size")]
    Invalid,
}

impl FromStr for Size {
    type Err = SizeFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tiny" => Ok(Self::Tiny),
            "small" => Ok(Self::Small),
            "medium" => Ok(Self::Medium),
            "large" => Ok(Self::Large),
            _ => Err(SizeFromStrError::Invalid),
        }
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
#[serde(transparent)]
pub struct Range(pub u16);

#[derive(Clone, Error, Debug, Deserialize, Serialize)]
pub enum RangeFromStrError {
    #[error("Failed to parse range")]
    Invalid,
}

impl FromStr for Range {
    type Err = RangeFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let x = s.parse().map_err(|_| RangeFromStrError::Invalid)?;
        Ok(Range(x))
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ft.", self.0)
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
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

#[derive(Clone, Error, Debug, Deserialize, Serialize)]
pub enum DieFromStrError {
    #[error("Failed to parse die")]
    Invalid,
}

impl FromStr for Die {
    type Err = DieFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "d4" => Ok(Self::D4),
            "d6" => Ok(Self::D6),
            "d8" => Ok(Self::D8),
            "d10" => Ok(Self::D10),
            "d12" => Ok(Self::D12),
            "d20" => Ok(Self::D20),
            _ => Err(DieFromStrError::Invalid),
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
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

#[derive(Clone, Error, Debug, Deserialize, Serialize)]
pub enum DieRollFromStrError {
    #[error("Failed to parse die roll")]
    Invalid,
}

impl FromStr for DieRoll {
    type Err = DieRollFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut count_end = 0;
        for (i, c) in s.char_indices() {
            count_end = i;
            if !c.is_digit(10) {
                break;
            }
        }
        if count_end == 0 {
            return Err(DieRollFromStrError::Invalid);
        }

        let count = s[..count_end]
            .parse()
            .map_err(|_| DieRollFromStrError::Invalid)?;
        let size = s[count_end..]
            .parse()
            .map_err(|_| DieRollFromStrError::Invalid)?;
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

#[derive(Clone, Error, Debug, Deserialize, Serialize)]
pub enum DamageTypeFromStrError {
    #[error("Unknown damage type {0:?}")]
    Unexpected(String),
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
#[serde(try_from = "smartstring::alias::String")]
pub enum DamageType {
    B,
    P,
    S,
}

try_from_str!(DamageType);

impl FromStr for DamageType {
    type Err = DamageTypeFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bludgeoning" | "B" => Ok(Self::B),
            "piercing" | "P" => Ok(Self::P),
            "slashing" | "S" => Ok(Self::S),
            _ => Err(DamageTypeFromStrError::Unexpected(s.into())),
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
