#[cfg(test)]
use proptest::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;
use serde::{
    de::{self, Deserializer, Visitor},
    ser, Deserialize, Serialize,
};
use smallvec::{smallvec, SmallVec};
use std::{convert::TryInto as _, fmt, ops, str::FromStr};
use thiserror::Error;

use crate::stats::{Level, Proficiency};

#[derive(Clone, Error, Debug, Deserialize, Serialize)]
pub enum FromValueTypeError {
    #[error("`from_value_type` doesn't make sense with proficiency bonuses")]
    ProficiencyTypeError,
    // #[error("`from_value_type` doesn't make sense with unregonized bonus types (got {0:?})")]
    // UnrecognizedTypeError(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
#[serde(try_from = "smartstring::alias::String")]
pub enum BonusType {
    #[serde(rename = "circumstance")]
    Circumstance,
    #[serde(rename = "item")]
    Item,
    #[serde(rename = "proficiency")]
    Proficiency,
    #[serde(rename = "status")]
    Status,
    #[serde(rename = "untyped")]
    Untyped,
    // #[serde(rename = "other")]
    // Other(
    //     #[cfg_attr(
    //         test,
    //         proptest(strategy = "any::<std::string::String>().prop_map_into()")
    //     )]
    //     String,
    // ),
}

try_from_str!(BonusType);

#[derive(Clone, Debug, Error, Deserialize, Serialize)]
#[error("Failed to parse bonus type")]
pub struct BonusTypeFromStrError;

impl FromStr for BonusType {
    type Err = BonusTypeFromStrError;

    fn from_str(s: &str) -> Result<BonusType, Self::Err> {
        match crate::parsers::bonus_type(s) {
            Ok(bt) => Ok(bt),
            Err(e) => {
                error!("Failed to parse bonus type from {:?}:\n{}", s, e);
                Err(BonusTypeFromStrError)
            }
        }
    }
}

impl fmt::Display for BonusType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Circumstance => write!(f, "circumstance"),
            Self::Item => write!(f, "item"),
            Self::Proficiency => write!(f, "proficiency"),
            Self::Status => write!(f, "status"),
            Self::Untyped => write!(f, "untyped"),
        }
    }
}

#[derive(Clone, Debug, Default, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Bonus {
    #[cfg_attr(test, proptest(strategy = "0u16..=20u16"))]
    circumstance: u16,
    #[cfg_attr(test, proptest(strategy = "0u16..=20u16"))]
    item: u16,
    #[cfg_attr(test, proptest(strategy = "0u16..=20u16"))]
    proficiency: u16,
    #[cfg_attr(test, proptest(strategy = "0u16..=20u16"))]
    status: u16,
    #[cfg_attr(
        test,
        proptest(strategy = "proptest::collection::vec(1i16..=20i16, 0..=10).prop_map_into()")
    )]
    untyped: SmallVec<[i16; 1]>,
}

try_from_str!(Bonus);

#[derive(Copy, Clone, Debug, Error, Deserialize, Serialize)]
#[error("Failed to parse bonus value")]
pub struct BonusFromStrError;

impl FromStr for Bonus {
    type Err = BonusFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match crate::parsers::bonus(s) {
            Ok(b) => Ok(b),
            Err(e) => {
                error!("Failed to parse bonus from {:?}:\n{}", s, e);
                Err(BonusFromStrError)
            }
        }
    }
}

impl std::cmp::PartialEq for Bonus {
    fn eq(&self, other: &Self) -> bool {
        self.circumstance == other.circumstance
            && self.item == other.item
            && self.status == other.status
            && self.proficiency == other.proficiency
            && self.untyped.iter().sum::<i16>() == other.untyped.iter().sum::<i16>()
    }
}

