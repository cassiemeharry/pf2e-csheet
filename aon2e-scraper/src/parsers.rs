use pf2e_csheet_shared::{
    calc::CalculatedString,
    choices::{Choice, ChoiceMeta},
    cond::{Condition, ProficiencyCondition, SingleCondition, UnenforcedCondition},
    effects::{self, Effect},
    items::ArmorCategory,
    stats::Proficiency,
    ResourceRef,
};
use smallvec::{smallvec, SmallVec};
use smartstring::alias::String;
use std::str::FromStr as _;

mod nlp;

pub use nlp::SEGMENTER;

#[derive(Debug, Default, PartialEq)]
pub struct ClassFeatureDescParsed {
    pub traits: Vec<String>,
    pub description: Option<CalculatedString>,
    pub choices: Vec<(Choice, ChoiceMeta)>,
    pub effects: Vec<Effect>,
}

#[allow(unused)]
impl ClassFeatureDescParsed {
    fn new() -> Self {
        Self::default()
    }

    fn add_trait(mut self, t: String) -> Self {
        self.traits.push(t);
        self
    }

    fn set_desc(mut self, desc: impl AsRef<str>) -> Self {
        self.description = match CalculatedString::from_str(desc.as_ref()) {
            Ok(cs) => Some(cs),
            Err(e) => {
                warn!("Failed to parse description into CalculatedString: {}\n--------------------\n{}\n--------------------", e, desc.as_ref());
                None
            }
        };
        self
    }

    fn append_desc(mut self, desc: impl AsRef<str>) -> Self {
        todo!()
    }

    fn add_choice(mut self, name: impl Into<Choice>, meta: ChoiceMeta) -> Self {
        self.choices.push((name.into(), meta));
        self
    }

    fn add_effect(mut self, effect: impl Into<Effect>) -> Self {
        self.effects.push(effect.into());
        self
    }
}

impl std::ops::Add for ClassFeatureDescParsed {
    type Output = Self;

    fn add(mut self, other: Self) -> Self {
        self.traits.extend(other.traits);
        self.description = match (self.description, other.description) {
            (None, None) => None,
            (Some(d), None) => Some(d),
            (None, Some(d)) => Some(d),
            (Some(x), Some(y)) => Some(x.concat(y)),
        };
        self.choices.extend(other.choices);
        self.effects.extend(other.effects);
        self
    }
}

// pub struct SentenceTreePosition {
//     indexes: Vec<usize>,
// }

// impl fmt::Display for SentenceTreePosition {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         fmt::Debug::fmt(&self.indexes, f)
//     }
// }

// impl peg::Parse for SentenceTree {
//     type PositionRepr = SentenceTreePosition;

//     fn start<'input>(&'input self) -> usize {
//         todo!()
//     }

//     fn position_repr<'input>(&'input self, offset: usize) -> Self::PositionRepr {
//         todo!()
//     }
// }

