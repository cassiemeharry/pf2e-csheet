use serde::{de::DeserializeOwned, Deserialize};
use smartstring::alias::String;
use std::{
    any::Any,
    collections::HashMap,
    fmt::{Debug, Display},
    hash::Hash,
    sync::Arc,
};

use crate::qa;

pub trait ValidIndex:
    Any + Sized + Send + Sync + Clone + Debug + Hash + Eq + DeserializeOwned
{
}

impl<T> ValidIndex for T where
    T: Any + Sized + Send + Sync + Clone + Debug + Hash + Eq + DeserializeOwned
{
}

pub trait ResourceExtra: Any + Sized + Send + Sync + Clone + Debug + Eq + DeserializeOwned {
    type Index: ValidIndex;

    fn apply_index(&mut self, index: &Self::Index);

    fn index_matches(&self, index: &Self::Index) -> bool;
}

impl ResourceExtra for () {
    type Index = ();

    fn apply_index(&mut self, index: &I) {}
}

pub trait Resource: Send + Sync + Any + Clone + Display + DeserializeOwned {
    type Extra: ResourceExtra = ();

    fn get_name(&self) -> &str;

    // /// Build the common index from this resource.
    // fn get_index_value(&self, extra: &Self::Extra) -> Self::Index;

    // /// Determine whether this resource matches an index value.
    // fn matches(&self, extra: &Self::Extra, index: &Self::Index) -> bool {
    //     let index_value = self.get_index_value(extra);
    //     index == &index_value
    // }

    fn get_questions(&self) -> Vec<qa::Question> {
        vec![]
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(bound = "R: Resource")]
pub struct ResourceInstance<R: Resource> {
    definition: Arc<R>,
    extra: R::Extra,
    questions: Vec<qa::Question>,
    answers: HashMap<String, qa::Answer>,
}

impl<R: Resource + ?Sized> ResourceInstance<R> {
    pub fn get_index_value(&self) -> <<R as Resource>::Extra as ResourceExtra>::Index {
        self.definition.get_index_value(&self.extra)
    }

    pub fn matches(&self, index: &R::Index) -> bool {
        self.definition.matches(&self.extra, index)
    }
}

// pub trait ResourceLoadable<Index: ValidIndex>: Resource {
//     /// Attempt to build an index from this resource.
//     ///
//     /// This `Index` type may not apply to this specific resource, in which case
//     /// this function would return `None`. For example, some feats are "skill"
//     /// feats. For an index that includes a `Skill`, this function should only
//     /// return `Some(index)` if `self` is a skill feat associated with a given
//     /// skill:
//     ///
//     /// ```
//     /// # use smartstring::alias::String;
//     /// # use crate::stats::Skill;
//     /// #
//     /// # impl Resource for Feat {
//     /// #     type CommonIndex = String;
//     /// #     fn get_index_value(&self) -> String { self.name.clone() }
//     /// # }
//     /// #
//     /// struct Feat {
//     ///     name: String,
//     ///     skill: Option<Skill>,
//     ///     is_skill_feat: bool,
//     /// }
//     ///
//     /// impl ResourceLoadable<(String, Skill)> for Feat {
//     ///     fn get_index_value(&self) -> Option<(String, Skill)> {
//     ///         if !self.is_skill_feat {
//     ///             return None;
//     ///         }
//     ///         match self.skill.as_ref() {
//     ///             Some(s) => Some((self.name.clone(), s.clone())),
//     ///             None => None,
//     ///         }
//     ///     }
//     ///
//     ///     // ...
//     /// #     fn apply_index(&self, index: &(String, Skill)) -> Option<Self> { None }
//     /// }
//     /// ```
//     fn get_index_value(&self) -> Option<Index>;

//     /// Check whether this specialized index matches this particular resource.
//     ///
//     /// This extends equality in that a generic version of a resource can match
//     /// a specialized index in some cases. For example, some skill feats can
//     /// apply to multiple skills. If `self` is the generic version of one of
//     /// those skill feats, it should match any index with the other components
//     /// matching (such as the feat's name), while letting the skill part vary.
//     fn loadable_matches(&self, index: &Index) -> bool;

//     /// Convert a specialized index to the default, common index for this
//     /// resource type.
//     ///
//     /// Typically, the common index will be the name of the resource, and one of
//     /// the components of a multi-part specialized index.
//     fn to_common_index(index: Index) -> Self::CommonIndex;
// }

// impl<R: Resource> ResourceLoadable<R::CommonIndex> for R {
//     #[inline]
//     fn get_index_value(&self) -> Option<R::CommonIndex> {
//         Some(R::get_index_value(self))
//     }

//     #[inline]
//     fn loadable_matches(&self, index: &R::CommonIndex) -> bool {
//         self.matches(index)
//     }

//     #[inline]
//     fn to_common_index(index: R::CommonIndex) -> R::CommonIndex {
//         index
//     }

//     #[inline]
//     fn apply_index(&self, index: &R::CommonIndex) -> Option<R> {
//         if R::matches(self, index) {
//             Some(self.clone())
//         } else {
//             None
//         }
//     }
// }
