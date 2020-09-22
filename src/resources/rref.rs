use anyhow::{ensure, Error, Result};
use serde::{de, Deserialize};
use smartstring::alias::String;
use std::{
    cmp::{Ord, Ordering, PartialEq, PartialOrd},
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::Deref,
    str::FromStr,
    sync::{Arc, RwLock},
};

use super::{HasResourceType, Resource, ResourceType};
use crate::try_from_str;

#[derive(Clone, Debug, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
enum ResourceRefName {
    Unresolved(String),
    Resolved(Arc<Resource>),
}

try_from_str!(ResourceRefName);

impl FromStr for ResourceRefName {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(Self::Unresolved(String::from(s)))
    }
}

impl PartialEq for ResourceRefName {
    fn eq(&self, other: &Self) -> bool {
        let name_1: &str = self.name();
        let name_2: &str = other.name();
        // TODO: should this be case-insensitive?
        name_1 == name_2
    }
}

impl Eq for ResourceRefName {}

impl Hash for ResourceRefName {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.name().hash(state)
    }
}

impl PartialOrd for ResourceRefName {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ResourceRefName {
    fn cmp(&self, other: &Self) -> Ordering {
        let name_1: &str = self.name();
        let name_2: &str = other.name();
        // TODO: should this be case-insensitive?
        name_1.cmp(name_2)
    }
}

impl ResourceRefName {
    fn name(&self) -> &str {
        match self {
            Self::Unresolved(s) => s.as_str(),
            Self::Resolved(r) => r.common().name.as_str(),
        }
    }

    fn resolve(&mut self, expected: Option<ResourceType>) -> Option<Arc<Resource>> {
        let verify_type = |r: Arc<Resource>| -> Option<Arc<Resource>> {
            if let Some(rt) = expected {
                if r.get_type() != rt {
                    return None;
                }
            }
            Some(r)
        };

        match self {
            Self::Unresolved(s) => {
                let r = Resource::lookup(s, expected)?;
                let r = verify_type(r)?;
                *self = Self::Resolved(r.clone());
                Some(r)
            }
            Self::Resolved(r) => verify_type(r.clone()),
        }
    }
}

impl fmt::Display for ResourceRefName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Clone, Debug)]
struct RRefInner {
    resource: Arc<RwLock<ResourceRefName>>,
    modifier: Option<String>,
}

impl RRefInner {
    #[inline]
    fn new_from_name(name: String) -> Self {
        Self {
            resource: Arc::new(RwLock::new(ResourceRefName::Unresolved(name))),
            modifier: None,
        }
    }

    #[inline]
    fn new_from_name_mod(name: String, modifier: String) -> Self {
        Self {
            resource: Arc::new(RwLock::new(ResourceRefName::Unresolved(name))),
            modifier: Some(modifier),
        }
    }

    #[inline]
    fn new_from_resolved(resolved: Arc<Resource>) -> Self {
        Self {
            resource: Arc::new(RwLock::new(ResourceRefName::Resolved(resolved))),
            modifier: None,
        }
    }

    #[inline]
    fn new_from_resolved_mod(resolved: Arc<Resource>, modifier: String) -> Self {
        Self {
            resource: Arc::new(RwLock::new(ResourceRefName::Resolved(resolved))),
            modifier: Some(modifier),
        }
    }

    #[inline]
    fn resource(&self, expected_type: Option<ResourceType>) -> Option<Arc<Resource>> {
        let mut lock = self.resource.write().unwrap();
        lock.resolve(expected_type)
    }

    fn name<'a>(&'a self) -> impl Deref<Target = str> + 'a {
        let guard = self.resource.read().unwrap();
        owning_ref::RwLockReadGuardRef::new(guard).map(|rref_name| rref_name.name())
    }

    fn rref_name<'a>(&'a self) -> impl Deref<Target = ResourceRefName> + 'a {
        let guard = self.resource.read().unwrap();
        owning_ref::RwLockReadGuardRef::new(guard)
    }

    #[inline]
    fn modifier(&self) -> Option<&str> {
        self.modifier.as_ref().map(|s| s.as_str())
    }

    fn unresolve(&self) {
        let mut guard = self.resource.write().unwrap();
        if let ResourceRefName::Resolved(r) = &*guard {
            let name = r.name().into();
            *guard = ResourceRefName::Unresolved(name);
        }
    }
}

impl FromStr for RRefInner {
    type Err = Error;

    fn from_str(s: &str) -> Result<RRefInner> {
        let mut paren_index = None;
        for (i, c) in s.char_indices() {
            if paren_index.is_none() && c == '(' {
                paren_index = Some(i);
                continue;
            }
        }
        let (name_str, mod_str_opt) = match paren_index {
            None => (s.trim(), None),
            Some(i) => {
                let (name, mut modifier) = s.split_at(i);
                loop {
                    let trimmed = modifier
                        .trim()
                        .trim_start_matches('(')
                        .trim_end_matches(')');
                    if trimmed == modifier {
                        break;
                    }
                    modifier = trimmed;
                }
                (name.trim(), Some(modifier))
            }
        };
        ensure!(!name_str.is_empty(), "Reference name cannot be empty");
        match mod_str_opt {
            None => Ok(RRefInner::new_from_name(String::from(name_str))),
            Some(mod_str) => {
                ensure!(
                    !mod_str.is_empty(),
                    "Reference modifier cannot be empty if parentheses are present"
                );
                Ok(RRefInner::new_from_name_mod(
                    name_str.into(),
                    mod_str.into(),
                ))
            }
        }
    }
}

