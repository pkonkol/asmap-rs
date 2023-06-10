//! module responsible for retrieving as locations from API
//!

// use asdb::models::As;

use gloo_console::log;
use gloo_net::http::Request;

const API_URL: &str = "localhost:8081";

pub async fn get_all_as() {
    let resp = Request::get(&format!("{API_URL}/as")).send().await.unwrap();
    log!("resp is {resp:?}");
    // let body = resp.body().unwrap();
    // body should be json
    let body = resp.text().await.unwrap();
    // let json: Vec<As> = resp.json().await.unwrap();
    println!("wtf im doing here {body:?}");
    log!("body is {body:?}");
}
