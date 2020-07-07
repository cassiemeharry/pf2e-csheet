#![deny(bindings_with_variant_name)]
#![deny(unreachable_patterns)]

use anyhow::{Error, Result};
use lazy_static::lazy_static;
use smartstring::alias::String;
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

mod bonuses;
mod pdf;
mod raw_pdf_manip;
// mod using_pdf_form;

use bonuses::{Bonus, HasModifiers, Modifier, Modifies, Penalty};
use pdf::{CheckboxID, PDFOutput, ProficiencyFields, SkillSlot, TextID, WeaponSlot};

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum Alignment {
    LawfulGood,
    LawfulNeutral,
    LawfulEvil,
    NeutralGood,
    Neutral,
    NeutralEvil,
    ChaoticGood,
    ChaoticNeutral,
    ChaoticEvil,
}

impl fmt::Display for Alignment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self, f.alternate()) {
            (Self::LawfulGood, false) => write!(f, "lawful good"),
            (Self::LawfulGood, true) => write!(f, "LG"),
            (Self::LawfulNeutral, false) => write!(f, "lawful neutral"),
            (Self::LawfulNeutral, true) => write!(f, "LN"),
            (Self::LawfulEvil, false) => write!(f, "lawful evil"),
            (Self::LawfulEvil, true) => write!(f, "LE"),
            (Self::NeutralGood, false) => write!(f, "neutral good"),
            (Self::NeutralGood, true) => write!(f, "NG"),
            (Self::Neutral, false) => write!(f, "true neutral"),
            (Self::Neutral, true) => write!(f, "N"),
            (Self::NeutralEvil, false) => write!(f, "neutral evil"),
            (Self::NeutralEvil, true) => write!(f, "NE"),
            (Self::ChaoticGood, false) => write!(f, "chaotic good"),
            (Self::ChaoticGood, true) => write!(f, "CG"),
            (Self::ChaoticNeutral, false) => write!(f, "chaotic neutral"),
            (Self::ChaoticNeutral, true) => write!(f, "CN"),
            (Self::ChaoticEvil, false) => write!(f, "chaotic evil"),
            (Self::ChaoticEvil, true) => write!(f, "CE"),
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Skill {
    Acrobatics,
    Arcana,
    Athletics,
    Crafting,
    Deception,
    Diplomacy,
    Intimidation,
    Lore(String),
    Medicine,
    Nature,
    Occultism,
    Performance,
    Religion,
    Society,
    Stealth,
    Survival,
    Thievery,
}

impl Skill {
    pub fn base_ability(&self) -> Ability {
        use Ability::*;

        match self {
            Self::Acrobatics => DEX,
            Self::Arcana => INT,
            Self::Athletics => STR,
            Self::Crafting => INT,
            Self::Deception => CHA,
            Self::Diplomacy => CHA,
            Self::Intimidation => CHA,
            Self::Lore(_) => INT,
            Self::Medicine => WIS,
            Self::Nature => WIS,
            Self::Occultism => INT,
            Self::Performance => CHA,
            Self::Religion => WIS,
            Self::Society => INT,
            Self::Stealth => DEX,
            Self::Survival => WIS,
            Self::Thievery => DEX,
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum Proficiency {
    Untrained,
    Trained,
    Expert,
    Master,
    Legendary,
}

impl Default for Proficiency {
    fn default() -> Self {
        Proficiency::Untrained
    }
}

impl Proficiency {
    #[inline]
    pub fn bonus(self, level: u8) -> Bonus {
        Bonus::proficiency(self, level)
    }
}

#[derive(Clone, Debug, Default)]
pub struct SkillProficiencies {
    acrobatics: Proficiency,
    arcana: Proficiency,
    athletics: Proficiency,
    crafting: Proficiency,
    deception: Proficiency,
    diplomacy: Proficiency,
    intimidation: Proficiency,
    lore: HashMap<String, Proficiency>,
    medicine: Proficiency,
    nature: Proficiency,
    occultism: Proficiency,
    performance: Proficiency,
    religion: Proficiency,
    society: Proficiency,
    stealth: Proficiency,
    survival: Proficiency,
    thievery: Proficiency,
}

// impl Default for SkillProficiencies {
//     fn default() -> Self {
//         Self {
//             acrobatics: Proficiency::default(),
//             arcana: Proficiency::default(),
//             athletics: Proficiency::default(),
//             crafting: Proficiency::default(),
//             deception: Proficiency::default(),
//             diplomacy: Proficiency::default(),
//             intimidation: Proficiency::default(),
//             lore: HashMap::new(),
//             medicine: Proficiency::default(),
//             nature: Proficiency::default(),
//             occultism: Proficiency::default(),
//             performance: Proficiency::default(),
//             religion: Proficiency::default(),
//             society: Proficiency::default(),
//             stealth: Proficiency::default(),
//             survival: Proficiency::default(),
//             thievery: Proficiency::default(),
//         }
//     }
// }

impl SkillProficiencies {
    fn proficiency(&self, skill: Skill) -> Proficiency {
        match skill {
            Skill::Acrobatics => self.acrobatics,
            Skill::Arcana => self.arcana,
            Skill::Athletics => self.athletics,
            Skill::Crafting => self.crafting,
            Skill::Deception => self.deception,
            Skill::Diplomacy => self.diplomacy,
            Skill::Intimidation => self.intimidation,
            Skill::Lore(topic) => self.lore.get(&topic).cloned().unwrap_or_default(),
            Skill::Medicine => self.medicine,
            Skill::Nature => self.nature,
            Skill::Occultism => self.occultism,
            Skill::Performance => self.performance,
            Skill::Religion => self.religion,
            Skill::Society => self.society,
            Skill::Stealth => self.stealth,
            Skill::Survival => self.survival,
            Skill::Thievery => self.thievery,
        }
    }

    fn fill_pdf<T: PDFOutput>(
        &self,
        ability_scores: &AbilityScores,
        level: u8,
        form: &mut T,
        item_bonus: Bonus,
        armor_penalty: Penalty,
    ) -> Result<()> {
        let lore_topics = {
            let mut topics: Vec<_> = self.lore.keys().collect();
            topics.sort();
            topics.truncate(2);
            topics
        };

        let mut skill_slots = vec![
            (SkillSlot::Acrobatics, Skill::Acrobatics),
            (SkillSlot::Arcana, Skill::Arcana),
            (SkillSlot::Athletics, Skill::Athletics),
            (SkillSlot::Crafting, Skill::Crafting),
            (SkillSlot::Deception, Skill::Deception),
            (SkillSlot::Diplomacy, Skill::Diplomacy),
            (SkillSlot::Intimidation, Skill::Intimidation),
            (SkillSlot::Medicine, Skill::Medicine),
            (SkillSlot::Nature, Skill::Nature),
            (SkillSlot::Occultism, Skill::Occultism),
            (SkillSlot::Performance, Skill::Performance),
            (SkillSlot::Religion, Skill::Religion),
            (SkillSlot::Society, Skill::Society),
            (SkillSlot::Stealth, Skill::Stealth),
            (SkillSlot::Survival, Skill::Survival),
            (SkillSlot::Thievery, Skill::Thievery),
        ];
        for (i, lore_topic) in lore_topics.into_iter().enumerate() {
            let pair = match i {
                0 => (SkillSlot::Lore1, Skill::Lore(lore_topic.clone())),
                1 => (SkillSlot::Lore2, Skill::Lore(lore_topic.clone())),
                _ => unreachable!(),
            };
            skill_slots.push(pair);
        }

        for (slot, skill) in skill_slots {
            let p = self.proficiency(skill.clone());
            let base_ability = skill.base_ability();
            let ability_bonus = ability_scores.bonus(base_ability);
            let prof_bonus = p.bonus(level);
            let mut modifier = Modifier::new(Modifies::Skill(skill.clone()))
                + ability_bonus
                + prof_bonus
                + item_bonus;
            if let Some(text_id) = TextID::armor_penalty_for_slot(slot) {
                modifier += armor_penalty;
                form.set_text(text_id, armor_penalty)?;
            }
            form.set_text(TextID::SkillBonusTotal(slot), &modifier)?;
            form.set_text(TextID::SkillAbilityBonus(slot), ability_bonus)?;
            form.set_text(TextID::SkillItemBonus(slot), modifier.item_part())?;
            form.set_proficiency(ProficiencyFields::Skill(slot), modifier)
                .map_err(Error::msg)?;

            match (slot, skill) {
                (SkillSlot::Lore1, Skill::Lore(topic)) => {
                    form.set_text(TextID::LoreSkillTopic1, &topic as &str)?
                }
                (SkillSlot::Lore2, Skill::Lore(topic)) => {
                    form.set_text(TextID::LoreSkillTopic2, &topic as &str)?
                }
                _ => (),
            }
        }

        Ok(())
    }
}

impl HasModifiers for SkillProficiencies {
    fn get_modifier(&self, c: &Character, m: Modifies) -> Modifier {
        match &m {
            Modifies::Skill(s) => {
                let p = self.proficiency(s.clone());
                Modifier::new(m) + p.bonus(c.level)
            }
            _ => Modifier::new(m),
        }
    }

    fn get_modified_skills(&self, _: &Character) -> HashSet<Skill> {
        let mut skills = HashSet::new();
        macro_rules! maybe_add_skills {
	    ($($field:expr => $skill:expr),+ $(,)*) => {
		$(if !matches!($field, Proficiency::Untrained) {
		    skills.insert($skill);
		})+
	    };
	}
        maybe_add_skills! {
                self.acrobatics => Skill::Acrobatics,
                self.arcana => Skill::Arcana,
                self.athletics => Skill::Athletics,
                self.crafting => Skill::Crafting,
                self.deception => Skill::Deception,
                self.diplomacy => Skill::Diplomacy,
                self.intimidation => Skill::Intimidation,
                self.medicine => Skill::Medicine,
                self.nature => Skill::Nature,
                self.occultism => Skill::Occultism,
                self.performance => Skill::Performance,
                self.religion => Skill::Religion,
                self.society => Skill::Society,
                self.stealth => Skill::Stealth,
                self.survival => Skill::Survival,
                self.thievery => Skill::Thievery,
        }
        for (lore_topic, p) in self.lore.iter() {
            maybe_add_skills! {
                p => Skill::Lore(lore_topic.clone()),
            }
        }

        skills
    }
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Size {
    Tiny,
    Small,
    Medium,
    Large,
}

#[derive(Clone, Debug)]
pub struct Ancestry {
    name: &'static str,
    size: Size,
    ability_boosts: Vec<Option<Ability>>,
    ability_flaws: Vec<Ability>,
    starting_languages: Vec<String>,
    flat_bonuses: HashMap<Modifies, Bonus>,
    flat_penalties: HashMap<Modifies, Penalty>,
    per_level_bonuses: HashMap<Modifies, Bonus>,
    per_level_penalties: HashMap<Modifies, Penalty>,
}

impl HasModifiers for Ancestry {
    fn get_modifier(&self, c: &Character, m: Modifies) -> Modifier {
        let mut bonus = Bonus::none();
        if let Some(flat) = self.flat_bonuses.get(&m) {
            bonus += *flat;
        }
        if let Some(per_level) = self.per_level_bonuses.get(&m) {
            bonus += *per_level * c.level;
        }

        let mut penalty = Penalty::none();
        if let Some(flat) = self.flat_penalties.get(&m) {
            penalty += *flat;
        }
        if let Some(per_level) = self.per_level_penalties.get(&m) {
            penalty += *per_level * c.level;
        }

        if let Modifies::Resistance(s) = m.clone() {
            match (s.as_ref(), self.name, c.level) {
                ("silver", "Werebear", 0) => (),
                ("silver", "Werebear", 1..=4) => {
                    penalty += Penalty::untyped(5);
                }
                ("silver", "Werebear", 5..=7) => {
                    penalty += Penalty::untyped(7);
                }
                ("silver", "Werebear", 8..=14) => {
                    penalty += Penalty::untyped(10);
                }
                ("silver", "Werebear", _) => {
                    penalty += Penalty::untyped(15);
                }
                _ => (),
            }
        }

        Modifier::new(m) + bonus + penalty
    }

    fn get_modified_resistances(&self, _c: &Character) -> HashSet<String> {
        match self.name {
            "Werebear" => vec!["silver".into()].into_iter().collect(),
            _ => HashSet::new(),
        }
    }
}

lazy_static! {
    pub static ref HUMAN: Ancestry = Ancestry {
        name: "Human",
        size: Size::Medium,
        ability_boosts: vec![None, None],
        ability_flaws: vec![],
        starting_languages: vec!["Common".into()],
        flat_bonuses: vec![
            (Modifies::HP, Bonus::untyped(8)),
            (Modifies::Speed, Bonus::untyped(25))
        ]
        .into_iter()
        .collect(),
        flat_penalties: HashMap::new(),
        per_level_bonuses: HashMap::new(),
        per_level_penalties: HashMap::new(),
    };
    pub static ref WEREBEAR: Ancestry = Ancestry {
        name: "Werebear",
        size: Size::Large,
        ability_boosts: vec![Some(Ability::WIS), Some(Ability::STR), Some(Ability::CON)],
        ability_flaws: vec![Ability::CHA],
        starting_languages: vec!["Common".into(), "bear empathy".into()],
        flat_bonuses: vec![
            (Modifies::HP, Bonus::untyped(5)),
            (Modifies::Speed, Bonus::untyped(25)),
            (Modifies::Attack, Bonus::untyped(1)),
            (Modifies::AC, Bonus::untyped(1)),
            (Modifies::FortitudeSave, Bonus::untyped(1)),
            (Modifies::ReflexSave, Bonus::untyped(1)),
            (Modifies::WillSave, Bonus::untyped(1)),
        ]
        .into_iter()
        .collect(),
        flat_penalties: HashMap::new(),
        per_level_bonuses: vec![(Modifies::HP, Bonus::untyped(5))]
            .into_iter()
            .collect(),
        per_level_penalties: HashMap::new(),
    };
}

#[derive(Copy, Clone, Debug)]
pub enum Background {
    Acolyte,
    Acrobat,
    AnimalWhisperer,
    Artisan,
    Artist,
    Barkeep,
    Barrister,
    BountyHunter,
    Charlatan,
    Criminal,
    Detective,
    Emissary,
    Entertainer,
    Farmhand,
    FieldMedic,
    FortuneTeller,
    Gambler,
    Gladiator,
    Guard,
    Herbalist,
    Hermit,
    Hunter,
    Laborer,
    MartialDisciple,
    Merchant,
    Miner,
    Noble,
    Nomad,
    Prisoner,
    Sailor,
    Scholar,
    Scout,
    StreetUrchin,
    Tinker,
    Warrior,
}

impl fmt::Display for Background {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Acolyte => write!(f, "Acolyte"),
            Self::Acrobat => write!(f, "Acrobat"),
            Self::AnimalWhisperer => write!(f, "Animal Whisperer"),
            Self::Artisan => write!(f, "Artisan"),
            Self::Artist => write!(f, "Artist"),
            Self::Barkeep => write!(f, "Barkeep"),
            Self::Barrister => write!(f, "Barrister"),
            Self::BountyHunter => write!(f, "Bounty Hunter"),
            Self::Charlatan => write!(f, "Charlatan"),
            Self::Criminal => write!(f, "Criminal"),
            Self::Detective => write!(f, "Detective"),
            Self::Emissary => write!(f, "Emissary"),
            Self::Entertainer => write!(f, "Entertainer"),
            Self::Farmhand => write!(f, "Farmhand"),
            Self::FieldMedic => write!(f, "Field Medic"),
            Self::FortuneTeller => write!(f, "Fortune Teller"),
            Self::Gambler => write!(f, "Gambler"),
            Self::Gladiator => write!(f, "Gladiator"),
            Self::Guard => write!(f, "Guard"),
            Self::Herbalist => write!(f, "Herbalist"),
            Self::Hermit => write!(f, "Hermit"),
            Self::Hunter => write!(f, "Hunter"),
            Self::Laborer => write!(f, "Laborer"),
            Self::MartialDisciple => write!(f, "Martial Disciple"),
            Self::Merchant => write!(f, "Merchant"),
            Self::Miner => write!(f, "Miner"),
            Self::Noble => write!(f, "Noble"),
            Self::Nomad => write!(f, "Nomad"),
            Self::Prisoner => write!(f, "Prisoner"),
            Self::Sailor => write!(f, "Sailor"),
            Self::Scholar => write!(f, "Scholar"),
            Self::Scout => write!(f, "Scout"),
            Self::StreetUrchin => write!(f, "Street Urchin"),
            Self::Tinker => write!(f, "Tinker"),
            Self::Warrior => write!(f, "Warrior"),
        }
    }
}

impl HasModifiers for Background {
    fn get_modifier(&self, _c: &Character, m: Modifies) -> Modifier {
        let bonus = match (self, &m) {
            // TODO
            (_, _) => Bonus::none(),
        };
        Modifier::new(m) + bonus
    }
}

#[derive(Clone, Debug)]
pub struct Class {
    name: &'static str,
    key_ability: Vec<Ability>,
    hp_per_level: u8,
    perception: Proficiency,
    fort_save: Proficiency,
    reflex_save: Proficiency,
    will_save: Proficiency,
    trained_skill_options: Vec<Option<&'static str>>,
    free_skill_trained: u8,
    weapon_proficiencies: HashMap<WeaponCategory, Proficiency>,
    armor_proficiencies: HashMap<ArmorCategory, Proficiency>,
}

impl fmt::Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.name.fmt(f)
    }
}

impl HasModifiers for Class {
    fn get_modifier(&self, c: &Character, m: Modifies) -> Modifier {
        let bonus = match m {
            Modifies::ClassDC => {
                let p = match c.level {
                    1..=10 => Proficiency::Trained,
                    11..=18 => Proficiency::Expert,
                    19..=20 => Proficiency::Master,
                    _ => panic!("Level should be between 1 and 20, found {}", c.level),
                };
                p.bonus(c.level)
            }
            Modifies::FortitudeSave => self.fort_save.bonus(c.level),
            Modifies::HP => {
                let con_mod: i16 = c.ability_scores.bonus(Ability::CON).total();
                let per_level = self.hp_per_level as i16 + con_mod;
                let level = c.level as i16;
                Bonus::untyped(level * per_level)
            }
            Modifies::Perception => self.perception.bonus(c.level),
            Modifies::ReflexSave => self.reflex_save.bonus(c.level),
            Modifies::WillSave => self.will_save.bonus(c.level),
            Modifies::Speed => {
                // should handle Monk speed boost here
                Bonus::none()
            }
            _ => Bonus::none(),
        };
        Modifier::new(m) + bonus
    }
}

lazy_static! {
    pub static ref FIGHTER: Class = Class {
        name: "Fighter",
        key_ability: vec![Ability::STR, Ability::DEX],
        hp_per_level: 10,
        perception: Proficiency::Expert,
        fort_save: Proficiency::Expert,
        reflex_save: Proficiency::Expert,
        will_save: Proficiency::Trained,
        trained_skill_options: vec![Some("Acrobatics"), Some("Athletics")],
        free_skill_trained: 3,
        weapon_proficiencies: [
            (WeaponCategory::Simple, Proficiency::Expert),
            (WeaponCategory::Martial, Proficiency::Expert),
            (WeaponCategory::Advanced, Proficiency::Trained),
            (WeaponCategory::Unarmed, Proficiency::Expert),
        ]
        .iter()
        .cloned()
        .collect(),
        armor_proficiencies: [
            (ArmorCategory::Light, Proficiency::Trained),
            (ArmorCategory::Medium, Proficiency::Trained),
            (ArmorCategory::Heavy, Proficiency::Trained),
            (ArmorCategory::Unarmored, Proficiency::Trained),
        ]
        .iter()
        .cloned()
        .collect(),
    };
    pub static ref CHAMPION: Class = Class {
        name: "Champion",
        key_ability: vec![Ability::STR, Ability::DEX],
        hp_per_level: 10,
        perception: Proficiency::Trained,
        fort_save: Proficiency::Expert,
        reflex_save: Proficiency::Trained,
        will_save: Proficiency::Expert,
        trained_skill_options: vec![Some("Religion"), None],
        free_skill_trained: 3,
        weapon_proficiencies: [
            (WeaponCategory::Simple, Proficiency::Trained),
            (WeaponCategory::Martial, Proficiency::Trained),
            (WeaponCategory::Unarmed, Proficiency::Trained),
        ]
        .iter()
        .cloned()
        .collect(),
        armor_proficiencies: [
            (ArmorCategory::Light, Proficiency::Trained),
            (ArmorCategory::Medium, Proficiency::Trained),
            (ArmorCategory::Heavy, Proficiency::Trained),
            (ArmorCategory::Unarmored, Proficiency::Trained),
        ]
        .iter()
        .cloned()
        .collect(),
    };
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum Ability {
    STR,
    DEX,
    CON,
    INT,
    WIS,
    CHA,
}

#[derive(Copy, Clone, Debug)]
pub struct AbilityScores {
    str: u8,
    dex: u8,
    con: u8,
    int: u8,
    wis: u8,
    cha: u8,
}

impl Default for AbilityScores {
    fn default() -> Self {
        Self {
            str: 10,
            dex: 10,
            con: 10,
            int: 10,
            wis: 10,
            cha: 10,
        }
    }
}

impl AbilityScores {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn score(&self, ability: Ability) -> u8 {
        use Ability::*;

        match ability {
            STR => self.str,
            DEX => self.dex,
            CON => self.con,
            INT => self.int,
            WIS => self.wis,
            CHA => self.cha,
        }
    }

    pub fn bonus(&self, ability: Ability) -> Bonus {
        let score = self.score(ability) as i16;
        Bonus::untyped((score - 10) / 2)
    }

    pub fn boost(mut self, ability: Ability) -> Self {
        let score: &mut _ = match ability {
            Ability::STR => &mut self.str,
            Ability::DEX => &mut self.dex,
            Ability::CON => &mut self.con,
            Ability::INT => &mut self.int,
            Ability::WIS => &mut self.wis,
            Ability::CHA => &mut self.cha,
        };
        if *score < 18 {
            *score += 2;
        } else {
            *score += 1;
        }
        self
    }

    pub fn flaw(mut self, ability: Ability) -> Self {
        let score: &mut _ = match ability {
            Ability::STR => &mut self.str,
            Ability::DEX => &mut self.dex,
            Ability::CON => &mut self.con,
            Ability::INT => &mut self.int,
            Ability::WIS => &mut self.wis,
            Ability::CHA => &mut self.cha,
        };
        *score -= 2;
        self
    }

    pub fn from_boosts(
        boosts: impl IntoIterator<Item = Ability>,
        flaws: impl IntoIterator<Item = Ability>,
    ) -> Self {
        let mut scores = Self::default();
        for ability in boosts {
            scores = scores.boost(ability);
        }
        for ability in flaws {
            scores = scores.flaw(ability);
        }
        scores
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Gold {
    total_copper: u32,
}

impl Gold {
    pub fn zero() -> Self {
        Self { total_copper: 0 }
    }

    pub fn cp(n: u8) -> Self {
        Self {
            total_copper: n as u32,
        }
    }

    pub fn sp(n: u8) -> Self {
        Self {
            total_copper: (n as u32) * 100,
        }
    }

    pub fn gp(n: u8) -> Self {
        Self {
            total_copper: (n as u32) * 10_000,
        }
    }

    #[inline]
    pub fn copper_part(&self) -> u8 {
        (self.total_copper % 100) as u8
    }

    #[inline]
    pub fn silver_part(&self) -> u8 {
        ((self.total_copper / 100) % 100) as u8
    }

    #[inline]
    pub fn gold_part(&self) -> u16 {
        (self.total_copper / 10_000) as u16
    }
}

impl fmt::Display for Gold {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let gold = self.gold_part();
        let copper = self.copper_part();
        let silver = self.silver_part();

        match (gold, silver, copper) {
            (0, 0, 0) => write!(f, "-"),
            (0, 0, c) => write!(f, "{} cp", c),
            (0, s, 0) => write!(f, "{} sp", s),
            (0, s, c) => write!(f, "{:.2} sp", s as f32 + ((c as f32) / 100.0)),
            (g, 0, 0) => write!(f, "{} gp", g),
            (g, 0, c) => write!(f, "{:.4} gp", g as f32 + ((c as f32) / 10_000.0)),
            (g, s, 0) => write!(f, "{:.2} gp", g as f32 + ((s as f32) / 100.0)),
            (g, s, c) => write!(
                f,
                "{:.4} gp",
                g as f32 + ((s as f32) / 100.0) + ((c as f32) / 10_000.0)
            ),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Bulk {
    Light,
    Heavy(u16),
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum ArmorCategory {
    Unarmored,
    Light,
    Medium,
    Heavy,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum WeaponCategory {
    Unarmed,
    Simple,
    Martial,
    Advanced,
    Other(String),
}

#[derive(Clone, Debug)]
pub enum WeaponGroup {
    Axe,
    Bomb,
    Bow,
    Brawling,
    Club,
    Dart,
    Flail,
    Hammer,
    Knife,
    Natural,
    Pick,
    Polearm,
    Shield,
    Sling,
    Spear,
    Sword,
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct Range(pub u16);

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ft.", self.0)
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum DamageType {
    B,
    P,
    S,
}

impl fmt::Display for DamageType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (self, f.alternate()) {
            (Self::B, false) => write!(f, "bludgeoning"),
            (Self::B, true) => write!(f, "B"),
            (Self::P, false) => write!(f, "piercing"),
            (Self::P, true) => write!(f, "P"),
            (Self::S, false) => write!(f, "slashing"),
            (Self::S, true) => write!(f, "S"),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum WeaponTrait {
    Agile,
    Attached,
    Backstabber,
    Backswing,
    Deadly(WeaponDie),
    Disarm,
    Dwarf,
    Elf,
    Fatal(WeaponDie),
    Finesse,
    Forceful,
    FreeHand,
    Gnome,
    Goblin,
    Grapple,
    Halfling,
    Jousting,
    Monk,
    Nonleathal,
    Orc,
    Parry,
    Propulsive,
    Reach,
    Shove,
    Sweep,
    Thrown,
    Trip,
    Twin,
    TwoHand(WeaponDie),
    Unarmed,
    Versatile(DamageType),
    Volley(Range),
}

impl fmt::Display for WeaponTrait {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Agile => write!(f, "agile"),
            Self::Attached => write!(f, "attached"),
            Self::Backstabber => write!(f, "backstabber"),
            Self::Backswing => write!(f, "backswing"),
            Self::Deadly(die) => write!(f, "deadly {}", die),
            Self::Disarm => write!(f, "disarm"),
            Self::Dwarf => write!(f, "dwarf"),
            Self::Elf => write!(f, "elf"),
            Self::Fatal(die) => write!(f, "fatal {}", die),
            Self::Finesse => write!(f, "finesse"),
            Self::Forceful => write!(f, "forceful"),
            Self::FreeHand => write!(f, "free-hand"),
            Self::Gnome => write!(f, "gnome"),
            Self::Goblin => write!(f, "goblin"),
            Self::Grapple => write!(f, "grapple"),
            Self::Halfling => write!(f, "halfling"),
            Self::Jousting => write!(f, "jousting"),
            Self::Monk => write!(f, "monk"),
            Self::Nonleathal => write!(f, "nonleathal"),
            Self::Orc => write!(f, "orc"),
            Self::Parry => write!(f, "parry"),
            Self::Propulsive => write!(f, "propulsive"),
            Self::Reach => write!(f, "reach"),
            Self::Shove => write!(f, "shove"),
            Self::Sweep => write!(f, "sweep"),
            Self::Thrown => write!(f, "thrown"),
            Self::Trip => write!(f, "trip"),
            Self::Twin => write!(f, "twin"),
            Self::TwoHand(die) => write!(f, "two-hand {}", die),
            Self::Unarmed => write!(f, "unarmed"),
            Self::Versatile(dt) => write!(f, "versatile {}", dt),
            Self::Volley(range) => write!(f, "volley {:#}", range),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum WeaponDie {
    D4,
    D6,
    D8,
    D10,
    D12,
}

impl fmt::Display for WeaponDie {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::D4 => write!(f, "d4"),
            Self::D6 => write!(f, "d6"),
            Self::D8 => write!(f, "d8"),
            Self::D10 => write!(f, "d10"),
            Self::D12 => write!(f, "d12"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ItemType {
    Armor {
        dex_cap: Option<u8>,
        check_penalty: Option<Penalty>,
        min_strength: u8,
    },
    Shield {
        hp: u16,
        hardness: u16,
    },
    Weapon {
        range: Option<u16>,
        hands: usize,
        damage_die: WeaponDie,
        damage_type: DamageType,
        category: WeaponCategory,
        group: WeaponGroup,
        traits: Vec<WeaponTrait>,
    },
}

#[derive(Clone, Debug)]
pub struct Item {
    name: String,
    level: u8,
    bulk: Option<Bulk>,
    price: Option<Gold>,
    item_type: ItemType,
    bonuses: HashMap<Modifies, Bonus>,
    penalties: HashMap<Modifies, Penalty>,
    traits: HashSet<String>,
}

impl HasModifiers for Item {
    fn get_modifier(&self, _c: &Character, m: Modifies) -> Modifier {
        let bonus = self.bonuses.get(&m).cloned().unwrap_or_default();
        let penalty = self.penalties.get(&m).cloned().unwrap_or_default();
        // This is a decent enough place to hook modifiers
        Modifier::new(m) + bonus + penalty
    }
}

#[derive(Clone, Debug)]
pub struct Character {
    name: String,
    player_name: String,
    ancestry: &'static Ancestry,
    background: Background,
    class: &'static Class,
    class_key_ability: Ability,
    alignment: Alignment,
    deity: String,
    traits: Vec<String>,
    xp: usize,
    level: u8,
    ability_scores: AbilityScores,
    skill_proficiencies: SkillProficiencies,
    items: Vec<Item>,
}

impl Character {
    pub fn save_character_sheet<T, F>(&self, filename: F) -> Result<()>
    where
        T: PDFOutput,
        F: AsRef<std::path::Path>,
    {
        use Ability::*;

        let mut pdf = T::load_empty()?;

        pdf.set_text(TextID::CharacterName, &self.name as &str)?;
        pdf.set_text(TextID::PlayerName, &self.player_name as &str)?;
        pdf.set_text(TextID::Class, format_args!("{} {}", self.class, self.level))?;
        pdf.set_text(TextID::Size, "M")?;
        pdf.set_text(TextID::Alignment, self.alignment)?;
        pdf.set_text(TextID::CharacterTraits, "These are some character traits")?;
        pdf.set_text(TextID::XP, self.xp)?;
        pdf.set_text(TextID::Deity, &self.deity as &str)?;
        pdf.set_text(TextID::CharacterLevel, self.level)?;
        pdf.set_text(TextID::HeroPoints, "?")?;
        pdf.set_text(TextID::AncestryAndHeritage, self.ancestry.name)?;
        pdf.set_text(TextID::Background, self.background)?;
        pdf.set_text(TextID::Speed, self.get_modifier(Modifies::Speed).as_score())?;
        pdf.set_text(TextID::MovementNotes, "Some movement notes here")?;

        // Stats
        pdf.set_text(TextID::STRScore, self.ability_scores.score(STR))?;
        pdf.set_text(TextID::DEXScore, self.ability_scores.score(DEX))?;
        pdf.set_text(TextID::CONScore, self.ability_scores.score(CON))?;
        pdf.set_text(TextID::INTScore, self.ability_scores.score(INT))?;
        pdf.set_text(TextID::WISScore, self.ability_scores.score(WIS))?;
        pdf.set_text(TextID::CHAScore, self.ability_scores.score(CHA))?;

        pdf.set_text(
            TextID::STRMod,
            format!("{:+}", self.ability_scores.bonus(STR)),
        )?;
        pdf.set_text(
            TextID::DEXMod,
            format!("{:+}", self.ability_scores.bonus(DEX)),
        )?;
        pdf.set_text(
            TextID::CONMod,
            format!("{:+}", self.ability_scores.bonus(CON)),
        )?;
        pdf.set_text(
            TextID::INTMod,
            format!("{:+}", self.ability_scores.bonus(INT)),
        )?;
        pdf.set_text(
            TextID::WISMod,
            format!("{:+}", self.ability_scores.bonus(WIS)),
        )?;
        pdf.set_text(
            TextID::CHAMod,
            format!("{:+}", self.ability_scores.bonus(CHA)),
        )?;

        // Class DC
        {
            let class_dc = self.get_modifier(Modifies::ClassDC) + Bonus::untyped(10);
            let ability_bonus = self.ability_scores.bonus(self.class_key_ability);
            pdf.set_text(TextID::ClassDC, class_dc.clone().as_score())?;
            pdf.set_text(TextID::ClassDCKeyAbilityBonus, ability_bonus)?;
            pdf.set_text(TextID::ClassDCItemBonus, class_dc.item_part())?;
            pdf.set_proficiency(ProficiencyFields::ClassDC, class_dc)?;
        }

        // Saves
        {
            let fort_save = self.get_modifier(Modifies::FortitudeSave);
            let ref_save = self.get_modifier(Modifies::ReflexSave);
            let will_save = self.get_modifier(Modifies::WillSave);

            pdf.set_text(TextID::FortSaveTotal, &fort_save)?;
            pdf.set_text(TextID::ReflexSaveTotal, &ref_save)?;
            pdf.set_text(TextID::WillSaveTotal, &will_save)?;

            pdf.set_text(TextID::FortSaveCONBonus, self.ability_scores.bonus(CON))?;
            pdf.set_text(TextID::ReflexSaveDEXBonus, self.ability_scores.bonus(DEX))?;
            pdf.set_text(TextID::WillSaveWISBonus, self.ability_scores.bonus(CON))?;

            pdf.set_text(TextID::FortSaveItemBonus, fort_save.item_part())?;
            pdf.set_text(TextID::ReflexSaveItemBonus, ref_save.item_part())?;
            pdf.set_text(TextID::WillSaveItemBonus, will_save.item_part())?;

            pdf.set_proficiency(ProficiencyFields::FortSave, fort_save)?;
            pdf.set_proficiency(ProficiencyFields::ReflexSave, ref_save)?;
            pdf.set_proficiency(ProficiencyFields::WillSave, will_save)?;

            pdf.set_text(TextID::SavingThrowNotes, "Some notes on her saving throws.")?;
        }

        // Max HP
        {
            let total = self.get_modifier(Modifies::HP);
            pdf.set_text(TextID::MaxHP, total.as_score())?;
            let mut formatted = vec![];
            for r in self.get_modified_resistances() {
                let m = self.get_modifier(Modifies::Resistance(r.clone()));
                let total = m.total();
                use std::cmp::Ordering::*;
                match total.cmp(&0) {
                    Less => formatted.push(format!("weakness ({}) {}", &r as &str, -total)),
                    Equal => (),
                    Greater => formatted.push(format!("resistance ({}) {}", &r as &str, -total)),
                }
            }
            let formatted = formatted.join(", ");
            pdf.set_text(TextID::ResistancesAndImmunities, formatted)?;
        }

        // AC
        {
            let (armor_bonus, armor_dex_cap): (Bonus, Option<i16>) = self
                .items
                .iter()
                .filter_map(|item| match &item.item_type {
                    ItemType::Armor { dex_cap, .. } => {
                        let ac_bonus = *item.bonuses.get(&Modifies::AC).unwrap_or(&Bonus::item(0));
                        Some((ac_bonus, dex_cap.map(|x| x as i16)))
                    }
                    _ => None,
                })
                .next()
                .unwrap_or((Bonus::item(0), None));
            // TODO
            let armor_prof = Proficiency::Trained;
            let prof_bonus = armor_prof.bonus(self.level);
            let before_dex =
                Modifier::new(Modifies::AC) + Bonus::untyped(10) + prof_bonus + armor_bonus;

            let dex_bonus = self.ability_scores.bonus(DEX);
            let dex_bonus_total = dex_bonus.total();
            let with_dex = before_dex.clone() + dex_bonus;
            let mut total = with_dex;

            match armor_dex_cap {
                Some(cap) if cap < dex_bonus_total => {
                    total = before_dex + Bonus::untyped(cap);
                    pdf.set_text(TextID::ACDexCap, cap)?;
                }
                _ => (),
            }
            pdf.set_text(TextID::TotalAC, total.as_score())?;
            pdf.set_text(TextID::ACDexBonus, dex_bonus)?;
            pdf.set_text(TextID::ACItemBonus, armor_bonus)?;
            pdf.set_proficiency(ProficiencyFields::ACArmor, total)?;

            // TODO
            for (category, fieldset) in [
                (ArmorCategory::Unarmored, ProficiencyFields::Unarmored),
                (ArmorCategory::Light, ProficiencyFields::LightArmor),
                (ArmorCategory::Medium, ProficiencyFields::MediumArmor),
                (ArmorCategory::Heavy, ProficiencyFields::HeavyArmor),
            ]
            .iter()
            {
                let m = self.get_modifier(Modifies::ArmorCategory(*category));
                pdf.set_proficiency(fieldset.clone(), m)?;
            }
        }

        // Shield
        {
            let shield_info = self
                .items
                .iter()
                .filter_map(|item| match &item.item_type {
                    ItemType::Shield { hp, hardness } => {
                        let ac_bonus = *item
                            .bonuses
                            .get(&Modifies::AC)
                            .unwrap_or(&Bonus::untyped(0));
                        Some((ac_bonus, hp, hardness))
                    }
                    _ => None,
                })
                .next();
            if let Some((ac_bonus, hp, hardness)) = shield_info {
                pdf.set_text(TextID::ShieldAC, ac_bonus)?;
                pdf.set_text(TextID::ShieldHardness, hardness)?;
                pdf.set_text(TextID::ShieldMaxHP, hp)?;
                pdf.set_text(TextID::ShieldBreakThreshold, hp / 2)?;
            }
        }

        // Perception
        {
            let wis_bonus = self.ability_scores.bonus(WIS);
            let prof = self.class.perception;
            let item_bonus = Bonus::item(0); // TODO
            let modifier = Modifier::new(Modifies::Perception)
                + wis_bonus
                + prof.bonus(self.level)
                + item_bonus;

            pdf.set_text(TextID::PerceptionBonus, modifier.clone())?;
            pdf.set_text(TextID::PerceptionWISBonus, wis_bonus)?;
            pdf.set_proficiency(ProficiencyFields::Perception, modifier)?;
            pdf.set_text(TextID::PerceptionItemBonus, item_bonus)?;
            pdf.set_text(TextID::PerceptionSenses, "TODO senses")?;
        }

        // Weapons
        {
            let mut melee_slots = vec![WeaponSlot::Melee3, WeaponSlot::Melee2, WeaponSlot::Melee1];
            let mut ranged_slots = vec![
                WeaponSlot::Ranged3,
                WeaponSlot::Ranged2,
                WeaponSlot::Ranged1,
            ];

            for item in self.items.iter() {
                if let ItemType::Weapon {
                    range,
                    hands: _,
                    damage_die,
                    damage_type,
                    category,
                    group: _,
                    traits,
                } = &item.item_type
                {
                    let slot = match &range {
                        Some(_) => match ranged_slots.pop() {
                            Some(s) => s,
                            None => {
                                eprintln!(
                                    "Too many ranged weapons, not rendering weapon {:?}",
                                    item.name
                                );
                                continue;
                            }
                        },
                        None => match melee_slots.pop() {
                            Some(s) => s,
                            None => {
                                eprintln!(
                                    "Too many melee weapons, not rendering weapon {:?}",
                                    item.name
                                );
                                continue;
                            }
                        },
                    };
                    let str_bonus = self.ability_scores.bonus(STR);
                    let dex_bonus = self.ability_scores.bonus(DEX);
                    let is_finesse = traits.iter().any(|t| matches!(t, WeaponTrait::Finesse));
                    let ability_bonus =
                        match (range, is_finesse, dex_bonus.total() > str_bonus.total()) {
                            (Some(_), _, _) => dex_bonus.clone(),
                            (None, true, true) => dex_bonus.clone(),
                            (None, _, _) => str_bonus.clone(),
                        };
                    let prof = self
                        .class
                        .weapon_proficiencies
                        .get(category)
                        .copied()
                        .unwrap_or_default();
                    let item_bonus = item
                        .bonuses
                        .get(&Modifies::Attack)
                        .copied()
                        .unwrap_or_default();
                    let attack_mod = Modifier::new(Modifies::Attack)
                        + ability_bonus
                        + prof.bonus(self.level)
                        + item_bonus;

                    // attack
                    pdf.set_text(TextID::WeaponName(slot), &item.name as &str)?;
                    pdf.set_text(TextID::WeaponAttackBonus(slot), attack_mod)?;
                    pdf.set_text(TextID::WeaponAttackAbilityBonus(slot), ability_bonus)?;
                    pdf.set_proficiency(
                        ProficiencyFields::Weapon(slot),
                        Modifier::new(Modifies::WeaponCategory(category.clone()))
                            + prof.bonus(self.level),
                    )?;
                    pdf.set_text(TextID::WeaponAttackItemBonus(slot), item_bonus)?;

                    // damage
                    pdf.set_text(TextID::WeaponDamageDice(slot), damage_die)?;
                    if range.is_none() {
                        pdf.set_text(TextID::WeaponDamageAbilityBonus(slot), str_bonus)?;
                    }
                    let (mut b, mut p, mut s) = match damage_type {
                        DamageType::B => (true, false, false),
                        DamageType::P => (false, true, false),
                        DamageType::S => (false, false, true),
                    };
                    for t in traits.iter() {
                        match t {
                            WeaponTrait::Versatile(DamageType::B) => b = true,
                            WeaponTrait::Versatile(DamageType::P) => p = true,
                            WeaponTrait::Versatile(DamageType::S) => s = true,
                            _ => (),
                        }
                    }
                    pdf.set_check_box(CheckboxID::WeaponDamageType(DamageType::B, slot), b)?;
                    pdf.set_check_box(CheckboxID::WeaponDamageType(DamageType::P, slot), p)?;
                    pdf.set_check_box(CheckboxID::WeaponDamageType(DamageType::S, slot), s)?;
                    pdf.set_text(TextID::WeaponDamageSpecial(slot), "?")?;
                    pdf.set_text(TextID::WeaponDamageOther(slot), "TODO")?;
                    {
                        let mut traits_strings = Vec::with_capacity(traits.len());
                        for t in traits {
                            traits_strings.push(format!("{}", t));
                        }
                        pdf.set_text(TextID::WeaponTraits(slot), traits_strings.join(", "))?;
                    }
                }
            }
            for category in [WeaponCategory::Simple, WeaponCategory::Martial].iter() {
                let p = self
                    .class
                    .weapon_proficiencies
                    .get(category)
                    .cloned()
                    .unwrap_or_default();
                pdf.set_proficiency(
                    ProficiencyFields::WeaponCategory(category.clone()),
                    Modifier::new(Modifies::WeaponCategory(category.clone())) + p.bonus(self.level),
                )?;
            }
        }

        self.skill_proficiencies.fill_pdf(
            &self.ability_scores,
            self.level,
            &mut pdf,
            Bonus::item(0),
            Penalty::item(0),
        )?;

        pdf.save(filename)?;

        Ok(())
    }

    fn get_modifier(&self, m: Modifies) -> Modifier {
        let mut total = Modifier::new(m.clone())
            + self.ancestry.get_modifier(self, m.clone())
            + self.background.get_modifier(self, m.clone())
            + self.class.get_modifier(self, m.clone());
        for item in self.items.iter() {
            total += item.get_modifier(self, m.clone());
        }

        match m {
            Modifies::ClassDC => {
                total += self.ability_scores.bonus(self.class_key_ability);
            }
            Modifies::FortitudeSave => {
                total += self.ability_scores.bonus(Ability::CON);
            }
            Modifies::ReflexSave => {
                total += self.ability_scores.bonus(Ability::DEX);
            }
            Modifies::WillSave => {
                total += self.ability_scores.bonus(Ability::WIS);
            }
            _ => (),
        }

        total
    }

    fn get_modified_resistances(&self) -> HashSet<String> {
        let mut resistances = HashSet::new();
        resistances.extend(self.ancestry.get_modified_resistances(self));
        resistances.extend(self.background.get_modified_resistances(self));
        resistances.extend(self.class.get_modified_resistances(self));

        for item in self.items.iter() {
            resistances.extend(item.get_modified_resistances(self));
        }

        resistances
    }
}

fn main() -> Result<()> {
    let character = Character {
        name: "Nadia Redmane".into(),
        player_name: "Cassie".into(),
        ancestry: &WEREBEAR,
        background: Background::Nomad,
        class: &CHAMPION,
        class_key_ability: Ability::STR,
        alignment: Alignment::LawfulGood,
        deity: "Erastil".into(),
        traits: vec!["Human".into(), "Humanoid".into()],
        xp: 0,
        level: 1,
        ability_scores: AbilityScores::new()
            // Werebear boosts STR, WIS, and CON, and gives CHA a flaw
            .boost(Ability::STR)
            .boost(Ability::WIS)
            .boost(Ability::CON)
            .flaw(Ability::CHA)
            // Nomad (background) boosts either CON or WIS, and gives one free.
            .boost(Ability::CON)
            .boost(Ability::DEX)
            // Champion (class) boosts key ability score (STR)
            .boost(Ability::STR)
            // At level 1, gain four free boosts
            .boost(Ability::STR)
            .boost(Ability::DEX)
            .boost(Ability::CON)
            .boost(Ability::CHA),
        skill_proficiencies: SkillProficiencies {
            diplomacy: Proficiency::Trained,
            nature: Proficiency::Trained,
            religion: Proficiency::Trained,
            survival: Proficiency::Trained,
            lore: vec![("Forest".into(), Proficiency::Trained)]
                .into_iter()
                .collect(),
            ..SkillProficiencies::default()
        },
        items: vec![
            Item {
                name: "Studded Leather".into(),
                level: 0,
                bulk: Some(Bulk::Heavy(1)),
                price: Some(Gold::gp(3)),
                bonuses: vec![(Modifies::AC, Bonus::item(2))].into_iter().collect(),
                penalties: HashMap::new(),
                item_type: ItemType::Armor {
                    dex_cap: Some(3),
                    check_penalty: Some(Penalty::item(1)),
                    min_strength: 12,
                },
                traits: vec![
                    "chain armor".into(),
                    "flexible armor".into(),
                    "noisy armor".into(),
                ]
                .into_iter()
                .collect(),
            },
            Item {
                name: "Claws".into(),
                level: 0,
                bulk: None,
                price: None,
                bonuses: HashMap::new(),
                penalties: HashMap::new(),
                item_type: ItemType::Weapon {
                    range: None,
                    hands: 1,
                    damage_die: WeaponDie::D8,
                    damage_type: DamageType::S,
                    category: WeaponCategory::Simple,
                    group: WeaponGroup::Natural,
                    traits: vec![WeaponTrait::Agile],
                },
                traits: HashSet::new(),
            },
            Item {
                name: "Longbow".into(),
                level: 0,
                bulk: Some(Bulk::Heavy(2)),
                price: Some(Gold::gp(6)),
                bonuses: HashMap::new(),
                penalties: HashMap::new(),
                item_type: ItemType::Weapon {
                    range: Some(100),
                    hands: 1,
                    damage_die: WeaponDie::D8,
                    damage_type: DamageType::P,
                    category: WeaponCategory::Martial,
                    group: WeaponGroup::Bow,
                    traits: vec![
                        WeaponTrait::Deadly(WeaponDie::D8),
                        WeaponTrait::Volley(Range(30)),
                    ],
                },
                traits: HashSet::new(),
            },
        ],
    };

    let output_filename = "Nadia Redmane - 01.pdf";
    // {
    //     println!("Writing test string");
    //     let mut pdf = raw_pdf_manip::PDF::load_empty()?;
    //     pdf.set_text(TextID::CharacterName, "This is a test");
    //     pdf.save(output_filename)?;
    //     println!("Saved output PDF");
    // }
    println!("Saving character sheet to {:?}...", output_filename);
    let start = std::time::Instant::now();
    character.save_character_sheet::<raw_pdf_manip::PDF, _>(output_filename)?;
    let end = std::time::Instant::now();
    let dt = end - start;
    println!("Successfully saved character sheet in {:?}", dt);

    Ok(())
}
