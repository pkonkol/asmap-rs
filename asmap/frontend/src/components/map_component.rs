use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    sync::{Arc, Mutex},
};

use gloo_console::log;
use gloo_file::{Blob, ObjectUrl};
use gloo_utils::document;
use js_sys::Object;
use leaflet::{
    Icon, IconOptions, LatLng, Map, MapOptions, Marker, MarkerOptions, Popup, PopupOptions,
    TileLayer, Tooltip, TooltipOptions,
};
use protocol::{AsFilters, AsFiltersHasOrg, AsForFrontend};
use serde::Deserialize;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_timer::SystemTime;
use web_sys::{Element, HtmlCollection, HtmlElement, HtmlInputElement, Node};
use yew::prelude::*;

use super::api::{get_all_as_filtered, get_as_details, get_as_whois};
use crate::models::{self, CsvAs, CsvAsDetailed, DownloadableCsvInput};
use asdb_models::{As, Bound, Coord};
use leaflet_markercluster::{markerClusterGroup, MarkerClusterGroup};

const POLAND_LAT: f64 = 52.11431;
const POLAND_LON: f64 = 19.423672;
const MARKER_ICON_URL: &str = "https://unpkg.com/leaflet@1.9.3/dist/images/marker-icon.png";

pub enum Msg {
    LoadAsBounded,
    LoadAsFiltered,
    GetDetails(u32, u64),
    UpdateFilters(FilterForm),
    ClearMarkers,
    DrawAs(Vec<AsForFrontend>),
    UpdateDetails(As, u64),
    DownloadFiltered,
    DownloadDetailed,

    // WHOIS - NEW
    SetActive(u32, u64),               // asn, marker_id (on popup open)
    CheckWhois(u32, u64),              // asn, marker_id (button click)
    UpdateWhois(u32, u64, String),     // asn, marker_id, whois text
    Noop,

    Error(anyhow::Error),
}

#[derive(Debug)]
pub enum FilterForm {
    HasOrg(String),
    MinAddresses(u64),
    MaxAddresses(u64),
    CountryCode(String),
    ExcludeCountry,
    MinRank(u64),
    MaxRank(u64),
    IsBounded,
    Category(Vec<String>),
}

pub struct MapComponent {
    map: Map,
    container: HtmlElement,
    marker_cluster: MarkerClusterGroup,
    /// Cached ASes which were manually opened and their detail downloaded
    detailed_ases: HashMap<u32, As>,
    /// these are actually just last drawn ases and serve as a proxy for the last filter use
    drawn_ases: HashMap<u32, AsForFrontend>,
    /// Filters that will be used when pressing load button
    next_filters: AsFilters,
    /// Filters executed during previous load
    prev_filters: AsFilters,

    // WHOIS - NEW
    active_asn: Option<u32>,
    active_marker_id: Option<u64>,
    whois_cache: HashMap<u32, String>,
    whois_loading: HashSet<u32>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Point(pub f64, pub f64);

#[derive(Properties, PartialEq, Clone)]
pub struct Props {}

// ============================================================================
// UI COMPONENTS - Render filter menu, buttons, and debug info
// ============================================================================
impl MapComponent {
    fn filter_menu(&self, ctx: &Context<Self>) -> Html {
        // TODO GIS style tailwindowe
        // TODO GIS
        html! {
            <div class="space-y-4">
                // Bounded Checkbox
                <div class="p-3 rounded-lg bg-slate-800/50 border border-slate-700">
                <div class="flex items-center gap-2 pt-1">
                    <input
                        type="checkbox"
                        id="isBounded"
                        checked={self.next_filters.bounds.is_some()}
                        class="w-4 h-4 bg-slate-900 border-slate-600 rounded focus:ring-2 focus:ring-blue-500"
                        oninput={ctx.link().callback(|_e: InputEvent| {
                            Msg::UpdateFilters(FilterForm::IsBounded)
                        })}
                    />
                    <label for="isBounded" class="text-xs text-slate-400 cursor-pointer">{"Bound to visible area"}</label>
                </div>
                </div>
                // Address Range Filter
                <div class="p-3 rounded-lg bg-slate-800/50 border border-slate-700">
                    <div class="space-y-2">
                        <label class="block text-xs font-medium text-slate-400">{"Min Addresses"}</label>
                        <input
                            type="number"
                            id="minAddresses"
                            value={self.next_filters.addresses.unwrap().0.to_string()}
                            min="0"
                            max="99999999"
                            class="w-full px-3 py-1.5 bg-slate-900 border border-slate-600 rounded text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            oninput={ctx.link().callback(|e: InputEvent| {
                                Msg::UpdateFilters(FilterForm::MinAddresses(
                                    e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                            })}
                        />

                        <label class="block text-xs font-medium text-slate-400 mt-3">{"Max Addresses"}</label>
                        <input
                            type="number"
                            id="maxAddresses"
                            value={self.next_filters.addresses.unwrap().1.to_string()}
                            min="0"
                            max="99999999"
                            class="w-full px-3 py-1.5 bg-slate-900 border border-slate-600 rounded text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            oninput={ctx.link().callback(|e: InputEvent| {
                                Msg::UpdateFilters(FilterForm::MaxAddresses(
                                    e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                            })}
                        />
                    </div>
                </div>

