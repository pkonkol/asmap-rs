use std::{net::SocketAddr, num::NonZeroU32};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use nonzero_ext::nonzero;
use tracing::{debug, info, trace};

use crate::state::ServerState;
use protocol::{AsFilters, WSRequest, WSResponse};

pub async fn as_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<ServerState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_as_socket(socket, addr, state))
}

#[tracing::instrument(skip(state, socket))]
pub async fn handle_as_socket(mut socket: WebSocket, addr: SocketAddr, state: ServerState) {
    loop {
        trace!("handle_as_socket loop start");
        let msg = if let Some(Ok(msg)) = socket.recv().await {
            msg
        } else {
            continue;
        };
        match msg {
            Message::Binary(b) => {
                let req: WSRequest = bincode::deserialize(&b).unwrap();
                match req {
                    WSRequest::FilteredAS(filters) => {
                        info!(
                            "reveived WSRequest::FilteredAs with filters {filters:?} from {}",
                            addr.ip()
                        );
                        let resp = filtered_as(filters, addr, &state).await;
                        socket.send(Message::Binary(resp)).await.unwrap();
                        socket.send(Message::Close(None)).await.unwrap();
                        break;
                    }
                    WSRequest::AsDetails(asn) => {
                        info!(
                            "reveived WSRequest::AsDetails for asn {asn} from {}",
                            addr.ip()
                        );
                        let resp = as_details(asn, addr, &state).await;
                        socket.send(Message::Binary(resp)).await.unwrap();
                        socket.send(Message::Close(None)).await.unwrap();
                        break;
                    }
                };
            }
            Message::Close(_x) => {
                info!("reveived websocket Message::Close from {}", addr.ip());
                break;
            }
            _ => {
                info!(
                    "Received unsupported message type: {msg:?} from {}",
                    addr.ip()
                );
                socket.send(Message::Close(None)).await.unwrap();
                break;
            }
        };
    }
}

/// returns WsResponse containing ases that match certain filters encoded using bincode
#[tracing::instrument(skip(state))]
async fn filtered_as(filters: AsFilters, addr: SocketAddr, state: &ServerState) -> Vec<u8> {
    let ases_count = state
        .asdb
        .count_ases_filtered(&asdb_models::AsFilters::from(filters.clone()))
        .await
        .unwrap();
    debug!("ases count for current filters is {ases_count}");

    state
        .simple_limiter
        .check_key_n(&addr.ip(), NonZeroU32::new(ases_count as u32).unwrap())
        .unwrap()
        .unwrap();

    let ases = state
        .asdb
        .get_ases_filtered(&asdb_models::AsFilters::from(filters.clone()))
        .await
        .unwrap();
    let resp = WSResponse::FilteredAS((filters.clone(), ases));
    let serialized = bincode::serialize(&resp).unwrap();
    debug!("successfuly encoded ases filtered by {filters:?} ");
    serialized
}

/// returns WsResponse containing details for single AS encoded using bincode
#[tracing::instrument(skip(state))]
async fn as_details(asn: u32, addr: SocketAddr, state: &ServerState) -> Vec<u8> {
    // separate limiter for detailed request? would be best TODO
    state
        .detailed_limiter
        .check_key_n(&addr.ip(), nonzero!(1u32))
        .unwrap()
        .unwrap();

    let as_ = state.asdb.get_as(asn).await.unwrap();
    let resp = WSResponse::AsDetails(as_);
    let serialized = bincode::serialize(&resp).unwrap();
    debug!("successfuly encoded AS{asn} details");
    serialized
}
