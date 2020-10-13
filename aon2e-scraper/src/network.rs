use anyhow::{ensure, Context as _, Result};
use chrono::{prelude::*, Duration};
use reqwest::header::CONTENT_TYPE;
use scraper::Html;
use smartstring::alias::String;
use std::{
    fmt,
    path::{Path, PathBuf},
};
use tokio::{sync::Mutex, time};
use url::Url;

pub const BASE_URL: &'static str = "https://2e.aonprd.com";
lazy_static::lazy_static! {
    static ref CACHE_DURATION: Duration = Duration::weeks(1);
}

#[derive(Eq, PartialEq)]
struct CacheKey(String);

impl<'a> From<&'a Url> for CacheKey {
    fn from(url: &'a Url) -> CacheKey {
        let key = format!(
            "{}?{}",
            url.path().trim_start_matches('/').replace('/', "__"),
            url.query().unwrap_or("")
        );
        assert!(!key.chars().any(|c| c == '/'));
        CacheKey(key)
    }
}

impl fmt::Display for CacheKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for CacheKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.0.as_str(), f)
    }
}

impl CacheKey {
    fn as_path(&self) -> Result<PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let cache_dir = root.join("aon2e-cache");
        if !cache_dir.is_dir() {
            debug!("Creating download cache directory");
            std::fs::create_dir(&cache_dir).context("Failed to create cache directory")?;
        };
        let item_path = cache_dir.join(format!("{}.html", self.0).as_str());
        debug!("Cache item path: {}", item_path.display());
        Ok(item_path)
    }
}

fn check_cache(key: &CacheKey, now: DateTime<Utc>) -> Result<Html> {
    let item_path = key.as_path()?;
    // let metadata = item_path
    //     .metadata()
    //     .with_context(|| format!("Failed to get metadata for cache entry {:?}", key))?;
    // let created = metadata.modified().with_context(|| {
    //     format!(
    //         "Failed to get created at timestamp of cache entry {:?}",
    //         key
    //     )
    // })?;
    // let created: DateTime<Utc> = created.into();
    // let age: Duration = now - created;
    // ensure!(age < *CACHE_DURATION, "Cache item {:?} expired", key);
    let contents = std::fs::read_to_string(&item_path)
        .with_context(|| format!("Failed to read cache file {}", item_path.display()))?;
    let html = Html::parse_document(contents.as_str());
    Ok(html)
}

lazy_static::lazy_static! {
    static ref LAST_PAGE_TIME: Mutex<time::Instant> = Mutex::new(time::Instant::now());
}

const NETWORK_COOLDOWN: time::Duration = time::Duration::from_secs(2);

pub async fn get_page(url: Url) -> Result<Html> {
    info!("Loading page {}", url);
    let key = (&url).into();
    debug!("Cache key: {:?}", key);
    match check_cache(&key, Utc::now()) {
        Ok(cached) => {
            debug!("Got page from cache");
            return Ok(cached);
        }
        Err(e) => warn!("Failed to get page from cache: {:}", e),
    }

    let mut last_page_time = loop {
        let guard = LAST_PAGE_TIME.lock().await;
        let now = time::Instant::now();
        let goal_time = *guard + NETWORK_COOLDOWN;
        if now >= goal_time {
            break guard;
        }
        drop(guard);
        time::delay_until(goal_time).await;
    };
    let key_path = key.as_path()?;
    let response = reqwest::get(url.as_str()).await?;
    ensure!(
        response.status().as_u16() == 200,
        "Got non-200 response: {}",
        response.status().as_str()
    );
    if let Some(ct_val) = response.headers().get(CONTENT_TYPE) {
        let ct_str = ct_val.to_str()?;
        ensure!(
            ct_str.starts_with("text/html"),
            "Unexpected content type: {:?} (expected \"text/html\")",
            ct_str
        );
    }
    let raw_html = response.text().await?;

    *last_page_time = time::Instant::now();
    drop(last_page_time);

    std::fs::write(key_path, raw_html.as_bytes())?;
    let html = Html::parse_document(raw_html.as_str());
    Ok(html)
}
