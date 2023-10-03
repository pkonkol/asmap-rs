use std::{
    collections::{HashMap, HashSet},
    io::Write,
};

use asdb_models::{As, Bound, Coord};
use gloo_console::log;
use gloo_file::{Blob, ObjectUrl};
use gloo_utils::document;
use leaflet::{Icon, LatLng, Map, Marker, TileLayer};

use protocol::{AsFilters, AsForFrontend};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{prelude::*, JsCast};
use wasm_timer::SystemTime;
use web_sys::{Element, HtmlElement, HtmlInputElement, Node};
use yew::prelude::*;

use super::api::{get_all_as, get_all_as_filtered};
use crate::models::{self, CsvAs};
use leaflet_markercluster::{markerClusterGroup, MarkerClusterGroup};

const POLAND_LAT: f64 = 52.11431;
const POLAND_LON: f64 = 19.423672;
const MARKER_ICON_URL: &str = "https://unpkg.com/leaflet@1.9.3/dist/images/marker-icon.png";

pub enum Msg {
    LoadAllAs,
    LoadAsBounded,
    LoadAsFiltered,
    UpdateFilters(FilterForm),
    ClearMarkers,
    DrawAs(Vec<AsForFrontend>),
    ShowAllCached,
    ShowFilteredCached,
    DownloadFiltered,
    DownloadAllCached,
    Error(anyhow::Error),
}

#[derive(Debug)]
pub enum FilterForm {
    HasOrg,
    MinAddresses(u64),
    MaxAddresses(u64),
    CountryCode(String),
    MinRank(u64),
    MaxRank(u64),
    IsBounded,
}

