use anyhow::{anyhow, Error, Result};
use serde::Deserialize;
use std::str::FromStr;

use crate::try_from_str;

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub enum ArmorCategory {
    Unarmored,
    Light,
    Medium,
    Heavy,
}

try_from_str!(ArmorCategory);

impl FromStr for ArmorCategory {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "" | "none" | "unarmored" => Ok(Self::Unarmored),
            "light" => Ok(Self::Light),
            "medium" => Ok(Self::Medium),
            "heavy" => Ok(Self::Heavy),
            _ => Err(anyhow!("Unknown armor category {:?}", s)),
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Deserialize)]
pub enum ArmorTrait {}
