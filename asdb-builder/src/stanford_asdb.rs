use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
use trauma::{download::Download, downloader::DownloaderBuilder};

use asdb::Asdb;
pub use error::{Error, Result};

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
    //let downloads = vec![Download::try_from(LATEST_ASDB_CSV).unwrap()];
    let d = Download::new(
        &reqwest::Url::parse(LATEST_ASDB_CSV).unwrap(),
        ASDB_DST_FILENAME,
    );
    //let mut dest_file = PathBuf::from(dest);

    let downloader = DownloaderBuilder::new()
        .directory(PathBuf::from(dest))
        .build();
    downloader.download(&[d]).await;
    Ok(())
}

async fn write_to_db(asdb: &Asdb, csv: &impl AsRef<Path>) -> Result<()> {
    use itertools::Itertools;
    let mut rdr = csv::ReaderBuilder::new().flexible(true).from_path(csv)?;
    //let h = rdr.headers();
    //println!("{h:#?}");
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
        //println!("done as {asn}\ncategories are:{categories:?}");
        print!(".");
        asdb.insert_stanford_asdb_categories(asn, categories.as_slice())
            .await
            .unwrap();
    }
    Ok(())
}