macro_rules! visit_number {
    ($name:ident : $input:ty $( => $mid:ty )*) => {
        fn $name<E: de::Error>(self, value: $input) -> Result<Self::Value, E> {
            trace!("{}({:?}: {}) => visit_i16", stringify!($name), value, stringify!($input));
            $(
                let value: $mid = value.try_into().map_err(E::custom)?;
            )*
            let value = value.try_into().map_err(E::custom)?;
            self.visit_i16(value)
        }
    };
    ($name:ident : $input:ty $( as $mid:ty )+) => {
        fn $name<E: de::Error>(self, value: $input) -> Result<Self::Value, E> {
            trace!("{}({:?}: {}) => visit_i16", stringify!($name), value, stringify!($input));
            $(
                let value = value as $mid;
            )+
            let value = value.try_into().map_err(E::custom)?;
            self.visit_i16(value)
        }
    };
}
macro_rules! visit_numbers_to_i16 {
    () => {
        visit_number!(visit_i8: i8);
        // i16 deliberately skipped
        visit_number!(visit_i32: i32);
        visit_number!(visit_i64: i64);
        visit_number!(visit_i128: i128);
        visit_number!(visit_u8: u8 => i8);
        visit_number!(visit_u16: u16);
        visit_number!(visit_u32: u32 => i32);
        visit_number!(visit_u64: u64 => i64);
        visit_number!(visit_u128: u128 => i128);
        visit_number!(visit_f32: f32 as i32);
        visit_number!(visit_f64: f64 as i64);
    }
}

impl<'de> Deserialize<'de> for Bonus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BonusVisitor;

        impl<'de> Visitor<'de> for BonusVisitor {
            type Value = Bonus;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "an integer or a string of an integer followed by one of circumstance, item, or status")
            }

            fn visit_i16<E>(self, i: i16) -> Result<Bonus, E>
            where
                E: de::Error,
            {
                Ok(Bonus::untyped(i))
            }

            visit_numbers_to_i16!();

            fn visit_str<E>(self, s: &str) -> Result<Bonus, E>
            where
                E: de::Error,
            {
                Bonus::from_str(s).map_err(E::custom)
            }

            fn visit_map<M>(self, mut map: M) -> Result<Bonus, M::Error>
            where
                M: de::MapAccess<'de>,
            {
                let mut circumstance = None;
                let mut item = None;
                let mut proficiency = None;
                let mut status = None;
                let mut untyped = None;
                macro_rules! handle_field {
                    ($name:ident) => {{
                        if $name.is_some() {
                            return Err(de::Error::duplicate_field(stringify!($name)));
                        }
                        $name = Some(map.next_value()?);
                    }};
                }
                while let Some(key) = map.next_key()? {
                    match key {
                        "circumstance" => handle_field!(circumstance),
                        "item" => handle_field!(item),
                        "proficiency" => handle_field!(proficiency),
                        "status" => handle_field!(status),
                        "untyped" => handle_field!(untyped),
                        other => {
                            return Err(de::Error::unknown_field(
                                other,
                                &["circumstance", "item", "proficiency", "status", "untyped"],
                            ))
                        }
                    }
                }
                Ok(Bonus {
                    circumstance: circumstance.unwrap_or_default(),
                    item: item.unwrap_or_default(),
                    proficiency: proficiency.unwrap_or_default(),
                    status: status.unwrap_or_default(),
                    untyped: untyped.map(|u| smallvec![u]).unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_any(BonusVisitor)
    }
}

impl Serialize for Bonus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let untyped = self.untyped.iter().sum::<i16>();

        match (
            self.circumstance,
            self.item,
            self.proficiency,
            self.status,
            untyped,
        ) {
            (0, 0, 0, 0, 0) => serializer.serialize_i16(0),
            (c, 0, 0, 0, 0) => {
                serializer.serialize_str(&format!("{} {}", c, BonusType::Circumstance))
            }
            (0, i, 0, 0, 0) => serializer.serialize_str(&format!("{} {}", i, BonusType::Item)),
            (0, 0, p, 0, 0) => {
                serializer.serialize_str(&format!("{} {}", p, BonusType::Proficiency))
            }
            (0, 0, 0, s, 0) => serializer.serialize_str(&format!("{} {}", s, BonusType::Status)),
            (0, 0, 0, 0, u) => serializer.serialize_i16(u),
            (c, i, p, s, u) => {
                use serde::ser::SerializeMap as _;

                let mut map = serializer.serialize_map(None)?;
                if c != 0 {
                    map.serialize_entry("circumstance", &c)?;
                }
                if i != 0 {
                    map.serialize_entry("item", &i)?;
                }
                if p != 0 {
                    map.serialize_entry("proficiency", &p)?;
                }
                if s != 0 {
                    map.serialize_entry("status", &s)?;
                }
                if u != 0 {
                    map.serialize_entry("untyped", &u)?;
                }
                map.end()
            }
        }
    }
}

