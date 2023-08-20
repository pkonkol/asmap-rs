pub mod api;
pub mod map_component;

use crate::components::map_component::{City, MapComponent, Point};
use yew::prelude::*;

pub struct MapContainer;

impl Component for MapContainer {
    type Message = ();
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        true
    }

    fn changed(&mut self, _ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <MapComponent  />
            </>
        }
    }
}
