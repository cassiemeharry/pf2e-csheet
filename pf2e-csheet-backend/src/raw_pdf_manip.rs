use anyhow::Result;
use lazy_static::lazy_static;
use lopdf::{
    content::{Content, Operation},
    dictionary, Document, Object,
};
use smartstring::alias::String;
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use crate::{
    pdf::{CheckboxID, PDFOutput, SkillSlot, TextID, WeaponSlot},
    resources::WeaponCategory,
    stats::{DamageType, Proficiency as P},
};

const PLAYER_FONT_NAME: &'static [u8] = b"PLAYER_FONT";
static PDF_BYTES: &'static [u8] = include_bytes!("../resources/PZO2101-CharacterSheet-Color.pdf");
// static PDF_BYTES: &'static [u8] = include_bytes!("../PZO2101-CharacterSheet-BW.pdf");

#[derive(Clone, Debug)]
pub struct PDF {
    document: Document,
    assigned_text_fields: HashMap<TextID, String>,
    assigned_checkbox_fields: HashSet<CheckboxID>,
}

fn weapon_offset(slot: WeaponSlot) -> u32 {
    use WeaponSlot::*;
    match slot {
        Melee1 => 0,
        Melee2 => 58,
        Melee3 => 116,
        Ranged1 => 190,
        Ranged2 => 246,
        Ranged3 => 303,
    }
}

fn skill_offset(slot: SkillSlot) -> u32 {
    use SkillSlot::*;
    let n = match slot {
        Acrobatics => 0,
        Arcana => 1,
        Athletics => 2,
        Crafting => 3,
        Deception => 4,
        Diplomacy => 5,
        Intimidation => 6,
        Lore1 => 7,
        Lore2 => 8,
        Medicine => 9,
        Nature => 10,
        Occultism => 11,
        Performance => 12,
        Religion => 13,
        Society => 14,
        Stealth => 15,
        Survival => 16,
        Thievery => 17,
    };
    (n as f32 * 21.85).round() as u32
}

