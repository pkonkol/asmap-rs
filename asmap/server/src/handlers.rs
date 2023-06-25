use axum::{extract::State, response::IntoResponse};
use tracing::info;

use crate::state::ServerState;
use asdb_models::As;

pub async fn hello() -> impl IntoResponse {
    "hello from server!"
}

pub async fn ases_handler(State(state): State<ServerState>) -> impl IntoResponse {
    let ases = state.asdb.get_ases(10, 0).await.unwrap();
    let serialized = bincode::serialize(&ases).unwrap();
    let deserialized: Vec<As> = bincode::deserialize(&serialized).unwrap();
    info!("ser: {ases:?}");
    info!("de: {deserialized:?}");
    serialized
}
