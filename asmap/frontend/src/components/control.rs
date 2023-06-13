use asdb_models::As;

use super::{api::get_all_as, map_component::City};
use gloo_console::log;
use yew::{html::ImplicitClone, prelude::*};

pub enum Msg {
    CityChosen(City),
    LoadAllAs,
    DrawAllAs(Vec<As>),
    Error,
}
pub struct Control {
    cities: Vec<City>,
}

#[derive(PartialEq, Clone)]
pub struct Cities {
    pub list: Vec<City>,
}

impl ImplicitClone for Cities {}

#[derive(PartialEq, Properties, Clone)]
pub struct Props {
    pub cities: Cities,
    pub select_city: Callback<City>,
}

impl Control {
    fn button(&self, ctx: &Context<Self>, city: City) -> Html {
        let name = city.name.clone();
        let cb = ctx.link().callback(move |_| Msg::CityChosen(city.clone()));
        html! {
            <button onclick={cb}>{name}</button>
        }
    }

    fn load_as_button(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(move |_| Msg::LoadAllAs);
        html! {
            <button onclick={cb}>{"Load ASes"}</button>
        }
    }
}

impl Component for Control {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        Control {
            cities: ctx.props().cities.list.clone(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::CityChosen(city) => {
                log!(format!("Update: {:?}", city.name));
                ctx.props().select_city.emit(city);
            }
            Msg::LoadAllAs => {
                log!("load ASes initiatied");
                ctx.link().send_future(async {
                    match get_all_as().await {
                        Ok(ases) => Msg::DrawAllAs(ases),
                        Err(_) => Msg::Error,
                    }
                });
            }
            Msg::DrawAllAs(ases) => {
                log!("ASes fetched, drawing them");
            }
            Msg::Error => {
                log!("error fetching ases");
            }
        }
        true
    }

    fn changed(&mut self, _ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="control component-container">
                <h1>{"Choose a city"}</h1>
                <div>
                    {for self.cities.iter().map(|city| Self::button(self, ctx, city.clone()))}
                    </div>
                <div>
                    {Self::load_as_button(self, ctx)}
                </div>
            </div>
        }
    }
}
