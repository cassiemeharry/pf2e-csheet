use anyhow::{anyhow, Error, Result};
use serde::{de, Deserialize};
use smallvec::SmallVec;
use smartstring::alias::String;
use std::{
    any::Any,
    cmp::{Ord, Ordering, PartialEq, PartialOrd},
    collections::{hash_map::Entry, BTreeMap, HashMap},
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    str::FromStr,
    sync::{Arc, Mutex, RwLock},
};

mod calc;
mod rref;

pub use calc::{CalculatedString, Calculation, Context};
pub use rref::{ResourceRef, TypedRef};

use crate::{
    character::Character,
    stats::{Ability, Level, Proficiency},
    try_from_str,
};

lazy_static::lazy_static! {
    static ref RESOURCES: Mutex<HashMap<String, HashMap<ResourceType, Arc<Resource>>>> = Mutex::new(HashMap::new());
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum ResourceType {
    Ancestry,
    Action,
    Background,
    Class,
    ClassFeature,
    Feat,
    Item,
    Spell,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name: &'static str = match self {
            Self::Ancestry => "ancestry",
            Self::Action => "action",
            Self::Background => "background",
            Self::Class => "class",
            Self::ClassFeature => "class feature",
            Self::Feat => "feat",
            Self::Item => "item",
            Self::Spell => "spell",
        };
        write!(f, "{}", name)
    }
}

pub trait HasResourceType {
    const RESOURCE_TYPE: ResourceType;
}

macro_rules! impl_has_resource_type {
    ($name:ident) => {
        impl HasResourceType for $name {
            const RESOURCE_TYPE: ResourceType = ResourceType::$name;
        }
    };
}

#[derive(Clone, Debug, Deserialize)]
pub enum Resource {
    #[serde(rename = "ancestry")]
    Ancestry(Ancestry),
    #[serde(rename = "action")]
    Action(Action),
    #[serde(rename = "background")]
    Background(Background),
    #[serde(rename = "class")]
    Class(Class),
    #[serde(rename = "class feature")]
    ClassFeature(ClassFeature),
    #[serde(rename = "feat")]
    Feat(Feat),
    #[serde(rename = "item")]
    Item(Item),
}

impl Resource {
    pub fn register(self) {
        let name = self.common().name.clone();
        let rtype = self.get_type();
        {
            let mut map = RESOURCES.lock().unwrap();
            let name_map = map.entry(name.clone()).or_insert(HashMap::new());
            match name_map.entry(rtype) {
                Entry::Occupied(entry) => {
                    warn!(
                        "Attempted to register a duplicate {} resource {}",
                        rtype, name
                    );
                }
                Entry::Vacant(entry) => {
                    entry.insert(Arc::new(self));
                }
            };
        }
    }

    pub fn lookup(name: &str, rtype: Option<ResourceType>) -> Option<Arc<Self>> {
        let mut map = RESOURCES.lock().unwrap();
        let name_map = map.get(name)?;
        match rtype {
            None => {
                if name_map.len() == 1 {
                    name_map.values().next().cloned()
                } else {
                    warn!("Attempted to lookup a resource named {:?} without specifying a type. There are multiple resources loaded with that name.", name);
                    None
                }
            }
            Some(rtype) => name_map.get(&rtype).cloned(),
        }
    }

    pub fn name(&self) -> &str {
        self.common().name.as_str()
    }

    pub fn get_type(&self) -> ResourceType {
        match self {
            Self::Ancestry(_) => ResourceType::Ancestry,
            Self::Action(_) => ResourceType::Action,
            Self::Background(_) => ResourceType::Background,
            Self::Class(_) => ResourceType::Class,
            Self::ClassFeature(_) => ResourceType::ClassFeature,
            Self::Feat(_) => ResourceType::Feat,
            Self::Item(_) => ResourceType::Item,
        }
    }

    pub fn common(&self) -> &ResourceCommon {
        match self {
            Self::Ancestry(a) => &a.common,
            Self::Action(a) => &a.common,
            Self::Background(b) => &b.common,
            Self::Class(c) => &c.common,
            Self::ClassFeature(f) => &f.common,
            Self::Feat(f) => &f.common,
            Self::Item(i) => &i.common,
        }
    }

    pub fn get_choice<T: FromStr>(self: &Arc<Self>, name: &str, context: Context<'_>) -> Option<T> {
        // The choice may not be associated with this resource, so we need to
        // look that up first before calling Character::get_choice.
        let meta = self.common().choices.get(name)?;
        let res: ResourceRef = match meta.from.as_ref() {
            None => context.rref.clone(),
            Some(res_ref) => res_ref.clone(),
        };
        context.character.get_choice::<T>(res, name)
    }