                // Country Filter
                <div class="p-3 rounded-lg bg-slate-800/50 border border-slate-700">
                    <div class="space-y-2">
                        <label class="block text-xs font-medium text-slate-400">{"Country Code"}</label>
                        <input
                            type="text"
                            id="countryCode"
                            value={self.next_filters.country.clone()}
                            maxlength="2"
                            placeholder="PL"
                            class="w-20 px-3 py-1.5 bg-slate-900 border border-slate-600 rounded text-sm text-slate-200 uppercase focus:outline-none focus:ring-2 focus:ring-blue-500"
                            oninput={ctx.link().callback(|e: InputEvent| {
                                Msg::UpdateFilters(FilterForm::CountryCode(
                                    e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                            })}
                        />

                        <div class="flex items-center gap-2 mt-2">
                            <input
                                type="checkbox"
                                id="excludeCountry"
                                checked={self.next_filters.exclude_country}
                                class="w-4 h-4 bg-slate-900 border-slate-600 rounded focus:ring-2 focus:ring-blue-500"
                                oninput={ctx.link().callback(|_e: InputEvent| {
                                    Msg::UpdateFilters(FilterForm::ExcludeCountry)
                                })}
                            />
                            <label for="excludeCountry" class="text-xs text-slate-400 cursor-pointer">{"Exclude country"}</label>
                        </div>
                    </div>
                </div>

                // Rank Range Filter
                <div class="p-3 rounded-lg bg-slate-800/50 border border-slate-700">
                    <div class="space-y-2">
                        <label class="block text-xs font-medium text-slate-400">{"Min Rank"}</label>
                        <input
                            type="number"
                            id="minRank"
                            value={self.next_filters.rank.unwrap().0.to_string()}
                            min="0"
                            max="999999"
                            class="w-24 px-3 py-1.5 bg-slate-900 border border-slate-600 rounded text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            oninput={ctx.link().callback(|e: InputEvent| {
                                Msg::UpdateFilters(FilterForm::MinRank(
                                    e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                            })}
                        />

                        <label class="block text-xs font-medium text-slate-400 mt-3">{"Max Rank"}</label>
                        <input
                            type="number"
                            id="maxRank"
                            value={self.next_filters.rank.unwrap().1.to_string()}
                            min="0"
                            max="999999"
                            class="w-24 px-3 py-1.5 bg-slate-900 border border-slate-600 rounded text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
                            oninput={ctx.link().callback(|e: InputEvent| {
                                Msg::UpdateFilters(FilterForm::MaxRank(
                                    e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                            })}
                        />
                    </div>
                </div>

                // Organization & Bounds Filter
                <div class="p-3 rounded-lg bg-slate-800/50 border border-slate-700">
                    <div class="space-y-3">
                        <div>
                            <label class="block text-xs font-medium text-slate-400 mb-2">{"Has Organization"}</label>
                            <select
                                id="hasOrg"
                                name="hasOrgSel"
                                class="w-full px-3 py-1.5 bg-slate-900 border border-slate-600 rounded text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                onchange={ctx.link().callback(|e: Event| {
                                    let selected = js_sys::Reflect::get(&e.target().unwrap(), &JsValue::from_str("value")).unwrap().as_string().unwrap();
                                    Msg::UpdateFilters(FilterForm::HasOrg(selected))
                            })}>
                                <option value="yes">{"Yes"}</option>
                                <option value="no">{"No"}</option>
                                <option value="both">{"Both"}</option>
                            </select>
                        </div>

