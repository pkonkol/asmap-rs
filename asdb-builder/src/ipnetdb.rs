use asdb::Asdb;

use ipnetwork::IpNetwork;

use std::{ffi::OsStr, path::Path};
use trauma::{download::Download, downloader::DownloaderBuilder};

pub use error::{Error, Result};

mod error;
mod read_models;

const LATEST_PREFIX_MMDB: &str = "https://cdn.ipnetdb.net/ipnetdb_prefix_latest.mmdb";
const LATEST_ASN_MMDB: &str = "https://cdn.ipnetdb.net/ipnetdb_asn_latest.mmdb";

pub async fn load(asdb: &Asdb) -> Result<()> {
    download(&"inputs").await?;
    read_asns(
        &"inputs/ipnetdb_asn_latest.mmdb",
        &"inputs/ipnetdb_prefix_latest.mmdb",
        asdb,
    )
    .await
    .unwrap();
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

async fn read_asns(
    asn_mmdb: &impl AsRef<Path>,
    prefix_mmdb: &impl AsRef<Path>,
    asdb: &Asdb,
) -> Result<()> {
    println!("importing ipnetdb asns from mmdb file to the database");
    let every_ip = IpNetwork::V4("0.0.0.0/0".parse()?);
    let asn_reader = maxminddb::Reader::open_readfile(asn_mmdb)?;
    let prefix_reader = maxminddb::Reader::open_readfile(prefix_mmdb)?;

    let asn_iter = asn_reader.within(every_ip, Default::default())?;
    let total_asns = asn_reader.within(every_ip, Default::default())?.count() as u64;
    let bar = indicatif::ProgressBar::new(total_asns);

    for asn_lookup in asn_iter.flatten() {
        let Some(decoded) = asn_lookup.decode::<read_models::IPNetDBAsn>()? else {
            continue;
        };
        let Ok(mut asn_model): std::result::Result<asdb_models::IPNetDBAsn, _> =
            decoded.clone().try_into()
        else {
            continue;
        };

        for prefix in &mut asn_model.ipv4_prefixes {
            prefix.details = prefix_reader
                .lookup(prefix.range.network())
                .ok()
                .and_then(|l| l.decode::<read_models::IPNetDBPrefix>().ok())
                .flatten()
                .and_then(|p| asdb_models::IPNetDBPrefixDetails::try_from(p).ok());
        }

        asdb.insert_ipnetdb_asn(decoded.as_, &asn_model).await?;
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
