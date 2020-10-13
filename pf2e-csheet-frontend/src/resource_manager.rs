use async_trait::async_trait;
use pf2e_csheet_shared::{storage::ResourceStorage, Resource, ResourceRef, ResourceType};
use smartstring::alias::String;
use std::{
    cell::{Cell, RefCell},
    collections::{hash_map::Entry, HashMap, HashSet},
    fmt,
    future::Future,
    rc::Rc,
    sync::Arc,
};
use thiserror::Error;
use url::Url;
use wasm_bindgen::{prelude::*, JsCast as _};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Request, RequestInit, RequestMode, Response};
use yew::prelude::*;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum ResourceRequest {
    Single {
        name: String,
        resource_type: Option<ResourceType>,
    },
    AllByType(ResourceType),
    AllByTrait(String),
}

impl ResourceRequest {
    fn build_url(&self, url_base: &Url) -> Url {
        match self {
            Self::Single {
                name,
                resource_type,
            } => {
                let mut url: Url = format!("{}resources", url_base).parse().unwrap();
                let mut query_pairs = url.query_pairs_mut();
                let rtype_value: serde_json::Value;
                let rtype_str = match resource_type.as_ref() {
                    Some(rtype) => {
                        rtype_value = serde_json::to_value(rtype).unwrap();
                        rtype_value.as_str().unwrap()
                    }
                    None => "",
                };
                query_pairs.append_pair(&name, rtype_str);
                drop(query_pairs);
                url
            }
            Self::AllByTrait(t) => {
                let mut url: Url = format!("{}resources/by-trait", url_base).parse().unwrap();
                let mut query_pairs = url.query_pairs_mut();
                query_pairs.append_pair("t", t);
                drop(query_pairs);
                url
            }
            Self::AllByType(rtype) => {
                let mut url: Url = format!("{}resources/by-type", url_base).parse().unwrap();
                let mut query_pairs = url.query_pairs_mut();
                let rtype_value = serde_json::to_value(rtype).unwrap();
                let rtype_str = rtype_value.as_str().unwrap();
                query_pairs.append_pair("t", rtype_str);
                drop(query_pairs);
                url
            }
        }
    }

    fn load(
        &self,
        url_base: &Url,
        inner: Rc<RefCell<ResourceManagerInner>>,
    ) -> impl Future<Output = ()> + 'static {
        let url = self.build_url(url_base);
        let req = self.clone();
        async move {
            match &req {
                Self::Single { .. } => {
                    let from_server: HashMap<String, Option<Arc<Resource>>> =
                        match ResourceManager::remote_request("GET", url).await {
                            Ok(map) => map,
                            Err(_e) => {
                                error!("Failed to load resource from server");
                                return;
                            }
                        };
                    let mut guard = inner.borrow_mut();
                    for (_, res_opt) in from_server {
                        if let Some(res) = res_opt {
                            guard.add_resource(res);
                        }
                    }
                }
                Self::AllByTrait(t) => {
                    let from_server: Vec<Arc<Resource>> =
                        match ResourceManager::remote_request("GET", url).await {
                            Ok(rs) => rs,
                            Err(_e) => {
                                error!("Failed to load resources with trait {:?} from server", t);
                                return;
                            }
                        };
                    let mut guard = inner.borrow_mut();
                    for res in from_server {
                        guard.add_resource(res);
                    }
                }
                Self::AllByType(rtype) => {
                    let from_server: HashSet<ResourceRef> =
                        match ResourceManager::remote_request("GET", url).await {
                            Ok(set) => set,
                            Err(_e) => {
                                error!("Failed to load resources of type {} from server", rtype);
                                return;
                            }
                        };
                    let mut guard = inner.borrow_mut();
                    for rref in from_server {
                        guard.add_rref(rref);
                    }
                }
            }
            let mut guard = inner.borrow_mut();
            guard.mark_completed(req);
        }
    }
}

#[derive(Clone)]
pub struct ResourceManager {
    url_base: Url,
    callback: Rc<dyn Fn(usize)>,
    inner: Rc<RefCell<ResourceManagerInner>>,
    in_flight: Rc<Cell<usize>>,
}

