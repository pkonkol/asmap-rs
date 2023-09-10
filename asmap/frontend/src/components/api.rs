//! module responsible for retrieving as locations from API
//!

use asdb_models::As;
use protocol::{AsFilters, WSRequest, WSResponse};

use anyhow::{anyhow, bail};
use futures::{SinkExt, StreamExt};
use gloo_console::log;
use gloo_net::websocket::{futures::WebSocket, Message};
use std::vec;

//const API_URL: &str = "127.0.0.1:8081";
const API_URL: &str = "[::1]:8081";

pub async fn get_all_as() -> anyhow::Result<Vec<As>> {
    let mut ws = WebSocket::open(&format!("ws://{API_URL}/as"))?;

    let mut out: Vec<As> = vec![];
    let mut page = 0;
    loop {
        let req = WSRequest::AllAs(page);
        ws.send(Message::Bytes(bincode::serialize(&req)?)).await?;
        log!("sent request for page ", page);

        let resp = ws
            .next()
            .await
            .ok_or(anyhow!("didn't receive the message"))??;
        log!("received response for page ", page);

        let resp: WSResponse = if let Message::Bytes(b) = resp {
            log!("deserializing message");
            bincode::deserialize(&b)?
        } else {
            bail!("Received message is not of Bytes type");
        };

        log!("appending recieved page to output");
        if let WSResponse::AllAs((page, total_pages, mut vec)) = resp {
            out.append(&mut vec);
            log!(format!("appending {vec:?} to out"));
            if page >= total_pages {
                break;
            }
        } else {
            bail!("wrong response");
        }
        page += 1;
        // for debug purposes
        if page > 10 {
            break;
        }
        // break; // tmp
    }

    // let ases: Vec<As> = if let Some(Ok(x)) = ws.next().await {
    //     match x {
    //         // TODO wrap the bytes in an enum which will define requests and responses, match on them
    //         // server and client side
    //         Message::Bytes(b) => bincode::deserialize(&b)?,
    //         Message::Text(t) => {
    //             log! {"got text message: ", t};
    //             vec![]
    //         }
    //     }
    // } else {
    //     vec![]
    // };
    ws.close(None, None)?;
    // Ok(ases)
    Ok(out)
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
