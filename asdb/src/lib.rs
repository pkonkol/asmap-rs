mod error;
pub mod models;

pub use error::{Error, Result};
use models::Asn;

use mongodb::{bson::doc, options::ClientOptions, Client};

enum Collection {
    ASNS,
    ORGANIZATIONS,
    PREFIXES,
    PRESONS,
}

struct Asdb {
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

    async fn get_collection_handle<T>(&self, c: Collection) -> Box<mongodb::Collection<T>> {
        //let admins_collection = c.database(&database).collection::<Admin>("admins");
        //self.mongo.database(&self.database).collection(c)

        todo!()
    }

    async fn ping(&self) -> Result<()> {
        self.client
            .database(&self.database)
            .run_command(doc! {"ping": 1}, None)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TESTED_DB: &str = "asdb";
    const TESTED_CONN_STR: &str = "mongodb://devuser:devpass@localhost:27017/?authSource=asdb";

    #[tokio::test]
    async fn asdb_initializes() {
        let asdb = Asdb::new(TESTED_CONN_STR, TESTED_DB).await.unwrap();
        asdb.ping().await.unwrap();
    }
}
