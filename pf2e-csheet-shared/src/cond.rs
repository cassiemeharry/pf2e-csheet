use derive_more::From;
#[cfg(test)]
use proptest::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use smartstring::alias::String;
use std::collections::HashMap;

use crate::{
    calc::CalcContext,
    common::ResourceRef,
    items::{ArmorCategory, ItemType},
    stats::Proficiency,
};

#[derive(Debug, Deserialize, Default, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
struct AllSingleConditions {
    #[serde(default, rename = "armor category")]
    #[serde(skip_serializing_if = "Option::is_none")]
    armor_category: Option<ArmorCategory>,
    #[serde(default, rename = "have resource")]
    #[serde(alias = "have class")]
    #[serde(alias = "have feat")]
    #[serde(alias = "have item")]
    #[serde(alias = "have spell")]
    #[serde(skip_serializing_if = "Option::is_none")]
    have_resource: Option<ResourceRef>,
    #[serde(default, rename = "item trait")]
    #[serde(skip_serializing_if = "Option::is_none")]
    item_trait: Option<ItemHasTraitCondition>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    proficiency: Option<ProficiencyCondition>,

    #[serde(default, rename = "unenforced (unknown)")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        test,
        proptest(strategy = "proptest::option::of(any::<std::string::String>().prop_map_into())")
    )]
    unenforced_unknown: Option<String>,

    #[serde(default, rename = "unenforced")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        test,
        proptest(strategy = "proptest::option::of(any::<std::string::String>().prop_map_into())")
    )]
    unenforced_known: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct UnenforcedCondition {
    known: bool,
    #[cfg_attr(
        test,
        proptest(strategy = "any::<std::string::String>().prop_map_into()")
    )]
    text: String,
}

impl UnenforcedCondition {
    pub fn known(text: impl Into<String>) -> Self {
        Self {
            known: true,
            text: text.into(),
        }
    }

    pub fn unknown(text: impl Into<String>) -> Self {
        Self {
            known: false,
            text: text.into(),
        }
    }
}

impl<T: Into<String>> From<T> for UnenforcedCondition {
    fn from(text: T) -> Self {
        Self::unknown(text)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, From)]
pub enum SingleCondition {
    ArmorCategory(ArmorCategory),
    ItemHasTrait(ItemHasTraitCondition),
    Proficiency(ProficiencyCondition),
    HaveResource(ResourceRef),
    #[from(ignore)]
    Unenforced(UnenforcedCondition),
}

impl<T: Into<UnenforcedCondition>> From<T> for SingleCondition {
    fn from(uc: T) -> Self {
        SingleCondition::Unenforced(uc.into())
    }
}

impl From<SingleCondition> for AllSingleConditions {
    fn from(sc: SingleCondition) -> Self {
        let mut all = Self::default();
        match sc {
            SingleCondition::ArmorCategory(ac) => all.armor_category = Some(ac),
            SingleCondition::HaveResource(r) => all.have_resource = Some(r),
            SingleCondition::ItemHasTrait(s) => all.item_trait = Some(s),
            SingleCondition::Proficiency(pc) => all.proficiency = Some(pc),
            SingleCondition::Unenforced(u) => {
                if u.known {
                    all.unenforced_known = Some(u.text);
                } else {
                    all.unenforced_unknown = Some(u.text);
                }
            }
        }
        all
    }
}

impl From<AllSingleConditions> for Condition {
    fn from(asc: AllSingleConditions) -> Self {
        let mut cond = Self::None;
        macro_rules! field {
            ($f:ident, $convert:expr) => {
                if let Some(x) = asc.$f {
                    cond &= ($convert(x)).into();
                }
            };
        }
        field!(armor_category, SingleCondition::ArmorCategory);
        field!(have_resource, SingleCondition::HaveResource);
        field!(item_trait, SingleCondition::ItemHasTrait);
        field!(proficiency, SingleCondition::Proficiency);
        field!(unenforced_unknown, UnenforcedCondition::unknown);
        field!(unenforced_known, UnenforcedCondition::known);

        cond
    }
}

