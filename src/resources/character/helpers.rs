use anyhow::{Error, Result};
use serde::Deserialize;
use smartstring::alias::String;
use std::collections::{HashMap, HashSet};

use super::Character;
use crate::{
    bonuses::{Bonus, HasModifiers, Modifier, Modifies, Penalty},
    pdf::{PDFOutput, ProficiencyFields, SkillSlot, TextID},
    stats::{Ability, Level, Proficiency, Skill},
};

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

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(try_from = "HashMap<Skill, Proficiency>")]
pub struct SkillProficiencies {
    #[serde(default)]
    acrobatics: Proficiency,
    #[serde(default)]
    arcana: Proficiency,
    #[serde(default)]
    athletics: Proficiency,
    #[serde(default)]
    crafting: Proficiency,
    #[serde(default)]
    deception: Proficiency,
    #[serde(default)]
    diplomacy: Proficiency,
    #[serde(default)]
    intimidation: Proficiency,
    lore: HashMap<String, Proficiency>,
    #[serde(default)]
    medicine: Proficiency,
    #[serde(default)]
    nature: Proficiency,
    #[serde(default)]
    occultism: Proficiency,
    #[serde(default)]
    performance: Proficiency,
    #[serde(default)]
    religion: Proficiency,
    #[serde(default)]
    society: Proficiency,
    #[serde(default)]
    stealth: Proficiency,
    #[serde(default)]
    survival: Proficiency,
    #[serde(default)]
    thievery: Proficiency,
}

impl From<HashMap<Skill, Proficiency>> for SkillProficiencies {
    fn from(mapping: HashMap<Skill, Proficiency>) -> Self {
        let mut skills = Self::default();

        macro_rules! add_skill {
            ($p:ident, lore, $topic:ident) => {{
                skills.lore.insert($topic, $p);
            }};
            ($p:ident, $attr:ident) => {{
                skills.$attr = $p;
            }};
        }

        for (skill, p) in mapping {
            match skill {
                Skill::Acrobatics => add_skill!(p, acrobatics),
                Skill::Arcana => add_skill!(p, arcana),
                Skill::Athletics => add_skill!(p, athletics),
                Skill::Crafting => add_skill!(p, crafting),
                Skill::Deception => add_skill!(p, deception),
                Skill::Diplomacy => add_skill!(p, diplomacy),
                Skill::Intimidation => add_skill!(p, intimidation),
                Skill::Lore(topic) => add_skill!(p, lore, topic),
                Skill::Medicine => add_skill!(p, medicine),
                Skill::Nature => add_skill!(p, nature),
                Skill::Occultism => add_skill!(p, occultism),
                Skill::Performance => add_skill!(p, performance),
                Skill::Religion => add_skill!(p, religion),
                Skill::Society => add_skill!(p, society),
                Skill::Stealth => add_skill!(p, stealth),
                Skill::Survival => add_skill!(p, survival),
                Skill::Thievery => add_skill!(p, thievery),
            }
        }

        skills
    }
}

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

    pub fn fill_pdf<T: PDFOutput>(
        &self,
        ability_scores: &AbilityScores,
        level: Level,
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
            let mut modifier = Modifier::new() + ability_bonus + prof_bonus + item_bonus;
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
                p.bonus(c.level).into()
            }
            _ => Modifier::new(),
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
