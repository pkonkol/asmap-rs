use ipnetwork::IpNetwork;
use maxminddb::Within;
use mongodb::bson::de;
use serde_json::Value;
use trauma::{download::Download, downloader::DownloaderBuilder};
use std::{fmt::Display, net::IpAddr, path::{Path, PathBuf}, ffi::OsStr};

pub use error::{Result, Error};

mod error;

const LATEST_PREFIX_MMDB: &str = "https://cdn.ipnetdb.net/ipnetdb_prefix_latest.mmdb";
const LATEST_ASN_MMDB: &str = "https://cdn.ipnetdb.net/ipnetdb_asn_latest.mmdb";


pub async fn load() -> Result<()> {
    download(&"inputs").await?;
    // download files if not there
    // dump them
    // load dumped into mongo
    Ok(())
}

async fn dump_mmdb(db: &impl AsRef<Path>) -> Result<Vec<(IpNetwork, Value)>> {
    let reader = maxminddb::Reader::open_readfile(db)?;
    let ip_net = IpNetwork::V4("0.0.0.0/0".parse().unwrap());
    let iter: Within<serde_json::Value, _> = reader.within(ip_net).unwrap();

    let mut out = Vec::new();
    for next in iter {
        let item = next.unwrap();
        out.push((item.ip_net, item.info));
    }
    Ok(out)
}

async fn get_prefixes_string(db: &impl AsRef<Path>) -> Result<String> {
    let vec = dump_mmdb(db).await?;
    let mut out = String::new();
    for (_, v) in vec.iter() {
        out.push_str(&format!("{v}\n"));
    }
    Ok(out)
}

async fn get_asns(db: &impl AsRef<Path>) -> Result<String> {
    let vec = dump_mmdb(db).await?;
    let mut out = String::new();
    for (_, v) in vec.iter() {
        out.push_str(&format!("{v}\n"));
    }
    Ok(out)
}

async fn get_ip_details(ip: IpAddr, db: &impl AsRef<Path>) -> Result<String> {
    let reader = maxminddb::Reader::open_readfile(db)?;
    let data: Value = reader.lookup(ip).unwrap();

    Ok(data.to_string())
}

async fn download<T: AsRef<Path> + AsRef<OsStr>>(dest: &T) -> Result<()> {
    let downloads = vec![Download::try_from(LATEST_ASN_MMDB).unwrap(),
    Download::try_from(LATEST_PREFIX_MMDB).unwrap(),];
    let downloader = DownloaderBuilder::new()
        .directory(dest.into())
        .build();
    downloader.download(&downloads).await;
    Ok(())
}

#[cfg(test)]
mod tests {

    use std::net::Ipv4Addr;
    use std::path::PathBuf;

    use super::*;
    const ASNS_PATH: &str = "inputs/test-data/ipnetdb_asn_latest.mmdb";
    const PREFIX_PATH: &str = "inputs/test-data/ipnetdb_prefix_latest.mmdb";
    const IP: IpAddr = IpAddr::V4(Ipv4Addr::new(153, 19, 64, 251));

    #[tokio::test(flavor = "multi_thread")]
    async fn test_prefixes_string() {
        let path = PathBuf::from(PREFIX_PATH);
        let str = get_prefixes_string(&path).await.unwrap();

        assert!(str.lines().count() > 1000000)
        //let mut f = File::create("ipnetdb-dump/prefixes.jsonl").unwrap();
        //f.write_all(str.as_bytes()).unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_asn_string() {
        let path = PathBuf::from(ASNS_PATH);
        let str = get_asns(&path).await.unwrap();

        assert!(str.lines().count() > 100000)
        //let mut f = File::create("ipnetdb-dump/asns.jsonl").unwrap();
        //f.write_all(str.as_bytes()).unwrap();
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_ip_details() {
        let path = PathBuf::from(PREFIX_PATH);
        let str = get_ip_details(IP, &path).await.unwrap();
        println!("{str}");
        // let mut f = File::create("ipnetdb-dump/asns.jsonl").unwrap();
        // f.write_all(str.as_bytes()).unwrap();
    }
}
