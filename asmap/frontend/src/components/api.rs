//! module responsible for retrieving as locations from API
//!

use asdb_models::As;
use protocol::{AsFilters, AsForFrontend, WSRequest, WSResponse};

use anyhow::{anyhow, bail};
use futures::{SinkExt, StreamExt};
use gloo_console::log;
use gloo_net::websocket::{futures::WebSocket, Message};
use std::vec;

const API_URL: &str = "[::1]:8081";

pub async fn get_all_as_filtered(filters: AsFilters) -> anyhow::Result<Vec<AsForFrontend>> {
    let mut ws = WebSocket::open(&format!("ws://{API_URL}/as"))?;

    let mut out: Vec<AsForFrontend> = vec![];

    let req = WSRequest::FilteredAS(filters);
    ws.send(Message::Bytes(bincode::serialize(&req)?)).await?;
    log!("sent request for filtered ASes");

    let resp = ws
        .next()
        .await
        .ok_or(anyhow!("didn't receive the message"))??;
    log!("received response for filtered ases");

    let resp: WSResponse = if let Message::Bytes(b) = resp {
        log!("deserializing message");
        bincode::deserialize(&b)?
    } else {
        bail!("Received message is not of Bytes type");
    };

    log!("appending recieved page to output");
    if let WSResponse::FilteredAS((_filters, mut ases)) = resp {
        log!(format!("appending {} ases to out", ases.len()));
        out.append(&mut ases);
    } else {
        bail!("wrong response");
    }

    ws.close(None, None)?;
    Ok(out)
}

pub async fn get_all_as() -> anyhow::Result<Vec<AsForFrontend>> {
    let mut ws = WebSocket::open(&format!("ws://{API_URL}/as"))?;

    let mut out: Vec<AsForFrontend> = vec![];
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
    }

    ws.close(None, None)?;
    Ok(out)
}
