mod asrank;
mod error;
mod ipnetdb;
mod stanford_asdb;
mod whois;

use std::path::{Path, PathBuf};

use asdb::Asdb;
use error::Result;

// TODO manage asdb object better
pub struct AsdbBuilder {
    a: Asdb,
    inputs: PathBuf,
}

impl AsdbBuilder {
    pub async fn new(conn_str: &str, database: &str, inputs_path: &str) -> Result<Self> {
        let a = Asdb::new(conn_str, database).await?;
        Ok(Self {
            a,
            inputs: PathBuf::from(inputs_path),
        })
    }

    pub async fn clear_database(&self) -> Result<()> {
        self.a.clear_database().await?;
        self.a.prepare_database().await?;
        Ok(())
    }

    pub async fn load_asrank_asns(&self, asns_jsonl: Option<impl AsRef<Path>>) -> Result<()> {
        asrank::load(&self.a, asns_jsonl.map(|x| self.inputs.join(x))).await?;
        //    .map_err(|e| e.into())
        Ok(())
    }

    pub async fn load_ipnetdb(&self) -> Result<()> {
        ipnetdb::load(&self.a).await?;
        Ok(())
    }

    pub async fn load_stanford_asdb(&self) -> Result<()> {
        stanford_asdb::load(&self.a).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use futures::stream::TryStreamExt;
    use mongodb::{bson::doc, options::ClientOptions, Client, Collection};

    use super::*;
    use asdb_models::As;
    use std::fs::read_to_string;
    use test_context::TestContext;

    const ASDB_CONN_STR: &str = "mongodb://root:devrootpass@localhost:27017";
    const ASNS_COLLECTION: &str = "asns";
    const ASNS: &str = "asns.jsonl";
    const ASNS2: &str = "asns2.jsonl";
    const INPUTS_PATH: &str = "inputs/test-data";

    #[tokio::test(flavor = "multi_thread")]
    async fn import_asrank_asns_fills_asdb() {
        let context = TestContext::new(ASDB_CONN_STR).await.unwrap();

        let m = AsdbBuilder::new(ASDB_CONN_STR, &context.db_name, INPUTS_PATH)
            .await
            .unwrap();
        m.clear_database().await.unwrap();
        m.load_asrank_asns(Some(&ASNS)).await.unwrap();

        let lines = count_lines(&PathBuf::from(INPUTS_PATH).join(ASNS));
        let docs = count_asn_entries(&context.db_name).await;

        assert_eq!(lines, docs);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn import_asrank_asns_twice_does_not_duplicate_entries() {
        let context = TestContext::new(ASDB_CONN_STR).await.unwrap();

        let m = AsdbBuilder::new(ASDB_CONN_STR, &context.db_name, INPUTS_PATH)
            .await
            .unwrap();
        m.clear_database().await.unwrap();
        m.load_asrank_asns(Some(&ASNS)).await.unwrap();

        let lines = count_lines(&PathBuf::from(INPUTS_PATH).join(ASNS));
        let docs = count_asn_entries(&context.db_name).await;
        assert_eq!(lines, docs);

        m.load_asrank_asns(Some(&ASNS)).await.unwrap();

        let lines = count_lines(&PathBuf::from(INPUTS_PATH).join(ASNS));
        let docs = count_asn_entries(&context.db_name).await;
        assert_eq!(lines, docs);

        // 1299 is just hardcoded value for asn that must be in test-data/asns.jsonl
        let ases = get_asn_entries(1299, &context.db_name).await;
        assert_eq!(ases.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn import_asrank_with_overlapping_supersets_appends_only_new_ones() {
        let context = TestContext::new(ASDB_CONN_STR).await.unwrap();

        let m = AsdbBuilder::new(ASDB_CONN_STR, &context.db_name, INPUTS_PATH)
            .await
            .unwrap();
        m.clear_database().await.unwrap();
        m.load_asrank_asns(Some(&ASNS)).await.unwrap();
        let first_docs = count_asn_entries(&context.db_name).await;

        m.load_asrank_asns(Some(&ASNS2)).await.unwrap();
        let second_docs = count_asn_entries(&context.db_name).await;

        let ases = m.a.get_ases(0, 0).await.unwrap();
        println!("ases: {ases:#?}");

        assert!(second_docs > first_docs);

        // 1299 must be both in asns.jsonl and in asns2.jsonl
        let ases = get_asn_entries(1299, &context.db_name).await;
        assert_eq!(ases.len(), 1);
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
