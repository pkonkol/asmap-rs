use asdb::Asdb;
use asdb_models::As;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    response::IntoResponse,
};

const TMP_DB: &str = "asdb";
const TMP_CONN_STR: &str = "mongodb://devuser:devpass@localhost:27017/?authSource=asdb";

pub async fn hello() -> impl IntoResponse {
    "hello from server!"
}

pub async fn as_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_as_socket)
}

pub async fn handle_as_socket(mut socket: WebSocket) {
    socket.send(Message::Text("db".to_owned())).await.unwrap();
    let asdb = Asdb::new(TMP_CONN_STR, TMP_DB).await.unwrap();
    let ases = asdb.get_ases(0, 0).await.unwrap();

    let mut buf = Vec::<u8>::new();
    ciborium::into_writer(&ases, &mut buf).unwrap();
    socket
        .send(Message::Text("start".to_owned()))
        .await
        .unwrap();
    socket.send(Message::Binary(buf)).await.unwrap();
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
