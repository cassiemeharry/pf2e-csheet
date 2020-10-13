use serde::Deserialize;
use smartstring::alias::String;
use std::{fmt, marker::PhantomData, str::FromStr};
use thiserror::Error;

use super::{HasResourceType, ResourceType};

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Deserialize)]
#[cfg_attr(test, derive(Arbitrary))]
#[serde(try_from = "smartstring::alias::String")]
pub struct ResourceRef {
    #[cfg_attr(
        test,
        proptest(strategy = "\"[a-zA-Z]([a-zA-Z_ /-]+[a-zA-Z])?\".prop_map_into()")
    )]
    pub name: String,
    #[cfg_attr(
        test,
        proptest(
            strategy = "proptest::option::of(\"[a-zA-Z0-9+-]([a-zA-Z0-9 _+-]+[a-zA-Z0-9])?\".prop_map_into())"
        )
    )]
    pub modifier: Option<String>,
    pub resource_type: Option<ResourceType>,
}

impl ResourceRef {
    pub fn new(name: impl Into<String>, modifier: Option<impl Into<String>>) -> Self {
        Self {
            name: name.into(),
            modifier: modifier.map(|m| m.into()),
            resource_type: None,
        }
    }

    pub fn with_type(mut self, resource_type: Option<ResourceType>) -> Self {
        self.resource_type = resource_type;
        self
    }

    pub fn as_typed<R: HasResourceType>(self) -> Result<TypedRef<R>, Self> {
        match self.resource_type {
            None => Ok(TypedRef {
                name: self.name,
                modifier: self.modifier,
                resource_type: PhantomData,
            }),
            Some(r) if r == R::RESOURCE_TYPE => Ok(TypedRef {
                name: self.name,
                modifier: self.modifier,
                resource_type: PhantomData,
            }),
            Some(_) => Err(self),
        }
    }
}

impl fmt::Display for ResourceRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self.modifier.as_ref(), self.resource_type.as_ref()) {
            (Some(m), Some(rt)) => write!(f, "{} ({}) [{}]", self.name, m, rt),
            (Some(m), None) => write!(f, "{} ({})", self.name, m),
            (None, Some(rt)) => write!(f, "{} [{}]", self.name, rt),
            (None, None) => fmt::Display::fmt(&self.name, f),
        }
    }
}

try_from_str!(ResourceRef);

#[derive(Debug, Error, Deserialize)]
#[error("ResourceRef parsing failed")]
pub struct ResourceRefFromStrError;

impl FromStr for ResourceRef {
    type Err = ResourceRefFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        trace!("Parsing resource ref from str {:?}", s);
        match crate::parsers::resource_ref(s) {
            Ok(rref) => {
                trace!("Parsed resource ref: {:?}", rref);
                Ok(rref)
            }
            Err(e) => {
                error!("Failed to parse a resource ref from {:?}:\n{}", s, e);
                Err(ResourceRefFromStrError)
            }
        }
    }
}

serialize_display!(ResourceRef);

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Deserialize)]
#[serde(try_from = "smartstring::alias::String")]
pub struct TypedRef<R: HasResourceType> {
    pub name: String,
    pub modifier: Option<String>,
    pub resource_type: PhantomData<fn(R) -> ()>,
}

#[cfg(test)]
impl<R: HasResourceType + fmt::Debug> Arbitrary for TypedRef<R> {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        (
            "[a-zA-Z]([a-zA-Z_ /-]+[a-zA-Z])?".prop_map_into(),
            proptest::option::of("[a-zA-Z0-9+-]([a-zA-Z0-9 _+-]+[a-zA-Z0-9])?".prop_map_into()),
        )
            .prop_map(|(name, modifier)| TypedRef {
                name,
                modifier,
                resource_type: PhantomData,
            })
            .boxed()
    }
}

impl<R: HasResourceType> TypedRef<R> {
    pub fn new(name: impl Into<String>, modifier: Option<impl Into<String>>) -> Self {
        Self {
            name: name.into(),
            modifier: modifier.map(|m| m.into()),
            resource_type: PhantomData,
        }
    }

    pub fn as_runtime(self) -> ResourceRef {
        ResourceRef {
            name: self.name,
            modifier: self.modifier,
            resource_type: Some(R::RESOURCE_TYPE),
        }
    }
}

impl<R: HasResourceType> fmt::Display for TypedRef<R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.modifier.as_ref() {
            Some(m) => write!(f, "{} ({}) [{}]", self.name, m, R::RESOURCE_TYPE),
            None => write!(f, "{} [{}]", self.name, R::RESOURCE_TYPE),
        }
    }
}

serialize_display!(TypedRef<R: HasResourceType>);

try_from_str!(TypedRef<R: HasResourceType>);

impl<R: HasResourceType> FromStr for TypedRef<R> {
    type Err = ResourceRefFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let untyped = ResourceRef::from_str(s)?;
        Ok(Self {
            name: untyped.name,
            modifier: untyped.modifier,
            resource_type: PhantomData,
        })
    }
}
