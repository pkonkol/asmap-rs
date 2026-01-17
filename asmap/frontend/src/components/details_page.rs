use gloo_console::log;
use yew::prelude::*;
use yew_router::prelude::*;

use asdb_models::{As, WhoIsAsn};
use crate::routes::Route;
use super::api::{get_as_details, fetch_as_whois_data};
use super::geocoding::{geocode_addresses, GeocodedAddress};

pub enum Msg {
    LoadData,
    SetAsDetails(As),
    SetWhois(Option<WhoIsAsn>),
    Error(String),
    GeocodeAddresses,
    GeocodeResult(Vec<GeocodedAddress>),
}

#[derive(Properties, PartialEq)]
pub struct DetailsPageProps {
    pub id: String,
}

pub struct DetailsPage {
    asn: Option<u32>,
    as_details: Option<As>,
    whois_data: Option<WhoIsAsn>,
    loading: bool,
    whois_loading: bool,
    geocoding: bool,
    error: Option<String>,
}

impl Component for DetailsPage {
    type Message = Msg;
    type Properties = DetailsPageProps;

    fn create(ctx: &Context<Self>) -> Self {
        let asn = ctx.props().id.parse::<u32>().ok();

        if asn.is_some() {
            ctx.link().send_message(Msg::LoadData);
        }

        Self {
            asn,
            as_details: None,
            whois_data: None,
            loading: true,
            whois_loading: true,
            geocoding: false,
            error: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadData => {
                if let Some(asn) = self.asn {
                    ctx.link().send_future(async move {
                        match get_as_details(asn).await {
                            Ok(details) => Msg::SetAsDetails(details),
                            Err(e) => Msg::Error(format!("Failed to load AS details: {}", e)),
                        }
                    });

                    ctx.link().send_future(async move {
                        match fetch_as_whois_data(asn).await {
                            Ok(whois) => Msg::SetWhois(whois),
                            Err(e) => {
                                log!(format!("WHOIS error: {}", e));
                                Msg::SetWhois(None)
                            }
                        }
                    });
                }
            }
            Msg::SetAsDetails(details) => {
                self.as_details = Some(details);
                self.loading = false;
            }
            Msg::SetWhois(whois) => {
                self.whois_data = whois;
                self.whois_loading = false;
            }
            Msg::Error(e) => {
                self.error = Some(e);
                self.loading = false;
            }
            Msg::GeocodeAddresses => {
                self.geocoding = true;
                // Collect all addresses from WHOIS data
                let addresses = self.collect_whois_addresses();
                if addresses.is_empty() {
                    log!("No addresses found in WHOIS data to geocode");
                    self.geocoding = false;
                } else {
                    log!(format!("Geocoding {} addresses...", addresses.len()));
                    ctx.link().send_future(async move {
                        let results = geocode_addresses(addresses).await;
                        Msg::GeocodeResult(results)
                    });
                }
            }
            Msg::GeocodeResult(results) => {
                self.geocoding = false;
                log!("=== GEOCODING RESULTS ===");
                for result in &results {
                    log!(format!("Original: {}", result.original_address));
                    log!(format!("Normalized: {}", result.normalized_address));
                    if let Some(coord) = &result.coordinate {
                        log!(format!("📍 Coordinates: {:.6}, {:.6}", coord.latitude, coord.longitude));
                    }
                    if let Some(display) = &result.display_name {
                        log!(format!("Display name: {}", display));
                    }
                    if let Some(err) = &result.error {
                        log!(format!("❌ Error: {}", err));
                    }
                    log!("---");
                }
                log!(format!("=== Total: {} addresses geocoded ===", results.len()));
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="min-h-screen bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950 text-slate-100">
                // Top navigation bar
                <nav class="sticky top-0 z-50 backdrop-blur-xl bg-slate-950/60 border-b border-slate-800/70">
                    <div class="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
                        <div class="flex items-center justify-between gap-4">
                            <Link<Route>
                                to={Route::Map}
                                classes="group inline-flex items-center gap-3 text-slate-300 hover:text-white transition-colors duration-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/70 focus-visible:ring-offset-2 focus-visible:ring-offset-slate-950 rounded-xl"
                            >
                                <div class="p-2 rounded-xl bg-slate-800/50 border border-slate-700/50 group-hover:bg-blue-600/15 group-hover:border-blue-500/30 transition-colors">
                                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18"/>
                                    </svg>
                                </div>
                                <span class="font-medium tracking-tight">{"Back to Map"}</span>
                            </Link<Route>>

                            if self.as_details.is_some() {
                                <div class="flex items-center gap-2">
                                    <span class="px-3 py-1 text-xs font-semibold bg-emerald-500/15 text-emerald-300 rounded-full border border-emerald-500/25">
                                        {"ACTIVE"}
                                    </span>
                                </div>
                            }
                        </div>
                    </div>
                </nav>

                <main class="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
                    <div class="space-y-8">
                        { self.render_content(ctx) }
                    </div>
                </main>
            </div>
        }
    }
}

impl DetailsPage {
    fn render_content(&self, ctx: &Context<Self>) -> Html {
        if let Some(error) = &self.error {
            return html! {
                <div class="flex items-center justify-center min-h-[60vh]">
                    <div class="max-w-md w-full p-7 sm:p-8 rounded-3xl bg-red-950/40 border border-red-800/40 backdrop-blur-sm shadow-[0_18px_60px_-35px_rgba(0,0,0,0.9)]">
                        <div class="flex items-start gap-4">
                            <div class="p-3 bg-red-500/15 rounded-2xl border border-red-500/20">
                                <svg class="w-6 h-6 text-red-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                                </svg>
                            </div>
                            <div class="min-w-0">
                                <h3 class="text-lg font-semibold text-red-200 tracking-tight">{"Error Loading Data"}</h3>
                                <p class="text-red-300/80 text-sm mt-1 break-words">{ error }</p>
                            </div>
                        </div>
                    </div>
                </div>
            };
        }

        if self.loading {
            return html! {
                <div class="flex items-center justify-center min-h-[60vh]">
                    <div class="text-center">
                        <div class="relative w-16 h-16 mx-auto mb-6">
                            <div class="absolute inset-0 rounded-full border-4 border-slate-700/70"></div>
                            <div class="absolute inset-0 rounded-full border-4 border-blue-500 border-t-transparent animate-spin"></div>
                        </div>
                        <p class="text-slate-300 font-medium tracking-tight">{"Loading AS details..."}</p>
                        <p class="text-slate-500 text-sm mt-1">{"Fetching core dataset + WHOIS in background"}</p>
                    </div>
                </div>
            };
        }

        if let Some(as_) = &self.as_details {
            let asrank = as_.asrank_data.as_ref();
            let ipnetdb = as_.ipnetdb_data.as_ref();
            let country_code = asrank.map(|a| a.country_iso.as_str()).unwrap_or("??");
            let country = celes::Country::from_alpha2(country_code);

            html! {
                <div class="space-y-8">
                    // Hero Header
                    { self.render_hero(as_, asrank, country_code, country) }

                    // Stats Cards
                    if let Some(asrank) = asrank {
                        { self.render_stats(asrank) }
                    }

                    // Main Content Grid
                    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
                        // Left Column - Main Info
                        <div class="lg:col-span-2 space-y-6">
                            // Prefixes Section
                            if let Some(ipnetdb) = ipnetdb {
                                { self.render_prefixes(ipnetdb) }
                            }

                            // Categories
                            if !as_.stanford_asdb.is_empty() {
                                { self.render_categories(&as_.stanford_asdb) }
                            }

                            // WHOIS Section
                            { self.render_whois(ctx) }
                        </div>

                        // Right Column - Quick Links & Info
                        <div class="space-y-6">
                            { self.render_external_links(as_.asn) }
                        </div>
                    </div>
                </div>
            }
        } else {
            html! {
                <div class="flex items-center justify-center min-h-[60vh]">
                    <div class="text-center p-7 sm:p-8 rounded-3xl bg-slate-900/40 border border-slate-800/60 backdrop-blur-sm shadow-[0_18px_60px_-35px_rgba(0,0,0,0.9)] max-w-md">
                        <div class="p-4 bg-slate-800/40 border border-slate-700/40 rounded-2xl w-fit mx-auto mb-4">
                            <svg class="w-8 h-8 text-slate-300/70" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.172 16.172a4 4 0 015.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                            </svg>
                        </div>
                        <h3 class="text-xl font-semibold text-slate-100 mb-2 tracking-tight">{"AS Not Found"}</h3>
                        <p class="text-slate-400 text-sm leading-relaxed">{"The requested autonomous system could not be found."}</p>
                    </div>
                </div>
            }
        }
    }

