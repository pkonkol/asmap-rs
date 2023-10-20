use itertools::Itertools;
use std::{
    ffi::OsStr,
    fs::File,
    io::{prelude::*, BufReader},
    path::{Path, PathBuf},
};
use trauma::{download::Download, downloader::DownloaderBuilder};

use asdb::Asdb;
pub use error::{Error, Result};

pub mod categories;
mod error;

const LATEST_ASDB_CSV: &str = "https://asdb.stanford.edu/data/2023-05_categorized_ases.csv";
const ASDB_DST_FILENAME: &str = "stanford-asdb.csv";

pub async fn load(asdb: &Asdb) -> Result<()> {
    download(&"inputs").await?;
    write_to_db(
        asdb,
        &["inputs", ASDB_DST_FILENAME].iter().collect::<PathBuf>(),
    )
    .await?;
    Ok(())
}

/// Takes in path to a directory where the file will be saved
async fn download<T: AsRef<Path> + AsRef<OsStr>>(dest: &T) -> Result<()> {
    println!("donwloading stanford asdb csv from {LATEST_ASDB_CSV}");
    let d = Download::new(
        &reqwest::Url::parse(LATEST_ASDB_CSV).unwrap(),
        ASDB_DST_FILENAME,
    );

    let downloader = DownloaderBuilder::new()
        .directory(PathBuf::from(dest))
        .build();
    downloader.download(&[d]).await;
    Ok(())
}

async fn write_to_db(asdb: &Asdb, csv: &impl AsRef<Path>) -> Result<()> {
    println!("Writing stanford asdb categories to the database");
    let bar = indicatif::ProgressBar::new(BufReader::new(File::open(csv)?).lines().count() as u64);
    let mut rdr = csv::ReaderBuilder::new().flexible(true).from_path(csv)?;
    for result in rdr.records() {
        let record = result?;
        let asn = record.get(0).unwrap();
        let asn = asn
            .trim()
            .strip_prefix("AS")
            .unwrap()
            .parse::<u32>()
            .unwrap();
        let mut categories = vec![];
        for mut chunk in &record.into_iter().skip(1).chunks(2) {
            let (layer1, layer2) = (chunk.next(), chunk.next());
            if layer1.is_none() || layer2.is_none() {
                println!("uneven amount of layers, l1: {layer1:?} and l2: {layer2:?}");
                continue;
            }
            let c = asdb_models::StanfordASdbCategory {
                layer1: layer1.unwrap().to_string(),
                layer2: layer2.unwrap().to_string(),
            };
            categories.push(c);
        }
        asdb.insert_stanford_asdb_categories(asn, categories.as_slice())
            .await
            .unwrap();
        bar.inc(1);
    }
    bar.finish();
    Ok(())
}