impl fmt::Debug for ResourceManager {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ResourceManager")
            .field("url_base", &self.url_base)
            .field("callback", &format_args!("<callback>"))
            .field("inner", &format_args!("<inner lock>"))
            .field("in_flight", &self.in_flight.get())
            .finish()
    }
}

impl ResourceManager {
    pub fn new(url_base: Url, update_callback: Callback<usize>) -> Self {
        debug!("Determine url_base to be {:?}", url_base);

        let callback = match update_callback {
            Callback::Callback(rc_fn) => rc_fn,
            Callback::CallbackOnce(_) => panic!("Got a CallbackOnce in ResourceManager!"),
        };

        let inner = Rc::new(RefCell::new(ResourceManagerInner::new()));
        let in_flight = Rc::new(Cell::new(0));

        Self {
            url_base,
            callback,
            inner,
            in_flight,
        }
    }

    pub fn is_loading(&self) -> bool {
        self.in_flight.get() > 0
    }
}

/// Resource loading methods
impl ResourceManager {
    async fn remote_request<T>(method: &str, url: Url) -> Result<T, JsValue>
    where
        T: serde::de::DeserializeOwned + Send,
    {
        let mut opts = RequestInit::new();
        opts.method(method);
        opts.mode(RequestMode::SameOrigin);

        let request = Request::new_with_str_and_init(url.as_ref(), &opts)?;
        request.headers().set("Accept", "application/json")?;

        let window = web_sys::window().unwrap();
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: Response = resp_value.dyn_into()?;
        let json_future = JsFuture::from(resp.json()?);
        drop(resp);
        let json = json_future.await?;
        json.into_serde().map_err(|e| {
            let msg = format!("Failed to parse server resources response: {}", e);
            JsValue::from_str(&msg)
        })
    }

    fn spawn_pending_requests(&self) {
        // Cloning is OK because the writable fields are wrapped with `Rc`.
        let this = self.clone();
        // This is run as an async task to allow multiple requests to pile up so
        // we can send these more efficiently.
        match web_sys::window() {
            Some(window) => {
                let cb = Closure::once_into_js(move || {
                    this.start_pending_requests();
                });
                let cb_fn: &js_sys::Function = cb.as_ref().unchecked_ref();
                let set_timeout_result =
                    window.set_timeout_with_callback_and_timeout_and_arguments_0(cb_fn, 100);
                match set_timeout_result {
                    Ok(_) => (),
                    Err(_) => error!("Failed to trigger pending requests"),
                }
            }
            None => spawn_local(async move {
                this.start_pending_requests();
            }),
        }
    }

    fn start_pending_requests(&self) {
        let pending = {
            let mut guard = self.inner.borrow_mut();
            guard.take_pending()
        };
        let (singles, others) = pending
            .into_iter()
            .partition(|req: &ResourceRequest| match req {
                ResourceRequest::Single { .. } => true,
                _ => false,
            });

        self.start_singles(singles);
        for req in others {
            spawn_local(self.run_notify(req.load(&self.url_base, Rc::clone(&self.inner))));
        }
    }

    fn run_notify(
        &self,
        future: impl Future<Output = ()> + 'static,
    ) -> impl Future<Output = ()> + 'static {
        let counter = Rc::clone(&self.in_flight);
        let callback = Rc::clone(&self.callback);

        counter.set(counter.get() + 1);
        async move {
            future.await;
            let new_value = counter.get() - 1;
            counter.set(new_value);
            callback(new_value);
        }
    }

    fn start_singles(&self, singles: HashSet<ResourceRequest>) {
        if singles.is_empty() {
            return;
        }
        let mut url: Url = format!("{}resources", self.url_base).parse().unwrap();
        {
            let mut query_pairs = url.query_pairs_mut();
            for req in singles.iter() {
                let (name, rtype_opt) = match req {
                    ResourceRequest::Single {
                        name,
                        resource_type,
                    } => (name, resource_type),
                    _ => unreachable!(),
                };
                let rtype_value: serde_json::Value;
                let rtype_str = match rtype_opt.as_ref() {
                    Some(rtype) => {
                        rtype_value = serde_json::to_value(rtype).unwrap();
                        rtype_value.as_str().unwrap()
                    }
                    None => "",
                };
                query_pairs.append_pair(&name, rtype_str);
            }
        }
        let singles_inner = Rc::clone(&self.inner);
        let future = async move {
            let from_server: HashMap<String, Option<Arc<Resource>>> =
                match Self::remote_request("GET", url).await {
                    Ok(map) => map,
                    Err(_e) => {
                        error!("Failed to load resources from server");
                        return;
                    }
                };
            let mut guard = singles_inner.borrow_mut();
            for (_, res_opt) in from_server {
                if let Some(res) = res_opt {
                    guard.add_resource(res);
                }
            }
        };
        spawn_local(self.run_notify(future));
    }
}