    fn render_hero(
        &self,
        as_: &As,
        asrank: Option<&asdb_models::AsrankAsn>,
        country_code: &str,
        country: Result<celes::Country, &str>
    ) -> Html {
        html! {
            <div class="relative overflow-hidden rounded-3xl bg-gradient-to-r from-blue-600/20 via-purple-600/15 to-cyan-600/20 border border-slate-700/60 shadow-[0_30px_90px_-60px_rgba(0,0,0,0.95)]">
                // Background decoration
                <div class="absolute inset-0 overflow-hidden pointer-events-none">
                    <div class="absolute -top-28 -right-28 w-[34rem] h-[34rem] bg-blue-500/10 rounded-full blur-3xl"></div>
                    <div class="absolute -bottom-28 -left-28 w-[34rem] h-[34rem] bg-purple-500/10 rounded-full blur-3xl"></div>
                    <div class="absolute inset-0 bg-[radial-gradient(ellipse_at_top,rgba(59,130,246,0.08),transparent_55%)]"></div>
                </div>

                <div class="relative p-6 sm:p-8 md:p-10">
                    <div class="flex flex-col md:flex-row md:items-start md:justify-between gap-6">
                        <div class="flex-1 min-w-0">
                            // ASN Badge
                            <div class="inline-flex items-center gap-2 px-4 py-2 bg-slate-950/35 backdrop-blur rounded-2xl border border-slate-600/45 shadow-sm mb-4">
                                <div class="w-2 h-2 bg-emerald-400 rounded-full animate-pulse"></div>
                                <span class="text-sm font-mono font-bold text-white">{ format!("AS{}", as_.asn) }</span>
                            </div>

                            // Name and Organization
                            if let Some(asrank) = asrank {
                                <h1 class="text-3xl sm:text-4xl md:text-5xl font-bold text-white mb-3 leading-[1.1] tracking-tight break-words">
                                    { &asrank.name }
                                </h1>
                                if let Some(org) = &asrank.organization {
                                    <p class="text-base md:text-lg text-slate-300 flex items-start gap-2 leading-snug">
                                        <svg class="w-5 h-5 text-slate-400 mt-0.5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4"/>
                                        </svg>
                                        <span class="break-words">{ org }</span>
                                    </p>
                                }
                            } else {
                                <h1 class="text-3xl sm:text-4xl md:text-5xl font-bold text-white leading-[1.1] tracking-tight">
                                    { format!("AS{}", as_.asn) }
                                </h1>
                                <p class="text-slate-400 text-sm mt-2">{"No ASRank metadata available."}</p>
                            }
                        </div>

                        // Country Badge
                        <div class="flex-shrink-0">
                            <div class="flex items-center gap-3 px-5 py-3 bg-slate-950/30 backdrop-blur rounded-2xl border border-slate-600/35 shadow-sm">
                                <span class="text-4xl">{ Self::country_flag(country_code) }</span>
                                <div>
                                    <p class="text-[11px] text-slate-400 uppercase tracking-wider">{"Country"}</p>
                                    <p class="text-lg font-semibold text-white tracking-tight">
                                        { country.map(|c| c.long_name).unwrap_or_else(|_| country_code) }
                                    </p>
                                </div>
                            </div>
                        </div>
                    </div>

                    // subtle bottom divider
                    <div class="mt-8 h-px bg-gradient-to-r from-transparent via-slate-700/60 to-transparent"></div>
                </div>
            </div>
        }
    }

