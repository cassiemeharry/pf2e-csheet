use anyhow::{ensure, Context as _, Result};
use pf2e_csheet_shared::{
    cond::Condition, effects, stats::Level, Action, ActionType, Feat, Resource, ResourceCommon,
    ResourceRef, ResourceType,
};
use scraper::{Html, Node, Selector};
use smartstring::alias::String;
use std::collections::{HashMap, HashSet};

use crate::{
    parsers,
    resources::{Aon2Page, ElementRefExt},
};

pub async fn parse_feat_page(doc: &Html) -> Result<(Feat, Vec<Resource>)> {
    let content = doc
        .select(&Selector::parse("#ctl00_MainContent_DetailedOutput").unwrap())
        .next()
        .context("Failed to find feat content")?;
    let (title, level, action_opt) = {
        let title_h1 = content
            .select(&Selector::parse("h1.title").unwrap())
            .next()
            .context("Failed to find feat title")?;
        let mut title_opt = None;
        let mut level_opt: Option<Level> = None;
        let mut action_opt = None;
        for elem in title_h1.children() {
            match elem.value() {
                Node::Element(e) => {
                    let text = elem.get_text();
                    match (e.name(), e.attr("href"), e.attr("alt")) {
                        ("a", Some(href), _) if href.contains("Feats.aspx?ID=") => {
                            title_opt = Some(text);
                        }
                        ("img", _, Some("Free Action")) => action_opt = Some(ActionType::Free),
                        ("img", _, Some("Reaction")) => action_opt = Some(ActionType::Reaction),
                        ("img", _, Some("Single Action")) => action_opt = Some(ActionType::One),
                        ("img", _, Some("Two Actions")) => action_opt = Some(ActionType::Two),
                        ("img", _, Some("Three Actions")) => action_opt = Some(ActionType::Two),
                        ("span", _, _) if text.starts_with("Feat ") => {
                            let level_text = &text[5..];
                            let level = level_text.parse().context("Failed to parse feat level")?;
                            level_opt = Some(level);
                        }
                        (other_name, _, _) => {
                            debug!("Skipping <{}> element", other_name);
                        }
                    }
                }
                _ => (),
            }
        }
        (
            title_opt.context("Failed to find title for feat")?,
            level_opt.context("Failed to find title for feat")?,
            action_opt,
        )
    };

    debug!("Found title for level {} feat: {:?}", level, title);
    debug!("action_opt: {:?}", action_opt);

    fn text_until_br<'a>(
        node_refs: impl IntoIterator<Item = ego_tree::NodeRef<'a, Node>>,
    ) -> String {
        let mut text = String::new();
        for node_ref in node_refs {
            match node_ref.value() {
                Node::Text(t) => text.push_str(&t),
                Node::Element(e) if e.name() == "br" => break,
                Node::Element(_) => text.push_str(&node_ref.get_text()),
                _ => debug!("Skipping unexpected node in text_until_br"),
            }
        }
        text.trim().into()
    }

    let (prereqs, reqs): (Condition, Condition) = {
        let mut prereqs = Condition::None;
        let mut reqs = Condition::None;
        let selector = Selector::parse("b").unwrap();
        for node in content.select(&selector) {
            let text = node.get_text();
            match text.as_str() {
                "Prerequisites" => {
                    let prereq_text = text_until_br(node.next_siblings());
                    trace!("Parsing prerequisites {:?}", prereq_text);
                    // for tree in sentences_to_trees(&prereq_text) {
                    //     debug!("Feat prereq sentence tree:\n{}", tree);
                    // }
                    match parsers::conditions(&prereq_text) {
                        Ok(p) => prereqs &= p,
                        Err(e) => warn!("Failed to parse prereq text {:?}: {}", prereq_text, e),
                    }
                }
                "Requirements" => {
                    let reqs_text = text_until_br(node.next_siblings());
                    trace!("Parsing requirements {:?}", reqs_text);
                    // for tree in sentences_to_trees(&reqs_text) {
                    //     debug!("Feat reqs sentence tree:\n{}", tree);
                    // }
                    match parsers::conditions(&reqs_text) {
                        Ok(r) => reqs &= r,
                        Err(e) => warn!("Failed to parse requirements text {:?}: {}", reqs_text, e),
                    }
                }
                _ => debug!("Found unexpected header {:?}", text),
            }
        }
        (prereqs, reqs)
    };

    debug!("Found prerequisites: {:?}", prereqs);
    debug!("Found requirements:  {:?}", reqs);

    let traits: Vec<String> = content
        .select(&Selector::parse("span.trait > a").unwrap())
        .map(|e| e.get_text())
        .collect();
    debug!("Found traits: {:?}", traits);

    let raw_description = {
        let mut buffer = String::new();
        let hr = content
            .select(&Selector::parse("hr").unwrap())
            .next()
            .context("Failed to find <hr> before feat description")?;
        for node in hr.next_siblings() {
            if let Node::Element(e) = node.value() {
                match e.name() {
                    "h1" | "h2" | "h3" | "h4" => break,
                    _ => (),
                }
            }
            buffer.push_str(&node.get_text());
        }
        buffer
    };
    // for tree in sentences_to_trees(&raw_description) {
    //     debug!("Feat description sentence tree:\n{}", tree);
    // }
    let description = raw_description
        .parse()
        .context("Failed to parse feat description")?;

    match action_opt {
        Some(action_type) => {
            let mut feat_common = ResourceCommon::new(title);
            feat_common.add_traits(&traits);
            feat_common.add_prerequisite(prereqs);
            feat_common.add_requirement(reqs);
            let mut action_common = feat_common.clone();
            action_common.set_description(description);
            let action = Action {
                common: action_common,
                action_type,
            };
            let effect = effects::GrantSpecificResourceEffect {
                common: effects::EffectCommon::default(),
                resource: ResourceRef::new(feat_common.name.as_str(), None::<&str>)
                    .with_type(Some(ResourceType::Action)),
            };
            feat_common.add_effect(effect);
            let feat_desc_raw = format!("You gain the {} action.", feat_common.name.as_str());
            let feat_desc = feat_desc_raw
                .parse()
                .context("Failed to set action feat description")?;
            feat_common.set_description(feat_desc);
            let feat = Feat {
                common: feat_common,
                level,
            };
            Ok((feat, vec![Resource::Action(action)]))
        }
        None => {
            let mut common = ResourceCommon::new(title);
            common.add_traits(&traits);
            common.add_prerequisite(prereqs);
            common.add_requirement(reqs);
            let feat = Feat { common, level };
            Ok((feat, vec![]))
        }
    }
}

