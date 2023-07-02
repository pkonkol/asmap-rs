//! module responsible for retrieving as locations from API
//!

use std::vec;

use asdb_models::As;

use gloo_console::log;
use gloo_net::http::Request;
use gloo_net::websocket::{futures::WebSocket, Message};

use futures::{SinkExt, StreamExt};

//const API_URL: &str = "127.0.0.1:8081";
const API_URL: &str = "[::1]:8081";

pub async fn get_all_as() -> Result<Vec<As>, ()> {
    let mut ws = WebSocket::open(&format!("ws://{API_URL}/as")).unwrap();
    let t = ws.extensions();
    log!("extensions: {}", t);
    let t = ws.protocol();
    log!("protocol: {}", t);

    let ases: Vec<As>;
    if let Some(Ok(x)) = ws.next().await {
        ases = match x {
            Message::Bytes(b) => {
                log!("got vec of bytes ", format!("{b:?}"));
                bincode::deserialize(&b).unwrap()
            }
            Message::Text(t) => {
                log! {"got text message: ", t};
                vec![]
            }
        };
        log! {"decoded cbor to ", format!("{ases:?}")};
    } else {
        ases = vec![];
    }
    ws.close(None, None).unwrap();
    Ok(ases)
}

pub async fn debug_ws() -> Result<Vec<As>, ()> {
    let mut ws = WebSocket::open(&format!("ws://{API_URL}/ws-test")).unwrap();
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
