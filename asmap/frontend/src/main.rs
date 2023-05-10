mod components;

use components::MapContainer;

use gloo_net::http::Request;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/")]
    Home,
    #[at("/hello-server")]
    HelloServer,
    #[at("/map")]
    Map,
    #[at("/struct")]
    Struct,
    #[not_found]
    #[at("/404")]
    NotFound,
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::Home => html! { <h1>{ "Hello Frontend" }</h1> },
        Route::HelloServer => html! { <HelloServer /> },
        Route::Map => html! { <MapContainer /> },
        Route::Struct => html! { <><Struct /><h1>{ "struct component router" }</h1></> },
        Route::NotFound => html! { <><Redirect<Route> to={Route::Home}/><h1>{ "test 404\nTODO redirect to /" }</h1></> },
    }
}

struct Struct {
    name: String,
    counter: i32,
}

impl Component for Struct {
    type Message = ();

    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        Self { name: String::from("first struct component"), counter: 1 }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html!{
            <>
            <h1>{"struct component view(), cnt: {self.counter} name: {self.name}  "}<br/>{ &self.name }<br/>{ &self.counter }</h1>
            </>
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

#[function_component(HelloServer)]
fn hello_server() -> Html {
    let data = use_state(|| None);

    // Request `/api/hello` once
    {
        let data = data.clone();
        use_effect(move || {
            if data.is_none() {
                spawn_local(async move {
                    let resp = Request::get("/api/hello").send().await.unwrap();
                    let result = {
                        if !resp.ok() {
                            Err(format!(
                                "Error fetching data {} ({})",
                                resp.status(),
                                resp.status_text()
                            ))
                        } else {
                            resp.text().await.map_err(|err| err.to_string())
                        }
                    };
                    data.set(Some(result));
                });
            }

            || {}
        });
    }

    match data.as_ref() {
        None => {
            html! {
                <div>{"No server response"}</div>
            }
        }
        Some(Ok(data)) => {
            html! {
                <div>{"Got server response: "}{data}</div>
            }
        }
        Some(Err(err)) => {
            html! {
                <div>{"Error requesting data from server: "}{err}</div>
            }
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"console websys test".into());
    yew::Renderer::<App>::new().render();
    // yew::Renderer::<Map>::new().render();
}
