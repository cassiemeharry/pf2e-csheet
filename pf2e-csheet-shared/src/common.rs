#[cfg(test)]
use proptest::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;
#[cfg(feature = "rocket")]
use rocket::request::FromFormValue;
use serde::{de, Deserialize, Serialize};
use smallvec::SmallVec;
use smartstring::alias::String;
use std::{collections::BTreeMap, fmt, str::FromStr};
use thiserror::Error;

use crate::{
    bonuses::{Bonus, Modifier},
    calc::{CalcContext, CalculatedString, Calculation},
    choices::{Choice, ChoiceMeta, ResourceChoices},
    cond::{Condition, Conditions},
    effects::{Effect, Effects},
    stats::{Ability, Level, Proficiency, Skill},
};

mod rref;

pub use rref::{ResourceRef, TypedRef};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
#[serde(rename_all = "kebab-case")]
pub enum ResourceType {
    Action,
    Ancestry,
    Background,
    Class,
    ClassFeature,
    Feat,
    Heritage,
    Item,
    Spell,
}

#[derive(Copy, Clone, Debug, Error)]
#[error("Failed to parse resource type")]
pub struct ResourceTypeFromStrError;

impl FromStr for ResourceType {
    type Err = ResourceTypeFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match crate::parsers::resource_type(s) {
            Ok(rt) => Ok(rt),
            Err(e) => {
                error!("Failed to parse resource type from {:?}: {}", s, e);
                Err(ResourceTypeFromStrError)
            }
        }
    }
}

#[cfg(feature = "rocket")]
impl<'v> FromFormValue<'v> for ResourceType {
    type Error = ();

    fn from_form_value(form_value: &'v rocket::http::RawStr) -> Result<Self, ()> {
        let decoded = form_value.url_decode().map(|s| s.to_lowercase());
        let decoded_str = decoded.as_ref().map(|s| s.as_str());
        match decoded_str {
            Ok("action") => Ok(Self::Action),
            Ok("ancestry") => Ok(Self::Ancestry),
            Ok("background") => Ok(Self::Background),
            Ok("class") => Ok(Self::Class),
            Ok("class-feature") => Ok(Self::ClassFeature),
            Ok("feat") => Ok(Self::Feat),
            Ok("heritage") => Ok(Self::Heritage),
            Ok("item") => Ok(Self::Item),
            Ok("spell") => Ok(Self::Spell),
            _ => Err(()),
        }
    }
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name: &'static str = match self {
            Self::Action => "action",
            Self::Ancestry => "ancestry",
            Self::Background => "background",
            Self::Class => "class",
            Self::ClassFeature => "class feature",
            Self::Feat => "feat",
            Self::Heritage => "heritage",
            Self::Item => "item",
            Self::Spell => "spell",
        };
        write!(f, "{}", name)
    }
}

pub trait HasResourceType: Into<Resource> {
    const RESOURCE_TYPE: ResourceType;
}

macro_rules! impl_has_resource_type {
    ($name:ident) => {
        impl Into<Resource> for $name {
            fn into(self) -> Resource {
                Resource::$name(self)
            }
        }

        impl HasResourceType for $name {
            const RESOURCE_TYPE: ResourceType = ResourceType::$name;
        }
    };
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct ResourceCommon {
    #[cfg_attr(
        test,
        proptest(strategy = "any::<std::string::String>().prop_map_into()")
    )]
    pub name: String,
    #[serde(default)]
    #[cfg_attr(
        test,
        proptest(
            strategy = "proptest::collection::vec(any::<std::string::String>().prop_map_into(), 0..=5)"
        )
    )]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub traits: Vec<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<CalculatedString>,
    #[serde(default)]
    #[serde(skip_serializing_if = "ResourceChoices::is_empty")]
    pub choices: ResourceChoices,
    #[serde(default)]
    #[serde(skip_serializing_if = "Effects::is_empty")]
    pub effects: Effects,
    /// `prerequisites` are conditions that must be met before being allowed to
    /// choose this resource under typical conditions.
    #[serde(default)]
    #[serde(skip_serializing_if = "Conditions::is_none")]
    pub prerequisites: Conditions,
    /// `requirements` are conditions that must be met before this resource's
    /// effects can apply under normal conditions.
    #[serde(default)]
    #[serde(skip_serializing_if = "Conditions::is_none")]
    pub requirements: Conditions,
}

