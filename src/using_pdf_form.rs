use anyhow::{Context as _, Error};
use pdf_form::{FieldType as FT, Form, ValueError};
use std::{collections::HashMap, fmt, num::NonZeroUsize};

use crate::{
    pdf::{CheckboxID, PDFOutput, SkillSlot, TextID, WeaponSlot},
    Proficiency as P,
};

// checkboxes:
// armor proficiencies unarmored: 490, 491, 492, 493
// armor proficiencies light: 494, 495, 496, 497
// armor proficiencies medium: 498, 499, 500, 501
// armor proficiencies heavy: 502, 503, 504, 505
// magic traditions: 261, 262, 263, 264
// melee weapon 1 proficiency:
// melee weapon 2 proficiency:
// melee weapon 3 proficiency: 470, 471, 472, 473
// ranged weapon 1 proficiency: 466, 467, 468, 469
// ranged weapon 2 proficiency: 462, 463, 464, 465
// ranged weapon 3 proficiency: 458, 459, 460, 461
// spell cantrip prep: 609, 610, 611, 612, 614, 615, 616
// spell prep: 560, 585, 586, 587, 588, 589, 590, 591, 592, 593, 594, 595, 596, 597, 598, 599, 600, 601, 602, 603,604, 605, 606, 607, 608
// weapon proficiencies simple: 486, 487, 488, 489
// weapon proficiencies martial: 482, 483, 484, 485
// weapon proficiencies other 1: 478, 479, 480, 481
// weapon proficiencies other 2: 474, 475, 476, 477
// unknown: 613

const FAKE_CHECKBOX_FLAG: usize = 1 << 31;

