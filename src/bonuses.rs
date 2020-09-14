use anyhow::{anyhow, Error, Result};
use serde::{
    de::{self, Deserializer, Visitor},
    Deserialize,
};
use smartstring::alias::String;
use std::{collections::HashSet, fmt, ops, str::FromStr};

use crate::{
    resources::{ArmorCategory, Character, WeaponCategory},
    stats::{Ability, Level, Proficiency, Skill},
    try_from_str,
};

#[derive(Copy, Clone, Debug, Default, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub struct Bonus {
    circumstance: u16,
    item: u16,
    proficiency: (Proficiency, Level),
    status: u16,
    untyped: i16,
}

try_from_str!(Bonus);

impl FromStr for Bonus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        macro_rules! parse_tail {
            ($tail:literal, $method:ident) => {
                if s.ends_with($tail) {
                    let head = &s[..s.len() - $tail.len()];
                    return Ok(Self::$method(head.parse()?));
                }
            };
        }

        parse_tail!(" circumstance", circumstance);
        parse_tail!(" item", item);
        parse_tail!(" status", status);
        if let Ok(i) = i16::from_str(s) {
            return Ok(Self::untyped(i));
        }
        Err(anyhow!("Failed to parse bonus value {:?}", s))
    }
}

impl Bonus {
    pub fn none() -> Bonus {
        Self::default()
    }

    pub fn circumstance(bonus: u16) -> Bonus {
        Bonus {
            circumstance: bonus,
            ..Default::default()
        }
    }

    pub fn item(bonus: u16) -> Bonus {
        Bonus {
            item: bonus,
            ..Default::default()
        }
    }

    pub fn proficiency(p: Proficiency, level: Level) -> Bonus {
        Bonus {
            proficiency: (p, level),
            ..Default::default()
        }
    }

    pub fn status(bonus: u16) -> Bonus {
        Bonus {
            status: bonus,
            ..Default::default()
        }
    }

    pub fn untyped(bonus: i16) -> Bonus {
        Bonus {
            untyped: bonus,
            ..Default::default()
        }
    }

    pub fn total(&self) -> i16 {
        let (p, l) = self.proficiency;
        let l = l.get() as i16;
        let p_bonus = match (p, l) {
            (Proficiency::Untrained, _) => 0,
            (Proficiency::Trained, l) => l + 2,
            (Proficiency::Expert, l) => l + 4,
            (Proficiency::Master, l) => l + 6,
            (Proficiency::Legendary, l) => l + 8,
        };
        self.circumstance as i16 + self.item as i16 + p_bonus + self.status as i16 + self.untyped
    }
}

impl fmt::Display for Bonus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:+}", self.total())
    }
}

impl From<(Proficiency, Level)> for Bonus {
    fn from((p, level): (Proficiency, Level)) -> Bonus {
        p.bonus(level)
    }
}

impl ops::Add<Bonus> for Bonus {
    type Output = Bonus;

    fn add(self, other: Self) -> Self {
        Bonus {
            // most bonuses only take the highest, but...
            circumstance: self.circumstance.max(other.circumstance),
            item: self.item.max(other.item),
            proficiency: (
                self.proficiency.0.max(other.proficiency.0),
                self.proficiency.1.max(other.proficiency.1),
            ),
            status: self.status.max(other.status),
            // ...untyped bonuses stack with each other.
            untyped: self.untyped + other.untyped,
        }
    }
}

impl ops::AddAssign for Bonus {
    fn add_assign(&mut self, other: Self) {
        self.circumstance = self.circumstance.max(other.circumstance);
        self.item = self.item.max(other.item);
        self.proficiency.0 = self.proficiency.0.max(other.proficiency.0);
        self.proficiency.1 = self.proficiency.1.max(other.proficiency.1);
        self.status = self.status.max(other.status);
        self.untyped += other.untyped;
    }
}

impl ops::Mul<Level> for Bonus {
    type Output = Bonus;

