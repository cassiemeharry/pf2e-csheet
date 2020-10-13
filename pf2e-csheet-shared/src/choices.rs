#[cfg(test)]
use proptest::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;
use ref_cast::RefCast;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use std::{
    borrow::Borrow,
    cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd},
    collections::HashMap,
    fmt,
    hash::{Hash, Hasher},
    str::FromStr,
};
use thiserror::Error;

use crate::{
    common::{ResourceRef, ResourceType},
    try_from_str,
};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum ChoiceKind {
    Ability,
    Distance,
    Level,
    OwnedItem,
    Resource {
        #[serde(rename = "type")]
        resource_type: ResourceType,
        #[serde(rename = "trait")]
        #[serde(skip_serializing_if = "Option::is_none")]
        #[cfg_attr(
            test,
            proptest(
                strategy = "proptest::option::of(any::<std::string::String>().prop_map_into())"
            )
        )]
        trait_filter: Option<String>,
    },
    SavingThrow,
    SpellTradition,
    Skill,
}

impl fmt::Display for ChoiceKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Ability => write!(f, "an ability"),
            Self::Distance => write!(f, "a distance"),
            Self::Level => write!(f, "a level"),
            Self::OwnedItem => write!(f, "an owned item"),
            Self::Resource {
                resource_type,
                trait_filter: Some(t),
            } => write!(f, "{} (with trait {:?})", resource_type, t),
            Self::Resource {
                resource_type,
                trait_filter: None,
            } => write!(f, "{}", resource_type),
            Self::SavingThrow => write!(f, "a saving throw"),
            Self::SpellTradition => write!(f, "a spell tradition"),
            Self::Skill => write!(f, "a skill"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct ChoiceMeta {
    pub kind: ChoiceKind,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<ResourceRef>,
    pub key: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "crate::is_default")]
    pub character_wide: bool,
    #[cfg_attr(
        test,
        proptest(strategy = "proptest::option::of(any::<std::string::String>().prop_map_into())")
    )]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl ChoiceMeta {
    pub fn kind(&self) -> ChoiceKind {
        self.kind.clone()
    }

    pub fn resource(&self) -> Option<&ResourceRef> {
        self.from.as_ref()
    }

    pub fn description(&self) -> &str {
        match self.description.as_ref() {
            Some(s) => s.as_str(),
            None => "No description provided",
        }
    }
}

#[repr(transparent)]
#[derive(Debug, RefCast)]
pub struct ChoiceRef(str);

impl ChoiceRef {
    fn lowercase_chars<'a>(&'a self) -> impl Iterator<Item = char> + 'a {
        self.0.chars().flat_map(char::to_lowercase)
    }
}

impl PartialEq for ChoiceRef {
    fn eq(&self, other: &Self) -> bool {
        let mut self_chars = self.lowercase_chars();
        let mut other_chars = other.lowercase_chars();
        loop {
            match (self_chars.next(), other_chars.next()) {
                (None, None) => return true,
                (Some(l), Some(r)) if l == r => continue,
                _ => return false,
            }
        }
    }
}

impl Eq for ChoiceRef {}

impl Hash for ChoiceRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for c in self.lowercase_chars() {
            c.hash(state);
        }
    }
}

impl PartialOrd for ChoiceRef {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ChoiceRef {
    fn cmp(&self, other: &Self) -> Ordering {
        let mut self_chars = self.lowercase_chars();
        let mut other_chars = other.lowercase_chars();
        loop {
            match (self_chars.next(), other_chars.next()) {
                (None, None) => return Ordering::Equal,
                (Some(_), None) => return Ordering::Greater,
                (None, Some(_)) => return Ordering::Less,
                (Some(l), Some(r)) => match l.cmp(&r) {
                    Ordering::Less => return Ordering::Less,
                    Ordering::Equal => continue,
                    Ordering::Greater => return Ordering::Greater,
                },
            }
        }
    }
}

impl fmt::Display for ChoiceRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[repr(transparent)]
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(Arbitrary))]
#[serde(try_from = "smartstring::alias::String")]
pub struct Choice(
    #[cfg_attr(
        test,
        proptest(
            strategy = "prop::string::string_regex(\"[a-zA-Z][a-zA-Z_]+\").unwrap().prop_map_into()"
        )
    )]
    String,
);

impl Serialize for Choice {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let c = format!("${}", self.0);
        serializer.serialize_str(&c)
    }
}

impl Borrow<ChoiceRef> for Choice {
    fn borrow(&self) -> &ChoiceRef {
        ChoiceRef::ref_cast(self.0.as_str())
    }
}

impl Borrow<ChoiceRef> for &'_ Choice {
    fn borrow(&self) -> &ChoiceRef {
        ChoiceRef::ref_cast(self.0.as_str())
    }
}

impl ToOwned for ChoiceRef {
    type Owned = Choice;

    fn to_owned(&self) -> Choice {
        Choice((&self.0).into())
    }
}

impl<'a> Borrow<ChoiceRef> for &'a str {
    fn borrow(&self) -> &ChoiceRef {
        ChoiceRef::ref_cast(self)
    }
}

#[derive(Error, Debug, Deserialize, Serialize)]
pub enum ChoiceFromStrError {
    #[error("Choice key must not be empty")]
    Empty,
    #[error("Choice key must start with \"$\"")]
    BadStart,
}

try_from_str!(Choice);

impl FromStr for Choice {
    type Err = ChoiceFromStrError;

    fn from_str(s: &str) -> Result<Choice, ChoiceFromStrError> {
        if s.len() == 0 {
            return Err(ChoiceFromStrError::Empty);
        }
        if s.chars().next().unwrap() != '$' {
            return Err(ChoiceFromStrError::BadStart);
        }
        let inner = s[1..].into();
        Ok(Choice(inner))
    }
}

impl PartialEq for Choice {
    fn eq(&self, other: &Self) -> bool {
        <Self as Borrow<ChoiceRef>>::borrow(self).eq(other.borrow())
    }
}

impl Eq for Choice {}

impl Hash for Choice {
    fn hash<H: Hasher>(&self, state: &mut H) {
        <Self as Borrow<ChoiceRef>>::borrow(self).hash(state)
    }
}

impl PartialOrd for Choice {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Choice {
    fn cmp(&self, other: &Self) -> Ordering {
        <Self as Borrow<ChoiceRef>>::borrow(self).cmp(other.borrow())
    }
}

impl fmt::Display for Choice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(<Self as Borrow<ChoiceRef>>::borrow(self), f)
    }
}

impl From<&'_ str> for Choice {
    fn from(s: &str) -> Choice {
        Choice(s.into())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct ResourceChoices {
    #[serde(flatten)]
    map: HashMap<Choice, ChoiceMeta>,
}

impl ResourceChoices {
    pub fn add(&mut self, key: impl Into<Choice>, value: ChoiceMeta) {
        let key = key.into();
        if value.key {
            for (k, v) in self.map.iter_mut() {
                if v.key {
                    warn!("Changing key choice from {} to {}", k, key);
                    v.key = false;
                }
            }
        }
        self.map.insert(key.into(), value);
    }

    pub fn get<'a, Q: ?Sized>(&'a self, key: &Q) -> Option<&'a ChoiceMeta>
    where
        Choice: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.map.get(key.into())
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Choice, &ChoiceMeta)> + '_ {
        self.map.iter()
    }
}
