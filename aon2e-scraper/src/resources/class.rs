#![allow(unused)]

use anyhow::{Context as _, Error, Result};
use pf2e_csheet_shared::{
    calc::{CalculatedString, Calculation},
    choices::{Choice, ChoiceKind, ChoiceMeta},
    cond::Condition,
    effects::{self, Effect},
    stats::{Level, Proficiency},
    Class, ClassFeature, ClassInitialProficiencies, Resource, ResourceCommon, ResourceType,
    TypedRef,
};
use scraper::{
    node::{Element, Node},
    ElementRef, Html, Selector,
};
use selectors::attr::CaseSensitivity;
use smallvec::{smallvec, SmallVec};
use smartstring::alias::String;
use std::str::FromStr;

use crate::{
    parsers,
    resources::{Aon2Page, Aon2PageMultiple, ElementRefExt},
};

fn parse_proficiencies(class: &Class, content: &ElementRef) -> Result<ClassInitialProficiencies> {
    let mut profs = ClassInitialProficiencies::default();

    let header = content
        .select(&Selector::parse("h1.title").unwrap())
        .filter(|elem| elem.get_text().as_str() == "Initial Proficiencies")
        .next()
        .context("Failed to find Initial Proficiencies header")?;
    let nodes = header
        .next_siblings()
        .take_while(|n| match n.value().as_element() {
            Some(e) => e.name() != "h1",
            None => true,
        });
    let mut header = None;
    for node in nodes {
        let text: &str = match node.value() {
            Node::Element(e) => {
                if e.name() == "h2" {
                    let e_ref = ElementRef::wrap(node).unwrap();
                    header = Some(e_ref.get_text());
                }
                continue;
            }
            Node::Text(t) => &*t,
            other => {
                debug!("Unexpected node {:?}", other);
                continue;
            }
        };
        if header.is_none() {
            debug!("Skipping node no header has been found yet");
            continue;
        }
        let parts: SmallVec<[&str; 1]> = text.trim().split_whitespace().collect();
        match (header.as_ref().unwrap().as_str(), parts.as_slice()) {
            ("Perception", [level, "in", "Perception"]) => {
                profs.perception = level
                    .parse()
                    .context("Failed to parse initial perception proficiency")?
            }
            ("Saving Throws", [level, "in", "Fortitude"]) => {
                profs.fort_save = level
                    .parse()
                    .context("Failed to parse initial FORT save proficiency")?
            }
            ("Saving Throws", [level, "in", "Reflex"]) => {
                profs.reflex_save = level
                    .parse()
                    .context("Failed to parse initial REF save proficiency")?
            }
            ("Saving Throws", [level, "in", "Will"]) => {
                profs.will_save = level
                    .parse()
                    .context("Failed to parse initial WILL save proficiency")?
            }
            (
                "Skills",
                ["Trained", "in", "a", "number", "of", "skills", "equal", "to", n, "plus", "your", "Intelligence", "modifier"],
            ) => {
                profs.free_skill_trained = n.parse().context("Failed to parse free skill count")?;
            }
            ("Attacks", [level, "in", "unarmed", "attacks"]) => {
                profs.weapon_proficiencies.unarmed = level
                    .parse()
                    .context("Failed to parse unarmed attack proficiencies")?;
            }
            ("Attacks", [level, "in", "simple", "weapons"]) => {
                profs.weapon_proficiencies.simple = level
                    .parse()
                    .context("Failed to parse simple weapon proficiencies")?;
            }
            ("Attacks", [level, "in", "martial", "weapons"]) => {
                profs.weapon_proficiencies.martial = level
                    .parse()
                    .context("Failed to parse martial weapon proficiencies")?;
            }
            ("Attacks", [level, "in", "advanced", "weapons"]) => {
                profs.weapon_proficiencies.advanced = level
                    .parse()
                    .context("Failed to parse advanced weapon proficiencies")?;
            }
            ("Defenses", [level, "in", "all", "armor"]) => {
                let level = level
                    .parse()
                    .context("Failed to parse armor proficiencies")?;
                profs.armor_proficiencies.light = level;
                profs.armor_proficiencies.medium = level;
                profs.armor_proficiencies.heavy = level;
            }
            ("Defenses", [level, "in", "unarmored", "defense"]) => {
                profs.armor_proficiencies.unarmored = level
                    .parse()
                    .context("Failed to parse unarmored proficiency")?;
            }
            ("Defenses", [level, "in", "light", "armor"]) => {
                profs.armor_proficiencies.light =
                    level.parse().context("Failed to parse light proficiency")?;
            }
            ("Defenses", [level, "in", "medium", "armor"]) => {
                profs.armor_proficiencies.medium = level
                    .parse()
                    .context("Failed to parse medium proficiency")?;
            }
            ("Defenses", [level, "in", "heavy", "armor"]) => {
                profs.armor_proficiencies.heavy =
                    level.parse().context("Failed to parse heavy proficiency")?;
            }
            ("Class DC", [level, "in", name, "class", "DC"]) => {
                debug!("Skipping {} class DC level {}", name, level);
            }
            (
                "Skills",
                ["Trained", "in", "a", "number", "of", "additional", "skills", "equal", "to", n, "plus", "your", "Intelligence", "modifier"],
            ) => {
                profs.free_skill_trained = n
                    .parse()
                    .context("Failed to parse free trained skill count")?;
            }
            ("Skills", words) if words.len() > 2 && words[1] == "in" => {
                let level: Proficiency = words[0].parse()?;
                let skill_name = words[2..].join(" ");
                match skill_name.parse() {
                    Ok(skill) => {
                        anyhow::ensure!(level == Proficiency::Trained, "Current model only supports initial skill proficiency of Trained, found {:?}", level);
                        profs.skills_trained.push(skill);
                    }
                    Err(e) => warn!(
                        "Failed to parse initial skill proficiency from {:?}: {}",
                        skill_name, e
                    ),
                }
            }
            (h, other) => {
                warn!(
                    "Found unexpected set of text for initial {:?} proficiencies: {:?}",
                    h, &other
                );
            }
        }
    }
    Ok(profs)
}