    fn render_stats(&self, asrank: &asdb_models::AsrankAsn) -> Html {
        let stats = vec![
            ("Global Rank", format!("#{}", asrank.rank), "chart-bar", "from-blue-500 to-cyan-500"),
            ("Prefixes", asrank.prefixes.to_string(), "globe-alt", "from-emerald-500 to-teal-500"),
            ("IP Addresses", Self::format_number(asrank.addresses.into()), "server", "from-purple-500 to-pink-500"),
            ("Connections", asrank.degree.total.to_string(), "share", "from-orange-500 to-amber-500"),
        ];

        html! {
            <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                { stats.into_iter().map(|(label, value, icon, gradient)| {
                    html! {
                        <div class="group relative p-5 rounded-2xl bg-slate-900/40 border border-slate-800/60 backdrop-blur-sm
                                    shadow-[0_10px_40px_-25px_rgba(0,0,0,0.85)]
                                    transition-all duration-300 hover:border-slate-700/70 hover:shadow-[0_18px_60px_-35px_rgba(0,0,0,0.9)] hover:-translate-y-[1px]">
                            <div class={format!("absolute inset-0 bg-gradient-to-br {} opacity-0 group-hover:opacity-[0.06] rounded-2xl transition-opacity", gradient)}></div>
                            <div class="relative">
                                <div class={format!("w-10 h-10 mb-3 rounded-xl bg-gradient-to-br {} flex items-center justify-center shadow-sm", gradient)}>
                                    { Self::render_icon(icon) }
                                </div>
                                <p class="text-2xl font-bold text-white mb-1 tracking-tight tabular-nums">{ value }</p>
                                <p class="text-sm text-slate-400">{ label }</p>
                            </div>
                        </div>
                    }
                }).collect::<Html>() }
            </div>
        }
    }

