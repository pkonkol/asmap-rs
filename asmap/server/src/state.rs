use std::sync::Arc;

use asdb::Asdb;

#[derive(Clone)]
pub struct ServerState {
    pub asdb: Arc<Asdb>,
}

impl ServerState {
    pub async fn new(conn_str: &str, db: &str) -> Self {
        let asdb = Asdb::new(conn_str, db).await.unwrap();
        Self {
            asdb: Arc::new(asdb),
        }
    }
}