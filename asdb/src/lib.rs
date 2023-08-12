mod error;

use asdb_models::{As, AsrankAsn, AsrankDegree, Coord, Nic, Organization};
pub use error::{Error, Result};

use futures::stream::TryStreamExt;
use mongodb::{
    bson::doc,
    options::{ClientOptions, FindOptions, IndexOptions},
    Client, IndexModel,
};

enum Collection {
    ASNS,
    ORGANIZATIONS,
    PREFIXES,
    PERSONS,
}

pub struct Asdb {
    client: Client,
    database: String,
}

impl Asdb {
    pub async fn new(conn_str: &str, database: &str) -> Result<Self> {
        let mut client_options = ClientOptions::parse(conn_str).await?;
        client_options.default_database = Some(database.to_string());
        let client = Client::with_options(client_options)?;
        Ok(Asdb {
            client,
            database: database.to_owned(),
        })
    }

    async fn ping(&self) -> Result<()> {
        self.client
            .database(&self.database)
            .run_command(doc! {"ping": 1}, None)
            .await?;
        Ok(())
    }

    pub async fn clear_database(&self) -> Result<()> {
        struct T {}
        for c in ["asns", "organisations", "prefixes", "persons"] {
            self.client
                .database(&self.database)
                .collection::<T>(c)
                .drop(None)
                .await?;
        }
        Ok(())
    }

    async fn prepare_database(&self) -> Result<()> {
        struct T {}
        let collection = self.client.database(&self.database).collection::<T>("asns");
        let index_options = IndexOptions::builder().unique(true).build();
        let index = IndexModel::builder()
            .keys(doc! {"asn": 1})
            .options(index_options)
            .build();
        collection.create_index(index, None).await?;
        Ok(())
    }

    pub async fn get_ases(&self, limit: i64, skip: u64) -> Result<Vec<As>> {
        let collection = self
            .client
            .database(&self.database)
            .collection::<As>("asns");
        let opts = FindOptions::builder()
            .skip(skip)
            .limit(limit)
            .sort(doc! { "asn": 1 })
            .build();
        let res = collection.find(doc! {}, opts).await?;
        let ases: Vec<As> = res.try_collect().await?;
        Ok(ases)
    }

    pub async fn get_as(&self, asn: u32) -> Result<As> {
        let collection = self
            .client
            .database(&self.database)
            .collection::<As>("asns");
        let res = collection.find_one(doc! {"asn": asn }, None).await?;
        res.ok_or(Error::AsNotFound)
    }

    pub async fn insert_as(&self, a: &As) -> Result<()> {
        let collection = self
            .client
            .database(&self.database)
            .collection::<As>("asns");
        collection.insert_one(a, None).await?;
        Ok(())
    }

    pub async fn insert_ases(&self, a: &[As]) -> Result<()> {
        let collection = self
            .client
            .database(&self.database)
            .collection::<As>("asns");
        collection.insert_many(a, None).await?;
        Ok(())
    }

    pub async fn update_as(&self, a: &As) -> Result<()> {
        let collection = self
            .client
            .database(&self.database)
            .collection::<As>("asns");
        todo!()
    }

    // TODO the same for prefixes, orgs and persons
}

#[cfg(test)]
mod tests {
    use test_context::TestContext;

    use super::*;

    //const TESTED_DB: &str = "asmap";
    const TESTED_CONN_STR: &str = "mongodb://root:devrootpass@localhost:27017";

    // TODO individual databases for each test
    //#[ctor::ctor]

    #[tokio::test(flavor = "multi_thread")]
    async fn asdb_initializes() {
        let context = TestContext::new(TESTED_CONN_STR).await.unwrap();
        let asdb = Asdb::new(TESTED_CONN_STR, &context.db_name).await.unwrap();

        asdb.ping().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_asns_executes() {
        let context = TestContext::new(TESTED_CONN_STR).await.unwrap();
        let asdb = Asdb::new(TESTED_CONN_STR, &context.db_name).await.unwrap();

        let asns = asdb.get_ases(0, 0).await.unwrap();
        println!("{asns:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn insert_asn_executes() {
        let tested_as = simple_as;
        let context = TestContext::new(TESTED_CONN_STR).await.unwrap();
        let asdb = Asdb::new(TESTED_CONN_STR, &context.db_name).await.unwrap();

        asdb.insert_as(&tested_as()).await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn insert_then_get_asns() {
        let tested_as = as_with_asrank;
        let tested_asn = tested_as().asn;

        let context = TestContext::new(TESTED_CONN_STR).await.unwrap();
        let asdb = Asdb::new(TESTED_CONN_STR, &context.db_name).await.unwrap();

        asdb.insert_as(&tested_as()).await.unwrap();
        let asns = asdb.get_ases(0, 0).await.unwrap();
        println!("{asns:?}");
        assert!(asns.iter().find(|x| x.asn == tested_asn).is_some());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn insert_then_get_asn() {
        let tested_as = as_with_asrank;
        let tested_asn = tested_as().asn;

        let context = TestContext::new(TESTED_CONN_STR).await.unwrap();
        let asdb = Asdb::new(TESTED_CONN_STR, &context.db_name).await.unwrap();
        asdb.insert_as(&tested_as()).await.unwrap();
        let asn = asdb.get_as(tested_asn).await.unwrap();
        println!("{asn:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn clear_database_then_get_asns() {
        let context = TestContext::new(TESTED_CONN_STR).await.unwrap();
        let asdb = Asdb::new(TESTED_CONN_STR, &context.db_name).await.unwrap();
        asdb.clear_database().await.unwrap();
        let asns = asdb.get_ases(0, 0).await.unwrap();
        println!("{asns:?}");
        assert_eq!(asns.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn inserting_twice_after_creating_index_fails() {
        let tested_as = simple_as;

        let context = TestContext::new(TESTED_CONN_STR).await.unwrap();
        let asdb = Asdb::new(TESTED_CONN_STR, &context.db_name).await.unwrap();

        asdb.clear_database().await.unwrap();
        asdb.prepare_database().await.unwrap();
        asdb.insert_as(&tested_as()).await.unwrap();
        let x = asdb.get_ases(0, 0).await.unwrap();
        println!("first: {x:?}");
        let second_insert = asdb.insert_as(&tested_as()).await;
        let x = asdb.get_ases(0, 0).await.unwrap();
        println!("second: {x:?}");

        assert!(second_insert.is_err());
    }

    fn simple_as() -> As {
        As {
            asn: 5551,
            asrank_data: None,
            ipnetdb_data: None,
            whois_data: None,
        }
    }

    fn as_with_asrank() -> As {
        let asrank = AsrankAsn {
            rank: 5476,
            organization: Some(
                "Technical University of Gdansk, Academic Computer Center TASK".to_string(),
            ),
            country_iso: String::from("PL"),
            country_name: String::from("Poland"),
            coordinates: Coord {
                lon: 18.5620133480526,
                lat: 54.3745639215642,
            },
            degree: AsrankDegree {
                provider: 2,
                peer: 10,
                customer: 2,
                total: 14,
                transit: 13,
                sibling: 1,
            },
            prefixes: 1,
            addresses: 65536,
        };
        As {
            asn: 5550,
            asrank_data: Some(asrank),
            ipnetdb_data: None,
            whois_data: None,
        }
    }
}
