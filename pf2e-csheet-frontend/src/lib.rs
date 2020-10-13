#![recursion_limit = "256"]

#[macro_use]
extern crate log;

#[macro_use(format)]
extern crate pf2e_csheet_shared;

use pf2e_csheet_shared::{choices::Choice, Character, ResourceRef, ResourceType};
use serde_json::Value;
use smartstring::alias::String;
use std::{collections::HashMap, rc::Rc};
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use yew::{
    prelude::*,
    services::storage::{Area, StorageService},
};

// mod icons;
mod panes;
mod resource_manager;
mod typed_select;

use resource_manager::ResourceManager;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Pane {
    Debug,
    SaveAndLoad,
    Core,
    Feats,
    Skills,
    Equipment,
    Spells,
}

impl Pane {
    fn label(&self) -> &'static str {
        match self {
            Self::Debug => "Debug",
            Self::SaveAndLoad => "Save & Load",
            Self::Core => "Core",
            Self::Feats => "Feats",
            Self::Skills => "Skills",
            Self::Equipment => "Equipment",
            Self::Spells => "Spells",
        }
    }

    fn show_for_character(&self, character: Option<&Character>) -> bool {
        match self {
            Self::Debug => character.is_some(),
            Self::SaveAndLoad => true,
            Self::Core => character.is_some(),
            Self::Feats => character.is_some(),
            Self::Skills => character.is_some(),
            Self::Equipment => character.is_some(),
            // TODO: only show this for spellcasters
            Self::Spells => false,
        }
    }
}

pub struct App {
    link: ComponentLink<Self>,
    character: Option<Rc<Character>>,
    current_pane: Pane,
    resource_manager: Rc<ResourceManager>,
    storage: StorageService,
}

#[derive(Debug)]
pub enum CharacterChange {
    SetChoice(ResourceRef, Choice, Value),
    SetCoreChoice(Choice, Value),
    RemoveChoice(ResourceRef, Choice),
    RemoveCoreChoice(Choice),
    SetSingletonResource(ResourceType, Option<ResourceRef>),
    SetName(String),
    SetPlayerName(String),
}

#[derive(Clone, Properties)]
pub struct AppProps {
    pub url_base: url::Url,
}

#[derive(Debug)]
pub enum Msg {
    CharacterChange(CharacterChange),
    ResourcesUpdated { pending: usize },
    SwitchPane(Pane),
    DeleteCharacter(Uuid),
    LoadCharacter(Rc<Character>),
    UnloadCharacter,
}

