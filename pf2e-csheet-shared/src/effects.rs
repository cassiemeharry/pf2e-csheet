use derive_more::From;
#[cfg(test)]
use proptest::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;
use serde::{de, Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use smartstring::alias::String;
use std::{fmt, marker::PhantomData, str::FromStr};
use thiserror::Error;

use crate::{
    bonuses::{Bonus, BonusType, Modifier, Penalty},
    calc::{CalcContext, Calculation},
    choices::Choice,
    common::{ResourceRef, ResourceType},
    cond::Conditions,
    stats::Proficiency,
};

#[derive(Clone, Debug, Eq, PartialEq, From, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum Effect {
    #[serde(rename = "bonus")]
    AddBonus(BonusEffect),
    #[serde(rename = "focus pool")]
    AddFocusPoolPoint(AddFocusPoolPointEffect),
    #[serde(rename = "gain focus pool")]
    AddSingleFocusPoolPoint,
    #[serde(rename = "penalty")]
    AddPenalty(PenaltyEffect),
    #[serde(rename = "choose resource")]
    GrantResourceChoice(GrantResourceChoiceEffect),
    #[serde(rename = "grant resource")]
    #[serde(deserialize_with = "effect_from_name_or_struct::<ResourceRef, _, _>")]
    GrantSpecificResource(GrantSpecificResourceEffect),
    #[serde(rename = "proficiency")]
    #[serde(alias = "add proficiency")]
    #[serde(alias = "gain proficiency")]
    #[serde(alias = "gain proficiency in")]
    IncreaseProficiency(IncreaseProficiencyEffect),
    #[serde(rename = "skill increase")]
    SkillIncrease(SkillIncreaseEffect),
}

impl Effect {
    pub fn common(&self) -> &EffectCommon {
        match self {
            Self::AddBonus(b) => &b.common,
            Self::AddFocusPoolPoint(fpp) => &fpp.common,
            Self::AddSingleFocusPoolPoint => &*EMPTY_EFFECT_COMMON,
            Self::AddPenalty(p) => &p.common,
            Self::GrantSpecificResource(f) => &f.common,
            Self::GrantResourceChoice(rc) => &rc.common,
            Self::IncreaseProficiency(ip) => &ip.common,
            Self::SkillIncrease(s) => &s.common,
        }
    }

    pub fn get_active_resources(&self, ctx: CalcContext<'_>) -> SmallVec<[ResourceRef; 1]> {
        if self.common().conditions.reject(ctx) {
            return smallvec![];
        }

        match self {
            Self::AddBonus(_) => smallvec![],
            Self::AddFocusPoolPoint(_) => smallvec![],
            Self::AddSingleFocusPoolPoint => smallvec![],
            Self::AddPenalty(_) => smallvec![],
            Self::GrantSpecificResource(r) => smallvec![r.resource.clone()],
            Self::GrantResourceChoice(_) => smallvec![],
            Self::IncreaseProficiency(_) => smallvec![],
            Self::SkillIncrease(_) => smallvec![],
        }
    }

    pub fn get_modifier(
        &self,
        label: &str,
        ctx: CalcContext<'_>,
    ) -> Result<Modifier, GetModifierError> {
        match self {
            Self::AddBonus(effect) if label == effect.target.as_str() => {
                let value = effect.value.evaluate(ctx);
                let bonus = Bonus::from_value_type(value, &effect.bonus_type)?;
                Ok(bonus.into())
            }
            Self::AddBonus(_) => Ok(Modifier::new()),
            Self::AddFocusPoolPoint(effect) if label == "Focus Pool Size" => {
                let value = effect.points.evaluate(ctx);
                let bonus = Bonus::untyped(value);
                Ok(bonus.into())
            }
            Self::AddFocusPoolPoint(_) => Ok(Modifier::new()),
            Self::AddSingleFocusPoolPoint if label == "Focus Pool Size" => {
                Ok(Bonus::untyped(1).into())
            }
            Self::AddSingleFocusPoolPoint => Ok(Modifier::new()),
            Self::AddPenalty(effect) => {
                if label == effect.target.as_str() {
                    let value = effect.value.evaluate(ctx);
                    let penalty = Penalty::from_value_type(value, &effect.penalty_type)?;
                    Ok(penalty.into())
                } else {
                    Ok(Modifier::new())
                }
            }
            Self::GrantSpecificResource(_) => Ok(Modifier::new()),
            Self::GrantResourceChoice(_) => Ok(Modifier::new()),
            Self::IncreaseProficiency(_) => Ok(Modifier::new()),
            Self::SkillIncrease(_) => Ok(Modifier::new()),
        }
    }
}