pub struct MapComponent {
    map: Map,
    container: HtmlElement,
    marker_cluster: MarkerClusterGroup,
    as_cache: HashMap<u32, AsForFrontend>,
    /// these are actually just last drawn ases and serve as a proxy for the last filter use
    drawn_filtered_ases: HashSet<u32>,
    filters: AsFilters,
    last_executed_filters: AsFilters,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Point(pub f64, pub f64);

#[derive(Properties, PartialEq, Clone)]
pub struct Props {}

// interface components
impl MapComponent {
    fn load_all_as_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::LoadAllAs);
        html! {
            <button onclick={cb}>{"Load ASes"}</button>
        }
    }

    fn load_as_bounded_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::LoadAsBounded);
        html! {
            <button onclick={cb}>{"Load ASes in visible range"}</button>
        }
    }

    fn load_as_filtered_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::LoadAsFiltered);
        html! {
            <button onclick={cb}>{"Load filtered ASes"}</button>
        }
    }

    fn show_cached_filtered_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::ShowFilteredCached);
        html! {
            <button onclick={cb}>{"show filtered cached ASes"}</button>
        }
    }

    fn show_cached_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::ShowAllCached);
        html! {
            <button onclick={cb}>{"show all cached ASes"}</button>
        }
    }

    fn download_cached_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::DownloadAllCached);
        html! {
            <button onclick={cb}>{"download all cached ASes"}</button>
        }
    }

    fn filter_menu(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div>
                <div >
                    <div style="display:inline-block;">{"min addr"}<br/>
                        <input title="test" type="number" id="minAddresses" value={self.filters.addresses.unwrap().0.to_string()} min="0" max="99999999" style="width: 5em;"
                            oninput={ctx.link().callback(|e: InputEvent| {
                                Msg::UpdateFilters(FilterForm::MinAddresses(
                                    e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                            })}
                        />
                    </div>
                    <div style="display:inline-block;">{"max addr"}<br/>
                        <input type="number" id="maxAddresses" value={self.filters.addresses.unwrap().1.to_string()} min="0" max="99999999" style="width: 5em;"
                            oninput={ctx.link().callback(|e: InputEvent| {
                                Msg::UpdateFilters(FilterForm::MaxAddresses(
                                    e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                            })}
                        />
                    </div>
                    <div style="display:inline-block;">{"country code"}<br/>
                        <input type="text" id="countryCode" value={self.filters.country.clone()} size="2"
                            oninput={ctx.link().callback(|e: InputEvent| {
                                Msg::UpdateFilters(FilterForm::CountryCode(
                                    e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                            })}
                        />
                    </div>
                    <div style="display:inline-block;">{"min rank"}<br/>
                        <input type="number" id="minRank" value={self.filters.rank.unwrap().0.to_string()} min="0" max="999999" style="width: 4em;"
                            oninput={ctx.link().callback(|e: InputEvent| {
                                Msg::UpdateFilters(FilterForm::MinRank(
                                    e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                            })}
                        />
                    </div>
                    <div style="display:inline-block;">{"max rank"}<br/>
                        <input type="number" id="maxRank" value={self.filters.rank.unwrap().1.to_string()} min="0" max="999999" style="width: 4em;"
                            oninput={ctx.link().callback(|e: InputEvent| {
                                Msg::UpdateFilters(FilterForm::MaxRank(
                                    e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                            })}
                        />
                    </div>
                    <div style="display:inline-block;">{"hasOrg\u{00a0}"}<br/>
                        <input type="checkbox" id="hasOrg" checked={self.filters.has_org.unwrap()}
                            oninput={ctx.link().callback(|_e: InputEvent| {
                                Msg::UpdateFilters(FilterForm::HasOrg)
                            })}

                        />
                    </div>
                    <div style="display:inline-block;">{"isBounded"}<br/>
                        <input type="checkbox" id="isBounded" checked={!self.filters.bounds.is_none()}
                            oninput={ctx.link().callback(|_e: InputEvent| {
                                Msg::UpdateFilters(FilterForm::IsBounded)
                            })}
                        />
                    </div>
                </div>
            </div>
        }
    }

    fn download_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::DownloadFiltered);
        html! {
            <button onclick={cb}>{"Download filtered"}</button>
        }
    }

    fn clear_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::ClearMarkers);
        html! {
            <button onclick={cb}>{"Clear"}</button>
        }
    }

    fn counter(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <>
            <b>{"drawn:"}</b>{self.drawn_filtered_ases.len()}<br/>
            <b>{"cached:"}</b>{ self.as_cache.len() }<br/>
            <b>{"session:"}</b>{"TODO"}
            </>
        }
    }
}

// utils
impl MapComponent {
    fn render_map(&self) -> Html {
        let node: &Node = &self.container.clone().into();
        Html::VRef(node.clone())
    }

    fn create_downloadable_csv<'a>(&self, ases: impl Iterator<Item = &'a AsForFrontend>) {
        let document = web_sys::window().unwrap().document().unwrap();
        let body = document.body().expect("document should have a body");

        let mut wtr = csv::Writer::from_writer(Vec::new());
        let mut ases_len = 0u64;
        for a in ases {
            ases_len += 1;
            wtr.serialize(CsvAs::from(a)).unwrap();
        }
        wtr.flush().unwrap();

        let blob = Blob::new_with_options(wtr.into_inner().unwrap().as_slice(), Some("text/plain"));
        let object_url = ObjectUrl::from(blob);

        let tmp_download_link = document.create_element("a").unwrap();
        tmp_download_link
            .set_attribute("href", &object_url)
            .unwrap();
        tmp_download_link
            .set_attribute(
                "download",
                &format!(
                    "filtered-as-{}-{}.csv",
                    ases_len,
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                ),
            )
            .unwrap();

        let tmp_node = body.append_child(&tmp_download_link).unwrap();
        tmp_node.clone().dyn_into::<HtmlElement>().unwrap().click();
        body.remove_child(&tmp_node).unwrap();
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
        leaflet_map.setMaxZoom(18.0);

        let marker_cluster_opts = js_sys::Object::new();
        let cluster_radius_func =
            Closure::wrap(Box::new(|zoom: f64| if zoom < 9. { 80f64 } else { 1f64 })
                as Box<dyn Fn(f64) -> f64>)
            .into_js_value();
        js_sys::Reflect::set(
            &marker_cluster_opts,
            &JsValue::from_str("maxClusterRadius"),
            &cluster_radius_func,
        )
        .unwrap();
        js_sys::Reflect::set(
            &marker_cluster_opts,
            &JsValue::from_str("chunkedLoading"),
            &serde_wasm_bindgen::to_value(&true).unwrap(),
        )
        .unwrap();

        let marker_cluster = markerClusterGroup(&marker_cluster_opts.into());

        marker_cluster.addTo(&leaflet_map);
        let initial_filters = AsFilters {
            country: Some("PL".to_string()),
            bounds: None,
            addresses: Some((0, 21000000)),
            rank: Some((0, 115000)),
            has_org: Some(false),
        };
        Self {
            map: leaflet_map,
            container,
            marker_cluster,
            as_cache: HashMap::new(),
            drawn_filtered_ases: HashSet::new(),
            filters: initial_filters.clone(),
            last_executed_filters: initial_filters,
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, first_render: bool) {
        if first_render {
            self.map.setView(&LatLng::new(POLAND_LAT, POLAND_LON), 5.0);
            add_tile_layer(&self.map);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadAllAs => {
                log!("load ASes initiatied");
                let bounds = self.map.getBounds();

                log!(format!("bounds: {:?}", bounds));
                let x: models::LatLngBounds =
                    serde_wasm_bindgen::from_value(bounds.into()).unwrap();
                log!(format!("bounds parsed to rust: {:?}", x));
                ctx.link().send_future(async {
                    match get_all_as().await {
                        Ok(ases) => Msg::DrawAs(ases),
                        Err(e) => Msg::Error(e),
                    }
                });
            }
            Msg::LoadAsBounded => {
                let bounds: models::LatLngBounds =
                    serde_wasm_bindgen::from_value(self.map.getBounds().into()).unwrap();
                log!(format!("load AS bounded initiatied, bounds: {bounds:?}"));

                let filters = AsFilters {
                    bounds: Some(Bound {
                        north_east: Coord {
                            lat: bounds._northEast.lat,
                            lon: bounds._northEast.lng,
                        },
                        south_west: Coord {
                            lat: bounds._southWest.lat,
                            lon: bounds._southWest.lng,
                        },
                    }),
                    ..Default::default()
                };
                ctx.link().send_future(async {
                    match get_all_as_filtered(filters).await {
                        Ok(ases) => Msg::DrawAs(ases),
                        Err(e) => Msg::Error(e),
                    }
                });
            }
            Msg::LoadAsFiltered => {
                log!("load ASes initiatied");
                // Bounds must be dynamically updated at the time of button press if checkbox is on
                if self.filters.bounds.is_some() {
                    let bounds: models::LatLngBounds =
                        serde_wasm_bindgen::from_value(self.map.getBounds().into()).unwrap();
                    self.filters.bounds = Some(Bound {
                        north_east: Coord {
                            lat: bounds._northEast.lat,
                            lon: bounds._northEast.lng,
                        },
                        south_west: Coord {
                            lat: bounds._southWest.lat,
                            lon: bounds._southWest.lng,
                        },
                    })
                };
                let filters = self.filters.clone();
                ctx.link().send_future(async {
                    match get_all_as_filtered(filters).await {
                        Ok(ases) => Msg::DrawAs(ases),
                        Err(e) => Msg::Error(e),
                    }
                });
            }
            Msg::UpdateFilters(filter) => {
                log!(format!("got filter update request for {filter:?}"));
                match filter {
                    FilterForm::MinAddresses(n) => {
                        self.filters.addresses = Some((n as i64, self.filters.addresses.unwrap().1))
                    }
                    FilterForm::MaxAddresses(n) => {
                        self.filters.addresses = Some((self.filters.addresses.unwrap().0, n as i64))
                    }
                    FilterForm::MinRank(n) => {
                        self.filters.rank = Some((n as i64, self.filters.rank.unwrap().1))
                    }
                    FilterForm::MaxRank(n) => {
                        self.filters.rank = Some((self.filters.rank.unwrap().0, n as i64))
                    }
                    FilterForm::CountryCode(code) => {
                        if code.is_empty() {
                            self.filters.country = None
                        } else {
                            self.filters.country = Some(code.to_uppercase())
                        }
                    }
                    FilterForm::IsBounded => {
                        // The value just needs to be some, it will be udated on load filtered button press
                        self.filters.bounds = if self.filters.bounds.is_none() {
                            Some(Bound {
                                north_east: Coord { lat: 0., lon: 0. },
                                south_west: Coord { lat: 0., lon: 0. },
                            })
                        } else {
                            None
                        };
                    }
                    FilterForm::HasOrg => {
                        self.filters.has_org = Some(!self.filters.has_org.unwrap())
                    }
                }
            }
            Msg::DrawAs(ases) => {
                log!(format!(
                    "{} ASes received to be drawn, drawing them signal at map_component.rs",
                    ases.len()
                ));
                let mut markers = Vec::new();
                for a in ases.into_iter() {
                    let asn = a.asn;
                    self.as_cache.insert(asn, a);
                    if self.drawn_filtered_ases.insert(asn) {
                        let as_ = self.as_cache.get(&asn).unwrap();
                        let marker_size = scale_as_marker(as_);
                        let country = celes::Country::from_alpha2(&as_.country_code);
                        let m = create_marker(
                            &format!(
                                "<b>asn</b>:{} <b>rank</b>:{} <b>prefixes</b>:{} <b>addresses</b>:{}
                                <b>links</b>:{},{}<br>
                                <b>name</b>:{}<br>
                                <b>org</b>:{}<br>
                                <b>country</b>:{}",
                                as_.asn,
                                as_.rank,
                                as_.prefixes,
                                as_.addresses,
                                format!("<a href=\"https://bgp.he.net/AS{asn}\" target=\"_blank\">bgp.he</a>"),
                                format!("<a href=\"https://bgpview.io/asn/{asn}\" target=\"_blank\">bgpview</a>"),
                                as_.name,
                                as_.organization.as_ref().map(|x| x.as_str()).unwrap_or("none"),
                                country.map(|c| c.long_name).unwrap_or(""),
                            ),
                            &Point(
                                as_.coordinates.lat,
                                as_.coordinates.lon,
                            ),
                            marker_size,
                        );
                        markers.push(m);
                    };
                }
                // TODO fix readding the same markers after clearLayers(). For some reason only new ones get drawn
                self.marker_cluster.addLayers(markers);
            }
            Msg::ClearMarkers => {
                //self.ases.clear();
                self.drawn_filtered_ases.clear();
                self.marker_cluster.clearLayers();
            }
            Msg::DownloadFiltered => {
                let ases = self
                    .as_cache
                    .iter()
                    .filter(|(asn, _)| self.drawn_filtered_ases.contains(asn))
                    .map(|(_, as_t)| as_t);
                self.create_downloadable_csv(ases);
            }
            Msg::DownloadAllCached => {
                let ases = self.as_cache.iter().map(|(_, as_t)| as_t);
                self.create_downloadable_csv(ases);
            }
            Msg::ShowAllCached => {
                self.drawn_filtered_ases.clear();
                self.marker_cluster.clearLayers();
                let ases = self.as_cache.iter().map(|(_, v)| v.clone()).collect();
                ctx.link().send_future(async { Msg::DrawAs(ases) });
            }
            Msg::ShowFilteredCached => {
                // There is no way to implement that with AsForFrontend as it is now
                // I will have to store used filters in a Vec cache, then store filter refs in the ases HashMap.
                // However in case of implementing full sessionStorage caching this will necessitate
                // sending a list of skipped ases which were sens in full before so that I can have a information whether given
                // as matches with the current filter.

                // matching bouds will be ok as we have exact coordinates
                // ok whatever, we have addresses, rank, coords, org and country name but not code right now so we can filter it
                // directly on the frontend. I may send jsut coutry code to fronted and then resolve it using a lib for that
                // so previous used filter will be enough, no need of caching more of them
                self.drawn_filtered_ases.clear();
                self.marker_cluster.clearLayers();

                let bounds: models::LatLngBounds =
                    serde_wasm_bindgen::from_value(self.map.getBounds().into()).unwrap();
                let ases = self.as_cache.iter()
                .filter(|(_asn, a)|
                    a.addresses >= self.filters.addresses.as_ref().unwrap().0 as u32
                    && a.addresses <= self.filters.addresses.as_ref().unwrap().1 as u32
                    && a.rank >= self.filters.rank.as_ref().unwrap().0 as u32
                    && a.rank <= self.filters.rank.as_ref().unwrap().1 as u32
                    // TODO pass country code instead of name so the comparison works
                    && &a.country_code == self.filters.country.as_ref().unwrap()
                    && a.organization.is_some() == self.filters.has_org.is_some()
                    // TODO take the bounds, pull the most up to date ones, compare if needed
                    && ((self.filters.bounds.is_some()
                        && (a.coordinates.lat <= bounds._northEast.lat && a.coordinates.lat >= bounds._southWest.lat)
                        && (a.coordinates.lon <= bounds._northEast.lng && a.coordinates.lon >= bounds._southWest.lng)
                        )
                    || self.filters.bounds.is_none()
                    )
                )
                .map(|(_, v)| v.clone()).collect();
                ctx.link().send_future(async { Msg::DrawAs(ases) });
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
                <div style="display: flex; flex-flow: column wrap;">
                    {Self::load_all_as_button(self, ctx)}
                    {Self::load_as_bounded_button(self, ctx)}
                    {Self::download_button(self, ctx)}
                    {Self::clear_button(self, ctx)}
                </div>
                <div>
                    {Self::filter_menu(self, ctx)}
                    {Self::load_as_filtered_button(self, ctx)}
                </div>
                <div style="display: flex; flex-flow: column wrap;">
                    {Self::show_cached_button(self, ctx)}
                    {Self::show_cached_filtered_button(self, ctx)}
                    {Self::download_cached_button(self, ctx)}
                </div>
                <div>
                    {Self::counter(self, ctx)}
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IconOpts {
    pub icon_url: String,
    pub icon_size: Vec<u64>,
    pub class_name: String,
}

fn create_marker(description: &str, coord: &Point, size: (u64, u64)) -> Marker {
    let opts = JsValue::from_str(r#"{"opacity": "0.5"}"#);
    let latlng = LatLng::new(coord.0, coord.1);
    let m = Marker::new_with_options(&latlng, &opts);

    let p = JsValue::from_str(description);
    m.bindPopup(&p, &JsValue::from_str("popup"));

    let i = Icon::new(
        &serde_wasm_bindgen::to_value(&IconOpts {
            icon_url: MARKER_ICON_URL.to_string(),
            icon_size: vec![size.0, size.1],
            class_name: "test-classname".to_string(),
        })
        .unwrap(),
    );
    m.setIcon(&i);
    m
}

/// returns (width, height) in pixels based on
/// by rank or by addresses amount? both would suit
/// both may be used, 1 as color other as marker size
fn scale_as_marker(a: &AsForFrontend) -> (u64, u64) {
    const RANK_RANGE: (u64, u64) = (0, 115000); // 0 not needed likely
                                                // const ADDRESS_RANGE: (u64, u64) = (0, 20017664);
    const AVG_PIXELS: (u64, u64) = (15, 24); //original is 25,41
    const MIN_PIXELS: (u64, u64) = (5, 8);
    let rank = a.rank;
    let scale = (rank as f64 / RANK_RANGE.1 as f64).clamp(0., 1.);
    // let addresses = a.asrank_data.as_ref().unwrap().addresses;
    // let scale = (addresses as f64 / ADDRESS_RANGE.1 as f64).clamp(0., 1.);
    let width = MIN_PIXELS.0 + AVG_PIXELS.0 - (AVG_PIXELS.0 as f64 * scale) as u64;
    let height = MIN_PIXELS.1 + AVG_PIXELS.1 - (AVG_PIXELS.1 as f64 * scale) as u64;

    (width, height)
}