fn parse_class_feature(
    class: &Class,
    profs: &ClassInitialProficiencies,
    header: ElementRef,
) -> Result<ClassFeature> {
    let header_elem: &Element = header.value();
    anyhow::ensure!(
        header_elem.name() == "h2"
            && header_elem.has_class("title", CaseSensitivity::AsciiCaseInsensitive),
        "Invalid starting element, expected one matching `h2.title`"
    );

    let feature_name: String = header
        .children()
        .filter_map(|n| n.value().as_text())
        .next()
        .context("Failed to find feature name")?
        .replace('\u{2019}', "'")
        .into();
    let feature_name_lowercase = feature_name.to_lowercase();
    let name_segments: SmallVec<[&str; 10]> = feature_name_lowercase.split_whitespace().collect();
    let mut common: ResourceCommon = match name_segments.as_slice() {
        ["ancestry", "and", "background"] => {
            let ancestry_effect = effects::GrantResourceChoiceEffect {
                common: effects::EffectCommon::default(),
                choice: "ancestry".into(),
                resource_type: ResourceType::Ancestry,
            };
            let background_effect = effects::GrantResourceChoiceEffect {
                common: effects::EffectCommon::default(),
                choice: "background".into(),
                resource_type: ResourceType::Ancestry,
            };
            let ancestry_choice_meta = ChoiceMeta {
                kind: ChoiceKind::Resource {
                    resource_type: ResourceType::Ancestry,
                    trait_filter: None,
                },
                from: None,
                key: false,
                character_wide: true,
                description: None,
            };
            let background_choice_meta = ChoiceMeta {
                kind: ChoiceKind::Resource {
                    resource_type: ResourceType::Background,
                    trait_filter: None,
                },
                from: None,
                key: false,
                character_wide: true,
                description: None,
            };
            let mut common = ResourceCommon::new("Ancestry and Background");
            common.add_choice(ancestry_effect.choice.clone(), ancestry_choice_meta);
            common.add_effect(Effect::GrantResourceChoice(ancestry_effect));
            common.add_choice(background_effect.choice.clone(), background_choice_meta);
            common.add_effect(Effect::GrantResourceChoice(background_effect));
            common
        }
        ["initial", "proficiencies"] => {
            let mut common = ResourceCommon::new(format!(
                "{} Initial Proficiencies",
                class.common.name.as_str()
            ));
            {
                let mut handle_prof = |level: Proficiency, name: &str| match level {
                    Proficiency::Untrained => (),
                    _ => {
                        let effect = effects::IncreaseProficiencyEffect {
                            common: effects::EffectCommon::default(),
                            target: name.into(),
                            level,
                        };
                        common.add_effect(Effect::IncreaseProficiency(effect));
                    }
                };
                handle_prof(profs.perception, "Perception");
                handle_prof(profs.fort_save, "FORT");
                handle_prof(profs.reflex_save, "REF");
                handle_prof(profs.will_save, "WILL");
                // TODO: skills
                handle_prof(profs.weapon_proficiencies.unarmed, "unarmed attacks");
                handle_prof(profs.weapon_proficiencies.simple, "simple weapons");
                handle_prof(profs.weapon_proficiencies.martial, "martial weapons");
                handle_prof(profs.weapon_proficiencies.advanced, "advanced weapons");
                handle_prof(profs.armor_proficiencies.unarmored, "unarmored defense");
                handle_prof(profs.armor_proficiencies.light, "light armor");
                handle_prof(profs.armor_proficiencies.medium, "medium armor");
                handle_prof(profs.armor_proficiencies.heavy, "heavy armor");
            }
            common
        }
        [_, "feat"] | [_, "feats"] => {
            let trait_name: String = feature_name.split_whitespace().next().unwrap().into();
            let effect = effects::GrantResourceChoiceEffect {
                common: effects::EffectCommon::default(),
                choice: "feat".into(),
                resource_type: ResourceType::Feat,
            };
            let feat_choice_meta = ChoiceMeta {
                kind: ChoiceKind::Resource {
                    resource_type: ResourceType::Feat,
                    trait_filter: Some(trait_name.clone()),
                },
                from: None,
                key: false,
                character_wide: false,
                description: None,
            };
            let mut common = ResourceCommon::new(format!("{} Feat", trait_name));
            common.add_choice(effect.choice.clone(), feat_choice_meta);
            common.add_effect(Effect::GrantResourceChoice(effect));
            common
        }
        ["skill", "increase"] | ["skill", "increases"] => {
            let effect = effects::SkillIncreaseEffect {
                common: effects::EffectCommon::default(),
            };
            let res_choice_meta = ChoiceMeta {
                kind: ChoiceKind::Skill,
                from: None,
                key: false,
                character_wide: false,
                description: None,
            };
            let mut common = ResourceCommon::new("Skill Increase");
            common.add_effect(Effect::SkillIncrease(effect));
            common.add_choice("skill", res_choice_meta);
            common
        }
        ["ability", "boosts"] => ResourceCommon::new("Ability Boosts"),
        _ => ResourceCommon::new(feature_name),
    };
    // Parse description and actions
    let mut description = String::new();
    for node in header.next_siblings() {
        match node.value() {
            Node::Text(t) => description.push_str(&*t),
            Node::Element(e) => match e.name() {
                "br" => description.push('\n'),
                "h1" => break,
                "h2" => break,
                "h3" => break,
                "h4" => break,
                name => {
                    trace!("Getting content from a {:?} element", name);
                    description.push_str(&ElementRef::wrap(node).unwrap().get_text())
                }
            },
            _ => (),
        }
    }
    // if description.contains("level") {
    //     common.add_choice(
    //         "level",
    //         ChoiceMeta {
    //             kind: ChoiceKind::Level,
    //             from: None,
    //             key: true,
    //             optional: false,
    //             description: None,
    //         },
    //     );
    // }
    for sentence in parsers::SEGMENTER.segment(description.trim()) {
        let sentence = sentence.trim();
        trace!("parsing sentence {:?}", sentence);
        match parsers::class_feature(sentence) {
            Ok(parts) => {
                for t in parts.traits {
                    common.traits.push(t);
                }
                common.description = parts.description;
                for (name, meta) in parts.choices {
                    common.add_choice(name, meta);
                }
                for e in parts.effects {
                    common.add_effect(e);
                }
            }
            Err(e) => warn!("Failed to parse description: {}", e),
        }
    }

    Ok(ClassFeature {
        common,
        class: TypedRef::new(class.common.name.clone(), None::<&str>),
    })
}

