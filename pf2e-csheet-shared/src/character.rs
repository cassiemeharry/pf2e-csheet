use serde::{Deserialize, Serialize};
use serde_json::Value;
use smartstring::alias::String;
use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    fmt,
};
use uuid::Uuid;

use crate::{
    bonuses::{Bonus, Modifier},
    calc::CalcContext,
    choices::{Choice, ChoiceMeta, ChoiceRef},
    common::{Class, ResourceRef, ResourceType, TypedRef},
    stats::Level,
    storage::ResourceStorage,
};

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Character {
    #[serde(default)]
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub player_name: String,
    #[serde(default)]
    pub resources: HashSet<ResourceRef>,
    #[serde(default)]
    pub choice_values: HashMap<ResourceRef, HashMap<Choice, Value>>,
    #[serde(default)]
    pub core_choices: HashMap<Choice, Value>,
}

impl Character {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            player_name: String::new(),
            resources: HashSet::new(),
            choice_values: HashMap::new(),
            core_choices: HashMap::new(),
        }
    }

    pub fn get_resouces_by_type(&self, rtype: ResourceType) -> Vec<&ResourceRef> {
        self.resources
            .iter()
            .filter(|rref| rref.resource_type == Some(rtype))
            .collect()
    }

    pub fn normalize_resources(&mut self, resources: &dyn ResourceStorage) {
        debug!("Normalizing character {}", self);
        let mut changes = true;
        while changes {
            let mut new_resources = HashSet::new();
            for rref in self.resources.iter() {
                let r = match resources.lookup_immediate(rref) {
                    Some(r) => r,
                    None => {
                        debug!("Not normalizing resource {}, not in storage", rref);
                        continue;
                    }
                };
                debug!("Getting granted resources from {}", rref);
                let ctx = CalcContext {
                    character: &self,
                    rref,
                    target: None,
                    resources,
                };
                for new_rref in r.granted_resources(ctx) {
                    debug!("Resource {} grants {}", rref, new_rref);
                    new_resources.insert(new_rref);
                }
            }
            let mut new_counter = 0;
            for rref in new_resources {
                if self.resources.contains(&rref) {
                    continue;
                }
                debug!("Normalization adds resource {} to {}", rref, self);
                self.resources.insert(rref);
                new_counter += 1;
            }
            changes = new_counter > 0;
        }
        debug!("Normalization finished");
    }

    pub fn get_modifier(
        &self,
        name: &str,
        target: Option<&ResourceRef>,
        resources: &dyn ResourceStorage,
    ) -> Modifier {
        match target {
            Some(t) => trace!(
                "Asking character {} for modifier {} targeting {}",
                self.name,
                name,
                t
            ),
            None => trace!("Asking character {} for modifier {}", self.name, name),
        }
        let mut m = match name {
            "STR" | "DEX" | "CON" | "INT" | "WIS" | "CHA" => Bonus::untyped(10).into(),
            "STR bonus" | "DEX bonus" | "CON bonus" | "INT bonus" | "WIS bonus" | "CHA bonus" => {
                let base = self.get_modifier(&name[..3], target, resources);
                let total = base.total();
                let bonus = Bonus::untyped((total - 10) / 2);
                bonus.into()
            }
            _ => Modifier::new(),
        };
        for rref in self.resources.iter() {
            let mut ctx = CalcContext::new(self, rref, resources);
            if let Some(target) = target {
                ctx = ctx.with_target(target);
            }
            let resource = match resources.lookup_immediate(rref) {
                Some(r) => r,
                None => {
                    debug!(
                        "Failed to find resource {} when looking up a modifier",
                        rref
                    );
                    continue;
                }
            };
            trace!("Checking resource {}", rref);
            let resource_mod = resource.get_modifier(name, ctx);
            trace!("Got modifier value {}", resource_mod);
            m += resource_mod;
        }
        m
    }

    pub fn set_character_choice<T>(
        &mut self,
        choice: Choice,
        value: &T,
    ) -> Result<(), serde_json::Error>
    where
        T: serde::Serialize,
    {
        let value = serde_json::to_value(value)?;
        self.core_choices.insert(choice, value);
        Ok(())
    }

    pub fn get_character_choice<T, C>(&self, choice: C) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
        C: Borrow<ChoiceRef>,
    {
        let choice = choice.borrow();
        trace!("Getting character choice ${} for {}", choice, self);
        let raw = self.core_choices.get(choice)?;
        trace!("Found raw choice value {:?}", raw);
        match serde_json::from_value(raw.clone()) {
            Ok(v) => Some(v),
            Err(_) => {
                warn!(
                    "Failed to parse choice ${} on {} (value: {:?}) as a {}",
                    choice,
                    self,
                    raw,
                    std::any::type_name::<T>(),
                );
                None
            }
        }
    }

    pub fn remove_character_choice(&mut self, choice: impl Borrow<ChoiceRef>) {
        self.core_choices.remove(choice.borrow());
    }

    pub fn set_choice<T>(
        &mut self,
        rref: &ResourceRef,
        choice: Choice,
        value: &T,
    ) -> Result<(), serde_json::Error>
    where
        T: serde::Serialize,
    {
        let value = serde_json::to_value(value)?;
        let resource_map = match self.choice_values.get_mut(rref) {
            Some(map) => map,
            None => {
                self.choice_values.insert(rref.clone(), HashMap::new());
                self.choice_values.get_mut(rref).unwrap()
            }
        };
        resource_map.insert(choice, value);
        Ok(())
    }

    pub fn get_choice<T, C>(&self, rref: &ResourceRef, choice: C) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
        C: Borrow<ChoiceRef>,
    {
        let choice = choice.borrow();
        trace!(
            "Getting choice ${} from {} on character {}",
            choice,
            rref,
            self
        );
        let choice_map: &HashMap<Choice, Value> = self.choice_values.get(rref)?;
        trace!("Found resource");
        let raw: Value = match choice_map.get(choice) {
            None => {
                trace!("Failed to find choice value");
                return None;
            }
            Some(val_ref) => val_ref.clone(),
        };
        trace!("Found raw choice value {:?}", raw);
        match serde_json::from_value(raw.clone()) {
            Ok(v) => Some(v),
            Err(_) => {
                warn!(
                    "Failed to parse choice ${} on {} (value: {:?}) as a {}",
                    choice,
                    rref,
                    raw,
                    std::any::type_name::<T>(),
                );
                None
            }
        }
    }

    pub fn remove_choice(&mut self, rref: &ResourceRef, choice: impl Borrow<ChoiceRef>) {
        let resource_map = match self.choice_values.get_mut(rref) {
            Some(map) => map,
            None => return,
        };
        resource_map.remove(choice.borrow());
    }

    pub fn all_choices<'a>(
        &'a self,
        resources: &'a dyn ResourceStorage,
    ) -> impl Iterator<Item = (&'a ResourceRef, Choice, ChoiceMeta)> + 'a {
        self.resources
            .iter()
            .filter_map(move |rref: &'a ResourceRef| {
                resources.lookup_immediate(rref).map(move |r| (rref, r))
            })
            .flat_map(
                |(rref, r): (&'a ResourceRef, std::sync::Arc<crate::Resource>)| {
                    r.all_choices()
                        .map(move |(c, cm)| (rref, c.clone(), cm.clone()))
                        .collect::<Vec<_>>()
                        .into_iter()
                },
            )
    }
}

impl fmt::Display for Character {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.name, f)
    }
}

impl Character {
    pub fn get_class_and_level(&self) -> Option<(TypedRef<Class>, Level)> {
        let class_rrefs = self.get_resouces_by_type(ResourceType::Class);
        debug_assert!(class_rrefs.len() < 2);
        let class_rref_dyn: &ResourceRef = class_rrefs.get(0)?;
        let class_rref: TypedRef<Class> = class_rref_dyn.clone().as_typed().ok()?;
        let level = self
            .get_choice::<Level, _>(class_rref_dyn, "Level")
            .unwrap_or(1.into());
        Some((class_rref, level))
    }
}
