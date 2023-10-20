//! generates static datastructures for stanford asdb categories based on `NAICSlite.csv` file
//!

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
use trauma::{download::Download, downloader::DownloaderBuilder};

use super::Result;

const NAICSLITE: &str = "https://asdb.stanford.edu/data/NAICSlite.csv";
const NAICSLITE_FILENAME: &str = "NAICSlite.csv";
const OUT_DIR: &str = "inputs";

pub async fn generate() {
    download(&OUT_DIR).await.unwrap();
    let mut rdr = csv::Reader::from_path(Path::new(
        &[OUT_DIR, NAICSLITE_FILENAME].iter().collect::<PathBuf>(),
    ))
    .unwrap();
    print!("pub const CATEGORIES: &[(&str, &[&str])] = &[\n");
    for (i, r) in rdr.records().enumerate() {
        let r = r.unwrap();
        let (category, layer) = (r.get(0).unwrap(), r.get(1).unwrap());

        if layer.trim().starts_with("1") {
            if i > 0 {
                print!("]),\n");
            };
            print!("    (\"{category}\", &[");
        } else {
            print!("\"{category}\",");
        }
    }
    print!("]), ");
    print!("\n];\n");

    println!("The preceeding code is supposed to be pasted into asdb-models/src/categories.rs\nin case of update to NAICSlite.csv")
}

async fn download<T: AsRef<Path> + AsRef<OsStr>>(dest: &T) -> Result<()> {
    println!("donwloading stanford asdb categories csv from {NAICSLITE}");
    let d = Download::new(&reqwest::Url::parse(NAICSLITE).unwrap(), NAICSLITE_FILENAME);

    let downloader = DownloaderBuilder::new()
        .directory(PathBuf::from(dest))
        .build();
    downloader.download(&[d]).await;
    Ok(())
}