impl Component for App {
    type Message = Msg;
    type Properties = AppProps;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let update_callback = link.callback(|pending: usize| Msg::ResourcesUpdated { pending });
        let resource_manager = Rc::new(ResourceManager::new(props.url_base, update_callback));
        let storage = StorageService::new(Area::Local).unwrap();
        let mut app = Self {
            link,
            character: None,
            current_pane: Pane::SaveAndLoad,
            resource_manager,
            storage,
        };
        let last_loaded_id = app
            .storage
            .restore::<Result<std::string::String, _>>("last_loaded_char_id")
            .ok()
            .and_then(|raw| {
                raw.parse()
                    .map_err(|e| {
                        warn!("Failed to parse character ID {:?} for auto-loading", e);
                    })
                    .ok()
            });
        if let Some(id) = last_loaded_id {
            debug!("Auto-loading character {:?}", id);
            app.load_character(id);
        }
        app
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        true
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        use CharacterChange as CC;
        match msg {
            Msg::CharacterChange(cc) => {
                let mut save = None;
                if let Some(c_rc) = self.character.as_mut() {
                    let c: &mut Character = Rc::make_mut(c_rc);
                    debug!("Applying change {:?} to character", cc);
                    match cc {
                        CC::SetChoice(rref, choice, value) => {
                            if let Err(e) = c.set_choice(&rref, choice.clone(), &value) {
                                error!(
                                    "Failed to set {} choice {} to {}: {}",
                                    rref, choice, value, e
                                );
                            }
                        }
                        CC::SetCoreChoice(choice, value) => {
                            if let Err(e) = c.set_character_choice(choice.clone(), &value) {
                                error!(
                                    "Failed to set character choice {} to {}: {}",
                                    choice, value, e
                                );
                            }
                        }
                        CC::RemoveChoice(rref, choice) => {
                            c.remove_choice(&rref, &choice);
                        }
                        CC::RemoveCoreChoice(choice) => {
                            c.remove_character_choice(&choice);
                        }
                        CC::SetSingletonResource(rtype, rref_opt) => {
                            c.resources.retain(|cr| cr.resource_type != Some(rtype));
                            if let Some(rref) = rref_opt {
                                self.resource_manager.ensure_one(&rref);
                                c.resources.insert(rref);
                            }
                        }
                        CC::SetName(new_name) => c.name = new_name,
                        CC::SetPlayerName(new_name) => c.player_name = new_name,
                    }
                    c.normalize_resources(&*self.resource_manager);
                    save = Some(c.id);
                }
                if let Some(id) = save {
                    let existing = self.storage.load_characters();
                    self.save_characters(existing);
                    self.load_character(id);
                }
                true
            }
            Msg::ResourcesUpdated { pending: _ } => {
                if let Some(c_rc) = self.character.as_mut() {
                    let c: &mut Character = Rc::make_mut(c_rc);
                    c.normalize_resources(&*self.resource_manager);
                }
                true
            }
            Msg::SwitchPane(new_pane) => {
                self.current_pane = new_pane;
                true
            }
            Msg::DeleteCharacter(id) => {
                if self.character.as_ref().map(|c| c.id) == Some(id) {
                    self.storage.remove("last_loaded_char_id");
                    self.character = None;
                }
                let mut existing = self.storage.load_characters();
                existing.remove(&id);
                self.save_characters(existing);
                true
            }
            Msg::LoadCharacter(mut c) => {
                {
                    let c_mut: &mut Character = Rc::make_mut(&mut c);
                    c_mut.normalize_resources(&*self.resource_manager);
                }
                self.storage
                    .store("last_loaded_char_id", Ok(format!("{:?}", c.id).into()));
                let mut existing = self.storage.load_characters();
                existing.insert(c.id, Ok(Rc::clone(&c)));
                self.save_characters(existing);
                self.current_pane = Pane::Core;
                self.character = Some(c);
                true
            }
            Msg::UnloadCharacter => {
                self.storage.remove("last_loaded_char_id");
                self.current_pane = Pane::SaveAndLoad;
                self.character = None;
                true
            }
        }
    }

    fn view(&self) -> Html {
        let nav_loading_spinner = if self.resource_manager.is_loading() {
            html! { <> { "Loading…" } </> }
        } else {
            html! { <></> }
        };
        let nav_button = |pane: Pane| {
            let disabled = !pane.show_for_character(self.character.as_ref().map(Rc::as_ref));
            html! {
                <button class=(if self.current_pane == pane { "active" } else { "" })
                        disabled=disabled onclick=self.link.callback(move |_| Msg::SwitchPane(pane))>
                    { pane.label() }
                </button>
            }
        };

        let nav = html! {
            <nav>
                { nav_loading_spinner }
                { nav_button(Pane::Debug) }
                { nav_button(Pane::SaveAndLoad) }
                { nav_button(Pane::Core) }
                { nav_button(Pane::Feats) }
                { nav_button(Pane::Skills) }
                { nav_button(Pane::Equipment) }
                { nav_button(Pane::Spells) }
            </nav>
        };

        let debug_pane = |c: Rc<Character>| -> Html {
            html! {
                <panes::DebugPane character=c />
            }
        };
        let save_load_pane = if let Pane::SaveAndLoad = self.current_pane {
            let on_delete = self.link.callback(Msg::DeleteCharacter);
            let on_select = self.link.callback(|c_opt: Option<Rc<Character>>| {
                c_opt
                    .map(Msg::LoadCharacter)
                    .unwrap_or(Msg::UnloadCharacter)
            });
            let raw_characters = self.storage.load_characters();
            let mut characters = Vec::with_capacity(raw_characters.len());
            for c_result in raw_characters.values() {
                if let Ok(c) = c_result.as_ref() {
                    characters.push(Rc::clone(c));
                }
            }
            let selected = self.character.as_ref().map(|c| c.id);
            html! {
                <panes::SaveAndLoadPane
                     characters=characters
                     selected=selected
                     on_delete_character=on_delete
                     on_select_character=on_select
                 />
            }
        } else {
            html! { <></> }
        };
        let core_pane = |c| {
            let r = Rc::clone(&self.resource_manager);
            html! {
                <panes::CorePane
                    resources=r
                    character=c
                    on_character_change=self.link.callback(|cc| Msg::CharacterChange(cc))
                 />
            }
        };
        let feats_pane = |_| html! { <></> };
        let skills_pane = |_| html! { <></> };
        let equipment_pane = |_| html! { <></> };
        let spells_pane = |_| html! { <></> };

        let hide_show_pane = |pane: Pane, pane_html_fn: &dyn Fn(Rc<Character>) -> Html| -> Html {
            if self.current_pane == pane {
                match self.character.as_ref() {
                    Some(c) => pane_html_fn(Rc::clone(c)),
                    None => html! { <></> },
                }
            } else {
                html! { <></> }
            }
        };

        html! {
            <>
                <main>
                    <header>
                        <h1> { self.current_pane.label() } </h1>
                        { nav }
                    </header>
                    { hide_show_pane(Pane::Debug, &debug_pane) }
                    { save_load_pane }
                    { hide_show_pane(Pane::Core, &core_pane) }
                    { hide_show_pane(Pane::Feats, &feats_pane) }
                    { hide_show_pane(Pane::Skills, &skills_pane) }
                    { hide_show_pane(Pane::Equipment, &equipment_pane) }
                    { hide_show_pane(Pane::Spells, &spells_pane) }
                </main>
                // <icons::IconDefinitions />
            </>
        }
    }
}