impl PDF {
    fn get_checkbox_layout(&self, id: CheckboxID) -> Option<TextLayout> {
        use CheckboxID as C;

        const TALL_X_FONT_SIZE: f32 = 8.0;
        const TALL_X_HEIGHT: f32 = 11.0;
        const TALL_X_WIDTH: f32 = 7.0;
        const SHORT_X_FONT_SIZE: f32 = 5.0;
        const SHORT_X_HEIGHT: f32 = 5.0;
        const SHORT_X_WIDTH: f32 = 5.0;
        const PROF_TALL_OFFSET: f32 = 8.0;
        const PROF_SHORT_OFFSET: f32 = 8.0;

        macro_rules! prof_tall {
            ($page:expr, ($x:expr, $y:expr), $p:expr) => {
                match $p {
                    P::Untrained => return None,
                    P::Trained => (
                        $page,
                        ($x + (0.0 * PROF_TALL_OFFSET), $y),
                        (TALL_X_WIDTH, TALL_X_HEIGHT),
                        TALL_X_FONT_SIZE,
                    ),
                    P::Expert => (
                        $page,
                        ($x + (1.0 * PROF_TALL_OFFSET), $y),
                        (TALL_X_WIDTH, TALL_X_HEIGHT),
                        TALL_X_FONT_SIZE,
                    ),
                    P::Master => (
                        $page,
                        ($x + (2.0 * PROF_TALL_OFFSET), $y),
                        (TALL_X_WIDTH, TALL_X_HEIGHT),
                        TALL_X_FONT_SIZE,
                    ),
                    P::Legendary => (
                        $page,
                        ($x + (3.0 * PROF_TALL_OFFSET), $y),
                        (TALL_X_WIDTH, TALL_X_HEIGHT),
                        TALL_X_FONT_SIZE,
                    ),
                }
            };
        }

        macro_rules! prof_short {
            ($page:expr, ($x:expr, $y:expr), $p:expr) => {
                match $p {
                    P::Untrained => return None,
                    P::Trained => (
                        $page,
                        ($x + (0.0 * PROF_SHORT_OFFSET), $y),
                        (SHORT_X_WIDTH, SHORT_X_HEIGHT),
                        SHORT_X_FONT_SIZE,
                    ),
                    P::Expert => (
                        $page,
                        ($x + (1.0 * PROF_SHORT_OFFSET), $y),
                        (SHORT_X_WIDTH, SHORT_X_HEIGHT),
                        SHORT_X_FONT_SIZE,
                    ),
                    P::Master => (
                        $page,
                        ($x + (2.0 * PROF_SHORT_OFFSET), $y),
                        (SHORT_X_WIDTH, SHORT_X_HEIGHT),
                        SHORT_X_FONT_SIZE,
                    ),
                    P::Legendary => (
                        $page,
                        ($x + (3.0 * PROF_SHORT_OFFSET), $y),
                        (SHORT_X_WIDTH, SHORT_X_HEIGHT),
                        SHORT_X_FONT_SIZE,
                    ),
                }
            };
        }

        let (page, (x, y), (width, height), font_size) = match id {
            C::ClassDCProficiency(p) => prof_tall!(1, (138.0, 465.0), p),
            C::ACArmorProficiency(p) => prof_tall!(1, (382.0, 641.0), p),
            C::UnarmoredProficiency(p) => prof_short!(1, (291.0, 618.0), p),
            C::LightArmorProficiency(p) => prof_short!(1, (328.0, 618.0), p),
            C::MediumArmorProficiency(p) => prof_short!(1, (365.0, 618.0), p),
            C::HeavyArmorProficiency(p) => prof_short!(1, (402.0, 618.0), p),
            C::FortSaveProficiency(p) => prof_tall!(1, (251.0, 489.0), p),
            C::ReflexSaveProficiency(p) => prof_tall!(1, (331.0, 489.0), p),
            C::WillSaveProficiency(p) => prof_tall!(1, (410.0, 489.0), p),
            C::PerceptionProficiency(p) => prof_tall!(1, (529.0, 498.0), p),
            C::SkillProficiency(slot, p) => {
                // 471 426
                prof_tall!(1, (497.0, 423.0 - skill_offset(slot) as f32), p)
            }
            C::WeaponAttackProficiency(w, p) => {
                prof_tall!(1, (252.0, 394.0 - weapon_offset(w) as f32), p)
            }
            C::WeaponDamageType(DamageType::B, w) => (
                1,
                (82.5, 378.0 - weapon_offset(w) as f32),
                (SHORT_X_WIDTH, SHORT_X_HEIGHT),
                SHORT_X_FONT_SIZE,
            ),
            C::WeaponDamageType(DamageType::P, w) => (
                1,
                (82.5, 370.75 - weapon_offset(w) as f32),
                (SHORT_X_WIDTH, SHORT_X_HEIGHT),
                SHORT_X_FONT_SIZE,
            ),
            C::WeaponDamageType(DamageType::S, w) => (
                1,
                (82.5, 363.0 - weapon_offset(w) as f32),
                (SHORT_X_WIDTH, SHORT_X_HEIGHT),
                SHORT_X_FONT_SIZE,
            ),
            C::WeaponCategoryProficiency(WeaponCategory::Simple, p) => {
                prof_short!(1, (22.0, 21.0), p)
            }
            C::WeaponCategoryProficiency(WeaponCategory::Martial, p) => {
                prof_short!(1, (60.9, 21.0), p)
            }
            C::WeaponCategoryProficiency(_, _) => todo!(),
            // _ => {
            //     println!("TODO: get_checkbox_layout for CheckboxID::{:?}", id);
            //     return None;
            // }
        };

        let layout = TextLayout {
            page,
            x,
            y,
            width,
            height,
            align: TextAlign::Center,
            font_size,
        };
        // println!("Got layout info for CheckboxID::{:?}: {:?}", id, layout);
        Some(layout)
    }

