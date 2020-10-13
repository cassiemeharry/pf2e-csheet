use pf2e_csheet_shared::Character;
use std::rc::Rc;
use yew::prelude::*;

pub struct DebugPane {
    #[allow(unused)]
    link: ComponentLink<Self>,
    character: Rc<Character>,
}

#[derive(Debug)]
pub enum Msg {}

#[derive(Clone, Debug, Properties)]
pub struct Props {
    pub character: Rc<Character>,
}

impl Component for DebugPane {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Props, link: ComponentLink<Self>) -> Self {
        Self {
            link,
            character: props.character,
        }
    }

    fn change(&mut self, props: Props) -> ShouldRender {
        self.character = props.character;
        true
    }

    fn update(&mut self, msg: Msg) -> ShouldRender {
        match msg {}
    }

    fn view(&self) -> Html {
        let c: &Character = &*self.character;
        html! {
            <div id="debug-pane">
                <pre><code>{ format!("{:#?}", c) }</code></pre>
            </div>
        }
    }
}
