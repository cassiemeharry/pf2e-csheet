// use anyhow::{Error, Result};
use async_trait::async_trait;
// use serde::{de, Deserialize};
// use smallvec::{smallvec, SmallVec};
use smartstring::alias::String;
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    sync::Arc,
};

pub use pf2e_csheet_shared::{
    choices::{Choice, ChoiceRef, ResourceChoices},
    storage::ResourceStorage,
    Resource, ResourceRef, ResourceType,
};

pub struct ResourceStore {
    map: HashMap<String, HashMap<ResourceType, Arc<Resource>>>,
}

impl ResourceStore {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn lookup_immediate(&self, rref: &ResourceRef) -> Option<Arc<Resource>> {
        let name = rref.name.as_str();
        let named_map = self.map.get(name)?;
        match &rref.resource_type {
            Some(rt) => named_map.get(rt).map(Arc::clone),
            None => match named_map.len() {
                0 => None,
                1 => named_map.values().next().map(Arc::clone),
                len => {
                    warn!("Attempted to load a resource named {}, but it was ambiguous (found {} resources). Supply a type to disambaguate the result.", name, len);
                    None
                }
            },
        }
    }

    pub async fn lookup_async(&self, rrefs: &[&ResourceRef]) -> Vec<Option<Arc<Resource>>> {
        let mut results = Vec::with_capacity(rrefs.len());
        for rref in rrefs.iter() {
            let r_opt = self.lookup_immediate(*rref);
            results.push(r_opt);
        }
        results
    }

    pub async fn all_by_type(&self, rtype: ResourceType) -> HashSet<ResourceRef> {
        let mut rrefs = HashSet::new();
        for (name, type_map) in self.map.iter() {
            for this_rtype in type_map.keys() {
                if rtype == *this_rtype {
                    rrefs.insert(
                        ResourceRef::new(name.as_str(), None::<&str>).with_type(Some(rtype)),
                    );
                }
            }
        }
        rrefs
    }

    pub async fn register(&mut self, resource: Resource) -> Result<(), String> {
        let common = resource.common();
        let named_map = self
            .map
            .entry(common.name.clone())
            .or_insert(HashMap::new());
        let rtype = resource.resource_type();
        match named_map.entry(rtype) {
            Entry::Occupied(_) => {
                warn!("Attempted to overwrite {} resource {}", rtype, common.name);
                Ok(())
            }
            Entry::Vacant(slot) => {
                slot.insert(Arc::new(resource));
                Ok(())
            }
        }
    }
}

#[async_trait(?Send)]
impl ResourceStorage for ResourceStore {
    async fn lookup_async(&self, rrefs: &[&ResourceRef]) -> Vec<Option<Arc<Resource>>> {
        self.lookup_async(rrefs).await
    }

    fn lookup_immediate(&self, rref: &ResourceRef) -> Option<Arc<Resource>> {
        self.lookup_immediate(rref)
    }

    async fn all_by_type(&self, rtype: ResourceType) -> HashSet<ResourceRef> {
        self.all_by_type(rtype).await
    }

    async fn register(&mut self, resource: Resource) -> Result<(), String> {
        self.register(resource).await
    }
}

// pub use cond::Conditions;
// pub use rref::{ResourceRef, TypedRef};

// use crate::{
//     bonuses::{Bonus, BonusType, Modifier, Penalty},
//     character::Character,
//     stats::{Ability, Level, Proficiency},
// };

// lazy_static::lazy_static! {
//     static ref RESOURCES: Mutex<HashMap<String, HashMap<ResourceType, Arc<Resource>>>> = Mutex::new(HashMap::new());
// }

// macro_rules! impl_has_resource_type {
//     ($name:ident) => {
//         impl Into<Resource> for $name {
//             fn into(self) -> Resource {
//                 Resource::$name(self)
//             }
//         }

//         impl HasResourceType for $name {
//             const RESOURCE_TYPE: ResourceType = ResourceType::$name;
//         }
//     };
// }

// #[derive(Clone, Debug, Deserialize)]
// pub enum Resource {
//     #[serde(rename = "ancestry")]
//     Ancestry(Ancestry),
//     #[serde(rename = "action")]
//     Action(Action),
//     #[serde(rename = "background")]
//     Background(Background),
//     #[serde(rename = "class")]
//     Class(Class),
//     #[serde(rename = "class feature")]
//     ClassFeature(ClassFeature),
//     #[serde(rename = "feat")]
//     Feat(Feat),
//     #[serde(rename = "item")]
//     Item(Item),
//     #[serde(rename = "spell")]
//     Spell(Spell),
// }

