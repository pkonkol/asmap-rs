use std::collections::HashMap;

use asdb_models::As;
use gloo_console::log;
use gloo_utils::document;
use leaflet::{LatLng, Map, Marker, TileLayer};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{Element, HtmlElement, Node};
use yew::prelude::*;

use super::api::{debug_ws, get_all_as};
const GDYNIA_LAT: f64 = 54.52500;
const GDYNIA_LON: f64 = 18.54992;

pub enum Msg {
    LoadAs,
    Debug,
    DrawAs(Vec<As>),
    Error,
}

pub struct MapComponent {
    map: Map,
    container: HtmlElement,
    ases: HashMap<u32, As>,
    // TODO here loaded bounds,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Point(pub f64, pub f64);

#[derive(Properties, PartialEq, Clone)]
pub struct Props {}

impl MapComponent {
    fn render_map(&self) -> Html {
        let node: &Node = &self.container.clone().into();
        Html::VRef(node.clone())
    }

    fn load_as_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::LoadAs);
        html! {
            <button onclick={cb}>{"Load ASes"}</button>
        }
    }

    fn debug_ws_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::Debug);
        html! {
            <button onclick={cb}>{"Debug WS"}</button>
        }
    }
}

impl Component for MapComponent {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        let container: Element = document().create_element("div").unwrap();
        let container: HtmlElement = container.dyn_into().unwrap();
        container.set_class_name("map");
        let leaflet_map = Map::new_with_element(&container, &JsValue::NULL);
        add_marker(
            &leaflet_map,
            "demo marker w gdyni",
            &Point(GDYNIA_LAT, GDYNIA_LON),
        );
        Self {
            map: leaflet_map,
            container,
            ases: HashMap::new(),
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, first_render: bool) {
        if first_render {
            self.map.setView(&LatLng::new(GDYNIA_LAT, GDYNIA_LON), 11.0);
            add_tile_layer(&self.map);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Debug => {
                log!("debug executed");
                ctx.link().send_future(async {
                    let res = debug_ws().await;
                    log!("res: {}", format!("{res:?}"));
                    Msg::Error
                });
            }
            Msg::LoadAs => {
                log!("load ASes initiatied");
                ctx.link().send_future(async {
                    match get_all_as().await {
                        Ok(ases) => Msg::DrawAs(ases),
                        Err(_) => Msg::Error,
                    }
                });
            }
            Msg::DrawAs(ases) => {
                let ases_str = format!("{:?}", ases);
                log!(format!("ASES ARE:\n {:#?}", ases_str));
                log!("ASes fetched, drawing them signal at map_component.rs");
                for a in ases.into_iter() {
                    let asn = a.asn.clone();
                    let i = self.ases.insert(asn, a);
                    if i.is_none() {
                        let aa = self.ases.get(&asn).unwrap();

                        add_marker(
                            &self.map,
                            &format!(
                                "asn:{}, country:{}, name: {}, rank: {}, org: {:?}",
                                aa.asn,
                                aa.asrank_data.as_ref().unwrap().country_name,
                                aa.asrank_data.as_ref().unwrap().name,
                                aa.asrank_data.as_ref().unwrap().rank,
                                aa.asrank_data.as_ref().unwrap().organization,
                            ),
                            &Point(
                                aa.asrank_data.as_ref().unwrap().coordinates.lon,
                                aa.asrank_data.as_ref().unwrap().coordinates.lat,
                            ),
                        );
                        log!("inserted asn ", asn);
                    };
                }
            }
            Msg::Error => {
                log!("error fetching ases");
            }
        }
        true
    }

    fn changed(&mut self, _ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
            <div class="map-container component-container">
                {self.render_map()}
            </div>
            <div class="control component-container">
                <div>
                    {Self::load_as_button(self, ctx)}
                </div>
                <div>
                    {Self::debug_ws_button(self, ctx)}
                </div>
            </div>
            </>
        }
    }
}

fn add_tile_layer(map: &Map) {
    TileLayer::new(
        "https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png",
        &JsValue::NULL,
    )
    .addTo(map);
}

fn add_marker(map: &Map, description: &str, coord: &Point) {
    let opts = JsValue::from_str(r#"{"opacity": "0.5"}"#);
    let latlng = LatLng::new(coord.0, coord.1);
    let m = Marker::new_with_options(&latlng, &opts);

    let p = JsValue::from_str(description);
    m.bindPopup(&p, &JsValue::from_str("popup"));
    // m.setPopupContent(&p);
    m.addTo(map);
}
