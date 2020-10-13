use anyhow::{Context as _, Result};
use pf2e_csheet_shared::Resource;
use scraper::{ElementRef, Node};
use smartstring::alias::String;
use std::{borrow::Cow, collections::HashMap, future::Future, pin::Pin};
use url::Url;

use crate::network;

mod class;
mod feats;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Aon2PageSingle {
    Class { id: usize },
    Feat { id: usize },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Aon2PageMultiple {
    Feats { trait_id: usize },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Aon2Page {
    Single(Aon2PageSingle),
    Multiple(Aon2PageMultiple),
}

type Query<'a> = HashMap<Cow<'a, str>, Cow<'a, str>>;

impl Aon2Page {
    pub fn from_path(path: &str) -> Result<Self> {
        let url: Url = if path.starts_with("http") {
            path.parse()?
        } else {
            format!("{}/{}", network::BASE_URL, path.trim_start_matches('/')).parse()?
        };
        anyhow::ensure!(
            url.host_str() == Some("2e.aonprd.com"),
            "Invalid hostname given to Aon2Resource::from_path"
        );
        let query: Query = url.query_pairs().collect();
        if let Some(single) = Aon2PageSingle::from_path(url.path(), &query) {
            Ok(Self::Single(single))
        } else if let Some(multiple) = Aon2PageMultiple::from_path(url.path(), &query) {
            Ok(Self::Multiple(multiple))
        } else {
            anyhow::bail!("Unknown page {:?}", url.as_str())
        }
    }

    // fn as_url(&self) -> Url {
    //     match self {
    //         Self::Single(s) => s.as_url(),
    //         Self::Multiple(m) => m.as_url(),
    //     }
    // }

    // async fn get_html(&self) -> Result<Html> {
    //     network::get_page(self.as_url()).await
    // }

    pub async fn as_single_shared_resource(&self) -> Result<(Resource, Vec<Resource>)> {
        match self {
            Self::Single(s) => s.as_shared_resource().await,
            Self::Multiple(_) => anyhow::bail!("This is a multiple-resource page"),
        }
    }

    pub async fn as_multiple_shared_resources(&self) -> Result<Vec<Resource>> {
        match self {
            Self::Single(_) => anyhow::bail!("This is a multiple-resource page"),
            Self::Multiple(m) => m.as_shared_resources().await,
        }
    }
}

impl Aon2PageSingle {
    fn from_path(path: &str, query: &Query) -> Option<Self> {
        match path {
            "/Classes.aspx" => {
                let class_id_str = query.get("ID")?;
                let id = class_id_str.parse().ok()?;
                Some(Self::Class { id })
            }
            "/Feats.aspx" => {
                let class_id_str = query.get("ID")?;
                let id = class_id_str.parse().ok()?;
                Some(Self::Feat { id })
            }
            _ => None,
        }
    }

    fn as_url(&self) -> Url {
        match self {
            Self::Class { id } => format!("{}/Classes.aspx?ID={}", network::BASE_URL, id)
                .parse()
                .unwrap(),
            Self::Feat { id } => format!("{}/Feats.aspx?ID={}", network::BASE_URL, id)
                .parse()
                .unwrap(),
        }
    }

    pub fn as_shared_resource(&self) -> BoxFuture<Result<(Resource, Vec<Resource>)>> {
        Box::pin(async move {
            let url = self.as_url();
            let html = network::get_page(url).await?;
            match self {
                Self::Class { .. } => {
                    let (class, extra) = class::html_to_shared(&html)
                        .await
                        .context("Failed to parse class")?;
                    Ok((Resource::Class(class), extra))
                }
                Self::Feat { .. } => {
                    let (class, extra) = feats::parse_feat_page(&html)
                        .await
                        .context("Failed to parse feat page")?;
                    Ok((Resource::Feat(class), extra))
                }
            }
        })
    }
}

impl Aon2PageMultiple {
    fn from_path(path: &str, query: &Query) -> Option<Self> {
        match path {
            "/Feats.aspx" => {
                let class_id_str = query.get("Traits")?;
                let trait_id = class_id_str.parse().ok()?;
                Some(Self::Feats { trait_id })
            }
            _ => None,
        }
    }

    fn as_url(&self) -> Url {
        match self {
            Self::Feats { trait_id } => {
                format!("{}/Feats.aspx?Traits={}", network::BASE_URL, trait_id)
                    .parse()
                    .unwrap()
            }
        }
    }

    pub fn as_shared_resources(&self) -> BoxFuture<Result<Vec<Resource>>> {
        Box::pin(async move {
            let url = self.as_url();
            let html = network::get_page(url).await?;
            match self {
                Self::Feats { .. } => {
                    let feats = feats::parse_feats_by_trait_page(&html).await?;
                    Ok(feats)
                }
            }
        })
    }
}

trait ElementRefExt {
    fn get_text(&self) -> String;
}

impl ElementRefExt for ElementRef<'_> {
    fn get_text(&self) -> String {
        let mut buffer = String::new();
        for s in self.text() {
            buffer.push_str(s);
        }
        buffer.trim().into()
    }
}

impl ElementRefExt for ego_tree::NodeRef<'_, Node> {
    fn get_text(&self) -> String {
        match self.value() {
            Node::Element(_) => ElementRef::wrap(*self).unwrap().get_text(),
            Node::Text(t) => t.trim().into(),
            _ => "".into(),
        }
    }
}
