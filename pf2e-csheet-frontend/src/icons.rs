use yew::prelude::*;

pub struct IconDefinitions;

impl Component for IconDefinitions {
    type Message = ();
    type Properties = ();

    fn create(_: (), _link: ComponentLink<Self>) -> Self {
        Self
    }

    fn change(&mut self, _: ()) -> ShouldRender {
        false
    }

    fn update(&mut self, _msg: ()) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        let free_action = html! {
            <g transform="translate(2, 2) rotate(-45 4 4)" id="icon-free-action">
                <rect x="0.75" y="0.75" width="6.5" height="6.5" fill="none" stroke="black" stroke-width="1.5" />
                <path d="M 4.5, 1 v 3.5 h -3.5" stroke="black" stroke-width="2" fill="none" />
            </g>
        };
        let reaction = html! {
            <g id="icon-reaction">
                <path d="M 0, 2 S 2, 0 4, 0 S 8, 2 8, 3 S 8, 5 5, 5.75 L 6, 7 L 1, 6 L 4.9, 3.5 L 4.75, 5 S 6, 4.5 6, 3.5 S 6, 1.5 4.5, 1 S 3, 1 0, 2 Z" fill="black" />
            </g>
        };
        let one_action = html! {
            <g transform="translate(2, 2) rotate(-45 4 4)" id="icon-one-action">
                <rect width="3" height="3" fill="#000000" />
                <polyline points="5, 0 8, 0 8, 8 0, 8 0, 5 5, 5" fill="#000000"/>
            </g>
        };
        let two_actions = html! {
            <g transform="translate(2, 2) rotate(-45 4 4)" id="icon-two-actions">
                <rect width="3" height="3" fill="#000000" />
                <polyline points="5, 0 8, 0 8, 8 0, 8 0, 5 5, 5" fill="#000000"/>
                <polyline points="5 9.5, 9.5 9.5, 9.5 5, 12 5, 12 12, 5 12 " fill="#000000"/>
            </g>
        };
        let three_actions = html! {
            <g transform="translate(2, 2) rotate(-45 4 4)" id="icon-three-actions">
                <rect width="3" height="3" fill="#000000" />
                <polyline points="5, 0 8, 0 8, 8 0, 8 0, 5 5, 5" fill="#000000"/>
                <polyline points="5 9.5, 9.5 9.5, 9.5 5, 12 5, 12 12, 5 12 " fill="#000000"/>
                <polyline points="10 13.5, 13.5 13.5, 13.5 10, 16 10, 16 16, 10 16" fill="#000000"/>
            </g>
        };
        html! {
            <svg xmlns="http://www.w3.org/2000/svg" style="display: none;">
                <defs>
                    { free_action }
                    { reaction }
                    { one_action }
                    { two_actions }
                    { three_actions }
                </defs>
            </svg>
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Icon {
    FreeAction,
    Reaction,
    OneAction,
    TwoActions,
    ThreeActions,
}

impl Icon {
    fn svg_view_box(&self) -> &'static str {
        match self {
            Self::FreeAction => "0 0 12 12",
            Self::Reaction => "0 0 8 8",
            Self::OneAction => "0 0 12 12",
            Self::TwoActions => "0 0 18 12",
            Self::ThreeActions => "0 0 24 12",
        }
    }

    fn id(&self) -> &'static str {
        match self {
            Self::FreeAction => "#icon-free-action",
            Self::Reaction => "#icon-reaction",
            Self::OneAction => "#icon-one-action",
            Self::TwoActions => "#icon-two-actions",
            Self::ThreeActions => "#icon-three-actions",
        }
    }

    pub fn as_html(&self) -> Html {
        html! {
            <svg xmlns="http://www.w3.org/2000/svg" viewBox=self.svg_view_box() class="icon">
                <use href=self.id() />
            </svg>
        }
    }
}