    fn mul(self, level: Level) -> Bonus {
        let level = level.get() as u16;
        Bonus {
            circumstance: self.circumstance * level,
            item: self.item * level,
            proficiency: self.proficiency,
            status: self.status * level,
            untyped: self.untyped * level as i16,
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Penalty {
    circumstance: u16,
    item: u16,
    status: u16,
    untyped: u16,
}

try_from_str!(Penalty);

impl FromStr for Penalty {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        macro_rules! parse_tail {
            ($tail:literal, $method:ident) => {
                if s.ends_with($tail) {
                    let head = &s[..s.len() - $tail.len()];
                    let mut value = head.parse::<i16>()?;
                    if value < 0 {
                        value = -value;
                    }
                    return Ok(Self::$method(value as u16));
                }
            };
        }

        parse_tail!(" circumstance", circumstance);
        parse_tail!(" item", item);
        parse_tail!(" status", status);
        if let Ok(mut i) = i16::from_str(s) {
            if i < 0 {
                i = -i;
            }
            return Ok(Self::untyped(i as u16));
        }
        Err(anyhow!("Failed to parse penalty value {:?}", s))
    }
}

impl<'de> Deserialize<'de> for Penalty {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PenaltyVisitor;

        macro_rules! passthru_number {
	    ($t:ty $(as $dest_t:ty)*, $name:ident) => {
		fn $name<E>(self, i: $t) -> Result<Penalty, E>
		where
		    E: de::Error,
		{
		    self.visit_i16(i $(as $dest_t)* as i16)
		}
	    }
	}

        impl<'de> Visitor<'de> for PenaltyVisitor {
            type Value = Penalty;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "an integer or a string of an integer followed by one of circumstance, item, or status")
            }

            passthru_number!(i64 as i32, visit_i64);
            passthru_number!(i32, visit_i32);
            passthru_number!(u64 as i64 as i32, visit_u64);
            passthru_number!(u32 as i32, visit_u32);

            fn visit_i16<E>(self, mut i: i16) -> Result<Penalty, E>
            where
                E: de::Error,
            {
                if i < 0 {
                    i = -i;
                }
                Ok(Penalty::untyped(i as u16))
            }

            fn visit_str<E>(self, s: &str) -> Result<Penalty, E>
            where
                E: de::Error,
            {
                Penalty::from_str(s).map_err(E::custom)
            }
        }

        deserializer.deserialize_any(PenaltyVisitor)
    }
}

impl Penalty {
    pub fn none() -> Penalty {
        Self::default()
    }

    pub fn circumstance(bonus: u16) -> Penalty {
        Penalty {
            circumstance: bonus,
            ..Default::default()
        }
    }

    pub fn item(bonus: u16) -> Penalty {
        Penalty {
            item: bonus,
            ..Default::default()
        }
    }

    pub fn status(bonus: u16) -> Penalty {
        Penalty {
            status: bonus,
            ..Default::default()
        }
    }

    pub fn untyped(bonus: u16) -> Penalty {
        Penalty {
            untyped: bonus,
            ..Default::default()
        }
    }

    fn total(&self) -> i16 {
        -(self.circumstance as i16 + self.item as i16 + self.status as i16 + self.untyped as i16)
    }
}

impl fmt::Display for Penalty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.total().fmt(f)
    }
}

impl ops::Add for Penalty {
    type Output = Penalty;

    fn add(self, other: Self) -> Self {
        Penalty {
            circumstance: self.circumstance + other.circumstance,
            item: self.item + other.item,
            status: self.status + other.status,
            untyped: self.untyped + other.untyped,
        }
    }
}

impl ops::AddAssign for Penalty {
    fn add_assign(&mut self, other: Self) {
        self.circumstance += other.circumstance;
        self.item += other.item;
        self.status += other.status;
        self.untyped += other.untyped;
    }
}

impl ops::Add<Bonus> for Penalty {
    type Output = Modifier;

    fn add(self, bonus: Bonus) -> Modifier {
        (bonus, self).into()
    }
}

impl ops::Add<Penalty> for Bonus {
    type Output = Modifier;

    fn add(self, penalty: Penalty) -> Modifier {
        (self, penalty).into()
    }
}

