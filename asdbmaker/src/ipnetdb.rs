use ipnetwork::IpNetwork;
use maxminddb::{geoip2, Within};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, fmt::Display, net::IpAddr, path::Path};

const IPNETDB_LATEST: &str = "https://ipnetdb.com/latest.json";

#[derive(Debug)]
pub enum Error {
    RequestError,
    DbReadError(String),
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "whois error {self:?}")
    }
}

impl std::error::Error for Error {}

impl From<maxminddb::MaxMindDBError> for Error {
    fn from(value: maxminddb::MaxMindDBError) -> Self {
        println!("{value}");
        Error::DbReadError(value.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

async fn dump_mmdb(db: &impl AsRef<Path>) -> Result<Vec<(IpNetwork, Value)>> {
    let reader = maxminddb::Reader::open_readfile(db)?;
    let ip_net = IpNetwork::V4("0.0.0.0/0".parse().unwrap());
    let mut iter: Within<serde_json::Value, _> = reader.within(ip_net).unwrap();

    let mut out = Vec::new();
    while let Some(next) = iter.next() {
        let item = next.unwrap();
        out.push((item.ip_net, item.info));
    }
    Ok(out)
}

pub async fn get_prefixes_string(db: &impl AsRef<Path>) -> Result<String> {
    let vec = dump_mmdb(db).await?;
    let mut out = String::new();
    for (_, v) in vec.iter() {
        out.push_str(&format!("{v}\n"));
    }
    Ok(out)
}

pub async fn get_asns(db: &impl AsRef<Path>) -> Result<String> {
    let vec = dump_mmdb(db).await?;
    let mut out = String::new();
    for (_, v) in vec.iter() {
        out.push_str(&format!("{v}\n"));
    }
    Ok(out)
}

pub async fn get_ip_details(ip: IpAddr, db: &impl AsRef<Path>) -> Result<String> {
    let reader = maxminddb::Reader::open_readfile(db)?;
    let data: Value = reader.lookup(ip).unwrap();

    Ok(data.to_string())
}

pub async fn get_latest_ipnetdb() {
    // reqwest.get(IPNETDB_LATEST)
    // res["prefix"](["url"], ["file"], ["sha256"])
    // the same for ["asn"]
    todo!()
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;
    use std::net::Ipv4Addr;
    use std::path::PathBuf;

    use super::*;
    const ASNS_PATH: &str = "external/ipnetdb_asn_latest.mmdb";
    const PREFIX_PATH: &str = "external/ipnetdb_prefix_latest.mmdb";
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