impl Bonus {
    pub fn none() -> Bonus {
        Self::default()
    }

    pub fn from_value_type(bonus: i16, bonus_type: &BonusType) -> Result<Self, FromValueTypeError> {
        match bonus_type {
            BonusType::Circumstance => Ok(Self::circumstance(bonus.max(0) as u16)),
            BonusType::Item => Ok(Self::item(bonus.max(0) as u16)),
            BonusType::Status => Ok(Self::status(bonus.max(0) as u16)),
            BonusType::Proficiency => Ok(Bonus {
                proficiency: bonus as u16,
                ..Bonus::default()
            }),
            BonusType::Untyped => Ok(Self::untyped(bonus)),
            // BonusType::Other(other) => {
            //     Err(FromValueTypeError::UnrecognizedTypeError(other.clone()))
            // }
        }
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
        let bonus = match (p, level.get() as u16) {
            (Proficiency::Untrained, _) => 0,
            (Proficiency::Trained, l) => l + 2,
            (Proficiency::Expert, l) => l + 4,
            (Proficiency::Master, l) => l + 6,
            (Proficiency::Legendary, l) => l + 8,
        };
        Bonus {
            proficiency: bonus,
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
        let mut b = Bonus::default();
        if bonus != 0 {
            b.untyped.push(bonus);
        }
        b
    }

    pub fn total(&self) -> i16 {
        let mut total = self.circumstance as i16
            + self.item as i16
            + self.proficiency as i16
            + self.status as i16;
        total += self.untyped.iter().copied().sum::<i16>();
        total
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

    fn add(mut self, other: Self) -> Self {
        self += other;
        self
    }
}

impl ops::AddAssign for Bonus {
    fn add_assign(&mut self, other: Self) {
        self.circumstance = self.circumstance.max(other.circumstance);
        self.item = self.item.max(other.item);
        self.proficiency = self.proficiency.max(other.proficiency);
        self.status = self.status.max(other.status);
        self.untyped.extend(other.untyped);
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
            untyped: self
                .untyped
                .into_iter()
                .map(|u| u * (level as i16))
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Penalty {
    #[cfg_attr(
        test,
        proptest(strategy = "proptest::collection::vec(-20i16..=-1i16, 0..=10).prop_map_into()")
    )]
    circumstance: SmallVec<[i16; 1]>,
    #[cfg_attr(
        test,
        proptest(strategy = "proptest::collection::vec(-20i16..=-1i16, 0..=10).prop_map_into()")
    )]
    item: SmallVec<[i16; 1]>,
    #[cfg_attr(
        test,
        proptest(strategy = "proptest::collection::vec(-20i16..=-1i16, 0..=10).prop_map_into()")
    )]
    status: SmallVec<[i16; 1]>,
    #[cfg_attr(
        test,
        proptest(strategy = "proptest::collection::vec(-20i16..=-1i16, 0..=10).prop_map_into()")
    )]
    untyped: SmallVec<[i16; 1]>,
}

try_from_str!(Penalty);

#[derive(Copy, Clone, Debug, Error, Deserialize, Serialize)]
#[error("Failed to parse penalty value")]
pub struct PenaltyFromStrError;

impl FromStr for Penalty {
    type Err = PenaltyFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match crate::parsers::penalty(s) {
            Ok(b) => Ok(b),
            Err(e) => {
                error!("Failed to parse penalty from {:?}:\n{}", s, e);
                Err(PenaltyFromStrError)
            }
        }
    }
}

impl std::cmp::PartialEq for Penalty {
    fn eq(&self, other: &Self) -> bool {
        macro_rules! sum_eq {
            ($field:ident) => {
                self.$field.iter().sum::<i16>() == other.$field.iter().sum::<i16>()
            };
        }
        sum_eq!(circumstance) && sum_eq!(item) && sum_eq!(status) && sum_eq!(untyped)
    }
}

impl<'de> Deserialize<'de> for Penalty {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PenaltyVisitor;