/// "Public" API
impl ResourceManager {
    pub(crate) fn all_by_type_immediate(&self, rtype: ResourceType) -> HashSet<ResourceRef> {
        let guard = self.inner.borrow();
        guard.all_by_type(rtype)
    }

    pub(crate) fn ensure_one<'a, 'b>(&'a self, rref: &'b ResourceRef) {
        let mut guard = self.inner.borrow_mut();
        let req = ResourceRequest::Single {
            name: rref.name.clone(),
            resource_type: rref.resource_type.clone(),
        };
        guard.mark_pending(req);
    }

    pub(crate) fn ensure_all<'a, 'b>(&'a self, rrefs: impl IntoIterator<Item = &'b ResourceRef>) {
        let mut guard = self.inner.borrow_mut();
        for rref in rrefs {
            let req = ResourceRequest::Single {
                name: rref.name.clone(),
                resource_type: rref.resource_type.clone(),
            };
            guard.mark_pending(req);
        }
    }

    pub(crate) fn ensure_all_by_trait(&self, t: &str) {
        let mut guard = self.inner.borrow_mut();
        guard.mark_pending(ResourceRequest::AllByTrait(t.into()));
        drop(guard);
        self.spawn_pending_requests();
    }

    pub(crate) fn ensure_all_by_type(&self, rtype: ResourceType) {
        let mut guard = self.inner.borrow_mut();
        guard.mark_pending(ResourceRequest::AllByType(rtype));
        drop(guard);
        self.spawn_pending_requests();
    }
}

#[derive(Debug, Error)]
enum LookupError {
    #[error("No resource known by that name")]
    UnknownName,
    #[error("No resource matches that name and type")]
    UnknownNameAndType,
    #[error("Resource name is ambiguous; there are {0} resources with that name")]
    AmbiguousName(usize),
    #[error("We know about this resource, but it's not loaded in full yet")]
    NotLoaded,
}

impl LookupError {
    fn should_fetch(&self) -> bool {
        match self {
            Self::UnknownName | Self::UnknownNameAndType | Self::NotLoaded => true,
            _ => false,
        }
    }
}

#[async_trait(?Send)]
impl ResourceStorage for ResourceManager {
    async fn lookup_async(&self, rrefs: &[&ResourceRef]) -> Vec<Option<Arc<Resource>>> {
        let mut to_lookup = Vec::new();
        let mut result = Vec::with_capacity(rrefs.len());
        let guard = self.inner.borrow();
        for rref in rrefs.iter().copied() {
            let res_opt = match guard.lookup_by_rref(&rref) {
                Ok(r) => Some(r),
                Err(e) if e.should_fetch() => {
                    to_lookup.push((rref.name.clone(), rref.resource_type.clone()));
                    None
                }
                Err(_e) => {
                    warn!("Failed to lookup rref {}", rref);
                    None
                }
            };
            result.push(res_opt);
        }
        if !to_lookup.is_empty() {
            let mut guard = self.inner.borrow_mut();
            for (name, resource_type) in to_lookup {
                let req = ResourceRequest::Single {
                    name,
                    resource_type,
                };
                guard.mark_pending(req);
            }
            drop(guard);
            self.spawn_pending_requests();
        }
        result
    }

    fn lookup_immediate(&self, rref: &ResourceRef) -> Option<Arc<Resource>> {
        {
            let guard = self.inner.borrow();
            match guard.lookup_by_rref(rref) {
                Ok(r) => return Some(r),
                Err(e) if e.should_fetch() => (),
                Err(_) => return None,
            }
        }
        let mut guard = self.inner.borrow_mut();
        guard.mark_pending(ResourceRequest::Single {
            name: rref.name.clone(),
            resource_type: rref.resource_type.clone(),
        });
        drop(guard);
        self.spawn_pending_requests();
        None
    }