                    </div>
                </div>

                // Category Filter
                <div class="p-3 rounded-lg bg-slate-800/50 border border-slate-700">
                    <label class="block text-xs font-medium text-slate-400 mb-2">{"Category"}</label>
                    <select
                        id="category"
                        name="category"
                        multiple=true
                        class="w-full h-40 px-3 py-2 bg-slate-900 border border-slate-600 rounded text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
                        onchange={ctx.link().callback(|e: Event| {
                            let selected_options = js_sys::Reflect::get(&e.target().unwrap(), &JsValue::from_str("selectedOptions"))
                                .unwrap()
                                .dyn_into::<HtmlCollection>()
                                .unwrap();

                            let mut selected = vec![];
                            for i in 0..selected_options.length() {
                                let item = selected_options.item(i).unwrap();
                                let category = js_sys::Reflect::get(&item, &JsValue::from_str("text")).unwrap().as_string().unwrap();
                                selected.push(category);
                            };
                            Msg::UpdateFilters(FilterForm::Category(selected))
                    })}>
                        <option value="Any">{"Any"}</option>
                        { asdb_models::categories::CATEGORIES.iter().map(|(category, _subcategories)| {
                            html!{<option value={ *category }>{ *category }</option>}
                        }).collect::<Html>() }
                    </select>
                </div>
            </div>
        }
    }

    fn load_as_bounded_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::LoadAsBounded);
        html! {
            <button
                onclick={cb}
                class="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 active:bg-blue-800 text-white text-sm font-medium rounded-lg transition-colors duration-150"
            >
                {"Load visible range"}
            </button>
        }
    }

    fn load_as_filtered_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::LoadAsFiltered);
        html! {
            <button
                onclick={cb}
                class="w-full px-4 py-2 bg-green-600 hover:bg-green-700 active:bg-green-800 text-white text-sm font-medium rounded-lg transition-colors duration-150"
            >
                {"Apply filters →"}
            </button>
        }
    }

    fn download_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::DownloadFiltered);
        html! {
            <button
                onclick={cb}
                class="w-full px-4 py-2 bg-slate-700 hover:bg-slate-600 active:bg-slate-500 text-slate-200 text-sm font-medium rounded-lg transition-colors duration-150"
            >
                {"Download loaded"}
            </button>
        }
    }

    fn download_detailed_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::DownloadDetailed);
        html! {
            <button
                onclick={cb}
                class="w-full px-4 py-2 bg-slate-700 hover:bg-slate-600 active:bg-slate-500 text-slate-200 text-sm font-medium rounded-lg transition-colors duration-150"
            >
                {"Download detailed"}
            </button>
        }
    }

    fn whois_button(&self, ctx: &Context<Self>) -> Html {
        let (disabled, label) = match (self.active_asn, self.active_marker_id) {
            (Some(asn), Some(_)) => {
                let loading = self.whois_loading.contains(&asn);
                (false, if loading { "Checking WHOIS..." } else { "Check WHOIS" })
            }
            _ => (true, "Check WHOIS"),
        };

        let active = (self.active_asn, self.active_marker_id);
        let cb = ctx.link().callback(move |_| {
            if let (Some(asn), Some(mid)) = active {
                Msg::CheckWhois(asn, mid)
            } else {
                Msg::Noop
            }
        });

        html! {
            <button
                disabled={disabled}
                onclick={cb}
                class={classes!("w-full","px-4","py-2","text-sm", "font-medium", "rounded-lg", "transition-colors", "duration-150",
                                if disabled {
                                    "bg-slate-700 text-slate-400 cursor-not-allowed"
                                } else {
                                    "bg-slate-700 hover:bg-slate-600 active:bg-slate-500 text-white"
                                }
                            )}
            >
                { label }
            </button>
        }
    }

    fn clear_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::ClearMarkers);
        html! {
            <button
                onclick={cb}
                class="w-full px-4 py-2 bg-red-600 hover:bg-red-700 active:bg-red-800 text-white text-sm font-medium rounded-lg transition-colors duration-150"
            >
                {"Clear map"}
            </button>
        }
    }

    fn debug_counter(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <div class="space-y-1">
                <div class="flex justify-between">
                    <span class="font-semibold text-slate-400">{"Drawn:"}</span>
                    <span class="text-slate-200">{self.drawn_ases.len()}</span>
                </div>
                <div class="flex justify-between">
                    <span class="font-semibold text-slate-400">{"Detailed:"}</span>
                    <span class="text-slate-200">{self.detailed_ases.len()}</span>
                </div>
            </div>
        }
    }

    fn whois_panel(&self) -> Html {
        if let Some(asn) = self.active_asn {
            if let Some(w) = self.whois_cache.get(&asn) {
                return html! {
                    <div class="p-3 rounded-lg border border-slate-800 bg-slate-950/60 text-sm">
                        <div class="text-xs text-slate-400 mb-2">{ format!("WHOIS (AS{})", asn) }</div>
                        <pre class="text-xs whitespace-pre-wrap max-h-64 overflow-auto">{ w.clone() }</pre>
                    </div>
                };
            }
        }
        html! {}
    }
}