    fn render_prefixes(&self, ipnetdb: &asdb_models::IPNetDBAsn) -> Html {
        if ipnetdb.ipv4_prefixes.is_empty() {
            return html! {};
        }

        html! {
            <div class="p-6 rounded-2xl bg-slate-900/40 border border-slate-800/60 backdrop-blur-sm
                        shadow-[0_10px_40px_-25px_rgba(0,0,0,0.85)]
                        transition-all duration-300 hover:border-slate-700/70 hover:shadow-[0_18px_60px_-35px_rgba(0,0,0,0.9)]">
                <div class="flex items-start gap-3 mb-5">
                    <div class="p-2 bg-emerald-500/15 rounded-xl border border-emerald-500/20">
                        <svg class="w-5 h-5 text-emerald-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9"/>
                        </svg>
                    </div>
                    <div class="min-w-0">
                        <h3 class="text-lg font-semibold text-white tracking-tight">{"IPv4 Prefixes"}</h3>
                        <p class="text-sm text-slate-400">{ format!("{} announced prefixes", ipnetdb.ipv4_prefixes.len()) }</p>
                    </div>
                </div>

                <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 max-h-80 overflow-y-auto pr-2
                            [scrollbar-width:thin] [scrollbar-color:rgba(148,163,184,0.55)_rgba(15,23,42,0.35)]
                            [&::-webkit-scrollbar]:w-2.5
                            [&::-webkit-scrollbar-track]:bg-slate-950/30 [&::-webkit-scrollbar-track]:rounded-full
                            [&::-webkit-scrollbar-thumb]:bg-slate-400/40 [&::-webkit-scrollbar-thumb]:rounded-full
                            [&::-webkit-scrollbar-thumb:hover]:bg-slate-300/50">
                    { ipnetdb.ipv4_prefixes.iter().map(|prefix| {
                        let cidr = prefix.range.to_string();
                        html! {
                            <div class="group flex items-center justify-between gap-3 p-3 rounded-xl
                                        bg-slate-800/35 border border-slate-700/40
                                        hover:bg-slate-800/55 hover:border-slate-600/60 transition-all duration-200">
                                <code class="font-mono text-sm text-emerald-200 font-semibold tracking-tight break-all">{ &cidr }</code>

                                <div class="flex items-center gap-1 opacity-100 sm:opacity-0 sm:group-hover:opacity-100 transition-opacity">
                                    <a
                                        href={format!("https://www.shodan.io/search?query=net%3A{}", cidr)}
                                        target="_blank"
                                        class="p-1.5 rounded-lg hover:bg-orange-500/15 border border-transparent hover:border-orange-500/25 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-orange-400/60"
                                        title="Search on Shodan"
                                    >
                                        <span class="text-xs font-bold text-orange-300">{"S"}</span>
                                    </a>
                                    <a
                                        href={format!("https://www.zoomeye.org/searchResult?q=cidr%3A{}", cidr)}
                                        target="_blank"
                                        class="p-1.5 rounded-lg hover:bg-blue-500/15 border border-transparent hover:border-blue-500/25 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-400/60"
                                        title="Search on ZoomEye"
                                    >
                                        <span class="text-xs font-bold text-blue-300">{"Z"}</span>
                                    </a>
                                    <a
                                        href={format!("https://search.censys.io/search?resource=hosts&q=ip%3A{}", cidr)}
                                        target="_blank"
                                        class="p-1.5 rounded-lg hover:bg-purple-500/15 border border-transparent hover:border-purple-500/25 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400/60"
                                        title="Search on Censys"
                                    >
                                        <span class="text-xs font-bold text-purple-300">{"C"}</span>
                                    </a>
                                </div>
                            </div>
                        }
                    }).collect::<Html>() }
                </div>
            </div>
        }
    }