        impl<'de> Visitor<'de> for PenaltyVisitor {
            type Value = Penalty;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "an integer or a string of an integer followed by one of circumstance, item, or status")
            }

            fn visit_i16<E>(self, i: i16) -> Result<Penalty, E>
            where
                E: de::Error,
            {
                Ok(Penalty::untyped(i))
            }

            visit_numbers_to_i16!();

            fn visit_str<E>(self, s: &str) -> Result<Penalty, E>
            where
                E: de::Error,
            {
                Penalty::from_str(s).map_err(E::custom)
            }

            fn visit_map<M>(self, mut map: M) -> Result<Penalty, M::Error>
            where
                M: de::MapAccess<'de>,
            {
                let mut circumstance = None;
                let mut item = None;
                let mut status = None;
                let mut untyped = None;
                macro_rules! handle_field {
                    ($name:ident) => {{
                        if $name.is_some() {
                            return Err(de::Error::duplicate_field(stringify!($name)));
                        }
                        $name = Some(map.next_value()?);
                    }};
                }
                while let Some(key) = map.next_key()? {
                    match key {
                        "circumstance" => handle_field!(circumstance),
                        "item" => handle_field!(item),
                        "status" => handle_field!(status),
                        "untyped" => handle_field!(untyped),
                        other => {
                            return Err(de::Error::unknown_field(
                                other,
                                &["circumstance", "item", "status", "untyped"],
                            ))
                        }
                    }
                }
                Ok(Penalty {
                    circumstance: circumstance.map(|c| smallvec![c]).unwrap_or_default(),
                    item: item.map(|i| smallvec![i]).unwrap_or_default(),
                    status: status.map(|s| smallvec![s]).unwrap_or_default(),
                    untyped: untyped.map(|u| smallvec![u]).unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_any(PenaltyVisitor)
    }
}

impl Serialize for Penalty {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let circumstance = self.circumstance.iter().sum::<i16>();
        let item = self.item.iter().sum::<i16>();
        let status = self.status.iter().sum::<i16>();
        let untyped = self.untyped.iter().sum::<i16>();

        match (circumstance, item, status, untyped) {
            (0, 0, 0, 0) => serializer.serialize_i16(0),
            (c, 0, 0, 0) => serializer.serialize_str(&format!("{} {}", c, BonusType::Circumstance)),
            (0, i, 0, 0) => serializer.serialize_str(&format!("{} {}", i, BonusType::Item)),
            (0, 0, s, 0) => serializer.serialize_str(&format!("{} {}", s, BonusType::Status)),
            (0, 0, 0, u) => serializer.serialize_i16(u),
            (c, i, s, u) => {
                use serde::ser::SerializeMap as _;

                let mut map = serializer.serialize_map(None)?;
                if c != 0 {
                    map.serialize_entry("circumstance", &c)?;
                }
                if i != 0 {
                    map.serialize_entry("item", &i)?;
                }
                if s != 0 {
                    map.serialize_entry("status", &s)?;
                }
                if u != 0 {
                    map.serialize_entry("untyped", &u)?;
                }
                map.end()
            }
        }
    }
}

impl Penalty {
    pub fn none() -> Penalty {
        Self::default()
    }

    pub fn from_value_type(bonus: i16, bonus_type: &BonusType) -> Result<Self, FromValueTypeError> {
        match bonus_type {
            BonusType::Circumstance => Ok(Self::circumstance(bonus)),
            BonusType::Item => Ok(Self::item(bonus)),
            BonusType::Status => Ok(Self::status(bonus)),
            BonusType::Proficiency => Err(FromValueTypeError::ProficiencyTypeError),
            BonusType::Untyped => Ok(Self::untyped(bonus)),
            // BonusType::Other(other) => {
            //     Err(FromValueTypeError::UnrecognizedTypeError(other.clone()))
            // }
        }
    }

    pub fn circumstance(bonus: i16) -> Penalty {
        let mut p = Penalty::default();
        if bonus != 0 {
            p.circumstance.push(bonus);
        }
        p
    }

    pub fn item(bonus: i16) -> Penalty {
        let mut p = Penalty::default();
        if bonus != 0 {
            p.item.push(bonus);
        }
        p
    }

    pub fn status(bonus: i16) -> Penalty {
        let mut p = Penalty::default();
        if bonus != 0 {
            p.status.push(bonus);
        }
        p
    }

