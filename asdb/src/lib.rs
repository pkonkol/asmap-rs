use futures::stream::TryStreamExt;
use itertools::Itertools;
use mongodb::{
    bson::{doc, Document},
    options::{ClientOptions, FindOptions, IndexOptions, InsertManyOptions, UpdateOptions},
    Client, IndexModel,
};

use asdb_models::{
    As, AsFilters, AsForFrontend, AsForFrontendFromDB, IPNetDBAsn, StanfordASdbCategory,
};
pub use error::{Error, Result};
use tracing::debug;

mod error;

pub struct Asdb {
    client: Client,
    database: String,
}

impl std::fmt::Debug for Asdb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Asdb {{ database: {} }}", self.database)
    }
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

    async fn _ping(&self) -> Result<()> {
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

    #[tracing::instrument]
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
    #[tracing::instrument]
    pub async fn get_ases_page(&self, limit: i64, skip: u64) -> Result<(Vec<AsForFrontend>, u64)> {
        let collection = self
            .client
            .database(&self.database)
            .collection::<AsForFrontendFromDB>("asns");
        let opts = FindOptions::builder()
            .skip(skip)
            .limit(limit)
            .projection(doc! {
                "asn": 1,
                "asrank_data.name": 1,
                "asrank_data.rank": 1,
                "asrank_data.country_iso": 1,
                "asrank_data.prefixes": 1,
                "asrank_data.addresses": 1,
                "asrank_data.coordinates": 1,
                "asrank_data.organization": 1,
            })
            .sort(doc! { "asn": 1 })
            .build();
        let res = collection.find(doc! {}, opts).await?;
        let count = collection.count_documents(doc! {}, None).await?;
        let ases: Vec<AsForFrontend> = res.map_ok(AsForFrontend::from).try_collect().await?;

        Ok((ases, count))
    }

    #[tracing::instrument]
    pub async fn get_ases(&self, asns: &[u32]) -> Result<(Vec<As>, u64)> {
        let collection = self
            .client
            .database(&self.database)
            .collection::<As>("asns");
        // let opts = FindOptions::builder().build();

        let res = collection
            .find(doc! {"asn": doc! { "$in": asns}}, None)
            .await?;
        let count = collection.count_documents(doc! {}, None).await?;
        let ases: Vec<As> = res.try_collect().await?;
        Ok((ases, count))
    }

    fn create_db_filter(filters: &AsFilters) -> Document {
        let mut db_filter = doc! {};
        if let Some(bounds) = &filters.bounds {
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
            db_filter.insert(
                "asrank_data.country_iso",
                if filters.exclude_country {
                    doc! { "$ne": x }
                } else {
                    doc! { "$eq": x }
                },
            );
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
        if !filters.category.iter().contains(&"Any".to_string()) && !filters.category.is_empty() {
            db_filter.insert(
                "stanford_asdb.layer1",
                doc! { "$all": filters.category.as_slice() },
            );
        }
        db_filter
    }

    pub async fn count_ases_filtered(&self, filters: &AsFilters) -> Result<u64> {
        let collection = self
            .client
            .database(&self.database)
            .collection::<As>("asns");
        let db_filter = Self::create_db_filter(filters);

        let res = collection.count_documents(db_filter, None).await?;
        Ok(res)
    }

    #[tracing::instrument]
    pub async fn get_ases_filtered(&self, filters: &AsFilters) -> Result<Vec<AsForFrontend>> {
        let collection = self
            .client
            .database(&self.database)
            .collection::<AsForFrontendFromDB>("asns");
        let db_filter = Self::create_db_filter(filters);

        // let opts = FindOptions::builder().sort(doc! { "asn": 1 }).build();
        let opts = FindOptions::builder()
            .projection(doc! {
                "asn": 1,
                "asrank_data.name": 1,
                "asrank_data.rank": 1,
                "asrank_data.country_iso": 1,
                "asrank_data.prefixes": 1,
                "asrank_data.addresses": 1,
                "asrank_data.coordinates": 1,
                "asrank_data.organization": 1,
            })
            .build();
        // TODO add projection ^ and verify if it speeds up the retrieval
        let res = collection.find(db_filter, opts).await?;
        debug!("cursor retrieved, starting collect");
        // let ases: Vec<AsForFrontendFromDB> = res.try_collect().await?;
        // res.for_each(|x| {x.unwrap(); future::ready(())}).await;
        let ases: Vec<AsForFrontend> = res.map_ok(AsForFrontend::from).try_collect().await?;
        debug!("collected entries into Vec<>");
        Ok(ases)
    }

    #[tracing::instrument]
    pub async fn get_as(&self, asn: u32) -> Result<As> {
        let collection = self
            .client
            .database(&self.database)
            .collection::<As>("asns");
        let res = collection.find_one(doc! {"asn": asn }, None).await?;
        res.ok_or(Error::AsNotFound)
    }

    #[tracing::instrument]
    pub async fn insert_as(&self, a: &As) -> Result<()> {
        let collection = self
            .client
            .database(&self.database)
            .collection::<As>("asns");
        collection.insert_one(a, None).await?;
        Ok(())
    }

    #[tracing::instrument]
    pub async fn insert_ases(&self, a: &[As]) -> Result<()> {
        let collection = self
            .client
            .database(&self.database)
            .collection::<As>("asns");
        let opts = InsertManyOptions::builder().ordered(false).build();
        collection.insert_many(a, opts).await?;
        Ok(())
    }

    /// Updates the record for given asn with the provided IPNetDB data
    #[tracing::instrument]
    pub async fn insert_ipnetdb_asn(&self, asn: u32, a: &IPNetDBAsn) -> Result<()> {
        let collection = self
            .client
            .database(&self.database)
            .collection::<As>("asns");
        let opts = UpdateOptions::builder().build();
        let update = doc! {
            "$set": {
                "ipnetdb_data": mongodb::bson::to_bson(a).expect("IPNetDBAsn should always be serializable to bson")
            }
        };
        collection
            .update_one(doc! {"asn": asn}, update, opts)
            .await?;
        Ok(())
    }

    /// Updates the record for given asn with the provided categories list from stanford asdb
    #[tracing::instrument]
    pub async fn insert_stanford_asdb_categories(
        &self,
        asn: u32,
        categories: &[StanfordASdbCategory],
    ) -> Result<()> {
        let collection = self
            .client
            .database(&self.database)
            .collection::<As>("asns");
        let opts = UpdateOptions::builder().build();
        let update = doc! {
            "$set": {
                "stanford_asdb": mongodb::bson::to_bson(categories).expect("StandordAsdbCategory should always be serializable to bson")
            }
        };
        collection
            .update_one(doc! {"asn": asn}, update, opts)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use asdb_models::{
        AsrankAsn, AsrankDegree, Coord, IPNetDBIX, IPNetDBPrefix, IPNetDBPrefixDetails,
        InternetRegistry,
    };
    use ipnetwork::IpNetwork;
    use itertools::Itertools;
    use test_context::TestContext;

    use super::*;

    const TESTED_CONN_STR: &str = "mongodb://root:devrootpass@localhost:27017";

    #[tokio::test(flavor = "multi_thread")]
    async fn asdb_initializes() {
        let context = TestContext::new(TESTED_CONN_STR).await.unwrap();
        let asdb = Asdb::new(TESTED_CONN_STR, &context.db_name).await.unwrap();

        asdb._ping().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_asns_executes() {
        let context = TestContext::new(TESTED_CONN_STR).await.unwrap();
        let asdb = Asdb::new(TESTED_CONN_STR, &context.db_name).await.unwrap();

        asdb.get_ases_page(0, 0).await.unwrap();
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
        let (asns, _count) = asdb.get_ases_page(0, 0).await.unwrap();
        assert!(asns.iter().any(|x| x.asn == tested_asn));
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
        let (asns, _count) = asdb.get_ases_page(0, 0).await.unwrap();
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
        let first_get = asdb.get_ases_page(0, 0).await.unwrap();
        let second_insert = asdb.insert_as(&tested_as()).await;
        let second_get = asdb.get_ases_page(0, 0).await.unwrap();

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

        let (first_get, _) = asdb.get_ases_page(0, 0).await.unwrap();

        let second_insert = asdb.insert_ases(&tested_ases()).await;
        let (second_get, _) = asdb.get_ases_page(0, 0).await.unwrap();

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
        let (before_prepare, _) = asdb.get_ases_page(0, 0).await.unwrap();
        asdb.prepare_database().await.unwrap();
        let (after_prepare, _) = asdb.get_ases_page(0, 0).await.unwrap();
        assert_eq!(before_prepare.len(), after_prepare.len());
        assert_eq!(
            after_prepare.len(),
            after_prepare.into_iter().unique_by(|x| x.asn).count()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn insert_then_get_ipnetdb_as() {
        let tested_ipnetdb_as = ipnetdb_as;
        let tested_as = as_with_asrank;
        assert!(tested_as().ipnetdb_data.is_none());
        let tested_asn = tested_as().asn;

        let context = TestContext::new(TESTED_CONN_STR).await.unwrap();
        let asdb = Asdb::new(TESTED_CONN_STR, &context.db_name).await.unwrap();
        // first insert normal As so it can be later updated with ipnetdb data
        asdb.insert_as(&tested_as()).await.unwrap();

        asdb.insert_ipnetdb_asn(tested_asn, &tested_ipnetdb_as())
            .await
            .unwrap();
        let as_ = asdb.get_as(tested_asn).await.unwrap();
        assert!(as_.ipnetdb_data.is_some());
        println!("{:#?}", as_);
        let retrieved_ipnetdb_data = as_.ipnetdb_data.unwrap();
        assert_eq!(retrieved_ipnetdb_data, tested_ipnetdb_as());
    }

    fn ipnetdb_as() -> IPNetDBAsn {
        // TODO fill these
        let ipv4_prefixes = vec![
            IPNetDBPrefix {
                range: IpNetwork::new(IpAddr::V4(Ipv4Addr::new(153, 19, 64, 251)), 16u8).unwrap(),
                details: Some(IPNetDBPrefixDetails {
                    allocation: IpNetwork::new(IpAddr::V4(Ipv4Addr::new(153, 19, 64, 251)), 12u8)
                        .ok(),
                    allocation_cc: Some("CC".to_string()),
                    allocation_registry: Some(InternetRegistry::AFRINIC),
                    prefix_entity: "prefix entity".to_string(),
                    prefix_name: "prefix name".to_string(),
                    prefix_origins: vec![1, 3241, 8888, 12345],
                    prefix_registry: "registro.br".to_string(), //Registry::APNIC,
                }),
            },
            IPNetDBPrefix {
                range: IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 10, 10, 1)), 24u8).unwrap(),
                details: None,
            },
        ];
        let ipv6_prefixes = vec![];
        let ix = vec![
            IPNetDBIX {
                exchange: "used exchange name?".to_string(),
                ipv4: Some([127, 0, 0, 1]), //Ipv4Addr::new(127, 0, 0, 1),
                ipv6: Some([1, 2, 3, 4, 5, 6, 7, 8, 8, 7, 6, 5, 4, 3, 2, 1]),
                name: Some("ixname".to_string()),
                speed: 10,
            },
            IPNetDBIX {
                exchange: "used exchange name?".to_string(),
                ipv4: Some([88, 23, 1, 99]), //Ipv4Addr::new(127, 0, 0, 1),
                ipv6: Some([2, 2, 3, 4, 5, 6, 7, 8, 8, 7, 6, 5, 4, 3, 2, 1]),
                name: Some("ixname".to_string()),
                speed: 10,
            },
        ];
        //let ix = vec![];
        IPNetDBAsn {
            cc: "PL".to_string(),
            entity: "ENTITY".to_string(),
            in_use: true,
            ipv4_prefixes,
            ipv6_prefixes,
            name: Some("name".to_string()),
            peers: vec![123, 3112, 99999],
            private: false,
            registry: InternetRegistry::RIPE,
            status: Some("status".to_string()),
            ix,
        }
    }

    fn simple_as() -> As {
        As {
            asn: 5551,
            asrank_data: None,
            ipnetdb_data: None,
            whois_data: None,
            ..Default::default()
        }
    }

    fn simple_vec_as() -> Vec<As> {
        vec![
            As {
                asn: 5551,
                asrank_data: None,
                ipnetdb_data: None,
                whois_data: None,
                ..Default::default()
            },
            As {
                asn: 5552,
                asrank_data: None,
                ipnetdb_data: None,
                whois_data: None,
                ..Default::default()
            },
            As {
                asn: 5553,
                asrank_data: None,
                ipnetdb_data: None,
                whois_data: None,
                ..Default::default()
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
            ..Default::default()
        }
    }
}
