use anyhow::Result;
use std::fmt;

use crate::{
    bonuses::Modifier,
    resources::WeaponCategory,
    stats::{DamageType, Proficiency as P},
};

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum SkillSlot {
    Acrobatics,
    Arcana,
    Athletics,
    Crafting,
    Deception,
    Diplomacy,
    Intimidation,
    Lore1,
    Lore2,
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

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum WeaponSlot {
    Melee1,
    Melee2,
    Melee3,
    Ranged1,
    Ranged2,
    Ranged3,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum TextID {
    CharacterName,
    PlayerName,
    XP,
    AncestryAndHeritage,
    Background,
    Class,
    Size,
    Speed,
    MovementNotes,
    Alignment,
    CharacterTraits,
    Deity,
    CharacterLevel,
    HeroPoints,
    STRScore,
    DEXScore,
    CONScore,
    INTScore,
    WISScore,
    CHAScore,
    STRMod,
    DEXMod,
    CONMod,
    INTMod,
    WISMod,
    CHAMod,
    ClassDC,
    ClassDCKeyAbilityBonus,
    ClassDCProficiency,
    ClassDCItemBonus,
    TotalAC,
    ACDexBonus,
    ACDexCap,
    ACArmorProficiency,
    ACItemBonus,
    ShieldAC,
    ShieldHardness,
    ShieldMaxHP,
    ShieldBreakThreshold,
    // ShieldCurrentHP,
    FortSaveTotal,
    FortSaveCONBonus,
    FortSaveProficiency,
    FortSaveItemBonus,
    ReflexSaveTotal,
    ReflexSaveDEXBonus,
    ReflexSaveProficiency,
    ReflexSaveItemBonus,
    WillSaveTotal,
    WillSaveWISBonus,
    WillSaveProficiency,
    WillSaveItemBonus,
    SavingThrowNotes,
    MaxHP,
    ResistancesAndImmunities,
    PerceptionBonus,
    PerceptionWISBonus,
    PerceptionProficiency,
    PerceptionItemBonus,
    PerceptionSenses,
    SkillBonusTotal(SkillSlot),
    SkillAbilityBonus(SkillSlot),
    SkillProficiency(SkillSlot),
    SkillItemBonus(SkillSlot),
    SkillArmorPenaltyAcrobatics,
    SkillArmorPenaltyAthletics,
    SkillArmorPenaltyStealth,
    SkillArmorPenaltyThievery,
    LoreSkillTopic1,
    LoreSkillTopic2,
    WeaponName(WeaponSlot),
    WeaponAttackBonus(WeaponSlot),
    WeaponAttackAbilityBonus(WeaponSlot),
    WeaponProficiency(WeaponSlot),
    WeaponAttackItemBonus(WeaponSlot),
    WeaponDamageDice(WeaponSlot),
    WeaponDamageAbilityBonus(WeaponSlot),
    WeaponDamageSpecial(WeaponSlot),
    WeaponDamageOther(WeaponSlot),
    WeaponTraits(WeaponSlot),
}

impl TextID {
    pub fn armor_penalty_for_slot(s: SkillSlot) -> Option<Self> {
        match s {
            SkillSlot::Acrobatics => Some(Self::SkillArmorPenaltyAcrobatics),
            SkillSlot::Athletics => Some(Self::SkillArmorPenaltyAthletics),
            SkillSlot::Stealth => Some(Self::SkillArmorPenaltyStealth),
            SkillSlot::Thievery => Some(Self::SkillArmorPenaltyThievery),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum CheckboxID {
    ClassDCProficiency(P),
    ACArmorProficiency(P),
    UnarmoredProficiency(P),
    LightArmorProficiency(P),
    MediumArmorProficiency(P),
    HeavyArmorProficiency(P),
    FortSaveProficiency(P),
    ReflexSaveProficiency(P),
    WillSaveProficiency(P),
    PerceptionProficiency(P),
    SkillProficiency(SkillSlot, P),
    WeaponAttackProficiency(WeaponSlot, P),
    WeaponDamageType(DamageType, WeaponSlot),
    WeaponCategoryProficiency(WeaponCategory, P),
}

pub struct ProficiencyIDs {
    pub number: Option<TextID>,
    pub trained_cb: CheckboxID,
    pub expert_cb: CheckboxID,
    pub master_cb: CheckboxID,
    pub legendary_cb: CheckboxID,
}

impl ProficiencyIDs {
    #[inline]
    pub fn new<F: Fn(P) -> CheckboxID>(number: Option<TextID>, make_cb: F) -> Self {
        Self {
            number,
            trained_cb: make_cb(P::Trained),
            expert_cb: make_cb(P::Expert),
            master_cb: make_cb(P::Master),
            legendary_cb: make_cb(P::Legendary),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ProficiencyFields {
    ClassDC,
    ACArmor,
    Unarmored,
    LightArmor,
    MediumArmor,
    HeavyArmor,
    FortSave,
    ReflexSave,
    WillSave,
    Perception,
    Skill(SkillSlot),
    Weapon(WeaponSlot),
    WeaponCategory(WeaponCategory),
}

impl ProficiencyFields {
    fn ids(self) -> ProficiencyIDs {
        macro_rules! ids {
            (true, $variant:ident) => {
                ProficiencyIDs::new(Some(TextID::$variant), CheckboxID::$variant)
            };
            (false, $variant:ident) => {
                ProficiencyIDs::new(None, CheckboxID::$variant)
            };
            (None, $make_cb:expr) => {
                ProficiencyIDs::new(None, $make_cb)
            };
            ($number:expr, $make_cb:expr) => {
                ProficiencyIDs::new(Some($number), $make_cb)
            };
        }
        match self {
            Self::ClassDC => ids!(true, ClassDCProficiency),
            Self::ACArmor => ids!(true, ACArmorProficiency),
            Self::Unarmored => ids!(false, UnarmoredProficiency),
            Self::LightArmor => ids!(false, LightArmorProficiency),
            Self::MediumArmor => ids!(false, MediumArmorProficiency),
            Self::HeavyArmor => ids!(false, HeavyArmorProficiency),
            Self::FortSave => ids!(true, FortSaveProficiency),
            Self::ReflexSave => ids!(true, ReflexSaveProficiency),
            Self::WillSave => ids!(true, WillSaveProficiency),
            Self::Perception => ids!(true, PerceptionProficiency),
            Self::Skill(slot) => ids!(TextID::SkillProficiency(slot), |p| {
                CheckboxID::SkillProficiency(slot, p)
            }),
            Self::Weapon(slot) => ids!(TextID::WeaponProficiency(slot), |p| {
                CheckboxID::WeaponAttackProficiency(slot, p)
            }),
            Self::WeaponCategory(cat) => ids!(None, |p| {
                CheckboxID::WeaponCategoryProficiency(cat.clone(), p)
            }), // _ => todo!("proficiency fields {:?}", self),
        }
    }
}

// pub trait ImplDisplay {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result;
// }

// impl<T: fmt::Display> ImplDisplay for T {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         fmt::Display::fmt(self, f)
//     }
// }

// impl ImplDisplay for String {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         let s: &str = self.as_ref();
//         fmt::Display::fmt(self, f)
//     }
// }

pub trait PDFOutput: Sized {
    fn load_empty() -> Result<Self>;

    fn save<P: AsRef<std::path::Path>>(self, filename: P) -> Result<()>;

    fn set_text<T: fmt::Display>(&mut self, id: TextID, value: T) -> Result<()>;

    fn set_check_box(&mut self, id: CheckboxID, checked: bool) -> Result<()>;

    fn set_proficiency(
        &mut self,
        fields: ProficiencyFields,
        modifier: impl Into<Modifier>,
    ) -> Result<()> {
        let modifier = modifier.into();
        let (bonus, p) = modifier.proficiency_part();
        let ids = fields.ids();
        if let Some(n) = ids.number {
            self.set_text(n, bonus)?;
        }
        let (t, e, m, l) = match p {
            P::Untrained => (false, false, false, false),
            P::Trained => (true, false, false, false),
            P::Expert => (true, true, false, false),
            P::Master => (true, true, true, false),
            P::Legendary => (true, true, true, true),
        };
        self.set_check_box(ids.trained_cb, t)?;
        self.set_check_box(ids.expert_cb, e)?;
        self.set_check_box(ids.master_cb, m)?;
        self.set_check_box(ids.legendary_cb, l)?;
        Ok(())
    }
}
