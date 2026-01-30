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

use super::api::{get_all_as_filtered, get_as_details, fetch_as_whois_data};
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
    /// Format large numbers with K/M suffix
    fn format_number(n: u64) -> String {
        if n >= 1_000_000 {
            format!("{:.1}M", n as f64 / 1_000_000.0)
        } else if n >= 1_000 {
            format!("{:.1}K", n as f64 / 1_000.0)
        } else {
            n.to_string()
        }
    }

    fn filter_menu(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="space-y-3">
                // Bounded Checkbox
                <div class="flex items-center gap-3 p-3 rounded-xl bg-slate-700/30 border border-slate-600/30 hover:bg-slate-700/40 transition-colors">
                    <input
                        type="checkbox"
                        id="isBounded"
                        checked={self.next_filters.bounds.is_some()}
                        class="w-4 h-4 bg-slate-800 border-slate-600 rounded text-blue-500 focus:ring-2 focus:ring-blue-500/50 focus:ring-offset-0"
                        oninput={ctx.link().callback(|_e: InputEvent| {
                            Msg::UpdateFilters(FilterForm::IsBounded)
                        })}
                    />
                    <label for="isBounded" class="text-sm text-slate-300 cursor-pointer select-none">{"Limit to visible area"}</label>
                </div>

                // Address Range Filter
                <div class="p-3 rounded-xl bg-slate-700/30 border border-slate-600/30">
                    <label class="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-3">{"Address Range"}</label>
                    <div class="grid grid-cols-2 gap-3">
                        <div>
                            <label class="block text-xs text-slate-500 mb-1">{"Min"}</label>
                            <input
                                type="number"
                                id="minAddresses"
                                value={self.next_filters.addresses.unwrap().0.to_string()}
                                min="0"
                                max="99999999"
                                class="w-full px-3 py-2 bg-slate-800/80 border border-slate-600/50 rounded-lg text-sm text-slate-200 
                                       focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500/50 transition-all"
                                oninput={ctx.link().callback(|e: InputEvent| {
                                    Msg::UpdateFilters(FilterForm::MinAddresses(
                                        e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                                })}
                            />
                        </div>
                        <div>
                            <label class="block text-xs text-slate-500 mb-1">{"Max"}</label>
                            <input
                                type="number"
                                id="maxAddresses"
                                value={self.next_filters.addresses.unwrap().1.to_string()}
                                min="0"
                                max="99999999"
                                class="w-full px-3 py-2 bg-slate-800/80 border border-slate-600/50 rounded-lg text-sm text-slate-200 
                                       focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500/50 transition-all"
                                oninput={ctx.link().callback(|e: InputEvent| {
                                    Msg::UpdateFilters(FilterForm::MaxAddresses(
                                        e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                                })}
                            />
                        </div>
                    </div>
                </div>

                // Country Filter
                <div class="p-3 rounded-xl bg-slate-700/30 border border-slate-600/30">
                    <label class="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-3">{"Country"}</label>
                    <div class="flex items-center gap-3">
                        <input
                            type="text"
                            id="countryCode"
                            value={self.next_filters.country.clone()}
                            maxlength="2"
                            placeholder="PL"
                            class="w-20 px-3 py-2 bg-slate-800/80 border border-slate-600/50 rounded-lg text-sm text-slate-200 uppercase 
                                   focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500/50 transition-all placeholder-slate-500"
                            oninput={ctx.link().callback(|e: InputEvent| {
                                Msg::UpdateFilters(FilterForm::CountryCode(
                                    e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                            })}
                        />
                        <label class="flex items-center gap-2 cursor-pointer select-none">
                            <input
                                type="checkbox"
                                id="excludeCountry"
                                checked={self.next_filters.exclude_country}
                                class="w-4 h-4 bg-slate-800 border-slate-600 rounded text-red-500 focus:ring-2 focus:ring-red-500/50 focus:ring-offset-0"
                                oninput={ctx.link().callback(|_e: InputEvent| {
                                    Msg::UpdateFilters(FilterForm::ExcludeCountry)
                                })}
                            />
                            <span class="text-xs text-slate-400">{"Exclude"}</span>
                        </label>
                    </div>
                </div>

                // Rank Range Filter
                <div class="p-3 rounded-xl bg-slate-700/30 border border-slate-600/30">
                    <label class="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-3">{"Rank Range"}</label>
                    <div class="grid grid-cols-2 gap-3">
                        <div>
                            <label class="block text-xs text-slate-500 mb-1">{"Min"}</label>
                            <input
                                type="number"
                                id="minRank"
                                value={self.next_filters.rank.unwrap().0.to_string()}
                                min="0"
                                max="999999"
                                class="w-full px-3 py-2 bg-slate-800/80 border border-slate-600/50 rounded-lg text-sm text-slate-200 
                                       focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500/50 transition-all"
                                oninput={ctx.link().callback(|e: InputEvent| {
                                    Msg::UpdateFilters(FilterForm::MinRank(
                                        e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                                })}
                            />
                        </div>
                        <div>
                            <label class="block text-xs text-slate-500 mb-1">{"Max"}</label>
                            <input
                                type="number"
                                id="maxRank"
                                value={self.next_filters.rank.unwrap().1.to_string()}
                                min="0"
                                max="999999"
                                class="w-full px-3 py-2 bg-slate-800/80 border border-slate-600/50 rounded-lg text-sm text-slate-200 
                                       focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500/50 transition-all"
                                oninput={ctx.link().callback(|e: InputEvent| {
                                    Msg::UpdateFilters(FilterForm::MaxRank(
                                        e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                                })}
                            />
                        </div>
                    </div>
                </div>

                // Organization Filter
                <div class="p-3 rounded-xl bg-slate-700/30 border border-slate-600/30">
                    <label class="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-3">{"Organization"}</label>
                    <select
                        id="hasOrg"
                        name="hasOrgSel"
                        class="w-full px-3 py-2 bg-slate-800/80 border border-slate-600/50 rounded-lg text-sm text-slate-200 
                               focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500/50 transition-all cursor-pointer"
                        onchange={ctx.link().callback(|e: Event| {
                            let selected = js_sys::Reflect::get(&e.target().unwrap(), &JsValue::from_str("value")).unwrap().as_string().unwrap();
                            Msg::UpdateFilters(FilterForm::HasOrg(selected))
                    })}>
                        <option value="yes">{"Has organization"}</option>
                        <option value="no">{"No organization"}</option>
                        <option value="both" selected=true>{"Both"}</option>
                    </select>
                </div>

                // Category Filter
                <div class="p-3 rounded-xl bg-slate-700/30 border border-slate-600/30">
                    <label class="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-3">{"Category"}</label>
                    <select
                        id="category"
                        name="category"
                        multiple=true
                        class="w-full h-32 px-3 py-2 bg-slate-800/80 border border-slate-600/50 rounded-lg text-sm text-slate-200 
                               focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500/50 transition-all"
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
                class="w-full px-4 py-2.5 bg-slate-700/60 hover:bg-slate-600/60 active:bg-slate-500/60 
                       text-slate-200 text-sm font-medium rounded-xl border border-slate-600/50
                       transition-all duration-200 hover:border-slate-500/50"
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
                class="w-full px-4 py-3 bg-gradient-to-r from-blue-600 to-blue-700 hover:from-blue-500 hover:to-blue-600 
                       active:from-blue-700 active:to-blue-800 text-white text-sm font-semibold rounded-xl 
                       shadow-lg shadow-blue-500/25 hover:shadow-blue-500/40
                       transition-all duration-200 flex items-center justify-center gap-2"
            >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
                </svg>
                {"Apply Filters"}
            </button>
        }
    }

    fn download_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::DownloadFiltered);
        html! {
            <button
                onclick={cb}
                class="w-full px-3 py-2 bg-slate-700/40 hover:bg-slate-600/50 active:bg-slate-500/50 
                       text-slate-300 text-xs font-medium rounded-lg border border-slate-600/40
                       transition-all duration-200 flex items-center justify-center gap-1.5"
            >
                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
                </svg>
                {"CSV"}
            </button>
        }
    }

    fn download_detailed_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::DownloadDetailed);
        html! {
            <button
                onclick={cb}
                class="w-full px-3 py-2 bg-slate-700/40 hover:bg-slate-600/50 active:bg-slate-500/50 
                       text-slate-300 text-xs font-medium rounded-lg border border-slate-600/40
                       transition-all duration-200 flex items-center justify-center gap-1.5"
            >
                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                </svg>
                {"Detailed"}
            </button>
        }
    }

    fn clear_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::ClearMarkers);
        html! {
            <button
                onclick={cb}
                class="w-full px-4 py-2.5 bg-red-600/20 hover:bg-red-600/30 active:bg-red-600/40 
                       text-red-400 hover:text-red-300 text-sm font-medium rounded-xl border border-red-600/30
                       transition-all duration-200 flex items-center justify-center gap-2"
            >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                </svg>
                {"Clear Map"}
            </button>
        }
    }

    fn debug_counter(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <div class="grid grid-cols-2 gap-3">
                <div class="p-3 rounded-xl bg-emerald-500/10 border border-emerald-500/20">
                    <p class="text-2xl font-bold text-emerald-400 tabular-nums">{self.drawn_ases.len()}</p>
                    <p class="text-xs text-slate-400">{"Drawn"}</p>
                </div>
                <div class="p-3 rounded-xl bg-purple-500/10 border border-purple-500/20">
                    <p class="text-2xl font-bold text-purple-400 tabular-nums">{self.detailed_ases.len()}</p>
                    <p class="text-xs text-slate-400">{"Detailed"}</p>
                </div>
            </div>
        }
    }

    fn whois_panel(&self) -> Html {
        if let Some(asn) = self.active_asn {
            if let Some(w) = self.whois_cache.get(&asn) {
                return html! {
                    <div class="p-4 rounded-2xl bg-slate-800/40 border border-slate-700/50 backdrop-blur-sm">
                        <div class="flex items-center gap-2 mb-3">
                            <svg class="w-4 h-4 text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                            </svg>
                            <span class="text-sm font-semibold text-slate-300">{ format!("WHOIS AS{}", asn) }</span>
                        </div>
                        <pre class="text-xs text-slate-400 whitespace-pre-wrap max-h-48 overflow-auto p-3 rounded-lg bg-slate-900/50 border border-slate-700/30">{ w.clone() }</pre>
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
                    match fetch_as_whois_data(asn).await {
                        Ok(Some(whois)) => {
                            // Format WhoIsAsn as string for display
                            let mut text = String::new();
                            if let Some(name) = &whois.as_name {
                                text.push_str(&format!("AS Name: {}\n", name));
                            }
                            if !whois.descr.is_empty() {
                                text.push_str(&format!("Description: {}\n", whois.descr.join(", ")));
                            }
                            if let Some(country) = &whois.country {
                                text.push_str(&format!("Country: {}\n", country));
                            }
                            if let Some(org) = &whois.organisation {
                                text.push_str(&format!("\nOrganisation: {}\n", org.org_name));
                                if !org.address.is_empty() {
                                    text.push_str(&format!("Address: {}\n", org.address.join(", ")));
                                }
                                if let Some(email) = &org.email {
                                    text.push_str(&format!("Email: {}\n", email));
                                }
                            }
                            if !whois.contacts.is_empty() {
                                text.push_str(&format!("\nContacts ({}):\n", whois.contacts.len()));
                                for c in &whois.contacts {
                                    text.push_str(&format!("  - {} ({})\n", c.name, c.nic_hdl));
                                }
                            }
                            log!(format!("WHOIS data for AS{}: {:?}", asn, whois));
                            Msg::UpdateWhois(asn, marker_id, text)
                        }
                        Ok(None) => {
                            log!(format!("No WHOIS data found for AS{}", asn));
                            Msg::UpdateWhois(asn, marker_id, "No WHOIS data available".to_string())
                        }
                        Err(e) => Msg::Error(e),
                    }
                });
            }
            Msg::UpdateWhois(asn, _marker_id, whois) => {
                // Just cache WHOIS data, don't add to popup
                self.whois_loading.remove(&asn);
                self.whois_cache.insert(asn, whois);
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
                            r#"<div style="font-family: system-ui, -apple-system, sans-serif; min-width: 300px; background: linear-gradient(135deg, #0f172a 0%, #1e293b 100%); border-radius: 16px; padding: 16px; border: 1px solid rgba(71, 85, 105, 0.5); box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);">
                            <div style="display: flex; align-items: center; gap: 12px; margin-bottom: 16px; padding-bottom: 12px; border-bottom: 1px solid rgba(71, 85, 105, 0.4);">
                                <div style="background: linear-gradient(135deg, #3b82f6, #1d4ed8); padding: 8px 14px; border-radius: 10px; box-shadow: 0 4px 15px rgba(59, 130, 246, 0.3);">
                                    <span style="color: white; font-weight: 700; font-size: 15px; letter-spacing: -0.5px;">AS{}</span>
                                </div>
                                <div style="flex: 1; min-width: 0;">
                                    <div style="font-weight: 600; color: #f1f5f9; font-size: 14px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">{}</div>
                                    <div style="color: #94a3b8; font-size: 11px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">{}</div>
                                </div>
                            </div>
                            <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; margin-bottom: 16px;">
                                <div style="background: rgba(251, 191, 36, 0.1); padding: 10px 8px; border-radius: 10px; text-align: center; border: 1px solid rgba(251, 191, 36, 0.2);">
                                    <div style="color: #fbbf24; font-weight: 700; font-size: 16px;">#{}</div>
                                    <div style="color: #64748b; font-size: 9px; text-transform: uppercase; letter-spacing: 0.5px; margin-top: 2px;">Rank</div>
                                </div>
                                <div style="background: rgba(52, 211, 153, 0.1); padding: 10px 8px; border-radius: 10px; text-align: center; border: 1px solid rgba(52, 211, 153, 0.2);">
                                    <div style="color: #34d399; font-weight: 700; font-size: 16px;">{}</div>
                                    <div style="color: #64748b; font-size: 9px; text-transform: uppercase; letter-spacing: 0.5px; margin-top: 2px;">Prefixes</div>
                                </div>
                                <div style="background: rgba(167, 139, 250, 0.1); padding: 10px 8px; border-radius: 10px; text-align: center; border: 1px solid rgba(167, 139, 250, 0.2);">
                                    <div style="color: #a78bfa; font-weight: 700; font-size: 16px;">{}</div>
                                    <div style="color: #64748b; font-size: 9px; text-transform: uppercase; letter-spacing: 0.5px; margin-top: 2px;">IPs</div>
                                </div>
                            </div>
                            <div style="display: flex; gap: 8px; flex-wrap: wrap;">
                                <a href="/details/{asn}" target="_blank" style="display: inline-flex; align-items: center; gap: 6px; padding: 8px 14px; background: linear-gradient(135deg, #3b82f6, #2563eb); color: white; border-radius: 8px; text-decoration: none; font-size: 12px; font-weight: 600; box-shadow: 0 4px 12px rgba(59, 130, 246, 0.25); transition: all 0.2s;">Details</a>
                                <a href="https://bgp.he.net/AS{asn}" target="_blank" style="display: inline-flex; align-items: center; gap: 6px; padding: 8px 14px; background: rgba(71, 85, 105, 0.5); color: #e2e8f0; border-radius: 8px; text-decoration: none; font-size: 12px; font-weight: 500; border: 1px solid rgba(71, 85, 105, 0.5);">BGP.HE</a>
                                <span id="shodan-link-{asn}" style="display: inline-flex; align-items: center; gap: 6px; padding: 8px 14px; background: rgba(51, 65, 85, 0.5); color: #64748b; border-radius: 8px; font-size: 12px; border: 1px solid rgba(51, 65, 85, 0.5);">Shodan</span>
                            </div>
                            </div>"#,
                            as_.asn,
                            as_.name,
                            as_.organization.as_deref().unwrap_or("—"),
                            as_.rank,
                            as_.prefixes,
                            Self::format_number(as_.addresses as u64),
                        ),
                        &format!("AS{}:{}:{:.20}",as_.asn, as_.name, as_.organization.as_deref().unwrap_or("")),
                        &Point(as_.coordinates.lat, as_.coordinates.lon),
                        marker_size,
                    );

                    // popup open handler: set active + fetch details + fetch whois
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

                    let whois_cb = ctx2
                        .lock()
                        .unwrap()
                        .link()
                        .callback(move |marker_id: u64| Msg::CheckWhois(asn, marker_id));

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
                        whois_cb.emit(id);
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

                // Build styled details section
                let degree = as_.asrank_data.as_ref().map(|d| d.degree.total).unwrap_or(0);
                
                let mut details = String::from(r#"<div style="margin-top: 16px; padding-top: 16px; border-top: 1px solid rgba(71, 85, 105, 0.4);">"#);
                
                // Degree badge
                details.push_str(&format!(
                    r#"<div style="display: inline-flex; align-items: center; gap: 8px; margin-bottom: 12px;">
                        <span style="background: rgba(249, 115, 22, 0.15); color: #fb923c; padding: 6px 12px; border-radius: 8px; font-size: 12px; font-weight: 600; border: 1px solid rgba(249, 115, 22, 0.2);">{} connections</span>
                    </div>"#,
                    degree
                ));

                // Add prefix details with Shodan links
                if let Some(ipnetdb) = as_.ipnetdb_data.as_ref() {
                    // Update shodan placeholder with actual link
                    let first_prefix = ipnetdb.ipv4_prefixes.first().map(|p| p.range.to_string()).unwrap_or_default();
                    old_str = old_str.replace(
                        &format!(r#"<span id="shodan-link-{}" style="display: inline-flex; align-items: center; gap: 6px; padding: 8px 14px; background: rgba(51, 65, 85, 0.5); color: #64748b; border-radius: 8px; font-size: 12px; border: 1px solid rgba(51, 65, 85, 0.5);">Shodan</span>"#, as_.asn),
                        &format!(
                            r#"<a href="https://www.shodan.io/search?query=net:{}" target="_blank" style="display: inline-flex; align-items: center; gap: 6px; padding: 8px 14px; background: linear-gradient(135deg, #dc2626, #b91c1c); color: white; border-radius: 8px; text-decoration: none; font-size: 12px; font-weight: 600; box-shadow: 0 4px 12px rgba(220, 38, 38, 0.25);">Shodan</a>"#,
                            first_prefix
                        ),
                    );
                    
                    // Prefix list
                    if !ipnetdb.ipv4_prefixes.is_empty() {
                        details.push_str(r#"<div style="margin-top: 12px;"><div style="color: #94a3b8; font-size: 10px; text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 8px; font-weight: 600;">Prefixes</div><div style="display: flex; flex-wrap: wrap; gap: 6px;">"#);
                        for prefix in ipnetdb.ipv4_prefixes.iter().take(8) {
                            let cidr = prefix.range.to_string();
                            details.push_str(&format!(
                                r#"<div style="background: rgba(30, 41, 59, 0.8); padding: 6px 10px; border-radius: 6px; font-size: 11px; border: 1px solid rgba(71, 85, 105, 0.3);">
                                    <span style="color: #e2e8f0; font-weight: 500;">{cidr}</span>
                                    <a href="https://www.shodan.io/search?query=net%3A{cidr}" target="_blank" style="color: #ef4444; margin-left: 6px; text-decoration: none; font-weight: 600;">S</a>
                                    <a href="https://www.zoomeye.org/searchResult?q=cidr%3A{cidr}" target="_blank" style="color: #3b82f6; margin-left: 4px; text-decoration: none; font-weight: 600;">Z</a>
                                    <a href="https://search.censys.io/search?resource=hosts&q=ip%3A{cidr}" target="_blank" style="color: #a855f7; margin-left: 4px; text-decoration: none; font-weight: 600;">C</a>
                                </div>"#
                            ));
                        }
                        if ipnetdb.ipv4_prefixes.len() > 8 {
                            details.push_str(&format!(r#"<span style="color: #64748b; font-size: 11px; padding: 6px; font-weight: 500;">+{} more</span>"#, ipnetdb.ipv4_prefixes.len() - 8));
                        }
                        details.push_str("</div></div>");
                    }
                };

                // Add Stanford ASDB categories
                let mut stanford = HashSet::new();
                for c in as_.stanford_asdb.iter() {
                    stanford.insert(c.layer1.as_str());
                }
                if !stanford.is_empty() {
                    details.push_str(r#"<div style="margin-top: 14px;"><div style="color: #94a3b8; font-size: 10px; text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 8px; font-weight: 600;">Categories</div><div style="display: flex; flex-wrap: wrap; gap: 6px;">"#);
                    for cat in stanford.iter() {
                        details.push_str(&format!(
                            r#"<span style="background: rgba(6, 182, 212, 0.15); color: #22d3ee; padding: 5px 10px; border-radius: 6px; font-size: 11px; font-weight: 500; border: 1px solid rgba(6, 182, 212, 0.2);">{cat}</span>"#
                        ));
                    }
                    details.push_str("</div></div>");
                }
                
                details.push_str("</div>");

                // Insert details inside the main container (before the closing </div>)
                let new_content = old_str.replacen("</div></div></div>", &format!("{}</div></div></div>", details), 1);
                marker.set_popup_content(&JsValue::from_str(&new_content));
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
            <div class="min-h-screen bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950 text-slate-100 flex flex-col lg:flex-row">
                // Map container
                <div class="flex-1 min-h-[50vh] lg:min-h-screen relative">
                    <div class="absolute inset-0">{ self.render_map() }</div>
                </div>

                // Sidebar
                <div class="w-full lg:w-80 xl:w-96 p-4 lg:p-6 space-y-4 lg:max-h-screen lg:overflow-y-auto 
                            bg-slate-900/80 backdrop-blur-xl border-t lg:border-t-0 lg:border-l border-slate-700/50">
                    
                    // Header
                    <div class="flex items-center gap-3 mb-2">
                        <div class="p-2 bg-blue-500/20 rounded-xl border border-blue-500/30">
                            <svg class="w-5 h-5 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 20l-5.447-2.724A1 1 0 013 16.382V5.618a1 1 0 011.447-.894L9 7m0 13l6-3m-6 3V7m6 10l4.553 2.276A1 1 0 0021 18.382V7.618a1 1 0 00-.553-.894L15 4m0 13V4m0 0L9 7"/>
                            </svg>
                        </div>
                        <div>
                            <h1 class="text-lg font-bold text-white">{"AS Map"}</h1>
                            <p class="text-xs text-slate-400">{"Autonomous Systems Explorer"}</p>
                        </div>
                    </div>

                    // Action buttons
                    <div class="p-4 rounded-2xl bg-slate-800/40 border border-slate-700/50 backdrop-blur-sm space-y-2">
                        { self.load_as_filtered_button(ctx) }
                        { self.load_as_bounded_button(ctx) }
                        { self.clear_button(ctx) }
                        <div class="grid grid-cols-2 gap-2">
                            { self.download_button(ctx) }
                            { self.download_detailed_button(ctx) }
                        </div>
                    </div>

                    // Filters
                    <div class="p-4 rounded-2xl bg-slate-800/40 border border-slate-700/50 backdrop-blur-sm">
                        <div class="flex items-center gap-2 mb-4">
                            <svg class="w-4 h-4 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z"/>
                            </svg>
                            <span class="text-sm font-semibold text-slate-300">{"Filters"}</span>
                        </div>
                        { self.filter_menu(ctx) }
                    </div>

                    // Stats
                    <div class="p-4 rounded-2xl bg-slate-800/40 border border-slate-700/50 backdrop-blur-sm">
                        <div class="flex items-center gap-2 mb-3">
                            <svg class="w-4 h-4 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"/>
                            </svg>
                            <span class="text-sm font-semibold text-slate-300">{"Statistics"}</span>
                        </div>
                        { self.debug_counter(ctx) }
                    </div>

                    { self.whois_panel() }
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
