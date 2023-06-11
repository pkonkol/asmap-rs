//! module responsible for retrieving as locations from API
//!

use asdb_models::As;

use gloo_console::log;
use gloo_net::http::Request;
use gloo_net::websocket::{futures::WebSocket, Message};
//use reqwasm::websocket::{futures::WebSocket, Message};

use futures::{SinkExt, StreamExt};

//const API_URL: &str = "127.0.0.1:8081";
const API_URL: &str = "[::1]:8081";

pub async fn get_all_as() -> Result<Vec<As>, ()> {
    let resp = Request::get(&format!("http://{API_URL}/hello"))
        .send()
        .await
        .unwrap();
    log!("http hello resp is {resp:?}");
    // let body = resp.body().unwrap();
    // body should be json
    let body = resp.text().await.unwrap();
    log!("body is {:?}", body);
    //let json: Vec<As> = resp.json().await.unwrap();
    //println!("json is: {json:?}");
    //Ok(json)
    let mut ws = WebSocket::open(&format!("ws://{API_URL}/as")).unwrap();
    let t = ws.extensions();
    log!("{}", t);
    let t = ws.protocol();
    log!("{}", t);

    //let (mut write, mut read) = ws.split();
    let t = ws.send(Message::Text("hello ws from yew".to_owned())).await;
    log!("send {}", t.is_ok());
    if let Some(x) = ws.next().await {
        let xfmt = format!("{x:?}");
        log!("read msg {} from ws", xfmt);
    }
    ws.close(None, None).unwrap();
    Ok(vec![])
}