    fn get_text_layout(&self, id: TextID) -> Option<TextLayout> {
        // for (i, page) in self.document.page_iter().enumerate() {
        //     println!("Looking for fonts on page {} ({:?})", i, page);
        //     for (font_key, font_dict) in self.document.get_page_fonts(page) {
        //         let font_name = String::from_utf8_lossy(&font_key);
        //         println!("Found font with key {:?}:", font_name);
        //         for (dict_key, dict_value) in font_dict.iter() {
        //             let key_name = String::from_utf8_lossy(&dict_key);
        //             println!("\t{}: {:?}", key_name, dict_value);
        //         }
        //     }
        // }

        use crate::pdf::TextID as T;
        use TextAlign::*;

        // patterns:
        //     [stat] [prof] TEML [item]
        //     |    | |           |
        //     0   23 26          86

        let (page, (x, y), (width, height), size, align) = match id {
            T::CharacterName => (1, (145, 732), (190, 20), 20, Left),
            T::PlayerName => (1, (165, 705), (170, 16), 16, Left),
            T::XP => (1, (185, 686), (150, 12), 14, Left),
            T::AncestryAndHeritage => (1, (345, 750), (180, 7), 9, Left),
            T::Background => (1, (345, 731), (180, 7), 9, Left),
            T::Class => (1, (345, 712), (180, 7), 9, Left),
            T::Size => (1, (358, 699), (10, 8), 9, Center),
            T::Alignment => (1, (376, 698), (45, 4), 5, Left),
            T::CharacterTraits => (1, (425, 698), (103, 4), 5, Left),
            T::Deity => (1, (365, 686), (160, 8), 9, Left),
            T::CharacterLevel => (1, (535, 731), (45, 20), 20, Center),
            T::HeroPoints => (1, (540, 692), (37, 20), 20, Center),
            T::Speed => (1, (80, 440), (37, 14), 14, Right),
            T::MovementNotes => (1, (155, 438), (155, 10), 12, Left),
            T::STRMod => (1, (20, 646), (34, 15), 16, Center),
            T::STRScore => (1, (165, 646), (28, 15), 16, Center),
            T::DEXMod => (1, (20, 618), (34, 15), 16, Center),
            T::DEXScore => (1, (165, 618), (28, 15), 16, Center),
            T::CONMod => (1, (20, 590), (34, 15), 16, Center),
            T::CONScore => (1, (165, 590), (28, 15), 16, Center),
            T::INTMod => (1, (20, 562), (34, 15), 16, Center),
            T::INTScore => (1, (165, 562), (28, 15), 16, Center),
            T::WISMod => (1, (20, 535), (34, 15), 16, Center),
            T::WISScore => (1, (165, 535), (28, 15), 16, Center),
            T::CHAMod => (1, (20, 507), (34, 15), 16, Center),
            T::CHAScore => (1, (165, 507), (28, 15), 16, Center),
            T::ClassDC => (1, (20, 467), (34, 15), 16, Center),
            T::ClassDCKeyAbilityBonus => (1, (86, 466), (23, 11), 12, Center),
            T::ClassDCProficiency => (1, (112, 466), (23, 11), 12, Center),
            T::ClassDCItemBonus => (1, (172, 466), (23, 11), 12, Center),
            T::TotalAC => (1, (235, 635), (35, 17), 16, Center),
            T::ACDexBonus => (1, (304, 643), (23, 11), 12, Center),
            T::ACDexCap => (1, (331, 643), (23, 11), 12, Center),
            T::ACArmorProficiency => (1, (356, 643), (23, 11), 12, Center),
            T::ACItemBonus => (1, (416, 643), (23, 11), 12, Center),
            T::ShieldAC => (1, (312, 595), (10, 11), 12, Center),
            T::ShieldHardness => (1, (335, 592), (30, 11), 12, Center),
            T::ShieldMaxHP => (1, (373, 592), (19, 11), 12, Right),
            T::ShieldBreakThreshold => (1, (397, 592), (10, 11), 12, Left),
            // T::ShieldCurrentHP => (1, (411, 592), (30, 11), 12, Center),
            T::FortSaveTotal => (1, (231, 540), (34, 15), 16, Center),
            T::FortSaveCONBonus => (1, (219, 516), (26, 11), 12, Center),
            T::FortSaveProficiency => (1, (250, 516), (26, 11), 12, Center),
            T::FortSaveItemBonus => (1, (219, 491), (26, 11), 12, Center),
            T::ReflexSaveTotal => (1, (311, 540), (34, 15), 16, Center),
            T::ReflexSaveDEXBonus => (1, (301, 516), (26, 11), 12, Center),
            T::ReflexSaveProficiency => (1, (331, 516), (26, 11), 12, Center),
            T::ReflexSaveItemBonus => (1, (301, 491), (26, 11), 12, Center),
            T::WillSaveTotal => (1, (391, 540), (34, 15), 16, Center),
            T::WillSaveWISBonus => (1, (379, 516), (26, 11), 12, Center),
            T::WillSaveProficiency => (1, (410, 516), (26, 11), 12, Center),
            T::WillSaveItemBonus => (1, (379, 491), (26, 11), 12, Center),
            T::SavingThrowNotes => (1, (240, 470), (195, 11), 12, Left),
            T::MaxHP => (1, (465, 657), (22, 12), 14, Center),
            T::ResistancesAndImmunities => (1, (467, 582), (112, 15), 12, Left),
            T::PerceptionBonus => (1, (456, 516), (28, 12), 12, Center),
            T::PerceptionWISBonus => (1, (480, 498), (23, 11), 12, Center),
            T::PerceptionProficiency => (1, (502, 498), (23, 11), 12, Center),
            T::PerceptionItemBonus => (1, (560, 498), (21, 11), 12, Center),
            T::PerceptionSenses => (1, (475, 468), (105, 15), 12, Left),
            T::SkillBonusTotal(s) => (1, (407, 425 - skill_offset(s)), (28, 11), 12, Center),
            T::SkillAbilityBonus(s) => (1, (444, 424 - skill_offset(s)), (23, 11), 12, Center),
            T::SkillProficiency(s) => (1, (471, 424 - skill_offset(s)), (23, 11), 12, Center),
            T::SkillItemBonus(s) => (1, (531, 424 - skill_offset(s)), (23, 11), 12, Center),
            T::SkillArmorPenaltyAcrobatics => (
                1,
                (561, 424 - skill_offset(SkillSlot::Acrobatics)),
                (20, 11),
                12,
                Center,
            ),
            T::SkillArmorPenaltyAthletics => (
                1,
                (561, 424 - skill_offset(SkillSlot::Athletics)),
                (20, 11),
                12,
                Center,
            ),
            T::SkillArmorPenaltyStealth => (
                1,
                (561, 424 - skill_offset(SkillSlot::Stealth)),
                (20, 11),
                12,
                Center,
            ),
            T::SkillArmorPenaltyThievery => (
                1,
                (561, 424 - skill_offset(SkillSlot::Thievery)),
                (20, 11),
                12,
                Center,
            ),
            T::LoreSkillTopic1 => (
                1,
                (321, 429 - skill_offset(SkillSlot::Lore1)),
                (55, 10),
                10,
                Left,
            ),
            T::LoreSkillTopic2 => (
                1,
                (321, 429 - skill_offset(SkillSlot::Lore2)),
                (55, 10),
                10,
                Left,
            ),
            T::WeaponName(w) => (1, (23, 398 - weapon_offset(w)), (120, 11), 10, Left),
            T::WeaponAttackBonus(w) => (1, (162, 399 - weapon_offset(w)), (15, 11), 14, Center),
            T::WeaponAttackAbilityBonus(w) => {
                (1, (200, 396 - weapon_offset(w)), (23, 11), 12, Center)
            }
            T::WeaponProficiency(w) => (1, (226, 396 - weapon_offset(w)), (23, 11), 12, Center),
            T::WeaponAttackItemBonus(w) => (1, (286, 396 - weapon_offset(w)), (23, 11), 12, Center),
            T::WeaponDamageDice(w) => (1, (22, 366 - weapon_offset(w)), (23, 11), 10, Center),
            T::WeaponDamageAbilityBonus(w) => {
                (1, (52, 366 - weapon_offset(w)), (23, 11), 10, Center)
            }
            T::WeaponDamageSpecial(w) => (1, (102, 366 - weapon_offset(w)), (23, 11), 10, Center),
            T::WeaponDamageOther(w) => (1, (131, 366 - weapon_offset(w)), (66, 11), 10, Left),
            T::WeaponTraits(w) => (1, (202, 366 - weapon_offset(w)), (108, 11), 10, Left),
            // _ => {
            //     println!("TODO: get_text_layout for TextID::{:?}", id);
            //     return None;
            // }
        };
        let layout = TextLayout {
            page,
            x: x as f32,
            y: y as f32,
            width: width as f32,
            height: height as f32,
            align,
            font_size: size as f32,
        };
        Some(layout)
    }
}