    fn render_categories(&self, categories: &[asdb_models::StanfordASdbCategory]) -> Html {
        html! {
            <div class="p-6 rounded-2xl bg-slate-900/40 border border-slate-800/60 backdrop-blur-sm
                        shadow-[0_10px_40px_-25px_rgba(0,0,0,0.85)]
                        transition-all duration-300 hover:border-slate-700/70 hover:shadow-[0_18px_60px_-35px_rgba(0,0,0,0.9)]">
                <div class="flex items-start gap-3 mb-5">
                    <div class="p-2 bg-violet-500/15 rounded-xl border border-violet-500/20">
                        <svg class="w-5 h-5 text-violet-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z"/>
                        </svg>
                    </div>
                    <div class="min-w-0">
                        <h3 class="text-lg font-semibold text-white tracking-tight">{"Classifications"}</h3>
                        <p class="text-sm text-slate-400">{"Stanford ASDB categories"}</p>
                    </div>
                </div>

                <div class="flex flex-wrap gap-2">
                    { categories.iter().map(|cat| {
                        let layer2 = if cat.layer2.is_empty() { "" } else { &cat.layer2 };
                        let color = Self::category_color(&cat.layer1);
                        html! {
                            <div class={format!("px-4 py-2 rounded-xl border transition-all duration-200 hover:scale-[1.03] hover:-translate-y-[1px] shadow-sm {}",
                                color)}>
                                <span class="font-semibold tracking-tight">{ &cat.layer1 }</span>
                                if !layer2.is_empty() {
                                    <span class="text-slate-400 mx-1">{"›"}</span>
                                    <span class="opacity-85">{ layer2 }</span>
                                }
                            </div>
                        }
                    }).collect::<Html>() }
                </div>
            </div>
        }
    }