    pub fn make_ref(self: Arc<Self>, modifier: Option<&str>) -> ResourceRef {
        match modifier {
            None => ResourceRef::new_from_resolved(self),
            Some(m) => ResourceRef::new_from_resolved_mod(self, m.into()),
        }
    }
}

impl fmt::Display for Resource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.common(), f)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ResourceCommon {
    name: String,
    #[serde(default)]
    traits: Vec<String>,
    #[serde(default)]
    description: CalculatedString,
    #[serde(default)]
    choices: Choices,
    #[serde(default)]
    effects: Effects,
}

impl fmt::Display for ResourceCommon {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.name, f)
    }
}

#[repr(transparent)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ChoiceLabel(String);

impl fmt::Display for ChoiceLabel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "${}", self.0)
    }
}

impl AsRef<str> for ChoiceLabel {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl std::borrow::Borrow<str> for ChoiceLabel {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub enum ChoiceKind {
    Ability,
    OwnedItem,
    SavingThrow,
    Speed,
    SpellTradition,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ChoiceMeta {
    kind: ChoiceKind,
    from: Option<ResourceRef>,
    optional: bool,
}

// pub trait ChoiceValueBounds: Any + fmt::Debug + fmt::Display {}

// impl<T> ChoiceValueBounds for T where T: Any + fmt::Debug + fmt::Display {}

// #[derive(Clone)]
// pub struct ChoiceValue {
//     inner: Arc<dyn ChoiceValueBounds>,
// }

// impl ChoiceValue {
//     pub fn new(value: impl ChoiceValueBounds) -> Self {
//         Self {
//             inner: Arc::new(value),
//         }
//     }

//     #[inline]
//     pub fn downcast_ref<T: ChoiceValueBounds>(&self) -> Option<&T> {
//         let inner = &self.inner as &dyn Any;
//         inner.downcast_ref::<T>()
//     }
// }

// impl fmt::Debug for ChoiceValue {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         fmt::Debug::fmt(&self.inner, f)
//     }
// }

// impl fmt::Display for ChoiceValue {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         fmt::Display::fmt(&self.inner, f)
//     }
// }

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Choices {
    map: HashMap<String, ChoiceMeta>,
}

impl Choices {
    fn get<'a>(&'a self, key: &'_ str) -> Option<&'a ChoiceMeta> {
        self.map.get(key)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub enum Effect {
    #[serde(rename = "bonus")]
    AddBonus(Calculation),
    #[serde(rename = "add focus pool")]
    #[serde(alias = "gain focus pool")]
    AddFocusPool,
    #[serde(rename = "penalty")]
    AddPenalty(Calculation),
    #[serde(rename = "add spell")]
    #[serde(alias = "gain spell")]
    AddSpell(TypedRef<Spell>),
    #[serde(rename = "add trait")]
    AddTrait {
        to: ResourceRef,
        trait_: String,
    },
    AddResource(ResourceRef),
    #[serde(alias = "action")]
    #[serde(alias = "add action")]
    #[serde(alias = "gain action")]
    GrantAction(Action),
    #[serde(rename = "proficiency")]
    #[serde(alias = "add proficiency")]
    #[serde(alias = "gain proficiency")]
    #[serde(alias = "gain proficiency in")]
    IncreaseProficiency {
        #[serde(rename = "in")]
        target: String,
        #[serde(alias = "increases to")]
        level: Proficiency,
    },
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Effects(Vec<Effect>);

#[derive(Clone, Debug)]
pub enum ActionType {
    Free,
    Reaction,
    One,
    Two,
    Three,
}

// Derive doesn't like a mix of numbers and text
impl<'de> Deserialize<'de> for ActionType {
    fn deserialize<D>(deserializer: D) -> Result<ActionType, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = ActionType;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "\"free\", \"reaction\", 1, 2, or 3")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    1 => Ok(ActionType::One),
                    2 => Ok(ActionType::Two),
                    3 => Ok(ActionType::Three),
                    _ => Err(E::custom("Expected 1, 2, or 3")),
                }
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                <ActionType as FromStr>::from_str(value).map_err(|e| E::custom(e))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

try_from_str!(ActionType);

impl FromStr for ActionType {
    type Err = Error;

    fn from_str(s: &str) -> Result<ActionType> {
        match s {
            "free" => Ok(Self::Free),
            "reaction" => Ok(Self::Reaction),
            "1" => Ok(Self::One),
            "1 action" => Ok(Self::One),
            "one" => Ok(Self::One),
            "one action" => Ok(Self::One),
            "2" => Ok(Self::Two),
            "2 actions" => Ok(Self::Two),
            "two" => Ok(Self::Two),
            "two actions" => Ok(Self::Two),
            "3" => Ok(Self::Three),
            "3 actions" => Ok(Self::Three),
            "three" => Ok(Self::Three),
            "three actions" => Ok(Self::Three),
            other => Err(anyhow::anyhow!("Unexpected action type {:?}", other)),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Action {
    #[serde(flatten)]
    common: ResourceCommon,
    #[serde(rename = "type")]
    action_type: ActionType,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Ancestry {
    #[serde(flatten)]
    common: ResourceCommon,
}

impl_has_resource_type!(Ancestry);

#[derive(Clone, Debug, Deserialize)]
pub struct Background {
    #[serde(flatten)]
    common: ResourceCommon,
}

impl_has_resource_type!(Background);

#[derive(Clone, Debug, Deserialize)]
pub struct Class {
    #[serde(flatten)]
    common: ResourceCommon,
    #[serde(rename = "key ability")]
    key_ability: SmallVec<[Ability; 2]>,
    #[serde(rename = "hp per level")]
    hp_per_level: u16,
    perception: Proficiency,
    #[serde(rename = "fort save")]
    fort_save: Proficiency,
    #[serde(rename = "reflex save")]
    reflex_save: Proficiency,
    #[serde(rename = "will save")]
    will_save: Proficiency,
    #[serde(rename = "free skill trained")]
    #[serde(alias = "free skills trained")]
    free_skill_trained: u16,
    #[serde(default, rename = "weapon proficiencies")]
    weapon_proficiencies: ClassWeaponProficiencies,
    #[serde(default, rename = "armor proficiencies")]
    armor_proficiencies: ClassArmorProficiencies,
    advancement: BTreeMap<Level, Vec<TypedRef<ClassFeature>>>,
}

impl_has_resource_type!(Class);

#[derive(Clone, Debug, Default, Deserialize)]
struct ClassWeaponProficiencies {
    #[serde(default)]
    unarmed: Proficiency,
    #[serde(default)]
    simple: Proficiency,
    #[serde(default)]
    martial: Proficiency,
    #[serde(default)]
    advanced: Proficiency,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ClassArmorProficiencies {
    #[serde(default)]
    unarmored: Proficiency,
    #[serde(default)]
    light: Proficiency,
    #[serde(default)]
    medium: Proficiency,
    #[serde(default)]
    heavy: Proficiency,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ClassFeature {
    #[serde(flatten)]
    common: ResourceCommon,
}

impl_has_resource_type!(ClassFeature);

#[derive(Clone, Debug, Deserialize)]
pub struct Item {
    #[serde(flatten)]
    common: ResourceCommon,
}

impl_has_resource_type!(Item);

#[derive(Clone, Debug, Deserialize)]
pub struct Feat {
    #[serde(flatten)]
    common: ResourceCommon,
    #[serde(default)]
    level: Level,
}

impl_has_resource_type!(Feat);

#[derive(Clone, Debug, Deserialize)]
pub struct Spell {
    #[serde(flatten)]
    common: ResourceCommon,
    #[serde(default)]
    level: Level,
}

impl_has_resource_type!(Spell);

#[test]
fn test_deserialize_feat() {
    let raw_feats = &[
        "\
name: Crane Stance
level: 1
effects:
  - action:
      name: Crane Stance
      traits: [monk, stance]
      conditions:
        armor category: unarmored
      type: 1 action
      description: >-
        You enter the stance of a crane, holding your arms in an imitation of a
        crane's wings and using flowing, defensive motions. You gain a +1
        circumstance bonus to AC, but the only Strikes you can make are crane wing
        attacks. These deal 1d6 bludgeoning damage; are in the brawling group; and
        have the agile, finesse, nonleathal, and unarmed traits.

        While in Crane Stance, reduce the DC for High Jump and Long Jump by 5, and
        when you Leap, you can move an additional 5 feet horizontally or 2 feet
        vertically.",
        "\
name: incredible movement
description: >-
  You move like the wind. You gain a +10-foot status bonus to your Speed
  whenever you're not wearing armor. The bonus increases by 5 feet every
  4 levels you have beyond 3rd.
modifiers:
  $speed: distance
effects:
  - bonus:
      type: status
      to: speed
      value: $speed
#    conditions:
#      armor category: unarmored",
    ];
    for raw in raw_feats.iter() {
        println!(
            "Parsing YAML:\n
{}
--------------",
            raw
        );
        let loaded: Feat = serde_yaml::from_str(raw).unwrap();
        println!("Loaded feat: {:#?}", loaded);
    }
    assert!(false);
}
