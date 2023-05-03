mod asrank;
mod error;
mod ipnetdb;
mod whois;

use asdb::models::{As, AsrankAsn};
use error::Result;

pub async fn import_asrank_asns() -> Result<()> {
    todo!()
}

pub async fn import_ipnetdb_prefixes() -> Result<()> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_download_smth() {
        assert!(true);
    }
}