#[test]
fn test_parse_rref_mod() {
    let rref = RRefInner::from_str("incredible movement (+30 ft)").unwrap();
    let actual_name = &*rref.name();
    assert_eq!(actual_name, "incredible movement");
    assert_eq!(rref.modifier(), Some("+30 ft"));
}

impl fmt::Display for RRefInner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let guard = self.resource.read().unwrap();
        match self.modifier.as_ref() {
            None => write!(f, "{}", &*guard),
            Some(m) => write!(f, "{} ({})", &*guard, m),
        }
    }
}

impl PartialEq for RRefInner {
    fn eq(&self, other: &Self) -> bool {
        let self_guard = self.resource.read().unwrap();
        let other_guard = other.resource.read().unwrap();
        (&*self_guard) == (&*other_guard) && &self.modifier == &other.modifier
    }
}

impl Eq for RRefInner {}

impl Hash for RRefInner {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.resource.read().unwrap().hash(state);
        self.modifier.hash(state);
    }
}

impl PartialOrd for RRefInner {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RRefInner {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_rref = self.resource.read().unwrap();
        let other_rref = other.resource.read().unwrap();
        self_rref
            .cmp(&other_rref)
            .then_with(|| self.modifier.cmp(&other.modifier))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub struct ResourceRef {
    inner: RRefInner,
    expected_type: Option<ResourceType>,
}

impl ResourceRef {
    pub fn resource(&self) -> Option<Arc<Resource>> {
        self.inner.resource(self.expected_type)
    }

    pub fn modifier(&self) -> Option<&str> {
        self.inner.modifier()
    }

    pub fn expected_type(&self) -> Option<ResourceType> {
        self.expected_type
    }

    pub fn new_from_name(name: String) -> Self {
        Self {
            inner: RRefInner::new_from_name(name),
            expected_type: None,
        }
    }

    pub fn new_from_name_mod(name: String, modifier: String) -> Self {
        Self {
            inner: RRefInner::new_from_name_mod(name, modifier),
            expected_type: None,
        }
    }

    pub fn new_from_resolved(resolved: Arc<Resource>) -> Self {
        let expected_type = Some(resolved.get_type());
        Self {
            inner: RRefInner::new_from_resolved(resolved),
            expected_type,
        }
    }

    pub fn new_from_resolved_mod(resolved: Arc<Resource>, modifier: String) -> Self {
        let expected_type = Some(resolved.get_type());
        Self {
            inner: RRefInner::new_from_resolved_mod(resolved, modifier),
            expected_type,
        }
    }

    pub fn expect_type(mut self, expected_type: ResourceType) -> Self {
        if let ResourceRefName::Resolved(ref r) = &*self.inner.rref_name() {
            if r.get_type() != expected_type {
                warn!("ResourceRef::expect_type changed the expected type of a resolved ref {:?}, unresolving it", self);
                self.inner.unresolve();
            }
        }
        let previous = self.expected_type.replace(expected_type);
        if let Some(p) = previous {
            if p != expected_type {
                warn!("ResourceRef::expect_type was called multiple times, last type was {:?}, new type was {:?}", p, expected_type);
            }
        }
        self
    }
}

impl fmt::Display for ResourceRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

try_from_str!(ResourceRef);

impl FromStr for ResourceRef {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let inner = RRefInner::from_str(s)?;
        Ok(Self {
            inner,
            expected_type: None::<ResourceType>,
        })
    }
}

#[derive(Clone, Debug)]
pub struct TypedRef<R: HasResourceType> {
    inner: RRefInner,
    expected_type: PhantomData<R>,
}

impl<R: HasResourceType> TypedRef<R> {
    pub fn resource(&self) -> Option<Arc<Resource>> {
        self.inner.resource(Some(R::RESOURCE_TYPE))
    }

    pub fn modifier(&self) -> Option<&str> {
        self.inner.modifier()
    }

    pub fn expected_type() -> ResourceType {
        R::RESOURCE_TYPE
    }

    pub fn new_from_name(name: String) -> Self {
        Self {
            inner: RRefInner::new_from_name(name),
            expected_type: PhantomData,
        }
    }

    pub fn new_from_name_mod(name: String, modifier: String) -> Self {
        Self {
            inner: RRefInner::new_from_name_mod(name, modifier),
            expected_type: PhantomData,
        }
    }

    pub fn to_runtime(self) -> ResourceRef {
        ResourceRef {
            inner: self.inner,
            expected_type: Some(R::RESOURCE_TYPE),
        }
    }
}

impl<'de, R> de::Deserialize<'de> for TypedRef<R>
where
    R: HasResourceType + de::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct TypedRefVisitor<R>(PhantomData<fn() -> R>);

        impl<'de, R> de::Visitor<'de> for TypedRefVisitor<R>
        where
            R: HasResourceType + de::Deserialize<'de>,
        {
            type Value = TypedRef<R>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    f,
                    "A string reference name or a map describing the {}",
                    R::RESOURCE_TYPE
                )
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                TypedRef::<R>::from_str(s).map_err(E::custom)
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mad = de::value::MapAccessDeserializer::new(map);
                de::Deserialize::deserialize(mad)
            }
        }

        deserializer.deserialize_any(TypedRefVisitor(PhantomData))
    }
}

impl<R: HasResourceType> fmt::Display for TypedRef<R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

try_from_str!(TypedRef<R: HasResourceType>);

impl<R: HasResourceType> FromStr for TypedRef<R> {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let inner = RRefInner::from_str(s)?;
        Ok(Self {
            inner,
            expected_type: PhantomData,
        })
    }
}
