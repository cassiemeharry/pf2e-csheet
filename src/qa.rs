use serde::Deserialize;
use smallvec::SmallVec;
use smartstring::alias::String;
use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, Mutex},
};

use crate::{
    resources::{self, refs::Ref, traits::Resource},
    stats,
};

/// Resources can sometimes be customized by user choices. These choices
/// influence the bonuses and abilities provided to the character.
///
/// Some examples:
///
/// * The "Assurance" skill feat requires a trained skill.
///
/// * Multiple sources grant bonus feats of a given kind, so long as their
///   prerequisites are met.
///
/// * Some classes have multiple options for their key ability, and the user
///   must pick one when choosing the class.
///
/// * The "Additional Lore" skill feat needs a new lore topic to become trained
///   in.
///
/// * The "Fighter Weapon Mastery" class feature requires a weapon group to
///   grant its bonuses to.
///
/// Each resource is responsible for maintaining the questions it needs
/// answered.
#[derive(Clone, Debug)]
pub struct AnsweredQuestion {
    pub question: Question,
    pub answer: Option<Arc<Answer>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Question {
    pub label: String,
    /// Tag should be unique for a given resource.
    pub tag: String,
    #[serde(flatten)]
    pub looking_for: QuestionOption,
}

impl fmt::Display for Question {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.label.fmt(f)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct QuestionRef {
    pub tag: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionOption {
    Ability {
        options: SmallVec<[stats::Ability; 2]>,
    },
    Ancestry,
    Feat {
        #[serde(default, rename = "trait")]
        trait_: Option<resources::feat::FeatTrait>,
        #[serde(default)]
        traits: SmallVec<[resources::feat::FeatTrait; 1]>,
        #[serde(default)]
        only_class: Option<Ref<resources::Class>>,
    },
    Item {
        #[serde(default)]
        min_level: Option<stats::Level>,
        #[serde(default)]
        max_level: Option<stats::Level>,
    },
    ItemFormula {
        #[serde(default)]
        min_level: Option<stats::Level>,
        #[serde(default)]
        max_level: Option<stats::Level>,
    },
    LoreTopic {
        #[serde(default = "default_true")]
        only_new: bool,
    },
    Skill {
        min_proficiency: stats::Proficiency,
    },
    Text,
}

#[inline(always)]
fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize)]
pub enum Answer {
    Ability(stats::Ability),
    Ancestry(Ref<resources::Ancestry>),
    Feat(Ref<resources::Feat>),
    LoreTopic(String),
    Skill(stats::Skill),
    Text(String),
    WeaponGroup(resources::item::WeaponGroup),
}

struct AnswerKey<R: Resource>(std::marker::PhantomData<R>);

impl<R> typemap::Key for AnswerKey<R>
where
    R: Resource,
{
    type Value = HashMap<R::Index, HashMap<String, Arc<Answer>>>;
}

#[derive(Clone)]
pub struct AnswerMap {
    type_map: Arc<Mutex<typemap::ShareMap>>,
}

impl Default for AnswerMap {
    fn default() -> Self {
        let type_map = typemap::ShareMap::custom();
        let type_map = Arc::new(Mutex::new(type_map));
        Self { type_map }
    }
}

impl fmt::Debug for AnswerMap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TODO: debug for AnswerMap")
    }
}

impl<'de> Deserialize<'de> for AnswerMap {
    fn deserialize<D>(deserialize: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        warn!("Not deserializing AnswerMap");
        Ok(Self::default())
    }
}

impl AnswerMap {
    pub fn answers_for_resource<'a, R>(&'a self, r: &R) -> Option<AnswerMapRef<'a>>
    where
        R: Resource,
    {
        let lock = self.type_map.lock().unwrap();
        let or = owning_ref::OwningRef::new(lock)
            .try_map(move |type_map| {
                type_map
                    .get::<AnswerKey<R>>()
                    .ok_or(())?
                    .get(&r.get_index_value())
                    .ok_or(())
            })
            .ok()?;
        Some(AnswerMapRef { or })
    }

    pub fn answers_for_resource_mut<'a, R>(&'a mut self, r: &R) -> AnswerMapRefMut<'a>
    where
        R: Resource,
    {
        let lock = self.type_map.lock().unwrap();
        let or = owning_ref::OwningRefMut::new(lock).map_mut(move |type_map| {
            type_map
                .entry::<AnswerKey<R>>()
                .or_insert_with(HashMap::new)
                .entry(r.get_index_value())
                .or_insert_with(HashMap::new)
        });
        AnswerMapRefMut { or }
    }
}

pub struct AnswerMapRef<'a> {
    or: owning_ref::MutexGuardRef<'a, typemap::ShareMap, HashMap<String, Arc<Answer>>>,
}

impl<'a> std::ops::Deref for AnswerMapRef<'a> {
    type Target = HashMap<String, Arc<Answer>>;

    fn deref(&self) -> &Self::Target {
        &*self.or
    }
}

pub struct AnswerMapRefMut<'a> {
    or: owning_ref::MutexGuardRefMut<'a, typemap::ShareMap, HashMap<String, Arc<Answer>>>,
}

impl<'a> std::ops::Deref for AnswerMapRefMut<'a> {
    type Target = HashMap<String, Arc<Answer>>;

    fn deref(&self) -> &Self::Target {
        &*self.or
    }
}

impl<'a> std::ops::DerefMut for AnswerMapRefMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.or
    }
}