// impl Resource {
//     pub fn register(self) {
//         let name = self.common().name.clone();
//         let rtype = self.get_type();
//         {
//             let mut map = RESOURCES.lock().unwrap();
//             let name_map = map.entry(name.clone()).or_insert(HashMap::new());
//             match name_map.entry(rtype) {
//                 Entry::Occupied(_) => {
//                     warn!(
//                         "Attempted to register a duplicate {} resource {}",
//                         rtype, name
//                     );
//                 }
//                 Entry::Vacant(entry) => {
//                     entry.insert(Arc::new(self));
//                 }
//             };
//         }
//     }

//     pub fn lookup(name: &str, rtype: Option<ResourceType>) -> Option<Arc<Self>> {
//         let map = RESOURCES.lock().unwrap();
//         let name_map = map.get(name)?;
//         match rtype {
//             None => {
//                 if name_map.len() == 1 {
//                     name_map.values().next().cloned()
//                 } else {
//                     warn!("Attempted to lookup a resource named {:?} without specifying a type. There are multiple resources loaded with that name.", name);
//                     None
//                 }
//             }
//             Some(rtype) => name_map.get(&rtype).cloned(),
//         }
//     }

//     pub fn name(&self) -> &str {
//         self.common().name.as_str()
//     }

//     pub fn get_type(&self) -> ResourceType {
//         match self {
//             Self::Ancestry(_) => ResourceType::Ancestry,
//             Self::Action(_) => ResourceType::Action,
//             Self::Background(_) => ResourceType::Background,
//             Self::Class(_) => ResourceType::Class,
//             Self::ClassFeature(_) => ResourceType::ClassFeature,
//             Self::Feat(_) => ResourceType::Feat,
//             Self::Item(_) => ResourceType::Item,
//             Self::Spell(_) => ResourceType::Spell,
//         }
//     }

//     pub fn common(&self) -> &ResourceCommon {
//         match self {
//             Self::Ancestry(a) => &a.common,
//             Self::Action(a) => &a.common,
//             Self::Background(b) => &b.common,
//             Self::Class(c) => &c.common,
//             Self::ClassFeature(f) => &f.common,
//             Self::Feat(f) => &f.common,
//             Self::Item(i) => &i.common,
//             Self::Spell(s) => &s.common,
//         }
//     }

//     pub fn get_choice<T: FromStr>(
//         self: &Arc<Self>,
//         name: &Choice,
//         context: Context<'_>,
//     ) -> Option<T> {
//         // The choice may not be associated with this resource, so we need to
//         // look that up first before calling Character::get_choice.
//         let meta = self.common().choices.get(name)?;
//         let res: ResourceRef = match meta.resource() {
//             None => context.rref.clone(),
//             Some(res_ref) => res_ref.clone().into(),
//         };
//         context.character.get_choice(&res, name)
//     }

//     pub fn make_ref(self: Arc<Self>, modifier: Option<&str>) -> ResourceRef {
//         match modifier {
//             None => ResourceRef::new_from_resolved(self),
//             Some(m) => ResourceRef::new_from_resolved_mod(self, m),
//         }
//     }

//     pub fn get_active_resources(
//         self: Arc<Self>,
//         character: &Character,
//         modifier: Option<&str>,
//     ) -> SmallVec<[ResourceRef; 1]> {
//         let mut resources = smallvec![];

//         let self_ref = self.clone().make_ref(modifier);
//         let ctx = Context::new(character, &self_ref);

//         let common = self.common();
//         if !common.conditions.applies(ctx) {
//             return smallvec![];
//         }

//         for effect in common.effects.0.iter() {
//             resources.extend(effect.get_active_resources(ctx));
//         }
//         resources
//     }

//     pub fn get_modifier(&self, label: &str, ctx: Context<'_>) -> Modifier {
//         let mut modifier = Modifier::new();

//         let common = self.common();
//         modifier += common.get_modifier(label, ctx);
//         match self {
//             Self::Class(c) => {
//                 if let Ok(m) = c.get_modifier(label, ctx) {
//                     modifier += m;
//                 }
//             }
//             _ => (),
//         };
//         modifier
//     }
// }

// impl fmt::Display for Resource {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         fmt::Display::fmt(self.common(), f)
//     }
// }