    fn render_external_links(&self, asn: u32) -> Html {
        let links = vec![
            ("BGP Hurricane Electric", format!("https://bgp.he.net/AS{}", asn), "from-blue-600 to-blue-700"),
            ("RIPE Stat", format!("https://stat.ripe.net/AS{}", asn), "from-amber-600 to-orange-700"),
        ];

        html! {
            <div class="p-6 rounded-2xl bg-slate-900/40 border border-slate-800/60 backdrop-blur-sm
                        shadow-[0_10px_40px_-25px_rgba(0,0,0,0.85)]
                        transition-all duration-300 hover:border-slate-700/70 hover:shadow-[0_18px_60px_-35px_rgba(0,0,0,0.9)]">
                <div class="flex items-center gap-3 mb-5">
                    <div class="p-2 bg-blue-500/15 rounded-xl border border-blue-500/20">
                        <svg class="w-5 h-5 text-blue-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/>
                        </svg>
                    </div>
                    <div class="min-w-0">
                        <h3 class="text-lg font-semibold text-white tracking-tight">{"External Resources"}</h3>
                        <p class="text-sm text-slate-400">{"Useful links for this ASN"}</p>
                    </div>
                </div>

                <div class="space-y-2">
                    { links.into_iter().map(|(name, url, gradient)| {
                        html! {
                            <a
                                href={url}
                                target="_blank"
                                class={format!(
                                    "flex items-center justify-between gap-3 p-3 rounded-xl text-white font-semibold tracking-tight
                                     bg-gradient-to-r {} shadow-sm
                                     hover:opacity-95 hover:translate-x-0.5 transition-all duration-200
                                     focus:outline-none focus-visible:ring-2 focus-visible:ring-white/30",
                                    gradient
                                )}
                            >
                                <span class="truncate">{ name }</span>
                                <svg class="w-4 h-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 5l7 7m0 0l-7 7m7-7H3"/>
                                </svg>
                            </a>
                        }
                    }).collect::<Html>() }
                </div>
            </div>
        }
    }

