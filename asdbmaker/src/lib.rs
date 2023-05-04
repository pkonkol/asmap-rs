mod asrank;
mod error;
mod ipnetdb;
mod whois;

use std::path::Path;

use asdb::{
    models::{As, AsrankAsn},
    Asdb,
};
use asrank::import_asns;
use error::Result;

const ASDB_CONN_STR: &str = "mongodb://devuser:devpass@localhost:27018/?authSource=asdbmaker";
const ASDB_DB: &str = "asdbmaker";
// TODO manage asdb object better
// struct Asdbmaker { a: Asdb } maybe

pub async fn import_asrank_asns(asns: &impl AsRef<Path>) -> Result<()> {
    let asdb = Asdb::new(ASDB_CONN_STR, ASDB_DB).await?;
    import_asns(asns, &asdb).await.map_err(|e| e.into())
}

pub async fn import_ipnetdb_prefixes() -> Result<()> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ASRANK_ASNS_PATH: &str = "asrank/asns.jsonl";
    const ASRANK_ORGS_PATH: &str = "asrank/organizations.jsonl";

    #[tokio::test]
    async fn import_asrank_asns_fills_asdb() {
        import_asrank_asns(&ASRANK_ASNS_PATH.to_string())
            .await
            .unwrap();

        // verify number of entries in mongo
    }

    #[ignore]
    #[tokio::test]
    async fn import_asrank_asns_twice_does_not_duplicate_entries() {
        import_asrank_asns(&ASRANK_ASNS_PATH.to_string())
            .await
            .unwrap();
        todo!()
        // verify number of entries in mongo
        // verify find for given AS returns only one
    }
}