impl ops::Mul<Level> for Penalty {
    type Output = Penalty;

    fn mul(self, level: Level) -> Penalty {
        let level = level.get() as u16;
        Penalty {
            circumstance: self.circumstance * level,
            item: self.item * level,
            status: self.status * level,
            untyped: self.untyped * level,
        }
    }
}

/// TODO: I'd like to be generic over things like skills, abilities,
/// and resistances, where it would be impractical to scan over all
/// possibilities (especially when they include things like strings,
/// which could be (almost) infinitely long). This `IndexedModifier`
/// trait is sort of what I'm aiming at, but I don't think this
/// implementation is actually usable.
pub trait IndexedModifier: Copy + Clone + fmt::Debug + std::hash::Hash + Eq {
    type Index: Clone + fmt::Debug + std::hash::Hash + Eq;
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub enum Modifies {
    Ability(Ability),
    AC,
    ArmorCategory(ArmorCategory),
    Attack,
    ClassDC,
    FortitudeSave,
    HP,
    Perception,
    ReflexSave,
    Resistance(String),
    Skill(Skill),
    Speed,
    WillSave,
    WeaponCategory(WeaponCategory),
}

static_assertions::assert_eq_size!(Modifies, [u8; 40]);

try_from_str!(Modifies);

impl FromStr for Modifies {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        if let Ok(a) = Ability::from_str(s) {
            return Ok(Self::Ability(a));
        }
        if let Ok(a) = ArmorCategory::from_str(s) {
            return Ok(Self::ArmorCategory(a));
        }
        if s.contains("resistance") || s.contains("weakness") {
            return Err(anyhow!(
                "TODO: FromStr for Modifies w.r.t. resistances, s = {:?}",
                s
            ));
        }
        if let Ok(s) = Skill::from_str(s) {
            return Ok(Self::Skill(s));
        }
        if let Ok(w) = WeaponCategory::from_str(s) {
            return Ok(Self::WeaponCategory(w));
        }
        match s {
            "ac" | "AC" => Ok(Self::AC),
            "attack" | "Attack" => Ok(Self::Attack),
            "class dc" | "class DC" | "Class DC" | "ClassDC" => Ok(Self::ClassDC),
            "fort" | "fortitude" | "fort save" | "fortitude save" => Ok(Self::FortitudeSave),
            "hp" | "HP" => Ok(Self::HP),
            "perception" | "Perception" => Ok(Self::Perception),
            "ref" | "reflex" | "ref save" | "reflex save" => Ok(Self::ReflexSave),
            "speed" | "Speed" => Ok(Self::Speed),
            "will" | "will save" => Ok(Self::WillSave),
            _ => Err(anyhow!("Unknown modifier type {:?}", s)),
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Debug)]
pub struct Score<'a> {
    modifier: &'a Modifier,
}

impl fmt::Display for Score<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.modifier.total())
    }
}

#[derive(Clone, Debug)]
pub struct Modifier {
    bonus: Bonus,
    penalty: Penalty,
}

impl<'de> Deserialize<'de> for Modifier {
    fn deserialize<D>(deserializer: D) -> Result<Modifier, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ModifierVisitor;

        impl<'de> Visitor<'de> for ModifierVisitor {
            type Value = Modifier;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    f,
                    "a string containing number followed by an optional bonus/penalty type"
                )
            }

            fn visit_i16<E: de::Error>(self, value: i16) -> Result<Modifier, E> {
                if value < 0 {
                    Ok(Penalty::untyped((-value) as u16).into())
                } else {
                    Ok(Bonus::untyped(value).into())
                }
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Modifier, E> {
                if v.starts_with("-") {
                    match Penalty::from_str(v) {
                        Ok(p) => Ok(p.into()),
                        Err(e) => Err(E::custom(e)),
                    }
                } else {
                    match Bonus::from_str(v) {
                        Ok(b) => Ok(b.into()),
                        Err(e) => Err(E::custom(e)),
                    }
                }
            }
        }

        deserializer.deserialize_str(ModifierVisitor)
    }
}