    fn render_whois(&self, ctx: &Context<Self>) -> Html {
        let on_geocode_click = ctx.link().callback(|_| Msg::GeocodeAddresses);
        let has_addresses = self.whois_data.as_ref().map_or(false, |w| {
            !w.organisation.as_ref().map_or(true, |o| o.address.is_empty())
            || w.contacts.iter().any(|c| !c.address.is_empty())
        });

        html! {
            <div class="p-6 rounded-2xl bg-slate-900/40 border border-slate-800/60 backdrop-blur-sm
                        shadow-[0_10px_40px_-25px_rgba(0,0,0,0.85)]
                        transition-all duration-300 hover:border-slate-700/70 hover:shadow-[0_18px_60px_-35px_rgba(0,0,0,0.9)]">
                <div class="flex items-start justify-between gap-3 mb-5">
                    <div class="flex items-start gap-3">
                        <div class="p-2 bg-amber-500/15 rounded-xl border border-amber-500/20">
                            <svg class="w-5 h-5 text-amber-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                            </svg>
                        </div>
                        <div class="min-w-0">
                            <h3 class="text-lg font-semibold text-white tracking-tight">{"WHOIS Information"}</h3>
                            <p class="text-sm text-slate-400">{"Registry data from RIPE"}</p>
                        </div>
                    </div>
                    // Geocode button
                    if has_addresses && !self.whois_loading {
                        <button
                            onclick={on_geocode_click}
                            disabled={self.geocoding}
                            class={classes!(
                                "inline-flex", "items-center", "gap-2", "px-4", "py-2",
                                "rounded-xl", "text-sm", "font-medium", "transition-all", "duration-200",
                                if self.geocoding {
                                    "bg-slate-700/50 text-slate-400 cursor-not-allowed"
                                } else {
                                    "bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 hover:bg-emerald-500/30 hover:border-emerald-400/40"
                                }
                            )}
                        >
                            if self.geocoding {
                                <div class="w-4 h-4 border-2 border-slate-500/70 border-t-emerald-400 rounded-full animate-spin"></div>
                                {"Geocoding..."}
                            } else {
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z"/>
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 11a3 3 0 11-6 0 3 3 0 016 0z"/>
                                </svg>
                                {"Geocode"}
                            }
                        </button>
                    }
                </div>

                {
                    if self.whois_loading {
                        html! {
                            <div class="flex items-center gap-3 p-4 rounded-xl bg-slate-800/35 border border-slate-700/40">
                                <div class="w-5 h-5 border-2 border-slate-500/70 border-t-blue-400 rounded-full animate-spin"></div>
                                <span class="text-slate-300">{"Fetching WHOIS data..."}</span>
                            </div>
                        }
                    } else if let Some(whois) = &self.whois_data {
                        html! {
                            <div class="space-y-4">
                                // AS Info Card
                                <div class="p-4 rounded-2xl bg-slate-800/35 border border-slate-700/40 space-y-3">
                                    <h4 class="text-xs font-semibold text-slate-300 uppercase tracking-wider">{"Registration Info"}</h4>
                                    { Self::render_whois_row("AS Name", whois.as_name.as_deref()) }
                                    { Self::render_whois_row("Description", Some(&whois.descr.join(" • "))) }
                                    { Self::render_whois_row("Country", whois.country.as_deref()) }
                                </div>

                                // Organisation Card
                                if let Some(org) = &whois.organisation {
                                    <div class="p-4 rounded-2xl bg-slate-800/35 border border-slate-700/40 space-y-3">
                                        <h4 class="text-xs font-semibold text-slate-300 uppercase tracking-wider">{"Organisation"}</h4>
                                        { Self::render_whois_row("Name", Some(&org.org_name)) }
                                        { Self::render_whois_row("Address", Some(&org.address.join(", "))) }
                                        { Self::render_whois_row("Email", org.email.as_deref()) }
                                    </div>
                                }

                                // Contacts
                                if !whois.contacts.is_empty() {
                                    <div class="p-4 rounded-2xl bg-slate-800/35 border border-slate-700/40">
                                        <h4 class="text-xs font-semibold text-slate-300 uppercase tracking-wider mb-3">{"Technical Contacts"}</h4>
                                        <div class="space-y-2">
                                            { whois.contacts.iter().map(|contact| {
                                                html! {
                                                    <div class="flex items-center justify-between gap-3 p-3 rounded-xl bg-slate-900/35 border border-slate-700/35">
                                                        <div class="flex items-center gap-3 min-w-0">
                                                            <div class="w-9 h-9 bg-slate-800/45 border border-slate-700/35 rounded-full flex items-center justify-center flex-shrink-0">
                                                                <svg class="w-4 h-4 text-slate-300/70" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"/>
                                                                </svg>
                                                            </div>
                                                            <div class="min-w-0">
                                                                <p class="text-sm font-semibold text-slate-100 tracking-tight truncate">{ &contact.name }</p>
                                                                <p class="text-xs text-slate-400 truncate">{ &contact.nic_hdl }</p>
                                                            </div>
                                                        </div>
                                                        if let Some(email) = &contact.email {
                                                            <a
                                                                href={format!("mailto:{}", email)}
                                                                class="text-sm text-blue-300 hover:text-blue-200 font-semibold transition-colors flex-shrink-0
                                                                       focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-400/60 rounded-lg px-2 py-1"
                                                            >
                                                                { email }
                                                            </a>
                                                        }
                                                    </div>
                                                }
                                            }).collect::<Html>() }
                                        </div>
                                    </div>
                                }
                            </div>
                        }
                    } else {
                        html! {
                            <div class="p-4 rounded-xl bg-slate-800/30 border border-slate-700/35 text-center">
                                <p class="text-slate-400">{"No WHOIS data available"}</p>
                            </div>
                        }
                    }
                }
            </div>
        }
    }