impl SingleCondition {
    fn reject(&self, _ctx: CalcContext<'_>) -> bool {
        match self {
            Self::ArmorCategory(ac) => todo!("ArmorCategory({:?}).reject", ac),
            Self::HaveResource(r) => todo!("HaveResource({}).reject", r),
            Self::ItemHasTrait(t) => todo!("ItemHasTrait({:?}).reject", t),
            Self::Proficiency(c) => todo!("Proficiency({:?}).reject", c),
            Self::Unenforced(_) => false,
        }
    }
}

#[derive(Debug, Deserialize, Default, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
struct AllConditions {
    #[serde(default, rename = "NOT")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        test,
        proptest(
            strategy = "proptest::option::of(any::<Condition>()).prop_map(|c_opt| c_opt.map(Box::new))"
        )
    )]
    not: Option<Box<Condition>>,
    #[serde(default, rename = "OR")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        test,
        proptest(strategy = "proptest::collection::vec(any::<Condition>(), 0..=5)")
    )]
    or: Vec<Condition>,
    #[serde(default, rename = "AND")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(
        test,
        proptest(strategy = "proptest::collection::vec(any::<Condition>(), 0..=5)")
    )]
    and: Vec<Condition>,

    #[serde(flatten)]
    singles: AllSingleConditions,

    #[serde(flatten)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[cfg_attr(
        test,
        proptest(
            strategy = "proptest::collection::hash_map(any::<std::string::String>().prop_map_into(), any::<i32>().prop_map_into(), 0..=3)"
        )
    )]
    other: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, From, Deserialize, Serialize)]
#[serde(try_from = "AllConditions", into = "AllConditions")]
pub enum Condition {
    None,
    Negate(Box<Condition>),
    Or(Vec<Condition>),
    And(Vec<Condition>),
    #[from(forward)]
    Single(SingleCondition),
}

impl std::ops::BitAnd for Condition {
    type Output = Condition;

    fn bitand(mut self, rhs: Self) -> Self {
        self &= rhs;
        self
    }
}

impl std::ops::BitAndAssign for Condition {
    fn bitand_assign(&mut self, rhs: Self) {
        let mut this = Self::None;
        std::mem::swap(self, &mut this);
        this = match (this, rhs) {
            (Self::None, rhs) => rhs,
            (lhs, Self::None) => lhs,
            (Self::And(mut l), Self::And(r)) => {
                l.extend(r);
                Self::And(l)
            }
            (Self::And(mut l), rhs) => {
                l.push(rhs);
                Self::And(l)
            }
            (lhs, Self::And(mut r)) => {
                r.insert(0, lhs);
                Self::And(r)
            }
            (left, right) => Self::And(vec![left, right]),
        };
        std::mem::swap(self, &mut this);
    }
}

#[cfg(test)]
impl Arbitrary for Condition {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        let leaf = prop_oneof![
            Just(Condition::None),
            any::<std::string::String>()
                .prop_map_into()
                .prop_map(Condition::ItemHasTrait),
            arb_proficiency_condition().prop_map(Condition::Proficiency),
        ];
        leaf.prop_recursive(
            3,  // levels deep
            20, // maximum node count
            5,  // items per collection
            |inner| {
                prop_oneof![
                    inner.clone().prop_map(|c| Condition::Negate(Box::new(c))),
                    prop::collection::vec(inner.clone(), 2..10).prop_map(Condition::Or),
                    prop::collection::vec(inner.clone(), 2..10).prop_map(Condition::And),
                ]
            },
        )
        .boxed()
    }
}

impl From<Condition> for AllConditions {
    fn from(c: Condition) -> AllConditions {
        let mut all = AllConditions::default();
        match c {
            Condition::None => (),
            Condition::Negate(neg) => all.not = Some(neg),
            Condition::Or(conds) => all.or = conds,
            Condition::And(conds) => all.and = conds,
            Condition::Single(sc) => all.singles = sc.into(),
        };
        all
    }
}