impl Modifier {
    pub fn new() -> Self {
        Self {
            bonus: Bonus::default(),
            penalty: Penalty::default(),
        }
    }

    pub fn total(&self) -> i16 {
        let bonus = self.bonus.total();
        let penalty = self.penalty.total();
        bonus + penalty
    }

    pub fn item_part(&self) -> Self {
        Self {
            bonus: Bonus::item(self.bonus.item),
            penalty: Penalty::item(self.penalty.item),
        }
    }

    pub fn proficiency_part(&self) -> (Self, Proficiency) {
        let (p, level) = self.bonus.proficiency;
        let m = Self {
            bonus: Bonus::proficiency(p, level),
            penalty: Penalty::none(),
        };
        (m, p)
    }

    pub fn as_score(&self) -> Score {
        Score { modifier: self }
    }
}

impl From<Bonus> for Modifier {
    fn from(bonus: Bonus) -> Modifier {
        Modifier {
            bonus,
            penalty: Penalty::none(),
        }
    }
}

impl From<Penalty> for Modifier {
    fn from(penalty: Penalty) -> Modifier {
        Modifier {
            bonus: Bonus::none(),
            penalty,
        }
    }
}

impl From<(Bonus, Penalty)> for Modifier {
    fn from((bonus, penalty): (Bonus, Penalty)) -> Modifier {
        Modifier { bonus, penalty }
    }
}

impl fmt::Display for Modifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:+}", self.total())
    }
}

impl ops::Add<Modifier> for Modifier {
    type Output = Modifier;

    fn add(self, other: Modifier) -> Modifier {
        Self {
            bonus: self.bonus + other.bonus,
            penalty: self.penalty + other.penalty,
        }
    }
}

impl ops::AddAssign<Modifier> for Modifier {
    fn add_assign(&mut self, other: Modifier) {
        self.bonus += other.bonus;
        self.penalty += other.penalty;
    }
}

impl ops::Add<Bonus> for Modifier {
    type Output = Modifier;

    fn add(self, bonus: Bonus) -> Modifier {
        self + Modifier::from(bonus)
    }
}

impl ops::AddAssign<Bonus> for Modifier {
    fn add_assign(&mut self, bonus: Bonus) {
        *self += Modifier::from(bonus);
    }
}

impl ops::Add<Penalty> for Modifier {
    type Output = Modifier;

    fn add(self, penalty: Penalty) -> Modifier {
        self + Modifier::from(penalty)
    }
}

impl ops::AddAssign<Penalty> for Modifier {
    fn add_assign(&mut self, penalty: Penalty) {
        *self += Modifier::from(penalty);
    }
}

impl ops::Add<Modifier> for Penalty {
    type Output = Modifier;

    fn add(self, m: Modifier) -> Modifier {
        m + Modifier::from(self)
    }
}

impl ops::Add<Modifier> for Bonus {
    type Output = Modifier;

    fn add(self, m: Modifier) -> Modifier {
        m + Modifier::from(self)
    }
}

impl ops::Add<(Bonus, Penalty)> for Modifier {
    type Output = Modifier;

    fn add(self, (b, p): (Bonus, Penalty)) -> Modifier {
        Modifier {
            bonus: self.bonus + b,
            penalty: self.penalty + p,
            ..self
        }
    }
}

impl ops::AddAssign<(Bonus, Penalty)> for Modifier {
    fn add_assign(&mut self, (b, p): (Bonus, Penalty)) {
        self.bonus += b;
        self.penalty += p;
    }
}

pub trait HasModifiers {
    fn get_modifier(&self, character: &Character, modifier: Modifies) -> Modifier;

    fn get_modified_abilities(&self, character: &Character) -> HashSet<Ability> {
        let _ = character;
        HashSet::new()
    }

    fn get_modified_resistances(&self, character: &Character) -> HashSet<String> {
        let _ = character;
        HashSet::new()
    }

    fn get_modified_skills(&self, character: &Character) -> HashSet<Skill> {
        let _ = character;
        HashSet::new()
    }
}