impl PDFOutput for PDF {
    fn load_empty() -> Result<Self> {
        Ok(Self {
            document: Document::load_mem(PDF_BYTES)?,
            assigned_text_fields: HashMap::new(),
            assigned_checkbox_fields: HashSet::new(),
        })
    }

    fn save<P: AsRef<std::path::Path>>(mut self, filename: P) -> Result<()> {
        // All the heavy lifting happens here to reduce allocations.
        // let page_id = self.document.page_iter().nth(0).unwrap();
        // let encodings = self
        //     .document
        //     .get_page_fonts(page_id)
        //     .into_iter()
        //     .map(|(name, font)| (name, font.get_font_encoding().to_owned()))
        //     .collect::<std::collections::BTreeMap<Vec<u8>, String>>();
        // for (font_key, font_dict) in self.document.get_page_fonts(page_id) {
        //     let font_key_str = String::from_utf8_lossy(&font_key);
        //     println!("Font {:?}: {:?}", font_key_str, font_dict);
        // }

        let mut char_widths: HashMap<u32, u16> = HashMap::new();
        {
            let scale = rusttype::Scale::uniform(1000.0);
            for c in
                " abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.,<>/?!@#$%^&*()-_=+"
                    .chars()
            {
                let key = c as u32;
                let glyph = FONT.glyph(c).scaled(scale);
                let width = glyph.h_metrics().advance_width.round().abs() as u16;
                char_widths.insert(key, width);
            }
        }
        let first_char = *char_widths.keys().min().unwrap();
        let last_char = *char_widths.keys().max().unwrap();
        let widths_array: Vec<Object> = {
            let mut ws = Vec::with_capacity((last_char - first_char) as usize);
            for key in first_char..=last_char {
                match char_widths.get(&key) {
                    Some(w) => ws.push((*w).into()),
                    None => ws.push(0.into()),
                }
            }
            ws
        };
        let font_bytes_id = self.document.add_object(
            lopdf::Stream::new(dictionary! {}, FONT_DATA_TTF.to_vec()).with_compression(false),
        );
        let widths_object_id = self.document.add_object(widths_array);
        let font_descriptor_id = self.document.add_object(dictionary! {
            "Type" => "FontDescriptor",
            "FontName" => "LesliesHand",
            "Flags" => 0,
            "FontFile2" => font_bytes_id,
        });

        let page_ids = self.document.get_pages();
        let font_obj_id = self.document.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "TrueType",
            "BaseFont" => "LesliesHand",
            "FirstChar" => first_char, // first character code defined in the Widths array
            "LastChar" => last_char, // last character code defined in the Widths array
            "Widths" => widths_object_id,
            "FontDescriptor" => font_descriptor_id,
        });
        for page_id in page_ids.values() {
            // let page_dict: &mut lopdf::Dictionary = self.get_object_mut(page_id)?.as_dict_mut()?;
            let resource_dict = self
                .document
                .get_or_create_resources(*page_id)?
                .as_dict_mut()?;
            let fonts = resource_dict.get_mut(b"Font")?.as_dict_mut()?;
            fonts.set(PLAYER_FONT_NAME, font_obj_id);
            println!("Page {:?} fonts: {:?}", page_id, fonts);
            // let fonts = resource_dict.get_mut(b"Font")?.as_dict_mut()?;
        }

        let mut content_by_page: HashMap<u32, Content> = HashMap::new();

        // let et_op = content.operations.pop().unwrap();
        // assert_eq!(&et_op.operator, "ET");
        // assert_eq!(et_op.operands.len(), 0);

        // println!("Document trailer: {:?}", self.document.trailer);

        fn write_text(content: &mut Content, text: &str, mut layout: TextLayout) {
            let (x_offset, y_offset) = layout.calc_for_text(text);
            let x = layout.x as i32 as i64;
            let y = layout.y as i32 as i64;

            let ops = vec![
                // // Draw a rectangle to describe the current box
                // Operation::new("q", vec![]),
                // Operation::new("w", vec![Object::Integer(1)]),
                // Operation::new(
                //     "m",
                //     vec![
                //         Object::Integer(x),
                //         Object::Integer(y),
                //         Object::Integer(width),
                //         Object::Integer(height),
                //     ],
                // ),
                // Operation::new("s", vec![]),
                // Operation::new("Q", vec![]),
                // Draw the actual text
                Operation::new("BT", vec![]),
                Operation::new(
                    "Tf",
                    vec![Object::Name(PLAYER_FONT_NAME.to_vec()), Object::Integer(1)],
                ),
                Operation::new(
                    "scn",
                    vec![Object::Real(0.0), Object::Real(0.0), Object::Real(0.0)],
                ),
                Operation::new(
                    "Tm",
                    vec![
                        Object::Integer(layout.font_size as u64 as i64),
                        Object::Integer(0),
                        Object::Integer(0),
                        Object::Integer(layout.font_size as u64 as i64),
                        Object::Real(x as f64 + x_offset as f64),
                        Object::Real(y as f64 + y_offset as f64),
                    ],
                ),
                Operation::new(
                    "Tj",
                    vec![Object::String(
                        Document::encode_text(None, &text),
                        lopdf::StringFormat::Literal,
                    )],
                ),
                Operation::new("ET", vec![]),
            ];
            // if let TextID::XP = id {
            //     println!("Got offset {:?} for XP layout {:?}", x_offset, layout);
            //     println!("ops: {:?}", ops);
            // }
            // println!("New ops: {:?}", ops);
            content.operations.extend(ops);
        }

        for (id, text) in self.assigned_text_fields.iter() {
            if text.is_empty() {
                // No sense clogging up the PDF with empty text boxes.
                continue;
            }
            // println!("Setting text field {:?} to text {:?}", id, text);
            let layout = match self.get_text_layout(*id) {
                Some(l) => l,
                None => continue,
            };
            let content = content_by_page
                .entry(layout.page as u32)
                .or_insert_with(|| {
                    let page_id = page_ids[&(layout.page as u32)];
                    let content_data = self.document.get_page_content(page_id).unwrap();
                    Content::decode(&content_data).unwrap()
                });

            write_text(content, text, layout);
        }

        for id in self.assigned_checkbox_fields.iter() {
            // if !checked {
            //     // No sense clogging up the PDF with empty text boxes.
            //     continue;
            // }
            let layout = match self.get_checkbox_layout(id.clone()) {
                Some(l) => l,
                None => continue,
            };
            let content = content_by_page
                .entry(layout.page as u32)
                .or_insert_with(|| {
                    let page_id = page_ids[&(layout.page as u32)];
                    let content_data = self.document.get_page_content(page_id).unwrap();
                    Content::decode(&content_data).unwrap()
                });

            write_text(content, "X", layout);
        }

        for (page_number, page_id) in page_ids {
            if let Some(content) = content_by_page.remove(&page_number) {
                let new_content_data = Content::encode(&content)?;
                self.document
                    .change_page_content(page_id, new_content_data)?;
            }
        }

        let _file = self.document.save(filename)?;
        Ok(())
    }

    fn set_text<T: fmt::Display>(&mut self, id: TextID, value: T) -> Result<()> {
        // if !self.assigned_text_fields.is_empty() {
        //     return Ok(());
        // }
        use std::fmt::Write;
        let mut s = String::new();
        write!(s, "{}", value).unwrap();
        self.assigned_text_fields.insert(id, s);
        Ok(())
    }

    fn set_check_box(&mut self, id: CheckboxID, checked: bool) -> Result<()> {
        if checked {
            self.assigned_checkbox_fields.insert(id);
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Copy, Clone, Debug)]
struct TextLayout {
    page: u8,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    align: TextAlign,
    font_size: f32,
}

