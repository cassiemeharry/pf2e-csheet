use smartstring::alias::String;
use std::{collections::HashSet, fmt, ops};

use crate::{Ability, ArmorCategory, Character, Proficiency, Skill, WeaponCategory};

#[derive(Copy, Clone, Debug, Default)]
pub struct Bonus {
    circumstance: u16,
    item: u16,
    proficiency: (Proficiency, u8),
    status: u16,
    untyped: i16,
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

    pub fn proficiency(p: Proficiency, level: u8) -> Bonus {
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

    pub fn as_modifier(self, modifies: Modifies) -> Modifier {
        Modifier {
            modifies,
            bonus: self,
            penalty: Penalty::default(),
        }
    }

    pub fn total(&self) -> i16 {
        let p = match self.proficiency {
            (Proficiency::Untrained, _) => 0,
            (Proficiency::Trained, l) => (l as i16) + 2,
            (Proficiency::Expert, l) => (l as i16) + 4,
            (Proficiency::Master, l) => (l as i16) + 6,
            (Proficiency::Legendary, l) => (l as i16) + 8,
        };
        self.circumstance as i16 + self.item as i16 + p + self.status as i16 + self.untyped
    }
}

impl fmt::Display for Bonus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:+}", self.total())
    }
}

impl From<(Proficiency, u8)> for Bonus {
    fn from((p, level): (Proficiency, u8)) -> Bonus {
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

impl ops::Mul<u8> for Bonus {
    type Output = Bonus;

    fn mul(self, level: u8) -> Bonus {
        Bonus {
            circumstance: self.circumstance * level as u16,
            item: self.item * level as u16,
            proficiency: self.proficiency,
            status: self.status * level as u16,
            untyped: self.untyped * level as u16 as i16,
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

impl ops::Mul<u8> for Penalty {
    type Output = Penalty;

    fn mul(self, level: u8) -> Penalty {
        Penalty {
            circumstance: self.circumstance * level as u16,
            item: self.item * level as u16,
            status: self.status * level as u16,
            untyped: self.untyped * level as u16,
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

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
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
    modifies: Modifies,
    bonus: Bonus,
    penalty: Penalty,
}

impl Modifier {
    pub fn new(modifies: Modifies) -> Self {
        Self {
            modifies,
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
            modifies: self.modifies.clone(),
            bonus: Bonus::item(self.bonus.item),
            penalty: Penalty::item(self.penalty.item),
        }
    }

    pub fn proficiency_part(&self) -> (Self, Proficiency) {
        let (p, level) = self.bonus.proficiency;
        let m = Self {
            modifies: self.modifies.clone(),
            bonus: Bonus::proficiency(p, level),
            penalty: Penalty::none(),
        };
        (m, p)
    }

    pub fn as_score(&self) -> Score {
        Score { modifier: self }
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
        debug_assert_eq!(self.modifies, other.modifies);
        Self {
            modifies: self.modifies,
            bonus: self.bonus + other.bonus,
            penalty: self.penalty + other.penalty,
        }
    }
}

impl ops::AddAssign<Modifier> for Modifier {
    fn add_assign(&mut self, other: Modifier) {
        debug_assert_eq!(self.modifies, other.modifies);
        self.bonus += other.bonus;
        self.penalty += other.penalty;
    }
}

impl ops::Add<Bonus> for Modifier {
    type Output = Modifier;

    fn add(self, bonus: Bonus) -> Modifier {
        Self {
            bonus: self.bonus + bonus,
            ..self
        }
    }
}

impl ops::AddAssign<Bonus> for Modifier {
    fn add_assign(&mut self, bonus: Bonus) {
        self.bonus += bonus;
    }
}

impl ops::Add<Penalty> for Modifier {
    type Output = Modifier;

    fn add(self, penalty: Penalty) -> Modifier {
        Self {
            penalty: self.penalty + penalty,
            ..self
        }
    }
}

impl ops::AddAssign<Penalty> for Modifier {
    fn add_assign(&mut self, penalty: Penalty) {
        self.penalty += penalty;
    }
}

impl ops::Add<Modifier> for Penalty {
    type Output = Modifier;

    fn add(self, m: Modifier) -> Modifier {
        m + self
    }
}

impl ops::Add<Modifier> for Bonus {
    type Output = Modifier;

    fn add(self, m: Modifier) -> Modifier {
        m + self
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
