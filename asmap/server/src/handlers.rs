use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use tracing::info;

use crate::state::ServerState;

pub async fn as_handler(
    ws: WebSocketUpgrade,
    State(state): State<ServerState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_as_socket(socket, state))
}

pub async fn handle_as_socket(mut socket: WebSocket, state: ServerState) {
    info!("started handling as socket");
    let ases = state.asdb.get_ases(10, 0).await.unwrap();

    let serialized = bincode::serialize(&ases).unwrap();
    info!("encoded ases");
    socket.send(Message::Binary(serialized)).await.unwrap();
    info!("sent encoded ases");
    socket
        .send(Message::Text("finish".to_owned()))
        .await
        .unwrap();
}

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
