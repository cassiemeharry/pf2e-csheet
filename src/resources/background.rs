use serde::Deserialize;
use smallvec::{smallvec, SmallVec};
use smartstring::alias::String;
use std::{fmt, fmt::Write as _};

use crate::{
    bonuses::{Bonus, HasModifiers, Modifier, Modifies},
    qa::{Answer, Question, QuestionOption},
    resources::refs::Ref,
    resources::{Character, Feat, Resource},
    stats::{Ability, AbilityBoost, Skill},
};

#[derive(Clone, Debug, Deserialize)]
pub struct Background {
    name: String,
    description: String,
    ability_boosts: [AbilityBoost; 2],
    skill_feat: Ref<Feat>,
    #[serde(default)]
    trained_lore: String,
    trained_skill: Skill,
}

impl Resource for Background {
    fn get_index_value(&self, _extra: &()) -> String {
        self.name.clone()
    }

    fn get_questions(&self) -> Vec<Question> {
        let mut qs = Vec::with_capacity(3);
        if self.trained_lore.is_empty() {
            qs.push(Question {
                label: "Background lore topic".into(),
                tag: "topic".into(),
                looking_for: QuestionOption::LoreTopic { only_new: true },
            });
        }

        let mut boost_tags: SmallVec<[&'static str; 2]> = smallvec!["boost_2", "boost_1",];

        let mut add_boost_q = |choices: SmallVec<[Ability; 2]>| {
            let tag = boost_tags.pop().unwrap();
            qs.push(Question {
                label: crate::format!("Background ability boost #{}", 2 - boost_tags.len()),
                tag: tag.into(),
                looking_for: QuestionOption::Ability { options: choices },
            });
        };

        for boost in self.ability_boosts.iter() {
            match boost {
                AbilityBoost::Choice(choices) => {
                    add_boost_q(choices.into_iter().copied().collect())
                }
                AbilityBoost::Fixed(_) => (),
                AbilityBoost::Free => add_boost_q(Ability::iter_all().collect()),
            }
        }

        qs
    }
}

impl fmt::Display for Background {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl HasModifiers for Background {
    fn get_modifier(&self, _c: &Character, m: Modifies) -> Modifier {
        let bonus = match (self, &m) {
            // TODO
            (_, _) => Bonus::none(),
        };
        Modifier::new() + bonus
    }
}
