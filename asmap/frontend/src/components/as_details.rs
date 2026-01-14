use yew::prelude::*;
use web_sys::UrlSearchParams;
use gloo_console::log;
use asdb_models::WhoIsAsn;

use crate::components::api::fetch_as_whois_data;

#[derive(Properties, PartialEq, Clone)]
pub struct Props {
    pub asn: u32,
}

pub enum Msg {
    Load,
    WhoisLoaded(Option<WhoIsAsn>),
    Error(String),
    Back,
}

pub struct AsDetailsPage {
    whois: Option<WhoIsAsn>,
    loading: bool,
    err: Option<String>,
}

impl Component for AsDetailsPage {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::Load);

        Self {
            whois: None,
            loading: true,
            err: None,
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, _old: &Self::Properties) -> bool {
        ctx.link().send_message(Msg::Load);
        true
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Load => {
                self.loading = true;
                self.err = None;
                self.whois = None;

                let asn = ctx.props().asn;
                ctx.link().send_future(async move {
                    match fetch_as_whois_data(asn).await {
                        Ok(data) => {
                            // Log the WHOIS data to console
                            log!(format!("WHOIS data for AS{}: {:?}", asn, data));
                            Msg::WhoisLoaded(data)
                        }
                        Err(e) => Msg::Error(format!("{e:?}")),
                    }
                });
            }
            Msg::WhoisLoaded(data) => {
                self.loading = false;
                self.whois = data;
            }
            Msg::Error(e) => {
                self.loading = false;
                self.err = Some(e);
            }
            Msg::Back => {
                if let Some(win) = web_sys::window() {
                    let _ = win.location().set_hash("/");
                }
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let asn = ctx.props().asn;

        html! {
            <div class="min-h-screen bg-slate-950 text-slate-100 p-4">
                <div class="max-w-5xl mx-auto space-y-4">
                    <div class="flex items-center justify-between">
                        <div class="text-xl font-semibold">{ format!("AS{} - WHOIS", asn) }</div>
                        <button
                            class="px-3 py-2 rounded-lg bg-slate-800 hover:bg-slate-700 text-sm"
                            onclick={ctx.link().callback(|_| Msg::Back)}
                        >
                            {"← Back"}
                        </button>
                    </div>

                    if let Some(err) = &self.err {
                        <div class="p-3 rounded-lg bg-red-900/40 border border-red-800 text-sm">{ err.clone() }</div>
                    }

                    <div class="p-4 rounded-xl bg-slate-900/60 border border-slate-800">
                        {
                            if self.loading {
                                html!{ <div class="text-sm text-slate-300">{"Loading WHOIS..."}</div> }
                            } else if let Some(w) = &self.whois {
                                self.render_whois(w)
                            } else {
                                html!{ <div class="text-sm text-slate-400">{"No WHOIS data available."}</div> }
                            }
                        }
                    </div>
                </div>
            </div>
        }
    }
}

impl AsDetailsPage {
    fn render_whois(&self, w: &WhoIsAsn) -> Html {
        html! {
            <div class="space-y-4 text-sm">
                if let Some(name) = &w.as_name {
                    <div><span class="text-slate-400">{"AS Name: "}</span>{ name }</div>
                }
                if !w.descr.is_empty() {
                    <div>
                        <span class="text-slate-400">{"Description: "}</span>
                        { w.descr.join(", ") }
                    </div>
                }
                if let Some(country) = &w.country {
                    <div><span class="text-slate-400">{"Country: "}</span>{ country }</div>
                }
                if let Some(org) = &w.organisation {
                    <div class="mt-4 p-3 rounded-lg bg-slate-800/50">
                        <div class="font-semibold mb-2">{"Organisation"}</div>
                        <div><span class="text-slate-400">{"Name: "}</span>{ &org.org_name }</div>
                        if let Some(t) = &org.org_type {
                            <div><span class="text-slate-400">{"Type: "}</span>{ t }</div>
                        }
                        if !org.address.is_empty() {
                            <div><span class="text-slate-400">{"Address: "}</span>{ org.address.join(", ") }</div>
                        }
                        if let Some(email) = &org.email {
                            <div><span class="text-slate-400">{"Email: "}</span>{ email }</div>
                        }
                    </div>
                }
                if !w.contacts.is_empty() {
                    <div class="mt-4">
                        <div class="font-semibold mb-2">{ format!("Contacts ({})", w.contacts.len()) }</div>
                        { for w.contacts.iter().map(|c| html! {
                            <div class="p-2 mb-2 rounded bg-slate-800/30">
                                <div>{ format!("{} ({})", c.name, c.nic_hdl) }</div>
                                if !c.address.is_empty() {
                                    <div class="text-xs text-slate-400">{ c.address.join(", ") }</div>
                                }
                            </div>
                        })}
                    </div>
                }
                if let Some(ts) = &w.fetched_at {
                    <div class="text-xs text-slate-500 mt-4">{ format!("Fetched at: {}", ts) }</div>
                }
            </div>
        }
    }
}