// ============================================================================
// UTILITY METHODS - CSV generation, downloads, and map rendering
// ============================================================================
impl MapComponent {
    fn render_map(&self) -> Html {
        let node: &Node = &self.container.clone().into();
        Html::VRef(node.clone())
    }

    fn get_simple_csv_writer<'a>(
        &self,
        ases: impl Iterator<Item = &'a AsForFrontend>,
    ) -> (csv::Writer<Vec<u8>>, u64) {
        let mut wtr = csv::Writer::from_writer(Vec::new());
        let mut ases_len = 0u64;
        for a in ases {
            ases_len += 1;
            wtr.serialize(CsvAs::from(a)).unwrap();
        }
        wtr.flush().unwrap();
        (wtr, ases_len)
    }

    fn get_detailed_csv_writer<'a>(&self, ases: impl Iterator<Item = &'a As>) -> (csv::Writer<Vec<u8>>, u64) {
        let mut wtr = csv::Writer::from_writer(Vec::new());
        let mut ases_len = 0u64;
        for a in ases {
            ases_len += 1;
            wtr.serialize(CsvAsDetailed::from(a)).unwrap();
        }
        wtr.flush().unwrap();
        (wtr, ases_len)
    }

    fn create_downloadable_csv(&self, input: DownloadableCsvInput) {
        let document = web_sys::window().unwrap().document().unwrap();
        let body = document.body().expect("document should have a body");

        let (wtr, filename) = match input {
            DownloadableCsvInput::Simple(x) => {
                let (wtr, ases_len) = self.get_simple_csv_writer(x);
                let filename = format!(
                    "asmap-{}-{}-{}.csv",
                    ases_len,
                    self.prev_filters,
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                );
                (wtr, filename)
            }
            DownloadableCsvInput::Detailed(x) => {
                let (wtr, ases_len) = self.get_detailed_csv_writer(x);
                let filename = format!(
                    "asmap-detailed-{}-{}.csv",
                    ases_len,
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                );
                (wtr, filename)
            }
        };

        let blob = Blob::new_with_options(wtr.into_inner().unwrap().as_slice(), Some("text/plain"));
        let object_url = ObjectUrl::from(blob);

        let tmp_download_link = document.create_element("a").unwrap();
        tmp_download_link.set_attribute("href", &object_url).unwrap();
        tmp_download_link.set_attribute("download", &filename).unwrap();

        let tmp_node = body.append_child(&tmp_download_link).unwrap();
        tmp_node.clone().dyn_into::<HtmlElement>().unwrap().click();
        body.remove_child(&tmp_node).unwrap();
    }
}

// ============================================================================
// COMPONENT LIFECYCLE - Create, render, update message handling
// ============================================================================
impl Component for MapComponent {
    type Message = Msg;
    type Properties = Props;

