use smallvec::{smallvec, SmallVec};
use std::convert::{TryFrom as _, TryInto as _};

use crate::{bonuses::*, calc::*, common::*, stats::*};

peg::parser! {
    grammar parsers() for str {
        rule eof() = ![_]

        // utility rules
        rule unsigned() -> u64
            = "0" { 0 }
            / n:$(['1'..='9'] ['0'..='9']*) {? n.parse().map_err(|_| "Failed to convert to u64") }

        rule positive() -> u64
            = "+"? n:unsigned() { n }

        rule ws() = quiet!{ [' ' | '\n' | '\t']+ }

        rule comma_sep() = quiet!{ "," + ws() }

        pub rule ability() -> Ability
            = ("strength" / "STRENGTH" / "str" / "STR") { Ability::STR }
            / ("dexterity" / "DEXTERITY" / "dex" / "DEX") { Ability::DEX }
            / ("constitution" / "CONSTITUTION" / "con" / "CON") { Ability::CON }
            / ("intelligence" / "INTELLIGENCE" / "int" / "INT") { Ability::INT }
            / ("wisdom" / "WISDOM" / "wis" / "WIS") { Ability::WIS }
            / ("charisma" / "CHARISMA" / "cha" / "CHA") { Ability::CHA }

        pub rule ability_boost() -> AbilityBoost
            = ( "free" / "any" ) { AbilityBoost::Free }
            / ( x:ability() ws()+ "or" ws()+ y:ability() { AbilityBoost::Choice(smallvec![x, y]) } )
            / ( first:ability() middle:(comma_sep() a:ability() { a })+ comma_sep() "or" ws() last:ability()
              {
                  let mut v = SmallVec::with_capacity(middle.len() + 2);
                  v.push(first);
                  v.extend(middle);
                  v.push(last);
                  v.sort();
                  AbilityBoost::Choice(v)
              }
            )
            / x:ability() { AbilityBoost::Fixed(x) }

        pub rule bonus_type() -> BonusType
            = "circumstance" { BonusType::Circumstance }
            / "item" { BonusType::Item }
            / "proficiency" { BonusType::Proficiency }
            / "status" { BonusType::Status }
            / "untyped" { BonusType::Untyped }

        pub rule bonus() -> Bonus
            = b:positive() bt_opt:( " " bt:bonus_type() { bt })?
              {? b.try_into().map_err(|_| "Bonus value is too large")
                 .and_then(|value| {
                     Bonus::from_value_type(value, &bt_opt.unwrap_or(BonusType::Untyped))
                         .map_err(|_| "Failed to create bonus")
                 })
              }

        pub rule penalty() -> Penalty
            = "-" p:unsigned() bt_opt:( " " bt:bonus_type() { bt })?
              {? i16::try_from(p).map_err(|_| "Penalty value is too large")
                 .and_then(|value| {
                     Penalty::from_value_type(-value, &bt_opt.unwrap_or(BonusType::Untyped))
                         .map_err(|_| "Failed to create penalty")
                 })
              }

        pub rule modifier() -> Modifier
            = p:penalty() { p.into() }
            / b:bonus() { b.into() }

        rule calculation_terminal() -> Calculation
            = "$" var:$(['a'..='z' | 'A'..='Z'] ['a'..='z' | 'A'..='Z' | '_']+) { Calculation::Choice(var.into()).normalized() }
            / name:$(['a'..='z' | 'A'..='Z'] ['a'..='z' | 'A'..='Z' | '_']+) { Calculation::Named(name.into()).normalized() }
            / m:modifier() { Calculation::Modifier(m).normalized() }

        pub rule calculation() -> Calculation = precedence! {
            x:(@) "+" y:@ { Calculation::Op(Op::Add, vec![x, y]).normalized() }
            --
            ws()* term:calculation_terminal() ws()* { term.normalized() }
            ws()* "(" ws()* inner:calculation() ws()* ")" ws()* { inner.normalized() }
        }


        rule currency_cp() -> Gold
            = c:unsigned() ws()? "cp" {? c.try_into().map(Gold::cp).map_err(|_| "Copper value is out of range") }

        rule currency_sp() -> Gold
            = s:unsigned() ws()? "sp" {? s.try_into().map(Gold::sp).map_err(|_| "Silver value is out of range") }

        rule currency_gp() -> Gold
            = g:unsigned() ws()? "gp" {? g.try_into().map(Gold::gp).map_err(|_| "Gold value is out of range") }

        rule currency_pp() -> Gold
            = p:unsigned() ws()? "pp" {? p.try_into().map(Gold::pp).map_err(|_| "Platinum value is out of range") }

        pub rule currency() -> Gold
            = ( p:currency_pp()
                gsc:(gsc_opt:( ws() g:currency_gp()
                      sc:(sc_opt:( ws() s:currency_sp()
                           c:(c_opt:( ws() c:currency_cp() { c })? { c_opt.unwrap_or(Gold::zero()) })
                           { s + c }
                      )? { sc_opt.unwrap_or(Gold::zero()) })
                      { g + sc }
                )? { gsc_opt.unwrap_or(Gold::zero()) })
                { p + gsc })
            / ( g:currency_gp()
                sc:(sc_opt:( ws() s:currency_sp()
                             c:(c_opt:( ws() c:currency_cp() { c })? { c_opt.unwrap_or(Gold::zero()) })
                             { s + c }
                )? { sc_opt.unwrap_or(Gold::zero()) })
                { g + sc })
            / ( s:currency_sp()
                c:(c_opt:( ws() c:currency_cp() { c })? { c_opt.unwrap_or(Gold::zero()) })
                { s + c })
            / c:currency_cp() { c }

        pub rule damage_type() -> DamageType
            = ( "B" / "bludgeoning" ) { DamageType::B }
            / ( "P" / "piercing" ) { DamageType::P }
            / ( "S" / "slashing" ) { DamageType::S }

        // pub rule feat_prereq() -> feat::Prereq
        //     = ( p:proficiency_level() ws() "in" ws() s:skill_choice() { feat::Prereq::SkillProf(s, p) })
        //     / ( a:ability() ws() m:unsigned()
        //         {?
        //          match m.try_into() {
        //              Ok(m) => Ok(feat::Prereq::MinAbilityScore(a, m)),
        //              Err(_) => Err("Minimum ability score value is too large"),
        //          }
        //         })

        pub rule proficiency_level() -> Proficiency
            = ( "untrained" / "Untrained" ) { Proficiency::Untrained }
            / ( "trained" / "Trained" ) { Proficiency::Trained }
            / ( "expert" / "Expert" ) { Proficiency::Expert }
            / ( "master" / "Master" ) { Proficiency::Master }
            / ( "legendary" / "Legendary" ) { Proficiency::Legendary }

        pub rule range() -> Range
            = n:positive() ( ws()? "ft" "."? )? {? n.try_into().map(Range).map_err(|_| "Too large of a range") }


        rule resource_ref_name() -> &'input str
            = n:$( ['a'..='z' | 'A'..='Z' ]
                   ['a'..='z' | 'A'..='Z' | '-' | '_' | ' ' | '/' | '\'' | '’' ]* ) { n.trim() }
            / expected!("resource name")

        rule resource_ref_modifier() -> &'input str
            = quiet!{ " "* } "("
              m:$( ['a'..='z' | 'A'..='Z' | '0'..='9' | '+' | '-' ]
                   ['a'..='z' | 'A'..='Z' | '0'..='9' | '+' | '-' | '_' | ' ' ]*) ")"
              { m.trim() }
            / expected!("resource modifier surrounded by parentheses")

        rule resource_ref_type() -> ResourceType
            = quiet!{ " "* } "[" rt:resource_type() "]" { rt }
            / expected!("resource type surrounded by square brackets")

        pub rule resource_ref() -> ResourceRef
            = ( name:resource_ref_name()
                modifier:resource_ref_modifier()?
                rtype:resource_ref_type()?
                { ResourceRef::new(name.trim(), modifier).with_type(rtype) }
              )
            / expected!("resource reference")

        pub rule resource_type() -> ResourceType
            = "action" { ResourceType::Action }
            / "ancestry" { ResourceType::Ancestry }
            / "background" { ResourceType::Background }
            / ("class feature" / "class-feature") { ResourceType::ClassFeature }
            / "class" { ResourceType::Class }
            / "feat" { ResourceType::Feat }
            / "heritage" { ResourceType::Heritage }
            / "item" { ResourceType::Item }
            / "spell" { ResourceType::Spell }

        pub rule skill() -> Skill
            = ( "acrobatics" / "Acrobatics" ) { Skill::Acrobatics }
            / ( "arcana" / "Arcana" ) { Skill::Arcana }
            / ( "athletics" / "Athletics" ) { Skill::Athletics }
            / ( "crafting" / "Crafting" ) { Skill::Crafting }
            / ( "deception" / "Deception" ) { Skill::Deception }
            / ( "diplomacy" / "Diplomacy" ) { Skill::Diplomacy }
            / ( "intimidation" / "Intimidation" ) { Skill::Intimidation }
            / ( ("lore (" / "Lore (") topic:$(['a'..='z' | 'A'..='Z' | '0'..='9' | ' ' | '-']+) ")" { Skill::Lore(topic.into()) })
            / ( "medicine" / "Medicine" ) { Skill::Medicine }
            / ( "nature" / "Nature" ) { Skill::Nature }
            / ( "occultism" / "Occultism" ) { Skill::Occultism }
            / ( "performance" / "Performance" ) { Skill::Performance }
            / ( "religion" / "Religion" ) { Skill::Religion }
            / ( "society" / "Society" ) { Skill::Society }
            / ( "stealth" / "Stealth" ) { Skill::Stealth }
            / ( "survival" / "Survival" ) { Skill::Survival }
            / ( "thievery" / "Thievery" ) { Skill::Thievery }
    }
}

pub use parsers::*;