fn parse_advancements(
    class: &mut Class,
    class_features: &[ClassFeature],
    content: &ElementRef,
) -> Result<()> {
    // advancement table
    let table = {
        let header_selector =
            Selector::parse("h1.title + table.inner[style] tr > td + td").unwrap();
        let mut table = None;
        for t in content.select(&header_selector) {
            if table.is_some() {
                break;
            }
            let text = t.get_text();
            if text.as_str() != "Class Features" {
                debug!("Found the wrong table! Second header cell is {:?}", text);
                continue;
            }
            table = t
                .ancestors()
                .filter(|elem| {
                    elem.value()
                        .as_element()
                        .map(|e| e.name() == "table")
                        .unwrap_or(false)
                })
                .next()
                .and_then(ElementRef::wrap);
        }
        table.context("Failed to find advancement table")?
    };
    debug!("Found advancement table");

    let td_selector = Selector::parse("td").unwrap();
    for row in table.select(&Selector::parse("tr").unwrap()) {
        let cells: SmallVec<[ElementRef; 2]> = row.select(&td_selector).collect();
        match cells.as_slice() {
            [level_cell, value_cell] => {
                let level_text = level_cell.get_text();
                let level = match level_text.parse::<Level>() {
                    Ok(level) => level,
                    Err(_) => continue,
                };
                let value_text = value_cell.get_text();
                let feature_refs = class.advancement.entry(level).or_insert(vec![]);
                for feature_name in value_text.split(',') {
                    let feature_name = match feature_name.trim().replace('\u{2019}', "'").as_str() {
                        "initial proficiencies" => {
                            format!("{} Initial Proficiencies", class.common.name.as_str())
                        }
                        other => other.into(),
                    };
                    let feature_name_lc = feature_name.to_ascii_lowercase();
                    let parts: SmallVec<[&str; 10]> = feature_name_lc.split_whitespace().collect();
                    trace!("Got class feature name {:?}", &feature_name);
                    // ensure the name starts with an existing class feature
                    let mut rref: Option<TypedRef<ClassFeature>> = None;
                    for cf in class_features.iter() {
                        let cf_name_lc = cf.common.name.to_ascii_lowercase();
                        trace!(
                            "Checking if {:?} starts with {:?}",
                            feature_name_lc,
                            cf_name_lc
                        );
                        let starts_with_plain_index = if feature_name_lc.starts_with(&cf_name_lc) {
                            Some(cf_name_lc.len())
                        } else {
                            None
                        };
                        let starts_with_class_index =
                            if format!("{} {}", class.common.name.to_lowercase(), feature_name_lc)
                                .starts_with(&cf_name_lc)
                            {
                                let prefix_len = class.common.name.to_lowercase().len() + 1;
                                Some(cf_name_lc.len() - prefix_len)
                            } else {
                                None
                            };
                        let rest: &str = match starts_with_plain_index.or(starts_with_class_index) {
                            Some(i) => &feature_name_lc[i..].trim(),
                            None => continue,
                        };
                        trace!("It does!");
                        let modifier: Option<String> = match rest {
                            "" => {
                                let mut level_mod = None;
                                for (choice, meta) in cf.common.choices.iter() {
                                    if meta.key {
                                        match meta.kind {
                                            ChoiceKind::Level => {
                                                trace!("Found level modifier {}", level);
                                                level_mod = Some(format!("{}", level));
                                                break;
                                            }
                                            _ => (),
                                        }
                                    }
                                }
                                level_mod
                            }
                            _ => {
                                trace!("Found modifier {:?}", rest);
                                Some(rest.into())
                            }
                        };
                        rref = Some(TypedRef::new(cf.common.name.as_str(), modifier));
                        break;
                    }
                    let rref = rref.context("Failed to match feature name")?;
                    feature_refs.push(rref);
                }
            }
            _ => (),
        }
    }
    Ok(())
}

