use anyhow::{Error, Result};
use serde::Deserialize;
use smallvec::{smallvec, SmallVec};
use smartstring::alias::String;
use std::{fmt, str::FromStr};

use crate::{
    qa::Question,
    resources::{
        refs::Ref,
        traits::{Resource, ResourceExtra},
        Class,
    },
    stats::{Ability, Level, Proficiency, Skill},
    try_from_str,
};

#[derive(Clone, Debug)]
pub enum SkillChoice {
    Any,
    AnyLore,
    Single(Skill),
    Choices(Vec<Skill>),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub enum Prereq {
    SkillProf(SkillChoice, Proficiency),
    Class(Ref<Class>),
    MinAbilityScore(Ability, u8),
}

try_from_str!(Prereq);

impl FromStr for Prereq {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(crate::parsers::feat_prereq(s)?)
    }
}

fn deserialize_prereqs<'de, D>(deserializer: D) -> Result<SmallVec<[Prereq; 1]>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    use serde::de::{self, IntoDeserializer as _, Visitor};

    struct PrereqsVisitor;

    impl<'de> Visitor<'de> for PrereqsVisitor {
        type Value = SmallVec<[Prereq; 1]>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "a string or a sequence of strings")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let p = <Prereq as Deserialize<'de>>::deserialize(value.into_deserializer())?;
            Ok(smallvec![p])
        }

        fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let ds = de::value::SeqAccessDeserializer::new(seq);
            <Self::Value as Deserialize<'de>>::deserialize(ds)
        }
    }

    deserializer.deserialize_any(PrereqsVisitor)
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
struct FeatModifiers {
    skill: Option<Skill>,
}

impl ResourceExtra<FeatIndex> for FeatModifiers {
    fn apply_index(&mut self, index: &FeatIndex) {
        match index {
            FeatIndex::Name(_name) => (),
            FeatIndex::NameAndSkill { name: _, skill } => {
                if self.skill.is_none() {
                    self.skill = Some(skill.clone());
                }
            }
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Feat {
    name: String,
    level: Level,
    #[serde(default)]
    traits: SmallVec<[FeatTrait; 2]>,
    #[serde(default, deserialize_with = "deserialize_prereqs")]
    prereqs: SmallVec<[Prereq; 1]>,
    #[serde(default)]
    frequency: Option<String>,
    #[serde(default)]
    requrements: SmallVec<[String; 1]>,
    description: String,
    #[serde(default)]
    questions: Vec<Question>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum FeatIndex {
    Name(String),
    NameAndSkill { name: String, skill: Skill },
}

impl Resource for Feat {
    type Index = FeatIndex;
    type Extra = FeatModifiers;

    fn get_index_value(&self, modifiers: &FeatModifiers) -> FeatIndex {
        match modifiers.skill.as_ref() {
            None => FeatIndex::Name(self.name.clone()),
            Some(skill) => FeatIndex::NameAndSkill {
                name: self.name.clone(),
                skill: skill.clone(),
            },
        }
    }

    fn matches(&self, modifiers: &FeatModifiers, index: &FeatIndex) -> bool {
        match index {
            FeatIndex::Name(n) => &self.name == n,
            FeatIndex::NameAndSkill { name, skill } => {
                if name != &self.name {
                    return false;
                }
                match modifiers.skill.as_ref() {
                    Some(skill_mod) => skill == skill_mod,
                    None => true,
                }
            }
        }
    }

    // fn apply_index(&self, index: &FeatIndex) -> Self {
    // }

    fn get_questions(&self) -> Vec<Question> {
        self.questions.clone()
    }
}

// impl ResourceLoadable<(String, Skill)> for Feat {
//     fn get_index_value(&self) -> Option<(String, Skill)> {
//         if let Some(s) = self.modifiers.skill.as_ref().cloned() {
//             return Some((self.name.clone(), s));
//         }
//         None
//     }

//     fn loadable_matches(&self, (name, skill): &(String, Skill)) -> bool {
//         if name != &self.name {
//             return false;
//         }
//         match self.modifiers.skill.as_ref() {
//             Some(skill_mod) => skill == skill_mod,
//             None => true,
//         }
//     }

//     #[inline]
//     fn to_common_index((name, _skill): (String, Skill)) -> FeatIndex {
//         name
//     }

//     fn apply_index(&self, index: &(String, Skill)) -> Option<Self> {
// }

#[test]
fn test_matches_generic() {
    use smallvec::smallvec;

    let name: String = "A Generic Feat".into();
    let generic_feat = Feat {
        name: name.clone(),
        modifiers: FeatModifiers::default(),
        traits: smallvec![FeatTrait::Skill],
        ..Feat::default()
    };
    println!("Initial generic feat: {:?}", generic_feat);
    let specific_key = FeatIndex::NameAndSkill {
        name: name.clone(),
        skill: Skill::Acrobatics,
    };
    println!("Using key: {:?}", specific_key);
    assert!(generic_feat.matches(&specific_key));
    let specific_feat = generic_feat.apply_index(&specific_key);
    assert_eq!(specific_feat.modifiers.skill, Some(Skill::Acrobatics));
    println!("Got specific feat: {:?}", specific_feat);
}

impl fmt::Display for Feat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub enum FeatTrait {
    Ancestry,
    Class,
    Fortune,
    General,
    Skill,
}
