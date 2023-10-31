use asdb::Asdb;

use ipnetwork::IpNetwork;
use maxminddb::Within;

use std::{ffi::OsStr, path::Path};
use trauma::{download::Download, downloader::DownloaderBuilder};

pub use error::{Error, Result};

mod error;
mod read_models;

const LATEST_PREFIX_MMDB: &str = "https://cdn.ipnetdb.net/ipnetdb_prefix_latest.mmdb";
const LATEST_ASN_MMDB: &str = "https://cdn.ipnetdb.net/ipnetdb_asn_latest.mmdb";

pub async fn load(asdb: &Asdb) -> Result<()> {
    download(&"inputs").await?;
    // download files if not there
    read_asns(&"inputs/ipnetdb_asn_latest.mmdb", asdb)
        .await
        .unwrap();
    // dump them
    // load dumped into mongo
    Ok(())
}

async fn download<T: AsRef<Path> + AsRef<OsStr>>(dest: &T) -> Result<()> {
    println!("downloading ipnetdb databases from {LATEST_PREFIX_MMDB} and {LATEST_ASN_MMDB}");
    let downloads = vec![
        Download::try_from(LATEST_ASN_MMDB).unwrap(),
        Download::try_from(LATEST_PREFIX_MMDB).unwrap(),
    ];
    let downloader = DownloaderBuilder::new().directory(dest.into()).build();
    downloader.download(&downloads).await;
    Ok(())
}

async fn read_asns(mmdb: &impl AsRef<Path>, asdb: &Asdb) -> Result<()> {
    println!("importing ipnetdb asns from mmdb file to the database");
    let reader = maxminddb::Reader::open_readfile(mmdb)?;
    let every_ip = IpNetwork::V4("0.0.0.0/0".parse().unwrap());
    let iter: Within<read_models::IPNetDBAsn, _> = reader.within(every_ip).unwrap();
    let prefix_reader = maxminddb::Reader::open_readfile("inputs/ipnetdb_prefix_latest.mmdb")?;

    let bar = indicatif::ProgressBar::new(reader.within::<()>(every_ip).unwrap().count() as u64);
    for next in iter {
        let item = next.unwrap();
        let asn = item.info.as_;
        let asn_model: std::result::Result<asdb_models::IPNetDBAsn, _> =
            item.info.clone().try_into();
        if asn_model.is_err() {
            println!("\ncouldn't parse asn read model into db model from read model {:#?} \nwith err {asn_model:#?}", item.info);
            return Err(Error::RequestError);
        }
        let mut asn_model = asn_model.expect("checked for err before");
        for i in asn_model.ipv4_prefixes.iter_mut() {
            let prefix_model =
                prefix_reader.lookup::<read_models::IPNetDBPrefix>(i.range.network());
            if prefix_model.is_err() {
                let serde_raw_prefix = prefix_reader.lookup::<serde_json::Value>(i.range.network());
                println!("\nraw serde value: {:#?}", serde_raw_prefix);
                println!("parsed value {:#?}", prefix_model);
                println!("omitting prefix details {:-<100}", "x");
                continue;
            }
            let details = asdb_models::IPNetDBPrefixDetails::try_from(prefix_model.unwrap());
            if details.is_err() {
                let serde_raw_prefix = prefix_reader.lookup::<serde_json::Value>(i.range.network());
                println!("couldn't cast read prefix model {serde_raw_prefix:#?} with error : {details:?}");
                continue;
            }
            i.details = Some(details.unwrap());
        }
        asdb.insert_ipnetdb_asn(asn, &asn_model).await.unwrap();
        bar.inc(1);
    }
    bar.finish();
    Ok(())
}

#[cfg(test)]
mod tests {

    // use std::net::Ipv4Addr;
    // use std::path::PathBuf;

    // use super::*;
    // const ASNS_PATH: &str = "../inputs/ipnetdb_asn_latest.mmdb";
    // const PREFIX_PATH: &str = "../inputs/ipnetdb_prefix_latest.mmdb";
    // const IP: IpAddr = IpAddr::V4(Ipv4Addr::new(153, 19, 64, 251));
    // TODO some up to date tests
}