    pub fn untyped(bonus: i16) -> Penalty {
        let mut p = Penalty::default();
        if bonus != 0 {
            p.untyped.push(bonus);
        }
        p
    }

    fn total(&self) -> i16 {
        let mut total: i16 = 0;
        total += self.circumstance.iter().copied().sum::<i16>();
        total += self.item.iter().copied().sum::<i16>();
        total += self.status.iter().copied().sum::<i16>();
        total += self.untyped.iter().copied().sum::<i16>();
        total
    }
}

impl fmt::Display for Penalty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.total().fmt(f)
    }
}

impl ops::Add for Penalty {
    type Output = Penalty;

    fn add(mut self, other: Self) -> Self {
        self += other;
        self
    }
}

impl ops::AddAssign for Penalty {
    fn add_assign(&mut self, other: Self) {
        self.circumstance.extend(other.circumstance);
        self.item.extend(other.item);
        self.status.extend(other.status);
        self.untyped.extend(other.untyped);
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
        let level = level.get() as u16 as i16;
        Penalty {
            circumstance: self.circumstance.into_iter().map(|c| c * level).collect(),
            item: self.item.into_iter().map(|i| i * level).collect(),
            status: self.status.into_iter().map(|s| s * level).collect(),
            untyped: self.untyped.into_iter().map(|u| u * level).collect(),
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Debug)]
// #[cfg_attr(test, derive(Arbitrary))]
pub struct Score<'a> {
    modifier: &'a Modifier,
}

impl fmt::Display for Score<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.modifier.total())
    }
}

#[derive(Clone, Debug, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Modifier {
    bonus: Bonus,
    penalty: Penalty,
}

impl PartialEq for Modifier {
    fn eq(&self, other: &Self) -> bool {
        macro_rules! cmp {
            ($field:ident, scalar) => {
                (self.bonus.$field as i16 + cmp!(@iter self.penalty.$field)) == (other.bonus.$field as i16 + cmp!(@iter other.penalty.$field))
            };
            ($field:ident, iter) => {
                (cmp!(@iter self.bonus.$field) + cmp!(@iter self.penalty.$field)) == (cmp!(@iter other.bonus.$field) + cmp!(@iter other.penalty.$field))
            };
            // (@iter $e:expr => i16) => {
            //     ($e.iter().sum::<u16>() as i16)
            // };
            (@iter $e:expr) => {
                $e.iter().sum::<i16>()
            };
        }
        cmp!(circumstance, scalar)
            && cmp!(item, scalar)
            && self.bonus.proficiency == other.bonus.proficiency
            && cmp!(status, scalar)
            && cmp!(untyped, iter)
    }
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
                    "a bonus or penalty value, or a map with \"bonus\" and \"penalty\" keys"
                )
            }

            fn visit_i16<E: de::Error>(self, value: i16) -> Result<Modifier, E> {
                trace!("In ModifierVisitor::visit_i16");
                if value < 0 {
                    Ok(Penalty::untyped(value).into())
                } else {
                    Ok(Bonus::untyped(value).into())
                }
            }

            visit_numbers_to_i16!();

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Modifier, E> {
                trace!("In ModifierVisitor::visit_str");
                Modifier::from_str(v).map_err(E::custom)
            }

            fn visit_map<M>(self, mut map: M) -> Result<Modifier, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                trace!("In ModifierVisitor::visit_map");
                let mut bonus = None;
                let mut penalty = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        "bonus" => {
                            if bonus.is_some() {
                                return Err(de::Error::duplicate_field("bonus"));
                            }
                            bonus = Some(map.next_value()?);
                        }
                        "penalty" => {
                            if penalty.is_some() {
                                return Err(de::Error::duplicate_field("penalty"));
                            }
                            penalty = Some(map.next_value()?);
                        }
                        other => {
                            return Err(de::Error::unknown_field(other, &["bonus", "penalty"]))
                        }
                    }
                }
                let bonus = bonus.ok_or_else(|| de::Error::missing_field("bonus"))?;
                let penalty = penalty.ok_or_else(|| de::Error::missing_field("penalty"))?;
                Ok(Modifier { bonus, penalty })
            }
        }

        deserializer.deserialize_any(ModifierVisitor)
    }
}