pub async fn parse_feats_by_trait_page(doc: &Html) -> Result<Vec<Resource>> {
    let mut by_name_type: HashMap<(String, ResourceType), Resource> = HashMap::new();
    let mut already_parsed: HashSet<Aon2Page> = HashSet::new();

    let a_selector = Selector::parse("td a[href]").unwrap();
    for feat_link in doc.select(&a_selector) {
        let href = feat_link.value().attr("href").unwrap();
        if !href.contains("Feats.aspx?ID=") {
            trace!("Skipping unexpected link to {:?}", href);
            continue;
        }
        let page = Aon2Page::from_path(href).context("Failed to parse feat page URL")?;
        if !already_parsed.insert(page) {
            warn!("Saw page {:?} again", page);
            continue;
        }
        let (feat, supporting) = page
            .as_single_shared_resource()
            .await
            .context("Failed to parse individual feat")?;
        let prev = by_name_type.insert((feat.common().name.clone(), ResourceType::Feat), feat);
        ensure!(
            prev.is_none(),
            "Feat {} was parsed multiple times",
            prev.unwrap().common().name
        );
        for s in supporting {
            let prev = by_name_type.insert((s.common().name.clone(), s.resource_type()), s);
            ensure!(
                prev.is_none(),
                "Resource {} ({:?}) was parsed multiple times",
                prev.as_ref().unwrap().common().name,
                prev.as_ref().unwrap().resource_type()
            );
        }
    }
    let mut resources: Vec<Resource> = by_name_type.into_iter().map(|(_k, v)| v).collect();
    resources.sort_by_cached_key(|r| (r.common().name.clone(), r.resource_type()));
    Ok(resources)
}