    async fn all_by_type(&self, rtype: ResourceType) -> HashSet<ResourceRef> {
        self.ensure_all_by_type(rtype);
        let inner_guard = self.inner.borrow();
        inner_guard.all_by_type(rtype)
    }

    async fn register(&mut self, resource: Resource) -> Result<(), String> {
        let _ = resource;
        todo!()
    }
}

#[derive(Debug)]
struct ResourceManagerInner {
    loaded: HashMap<String, HashMap<ResourceType, Option<Arc<Resource>>>>,
    pending: HashSet<ResourceRequest>,
    in_flight: HashSet<ResourceRequest>,
    completed: HashSet<ResourceRequest>,
}

impl ResourceManagerInner {
    fn new() -> Self {
        Self {
            loaded: HashMap::new(),
            pending: HashSet::new(),
            in_flight: HashSet::new(),
            completed: HashSet::new(),
        }
    }

    fn mark_pending(&mut self, req: ResourceRequest) -> bool {
        if self.completed.contains(&req) {
            return false;
        }
        if self.in_flight.contains(&req) {
            return false;
        }
        self.pending.insert(req)
    }

    fn take_pending(&mut self) -> HashSet<ResourceRequest> {
        let mut reqs = HashSet::new();
        let pending = self.pending.drain().collect::<Vec<_>>();
        for req in pending {
            if self.mark_in_flight(req.clone()) {
                reqs.insert(req);
            }
        }
        reqs
    }

    fn mark_in_flight(&mut self, req: ResourceRequest) -> bool {
        if self.completed.contains(&req) {
            return false;
        }
        self.pending.remove(&req);
        self.in_flight.insert(req)
    }

    fn mark_completed(&mut self, req: ResourceRequest) -> bool {
        self.pending.remove(&req);
        self.in_flight.remove(&req);
        self.completed.insert(req)
    }

    fn add_rref(&mut self, rref: ResourceRef) {
        let rtype = match rref.resource_type {
            Some(rt) => rt,
            None => return,
        };
        let type_map = self
            .loaded
            .entry(rref.name.clone())
            .or_insert(HashMap::new());
        type_map.entry(rtype).or_insert(None);
    }

    fn add_resource(&mut self, resource: Arc<Resource>) {
        let type_map = self
            .loaded
            .entry(resource.common().name.clone())
            .or_insert(HashMap::new());
        match type_map.entry(resource.resource_type()) {
            Entry::Occupied(mut slot) => {
                warn!(
                    "Overwrote resource {} [{}] in ResourceManager",
                    resource.common().name.as_str(),
                    resource.resource_type()
                );
                slot.insert(Some(resource));
            }
            Entry::Vacant(slot) => {
                slot.insert(Some(resource));
            }
        }
    }

    fn lookup_by_rref(&self, rref: &ResourceRef) -> Result<Arc<Resource>, LookupError> {
        let type_map = match self.loaded.get(&rref.name) {
            Some(tm) => tm,
            None => return Err(LookupError::UnknownName),
        };
        match rref.resource_type {
            Some(rt) => match type_map.get(&rt) {
                Some(Some(r)) => Ok(Arc::clone(r)),
                Some(None) => Err(LookupError::NotLoaded),
                None => Err(LookupError::UnknownNameAndType),
            },
            None => match type_map.len() {
                0 => Err(LookupError::UnknownNameAndType),
                1 => match type_map.values().next().unwrap() {
                    Some(r) => Ok(Arc::clone(r)),
                    None => Err(LookupError::NotLoaded),
                },
                len => Err(LookupError::AmbiguousName(len)),
            },
        }
    }

    fn all_by_type(&self, rtype: ResourceType) -> HashSet<ResourceRef> {
        let mut rrefs = HashSet::new();
        for (name, type_map) in self.loaded.iter() {
            for this_type in type_map.keys() {
                if rtype == *this_type {
                    let rref = ResourceRef::new(name.as_str(), None::<&str>).with_type(Some(rtype));
                    rrefs.insert(rref);
                }
            }
        }
        rrefs
    }
}
