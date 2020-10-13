use serde::{de::DeserializeOwned, Serialize};
use std::{
    cmp::PartialEq,
    fmt::{Debug, Display},
    string::ToString,
};
use yew::prelude::*;

pub trait SelectItem:
    Clone + Debug + Display + PartialEq + DeserializeOwned + Serialize + 'static
{
}

impl<T> SelectItem for T where
    T: Clone + Debug + Display + PartialEq + DeserializeOwned + Serialize + 'static
{
}

pub struct TypedSelect<T: SelectItem> {
    link: ComponentLink<Self>,
    choices: Vec<T>,
    selected: Option<T>,
    onselect: Callback<Option<T>>,
    disabled: bool,
}

#[derive(Debug)]
pub enum Msg<T: SelectItem> {
    NoOp,
    Selected(Option<T>),
}

#[derive(Clone, Properties)]
pub struct Props<T: SelectItem> {
    pub choices: Vec<T>,
    #[prop_or_default]
    pub selected: Option<T>,
    pub onselect: Callback<Option<T>>,
    #[prop_or(false)]
    pub disabled: bool,
}

impl<T: SelectItem> Component for TypedSelect<T> {
    type Message = Msg<T>;
    type Properties = Props<T>;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            choices: props.choices,
            selected: props.selected,
            onselect: props.onselect,
            disabled: props.disabled,
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.choices = props.choices;
        self.selected = props.selected;
        self.onselect = props.onselect;
        self.disabled = props.disabled;
        true
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::NoOp => false,
            Msg::Selected(opt) => {
                self.onselect.emit(opt);
                true
            }
        }
    }

    fn view(&self) -> Html {
        html! {
            <select onchange=self.link.callback(Self::handle_select) disabled=self.disabled>
                <option selected=self.selected.is_none() value="">{ "-----" }</option>
                { for self.choices.iter().map(|item| self.view_option(item)) }
            </select>
        }
    }
}

impl<T: SelectItem> TypedSelect<T> {
    fn view_option(&self, choice: &T) -> Html {
        let is_selected = Some(choice) == self.selected.as_ref();
        let value = match serde_json::to_string(choice) {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to serialize select option {:?}: {}", choice, e);
                return html! { <> </> };
            }
        };
        html! {
            <option selected=is_selected value=value>{ choice }</option>
        }
    }

    fn handle_select(e: ChangeData) -> Msg<T> {
        match e {
            ChangeData::Select(elem) => {
                let raw = elem.value();
                if raw.is_empty() {
                    Msg::Selected(None)
                } else {
                    match serde_json::from_str::<T>(&raw) {
                        Ok(item) => Msg::Selected(Some(item)),
                        Err(e) => {
                            error!(
                                "Failed to parse a {} from {:?}: {}",
                                std::any::type_name::<T>(),
                                raw,
                                e
                            );
                            Msg::NoOp
                        }
                    }
                }
            }
            other => unreachable!("TypedSelect onselect handler unreachable: {:?}", other),
        }
    }
}
