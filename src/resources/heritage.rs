use serde::Deserialize;
use smartstring::alias::String;
use std::fmt;

use crate::resources::{refs::Ref, traits::Resource, Ancestry};

#[derive(Clone, Debug, Deserialize)]
pub struct Heritage {
    name: String,
    ancestry: Ref<Ancestry>,
    description: String,
}

impl Resource for Heritage {
    fn get_index_value(&self) -> String {
        self.name.clone()
    }
}

impl fmt::Display for Heritage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
