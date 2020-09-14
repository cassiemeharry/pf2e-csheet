use anyhow::{Error, Result};
use lazy_static::lazy_static;
use smallvec::SmallVec;
use smartstring::alias::String;
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use crate::{Ability, Ancestry, Background, Character, Feat, Heritage};

pub enum ChoiceQuestion {
    Ancestry,
    Heritage,
    Background,
    AbilityBoost(Option<(Ability, Ability)>),
    Class,
    ClassFeat,
    ClassKeyAbilityScore(Ability, Ability),
}

pub enum ChoiceAnswer {
    Ancestry(Ancestry),
    Heritage(Heritage),
    Background(Background),
    AbilityBoost(Ability),
    Class(String),
    ClassFeat(Feat),
    ClassKeyAbilityScore(Ability),
}

pub trait HasChoices {
    fn choices(&self, character: &Character) -> Vec<(Level, ChoiceQuestion)>;
}
