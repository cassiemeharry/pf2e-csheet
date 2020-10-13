use async_trait::async_trait;
use smartstring::alias::String;
use std::{collections::HashSet, sync::Arc};

use crate::common::{Resource, ResourceRef, ResourceType};

#[async_trait(?Send)]
pub trait ResourceStorage: 'static {
    async fn lookup_async(&self, rrefs: &[&ResourceRef]) -> Vec<Option<Arc<Resource>>>;

    fn lookup_immediate(&self, rref: &ResourceRef) -> Option<Arc<Resource>>;

    async fn all_by_type(&self, rtype: ResourceType) -> HashSet<ResourceRef>;

    async fn register(&mut self, resource: Resource) -> Result<(), String>;
}