impl Serialize for Modifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let b = self.bonus.total();
        let p = self.penalty.total();
        if b == 0 && p == 0 {
            serializer.serialize_i16(0)
        // } else if p == 0 {
        //     Bonus::serialize(&self.bonus, serializer)
        // } else if b == 0 {
        //     Penalty::serialize(&self.penalty, serializer)
        } else {
            // If both are given, serialize a map
            use serde::ser::SerializeMap as _;
            let mut map = serializer.serialize_map(Some(2))?;
            map.serialize_entry("bonus", &self.bonus)?;
            map.serialize_entry("penalty", &self.penalty)?;
            map.end()
        }
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
            bonus: Bonus {
                item: self.bonus.item.clone(),
                ..Bonus::default()
            },
            penalty: Penalty {
                item: self.penalty.item.clone(),
                ..Penalty::default()
            },
        }
    }

    pub fn proficiency_part(&self) -> Self {
        Self {
            bonus: Bonus {
                proficiency: self.bonus.proficiency,
                ..Bonus::default()
            },
            penalty: Penalty::none(),
        }
    }

    pub fn bonus_part(&self) -> Bonus {
        self.bonus.clone()
    }

    pub fn penalty_part(&self) -> Penalty {
        self.penalty.clone()
    }

    pub fn as_score(&self) -> Score {
        Score { modifier: self }
    }
}

#[derive(Copy, Clone, Debug, Error, Deserialize, Serialize)]
#[error("Failed to parse modifier value")]
pub struct ModifierFromStrError;

impl FromStr for Modifier {
    type Err = ModifierFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match crate::parsers::modifier(s) {
            Ok(b) => Ok(b),
            Err(e) => {
                error!("Failed to parse modifier from {:?}:\n{}", s, e);
                Err(ModifierFromStrError)
            }
        }
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
        let c = (self.bonus.circumstance as i16) + self.penalty.circumstance.iter().sum::<i16>();
        let i = (self.bonus.item as i16) + self.penalty.item.iter().sum::<i16>();
        let p = self.bonus.proficiency as i16;
        let s = (self.bonus.status as i16) + self.penalty.status.iter().sum::<i16>();
        let u = self.bonus.untyped.iter().sum::<i16>() + self.penalty.untyped.iter().sum::<i16>();
        match (c, i, p, s, u) {
            (0, 0, 0, 0, 0) => write!(f, "0"),
            (c, 0, 0, 0, 0) => write!(f, "{} {}", c, BonusType::Circumstance),
            (0, i, 0, 0, 0) => write!(f, "{} {}", i, BonusType::Item),
            (0, 0, p, 0, 0) => write!(f, "{} {}", p, BonusType::Proficiency),
            (0, 0, 0, s, 0) => write!(f, "{} {}", s, BonusType::Status),
            (0, 0, 0, 0, u) => write!(f, "{}", u),
            (c, i, p, s, u) => {
                write!(f, "(")?;
                let mut has_prev = false;
                if c != 0 {
                    write!(f, "{} {}", c, BonusType::Circumstance)?;
                    has_prev = true;
                }
                if i != 0 {
                    if has_prev {
                        write!(f, " + ")?;
                    }
                    write!(f, "{} {}", i, BonusType::Item)?;
                    has_prev = true;
                }
                if p != 0 {
                    if has_prev {
                        write!(f, " + ")?;
                    }
                    write!(f, "{} {}", p, BonusType::Proficiency)?;
                    has_prev = true;
                }
                if s != 0 {
                    if has_prev {
                        write!(f, " + ")?;
                    }
                    write!(f, "{} {}", s, BonusType::Status)?;
                    has_prev = true;
                }
                if u != 0 {
                    if has_prev {
                        write!(f, " + ")?;
                    }
                    write!(f, "{}", u)?;
                }
                write!(f, ")")
            }
        }
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

// pub trait HasModifiers {
//     fn get_modifier(&self, character: &Character, modifier: &str) -> Modifier;

//     fn get_modified_abilities(&self, character: &Character) -> HashSet<Ability> {
//         let _ = character;
//         HashSet::new()
//     }

//     fn get_modified_resistances(&self, character: &Character) -> HashSet<String> {
//         let _ = character;
//         HashSet::new()
//     }

//     fn get_modified_skills(&self, character: &Character) -> HashSet<Skill> {
//         let _ = character;
//         HashSet::new()
//     }
// }
