use mongodb::{options::ClientOptions, Client};
use rand::distributions::{Alphanumeric, DistString};
use thiserror::Error;

/// Returns a guard handle to the database with randomly created alphanumeric string as a name
/// The database is dropped when the handle is dropped
/// It only works in multi threaded runtimes, tests must use #[tokio::test(flavor = "multi_thread")]
///
/// # Arguments
///
/// * `root_conn_str` - mongo connection string with root permissions
/// # Examples
///
/// ```ignore
/// let context = TestContext::new(ASDB_CONN_STR).await.unwrap();
///
/// let client_options = ClientOptions::parse(ASDB_CONN_STR).await.unwrap();
/// let client = Client::with_options(client_options).unwrap();
/// client.database(&context.db_name).create_collection("test", None).await.unwrap();
/// ```

pub struct TestContext {
    pub db_name: String,
    root_conn_str: String,
}

impl TestContext {
    pub async fn new(root_conn_str: &str) -> Result<Self, TestContextError> {
        Ok(Self {
            db_name: Self::create_random_database(root_conn_str).await?,
            root_conn_str: root_conn_str.to_string(),
        })
    }

    async fn create_random_database(conn_str: &str) -> Result<String, TestContextError> {
        let new_database = Alphanumeric.sample_string(&mut rand::thread_rng(), 63);

        let client_options = ClientOptions::parse(conn_str).await?;
        let client = Client::with_options(client_options)?;

        let db = client.database(&new_database);

        db.list_collection_names(None).await?;

        Ok(new_database)
    }

    async fn drop_database(name: &str, conn_str: &str) {
        let client_options = ClientOptions::parse(conn_str).await.unwrap();
        let client = Client::with_options(client_options).unwrap();

        let db = client.database(name);
        let t = db.drop(None).await;

        t.unwrap();
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        let h = match tokio::runtime::Handle::try_current() {
            Ok(h) => h,
            Err(_) => tokio::runtime::Runtime::new().unwrap().handle().clone(),
        };
        let db_name = self.db_name.clone();
        let conn_str = self.root_conn_str.clone();
        tokio::task::block_in_place(move || {
            h.block_on(Self::drop_database(&db_name, &conn_str));
        });
    }
}

#[derive(Error, Debug)]
pub enum TestContextError {
    #[error("couldn't create the database")]
    CreateDatabase(#[from] mongodb::error::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    const ASDB_CONN_STR: &str = "mongodb://root:devrootpass@localhost:27017";
    const ASDB_DB: &str = "asmap";

    #[tokio::test(flavor = "multi_thread")]
    async fn it_works() {
        let context = TestContext::new(ASDB_CONN_STR).await.unwrap();
        let context_db_name = context.db_name.clone();

        let client_options = ClientOptions::parse(ASDB_CONN_STR).await.unwrap();
        let client = Client::with_options(client_options).unwrap();

        client
            .database(&context.db_name)
            .create_collection("test", None)
            .await
            .unwrap();

        let names = client.list_database_names(None, None).await.unwrap();
        assert!(names.contains(&context_db_name));
        println!("{names:?}");

        drop(context);

        let names = client.list_database_names(None, None).await.unwrap();
        println!("{names:?}");
        assert!(!names.contains(&context_db_name));
    }
}
