pub mod api;
pub mod control;
pub mod map_component;

use crate::components::{
    control::{Cities, Control},
    map_component::{City, MapComponent, Point},
};
use asdb_models::As;
use yew::prelude::*;

pub enum Msg {
    SelectCity(City),
    DrawAses(Vec<As>),
}

pub struct MapContainer {
    city: City,
    cities: Cities,
}

impl Component for MapContainer {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let aachen = City {
            name: "Aachen".to_string(),
            lat: Point(50.7597f64, 6.0967f64),
        };
        let stuttgart = City {
            name: "Stuttgart".to_string(),
            lat: Point(48.7784f64, 9.1742f64),
        };
        let gdynia = City {
            name: "Gdynia".to_string(),
            lat: Point(54.5189f64, 18.5305f64),
        };
        let city = gdynia.clone();
        let cities: Cities = Cities {
            list: vec![aachen, stuttgart, gdynia],
        };
        // let city = cities.list[0].clone();
        Self { city, cities }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SelectCity(city) => {
                self.city = self
                    .cities
                    .list
                    .iter()
                    .find(|c| c.name == city.name)
                    .unwrap()
                    .clone();
            }
            Msg::DrawAses(v) => {
                println!("")
            }
        }
        true
    }

    fn changed(&mut self, _ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let cb = ctx.link().callback(Msg::SelectCity);
        let draw_as_cb = ctx.link().callback(Msg::DrawAses);
        html! {
            <>
                <MapComponent city={&self.city}  />
                <Control select_city={cb} draw_ases={draw_as_cb} cities={&self.cities}/>
            </>
        }
    }
}