impl From<AllConditions> for Condition {
    fn from(ac: AllConditions) -> Condition {
        #[cfg(test)]
        trace!(
            "Converting an AllConditions struct into Condition: {:#?}",
            ac
        );

        // Bet that most condition maps will have one entry, and delay
        // allocating until we discover otherwise.
        let mut parts: SmallVec<[Condition; 1]> = smallvec![];

        if let Some(cond) = ac.not {
            parts.push(Condition::Negate(cond));
        }
        if !ac.or.is_empty() {
            parts.push(Condition::Or(ac.or));
        }
        if !ac.and.is_empty() {
            parts.push(Condition::And(ac.and));
        }
        match ac.singles.into() {
            Self::None => (),
            singles => parts.push(singles),
        }
        if !ac.other.is_empty() {
            let other_keys = ac.other.keys().collect::<SmallVec<[&String; 1]>>();
            if cfg!(debug_assertions) || cfg!(test) {
                panic!("Found unexpected keys in conditions map: {:?}", other_keys)
            } else {
                warn!("Skipped parsing conditions {:?}", other_keys);
            }
        }

        match parts.len() {
            0 => Condition::None,
            1 => parts.pop().unwrap(),
            _ => Condition::And(parts.to_vec()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct ItemHasTraitCondition {
    #[serde(skip_serializing_if = "crate::is_default")]
    item_slots: ItemType,
    #[cfg_attr(
        test,
        proptest(
            strategy = "proptest::collection::vec(any::<std::string::String>().prop_map_into(), 0..=2).prop_map_into()"
        )
    )]
    item_traits: SmallVec<[String; 1]>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
#[serde(untagged)]
pub enum ProficiencyCondition {
    AtLeast {
        #[serde(rename = "in")]
        #[cfg_attr(
            test,
            proptest(strategy = "any::<std::string::String>().prop_map_into()")
        )]
        target: String,
        #[serde(rename = "at least")]
        at_least: Proficiency,
    },
    Exactly {
        #[serde(rename = "in")]
        #[cfg_attr(
            test,
            proptest(strategy = "any::<std::string::String>().prop_map_into()")
        )]
        target: String,
        exactly: Proficiency,
    },
}

#[cfg(test)]
pub(crate) fn arb_proficiency_condition() -> impl Strategy<Value = ProficiencyCondition> {
    prop_oneof![
        (
            any::<std::string::String>().prop_map_into(),
            any::<Proficiency>(),
        )
            .prop_map(|(target, at_least)| ProficiencyCondition::AtLeast { target, at_least }),
        (
            any::<std::string::String>().prop_map_into(),
            any::<Proficiency>(),
        )
            .prop_map(|(target, exactly)| ProficiencyCondition::Exactly { target, exactly }),
    ]
}

impl Condition {
    fn reject(&self, ctx: CalcContext<'_>) -> bool {
        match self {
            Self::None => false,
            Self::Negate(c) => c.reject(ctx),
            Self::Or(conds) => conds.iter().any(|c| c.reject(ctx)),
            Self::And(conds) => conds.iter().all(|c| c.reject(ctx)),
            Self::Single(sc) => sc.reject(ctx),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Arbitrary))]
#[serde(transparent)]
pub struct Conditions {
    inner: Condition,
}

impl Conditions {
    pub fn reject(&self, ctx: CalcContext<'_>) -> bool {
        self.inner.reject(ctx)
    }

    pub fn is_none(&self) -> bool {
        match self.inner {
            Condition::None => true,
            _ => false,
        }
    }
}

impl Default for Conditions {
    fn default() -> Self {
        Self {
            inner: Condition::None,
        }
    }
}

impl std::ops::BitAndAssign<Condition> for Conditions {
    fn bitand_assign(&mut self, rhs: Condition) {
        self.inner &= rhs;
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[inline]
//     fn wrap_single(c: Condition) -> Conditions {
//         Conditions { inner: c }
//     }

//     #[test]
//     fn deserialize_proficiency_at_least() {
//         let raw = r###"\
// { "proficiency":
//   { "in": "martial weapons"
//   , "at least": "master"
//   }
// }"###;
//         let parsed: Conditions = serde_json::from_str(raw).unwrap();
//         let expected = Condition::Proficiency(ProficiencyCondition::AtLeast {
//             target: "martial weapons".into(),
//             at_least: Proficiency::Master,
//         });
//         assert_eq!(parsed, wrap_single(expected));
//     }
// }
