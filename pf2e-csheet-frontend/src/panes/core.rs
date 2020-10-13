use pf2e_csheet_shared::{
    choices::{Choice, ChoiceMeta},
    stats::{Alignment, Level},
    Ancestry, Background, Character, Class, HasResourceType, Heritage, ResourceRef, ResourceType,
    TypedRef,
};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use std::{fmt, rc::Rc};
use yew::prelude::*;

use crate::{resource_manager::ResourceManager, typed_select::TypedSelect, CharacterChange as CC};

pub struct CorePane {
    link: ComponentLink<Self>,
    resources: Rc<ResourceManager>,
    character: Rc<Character>,
    on_character_change: Callback<CC>,
}

#[derive(Debug)]
pub enum Msg {
    CC(CC),
    NoOp,
}

#[derive(Clone, Debug, Properties)]
pub struct Props {
    pub resources: Rc<ResourceManager>,
    pub character: Rc<Character>,
    pub on_character_change: Callback<CC>,
}

impl Component for CorePane {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            resources: props.resources,
            character: props.character,
            on_character_change: props.on_character_change,
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.resources = props.resources;
        self.character = props.character;
        self.on_character_change = props.on_character_change;
        true
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::CC(cc) => {
                self.on_character_change.emit(cc);
                true
            }
            Msg::NoOp => false,
        }
    }

    fn view(&self) -> Html {
        let ability_scores = self.view_ability_scores();
        let core_info = self.view_core_info();
        let choices = self.view_choices();
        html! {
            <div id="core-pane">
                { ability_scores }
                { choices }
                { core_info }
            </div>
        }
    }
}

impl CorePane {
    fn view_choices(&self) -> Html {
        let mut all_choices = self
            .character
            .all_choices(&*self.resources)
            .filter(|(rref, c, cm)| {
                let val_opt = if cm.character_wide {
                    self.character
                        .get_character_choice::<serde_json::Value, _>(c)
                } else {
                    self.character.get_choice::<serde_json::Value, _>(rref, c)
                };
                val_opt.is_none()
            })
            .collect::<Vec<_>>();
        all_choices.sort_by_key(|(_rref, c, cm)| (cm.kind(), c.clone()));
        let choices_rows = all_choices
            .into_iter()
            .map(|(rref, c, cm)| self.view_choice(rref, c, cm))
            .collect::<Vec<Html>>();
        let content = if choices_rows.is_empty() {
            html! { "No choices remaining" }
        } else {
            html! { <ul>{ for choices_rows }</ul> }
        };
        html! {
            <div id="choices-to-make">
                { content }
            </div>
        }
    }

    fn view_choice(&self, _rref: &ResourceRef, choice: Choice, choice_meta: ChoiceMeta) -> Html {
        html! {
            <li>{ choice }{ ": " }{ choice_meta.kind }</li>
        }
    }

    fn view_ability_score_row(&self, stat: &str) -> Html {
        let resources = &self.resources;
        let stat_value = self.character.get_modifier(stat, None, &**resources);
        let modifier = self
            .character
            .get_modifier(&format!("{} bonus", stat), None, &**resources);
        let remove_button = html! {
            <button disabled=true>{ "-" }</button>
        };
        let add_button = html! {
            <button disabled=true>{ "+" }</button>
        };
        html! {
            <tr>
                <td>{ stat }</td>
                <td style="font-weight: bold;">{ modifier }</td>
                <td>{ stat_value.as_score() }</td>
                <td>{ remove_button }</td>
                <td>{ add_button }</td>
                </tr>
        }
    }

    fn view_ability_scores(&self) -> Html {
        html! {
            <table class="ability-scores">
                <thead>
                    <tr>
                        <th>{ "Ability" }</th>
                        <th>{ "Bonus" }</th>
                        <th>{ "Value" }</th>
                        <th></th>
                        <th></th>
                    </tr>
                </thead>
                <tbody>
                    { self.view_ability_score_row("STR") }
                    { self.view_ability_score_row("DEX") }
                    { self.view_ability_score_row("CON") }
                    { self.view_ability_score_row("INT") }
                    { self.view_ability_score_row("WIS") }
                    { self.view_ability_score_row("CHA") }
                </tbody>
            </table>
        }
    }

    fn view_alignment_row(&self) -> Html {
        let alignments = vec![
            Alignment::ChaoticGood,
            Alignment::NeutralGood,
            Alignment::LawfulGood,
            Alignment::ChaoticNeutral,
            Alignment::Neutral,
            Alignment::LawfulNeutral,
            Alignment::ChaoticEvil,
            Alignment::NeutralEvil,
            Alignment::LawfulEvil,
        ];
        let selected = self.character.get_character_choice("Alignment");
        let onselect = |alignment_opt| match alignment_opt {
            Some(a) => match serde_json::to_value(&a) {
                Ok(value) => Msg::CC(CC::SetCoreChoice("Alignment".into(), value)),
                Err(_e) => Msg::NoOp,
            },
            None => Msg::CC(CC::RemoveCoreChoice("Alignment".into())),
        };
        html! {
            <tr>
                <th>{ "Alignment:" }</th>
                <td>
                    <TypedSelect::<Alignment>
                         choices=alignments
                         selected=selected
                         onselect=self.link.callback(onselect)
                    />
                </td>
            </tr>
        }
    }