// impl<'a> From<&'a Resource> for common::Resource {
//     fn from(res: &'a Resource) -> common::Resource {
//         match res {
//             Resource::Ancestry(inner) => common::Resource::Ancestry(inner.into()),
//             Resource::Action(inner) => common::Resource::Action(inner.into()),
//             Resource::Background(inner) => common::Resource::Background(inner.into()),
//             Resource::Class(inner) => common::Resource::Class(inner.into()),
//             Resource::ClassFeature(inner) => common::Resource::ClassFeature(inner.into()),
//             Resource::Feat(inner) => common::Resource::Feat(inner.into()),
//             Resource::Item(inner) => common::Resource::Item(inner.into()),
//             Resource::Spell(inner) => common::Resource::Spell(inner.into()),
//         }
//     }
// }

// impl<'a> From<&'a Action> for common::Action {
//     fn from(_: &'a Action) -> common::Action {
//         common::Action
//     }
// }

// #[derive(Clone, Debug, Deserialize)]
// pub struct Ancestry {
//     #[serde(flatten)]
//     common: ResourceCommon,
// }

// impl_has_resource_type!(Ancestry);

// impl<'a> From<&'a Ancestry> for common::Ancestry {
//     fn from(_: &'a Ancestry) -> common::Ancestry {
//         common::Ancestry
//     }
// }

// #[derive(Clone, Debug, Deserialize)]
// pub struct Background {
//     #[serde(flatten)]
//     common: ResourceCommon,
// }

// impl_has_resource_type!(Background);

// impl<'a> From<&'a Background> for common::Background {
//     fn from(_: &'a Background) -> common::Background {
//         common::Background
//     }
// }

// impl_has_resource_type!(Class);

// impl<'a> From<&'a Class> for common::Class {
//     fn from(_: &'a Class) -> common::Class {
//         common::Class
//     }
// }

// #[derive(Clone, Debug, Deserialize)]
// pub struct ClassFeature {
//     #[serde(flatten)]
//     common: ResourceCommon,
// }

// impl_has_resource_type!(ClassFeature);

// impl<'a> From<&'a ClassFeature> for common::ClassFeature {
//     fn from(_: &'a ClassFeature) -> common::ClassFeature {
//         common::ClassFeature
//     }
// }

// #[derive(Clone, Debug, Deserialize)]
// pub struct Item {
//     #[serde(flatten)]
//     common: ResourceCommon,
// }

// impl_has_resource_type!(Item);

// impl<'a> From<&'a Item> for common::Item {
//     fn from(_: &'a Item) -> common::Item {
//         common::Item
//     }
// }

// #[derive(Clone, Debug, Deserialize)]
// pub struct Feat {
//     #[serde(flatten)]
//     common: ResourceCommon,
//     #[serde(default)]
//     level: Level,
// }

// impl_has_resource_type!(Feat);

// impl<'a> From<&'a Feat> for common::Feat {
//     fn from(_: &'a Feat) -> common::Feat {
//         common::Feat
//     }
// }

// #[derive(Clone, Debug, Deserialize)]
// pub struct Spell {
//     #[serde(flatten)]
//     common: ResourceCommon,
//     #[serde(default)]
//     level: Level,
// }

// impl_has_resource_type!(Spell);

// impl<'a> From<&'a Spell> for common::Spell {
//     fn from(_: &'a Spell) -> common::Spell {
//         common::Spell
//     }
// }

// #[test]
// fn test_deserialize_feat() {
//     let raw_feats = &[
//         "\
// name: Crane Stance
// level: 1
// effects:
//   - action:
//       name: Crane Stance
//       traits: [monk, stance]
//       conditions:
//         armor category: unarmored
//       type: 1 action
//       description: >-
//         You enter the stance of a crane, holding your arms in an imitation of a
//         crane's wings and using flowing, defensive motions. You gain a +1
//         circumstance bonus to AC, but the only Strikes you can make are crane wing
//         attacks. These deal 1d6 bludgeoning damage; are in the brawling group; and
//         have the agile, finesse, nonleathal, and unarmed traits.

//         While in Crane Stance, reduce the DC for High Jump and Long Jump by 5, and
//         when you Leap, you can move an additional 5 feet horizontally or 2 feet
//         vertically.",
//         "\
// name: incredible movement
// description: >-
//   You move like the wind. You gain a +[[ $speed ]] status bonus to your Speed
//   whenever you're not wearing armor. The bonus increases by 5 feet every
//   4 levels you have beyond 3rd.
// modifiers:
//   $speed: distance
// effects:
//   - bonus:
//       type: status
//       to: speed
//       value: $speed
//       conditions:
//         armor category: unarmored",
//     ];
//     for raw in raw_feats.iter() {
//         println!(
//             "Parsing YAML:\n
// --------------
// {}
// --------------",
//             raw
//         );
//         let loaded: Feat = serde_yaml::from_str(raw).unwrap();
//         println!("Loaded feat:\n{:#?}\n====================", loaded);
//     }
// }