impl TextLayout {
    /// Returns chunks of (x, y) offset to align text after possibly
    /// shrinking text to fit the box given. Each chunk should be
    /// rendered separately, as they represent different lines.
    fn calc_for_text<'a>(&mut self, text: &'a str) -> (f32, f32) {
        assert!(!text.is_empty());
        let start = rusttype::Point { x: 0.0, y: 0.0 };
        let target_width = self.width;
        let target_height = self.height;
        let align = self.align;

        let try_size = |font_size: f32| -> Option<(f32, f32)> {
            let scale = rusttype::Scale::uniform(font_size);
            let mut actual_width: f32 = 0.0;
            let mut actual_height: f32 = 0.0;
            for glyph_layout in FONT.layout(text, scale, start) {
                if let Some(bounding_box) = glyph_layout.pixel_bounding_box() {
                    actual_width = actual_width.max(bounding_box.max.x as f32);
                    actual_height = actual_height.max(-(bounding_box.min.y as f32));
                }
            }
            assert!(
                actual_width > 0.0,
                "Got too small actual_width {:?}",
                actual_width
            );
            assert!(
                actual_height > 0.0,
                "Got too small actual_height {:?}",
                actual_height
            );
            // println!(
            //     "For text {:?} at size {:?}, calculated width = {:?} and height = {:?}",
            //     text, font_size, actual_width, actual_height
            // );
            if actual_width <= target_width && actual_height <= target_height {
                let x_diff = target_width - actual_width;
                let x_offset = match align {
                    TextAlign::Left => 0.0,
                    TextAlign::Center => x_diff / 2.0,
                    TextAlign::Right => x_diff,
                };
                let y_offset = (target_height - actual_height) / 2.0;
                Some((x_offset, y_offset))
            } else {
                None
            }
        };

        fn bisect<F: Fn(f32) -> Option<(f32, f32)>>(
            try_size: F,
            min: u16,
            max: u16,
        ) -> (u16, (f32, f32)) {
            if let Some(offsets) = try_size(max as f32) {
                return (max, offsets);
            }

            let middle = (min + max) / 2;
            match try_size(middle as f32) {
                Some(middle_offsets) => {
                    // The real best font size must exist in [middle, self.font_size)
                    // Bisect that range
                    if middle < max - 1 {
                        bisect(try_size, middle, max - 1)
                    } else {
                        (middle, middle_offsets)
                    }
                }
                None => {
                    // The real best font size must exist between [min, middle)
                    // Bisect that range
                    if min < middle - 1 {
                        bisect(try_size, min, middle - 1)
                    } else {
                        (min, (0.0, 0.0))
                    }
                }
            }
        };

        // println!("Trying to fit text {:?} in width {}", text, target_width);
        let (new_size, offset) = bisect(try_size, 2, self.font_size.round() as u16);
        if new_size as f32 != self.font_size {
            println!(
                "Resized text layout for string {:?} from size {} to {} (align offset {:?})",
                text, self.font_size, new_size, offset
            );
            self.font_size = new_size as f32;
        }
        offset
    }
}

static FONT_DATA_TTF: &'static [u8] = include_bytes!("../resources/LesliesHand.ttf");

lazy_static! {
    static ref FONT: rusttype::Font<'static> =
        rusttype::Font::try_from_bytes(FONT_DATA_TTF).unwrap();
}
