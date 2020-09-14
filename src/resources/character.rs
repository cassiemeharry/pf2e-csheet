use anyhow::Result;
use serde::Deserialize;
use smartstring::alias::String;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{
    bonuses::{Bonus, HasModifiers, Modifier, Modifies, Penalty},
    pdf::{CheckboxID, PDFOutput, ProficiencyFields, TextID, WeaponSlot},
    qa::{Answer, AnswerMap, AnswerMapRef, Question},
    resources::refs::Ref,
    resources::traits::Resource,
    resources::{
        Ancestry, ArmorCategory, ArmorInfo, Background, Class, Feat, Heritage, Item, ShieldInfo,
        WeaponCategory, WeaponTrait,
    },
    stats::{
        Ability, Alignment, DamageType, Level, Proficiency, ProficiencyCategory,
        ProvidesProficiency,
    },
};

mod helpers;

use helpers::{AbilityScores, SkillProficiencies};

#[derive(Clone, Debug, Deserialize)]
pub struct Character {
    name: String,
    player_name: String,
    ancestry: Ref<Ancestry>,
    heritage: Ref<Heritage>,
    background: Ref<Background>,
    class: Ref<Class>,
    #[serde(default)]
    feats: Vec<Ref<Feat>>,
    class_key_ability: Ability,
    alignment: Alignment,
    deity: String,
    traits: Vec<String>,
    xp: usize,
    level: Level,
    #[serde(skip)]
    ability_scores: AbilityScores,
    #[serde(skip)]
    skill_proficiencies: SkillProficiencies,
    #[serde(default)]
    equipped_items: Vec<Ref<Item>>,
    #[serde(default)]
    unequipped_items: Vec<Ref<Item>>,
    #[serde(default)]
    answers: AnswerMap,
}

impl Character {
    pub fn character_level(&self) -> Level {
        self.level
    }

    pub fn class_level(&self, class_name: &str) -> Level {
        // TODO: account for multi-classing
        let _ = class_name;
        self.level
    }

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
        pdf.set_text(
            TextID::AncestryAndHeritage,
            format_args!("{} ({})", self.ancestry, self.heritage),
        )?;
        pdf.set_text(TextID::Background, &self.background)?;
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
            let (armor_bonus, armor_info): (Modifier, ArmorInfo) = self
                .equipped_items
                .iter()
                .filter_map(|item| {
                    let item = item.get();
                    match item.armor_info() {
                        Some(info) => Some((item.get_modifier(self, Modifies::AC), info.clone())),
                        None => None,
                    }
                })
                .next()
                .unwrap_or_else(|| (Modifier::new(), ArmorInfo::no_armor()));
            let armor_prof = self.get_proficiency(ProficiencyCategory::Armor(armor_info.category));
            let prof_bonus = armor_prof.bonus(self.level);
            let before_dex = Bonus::untyped(10) + prof_bonus + armor_bonus.clone();

            let dex_bonus = self.ability_scores.bonus(DEX);
            let dex_bonus_total = dex_bonus.total();
            let with_dex = before_dex.clone() + dex_bonus;
            let mut total: Modifier = with_dex.into();