    fn view_singleton_resource_row<R>(
        &self,
        label: &str,
        current: Option<TypedRef<R>>,
        options: Option<Vec<TypedRef<R>>>,
    ) -> Html
    where
        R: Clone + fmt::Debug + HasResourceType + PartialEq + 'static,
    {
        let current = current.map(DW);
        let all_rrefs: Vec<TypedRef<R>> = options.unwrap_or_else(|| {
            self.resources
                .all_by_type_immediate(R::RESOURCE_TYPE)
                .into_iter()
                .filter_map(|rref| rref.as_typed().ok())
                .collect()
        });
        let all_rrefs: Vec<DW<TypedRef<R>>> = all_rrefs.into_iter().map(DW).collect();
        let onselect = |rref: Option<DW<TypedRef<R>>>| {
            Msg::CC(CC::SetSingletonResource(
                R::RESOURCE_TYPE,
                rref.map(|dw| dw.0.as_runtime()),
            ))
        };
        html! {
            <tr>
                <th>{ label }</th>
                <td>
                    <TypedSelect::<DW<TypedRef<R>>>
                         choices=all_rrefs
                         selected=current
                         onselect=self.link.callback(onselect)
                    />
                </td>
            </tr>
        }
    }

    fn view_ancestry_heritage_rows(&self) -> (Html, Html) {
        let current_ancestry: Option<TypedRef<Ancestry>> = self
            .character
            .resources
            .iter()
            .filter_map(|rref| {
                if rref.resource_type == Some(ResourceType::Ancestry) {
                    rref.clone().as_typed().ok()
                } else {
                    None
                }
            })
            .next();
        let heritage_row = match current_ancestry.as_ref() {
            Some(_a) => html! { <tr><th>{ "Heritage:" }</th><td><span>{ "TODO" }</span></td></tr> },
            None => {
                // TODO
                let current = None;
                self.view_singleton_resource_row::<Heritage>("Heritage:", current, None)
            }
        };
        let ancestry_row = self.view_singleton_resource_row("Ancestry:", current_ancestry, None);
        (ancestry_row, heritage_row)
    }

    fn view_background_row(&self) -> Html {
        let current: Option<TypedRef<Background>> = self
            .character
            .resources
            .iter()
            .filter_map(|rref| {
                if rref.resource_type == Some(ResourceType::Background) {
                    rref.clone().as_typed().ok()
                } else {
                    None
                }
            })
            .next();
        self.view_singleton_resource_row("Background:", current, None)
    }

    fn view_class_row(&self) -> Html {
        let current: Option<TypedRef<Class>> =
            self.character.get_class_and_level().map(|(c, _l)| c);
        self.view_singleton_resource_row("Class:", current, None)
    }

    fn view_core_info(&self) -> Html {
        let info_row = |label: &str, selector: Html| -> Html {
            html! {
                <tr>
                    <th>{ label }{ ":" }</th>
                    <td>{ selector }</td>
                </tr>
            }
        };
        let player_name = info_row(
            "Player Name",
            html! {
                <input type="text"
                     oninput=self.link.callback(|e: InputData| Msg::CC(CC::SetPlayerName(e.value.into())))
                     value=self.character.player_name.as_str()
                 />
            },
        );
        let character_name = info_row(
            "Character Name",
            html! {
                <input type="text"
                     oninput=self.link.callback(|e: InputData| Msg::CC(CC::SetName(e.value.into())))
                     value=self.character.name.as_str()
                 />
            },
        );
        let alignment = self.view_alignment_row();
        let deity = info_row(
            "Deity",
            html! {
                <input type="text" value="TODO" />
            },
        );
        let (ancestry, heritage) = self.view_ancestry_heritage_rows();
        let background = self.view_background_row();
        let level = {
            let (rref, level): (Option<ResourceRef>, u8) =
                match self.character.get_class_and_level() {
                    Some((rref, level)) => (Some(rref.as_runtime()), level.get()),
                    None => (None, 1),
                };
            let level_change_callback = |level| {
                let rref = rref.clone();
                move |_| {
                    let rref = rref.as_ref().unwrap().clone();
                    let value = serde_json::to_value(&Level::from(level)).unwrap();
                    Msg::CC(CC::SetChoice(rref, "Level".into(), value))
                }
            };
            let button_minus = html! {
                <button type="button" style="flex: 1 0;"
                        disabled=(rref.is_none() || level <= 1)
                        onclick=self.link.callback(level_change_callback(level - 1))
                 >{ "Remove" }</button>
            };
            let button_plus = html! {
                <button type="button" style="flex: 1 0;"
                        disabled=(rref.is_none() || level >= 20)
                        onclick=self.link.callback(level_change_callback(level + 1))
                 >{ "Add" }</button>
            };
            html! {
                <tr>
                    <th>{ "Level:" }</th>
                    <td style="display: flex;">
                        { button_minus }
                        <span style="flex: 1 1; text-align: center;">{ level }</span>
                        { button_plus }
                    </td>
                </tr>
            }
        };
        html! {
            <div class="core-info">
                <table>
                    { player_name }
                    { character_name }
                    { alignment }
                    { deity }
                    <tr><td colspan=2 class="divider" /></tr>
                    { ancestry }
                    { heritage }
                    { background }
                    <tr><td colspan=2 class="divider" /></tr>
                    { self.view_class_row() }
                    { level }
                </table>
            </div>
        }
    }
}

trait DisplayWrapper {
    fn display(&self) -> String;
}

#[repr(transparent)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(transparent)]
struct DW<T: DisplayWrapper>(T);

impl<T: DisplayWrapper> fmt::Display for DW<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = self.0.display();
        write!(f, "{}", s)
    }
}

impl<R: HasResourceType> DisplayWrapper for TypedRef<R> {
    fn display(&self) -> String {
        self.name.clone()
    }
}

impl DisplayWrapper for ResourceRef {
    fn display(&self) -> String {
        self.name.clone()
    }
}