    // Helper functions
    fn render_whois_row(label: &str, value: Option<&str>) -> Html {
        if let Some(val) = value {
            if !val.is_empty() {
                return html! {
                    <div class="flex flex-col sm:flex-row sm:items-start gap-1 sm:gap-3">
                        <span class="text-xs sm:text-sm text-slate-400 sm:w-28 flex-shrink-0 uppercase tracking-wide sm:normal-case sm:tracking-normal">
                            { label }
                        </span>
                        <span class="text-sm text-slate-100/90 leading-snug break-words">{ val }</span>
                    </div>
                };
            }
        }
        html! {}
    }

    fn render_icon(name: &str) -> Html {
        match name {
            "chart-bar" => html! {
                <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"/>
                </svg>
            },
            "globe-alt" => html! {
                <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9"/>
                </svg>
            },
            "server" => html! {
                <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01"/>
                </svg>
            },
            "share" => html! {
                <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.368 2.684 3 3 0 00-5.368-2.684z"/>
                </svg>
            },
            _ => html! {}
        }
    }

    fn format_number(n: u64) -> String {
        if n >= 1_000_000_000 {
            format!("{:.1}B", n as f64 / 1_000_000_000.0)
        } else if n >= 1_000_000 {
            format!("{:.1}M", n as f64 / 1_000_000.0)
        } else if n >= 1_000 {
            format!("{:.1}K", n as f64 / 1_000.0)
        } else {
            n.to_string()
        }
    }

    fn country_flag(code: &str) -> String {
        if code.len() != 2 {
            return "🌐".to_string();
        }
        let code = code.to_uppercase();
        let chars: Vec<char> = code.chars().collect();
        let flag: String = chars.iter()
            .filter_map(|c| {
                let base = 0x1F1E6 - 'A' as u32;
                char::from_u32(base + *c as u32)
            })
            .collect();
        if flag.len() == 2 { flag } else { "🌐".to_string() }
    }

    fn category_color(layer1: &str) -> &'static str {
        match layer1.to_lowercase().as_str() {
            "isp" | "transit" => "bg-blue-500/15 border-blue-500/25 text-blue-200",
            "enterprise" | "business" => "bg-emerald-500/15 border-emerald-500/25 text-emerald-200",
            "education" | "research" => "bg-violet-500/15 border-violet-500/25 text-violet-200",
            "government" => "bg-amber-500/15 border-amber-500/25 text-amber-200",
            "content" | "cdn" => "bg-pink-500/15 border-pink-500/25 text-pink-200",
            "hosting" | "cloud" => "bg-cyan-500/15 border-cyan-500/25 text-cyan-200",
            _ => "bg-slate-700/35 border-slate-600/40 text-slate-200",
        }
    }

    /// Collect all addresses from WHOIS data for geocoding.
    /// 
    /// WHOIS data stores addresses as Vec<String> where each element is one line
    /// (e.g., ["ul. Narutowicza 11/12", "80-233 Gdansk", "Poland"]).
    /// This method joins each address's lines into a single string for geocoding.
    fn collect_whois_addresses(&self) -> Vec<String> {
        let mut addresses = Vec::new();
        
        if let Some(whois) = &self.whois_data {
            // Add organisation address (all lines joined as one address)
            if let Some(org) = &whois.organisation {
                if !org.address.is_empty() {
                    let full_address = org.address
                        .iter()
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join(", ");
                    if !full_address.is_empty() {
                        addresses.push(full_address);
                    }
                }
            }
            
            // Add contact addresses (each contact's address lines joined as one address)
            for contact in &whois.contacts {
                if !contact.address.is_empty() {
                    let full_address = contact.address
                        .iter()
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join(", ");
                    if !full_address.is_empty() {
                        addresses.push(full_address);
                    }
                }
            }
        }
        
        // Remove duplicates while preserving order
        let mut seen = std::collections::HashSet::new();
        addresses.retain(|addr| seen.insert(addr.clone()));
        
        addresses
    }
}
