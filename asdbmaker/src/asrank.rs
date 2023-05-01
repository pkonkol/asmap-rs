//! methods for executing and parsing asrank data

use std::{fmt::Display, path::Path, process::Command, string::FromUtf8Error};

const API_URL: &str = "https://api.asrank.caida.org/v2/graphql";

#[derive(Debug)]
pub enum Error {
    RequestError,
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "whois error {self:?}")
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

pub async fn download_asns() {
    todo!("Reimplement graphQL requests based on caida provided asrank-download.py")
}

pub async fn download_orgs() {
    todo!()
}

pub async fn download_asnlinks() {
    todo!()
}

pub async fn import_asns(asns: &impl AsRef<Path>, db: &str) -> Result<()> {
    // open asns file
    // open mongo connection
    // insert chosen fields into mongo
    todo!()
}

pub async fn import_orgs() {
    todo!()
}

pub async fn import_asnlinks() {
    todo!()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    const ASNS_PATH: &str = "asrank-output/asns.jsonl";

    #[tokio::test(flavor = "multi_thread")]
    async fn test_import_asns() {
        let path = PathBuf::from(ASNS_PATH);
        import_asns(&path, "").await.unwrap();
        //check DB
        // where to store db connection/db pool?
    }
}