#[allow(unused)]
fn locate_near(pdf: &Form) -> Result<(), ValueError> {
    type BB = ((f64, f64), (f64, f64));
    fn center_of_bb(bb: BB) -> (f64, f64) {
        let ((x1, y1), (x2, y2)) = bb;
        ((x1 + x2) / 2.0, (y1 + y2) / 2.0)
    }

    let field_types = pdf.get_all_types();
    let mut bounding_boxes = HashMap::new();
    let mut buttons = vec![];
    let mut radios = vec![];
    let mut checkboxes = vec![];
    let mut lists = vec![];
    let mut combos = vec![];
    for (i, ft) in field_types.into_iter().enumerate() {
        let state = pdf.get_state(i);
        if let Some(bb) = state.bounding_rect() {
            bounding_boxes.insert(i, bb);
        }
        match ft {
            FT::Button => {
                buttons.push(i);
            }
            FT::Radio => {
                radios.push(i);
            }
            FT::CheckBox => {
                checkboxes.push(i);
            }
            FT::ListBox => {
                lists.push(i);
            }
            FT::ComboBox => {
                combos.push(i);
            }
            FT::Text => (),
        }
    }

    println!("Buttons: {:?}", buttons);
    println!("Radios: {:?}", radios);
    println!("Check boxes: {:?}", checkboxes);
    println!("Lists: {:?}", lists);
    println!("Combos: {:?}", combos);

    // let offset = 240;
    // let count = 4;
    // let subset = &checkboxes[offset..offset + count];
    // println!("Checking boxes {:?}", subset);
    // for cbox_index in subset {
    //     let state = pdf.get_state(*cbox_index);
    //     println!("Box {} label: {:?}", cbox_index, state.label());
    //     pdf.set_check_box(*cbox_index, true)?;
    // }

    let near_id = 70;
    let near_id_loc = bounding_boxes[&near_id];
    let near_id_center = center_of_bb(near_id_loc);
    let max_distance = 100.0;
    let mut cboxes_near = checkboxes
        .iter()
        .filter_map(|cbox_id| {
            let cbox_bb = bounding_boxes.get(cbox_id)?;
            let cbox_center = center_of_bb(*cbox_bb);
            let dx = cbox_center.0 - near_id_center.0;
            let dy = cbox_center.1 - near_id_center.1;
            let distance = (dx * dx + dy * dy).sqrt();
            let angle = dy.atan2(dx);
            if distance <= max_distance && distance > 0.0 && angle.abs() < 0.2 {
                Some((distance.round() as usize, *cbox_id, angle))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    cboxes_near.sort_by_key(|(d, _, _)| *d);
    println!(
        "Found {} checkboxes near {} {:.2?}",
        cboxes_near.len(),
        near_id,
        near_id_loc
    );
    for (distance, cbox_id, angle) in cboxes_near {
        println!(
            "\t{} (distance: {}, angle: {:.2})",
            cbox_id, distance, angle
        );
    }

    Ok(())
}

impl PDFOutput for pdf_form::Form {
    fn load_empty() -> Result<Self, Error> {
        let pdf = pdf_form::Form::load("CharacterSheet-Color-fillable-1.2.pdf")?;
        // locate_near(&pdf)?;
        Ok(pdf)
    }

    fn save<P: AsRef<std::path::Path>>(mut self, filename: P) -> Result<(), Error> {
        let _: () = pdf_form::Form::save(&mut self, filename).context("Saving pdf_form::Form")?;
        Ok(())
    }

    #[inline]
    fn set_text<T: fmt::Display>(&mut self, id: TextID, value: T) -> Result<(), Error> {
        let object_id: ObjectID = id.into();
        // println!(
        //     "Converted text ID {:?} to PDF object ID {:?}",
        //     id, object_id
        // );

        // A few of these text fields map to the same PDF field, and
        // that must be handled specially here.
        let value = match id {
            TextID::ACDexBonus => {
                let prev_text = match self.get_state(object_id.0) {
                    pdf_form::FieldState::Text { text, .. } => text,
                    _ => panic!("Found a non-text field when setting AC dex bonus!"),
                };
                let mut new_text = format!("{:^3}", value);
                if prev_text.len() >= 7 {
                    // A cap has already been set, so we should copy that over.
                    let cap = &prev_text[7..];
                    while new_text.len() < 7 {
                        new_text.push(' ');
                    }
                    new_text.push_str(cap);
                }
                new_text
            }
            TextID::ACDexCap => {
                use std::fmt::Write;

                let mut new_text = match self.get_state(object_id.0) {
                    pdf_form::FieldState::Text { text, .. } => text,
                    _ => panic!("Found a non-text field when setting AC dex bonus cap!"),
                };
                if new_text.capacity() < 10 {
                    new_text.reserve(10 - new_text.capacity());
                }
                new_text.truncate(7);
                while new_text.len() < 7 {
                    new_text.push(' ');
                }
                write!(new_text, "{:^3}", value).unwrap();
                new_text
            }
            TextID::ShieldBreakThreshold => {
                // This field shares a slot with ShieldMaxHP, but
                // there isn't enough room for both. All of the
                // shields in the CRB have a BT of half max, so let's
                // leave that implied and not write it on the sheet.
                return Ok(());
            }
            _ => value.to_string(),
        };

        Ok(self
            .set_text(object_id.0, value.clone())
            .with_context(|| format!("Setting text field {:?} to {:?}", id, value))?)
    }

    #[inline]
    fn set_check_box(&mut self, id: CheckboxID, checked: bool) -> Result<(), Error> {
        let object_id: ObjectID = id.into();
        if object_id.0 & FAKE_CHECKBOX_FLAG != 0 {
            // Some "checkboxes" are represented on the sheet as tiny text boxes.
            let object_id = object_id.0 & !FAKE_CHECKBOX_FLAG;
            assert!(object_id < FAKE_CHECKBOX_FLAG);
            let text = if checked { "x" } else { "" };
            return Ok(self.set_text(object_id, text.to_string())?);
        }
        Ok(self
            .set_check_box(object_id.0, checked)
            .with_context(|| format!("Setting check box {:?} to {:?}", id, checked))?)
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
struct ObjectID(usize);

impl From<usize> for ObjectID {
    fn from(x: usize) -> ObjectID {
        ObjectID(x)
    }
}

impl From<ObjectID> for usize {
    fn from(oid: ObjectID) -> usize {
        oid.0
    }
}

impl From<TextID> for ObjectID {
    fn from(id: TextID) -> ObjectID {
        use TextID::*;

        let n = match id {
            CharacterName => 0,
            PlayerName => 1,
            XP => 6,
            AncestryAndHeritage => 385,
            Background => 384,
            Class => 2,
            Size => 3,
            Speed => 517,
            MovementNotes => 38,
            Alignment => 4,
            CharacterTraits => 5,
            Deity => 7,
            CharacterLevel => 258,
            HeroPoints => 818,
            STRScore => 257,
            DEXScore => 256,
            CONScore => 13,
            INTScore => 17,
            WISScore => 19,
            CHAScore => 26,
            STRMod => 255,
            DEXMod => 506,
            CONMod => 507,
            INTMod => 508,
            WISMod => 509,
            CHAMod => 510,
            ClassDC => 511,
            ClassDCKeyAbilityBonus => 33,
            ClassDCProficiency => 34,
            ClassDCItemBonus => 35,
            TotalAC => 515,
            ACDexBonus => 516,
            ACDexCap => 516,
            ACArmorProficiency => 8,
            ACItemBonus => 9,
            ShieldAC => 537,
            ShieldHardness => 14,
            ShieldMaxHP => 15,
            ShieldBreakThreshold => 15,
            ShieldCurrentHP => 839,
            FortSaveTotal => 512,
            FortSaveCONBonus => 20,
            FortSaveProficiency => 21,
            FortSaveItemBonus => 27,
            ReflexSaveTotal => 513,
            ReflexSaveDEXBonus => 22,
            ReflexSaveProficiency => 23,
            ReflexSaveItemBonus => 28,
            WillSaveTotal => 514,
            WillSaveWISBonus => 24,
            WillSaveProficiency => 25,
            WillSaveItemBonus => 29,
            SavingThrowNotes => 36,
            MaxHP => 838,
            ResistancesAndImmunities => 16,
            PerceptionBonus => 536,
            PerceptionWISBonus => 30,
            PerceptionProficiency => 31,
            PerceptionItemBonus => 32,
            PerceptionSenses => 37,
            LoreSkillTopic1 => 83,
            LoreSkillTopic2 => 102,
            SkillAbilityBonus(SkillSlot::Acrobatics) => 39,
            SkillAbilityBonus(SkillSlot::Arcana) => 47,
            SkillAbilityBonus(SkillSlot::Athletics) => 53,
            SkillAbilityBonus(SkillSlot::Crafting) => 62,
            SkillAbilityBonus(SkillSlot::Deception) => 69,
            SkillAbilityBonus(SkillSlot::Diplomacy) => 80,
            SkillAbilityBonus(SkillSlot::Intimidation) => 84,
            SkillAbilityBonus(SkillSlot::Lore1) => 91,
            SkillAbilityBonus(SkillSlot::Lore2) => 103,
            SkillAbilityBonus(SkillSlot::Medicine) => 106,
            SkillAbilityBonus(SkillSlot::Nature) => 113,
            SkillAbilityBonus(SkillSlot::Occultism) => 120,
            SkillAbilityBonus(SkillSlot::Performance) => 127,
            SkillAbilityBonus(SkillSlot::Religion) => 134,
            SkillAbilityBonus(SkillSlot::Society) => 145,
            SkillAbilityBonus(SkillSlot::Stealth) => 152,
            SkillAbilityBonus(SkillSlot::Survival) => 159,
            SkillAbilityBonus(SkillSlot::Thievery) => 166,
            SkillArmorPenaltyAcrobatics => 42,
            SkillArmorPenaltyAthletics => 56,
            SkillArmorPenaltyStealth => 155,
            SkillArmorPenaltyThievery => 169,
            SkillBonusTotal(SkillSlot::Acrobatics) => 518,
            SkillBonusTotal(SkillSlot::Arcana) => 519,
            SkillBonusTotal(SkillSlot::Athletics) => 520,
            SkillBonusTotal(SkillSlot::Crafting) => 521,
            SkillBonusTotal(SkillSlot::Deception) => 522,
            SkillBonusTotal(SkillSlot::Diplomacy) => 523,
            SkillBonusTotal(SkillSlot::Intimidation) => 524,
            SkillBonusTotal(SkillSlot::Lore1) => 525,
            SkillBonusTotal(SkillSlot::Lore2) => 526,
            SkillBonusTotal(SkillSlot::Medicine) => 527,
            SkillBonusTotal(SkillSlot::Nature) => 528,
            SkillBonusTotal(SkillSlot::Occultism) => 529,
            SkillBonusTotal(SkillSlot::Performance) => 530,
            SkillBonusTotal(SkillSlot::Religion) => 531,
            SkillBonusTotal(SkillSlot::Society) => 532,
            SkillBonusTotal(SkillSlot::Stealth) => 533,
            SkillBonusTotal(SkillSlot::Survival) => 534,
            SkillBonusTotal(SkillSlot::Thievery) => 535,
            SkillItemBonus(SkillSlot::Acrobatics) => 41,
            SkillItemBonus(SkillSlot::Arcana) => 49,
            SkillItemBonus(SkillSlot::Athletics) => 55,
            SkillItemBonus(SkillSlot::Crafting) => 64,
            SkillItemBonus(SkillSlot::Deception) => 71,
            SkillItemBonus(SkillSlot::Diplomacy) => 82,
            SkillItemBonus(SkillSlot::Intimidation) => 87,
            SkillItemBonus(SkillSlot::Lore1) => 93,
            SkillItemBonus(SkillSlot::Lore2) => 105,
            SkillItemBonus(SkillSlot::Medicine) => 108,
            SkillItemBonus(SkillSlot::Nature) => 115,
            SkillItemBonus(SkillSlot::Occultism) => 126,
            SkillItemBonus(SkillSlot::Performance) => 132,
            SkillItemBonus(SkillSlot::Religion) => 136,
            SkillItemBonus(SkillSlot::Society) => 147,
            SkillItemBonus(SkillSlot::Stealth) => 154,
            SkillItemBonus(SkillSlot::Survival) => 165,
            SkillItemBonus(SkillSlot::Thievery) => 168,
            SkillProficiency(SkillSlot::Acrobatics) => 40,
            SkillProficiency(SkillSlot::Arcana) => 48,
            SkillProficiency(SkillSlot::Athletics) => 54,
            SkillProficiency(SkillSlot::Crafting) => 63,
            SkillProficiency(SkillSlot::Deception) => 70,
            SkillProficiency(SkillSlot::Diplomacy) => 81,
            SkillProficiency(SkillSlot::Intimidation) => 85,
            SkillProficiency(SkillSlot::Lore1) => 92,
            SkillProficiency(SkillSlot::Lore2) => 104,
            SkillProficiency(SkillSlot::Medicine) => 107,
            SkillProficiency(SkillSlot::Nature) => 114,
            SkillProficiency(SkillSlot::Occultism) => 121,
            SkillProficiency(SkillSlot::Performance) => 128,
            SkillProficiency(SkillSlot::Religion) => 135,
            SkillProficiency(SkillSlot::Society) => 146,
            SkillProficiency(SkillSlot::Stealth) => 153,
            SkillProficiency(SkillSlot::Survival) => 160,
            SkillProficiency(SkillSlot::Thievery) => 167,
            WeaponName(WeaponSlot::Melee1) => 43,
            WeaponAttackBonus(WeaponSlot::Melee1) => 769,
            WeaponAttackAbilityBonus(WeaponSlot::Melee1) => 44,
            WeaponProficiency(WeaponSlot::Melee1) => 45,
            WeaponAttackItemBonus(WeaponSlot::Melee1) => 46,
            WeaponDamageDice(WeaponSlot::Melee1) => 50,
            WeaponDamageAbilityBonus(WeaponSlot::Melee1) => 51,
            WeaponDamageSpecial(WeaponSlot::Melee1) => 59,
            WeaponDamageOther(WeaponSlot::Melee1) => 60,
            WeaponTraits(WeaponSlot::Melee1) => 61,
            WeaponName(WeaponSlot::Melee2) => 65,
            WeaponAttackBonus(WeaponSlot::Melee2) => 768,
            WeaponAttackAbilityBonus(WeaponSlot::Melee2) => 66,
            WeaponProficiency(WeaponSlot::Melee2) => 67,
            WeaponAttackItemBonus(WeaponSlot::Melee2) => 68,
            WeaponDamageDice(WeaponSlot::Melee2) => 72,
            WeaponDamageAbilityBonus(WeaponSlot::Melee2) => 73,
            WeaponDamageSpecial(WeaponSlot::Melee2) => 77,
            WeaponDamageOther(WeaponSlot::Melee2) => 78,
            WeaponTraits(WeaponSlot::Melee2) => 79,
            WeaponName(WeaponSlot::Melee3) => 86,
            WeaponAttackBonus(WeaponSlot::Melee3) => 767,
            WeaponAttackAbilityBonus(WeaponSlot::Melee3) => 88,
            WeaponProficiency(WeaponSlot::Melee3) => 89,
            WeaponAttackItemBonus(WeaponSlot::Melee3) => 90,
            WeaponDamageDice(WeaponSlot::Melee3) => 94,
            WeaponDamageAbilityBonus(WeaponSlot::Melee3) => 95,
            WeaponDamageSpecial(WeaponSlot::Melee3) => 99,
            WeaponDamageOther(WeaponSlot::Melee3) => 100,
            WeaponTraits(WeaponSlot::Melee3) => 101,
            WeaponName(WeaponSlot::Ranged1) => 109,
            WeaponAttackBonus(WeaponSlot::Ranged1) => 766,
            WeaponAttackAbilityBonus(WeaponSlot::Ranged1) => 110,
            WeaponProficiency(WeaponSlot::Ranged1) => 111,
            WeaponAttackItemBonus(WeaponSlot::Ranged1) => 112,
            WeaponDamageDice(WeaponSlot::Ranged1) => 116,
            WeaponDamageAbilityBonus(WeaponSlot::Ranged1) => 117,
            WeaponDamageSpecial(WeaponSlot::Ranged1) => 123,
            WeaponDamageOther(WeaponSlot::Ranged1) => 124,
            WeaponTraits(WeaponSlot::Ranged1) => 125,
            WeaponName(WeaponSlot::Ranged2) => 129,
            WeaponAttackBonus(WeaponSlot::Ranged2) => 765,
            WeaponAttackAbilityBonus(WeaponSlot::Ranged2) => 130,
            WeaponProficiency(WeaponSlot::Ranged2) => 131,
            WeaponAttackItemBonus(WeaponSlot::Ranged2) => 133,
            WeaponDamageDice(WeaponSlot::Ranged2) => 137,
            WeaponDamageAbilityBonus(WeaponSlot::Ranged2) => 138,
            WeaponDamageSpecial(WeaponSlot::Ranged2) => 142,
            WeaponDamageOther(WeaponSlot::Ranged2) => 143,
            WeaponTraits(WeaponSlot::Ranged2) => 144,
            WeaponName(WeaponSlot::Ranged3) => 148,
            WeaponAttackBonus(WeaponSlot::Ranged3) => 764,
            WeaponAttackAbilityBonus(WeaponSlot::Ranged3) => 149,
            WeaponProficiency(WeaponSlot::Ranged3) => 150,
            WeaponAttackItemBonus(WeaponSlot::Ranged3) => 151,
            WeaponDamageDice(WeaponSlot::Ranged3) => 156,
            WeaponDamageAbilityBonus(WeaponSlot::Ranged3) => 157,
            WeaponDamageSpecial(WeaponSlot::Ranged3) => 163,
            WeaponDamageOther(WeaponSlot::Ranged3) => 763,
            WeaponTraits(WeaponSlot::Ranged3) => 164,
        };
        ObjectID(n)
    }
}

impl From<CheckboxID> for ObjectID {
    fn from(id: CheckboxID) -> ObjectID {
        macro_rules! for_prof {
            ($p:ident, $t:expr, $e:expr, $m:expr, $l:expr) => {
                match $p {
                    P::Untrained => {
                        panic!("From<CheckboxID> for ObjectID: got untrained proficiency")
                    }
                    P::Trained => $t,
                    P::Expert => $e,
                    P::Master => $m,
                    P::Legendary => $l,
                }
            };
        }

        use CheckboxID as C;
        use SkillSlot::*;

        let n = match id {
            C::ClassDCProficiency(p) => for_prof!(p, 835, 836, 851, 837),
            C::ACArmorProficiency(p) => for_prof!(p, 822, 823, 824, 825),
            C::UnarmoredProficiency(_) => todo!("Need to find proficiency checkboxes for {:?}", id),
            C::LightArmorProficiency(_) => {
                todo!("Need to find proficiency checkboxes for {:?}", id)
            }
            C::MediumArmorProficiency(_) => {
                todo!("Need to find proficiency checkboxes for {:?}", id)
            }
            C::HeavyArmorProficiency(_) => {
                todo!("Need to find proficiency checkboxes for {:?}", id)
            }
            C::FortSaveProficiency(p) => for_prof!(p, 388, 389, 390, 391),
            C::ReflexSaveProficiency(p) => for_prof!(p, 826, 827, 828, 829),
            C::WillSaveProficiency(p) => for_prof!(p, 392, 393, 816, 394),
            C::PerceptionProficiency(p) => for_prof!(p, 835, 836, 851, 837),
            C::SkillProficiency(Acrobatics, p) => for_prof!(p, 830, 831, 833, 834),
            C::SkillProficiency(Arcana, p) => for_prof!(p, 395, 396, 397, 398),
            C::SkillProficiency(Athletics, p) => for_prof!(p, 399, 400, 401, 402),
            C::SkillProficiency(Crafting, p) => for_prof!(p, 403, 404, 405, 406),
            C::SkillProficiency(Deception, p) => for_prof!(p, 407, 408, 409, 410),
            C::SkillProficiency(Diplomacy, p) => for_prof!(p, 411, 412, 413, 414),
            C::SkillProficiency(Intimidation, p) => for_prof!(p, 415, 416, 417, 418),
            C::SkillProficiency(Lore1, p) => for_prof!(p, 419, 821, 420, 421),
            C::SkillProficiency(Lore2, p) => for_prof!(p, 422, 423, 424, 425),
            C::SkillProficiency(Medicine, p) => for_prof!(p, 426, 427, 428, 429),
            C::SkillProficiency(Nature, p) => for_prof!(p, 430, 431, 432, 433),
            C::SkillProficiency(Occultism, p) => for_prof!(p, 434, 435, 436, 437),
            C::SkillProficiency(Performance, p) => for_prof!(p, 438, 439, 440, 441),
            C::SkillProficiency(Religion, p) => for_prof!(p, 442, 443, 444, 445),
            C::SkillProficiency(Society, p) => for_prof!(p, 446, 447, 448, 449),
            C::SkillProficiency(Stealth, p) => for_prof!(p, 450, 451, 452, 453),
            C::SkillProficiency(Survival, p) => for_prof!(p, 454, 455, 456, 457),
            C::SkillProficiency(Thievery, p) => for_prof!(p, 855, 857, 859, 861),
            C::WeaponProficiency(WeaponSlot::Melee1, p) => for_prof!(p, 815, 832, 820, 817),
            C::WeaponDamageTypeB(WeaponSlot::Melee1) => 52 | FAKE_CHECKBOX_FLAG,
            C::WeaponDamageTypeP(WeaponSlot::Melee1) => 57 | FAKE_CHECKBOX_FLAG,
            C::WeaponDamageTypeS(WeaponSlot::Melee1) => 58 | FAKE_CHECKBOX_FLAG,
            C::WeaponProficiency(WeaponSlot::Melee2, p) => for_prof!(p, 810, 811, 812, 813),
            C::WeaponDamageTypeB(WeaponSlot::Melee2) => 74 | FAKE_CHECKBOX_FLAG,
            C::WeaponDamageTypeP(WeaponSlot::Melee2) => 75 | FAKE_CHECKBOX_FLAG,
            C::WeaponDamageTypeS(WeaponSlot::Melee2) => 76 | FAKE_CHECKBOX_FLAG,
            C::WeaponProficiency(WeaponSlot::Melee3, p) => for_prof!(p, 470, 471, 472, 473),
            C::WeaponDamageTypeB(WeaponSlot::Melee3) => 96 | FAKE_CHECKBOX_FLAG,
            C::WeaponDamageTypeP(WeaponSlot::Melee3) => 97 | FAKE_CHECKBOX_FLAG,
            C::WeaponDamageTypeS(WeaponSlot::Melee3) => 98 | FAKE_CHECKBOX_FLAG,
            C::WeaponProficiency(WeaponSlot::Ranged1, p) => for_prof!(p, 466, 467, 468, 469),
            C::WeaponDamageTypeB(WeaponSlot::Ranged1) => 118 | FAKE_CHECKBOX_FLAG,
            C::WeaponDamageTypeP(WeaponSlot::Ranged1) => 119 | FAKE_CHECKBOX_FLAG,
            C::WeaponDamageTypeS(WeaponSlot::Ranged1) => 122 | FAKE_CHECKBOX_FLAG,
            C::WeaponProficiency(WeaponSlot::Ranged2, p) => for_prof!(p, 462, 463, 464, 465),
            C::WeaponDamageTypeB(WeaponSlot::Ranged2) => 139 | FAKE_CHECKBOX_FLAG,
            C::WeaponDamageTypeP(WeaponSlot::Ranged2) => 140 | FAKE_CHECKBOX_FLAG,
            C::WeaponDamageTypeS(WeaponSlot::Ranged2) => 141 | FAKE_CHECKBOX_FLAG,
            C::WeaponProficiency(WeaponSlot::Ranged3, p) => for_prof!(p, 458, 459, 460, 461),
            C::WeaponDamageTypeB(WeaponSlot::Ranged3) => 158 | FAKE_CHECKBOX_FLAG,
            C::WeaponDamageTypeP(WeaponSlot::Ranged3) => 161 | FAKE_CHECKBOX_FLAG,
            C::WeaponDamageTypeS(WeaponSlot::Ranged3) => 162 | FAKE_CHECKBOX_FLAG,
            // other => todo!("Need to find checkbox ID for {:?}", other),
        };
        ObjectID(n)
    }
}

#[derive(Clone, Debug)]
struct SkillPDFIDs {
    total: usize,
    ability: usize,
    prof: usize,
    prof_t: usize,
    prof_e: usize,
    prof_m: usize,
    prof_l: usize,
    item: usize,
    armor_penalty: Option<NonZeroUsize>,
    lore_topic: Option<NonZeroUsize>,
}

// impl Skill {
//     fn pdf_ids(&self) -> SkillPDFIDs {
//         match self {
//             Self::Acrobatics => SkillPDFIDs {
//                 total: 518,
//                 ability: 39,
//                 prof: 40,
//                 prof_t: 830,
//                 prof_e: 831,
//                 prof_m: 833,
//                 prof_l: 834,
//                 item: 41,
//                 armor_penalty: NonZeroUsize::new(42),
//                 lore_topic: None,
//             },
//             Self::Arcana => SkillPDFIDs {
//                 total: 519,
//                 ability: 47,
//                 prof: 48,
//                 prof_t: 395,
//                 prof_e: 396,
//                 prof_m: 397,
//                 prof_l: 398,
//                 item: 49,
//                 armor_penalty: None,
//                 lore_topic: None,
//             },
//             Self::Athletics => SkillPDFIDs {
//                 total: 520,
//                 ability: 53,
//                 prof: 54,
//                 prof_t: 399,
//                 prof_e: 400,
//                 prof_m: 401,
//                 prof_l: 402,
//                 item: 55,
//                 armor_penalty: NonZeroUsize::new(56),
//                 lore_topic: None,
//             },
//             Self::Crafting => SkillPDFIDs {
//                 total: 521,
//                 ability: 62,
//                 prof: 63,
//                 prof_t: 403,
//                 prof_e: 404,
//                 prof_m: 405,
//                 prof_l: 406,
//                 item: 64,
//                 armor_penalty: None,
//                 lore_topic: None,
//             },
//             Self::Deception => SkillPDFIDs {
//                 total: 522,
//                 ability: 69,
//                 prof: 70,
//                 prof_t: 0,
//                 prof_e: 0,
//                 prof_m: 0,
//                 prof_l: 0,
//                 item: 71,
//                 armor_penalty: None,
//                 lore_topic: None,
//             },
//             Self::Diplomacy => SkillPDFIDs {
//                 total: 523,
//                 ability: 80,
//                 prof: 81,
//                 prof_t: 411,
//                 prof_e: 412,
//                 prof_m: 413,
//                 prof_l: 414,
//                 item: 82,
//                 armor_penalty: None,
//                 lore_topic: None,
//             },
//             Self::Intimidation => SkillPDFIDs {
//                 total: 524,
//                 ability: 84,
//                 prof: 85,
//                 prof_t: 415,
//                 prof_e: 416,
//                 prof_m: 417,
//                 prof_l: 418,
//                 item: 87,
//                 armor_penalty: None,
//                 lore_topic: None,
//             },
//             Self::Lore(_) => panic!("Lore skills should use Skill::lore_pdf_ids"),
//             Self::Medicine => SkillPDFIDs {
//                 total: 527,
//                 ability: 106,
//                 prof: 107,
//                 prof_t: 426,
//                 prof_e: 427,
//                 prof_m: 428,
//                 prof_l: 429,
//                 item: 108,
//                 armor_penalty: None,
//                 lore_topic: None,
//             },
//             Self::Nature => SkillPDFIDs {
//                 total: 528,
//                 ability: 113,
//                 prof: 114,
//                 prof_t: 430,
//                 prof_e: 431,
//                 prof_m: 432,
//                 prof_l: 433,
//                 item: 115,
//                 armor_penalty: None,
//                 lore_topic: None,
//             },
//             Self::Occultism => SkillPDFIDs {
//                 total: 529,
//                 ability: 120,
//                 prof: 121,
//                 prof_t: 434,
//                 prof_e: 435,
//                 prof_m: 436,
//                 prof_l: 437,
//                 item: 126,
//                 armor_penalty: None,
//                 lore_topic: None,
//             },
//             Self::Performance => SkillPDFIDs {
//                 total: 530,
//                 ability: 127,
//                 prof: 128,
//                 prof_t: 438,
//                 prof_e: 439,
//                 prof_m: 440,
//                 prof_l: 441,
//                 item: 132,
//                 armor_penalty: None,
//                 lore_topic: None,
//             },
//             Self::Religion => SkillPDFIDs {
//                 total: 531,
//                 ability: 134,
//                 prof: 135,
//                 prof_t: 442,
//                 prof_e: 443,
//                 prof_m: 444,
//                 prof_l: 445,
//                 item: 136,
//                 armor_penalty: None,
//                 lore_topic: None,
//             },
//             Self::Society => SkillPDFIDs {
//                 total: 532,
//                 ability: 145,
//                 prof: 146,
//                 prof_t: 446,
//                 prof_e: 447,
//                 prof_m: 448,
//                 prof_l: 449,
//                 item: 147,
//                 armor_penalty: None,
//                 lore_topic: None,
//             },
//             Self::Stealth => SkillPDFIDs {
//                 total: 533,
//                 ability: 152,
//                 prof: 153,
//                 prof_t: 450,
//                 prof_e: 451,
//                 prof_m: 452,
//                 prof_l: 453,
//                 item: 154,
//                 armor_penalty: NonZeroUsize::new(155),
//                 lore_topic: None,
//             },
//             Self::Survival => SkillPDFIDs {
//                 total: 534,
//                 ability: 159,
//                 prof: 160,
//                 prof_t: 454,
//                 prof_e: 455,
//                 prof_m: 456,
//                 prof_l: 457,
//                 item: 165,
//                 armor_penalty: None,
//                 lore_topic: None,
//             },
//             Self::Thievery => SkillPDFIDs {
//                 total: 535,
//                 ability: 166,
//                 prof: 167,
//                 prof_t: 855,
//                 prof_e: 857,
//                 prof_m: 859,
//                 prof_l: 861,
//                 item: 168,
//                 armor_penalty: NonZeroUsize::new(169),
//                 lore_topic: None,
//             },
//         }
//     }

//     fn lore_pdf_ids(slot: usize) -> SkillPDFIDs {
//         match slot {
//             _ => panic!("Only two lore skills can be shown on the character sheet"),
//         }
//     }
// }