peg::parser! {
    grammar parsers() for str {
        rule _() = quiet!{ [' ']+ / ![_] }
        rule newline() = quiet!{ ['\n']+ / ![_] }
        rule __() = quiet!{ [' ' | '\n']* }

        pub rule class_feature() -> ClassFeatureDescParsed
            = parts:class_feature_part()+
              { parts.into_iter().fold(
                    ClassFeatureDescParsed::new(),
                    |existing, next| existing + next,
                )
              }

        rule class_feature_part() -> ClassFeatureDescParsed
            = proficiency_ranks()
            / c:$([_]+) { ClassFeatureDescParsed::new().set_desc(c) }

        pub rule proficiency_ranks() -> ClassFeatureDescParsed
            = s1:$(("Y" / "y") "our" _ "proficiency" _ "rank" "s"? _ "for" _) ts:proficiency_targets() s2:$(_ "increase" "s"? _ "to" _) level:proficiency_level()
              { let mut targets_name = String::new();
                for (i, t) in ts.iter().enumerate() {
                    if ts.len() > 2 && (i + 1) == ts.len() {
                        targets_name.push_str(", and ");
                        targets_name.push_str(&*t);
                    } else if i > 0 {
                        targets_name.push_str(", ");
                        targets_name.push_str(&*t);
                    } else {
                        targets_name.push_str(&*t);
                    }
                }
                let desc = format!("{}{}{}{:?}", s1, targets_name, s2, level);
                let mut cfdp = ClassFeatureDescParsed::new().set_desc(desc);
                for t in ts {
                    cfdp = cfdp.add_effect(effects::IncreaseProficiencyEffect {
                      common: effects::EffectCommon::default(),
                      target: t.into(),
                      level,
                    });
                }
                cfdp
              }

        pub rule proficiency_targets() -> SmallVec<[&'input str; 3]>
            = t_first:proficiency_target()
              ts:("," _ t:proficiency_target() { t })*
              t_last:(","? _ "and" _ t:proficiency_target() { t })?
              { let mut output: SmallVec<[&'input str; 3]> = t_first;
                output.extend(ts.into_iter().flatten());
                output.extend(t_last.into_iter().flatten());
                output
              }

        pub rule proficiency_target() -> SmallVec<[&'input str; 3]>
            = "Fortitude saves" { smallvec!["FORT"] }
            / "Reflex saves" { smallvec!["REF"]}
            / "Will saves" { smallvec!["WILL"] }
            / s:$("simple and martial weapons") { smallvec!["simple weapons", "martial weapons"] }
            / s:$(['a'..='z' | 'A'..='Z' | ' ']+ &" increase") { smallvec![s.trim()] }
            / s:$( (!(_ ("," / "and" / "increase")) [_])+ ) { smallvec![s.trim()] }

        rule proficiency_level() -> Proficiency
            = ( "untrained" / "Untrained" ) { Proficiency::Untrained }
            / ( "trained" / "Trained" ) { Proficiency::Trained }
            / ( "expert" / "Expert" ) { Proficiency::Expert }
            / ( "master" / "Master" ) { Proficiency::Master }
            / ( "legendary" / "Legendary" ) { Proficiency::Legendary }

        pub rule conditions() -> Condition
            = c1:condition() _ "and" _ c2:condition() __"."? ![_] { c1 & c2 }
            / cs:(condition() ** ("," _)) __ "."? ![_]
              { let mut conds = Condition::None;
                for c in cs {
                    conds &= c;
                }
                conds
              }

        pub rule condition() -> Condition
            = at_least:proficiency_level() _ "in" _ s:$((![','] [_])+)
              { let pc = ProficiencyCondition::AtLeast {
                  target: s.into(),
                  at_least,
                };
                pc.into()
              }
            / you() _ "are" _ "unarmored" { ArmorCategory::Unarmored.into() }
            / s:$(you() _ "are" _ "in" (_ !"Stance" word())+ _ "Stance") { UnenforcedCondition::known(s).into() }
            / s:$((!("," / "." / "and") [_])+)
              { let mut title_words = 0;
                let mut lower_words = 0;
                for word in s.split_whitespace() {
                    match word.chars().next().map(|c| c.is_uppercase()) {
                        Some(true) | None => title_words += 1,
                        Some(false) => lower_words += 1,
                    }
                }
                let is_titlecase = (lower_words * 2) <= title_words;
                if is_titlecase {
                    SingleCondition::HaveResource(ResourceRef::new(s, None::<&str>)).into()
                } else {
                    let s = String::from(s);
                    s.into()
                }
              }

        rule you() = "You" / "you"
        rule word() -> &'input str = titlecase_word() / lowercase_word()
        rule titlecase_word() -> &'input str
            = $(['A'..='Z'] ['a'..='z']*)
        rule lowercase_word() -> &'input str
            = $(['a'..='z']+)
    }
}

pub use self::parsers::*;

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    fn display_lines(s: &str) {
        let mut lines = 0;
        let mut longest_line = 0;
        for line in s.lines() {
            lines += 1;
            longest_line = longest_line.max(line.len());
        }
        if lines == 0 {
            return;
        }
        let left_width = (((lines + 1) as f32).log10().ceil() as usize).max(1);
        println!(
            "(lines = {:?}, longest_line = {:?}, left_width = {:?})",
            lines, longest_line, left_width
        );
        if longest_line > 0 {
            println!("{:-^len$}", "-", len = (left_width + 3 + longest_line));
            print!("{:width$} | ", "", width = left_width);
            for i in 1..(longest_line + 1) {
                if i % 10 == 0 {
                    print!("{}", i / 10);
                } else {
                    print!(" ");
                }
            }
            println!("");
            print!("{:width$} | ", "", width = left_width);
            for i in 1..(longest_line + 1) {
                print!("{}", i % 10);
            }
            println!("");
        }
        println!("{:-^len$}", "-", len = (left_width + 3 + longest_line));
        for (i, line) in s.lines().enumerate() {
            println!("{:>width$} | {}", i + 1, line, width = left_width);
        }
        println!("{:-^len$}", "-", len = (left_width + 3 + longest_line));
    }

    #[test]
    fn test_parse_desc_proficiency_targets() {
        let text = "simple and martial weapons and unarmed attacks";
        let expected = &["simple weapons", "martial weapons", "unarmed attacks"];
        let actual = proficiency_targets(text).unwrap();
        assert_eq!(expected, actual.as_slice());
    }

    #[test]
    fn test_parse_desc_proficiency() {
        let desc = "Your proficiency ranks for simple and martial weapons and unarmed attacks increases to expert.";
        display_lines(desc);
        let parsed = class_feature(desc).unwrap();
        let expected = ClassFeatureDescParsed::new()
            .set_desc("Your proficiency ranks for simple weapons, martial weapons, and unarmed attacks increases to Expert.")
            .add_effect(
                effects::IncreaseProficiencyEffect {
                    common: effects::EffectCommon::default(),
                    target: "simple weapons".into(),
                    level: Proficiency::Expert,
                },
            )
            .add_effect(
                effects::IncreaseProficiencyEffect {
                    common: effects::EffectCommon::default(),
                    target: "martial weapons".into(),
                    level: Proficiency::Expert,
                },
            )
            .add_effect(
                effects::IncreaseProficiencyEffect {
                    common: effects::EffectCommon::default(),
                    target: "unarmed attacks".into(),
                    level: Proficiency::Expert,
                },
            );
        assert_eq!(expected, parsed);
    }

    #[test]
    fn test_prerequisites() {
        let positive_examples: &[(&'static str, Condition)] = &[
            (
                "trained in Deception",
                ProficiencyCondition::AtLeast {
                    target: "Deception".into(),
                    at_least: Proficiency::Trained,
                }
                .into(),
            ),
            (
                "You are in Monastic Archer Stance and wielding a bow usable with that stance.",
                Condition::And(vec![
                    UnenforcedCondition::known("You are in Monastic Archer Stance").into(),
                    UnenforcedCondition::unknown("wielding a bow usable with that stance").into(),
                ]),
            ),
        ];
        for (input, expected) in positive_examples.iter() {
            display_lines(input);
            let actual = conditions(input).unwrap();
            assert_eq!(expected, &actual);
        }
    }
}
