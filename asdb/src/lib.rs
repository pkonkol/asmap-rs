mod error;

use asdb_models::{As, AsFilters, AsrankAsn, AsrankDegree, Coord};
pub use error::{Error, Result};

use futures::stream::TryStreamExt;
use mongodb::{
    bson::{doc, Document},
    options::{ClientOptions, FindOptions, IndexOptions, InsertManyOptions},
    Client, IndexModel,
};
use tracing::info;

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
        let s = Asdb {
            client,
            database: database.to_owned(),
        };
        Self::prepare_database(&s).await?;
        Ok(s)
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

    pub async fn prepare_database(&self) -> Result<()> {
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

    // TODO consider merging get_ases and get_as_filtered, just do filters: Option<AsFilters>
    /// returns result with found ases and total count of ases in the DB
    pub async fn get_ases(&self, limit: i64, skip: u64) -> Result<(Vec<As>, u64)> {
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
        let count = collection.count_documents(doc! {}, None).await?;
        let ases: Vec<As> = res.try_collect().await?;
        info!(
            "found {} mathcing ases for skip {skip} and limit {limit}",
            ases.len()
        );
        Ok((ases, count))
    }

    fn create_db_filter(filters: &AsFilters) -> Document {
        let mut db_filter = doc! {};
        if let Some(bounds) = &filters.bounds {
            // are the comparisons correct?
            db_filter.insert(
                "asrank_data.coordinates.lat",
                doc! {"$lte": bounds.north_east.lat, "$gt": bounds.south_west.lat},
            );
            db_filter.insert(
                "asrank_data.coordinates.lon",
                doc! {"$lte": bounds.north_east.lon, "$gt": bounds.south_west.lon},
            );
        }
        if let Some(x) = &filters.country_iso {
            db_filter.insert("asrank_data.country_iso", x);
        }
        if let Some((min, max)) = &filters.addresses {
            // gt than min and lt than max
            db_filter.insert("asrank_data.addresses", doc! {"$gte": min, "$lte": max});
        }
        if let Some((min, max)) = &filters.rank {
            // gt than min and lt than max
            db_filter.insert("asrank_data.rank", doc! {"$gte": min, "$lte": max});
        }
        if let Some(true) = &filters.has_org {
            // gt than min and lt than max
            db_filter.insert("asrank_data.organization", doc! {"$ne": null});
        }
        db_filter
    }

    pub async fn count_ases_filtered(&self, filters: &AsFilters) -> Result<u64> {
        let collection = self
            .client
            .database(&self.database)
            .collection::<As>("asns");
        let db_filter = Self::create_db_filter(filters);

        // let opts = FindOptions::builder().sort(doc! { "asn": 1 }).build();
        let res = collection.count_documents(db_filter, None).await?;
        Ok(res)
    }

    pub async fn get_ases_filtered(&self, filters: &AsFilters) -> Result<Vec<As>> {
        let collection = self
            .client
            .database(&self.database)
            .collection::<As>("asns");
        let db_filter = Self::create_db_filter(filters);

        let opts = FindOptions::builder().sort(doc! { "asn": 1 }).build();
        let res = collection.find(db_filter, opts).await?;
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
        // let opts = UpdateOptions::builder().upsert(true).build();
        // collection.update_many(doc! {}, a, opts).await?;
        let opts = InsertManyOptions::builder().ordered(false).build();
        collection.insert_many(a, opts).await?;
        Ok(())
    }

    pub async fn update_as(&self, _a: &As) -> Result<()> {
        let _collection = self
            .client
            .database(&self.database)
            .collection::<As>("asns");
        todo!()
    }

    // TODO the same for prefixes, orgs and persons
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use test_context::TestContext;

    use super::*;

    const TESTED_CONN_STR: &str = "mongodb://root:devrootpass@localhost:27017";

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

        asdb.get_ases(0, 0).await.unwrap();
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
        let (asns, count) = asdb.get_ases(0, 0).await.unwrap();
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
        assert_eq!(
            tested_as().asrank_data.unwrap().rank,
            asn.asrank_data.unwrap().rank
        );
        assert_eq!(tested_as().whois_data.is_none(), asn.whois_data.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn clear_database_then_get_asns() {
        let context = TestContext::new(TESTED_CONN_STR).await.unwrap();
        let asdb = Asdb::new(TESTED_CONN_STR, &context.db_name).await.unwrap();
        asdb.clear_database().await.unwrap();
        let (asns, count) = asdb.get_ases(0, 0).await.unwrap();
        assert_eq!(asns.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn inserting_twice_does_not_duplicate() {
        let tested_as = simple_as;

        let context = TestContext::new(TESTED_CONN_STR).await.unwrap();
        let asdb = Asdb::new(TESTED_CONN_STR, &context.db_name).await.unwrap();

        asdb.clear_database().await.unwrap();
        asdb.prepare_database().await.unwrap();
        asdb.insert_as(&tested_as()).await.unwrap();
        let first_get = asdb.get_ases(0, 0).await.unwrap();
        let second_insert = asdb.insert_as(&tested_as()).await;
        let second_get = asdb.get_ases(0, 0).await.unwrap();

        assert!(second_insert.is_err());
        assert_eq!(first_get.0.len(), second_get.0.len());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn inserting_twice_many_does_not_duplicate() {
        let tested_ases = simple_vec_as;

        let context = TestContext::new(TESTED_CONN_STR).await.unwrap();
        let asdb = Asdb::new(TESTED_CONN_STR, &context.db_name).await.unwrap();

        asdb.clear_database().await.unwrap();
        asdb.prepare_database().await.unwrap();
        asdb.insert_ases(&tested_ases()).await.unwrap();

        let (first_get, _) = asdb.get_ases(0, 0).await.unwrap();

        let second_insert = asdb.insert_ases(&tested_ases()).await;
        let (second_get, _) = asdb.get_ases(0, 0).await.unwrap();

        assert!(second_insert.is_err());
        assert_eq!(first_get.len(), second_get.len());
        assert_eq!(
            first_get.len(),
            first_get.into_iter().unique_by(|x| x.asn).count()
        );
        assert_eq!(
            second_get.len(),
            second_get.into_iter().unique_by(|x| x.asn).count()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn prepare_database_multiple_times_does_not_break_database() {
        let tested_as = simple_as;

        let context = TestContext::new(TESTED_CONN_STR).await.unwrap();
        let asdb = Asdb::new(TESTED_CONN_STR, &context.db_name).await.unwrap();

        asdb.clear_database().await.unwrap();
        asdb.prepare_database().await.unwrap();
        asdb.prepare_database().await.unwrap();
        asdb.insert_as(&tested_as()).await.unwrap();
        let (before_prepare, _) = asdb.get_ases(0, 0).await.unwrap();
        asdb.prepare_database().await.unwrap();
        let (after_prepare, _) = asdb.get_ases(0, 0).await.unwrap();
        assert_eq!(before_prepare.len(), after_prepare.len());
        assert_eq!(
            after_prepare.len(),
            after_prepare.into_iter().unique_by(|x| x.asn).count()
        );
    }

    fn simple_as() -> As {
        As {
            asn: 5551,
            asrank_data: None,
            ipnetdb_data: None,
            whois_data: None,
        }
    }

    fn simple_vec_as() -> Vec<As> {
        vec![
            As {
                asn: 5551,
                asrank_data: None,
                ipnetdb_data: None,
                whois_data: None,
            },
            As {
                asn: 5552,
                asrank_data: None,
                ipnetdb_data: None,
                whois_data: None,
            },
            As {
                asn: 5553,
                asrank_data: None,
                ipnetdb_data: None,
                whois_data: None,
            },
        ]
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
            name: String::from("Test Name"),
        };
        As {
            asn: 5550,
            asrank_data: Some(asrank),
            ipnetdb_data: None,
            whois_data: None,
        }
    }
}
