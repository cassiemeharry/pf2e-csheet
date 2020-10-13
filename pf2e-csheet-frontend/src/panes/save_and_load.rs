use pf2e_csheet_shared::Character;
use smartstring::alias::String;
use std::rc::Rc;
use uuid::Uuid;
use yew::prelude::*;

pub struct SaveAndLoadPane {
    link: ComponentLink<Self>,
    characters: Vec<Rc<Character>>,
    selected: Option<Uuid>,
    on_delete_character: Callback<Uuid>,
    on_select_character: Callback<Option<Rc<Character>>>,
}

#[derive(Clone, Properties)]
pub struct SaveAndLoadPaneProps {
    pub characters: Vec<Rc<Character>>,
    pub selected: Option<Uuid>,
    pub on_delete_character: Callback<Uuid>,
    pub on_select_character: Callback<Option<Rc<Character>>>,
}

pub enum Msg {
    CreateNewCharacter,
    DeleteCharacter(Uuid),
    LoadCharacter(Rc<Character>),
    UnloadCharacter,
}

impl Component for SaveAndLoadPane {
    type Message = Msg;
    type Properties = SaveAndLoadPaneProps;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut characters = props.characters;
        characters.sort_by_key(|c| c.name.to_lowercase());
        Self {
            link,
            characters,
            selected: props.selected,
            on_delete_character: props.on_delete_character,
            on_select_character: props.on_select_character,
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.characters = props.characters;
        self.characters.sort_by_key(|c| c.name.to_lowercase());
        self.selected = props.selected;
        self.on_select_character = props.on_select_character;
        true
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::CreateNewCharacter => {
                let new_character = Rc::new(Character::new("Unnamed Character"));
                self.on_select_character.emit(Some(new_character));
                false
            }
            Msg::DeleteCharacter(id) => {
                self.on_delete_character.emit(id);
                false
            }
            Msg::LoadCharacter(c) => {
                self.on_select_character.emit(Some(c));
                false
            }
            Msg::UnloadCharacter => {
                self.on_select_character.emit(None);
                false
            }
        }
    }

    fn view(&self) -> Html {
        let create_new_row = html! {
            <tr class="create-new">
                <td class="loaded"></td>
                <td class="name"></td>
                <td class="class"></td>
                <td class="level"></td>
                <td class="actions">
                    <button onclick=self.link.callback(|_| Msg::CreateNewCharacter)>{
                        "Create New"
                    }</button>
                </td>
            </tr>
        };

        html! {
            <div id="save-load-pane">
                <table style="width: 100%;">
                    <thead>
                        <tr>
                            <th class="loaded">{ "Loaded?" }</th>
                            <th class="name">{ "Character Name" }</th>
                            <th class="class">{ "Class" }</th>
                            <th class="level">{ "Level" }</th>
                            <th class="actions">{ "Actions" }</th>
                        </tr>
                    </thead>
                    <tbody>
                        { for self.characters.iter().map(|c| self.render_char_row(c)) }
                        { create_new_row }
                    </tbody>
                </table>
            </div>
        }
    }
}

impl SaveAndLoadPane {
    fn render_char_row(&self, character: &Rc<Character>) -> Html {
        let (loaded, load_button) = if Some(character.id) == self.selected {
            (
                "✓",
                html! {
                    <button onclick=self.link.callback(move |_| Msg::UnloadCharacter)>{ "Unload" }</button>
                },
            )
        } else {
            let c = Rc::clone(character);
            let load_callback = move |_| Msg::LoadCharacter(Rc::clone(&c));
            let button = html! {
                <button onclick=self.link.callback(load_callback)>{ "Load" }</button>
            };
            ("", button)
        };
        let class_level_action = {
            let (class_name, level) = match character.get_class_and_level() {
                Some((class_rref, level)) => (class_rref.name, level),
                None => (String::new(), 1.into()),
            };
            let id = character.id;
            html! {
                <>
                    <td class="class">{ class_name }</td>
                    <td class="level">{ level }</td>
                    <td class="actions">
                        { load_button }
                        <button onclick=self.link.callback(move |_| Msg::DeleteCharacter(id))>{ "❌" }</button>
                    </td>
                </>
            }
        };
        html! {
            <tr>
                <td class="loaded">{ loaded }</td>
                <td class="name">{ character.name.as_str() }</td>
                { class_level_action }
            </tr>
        }
    }
}
