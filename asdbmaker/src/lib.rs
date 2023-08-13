mod asrank;
mod error;
mod ipnetdb;
mod whois;

use std::path::{Path, PathBuf};

use asdb::Asdb;
use asdb_models::{As, AsrankAsn};
use asrank::import_asns;
use error::Result;
use test_context::TestContext;

const ASNS: &str = "asns.jsonl";

// TODO manage asdb object better
pub struct Asdbmaker {
    a: Asdb,
    inputs: PathBuf,
}

impl Asdbmaker {
    pub async fn new(conn_str: &str, database: &str, inputs_path: &str) -> Result<Self> {
        let a = Asdb::new(conn_str, database).await?;
        Ok(Self {
            a,
            inputs: PathBuf::from(inputs_path),
        })
    }

    pub async fn clear_database(&self) -> Result<()> {
        self.a.clear_database().await?;
        Ok(())
    }

    pub async fn import_asrank_asns(&self) -> Result<()> {
        import_asns(&self.inputs.join(&ASNS), &self.a)
            .await
            .map_err(|e| e.into())
    }

    pub async fn import_ipnetdb_prefixes() -> Result<()> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use futures::stream::TryStreamExt;
    use mongodb::{bson::doc, options::ClientOptions, Client, Collection};

    use super::*;
    use std::fs::read_to_string;

    // const ASRANK_ASNS_PATH: &str = "asrank/asns.jsonl";
    const ASDB_CONN_STR: &str = "mongodb://root:devrootpass@localhost:27017";
    // const ASDB_DB: &str = "asmap";
    const ASNS_COLLECTION: &str = "asns";
    const INPUTS_PATH: &str = "inputs/test-data";
    // const ASRANK_ORGS_PATH: &str = "asrank/organizations.jsonl";

    #[tokio::test(flavor = "multi_thread")]
    async fn import_asrank_asns_fills_asdb() {
        let context = TestContext::new(ASDB_CONN_STR).await.unwrap();

        let m = Asdbmaker::new(ASDB_CONN_STR, &context.db_name, INPUTS_PATH)
            .await
            .unwrap();
        m.clear_database().await.unwrap();
        m.import_asrank_asns().await.unwrap();

        let lines = count_lines(&PathBuf::from(INPUTS_PATH).join(ASNS));
        let docs = count_asn_entries(&context.db_name).await;

        assert_eq!(lines, docs);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn import_asrank_asns_twice_does_not_duplicate_entries() {
        let context = TestContext::new(ASDB_CONN_STR).await.unwrap();

        let m = Asdbmaker::new(ASDB_CONN_STR, &context.db_name, INPUTS_PATH)
            .await
            .unwrap();
        m.clear_database().await.unwrap();
        m.import_asrank_asns().await.unwrap();

        let lines = count_lines(&PathBuf::from(INPUTS_PATH).join(ASNS));
        let docs = count_asn_entries(&context.db_name).await;

        assert_eq!(lines, docs);

        m.import_asrank_asns().await.unwrap();

        let lines = count_lines(&PathBuf::from(INPUTS_PATH).join(ASNS));
        let docs = count_asn_entries(&context.db_name).await;

        assert_eq!(lines, docs);

        let ases = get_asn_entries(5550, &context.db_name).await;
        assert_eq!(ases.len(), 1);

        // verify find for given AS returns only one
        // could it end up otherwise anyway? yeah it's mongo
    }

    fn count_lines(path: &impl AsRef<Path>) -> u64 {
        read_to_string(path).unwrap().lines().map(|_| 1).sum()
    }

    async fn count_asn_entries(db_name: &str) -> u64 {
        let mut client_options = ClientOptions::parse(ASDB_CONN_STR).await.unwrap();
        client_options.default_database = Some(db_name.to_string());
        let client = Client::with_options(client_options).unwrap();
        let c: Collection<As> = client.database(db_name).collection(ASNS_COLLECTION);
        c.count_documents(None, None).await.unwrap()
    }

    async fn get_asn_entries(asn: u32, db_name: &str) -> Vec<As> {
        let mut client_options = ClientOptions::parse(ASDB_CONN_STR).await.unwrap();
        client_options.default_database = Some(db_name.to_string());
        let client = Client::with_options(client_options).unwrap();
        let c: Collection<As> = client.database(db_name).collection(ASNS_COLLECTION);
        let cur = c.find(doc! {"asn": asn}, None).await.unwrap();
        cur.try_collect().await.unwrap()
    }
}
