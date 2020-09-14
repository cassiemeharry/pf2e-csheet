use anyhow::{anyhow, Result};
use serde::{de, Deserialize};
use std::{
    any::type_name,
    fmt,
    sync::{Arc, Mutex},
};

use super::traits::Resource;

#[derive(Clone)]
pub struct Ref<R: Resource> {
    inner: Arc<Mutex<RefInner<R>>>,
}

impl<R: Resource> Ref<R> {
    pub fn get(&self) -> Arc<R> {
        let mut inner = self.inner.lock().unwrap();
        inner.resolve().unwrap()
    }

    pub fn try_get(&self) -> Result<Arc<R>> {
        let mut inner = self.inner.lock().unwrap();
        inner.resolve()
    }
}

impl<'de, R: Resource> Deserialize<'de> for Ref<R> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let inner: RefInner<R> = Deserialize::deserialize(deserializer)?;
        Ok(Ref {
            inner: Arc::new(Mutex::new(inner)),
        })
    }
}

impl<R> fmt::Display for Ref<R>
where
    R: Resource + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut inner = self.inner.lock().unwrap();

        // Intentionally ignore failure to resovle, as that's handled in the
        // match statement.
        let _ = inner.resolve();
        match &*inner {
            RefInner::Resolved(r) => <R as fmt::Display>::fmt(r, f),
            RefInner::Unresolved(i) => write!(f, "{:?}", i),
        }
    }
}

impl<R: Resource + fmt::Debug> fmt::Debug for Ref<R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let inner = self.inner.lock().unwrap();
        let inner_ref: &RefInner<R> = &inner;
        inner_ref.fmt(f)
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
#[serde(bound = "R: Resource")]
enum RefInner<R: Resource> {
    Resolved(Arc<R>),
    Unresolved(<R as Resource>::Index),
}

impl<R: Resource> RefInner<R> {
    fn resolve(&mut self) -> Result<Arc<R>> {
        match self {
            RefInner::Resolved(r) => Ok(r.clone()),
            RefInner::Unresolved(index) => match crate::resources::get::<R>(index) {
                Some(r) => {
                    *self = RefInner::Resolved(r.clone());
                    Ok(r)
                }
                None => Err(anyhow!(
                    "Failed to resolve {:?} into {}",
                    index,
                    type_name::<R>()
                )),
            },
        }
    }
}