pub async fn html_to_shared(doc: &Html) -> Result<(Class, Vec<Resource>)> {
    let content_selector = Selector::parse("#ctl00_MainContent_DetailedOutput").unwrap();
    let content = doc
        .select(&content_selector)
        .next()
        .context("Failed to find body content")?;

    let class_name = content
        .select(&Selector::parse("h1.title").unwrap())
        .next()
        .context("Failed to get name of class")?
        .get_text();
    debug!("Parsing class {:?}", class_name.as_str());

    let class_description = content
        .select(&Selector::parse("#ctl00_MainContent_DetailedOutput > i").unwrap())
        .next()
        .context("Failed to get class description")?
        .get_text();
    let class_traits = vec![class_name.clone()];
    let mut common = ResourceCommon::new(class_name);
    common.description = Some(class_description.parse()?);
    common.traits = class_traits;

    let key_ability = content
        .select(&Selector::parse("#ctl00_MainContent_DetailedOutput > b").unwrap())
        .filter_map(|elem| {
            let s = elem.get_text();
            if s.starts_with("Key Ability: ") {
                let rest = &s["Key Ability: ".len()..];
                let mut abilities = smallvec![];
                for ability_text in rest.split(" OR ") {
                    match ability_text.parse() {
                        Ok(a) => abilities.push(a),
                        Err(e) => {
                            warn!("Failed to parse abilities: {}", e);
                            return None;
                        }
                    }
                }
                Some(abilities)
            } else {
                None
            }
        })
        .next()
        .context("Failed to find key ability for class")?;

    let hp_per_level = content
        .select(&Selector::parse("#ctl00_MainContent_DetailedOutput > b").unwrap())
        .filter_map(|elem| {
            let s = elem.get_text();
            if s.starts_with("Hit Points: ") {
                let rest = &s["Hit Points: ".len()..].trim();
                let mut digits_end = 0;
                for (i, c) in rest.char_indices() {
                    digits_end = i;
                    if !c.is_ascii_digit() {
                        break;
                    }
                }
                let digits = &rest[..digits_end];
                debug!("Parsing HP per level from {:?}", digits);
                let hp_per_level: Calculation = digits.parse().unwrap();
                Some(hp_per_level)
            } else {
                None
            }
        })
        .next()
        .context("Failed to find HP per level for class")?;

    let mut class = Class {
        common: common,
        key_ability,
        hp_per_level,
        advancement: Default::default(),
    };

    let profs = parse_proficiencies(&class, &content)?;
    let class_features = {
        let mut features = vec![];
        let header_selector = Selector::parse("h1.title, h2.title").unwrap();
        let headers = content
            .select(&header_selector)
            .skip_while(|e| e.get_text().as_str() != "Class Features")
            .skip(1)
            .take_while(|e| e.value().name() != "h1");
        for header in headers {
            let cf = parse_class_feature(&class, &profs, header)
                .context("Failed to parse class feature")?;
            features.push(cf);
        }
        features
    };

    let class_feats = {
        let nav_link_selector =
            Selector::parse("#ctl00_MainContent_SubNavigation a[href]").unwrap();
        let mut page = None;
        for a_elemref in doc.select(&nav_link_selector) {
            let href: &str = a_elemref.value().attr("href").unwrap();
            match Aon2Page::from_path(href) {
                Ok(r @ Aon2Page::Multiple(Aon2PageMultiple::Feats { .. })) => {
                    page = Some(r);
                    break;
                }
                _ => (),
            }
        }
        let page = page.context("Failed to find class feats page")?;
        let feats = page
            .as_multiple_shared_resources()
            .await
            .context("Failed to get class feats page")?;
        feats
        // vec![]
    };

    parse_advancements(&mut class, &class_features, &content)?;

    let other_resources = {
        let mut rs = Vec::with_capacity(class_features.len() + class_feats.len());
        rs.extend(class_features.into_iter().map(Resource::ClassFeature));
        rs.extend(class_feats);
        rs
    };

    Ok((class, other_resources))
}
