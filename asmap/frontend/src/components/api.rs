//! module responsible for retrieving as locations from API
//!

use asdb_models::As;

use gloo_console::log;
use gloo_net::http::Request;

const API_URL: &str = "[::1]:8080";

pub async fn get_all_as() -> Result<Vec<As>, ()> {
    let resp = Request::get(&format!("http://{API_URL}/api/ases"))
        .send()
        .await
        .unwrap();
    log!("resp is {resp:?}");
    // let body = resp.body().unwrap();
    // body should be json
    // let bin = resp.binary().await.unwrap();
    // let bin: Vec<u8> = resp.;
    // let decoded: Vec<As> = bincode::deserialize(&bin).unwrap();
    let body = resp.text().await.unwrap();
    log!("body is {}", body);
    // let json: Vec<As> = resp.json().await.unwrap();
    Ok(vec![])
}