#[derive(Clone, Debug, Error)]
pub enum GetModifierError {
    #[error("Failed to parse modifier")]
    BonusError(#[from] crate::bonuses::FromValueTypeError),
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct EffectCommon {
    // #[serde(flatten)]
    // effect: EffectType,
    #[serde(default)]
    #[serde(skip_serializing_if = "crate::is_default")]
    pub conditions: Conditions,
}

lazy_static::lazy_static! {
    static ref EMPTY_EFFECT_COMMON: EffectCommon = EffectCommon {
        conditions: Conditions::default(),
    };
}

fn effect_from_name_or_struct<'de, R, E, D>(deserializer: D) -> Result<E, D::Error>
where
    D: de::Deserializer<'de>,
    E: de::Deserialize<'de> + From<R>,
    R: FromStr,
    <R as FromStr>::Err: fmt::Display,
{
    struct NameOrStruct<E, R>(PhantomData<fn(R) -> E>);

    impl<'de, E, R> de::Visitor<'de> for NameOrStruct<E, R>
    where
        E: Deserialize<'de> + From<R>,
        R: FromStr,
        <R as FromStr>::Err: fmt::Display,
    {
        type Value = E;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("string or a map")
        }

        fn visit_str<Er>(self, s: &str) -> Result<E, Er>
        where
            Er: de::Error,
        {
            let rref = R::from_str(s).map_err(|e| Er::custom(e))?;
            Ok(rref.into())
        }

        fn visit_map<M>(self, map: M) -> Result<E, M::Error>
        where
            M: de::MapAccess<'de>,
        {
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(NameOrStruct(PhantomData))
}

macro_rules! effect_from_rref {
    ($effect:ty => $field:ident : $rref:ty) => {
        impl From<$rref> for $effect {
            fn from(rref: $rref) -> $effect {
                Self {
                    common: EffectCommon::default(),
                    $field: rref,
                }
            }
        }
    };
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct AddFocusPoolPointEffect {
    #[serde(flatten)]
    pub common: EffectCommon,
    #[serde(default = "default_calc_one")]
    pub points: Calculation,
}

fn default_calc_one() -> Calculation {
    Calculation::from_number(1)
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct BonusEffect {
    #[serde(flatten)]
    pub common: EffectCommon,
    #[serde(rename = "type")]
    pub bonus_type: BonusType,
    #[serde(rename = "to")]
    #[cfg_attr(
        test,
        proptest(strategy = "any::<std::string::String>().prop_map_into()")
    )]
    pub target: String,
    pub value: Calculation,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct PenaltyEffect {
    #[serde(flatten)]
    pub common: EffectCommon,
    #[serde(rename = "type")]
    pub penalty_type: BonusType,
    #[serde(rename = "to")]
    #[cfg_attr(
        test,
        proptest(strategy = "any::<std::string::String>().prop_map_into()")
    )]
    pub target: String,
    pub value: Calculation,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct GrantResourceChoiceEffect {
    #[serde(flatten)]
    pub common: EffectCommon,
    pub choice: Choice,
    #[serde(rename = "type")]
    pub resource_type: ResourceType,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct GrantSpecificResourceEffect {
    #[serde(flatten)]
    pub common: EffectCommon,
    pub resource: ResourceRef,
}

effect_from_rref!(GrantSpecificResourceEffect => resource: ResourceRef);

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct IncreaseProficiencyEffect {
    #[serde(flatten)]
    pub common: EffectCommon,
    #[serde(rename = "in")]
    #[cfg_attr(
        test,
        proptest(strategy = "any::<std::string::String>().prop_map_into()")
    )]
    pub target: String,
    #[serde(alias = "increases to")]
    pub level: Proficiency,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct SkillIncreaseEffect {
    #[serde(flatten)]
    pub common: EffectCommon,
}

#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq, Default, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
#[serde(transparent)]
pub struct Effects(Vec<Effect>);

impl Effects {
    pub fn add_effect(&mut self, e: Effect) {
        self.0.push(e);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Effect> + '_ {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