impl fmt::Display for ResourceCommon {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.name, f)
    }
}

impl ResourceCommon {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            traits: vec![],
            description: None,
            choices: ResourceChoices::default(),
            effects: Effects::default(),
            prerequisites: Conditions::default(),
            requirements: Conditions::default(),
        }
    }

    fn get_modifier(&self, label: &str, ctx: CalcContext<'_>) -> Modifier {
        let mut modifier = Modifier::new();
        if self.requirements.reject(ctx) {
            return modifier;
        }

        for effect in self.effects.iter() {
            let common_e = effect.common();
            if common_e.conditions.reject(ctx) {
                continue;
            }
            match effect.get_modifier(label, ctx) {
                Ok(e_mod) => modifier += e_mod,
                Err(err) => error!("Failed to get modifier from effect {:?}: {}", effect, err),
            }
        }
        modifier
    }

    pub fn get_choice<T: serde::de::DeserializeOwned>(
        self: &Self,
        name: &Choice,
        context: CalcContext<'_>,
    ) -> Option<T> {
        let meta = self.choices.get(name)?;
        let res: &ResourceRef = match meta.resource() {
            None => context.rref,
            Some(res_ref) => res_ref,
        };
        context.character.get_choice(res, name)
    }

    pub fn all_choices(&self) -> impl Iterator<Item = (&Choice, &ChoiceMeta)> + '_ {
        self.choices.iter()
    }

    pub fn effects(&self) -> impl Iterator<Item = &Effect> + '_ {
        self.effects.iter()
    }

    pub fn granted_resources<'a>(
        &'a self,
        ctx: CalcContext<'a>,
    ) -> impl Iterator<Item = ResourceRef> + 'a {
        self.effects().filter_map(move |effect| match effect {
            Effect::GrantResourceChoice(e) => ctx
                .character
                .get_choice::<ResourceRef, _>(ctx.rref, &e.choice),
            Effect::GrantSpecificResource(e) => Some(e.resource.clone()),
            _ => None,
        })
    }

    pub fn set_description(&mut self, desc: CalculatedString) {
        self.description = Some(desc);
    }

    pub fn add_choice(&mut self, name: impl Into<Choice>, details: ChoiceMeta) {
        self.choices.add(name, details);
    }

    pub fn add_effect(&mut self, effect: impl Into<Effect>) {
        self.effects.add_effect(effect.into());
    }

    pub fn add_prerequisite(&mut self, prereq: Condition) {
        self.prerequisites &= prereq;
    }

    pub fn add_requirement(&mut self, req: Condition) {
        self.requirements &= req;
    }

    pub fn add_traits(&mut self, traits: &[impl Into<String> + Clone]) {
        for t in traits {
            let t = t.clone().into();
            if !self.traits.iter().any(|this_t| this_t == &t) {
                self.traits.push(t);
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
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
    #[serde(rename = "heritage")]
    Heritage(Heritage),
    #[serde(rename = "item")]
    Item(Item),
    #[serde(rename = "spell")]
    Spell(Spell),
}

#[cfg(test)]
impl Arbitrary for Resource {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        prop_oneof![
            any::<Ancestry>().prop_map(Resource::Ancestry),
            any::<Action>().prop_map(Resource::Action),
            any::<Background>().prop_map(Resource::Background),
            any::<Class>().prop_map(Resource::Class),
            any::<ClassFeature>().prop_map(Resource::ClassFeature),
            any::<Feat>().prop_map(Resource::Feat),
            any::<Heritage>().prop_map(Resource::Heritage),
            any::<Item>().prop_map(Resource::Item),
            any::<Spell>().prop_map(Resource::Spell),
        ]
        .boxed()
    }
}

impl fmt::Display for Resource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.common(), f)
    }
}

impl Resource {
    pub fn common(&self) -> &ResourceCommon {
        match self {
            Self::Ancestry(r) => &r.common,
            Self::Action(r) => &r.common,
            Self::Background(r) => &r.common,
            Self::Class(r) => &r.common,
            Self::ClassFeature(r) => &r.common,
            Self::Feat(r) => &r.common,
            Self::Heritage(r) => &r.common,
            Self::Item(r) => &r.common,
            Self::Spell(r) => &r.common,
        }
    }

