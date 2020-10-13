use anyhow::Result;
use pf2e_csheet_shared::{Resource, ResourceRef, ResourceType};
use rocket::{
    config::{Config, Environment},
    request::{FromQuery, Query},
    response::content,
    State,
};
use rocket_contrib::{json::Json, serve::StaticFiles};
use smartstring::alias::String as SmartString;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::RwLock;

use crate::resources::ResourceStore;

type ManagedResourceStore = RwLock<ResourceStore>;

struct ResourceNameList {
    map: HashMap<String, Option<ResourceType>>,
}

impl<'q> FromQuery<'q> for ResourceNameList {
    type Error = String;

    fn from_query(query: Query<'q>) -> Result<Self, Self::Error> {
        let mut map = HashMap::new();
        for item in query {
            let key = item
                .key
                .url_decode()
                .map_err(|e| std::format!("Failed to decode resource name list: {}", e))?;
            let rtype = match rocket::request::FromFormValue::from_form_value(&item.value) {
                Ok(rt) => rt,
                Err(e) => return Err(std::format!("Failed to decode resource name list: {}", e))?,
            };
            map.insert(key, rtype);
        }
        Ok(Self { map })
    }
}

#[get("/resources?<names..>")]
async fn get_resources(
    store: State<'_, ManagedResourceStore>,
    names: ResourceNameList,
) -> Option<Json<HashMap<SmartString, Option<Arc<Resource>>>>> {
    let mut map = HashMap::new();
    let store_read = store.read().await;
    let rrefs = names
        .map
        .into_iter()
        .map(|(name, rtype)| ResourceRef::new(&name, None::<&str>).with_type(rtype))
        .collect::<Vec<_>>();
    let rrefs_indirect = rrefs.iter().collect::<Vec<&_>>();
    let results = store_read.lookup_async(rrefs_indirect.as_slice()).await;
    for (rref, result) in rrefs.into_iter().zip(results.into_iter()) {
        map.insert(rref.name.into(), result);
    }
    Some(Json(map))
}

#[get("/resources/by-type?<t>")]
async fn get_resources_by_type(
    store: State<'_, ManagedResourceStore>,
    t: ResourceType,
) -> Json<HashSet<ResourceRef>> {
    let store_read = store.read().await;
    let result = store_read.all_by_type(t).await;
    Json(result)
}

#[get("/resources/by-trait?<t>")]
async fn get_resources_by_trait(
    store: State<'_, ManagedResourceStore>,
    t: String,
) -> Json<Vec<Arc<Resource>>> {
    let _ = store;
    let _ = t;
    Json(vec![])
}

#[get("/")]
async fn homepage() -> content::Html<&'static str> {
    let page = r###"<!doctype html>
<html lang="en">
  <head>
    <title>Pathfinder 2E Character Sheet Builder</title>
    <link rel="stylesheet" href="/static/stylesheet.css">
  </head>
  <body>
    <script type="module">
import init from '/static/pf2e_csheet_frontend.js';

async function run() {
  const wasm = await init();
  wasm.run_app();
}
run();
    </script>
</html>
"###;

    content::Html(page)
}

pub async fn serve(storage: ResourceStore) -> Result<()> {
    let config = Config::build(Environment::Development)
        .address("192.168.1.252")
        .port(8000)
        .finalize()?;

    info!("About to run server");
    rocket::custom(config)
        .manage(RwLock::new(storage))
        .mount(
            "/static",
            StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../static")),
        )
        .mount(
            "/",
            routes![
                homepage,
                get_resources,
                get_resources_by_type,
                get_resources_by_trait
            ],
        )
        .launch()
        .await?;

    Ok(())
}