impl App {
    fn load_character<'s>(&'s mut self, id: Uuid) {
        let chars = self.storage.load_characters();
        match chars.get(&id) {
            Some(Ok(c)) => {
                self.character = Some(Rc::clone(c));
                self.current_pane = Pane::Core;
            }
            Some(Err(_)) => {
                warn!("Failed to load character with ID {:?}", id);
                self.character = None;
                self.current_pane = Pane::SaveAndLoad;
            }
            None => {
                warn!("Unknown character ID {:?}", id);
                self.character = None;
                self.current_pane = Pane::SaveAndLoad;
            }
        };
        if let Some(c) = self.character.as_ref() {
            debug!("Going to ensure character's resoruces are loaded in the cache");
            // Ensure resources are loaded
            self.resource_manager.ensure_all(c.resources.iter());
            if let Some((class_rref, _)) = c.get_class_and_level() {
                // Ensure class resources are loaded
                self.resource_manager.ensure_all_by_trait(&class_rref.name);
            }
        }
    }

    fn save_characters(&mut self, mut existing: HashMap<Uuid, Result<Rc<Character>, Value>>) {
        if let Some(c) = self.character.as_ref() {
            existing.insert(c.id, Ok(Rc::clone(c)));
        }
        let mut serialized: HashMap<Uuid, Value> = HashMap::new();
        for (id, c_result) in existing {
            match c_result {
                Ok(c) => match serde_json::to_value(&*c) {
                    Ok(v) => serialized.insert(id, v),
                    Err(e) => {
                        error!(
                            "Failed to save characters; character {} ({:?}) failed to serialize: {}",
                            c.name.as_str(), c.id, e,
                        );
                        return;
                    }
                },
                Err(v) => serialized.insert(id, v),
            };
        }
        self.storage.store(
            "characters",
            serde_json::to_string(&serialized).map_err(|e| e.into()),
        );
    }
}

trait StorageExt {
    fn load_characters(&self) -> HashMap<Uuid, Result<Rc<Character>, Value>>;
}

impl StorageExt for StorageService {
    fn load_characters(&self) -> HashMap<Uuid, Result<Rc<Character>, Value>> {
        let raw: std::string::String = match self.restore("characters") {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to load characters: {}", e);
                return HashMap::new();
            }
        };
        match serde_json::from_str::<HashMap<Uuid, Value>>(&raw) {
            Ok(id_values_map) => {
                let mut parsed = HashMap::new();
                for (id, value) in id_values_map {
                    match serde_json::from_value::<Character>(value.clone()) {
                        Ok(c) => {
                            parsed.insert(id, Ok(Rc::new(c)));
                        }
                        Err(e) => {
                            warn!("Failed to parse a character from raw JSON: {}", e);
                            parsed.insert(id, Err(value));
                        }
                    }
                }
                parsed
            }
            Err(e) => {
                error!("Failed to parse characters: {}", e);
                HashMap::new()
            }
        }
    }
}

#[wasm_bindgen]
pub fn run_app() -> Result<(), JsValue> {
    console_log::init_with_level(log::Level::Debug)
        .map_err(|e| JsValue::from_str(&format!("Failed to set log level: {}", e)))?;

    let window = web_sys::window().unwrap();
    let location = window.location();
    let url_base = location
        .href()?
        .parse()
        .map_err(|e| JsValue::from_str(&format!("Failed to parse location: {}", e)))?;
    yew::start_app_with_props::<App>(AppProps { url_base });

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