    pub fn make_rref_no_mod(&self) -> ResourceRef {
        let name = self.common().name.as_str();
        let rtype = self.resource_type();
        ResourceRef::new(name, None::<&str>).with_type(Some(rtype))
    }

    pub fn resource_type(&self) -> ResourceType {
        match self {
            Self::Ancestry(_) => ResourceType::Ancestry,
            Self::Action(_) => ResourceType::Action,
            Self::Background(_) => ResourceType::Background,
            Self::Class(_) => ResourceType::Class,
            Self::ClassFeature(_) => ResourceType::ClassFeature,
            Self::Feat(_) => ResourceType::Feat,
            Self::Heritage(_) => ResourceType::Heritage,
            Self::Item(_) => ResourceType::Item,
            Self::Spell(_) => ResourceType::Spell,
        }
    }

    pub(crate) fn get_modifier(&self, name: &str, ctx: CalcContext<'_>) -> Modifier {
        let mut m = self.common().get_modifier(name, ctx);
        match self {
            Self::Class(cls) => m += cls.get_modifier(name, ctx),
            _ => (),
        }
        m
    }

    pub fn all_choices(&self) -> impl Iterator<Item = (&Choice, &ChoiceMeta)> + '_ {
        self.common().all_choices()
    }

    pub fn granted_resources(&self, ctx: CalcContext<'_>) -> Vec<ResourceRef> {
        let mut rrefs: Vec<ResourceRef> = self.common().granted_resources(ctx).collect();
        match self {
            Self::Class(class) => rrefs.extend(class.granted_resources(ctx)),
            _ => (),
        }
        rrefs
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum ActionType {
    #[serde(rename = "free action")]
    #[serde(alias = "free")]
    Free,
    #[serde(rename = "reaction")]
    Reaction,
    #[serde(rename = "one action")]
    #[serde(alias = "one")]
    #[serde(alias = "1")]
    One,
    #[serde(rename = "two actions")]
    #[serde(alias = "two")]
    #[serde(alias = "2")]
    Two,
    #[serde(rename = "three actions")]
    #[serde(alias = "three")]
    #[serde(alias = "3")]
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

#[derive(Debug, Error)]
#[error("Unexpected action type")]
pub struct ActionTypeFromStrError;

impl FromStr for ActionType {
    type Err = ActionTypeFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "free" | "free action" => Ok(Self::Free),
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
            _ => Err(ActionTypeFromStrError),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Action {
    #[serde(flatten)]
    pub common: ResourceCommon,
    #[serde(rename = "type")]
    pub action_type: ActionType,
}

impl_has_resource_type!(Action);

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Ancestry {
    #[serde(flatten)]
    common: ResourceCommon,
}

impl_has_resource_type!(Ancestry);

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Background {
    #[serde(flatten)]
    pub common: ResourceCommon,
}

impl_has_resource_type!(Background);

#[derive(Clone, Debug, Eq, PartialEq, Default, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct ClassWeaponProficiencies {
    #[serde(default)]
    #[serde(skip_serializing_if = "crate::is_default")]
    pub unarmed: Proficiency,
    #[serde(default)]
    #[serde(skip_serializing_if = "crate::is_default")]
    pub simple: Proficiency,
    #[serde(default)]
    #[serde(skip_serializing_if = "crate::is_default")]
    pub martial: Proficiency,
    #[serde(default)]
    #[serde(skip_serializing_if = "crate::is_default")]
    pub advanced: Proficiency,
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct ClassArmorProficiencies {
    #[serde(default)]
    #[serde(skip_serializing_if = "crate::is_default")]
    pub unarmored: Proficiency,
    #[serde(default)]
    #[serde(skip_serializing_if = "crate::is_default")]
    pub light: Proficiency,
    #[serde(default)]
    #[serde(skip_serializing_if = "crate::is_default")]
    pub medium: Proficiency,
    #[serde(default)]
    #[serde(skip_serializing_if = "crate::is_default")]
    pub heavy: Proficiency,
}
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Class {
    #[serde(flatten)]
    pub common: ResourceCommon,
    #[serde(rename = "key ability")]
    #[cfg_attr(
        test,
        proptest(strategy = "proptest::collection::vec(any::<Ability>(), 0..=6).prop_map_into()")
    )]
    pub key_ability: SmallVec<[Ability; 2]>,
    #[serde(rename = "hp per level")]
    pub hp_per_level: Calculation,
    #[cfg_attr(
        test,
        proptest(
            strategy = "proptest::collection::btree_map(any::<Level>(), proptest::collection::vec(any::<TypedRef<ClassFeature>>(), 1..=5), 5..=15)"
        )
    )]
    pub advancement: BTreeMap<Level, Vec<TypedRef<ClassFeature>>>,
}

