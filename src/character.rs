use serde::Deserialize;
use smartstring::alias::String;
use std::{
    any::Any,
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
};

use crate::{
    bonuses::Modifier,
    resources::{Resource, ResourceRef},
};

#[derive(Debug, Deserialize)]
struct ResourceData {
    resource: Arc<Resource>,
    choice_values: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct Character {
    name: String,
    resources: HashMap<ResourceRef, Arc<ResourceData>>,
    resolving_choices: Mutex<Vec<(ResourceRef, String)>>,
    resolving_resources: Mutex<Vec<ResourceRef>>,
    resolving_modifiers: Mutex<Vec<String>>,
}

impl Character {
    fn checked_recurse<T, U, F, G>(&self, label: &'static str, key: &T, get_lock: G, f: F) -> U
    where
        T: Clone + std::fmt::Debug + std::cmp::Eq,
        F: FnOnce() -> U,
        G: Fn(&Character) -> &Mutex<Vec<T>>,
    {
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

    pub fn get_choice<T: FromStr>(&self, reference: ResourceRef, choice: &str) -> Option<T> {
        self.checked_recurse(
            "choice",
            &(reference.clone(), String::from(choice)),
            |s| &s.resolving_choices,
            || {
                let data = self.resources.get(&reference)?;
                let raw = data.choice_values.get(choice)?.as_str();
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

    pub fn get_modifier(&self, name: &str) -> Modifier {
        self.checked_recurse(
            "modifier",
            &String::from(name),
            |s| &s.resolving_modifiers,
            || Modifier::new(),
        )
    }
}
