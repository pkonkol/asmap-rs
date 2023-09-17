use std::collections::HashMap;

use anyhow::anyhow;
use asdb_models::As;
use gloo_console::log;
use gloo_utils::{document, format::JsValueSerdeExt};
use leaflet::{Icon, LatLng, Map, Marker, TileLayer};
use serde::Serialize;
use wasm_bindgen::{prelude::*, JsCast, JsObject};
use web_sys::{Element, HtmlElement, Node};
use yew::prelude::*;

use super::api::{debug_ws, get_all_as, get_all_as_filtered};
const POLAND_LAT: f64 = 52.11431;
const POLAND_LON: f64 = 19.423672;
const ICON_URL: &str = "https://unpkg.com/leaflet@1.9.3/dist/images/marker-icon.png";

pub enum Msg {
    LoadAs,
    LoadAsFiltered,
    Debug,
    DrawAs(Vec<As>),
    Error(anyhow::Error),
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

    fn load_as_filtered_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::LoadAsFiltered);
        html! {
            <button onclick={cb}>{"Load filtered ASes"}</button>
        }
    }

    fn filter_menu(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div>
                <div >
                    <div style="display:inline-block;"><p>{"min addr"}</p>
                        <input title="test" type="number" id="minAddresses" value="0" min="0" max="9999999"/>
                    </div>
                    <div style="display:inline-block;"><p>{"max addr"}</p>
                        <input type="number" id="maxAddresses" value="100000" min="0" max="9999999"/>
                    </div>
                    <div style="display:inline-block;"><p>{"country code"}</p>
                        <input type="text" id="countryCode" value="PL"/>
                    </div>
                    <div style="display:inline-block;"><p>{"min rank"}</p>
                        <input type="number" id="minRank" value="0" min="0" max="999999"/>
                    </div>
                    <div style="display:inline-block;"><p>{"max rank"}</p>
                        <input type="number" id="maxRank" value="1000000" min="0" max="999999"/>
                    </div>
                    <div style="display:inline-block;"><p>{"has org"}</p>
                        <input type="checkbox" id="hasOrg" />
                    </div>
                </div>
            </div>
        }
    }

    fn download_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::Debug);
        html! {
            <button onclick={cb}>{"Download"}</button>
        }
    }

    fn debug_ws_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::Debug);
        html! {
            <button onclick={cb}>{"Debug WS"}</button>
        }
    }
    // TODO filtering interface, country dropdown allowing up to 1 choice
    // bounds should come from the visible screen area so not here. And should be default for load ases up to some zoom level
    // addresses range slider (min&max)
    // rank range slider (min&max)
    // load by filter button
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
            "geometric center of poland, test",
            &Point(POLAND_LAT, POLAND_LON),
            (25, 41),
        );
        Self {
            map: leaflet_map,
            container,
            ases: HashMap::new(),
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, first_render: bool) {
        if first_render {
            self.map.setView(&LatLng::new(POLAND_LAT, POLAND_LON), 8.0);
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
                    Msg::Error(anyhow!("test error"))
                });
            }
            Msg::LoadAs => {
                log!("load ASes initiatied");
                ctx.link().send_future(async {
                    match get_all_as().await {
                        Ok(ases) => Msg::DrawAs(ases),
                        Err(e) => Msg::Error(e),
                    }
                });
            }
            Msg::LoadAsFiltered => {
                log!("load ASes initiatied");
                ctx.link().send_future(async {
                    match get_all_as_filtered().await {
                        Ok(ases) => Msg::DrawAs(ases),
                        Err(e) => Msg::Error(e),
                    }
                });
            }
            Msg::DrawAs(ases) => {
                // let ases_str = format!("{:?}", ases);
                // log!(format!("ASES ARE:\n {:#?}", ases_str));
                log!(
                    "{} ASes fetched, drawing them signal at map_component.rs",
                    ases.len()
                );
                for a in ases.into_iter() {
                    let asn = a.asn.clone();
                    let i = self.ases.insert(asn, a);
                    if i.is_none() {
                        let aa = self.ases.get(&asn).unwrap();
                        let aa_size = scale_as_marker(&aa);
                        let asrank_data = aa.asrank_data.as_ref().unwrap();
                        add_marker(
                            &self.map,
                            &format!(
                                "asn:{}, country:{}, name: {}, rank: {}, org: {:?}, prefixes: {}, addresses: {}",
                                aa.asn,
                                asrank_data.country_name,
                                asrank_data.name,
                                asrank_data.rank,
                                asrank_data.organization,
                                // asrank_data.coordinates.lat,
                                // asrank_data.coordinates.lon,
                                asrank_data.prefixes,
                                asrank_data.addresses,
                            ),
                            &Point(
                                asrank_data.coordinates.lat,
                                asrank_data.coordinates.lon,
                            ),
                            aa_size,
                        );
                    };
                }
            }
            Msg::Error(e) => {
                log!(format!("error fetching ases, received error '{e:?}'"));
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
                    {Self::load_as_filtered_button(self, ctx)}
                    {Self::filter_menu(self, ctx)}
                </div>
                <div>
                    {Self::download_button(self, ctx)}
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

/// will create marker with given description in a popup at given coordinate.
/// marker size will be `size` in pixels as (width, height)
fn add_marker(map: &Map, description: &str, coord: &Point, size: (u64, u64)) {
    const ICON_SIZE: (u32, u32) = (25, 41);
    let opts = JsValue::from_str(r#"{"opacity": "0.5"}"#);
    let latlng = LatLng::new(coord.0, coord.1);
    let m = Marker::new_with_options(&latlng, &opts);

    let p = JsValue::from_str(description);
    m.bindPopup(&p, &JsValue::from_str("popup"));

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct IconOpts {
        pub icon_url: String,
        pub icon_size: Vec<u64>,
        pub class_name: String,
    }
    let i = Icon::new(
        &serde_wasm_bindgen::to_value(&IconOpts {
            icon_url: ICON_URL.to_string(),
            icon_size: vec![size.0, size.1],
            class_name: "test-classname".to_string(),
        })
        .unwrap(),
    );
    // log!("{}", format!("icon: {i:?}"));
    m.setIcon(&i);
    // m.setPopupContent(&p);
    m.addTo(map);
}

/// returns (width, height) in pixels based on
/// by rank or by addresses amount? both would suit
/// both may be used, 1 as color other as marker size
fn scale_as_marker(a: &As) -> (u64, u64) {
    const RANK_RANGE: (u64, u64) = (0, 115000); // 0 not needed likely
    const ADDRESS_RANGE: (u64, u64) = (0, 20017664);
    const AVG_PIXELS: (u64, u64) = (15, 24); //original is 25,41
    const MIN_PIXELS: (u64, u64) = (5, 8);
    let rank = a.asrank_data.as_ref().unwrap().rank;
    let scale = (rank as f64 / RANK_RANGE.1 as f64).clamp(0., 1.);
    // let addresses = a.asrank_data.as_ref().unwrap().addresses;
    // let scale = (addresses as f64 / ADDRESS_RANGE.1 as f64).clamp(0., 1.);
    let width = MIN_PIXELS.0 + AVG_PIXELS.0 - (AVG_PIXELS.0 as f64 * scale) as u64;
    let height = MIN_PIXELS.1 + AVG_PIXELS.1 - (AVG_PIXELS.1 as f64 * scale) as u64;

    (width, height)
}
