//! module responsible for retrieving as locations from API
//!

use std::vec;

use asdb_models::As;

use gloo_console::log;
use gloo_net::websocket::{futures::WebSocket, Message};

use futures::{SinkExt, StreamExt};

//const API_URL: &str = "127.0.0.1:8081";
const API_URL: &str = "[::1]:8081";

pub async fn get_all_as() -> Result<Vec<As>, ()> {
    let mut ws = WebSocket::open(&format!("ws://{API_URL}/as")).unwrap();

    let ases: Vec<As>;
    if let Some(Ok(x)) = ws.next().await {
        ases = match x {
            Message::Bytes(b) => bincode::deserialize(&b).unwrap(),
            Message::Text(t) => {
                log! {"got text message: ", t};
                vec![]
            }
        };
    } else {
        ases = vec![];
    }
    ws.close(None, None).unwrap();
    Ok(ases)
}

pub async fn debug_ws() -> anyhow::Result<()> {
    let mut ws = WebSocket::open(&format!("ws://{API_URL}/ws-test"))?;

    let t = ws.send(Message::Text("hello ws from yew".to_owned())).await;
    log!("sent: {}", t.is_ok());
    if let Some(x) = ws.next().await {
        let xfmt = format!("{x:?}");
        log!("read msg {} from ws", xfmt);
    }
    ws.close(None, None).unwrap();
    Ok(())
}
