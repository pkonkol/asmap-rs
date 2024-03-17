use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    sync::{Arc, Mutex, RwLock},
};

use gloo_console::log;
use gloo_file::{Blob, ObjectUrl};
use gloo_utils::document;
use js_sys::Object;
use leaflet::{Icon, LatLng, Map, Marker, TileLayer};
use protocol::{AsFilters, AsFiltersHasOrg, AsForFrontend};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{prelude::*, JsCast};
use wasm_timer::SystemTime;
use web_sys::{Element, HtmlCollection, HtmlElement, HtmlInputElement, Node};
use yew::prelude::*;

use super::api::{get_all_as_filtered, get_as_details};
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
    // as_cache: HashMap<u32, AsForFrontend>,
    /// Cached ASes which were manually opened and their detail downloaded
    detailed_ases: HashMap<u32, As>,
    /// these are actually just last drawn ases and serve as a proxy for the last filter use
    drawn_ases: HashMap<u32, AsForFrontend>,
    /// Filters that will be used when pressing load button
    next_filters: AsFilters,
    /// Filters executed during previous load
    prev_filters: AsFilters,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Point(pub f64, pub f64);

#[derive(Properties, PartialEq, Clone)]
pub struct Props {}

// interface components
impl MapComponent {
    fn load_as_bounded_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::LoadAsBounded);
        html! {
            <button onclick={cb}>{"Load all ASes in visible range"}</button>
        }
    }

    fn load_as_filtered_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::LoadAsFiltered);
        html! {
            <button onclick={cb}>{"Load ASes by filters ->"}</button>
        }
    }

    fn filter_menu(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div>
                <div >
                    <div style="display:inline-block;">{"min addr"}<br/>
                        <input title="test" type="number" id="minAddresses" value={self.next_filters.addresses.unwrap().0.to_string()} min="0" max="99999999" style="width: 5em;"
                            oninput={ctx.link().callback(|e: InputEvent| {
                                Msg::UpdateFilters(FilterForm::MinAddresses(
                                    e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                            })}
                        />
                        <br/>{"max addr"}<br/>
                        <input type="number" id="maxAddresses" value={self.next_filters.addresses.unwrap().1.to_string()} min="0" max="99999999" style="width: 5em;"
                            oninput={ctx.link().callback(|e: InputEvent| {
                                Msg::UpdateFilters(FilterForm::MaxAddresses(
                                    e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                            })}
                        />
                    </div>
                    <div style="display:inline-block;">{"country"}<br/>
                        <input type="text" id="countryCode" value={self.next_filters.country.clone()} size="2"
                            oninput={ctx.link().callback(|e: InputEvent| {
                                Msg::UpdateFilters(FilterForm::CountryCode(
                                    e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                            })}
                        />
                        <br/>
                        {"exclude"}<br/>
                        <input type="checkbox" id="excludeCountry" checked={self.next_filters.exclude_country}
                            oninput={ctx.link().callback(|_e: InputEvent| {
                                Msg::UpdateFilters(FilterForm::ExcludeCountry)
                            })}
                        />
                    </div>
                    <div style="display:inline-block;">{"min rank"}<br/>
                        <input type="number" id="minRank" value={self.next_filters.rank.unwrap().0.to_string()} min="0" max="999999" style="width: 4em;"
                            oninput={ctx.link().callback(|e: InputEvent| {
                                Msg::UpdateFilters(FilterForm::MinRank(
                                    e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                            })}
                        />
                        <br/>{"max rank"}<br/>
                            <input type="number" id="maxRank" value={self.next_filters.rank.unwrap().1.to_string()} min="0" max="999999" style="width: 4em;"
                                oninput={ctx.link().callback(|e: InputEvent| {
                                    Msg::UpdateFilters(FilterForm::MaxRank(
                                        e.target_unchecked_into::<HtmlInputElement>().value().parse().unwrap()))
                                })}
                            />
                    </div>
                    <div style="display:inline-block;">
                        {"hasOrg\u{00a0}"}<br/>
                        <select id="hasOrg" name="hasOrgSel"
                            onchange={ctx.link().callback(|e: Event| {
                                let selected = js_sys::Reflect::get(&e.target().unwrap(), &JsValue::from_str("value")).unwrap().as_string().unwrap();
                                Msg::UpdateFilters(FilterForm::HasOrg(selected))
                        })}>
                            <option value="yes">{"yes"}</option>
                            <option value="no">{"no"}</option>
                            <option value="both">{"both"}</option>
                        </select>
                        <br/>{"isBounded"}<br/>
                        <input type="checkbox" id="isBounded" checked={self.next_filters.bounds.is_some()}
                            oninput={ctx.link().callback(|_e: InputEvent| {
                                Msg::UpdateFilters(FilterForm::IsBounded)
                            })}
                        />
                    </div>
                    <div style="display:inline-block;">
                        {"category\u{00a0}"}<br/>
                        <select id="category" name="category" multiple=true style="width: 20em;"
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
                                html!{<option value={ *category }>{ category }</option>}
                            }).collect::<Html>() }
                        </select>
                    </div>
                </div>
            </div>
        }
    }

    fn download_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::DownloadFiltered);
        html! {
            <button onclick={cb}>{"Download currently loaded"}</button>
        }
    }

    fn download_detailed_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::DownloadDetailed);
        html! {
            <button onclick={cb}>{"Download previously opened"}</button>
        }
    }

    fn clear_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::ClearMarkers);
        html! {
            <button onclick={cb}>{"Clear map"}</button>
        }
    }

    fn debug_counter(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <>
            <b>{"drawn:"}</b>{self.drawn_ases.len()}<br/>
            <b>{"detailed:"}</b>{ self.detailed_ases.len() }<br/>
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

    fn get_detailed_csv_writer<'a>(
        &self,
        ases: impl Iterator<Item = &'a As>,
    ) -> (csv::Writer<Vec<u8>>, u64) {
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
        tmp_download_link
            .set_attribute("href", &object_url)
            .unwrap();
        tmp_download_link
            .set_attribute("download", &filename)
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
            // as_cache: HashMap::new(),
            detailed_ases: HashMap::new(),
            drawn_ases: HashMap::new(),
            next_filters: initial_filters.clone(),
            prev_filters: initial_filters,
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
            Msg::LoadAsBounded => {
                let bounds: models::LatLngBounds =
                    serde_wasm_bindgen::from_value(self.map.getBounds().into()).unwrap();
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
                // Bounds must be dynamically updated at the time of button press if checkbox is on
                if self.next_filters.bounds.is_some() {
                    let bounds: models::LatLngBounds =
                        serde_wasm_bindgen::from_value(self.map.getBounds().into()).unwrap();
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
                        // The value just needs to be some, it will be udated on load filtered button press
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
                // use rand::prelude::*;
                //use rand::
                log!(format!(
                    "{} ASes received to be drawn, drawing them signal at map_component.rs",
                    ases.len()
                ));
                // let pool = rayon::ThreadPoolBuilder::new().num_threads(22).build().unwrap();
                // let thread_pool = rayon_wasm::ThreadPoolBuilder::new()
                // .num_threads(24)
                // .spawn_handler(|thread| {
                //     pool.run(|| thread.run()).unwrap();
                //     Ok(())
                // })
                // .build()
                // .unwrap();

                // use gloo_worker::oneshot::oneshot;
                // use gloo_worker::Spawnable;
                // log!("b4 oneshot");
                // #[oneshot]
                // async fn Squared(input: u32) -> u32 {
                //     input.pow(2)
                // }
                // this shit looks for  http:[...]/huj123 file to execute
                // let mut sb = Squared::spawner().spawn("huj123");
                // wasm_bindgen_futures::spawn_local(async move {
                //     let xd = sb.run(2).await;
                //     log!(xd);
                //     assert_eq!(xd, 4);
                // });
                // log!("after spwan");

                let test = (1..1).collect::<Vec<_>>();
                test.into_iter().for_each(|x| {
                    let huj = x.clone();
                    let ms = rand::random::<u32>() % 1000;
                    // gloo_timers::future::TimeoutFuture::new(ms).await;
                    let mut huj2 = 0;
                    for i in 0..(ms * 10000) {
                        huj2 += 1;
                    }
                    log!(format!("parallel x: {huj}, slept {ms}"));
                    // wasm_bindgen_futures::spawn_local(async move {
                    //     let ms = rand::random::<u32>() % 1000;
                    //     gloo_timers::future::TimeoutFuture::new(ms).await;
                    //     log!(format!("parallel x: {huj}, slept {ms}"));
                    // });
                });
                let start = js_sys::Date::now();
                log!("generating markers");
                // for as_ in ases.into_iter() {
                let markers = Arc::new(Mutex::new(Vec::new()));
                let ctx2 = Arc::new(Mutex::new(ctx));
                let mut drawn_ases2 = Arc::new(Mutex::new(self.drawn_ases.clone()));
                ases.iter().for_each(|as_| {
                    let drawn_ases = Arc::clone(&mut drawn_ases2);
                    let asn = as_.asn;
                    // self.as_cache.insert(asn, a);
                    if drawn_ases.lock().unwrap().contains_key(&asn) {
                        return ();
                    }
                    {
                        let mut xd = drawn_ases.lock().unwrap();
                        xd.insert(asn, as_.clone());
                    }

                    // TODO parallelize this block of code
                    let marker_size = scale_as_marker(&as_);
                    let country = celes::Country::from_alpha2(&as_.country_code);
                    // let m = create_marker("huj", "dupa", &Point(0.0, 0.0), (1, 1));
                    let m = create_marker(
                        &format!(
                            "<b>asn</b>:{} <b>rank</b>:{} <b>prefixes</b>:{} <b>addresses</b>:{}
                            <b>links</b>:<a href=\"https://bgp.he.net/AS{asn}\" target=\"_blank\">bgp.he</a>, <a href=\"https://bgpview.io/asn/{asn}\" target=\"_blank\">bgpview</a>, shodan<br>
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
                        &Point(
                            as_.coordinates.lat,
                            as_.coordinates.lon,
                        ),
                        marker_size,
                    );
                    // TODO can i parallelize this shite too?
                    let details_cb = ctx2.lock().unwrap()
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
                        details_cb.emit(id);
                    })
                        as Box<dyn Fn(JsValue)>);
                    let js = detail_update_closure.into_js_value();
                    m.on("popupopen", &js);


                    markers.lock().unwrap().push(m);
                });
                log!((js_sys::Date::now() - start));
                log!("adding marker Layers to map");
                // TODO move normally instead of lcone
                let mk = markers.lock().unwrap().clone();
                self.marker_cluster.addLayers(mk);
                log!("done");
            }
            Msg::UpdateDetails(as_, marker_id) => {
                log!(format!(
                    "got details for {}, proceeding to update popup in marker {marker_id}",
                    as_.asn
                ));
                let marker = self.marker_cluster.getLayer(marker_id);
                let mut old_str = marker.getPopup().getContent().as_string().unwrap();

                let mut details = String::new();
                details.push_str(&format!(
                    "<b>degree</b>: {}",
                    as_.asrank_data.as_ref().unwrap().degree
                ));

                let prefixes = if let Some(ipnetdb) = as_.ipnetdb_data.as_ref() {
                    old_str = old_str.replace("shodan", &format!("<a href=\"https://www.shodan.io/search?query=net:{}\" target=\"_blank\">shodan</a>", ipnetdb
                        .ipv4_prefixes
                        .iter()
                        .map(|x| x.range.to_string())
                        .map(|mut x| {
                            x.push(',');
                            x
                        })
                        .collect::<String>()));
                    format!(
                        "<br><b>prefixes</b>: {}",
                        ipnetdb
                            .ipv4_prefixes
                            .iter()
                            .fold(String::new(),  |mut output, x| {
                                let cidr = x.range.to_string();
                                let _ = write!(output, "{cidr}:<b><a href=\"https://www.shodan.io/search?query=net%3A{cidr}\" target=\"_blank\">s</a>\
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

                let mut stanford = HashSet::new();
                for c in as_.stanford_asdb.iter() {
                    //log!(format!("found category {:?}", c));
                    stanford.insert(c.layer1.as_str());
                }
                details.push_str(&format!(
                    "<br><b>categories</b>: {}",
                    stanford.iter().fold(String::new(), |mut output, x| {
                        let _ = writeln!(output, "<b>></b>{x}<b>.</b><br>");
                        output
                    })
                ));

                marker.setPopupContent(&JsValue::from_str(&format!("{old_str}<br>{details}")));
                self.detailed_ases.insert(as_.asn, as_);
                log!("marker updated");
            }
            Msg::ClearMarkers => {
                self.drawn_ases.clear();
                self.marker_cluster.clearLayers();
            }
            Msg::DownloadFiltered => {
                let ases = self.drawn_ases.iter().map(|(_, as_t)| as_t);
                // .filter(|(asn, _)| self.drawn_ases.contains(asn))
                self.create_downloadable_csv(DownloadableCsvInput::Simple(Box::new(ases)));
            }
            Msg::DownloadDetailed => {
                let ases = self.detailed_ases.values();
                self.create_downloadable_csv(DownloadableCsvInput::Detailed(Box::new(ases)));
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
                    {Self::load_as_bounded_button(self, ctx)}
                    {Self::load_as_filtered_button(self, ctx)}
                    {Self::download_button(self, ctx)}
                    {Self::download_detailed_button(self, ctx)}
                    {Self::clear_button(self, ctx)}
                </div>
                <div style="display: normal; padding-right: 20px;" >
                    {Self::filter_menu(self, ctx)}
                </div>
                // <div style="display: flex; flex-flow: column wrap;">
                // </div>
                <div>
                    {Self::debug_counter(self, ctx)}
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PopupOpts {
    pub max_width: i64,
}

fn create_marker(description: &str, tooltip: &str, coord: &Point, size: (u64, u64)) -> Marker {
    let opts = JsValue::from_str(r#"{"opacity": "0.5"}"#);
    let latlng = LatLng::new(coord.0, coord.1);
    let m = Marker::new_with_options(&latlng, &opts);

    m.bindPopup(
        &JsValue::from_str(description),
        &serde_wasm_bindgen::to_value(&PopupOpts { max_width: 600 }).unwrap(),
    );
    m.bindTooltip(&JsValue::from_str(tooltip), &JsValue::NULL);

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