impl_has_resource_type!(Class);

impl Class {
    fn granted_resources(&self, ctx: CalcContext<'_>) -> impl Iterator<Item = ResourceRef> + '_ {
        let mut rrefs = vec![];
        let cls_level_opt = ctx.character.get_class_and_level();
        match cls_level_opt {
            Some((other_class, current_level)) if &other_class.name == &self.common.name => {
                debug!("Granting resources up to level {}", current_level);
                for (level, cf_refs) in self.advancement.iter() {
                    if level > &current_level {
                        continue;
                    }
                    debug!("Granting level {} class features: {:?}", level, cf_refs);
                    rrefs.extend(cf_refs.iter().map(|tr| tr.clone().as_runtime()));
                }
            }
            Some((other_class, _current_level)) => {
                debug!(
                    "Not granting any resources, character has different class (expected {:?}, found {:?})",
                    self.common.name,
                    other_class.name,
                );
            }
            None => {
                debug!(
                    "Not granting any resources, character has no class (expected {:?})",
                    self.common.name,
                );
            }
        }
        rrefs.into_iter()
    }

    fn get_modifier(&self, label: &str, ctx: CalcContext<'_>) -> Modifier {
        let cls_rref = ResourceRef::new(self.common.name.as_str(), None::<&str>)
            .with_type(Some(ResourceType::Class));
        let level: Level = ctx
            .character
            .get_choice(&cls_rref, "Level")
            .unwrap_or_default();
        match label {
            "Max HP" => {
                let per_level_val = self.hp_per_level.evaluate(ctx).max(1);
                let per_level_bonus = Bonus::untyped(per_level_val);
                (per_level_bonus * level).into()
            }
            _ => Modifier::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct ClassFeature {
    #[serde(flatten)]
    pub common: ResourceCommon,
    // pub details: ClassFeatureDetails,
    pub class: TypedRef<Class>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct ClassInitialProficiencies {
    #[serde(skip_serializing_if = "crate::is_default")]
    pub perception: Proficiency,
    #[serde(rename = "fort save")]
    #[serde(skip_serializing_if = "crate::is_default")]
    pub fort_save: Proficiency,
    #[serde(rename = "reflex save")]
    #[serde(skip_serializing_if = "crate::is_default")]
    pub reflex_save: Proficiency,
    #[serde(rename = "will save")]
    #[serde(skip_serializing_if = "crate::is_default")]
    pub will_save: Proficiency,
    #[serde(rename = "skills trained")]
    #[serde(default)]
    #[cfg_attr(
        test,
        proptest(strategy = "proptest::collection::vec(any::<Skill>(), 0..4).prop_map_into()")
    )]
    #[serde(skip_serializing_if = "crate::is_default")]
    pub skills_trained: SmallVec<[Skill; 4]>,
    #[serde(rename = "free skills trained")]
    #[serde(alias = "free skill trained")]
    #[serde(default)]
    #[serde(skip_serializing_if = "crate::is_default")]
    pub free_skill_trained: u16,
    #[serde(default, rename = "weapon proficiencies")]
    #[serde(skip_serializing_if = "crate::is_default")]
    pub weapon_proficiencies: ClassWeaponProficiencies,
    #[serde(default, rename = "armor proficiencies")]
    #[serde(skip_serializing_if = "crate::is_default")]
    pub armor_proficiencies: ClassArmorProficiencies,
    #[serde(rename = "class DC")]
    #[serde(alias = "class dc")]
    #[serde(skip_serializing_if = "crate::is_default")]
    pub class_dc: Proficiency,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum ClassFeatureDetails {
    AncestryAndBackground,
    InitialProficiencies(ClassInitialProficiencies),
    Feat,
    SkillIncrease,
    AbilityBoosts,
    Other,
}

impl_has_resource_type!(ClassFeature);

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Feat {
    #[serde(flatten)]
    pub common: ResourceCommon,
    #[serde(default)]
    pub level: Level,
}

impl_has_resource_type!(Feat);

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Heritage {
    #[serde(flatten)]
    common: ResourceCommon,
    ancestry: TypedRef<Heritage>,
}

impl_has_resource_type!(Heritage);

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Item {
    #[serde(flatten)]
    common: ResourceCommon,
    #[serde(default)]
    level: Level,
}

impl_has_resource_type!(Item);

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Spell {
    #[serde(flatten)]
    common: ResourceCommon,
    #[serde(default)]
    level: Level,
}

impl_has_resource_type!(Spell);

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn add_line_numbers(s: &str) -> std::string::String {
        use std::fmt::Write;
        let mut buffer = std::string::String::with_capacity(s.len());
        for (i, line) in s.lines().enumerate() {
            writeln!(&mut buffer, "{:>4} | {}", i + 1, line).unwrap();
        }
        buffer
    }

    fn init_logging() {
        let mut builder = pretty_env_logger::formatted_timed_builder();
        builder.is_test(true);

        if let Ok(s) = std::env::var("RUST_LOG") {
            builder.parse_filters(&s);
        }

        let _ = builder.try_init();
    }

    fn compare_json(left: &str, right: &str) {
        let left_value = serde_json::from_str::<serde_json::Value>(left).unwrap();
        let right_value = serde_json::from_str::<serde_json::Value>(right).unwrap();
        assert_eq!(left_value, right_value);
    }

    macro_rules! make_roundtrip_proptest {
        ($test_name:ident : $t:ty) => {
            proptest! {
                #[test]
                fn $test_name(x_1 in any::<$t>()) {
                    init_logging();

                    let serialized_1 = serde_json::to_string_pretty(&x_1).unwrap();
                    let x_2 = match serde_json::from_str::<$t>(&serialized_1) {
                        Ok(r) => r,
                        Err(e) => {
                            panic!(
                                "Failed to parse first round of serialization:\nError:\n{}\n--------------------\n{}\n--------------------\n",
                                e, add_line_numbers(&serialized_1),
                            )
                        }
                    };
                    assert_eq!(&x_1, &x_2, "Serialized was:\n--------------------\n{}\n--------------------\n", &serialized_1);
                    let serialized_2 = serde_json::to_string_pretty(&x_2).unwrap();
                    compare_json(&serialized_1, &serialized_2);
                }
            }
        };
    }

    make_roundtrip_proptest!(roundtrip_rref: ResourceRef);
    make_roundtrip_proptest!(roundtrip_typed_ref: TypedRef<Action>);

    make_roundtrip_proptest!(roundtrip_bonus: crate::bonuses::Bonus);
    make_roundtrip_proptest!(roundtrip_penalty: crate::bonuses::Penalty);
    make_roundtrip_proptest!(roundtrip_modifier: crate::bonuses::Modifier);

    make_roundtrip_proptest!(roundtrip_calculation: Calculation);
    make_roundtrip_proptest!(roundtrip_calc_string: CalculatedString);
    make_roundtrip_proptest!(roundtrip_conditions: Conditions);
    make_roundtrip_proptest!(roundtrip_effect: Effect);
    make_roundtrip_proptest!(roundtrip_skill: Skill);

    // make_roundtrip_proptest!(roundtrip_ancestry: Ancestry);
    // make_roundtrip_proptest!(roundtrip_action: Action);
    // make_roundtrip_proptest!(roundtrip_background: Background);
    // // make_roundtrip_proptest!(roundtrip_class: Class);
    // // make_roundtrip_proptest!(roundtrip_classfeature: ClassFeature);
    // make_roundtrip_proptest!(roundtrip_feat: Feat);
    // make_roundtrip_proptest!(roundtrip_heritage: Heritage);
    // make_roundtrip_proptest!(roundtrip_item: Item);
    // make_roundtrip_proptest!(roundtrip_spell: Spell);

    // make_roundtrip_proptest!(roundtrip_resource: Resource);
}