            match armor_info.dex_cap {
                Some(cap) if (cap as u16 as i16) < dex_bonus_total => {
                    total = (before_dex + Bonus::untyped(cap as u16 as i16)).into();
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
            let shield_info_opt: Option<(Modifier, ShieldInfo)> = self
                .equipped_items
                .iter()
                .filter_map(|item| {
                    let item = item.get();
                    match item.shield_info() {
                        Some(si) => Some((item.get_modifier(self, Modifies::AC), si.clone())),
                        None => None,
                    }
                })
                .next();
            if let Some((ac_mod, shield_info)) = shield_info_opt {
                pdf.set_text(TextID::ShieldAC, ac_mod)?;
                pdf.set_text(TextID::ShieldHardness, shield_info.hardness)?;
                pdf.set_text(TextID::ShieldMaxHP, shield_info.hp)?;
                pdf.set_text(TextID::ShieldBreakThreshold, shield_info.hp / 2)?;
            }
        }

        // Perception
        {
            let wis_bonus = self.ability_scores.bonus(WIS);
            let prof = self.get_proficiency(ProficiencyCategory::Perception);
            let item_bonus = Bonus::item(0); // TODO
            let modifier = wis_bonus + prof.bonus(self.level) + item_bonus;

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

            for item in self.equipped_items.iter() {
                let item = item.get();
                if let Some(weapon) = item.weapon_info() {
                    let slot = match &weapon.range {
                        Some(_) => match ranged_slots.pop() {
                            Some(s) => s,
                            None => {
                                eprintln!("Too many ranged weapons, not rendering weapon {}", item);
                                continue;
                            }
                        },
                        None => match melee_slots.pop() {
                            Some(s) => s,
                            None => {
                                eprintln!("Too many melee weapons, not rendering weapon {}", item);
                                continue;
                            }
                        },
                    };
                    let str_bonus = self.ability_scores.bonus(STR);
                    let dex_bonus = self.ability_scores.bonus(DEX);
                    let is_finesse = weapon
                        .traits
                        .iter()
                        .any(|t| matches!(t, WeaponTrait::Finesse));
                    let ability_bonus = match (
                        weapon.range,
                        is_finesse,
                        dex_bonus.total() > str_bonus.total(),
                    ) {
                        (Some(_), _, _) => dex_bonus.clone(),
                        (None, true, true) => dex_bonus.clone(),
                        (None, _, _) => str_bonus.clone(),
                    };
                    let prof =
                        self.get_proficiency(ProficiencyCategory::Weapon(weapon.category.clone()));
                    let item_bonus = item.get_modifier(self, Modifies::Attack);
                    let attack_mod = ability_bonus + prof.bonus(self.level) + item_bonus.clone();

                    // attack
                    pdf.set_text(TextID::WeaponName(slot), &item)?;
                    pdf.set_text(TextID::WeaponAttackBonus(slot), attack_mod)?;
                    pdf.set_text(TextID::WeaponAttackAbilityBonus(slot), ability_bonus)?;
                    pdf.set_proficiency(ProficiencyFields::Weapon(slot), prof.bonus(self.level))?;
                    pdf.set_text(TextID::WeaponAttackItemBonus(slot), item_bonus)?;

                    // damage
                    pdf.set_text(TextID::WeaponDamageDice(slot), weapon.damage_die)?;
                    if weapon.range.is_none() {
                        pdf.set_text(TextID::WeaponDamageAbilityBonus(slot), str_bonus)?;
                    }
                    let (mut b, mut p, mut s) = match weapon.damage_type {
                        DamageType::B => (true, false, false),
                        DamageType::P => (false, true, false),
                        DamageType::S => (false, false, true),
                    };
                    for t in weapon.traits.iter() {
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
                        let mut traits_strings = Vec::with_capacity(weapon.traits.len());
                        for t in &weapon.traits {
                            traits_strings.push(format!("{}", t));
                        }
                        pdf.set_text(TextID::WeaponTraits(slot), traits_strings.join(", "))?;
                    }
                }
            }
            for category in [WeaponCategory::Simple, WeaponCategory::Martial].iter() {
                let p = self.get_proficiency(ProficiencyCategory::Weapon(category.clone()));
                pdf.set_proficiency(
                    ProficiencyFields::WeaponCategory(category.clone()),
                    p.bonus(self.level),
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

    fn get_proficiency(&self, p: ProficiencyCategory) -> Proficiency {
        let mut result = Proficiency::Untrained;
        result = result.max(self.class.get().get_proficiency_level(self, &p));
        result
    }

    fn get_modifier(&self, m: Modifies) -> Modifier {
        let mut total = self.ancestry.get().get_modifier(self, m.clone())
            + self.background.get().get_modifier(self, m.clone())
            + self.class.get().get_modifier(self, m.clone());

        for item in self.equipped_items.iter() {
            total += item.get().get_modifier(self, m.clone());
        }

        match m {
            Modifies::ClassDC => {
                total += self.ability_scores.bonus(self.class_key_ability);
            }
            Modifies::FortitudeSave => {
                total += self.ability_scores.bonus(Ability::CON);
            }
            Modifies::HP => {
                total += self.ability_scores.bonus(Ability::CON) * self.level;
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
        resistances.extend(self.ancestry.get().get_modified_resistances(self));
        resistances.extend(self.background.get().get_modified_resistances(self));
        resistances.extend(self.class.get().get_modified_resistances(self));

        for item in self.equipped_items.iter() {
            resistances.extend(item.get().get_modified_resistances(self));
        }

        resistances
    }

    pub fn get_unanswered_questions(&self) -> Vec<Question> {
        fn get<'a, R: Resource>(c: &'a Character, rr: &Ref<R>) -> Vec<Question> {
            let r: Arc<R> = match rr.try_get() {
                Ok(r) => r,
                Err(e) => return vec![],
            };
            let qs = r.get_questions();
            let fake_answers: HashMap<String, Arc<Answer>>;
            let real_answers;
            let answers: &HashMap<String, Arc<Answer>> = match c.get_answers(&*r) {
                Some(am) => {
                    real_answers = am;
                    &*real_answers
                }
                None => {
                    fake_answers = HashMap::new();
                    &fake_answers
                }
            };
            qs.into_iter()
                .filter(move |q| !answers.contains_key(&q.tag))
                .collect()
        }
        let mut result = vec![];
        macro_rules! get {
            ($field:ident) => {
                result.extend(get(&self, &self.$field))
            };
        }
        get!(ancestry);
        get!(heritage);
        get!(background);
        get!(class);
        result
    }

    pub fn get_answers<'a, R: Resource>(&'a self, resource: &R) -> Option<AnswerMapRef<'a>> {
        self.answers.answers_for_resource(resource)
    }

    pub fn provide_answer_to_question(
        &mut self,
        question: &Question,
        answer: Answer,
    ) -> Result<()> {
        // Need to figure out a better way to map questions back to the resource
        // that asked for it.
        anyhow::bail!("TODO: Character::provide_answer_to_question")
    }
}
