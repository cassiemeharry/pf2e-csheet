use anyhow::{anyhow, Result};
use serde::Deserialize;
use smartstring::alias::String;
use std::{
    borrow::{Borrow, ToOwned},
    collections::{hash_map::Entry, HashMap},
    fmt,
    str::FromStr,
    sync::{Arc, Mutex},
};

use crate::{
    bonuses::Modifier,
    resources::{calc::Context, Choice, ChoiceRef, Resource, ResourceRef},
};

#[derive(Debug, Deserialize)]
struct ResourceData {
    resource: Arc<Resource>,
    choice_values: HashMap<Choice, String>,
}

#[derive(Debug, Deserialize)]
struct SerializedResourceData {
    resource: ResourceRef,
    #[serde(default)]
    choices: HashMap<Choice, String>,
}

#[derive(Debug, Deserialize)]
struct SerializedCharacter {
    name: String,
    resources: Vec<SerializedResourceData>,
}

impl From<SerializedCharacter> for Character {
    fn from(sc: SerializedCharacter) -> Self {
        let resources = {
            let mut rs = HashMap::new();
            for srd in sc.resources {
                let rref = srd.resource;
                match rref.resource() {
                    Some(r) => {
                        let rd = ResourceData {
                            resource: r,
                            choice_values: srd.choices,
                        };
                        rs.insert(rref, rd);
                    }
                    None => warn!(
                        "Failed to find resource {} when loading character {:?}",
                        rref, sc.name
                    ),
                }
            }
            rs
        };
        Self {
            name: sc.name,
            resources,
            resolving_choices: Mutex::new(vec![]),
            resolving_modifiers: Mutex::new(vec![]),
            resolving_proficiencies: Mutex::new(vec![]),
            resolving_resources: Mutex::new(vec![]),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(from = "SerializedCharacter")]
pub struct Character {
    pub name: String,
    resources: HashMap<ResourceRef, ResourceData>,
    resolving_choices: Mutex<Vec<(ResourceRef, Choice)>>,
    resolving_modifiers: Mutex<Vec<String>>,
    resolving_proficiencies: Mutex<Vec<String>>,
    resolving_resources: Mutex<Vec<ResourceRef>>,
}

impl fmt::Display for Character {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.name, f)
    }
}

impl Character {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            resources: HashMap::new(),
            resolving_choices: Mutex::new(vec![]),
            resolving_modifiers: Mutex::new(vec![]),
            resolving_proficiencies: Mutex::new(vec![]),
            resolving_resources: Mutex::new(vec![]),
        }
    }

    pub fn add_resource(&mut self, rref: &ResourceRef) -> Result<()> {
        match self.resources.entry(rref.clone()) {
            Entry::Occupied(slot) => Err(anyhow!(
                "Tried to add resource {} multiple times",
                slot.key()
            )),
            Entry::Vacant(slot) => match slot.key().resource() {
                Some(r) => {
                    let rd = ResourceData {
                        resource: r,
                        choice_values: HashMap::new(),
                    };
                    slot.insert(rd);
                    Ok(())
                }
                None => Err(anyhow!(
                    "Tried to add unloaded resource {} to character",
                    slot.key()
                )),
            },
        }
    }

    pub fn set_choice(
        &mut self,
        rref: &ResourceRef,
        choice: impl Into<Choice>,
        value: impl Into<String>,
    ) {
        let rd = match self.resources.get_mut(rref) {
            None => return,
            Some(rd) => rd,
        };
        let key = choice.into();
        rd.choice_values.insert(key.into(), value.into());
    }

    fn checked_recurse<T, U, F, G>(&self, label: &'static str, key: &T, get_lock: G, f: F) -> U
    where
        T: Clone + std::fmt::Debug + std::cmp::Eq,
        F: FnOnce() -> U,
        G: Fn(&Character) -> &Mutex<Vec<T>>,
    {
        trace!("In checked recurse");
        {
            let mut entries = get_lock(self).lock().unwrap();
            if entries.contains(key) {
                panic!(
                    "Attempted to evaluate {} {:?} multiple times! Path was {:?}",
                    label, key, &*entries
                );
            }
            entries.push(key.clone());
        }
        let result = f();
        {
            let mut entries = get_lock(self).lock().unwrap();
            let popped = entries.pop().unwrap();
            assert_eq!(&popped, key);
        }
        result
    }

    pub fn get_choice<T, C>(&self, reference: &ResourceRef, choice: C) -> Option<T>
    where
        T: FromStr,
        C: Borrow<ChoiceRef>,
    {
        let choice: &ChoiceRef = choice.borrow();
        let choice_owned: Choice = choice.to_owned();
        trace!("Character::get_choice({:?}, {:?})", reference, choice);
        self.checked_recurse(
            "choice",
            &(reference.clone(), choice_owned),
            |s| &s.resolving_choices,
            || {
                trace!(
                    "Getting choice ${} from {} on character {}",
                    choice,
                    reference,
                    self
                );
                let data = self.resources.get(reference)?;
                trace!("Found resource");
                let raw = match data.choice_values.get(choice) {
                    None => {
                        trace!("Failed to find choice value");
                        return None;
                    }
                    Some(raw_string) => raw_string.as_str(),
                };
                trace!("Found raw choice value {:?}", raw);
                match raw.parse::<T>() {
                    Ok(v) => Some(v),
                    Err(_) => {
                        warn!(
                            "Failed to parse choice ${} on {} (value: {:?}) as a {}",
                            choice,
                            reference,
                            raw,
                            std::any::type_name::<T>(),
                        );
                        None
                    }
                }
            },
        )
    }

    pub fn get_resource(&self, reference: ResourceRef) -> Option<Arc<Resource>> {
        self.checked_recurse(
            "resource",
            &reference,
            |s| &s.resolving_resources,
            || {
                self.resources
                    .get(&reference)
                    .map(|data| data.resource.clone())
            },
        )
    }

    pub fn get_modifier(&self, name: &str, target: Option<&ResourceRef>) -> Modifier {
        trace!("Character::get_modifier({:?}, {:?})", name, target);
        self.checked_recurse(
            "modifier",
            &String::from(name),
            |s| &s.resolving_modifiers,
            || {
                trace!(
                    "Getting modifier {:?} from {:?} on character {}",
                    name,
                    target,
                    self
                );
                let mut m = Modifier::new();
                for (rref, rd) in self.resources.iter() {
                    let mut ctx = Context::new(self, rref);
                    if let Some(target) = target {
                        ctx = ctx.with_target(target);
                    }
                    trace!("Checking resource {}", rref);
                    let resource_mod = rd.resource.get_modifier(name, ctx);
                    trace!("Got modifier value {}", resource_mod);
                    m += resource_mod;
                }
                trace!(
                    "Got final value for {} with target {:?} on character {}: {}",
                    name,
                    target,
                    self,
                    m
                );
                m
            },
        )
    }

    pub fn get_proficiency(&self, name: &str) -> Modifier {
        self.checked_recurse(
            "proficiency",
            &String::from(name),
            |s| &s.resolving_proficiencies,
            || Modifier::new(),
        )
    }
}
