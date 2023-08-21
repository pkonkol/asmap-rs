use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use tracing::info;

use crate::state::ServerState;
use protocol::{AsFilters, WSRequest, WSResponse};

const PAGE_SIZE: i64 = 1000;

pub async fn as_handler(
    ws: WebSocketUpgrade,
    State(state): State<ServerState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_as_socket(socket, state))
}

pub async fn handle_as_socket(mut socket: WebSocket, state: ServerState) {
    info!("started handling as socket");

    loop {
        info!("handle as socket loop start");
        let msg = if let Some(Ok(msg)) = socket.recv().await {
            info!("recieved message {msg:?}");
            msg
        } else {
            continue;
        };
        match msg {
            Message::Binary(b) => {
                let req: WSRequest = bincode::deserialize(&b).unwrap();
                match req {
                    WSRequest::AllAs(page) => {
                        let resp = all_as(page, &state).await;
                        socket.send(Message::Binary(resp)).await.unwrap();
                    }
                    WSRequest::FilteredAS(filters) => {
                        let resp = filtered_as(filters, &state).await;
                        socket.send(Message::Binary(resp)).await.unwrap();
                    }
                };
            }
            Message::Close(x) => {
                break;
            }
            _ => {}
        };
    }
    info!("closing AS socket handler");
}

/// returns WsResponse containing requested page of ases encoded using bincode
async fn all_as(page: u32, state: &ServerState) -> Vec<u8> {
    let (limit, skip): (i64, u64) = (
        page as i64 * PAGE_SIZE,
        (page as u64 * (PAGE_SIZE + 1) as u64),
    );
    let ases = state.asdb.get_ases(10, 0).await.unwrap();
    // TODO get real value of total pages instead of 10
    let resp = WSResponse::AllAs((page, 10, ases));
    let serialized = bincode::serialize(&resp).unwrap();
    info!("successfuly encoded page {page} of ases");
    serialized
}

/// returns WsResponse containing ases that match certain filters encoded using bincode
async fn filtered_as(filters: AsFilters, state: &ServerState) -> Vec<u8> {
    let ases = state
        .asdb
        .get_ases_filtered(&asdb_models::AsFilters::from(filters.clone()))
        .await
        .unwrap();
    let resp = WSResponse::FilteredAS((filters.clone(), ases));
    let serialized = bincode::serialize(&resp).unwrap();
    info!("successfuly encoded ases filtered by {filters:?} ");
    serialized
}

//
// TEST
//

pub async fn ws_test_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_test_socket)
}

pub async fn handle_test_socket(mut socket: WebSocket) {
    let r = socket.send(Message::Ping(vec![8, 2, 3])).await;
    println!("ping {r:?}");
    let r = socket
        .send(Message::Text("hello websocket".to_owned()))
        .await;
    println!("t {r:?}");
    let r = socket.send(Message::Binary(vec![138, 0])).await;
    println!("b {r:?}");
    //    let r = socket.send(Message::Close(None)).await;
    //    println!("c {r:?}");
    while let Some(msg) = socket.recv().await {
        let msg = if let Ok(msg) = msg {
            println!("msg is {msg:?}");
            msg
        } else {
            println!("client disconnected");
            return;
        };

        if let Message::Text(t) = msg {
            if socket
                .send(Message::Text(format!("received text: {t}")))
                .await
                .is_err()
            {
                println!("client disconnected on resend");
                return;
            };
        }
    }
}
