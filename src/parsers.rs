use smallvec::{smallvec, SmallVec};
use std::convert::TryInto as _;

use crate::{resources::*, stats::*};

peg::parser! {
    grammar parsers() for str {
        rule eof() = ![_]

        // utility rules
        rule unsigned() -> u64
            = "0" { 0 }
            / n:$(['1'..='9'] ['0'..='9']*) {? n.parse().map_err(|_| "Failed to convert to u64") }

        rule ws() = quiet!{ [' ' | '\n' | '\t']+ }

        rule comma_sep() = quiet!{ "," + ws() }

        pub rule ability() -> Ability
            = ("str" / "STR") { Ability::STR }
            / ("dex" / "DEX") { Ability::DEX }
            / ("con" / "CON") { Ability::CON }
            / ("int" / "INT") { Ability::INT }
            / ("wis" / "WIS") { Ability::WIS }
            / ("cha" / "CHA") { Ability::CHA }

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

        pub rule feat_prereq() -> feat::Prereq
            = ( p:proficiency_level() ws() "in" ws() s:skill_choice() { feat::Prereq::SkillProf(s, p) })
            / ( a:ability() ws() m:unsigned()
                {?
                 match m.try_into() {
                     Ok(m) => Ok(feat::Prereq::MinAbilityScore(a, m)),
                     Err(_) => Err("Minimum ability score value is too large"),
                 }
                })

        pub rule proficiency_level() -> Proficiency
            = ( "untrained" / "Untrained" ) { Proficiency::Untrained }
            / ( "trained" / "Trained" ) { Proficiency::Trained }
            / ( "expert" / "Expert" ) { Proficiency::Expert }
            / ( "master" / "Master" ) { Proficiency::Master }
            / ( "legendary" / "Legendary" ) { Proficiency::Legendary }

        pub rule range() -> Range
            = n:unsigned() ( ws()? "ft" "."? )? {? n.try_into().map(Range).map_err(|_| "Too large of a range") }

        pub rule skill() -> Skill
            = ( "acrobatics" / "Acrobatics" ) { Skill::Acrobatics }
            / ( "arcana" / "Arcana" ) { Skill::Arcana }
            / ( "athletics" / "Athletics" ) { Skill::Athletics }
            / ( "crafting" / "Crafting" ) { Skill::Crafting }
            / ( "deception" / "Deception" ) { Skill::Deception }
            / ( "diplomacy" / "Diplomacy" ) { Skill::Diplomacy }
            / ( "intimidation" / "Intimidation" ) { Skill::Intimidation }
            / ( ("lore (" / "Lore (") topic:$(![')']+) ")" { Skill::Lore(topic.into()) })
            / ( "medicine" / "Medicine" ) { Skill::Medicine }
            / ( "nature" / "Nature" ) { Skill::Nature }
            / ( "occultism" / "Occultism" ) { Skill::Occultism }
            / ( "performance" / "Performance" ) { Skill::Performance }
            / ( "religion" / "Religion" ) { Skill::Religion }
            / ( "society" / "Society" ) { Skill::Society }
            / ( "stealth" / "Stealth" ) { Skill::Stealth }
            / ( "survival" / "Survival" ) { Skill::Survival }
            / ( "thievery" / "Thievery" ) { Skill::Thievery }

        pub rule skill_choice() -> feat::SkillChoice
            = ( "any skill" / "at least one skill" ) { feat::SkillChoice::Any }
            / s:skill() { feat::SkillChoice::Single(s) }
            / ( "lore" / "Lore" ) { feat::SkillChoice::AnyLore }

        pub rule weapon_die() -> WeaponDie
            = ( "d4" / "D4" ) { WeaponDie::D4 }
            / ( "d6" / "D6" ) { WeaponDie::D6 }
            / ( "d8" / "D8" ) { WeaponDie::D8 }
            / ( "d10" / "D10" ) { WeaponDie::D10 }
            / ( "d12" / "D12" ) { WeaponDie::D12 }

        pub rule weapon_trait() -> WeaponTrait
            = "agile" { WeaponTrait::Agile }
            / "attached" { WeaponTrait::Attached }
            / "backstabber" { WeaponTrait::Backstabber }
            / "backswing" { WeaponTrait::Backswing }
            / "deadly " d:weapon_die() { WeaponTrait::Deadly(d) }
            / "disarm" { WeaponTrait::Disarm }
            / "dwarf" { WeaponTrait::Dwarf }
            / "elf" { WeaponTrait::Elf }
            / "fatal " d:weapon_die() { WeaponTrait::Fatal(d) }
            / "finesse" { WeaponTrait::Finesse }
            / "forceful" { WeaponTrait::Forceful }
            / "free-hand" { WeaponTrait::FreeHand }
            / "gnome" { WeaponTrait::Gnome }
            / "goblin" { WeaponTrait::Goblin }
            / "grapple" { WeaponTrait::Grapple }
            / "halfling" { WeaponTrait::Halfling }
            / "jousting" { WeaponTrait::Jousting }
            / "monk" { WeaponTrait::Monk }
            / "nonleathal" { WeaponTrait::Nonleathal }
            / "orc" { WeaponTrait::Orc }
            / "parry" { WeaponTrait::Parry }
            / "propulsive" { WeaponTrait::Propulsive }
            / "reach" { WeaponTrait::Reach }
            / "shove" { WeaponTrait::Shove }
            / "sweep" { WeaponTrait::Sweep }
            / "thrown" { WeaponTrait::Thrown }
            / "trip" { WeaponTrait::Trip }
            / "twin" { WeaponTrait::Twin }
            / "two-hand " d:weapon_die() { WeaponTrait::TwoHand(d) }
            / "unarmed" { WeaponTrait::Unarmed }
            / "versatile " dt:damage_type() { WeaponTrait::Versatile(dt) }
            / "volley " r:range() { WeaponTrait::Volley(r) }
    }
}

pub use parsers::*;