    fn create(_ctx: &Context<Self>) -> Self {
        let container: Element = document().create_element("div").unwrap();
        let container: HtmlElement = container.dyn_into().unwrap();
        container.set_class_name("map w-full h-full");
        // TODO GIS tailwind dla mapy

        let x: MapOptions = MapOptions::new();
        let leaflet_map = Map::new_with_element(&container, &x).unwrap();
        leaflet_map.set_max_zoom(18.0);

        // Configure marker clustering with dynamic radius based on zoom
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
        marker_cluster.add_to(&leaflet_map);

        let initial_filters = AsFilters {
            country: Some("PL".to_string()),
            exclude_country: false,
            bounds: None,
            addresses: Some((0, 21000000)),
            rank: Some((0, 115000)),
            has_org: AsFiltersHasOrg::Both,
            category: vec![],
        };

        Self {
            map: leaflet_map,
            container,
            marker_cluster,
            detailed_ases: HashMap::new(),
            drawn_ases: HashMap::new(),
            next_filters: initial_filters.clone(),
            prev_filters: initial_filters,

            // WHOIS - NEW
            active_asn: None,
            active_marker_id: None,
            whois_cache: HashMap::new(),
            whois_loading: HashSet::new(),
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, first_render: bool) {
        if first_render {
            self.map.set_view(&LatLng::new(POLAND_LAT, POLAND_LON), 5.0);
            add_tile_layer(&self.map);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadAsBounded => {
                let bounds: models::LatLngBounds =
                    serde_wasm_bindgen::from_value(self.map.get_bounds().into()).unwrap();
                log!(format!("load AS bounded initiatied, bounds: {bounds:?}"));

                let filters = AsFilters {
                    bounds: Some(Bound {
                        north_east: Coord {
                            lat: bounds._north_east.lat,
                            lon: bounds._north_east.lng,
                        },
                        south_west: Coord {
                            lat: bounds._south_west.lat,
                            lon: bounds._south_west.lng,
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
                // Update bounds dynamically if bounded filter is enabled
                if self.next_filters.bounds.is_some() {
                    let bounds: models::LatLngBounds =
                        serde_wasm_bindgen::from_value(self.map.get_bounds().into()).unwrap();
                    self.next_filters.bounds = Some(Bound {
                        north_east: Coord {
                            lat: bounds._north_east.lat,
                            lon: bounds._north_east.lng,
                        },
                        south_west: Coord {
                            lat: bounds._south_west.lat,
                            lon: bounds._south_west.lng,
                        },
                    })
                };
                let filters = self.next_filters.clone();
                self.prev_filters = filters.clone();
                ctx.link().send_future(async {
                    match get_all_as_filtered(filters).await {
                        Ok(ases) => Msg::DrawAs(ases),
                        Err(e) => Msg::Error(e),
                    }
                });
            }
            Msg::GetDetails(asn, marker_id) => {
                if !self.detailed_ases.contains_key(&asn) {
                    log!(format!(
                        "sending get details request for asn {asn} which has marker {marker_id}"
                    ));
                    ctx.link().send_future(async move {
                        match get_as_details(asn).await {
                            Ok(as_) => Msg::UpdateDetails(as_, marker_id),
                            Err(e) => Msg::Error(e),
                        }
                    });
                } else {
                    log!("as details are already in cache");
                }
            }
            Msg::SetActive(asn, marker_id) => {
                self.active_asn = Some(asn);
                self.active_marker_id = Some(marker_id);
            }
            Msg::CheckWhois(asn, marker_id) => {
                if self.whois_cache.contains_key(&asn) || self.whois_loading.contains(&asn) {
                    return true;
                }
                self.whois_loading.insert(asn);

                ctx.link().send_future(async move {
                    match get_as_whois(asn).await {
                        Ok(whois) => Msg::UpdateWhois(asn, marker_id, whois),
                        Err(e) => Msg::Error(e),
                    }
                });
            }
            Msg::UpdateWhois(asn, marker_id, whois) => {
                self.whois_loading.remove(&asn);
                self.whois_cache.insert(asn, whois.clone());

                // update popup content
                let marker = self.marker_cluster.getLayer(marker_id);
                let old_str = marker
                    .get_popup()
                    .get_content()
                    .as_string()
                    .unwrap_or_default();

                // minimal escape to avoid breaking HTML
                let safe = whois
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;");

                let appended = format!(
                    r#"{old}<br><b>whois</b>:<pre style="white-space:pre-wrap;max-height:250px;overflow:auto;">{safe}</pre>"#,
                    old = old_str
                );
                marker.set_popup_content(&JsValue::from_str(&appended));
            }
            Msg::UpdateFilters(filter) => {
                log!(format!("got filter update request for {filter:?}"));
                match filter {
                    FilterForm::MinAddresses(n) => {
                        self.next_filters.addresses =
                            Some((n as i64, self.next_filters.addresses.unwrap().1))
                    }
                    FilterForm::MaxAddresses(n) => {
                        self.next_filters.addresses =
                            Some((self.next_filters.addresses.unwrap().0, n as i64))
                    }
                    FilterForm::MinRank(n) => {
                        self.next_filters.rank = Some((n as i64, self.next_filters.rank.unwrap().1))
                    }
                    FilterForm::MaxRank(n) => {
                        self.next_filters.rank = Some((self.next_filters.rank.unwrap().0, n as i64))
                    }
                    FilterForm::CountryCode(code) => {
                        if code.is_empty() {
                            self.next_filters.country = None
                        } else {
                            self.next_filters.country = Some(code.to_uppercase())
                        }
                    }
                    FilterForm::ExcludeCountry => {
                        self.next_filters.exclude_country = !self.next_filters.exclude_country;
                    }
                    FilterForm::IsBounded => {
                        // Toggle bounded filter (actual bounds set on load)
                        self.next_filters.bounds = if self.next_filters.bounds.is_none() {
                            Some(Bound {
                                north_east: Coord { lat: 0., lon: 0. },
                                south_west: Coord { lat: 0., lon: 0. },
                            })
                        } else {
                            None
                        };
                    }
                    FilterForm::HasOrg(s) => {
                        self.next_filters.has_org = match s.as_str() {
                            "yes" => AsFiltersHasOrg::Yes,
                            "no" => AsFiltersHasOrg::No,
                            _ => AsFiltersHasOrg::Both,
                        };
                    }
                    FilterForm::Category(s) => {
                        self.next_filters.category = s;
                    }
                }
            }
            Msg::DrawAs(ases) => {
                log!(format!(
                    "{} ASes received to be drawn, drawing them signal at map_component.rs",
                    ases.len()
                ));

                let start = js_sys::Date::now();
                log!("generating markers");

                let markers = Arc::new(Mutex::new(Vec::new()));
                let ctx2 = Arc::new(Mutex::new(ctx));
                let mut drawn_ases2 = Arc::new(Mutex::new(self.drawn_ases.clone()));

                // Create markers for each AS
                ases.iter().for_each(|as_| {
                    let drawn_ases = Arc::clone(&mut drawn_ases2);
                    let asn = as_.asn;

                    // Skip if already drawn
                    if drawn_ases.lock().unwrap().contains_key(&asn) {
                        return ();
                    }
                    {
                        let mut xd = drawn_ases.lock().unwrap();
                        xd.insert(asn, as_.clone());
                    }

                    let marker_size = scale_as_marker(&as_);
                    let country = celes::Country::from_alpha2(&as_.country_code);

                    let m = create_marker(
                        &format!(
                            "<b>asn</b>:{} <b>rank</b>:{} <b>prefixes</b>:{} <b>addresses</b>:{}<br>
                            <b>links</b>:<a href=\"https://bgp.he.net/AS{asn}\" target=\"_blank\">bgp.he</a>, shodan<br>
                            <b>name</b>:{}<br>
                            <b>org</b>:{}<br>
                            <b>country</b>:{}",
                            as_.asn,
                            as_.rank,
                            as_.prefixes,
                            as_.addresses,
                            as_.name,
                            as_.organization.as_deref().unwrap_or("none"),
                            country.map(|c| c.long_name).unwrap_or(""),
                        ),
                        &format!("AS{}:{}:{:.20}",as_.asn, as_.name, as_.organization.as_deref().unwrap_or("")),
                        &Point(as_.coordinates.lat, as_.coordinates.lon),
                        marker_size,
                    );

                    // popup open handler: set active + fetch details
                    let set_active_cb = ctx2
                        .lock()
                        .unwrap()
                        .link()
                        .callback(move |marker_id: u64| Msg::SetActive(asn, marker_id));

                    let details_cb = ctx2
                        .lock()
                        .unwrap()
                        .link()
                        .callback(move |marker_id: u64| Msg::GetDetails(asn, marker_id));

                    let detail_update_closure = Closure::wrap(Box::new(move |e: JsValue| {
                        #[derive(Deserialize, Debug)]
                        struct Target {
                            _leaflet_id: u64,
                        }
                        #[derive(Deserialize, Debug)]
                        struct LeafletId {
                            target: Target,
                        }

                        let x: Object = e.unchecked_into();
                        let m = serde_wasm_bindgen::from_value::<LeafletId>(x.into()).unwrap();
                        let id = m.target._leaflet_id;
                        log!(format!("marker id: {id}"));

                        set_active_cb.emit(id);
                        details_cb.emit(id);
                    }) as Box<dyn Fn(JsValue)>);

                    let js = detail_update_closure.into_js_value();
                    m.on("popupopen", &js);

                    markers.lock().unwrap().push(m);
                });

                log!(js_sys::Date::now() - start);
                log!("adding marker Layers to map");
                let mk = markers.lock().unwrap().clone();
                self.marker_cluster.addLayers(mk);
                log!("done");
                self.drawn_ases = drawn_ases2.lock().unwrap().clone();
            }
            Msg::UpdateDetails(as_, marker_id) => {
                log!(format!(
                    "got details for {}, proceeding to update popup in marker {marker_id}",
                    as_.asn
                ));
                let marker = self.marker_cluster.getLayer(marker_id);
                let mut old_str = marker.get_popup().get_content().as_string().unwrap_or_default();

                let mut details = String::new();
                details.push_str(&format!(
                    "<b>degree</b>: {}",
                    as_.asrank_data.as_ref().unwrap().degree
                ));

                // Add prefix details with Shodan links
                let prefixes = if let Some(ipnetdb) = as_.ipnetdb_data.as_ref() {
                    old_str = old_str.replace(
                        "shodan",
                        &format!(
                            "<a href=\"https://www.shodan.io/search?query=net:{}\" target=\"_blank\">shodan</a>",
                            ipnetdb
                                .ipv4_prefixes
                                .iter()
                                .map(|x| x.range.to_string())
                                .map(|mut x| {
                                    x.push(',');
                                    x
                                })
                                .collect::<String>()
                        ),
                    );
                    format!(
                        "<br><b>prefixes</b>: {}",
                        ipnetdb.ipv4_prefixes.iter().fold(String::new(), |mut output, x| {
                            let cidr = x.range.to_string();
                            let _ = write!(
                                output,
                                "{cidr}:<b><a href=\"https://www.shodan.io/search?query=net%3A{cidr}\" target=\"_blank\">s</a>\
                                </b>|<b><a href=\"https://www.zoomeye.org/searchResult?q=cidr%3A{cidr}\" target=\"_blank\">z</a>\
                                </b>|<b><a href=\"https://search.censys.io/search?resource=hosts&sort=RELEVANCE&per_page=25&virtual_hosts=EXCLUDE&q=ip%3A{cidr}\" target=\"_blank\">c</a></b> ",
                            );
                            output
                        })
                    )
                } else {
                    String::new()
                };
                details.push_str(&prefixes);

                // Add Stanford ASDB categories
                let mut stanford = HashSet::new();
                for c in as_.stanford_asdb.iter() {
                    stanford.insert(c.layer1.as_str());
                }
                details.push_str(&format!(
                    "<br><b>categories</b>: {}",
                    stanford.iter().fold(String::new(), |mut output, x| {
                        let _ = writeln!(output, "<b>></b>{x}<b>.</b><br>");
                        output
                    })
                ));

                marker.set_popup_content(&JsValue::from_str(&format!("{old_str}<br>{details}")));
                self.detailed_ases.insert(as_.asn, as_);
                log!("marker updated");
            }
            Msg::ClearMarkers => {
                self.drawn_ases.clear();
                self.detailed_ases.clear();
                self.marker_cluster.clearLayers();

                // reset whois + selection
                self.active_asn = None;
                self.active_marker_id = None;
                self.whois_cache.clear();
                self.whois_loading.clear();
            }
            Msg::DownloadFiltered => {
                let ases = self.drawn_ases.iter().map(|(_, as_t)| as_t);
                self.create_downloadable_csv(DownloadableCsvInput::Simple(Box::new(ases)));
            }
            Msg::DownloadDetailed => {
                let ases = self.detailed_ases.values();
                self.create_downloadable_csv(DownloadableCsvInput::Detailed(Box::new(ases)));
            }
            Msg::Noop => {}
            Msg::Error(e) => {
                log!(format!("error fetching ases, received error '{e:?}'"));
            }
        }
        true
    }

    fn changed(&mut self, _ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        true
    }

    // TODO GIS style tailwindowe
    // TODO GIS obsluga zapytania o skan whoisa w popupie i na stronie z detalami
    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="min-h-screen bg-slate-50 text-slate-100 flex flex-col md:flex-row gap-4 p-4">
                <div class="flex-1 min-h-[60vh] rounded-xl border border-slate-800 shadow-lg overflow-hidden">
                    <div class="h-full">{ self.render_map() }</div>
                </div>

                <div class="w-full md:w-96 space-y-4">
                    <div class="p-4 rounded-xl border border-slate-800 bg-slate-900/60 shadow">
                        <div class="flex flex-col gap-2">
                            { self.load_as_bounded_button(ctx) }
                            { self.load_as_filtered_button(ctx) }
                            { self.download_button(ctx) }
                            { self.download_detailed_button(ctx) }

                            // WHOIS - NEW
                            { self.whois_button(ctx) }

                            { self.clear_button(ctx) }
                        </div>
                    </div>

                    <div class="p-4 rounded-xl text-sm border border-slate-800 bg-slate-900/60 shadow">
                        { self.filter_menu(ctx) }
                    </div>

                    { self.whois_panel() }

                    <div class="p-3 rounded-lg border border-slate-800 bg-slate-900/60 text-sm">
                        { self.debug_counter(ctx) }
                    </div>
                </div>
            </div>
        }
    }
}

// ============================================================================
// LEAFLET HELPERS - Map initialization and marker creation
// ============================================================================
fn add_tile_layer(map: &Map) {
    TileLayer::new("https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png").add_to(map);
}

/// Creates a Leaflet marker with popup and tooltip
fn create_marker(description: &str, tooltip: &str, coord: &Point, size: (u64, u64)) -> Marker {
    // Create marker at coordinates
    let marker = Marker::new_with_options(&LatLng::new(coord.0, coord.1), &MarkerOptions::new());

    // Configure and bind popup (max width 600px)
    let popup = Popup::new(
        &{
            let opts = PopupOptions::new();
            opts.set_max_width(600.0);
            opts
        },
        None,
    );
    popup.set_content(&JsValue::from_str(description));
    marker.bind_popup(&popup);

    // Configure and bind tooltip
    let tooltip_elem = Tooltip::new(&TooltipOptions::new(), None);
    tooltip_elem.set_content(&JsValue::from_str(tooltip));
    marker.bind_tooltip(&tooltip_elem);

    // Set custom icon with size
    let icon = Icon::new(&{
        let opts = IconOptions::new();
        opts.set_class_name("test-classname".to_string());
        opts.set_icon_size(leaflet::Point::new(size.0 as f64, size.1 as f64));
        opts.set_icon_url(MARKER_ICON_URL.to_string());
        opts
    });
    marker.set_icon(&icon);

    marker
}

/// Scales marker size based on AS rank (lower rank = larger marker)
/// Returns (width, height) in pixels
fn scale_as_marker(a: &AsForFrontend) -> (u64, u64) {
    const RANK_RANGE: (u64, u64) = (0, 115000);
    const AVG_PIXELS: (u64, u64) = (15, 24);
    const MIN_PIXELS: (u64, u64) = (5, 8);

    let rank = a.rank;
    let scale = (rank as f64 / RANK_RANGE.1 as f64).clamp(0., 1.);
    let width = MIN_PIXELS.0 + AVG_PIXELS.0 - (AVG_PIXELS.0 as f64 * scale) as u64;
    let height = MIN_PIXELS.1 + AVG_PIXELS.1 - (AVG_PIXELS.1 as f64 * scale) as u64;

    (width, height)
}
