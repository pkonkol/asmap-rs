mod components;

use components::MapContainer;

use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/")]
    Map,
    #[not_found]
    #[at("/404")]
    NotFound,
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::Map => html! { <MapContainer /> },
        Route::NotFound => {
            html! { <><Redirect<Route> to={Route::Map}/><h1>{ "test 404\nTODO redirect to /" }</h1></> }
        }
    }
}

#[function_component(App)]
fn app() -> Html {
    html! {
        <div id="top">
            <BrowserRouter>
                <Switch<Route> render={switch} />
            </BrowserRouter>
        </div>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"console websys test".into());
    yew::Renderer::<App>::new().render();
}
