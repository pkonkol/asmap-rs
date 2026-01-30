pub mod api;
pub mod geocoding;
pub mod map_component;
pub mod details_page;

use yew::prelude::*;
use yew_router::prelude::*;

use crate::routes::Route;
use details_page::DetailsPage;
use map_component::MapComponent;


pub struct App;

fn switch(route: Route) -> Html {
    match route {
        Route::Map => html!(<MapComponent />),
        Route::Details { id } => html!(<DetailsPage id={id} />),
        Route::NotFound => html!(<h1>{"404"}</h1>),
    }
}

impl Component for App {
    type Message = ();
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, _ctx: &Context<Self>, _msg: Self::Message) -> bool {
        true
    }

    fn changed(&mut self, _ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        false
    }

    // TODO GIS dodatkowa podstrona z detalami
    fn view(&self, _ctx: &Context<Self>) -> Html {
         html! {
            <BrowserRouter>
                <Switch<Route> render={switch} />
            </BrowserRouter>
        }
    }
}
