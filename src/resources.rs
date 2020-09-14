use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::{
    any::type_name,
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    marker::PhantomData,
    sync::{Arc, Mutex, RwLock},
};

pub mod traits;

use traits::Resource;

struct MapKey<R: Resource>(PhantomData<<R as Resource>::Index>);

type ValueMap<R> = Arc<RwLock<HashMap<<R as Resource>::Index, Arc<R>>>>;

impl<R: Resource> typemap::Key for MapKey<R> {
    type Value = ValueMap<R>;
}

lazy_static::lazy_static! {
    static ref GLOBAL_CACHE: Mutex<typemap::ShareMap> = {
        Mutex::new(typemap::ShareMap::custom())
    };
}

pub fn get<R: Resource>(index: &<R as Resource>::Index) -> Option<Arc<R>> {
    let value_map_lock: ValueMap<R> = {
        let mut global_cache = GLOBAL_CACHE.lock().unwrap();
        global_cache
            .entry::<MapKey<R>>()
            .or_insert_with(|| Arc::new(RwLock::new(HashMap::new())))
            .clone()
    };

    let value_map = value_map_lock.read().unwrap();

    value_map.get(index).cloned()
    // if let Some(r) = value_map.get(index) {
    //     return Some(r);
    // }

    // // No exact match, so iterate over all and look for a single other match.
    // let matches: SmallVec<[_; 1]> = value_map.iter().filter(|(_, v)| v.matches(index)).collect();
    // debug!("Doing fallback search, found {:?} matches", matches.len());
    // match matches.len() {
    //     0 => None,
    //     1 => matches[0].1.indexed(index).map(Arc::new),
    //     n => {
    //         warn!("Found {} matches for index value {:?}:", n, index);
    //         for (k, _) in matches {
    //             warn!("\t{:?}", k);
    //         }
    //         None
    //     }
    // }
}

pub fn insert<R: Resource>(resource: &R) -> Result<()> {
    let index = resource.get_index_value();
    let value_map_lock: ValueMap<R> = {
        let mut global_cache = GLOBAL_CACHE.lock().unwrap();
        global_cache
            .entry::<MapKey<R>>()
            .or_insert_with(|| Arc::new(RwLock::new(HashMap::new())))
            .clone()
    };
    let mut value_map = value_map_lock.write().unwrap();
    match value_map.entry(index.clone()) {
        Entry::Occupied(_entry) => {
            // let existing: &R = &*entry.get();
            Err(anyhow!(
                "Found duplicate {} value for index {:?}",
                type_name::<R>(),
                index
            ))
        }
        Entry::Vacant(entry) => {
            entry.insert(Arc::new(resource.clone()));
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopLevelResource {
    Ancestry(Ancestry),
    Background(Background),
    Class(Class),
    Feat(Feat),
    Heritage(Heritage),
    Item(Item),
}

impl TopLevelResource {
    pub fn register_in_cache(&self) -> Result<()> {
        match self {
            Self::Ancestry(a) => insert(a),
            Self::Background(b) => insert(b),
            Self::Class(c) => insert(c),
            Self::Feat(f) => insert(f),
            Self::Heritage(h) => insert(h),
            Self::Item(i) => insert(i),
        }
    }
}

pub mod ancestry;
pub mod background;
pub mod character;
pub mod class;
pub mod feat;
pub mod heritage;
pub mod item;

pub use ancestry::Ancestry;
pub use background::Background;
pub use character::Character;
pub use class::Class;
pub use feat::*;
pub use heritage::Heritage;
pub use item::*;

pub mod refs;
