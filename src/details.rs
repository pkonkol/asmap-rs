use serde_json::Serializer;
use std::{
    fmt::write,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use asdb_models::As;

// #[thiserror::Error]
// struct Error { }

pub fn generate_json(ases: &[As], path: &impl AsRef<Path>) {
    println!("generating detailed json");
    // let ases_json = serde_json::to_string(ases).unwrap();
    let file = File::create(path).unwrap();
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, ases).unwrap();
    writer.flush().unwrap();
    //write!(f, &ases_json);
}

/// returns vec of asns contained in input csv
pub fn parse_input_csv(path: &impl AsRef<Path>) -> Vec<u32> {
    println!("parse input csv");
    let mut out = vec![];
    // let bar = indicatif::ProgressBar::new(BufReader::new(File::open(csv)?).lines().count() as u64);
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(path)
        .unwrap();
    for result in rdr.records() {
        let l = result.unwrap();
        println!("read line: {l:?}");
        l.len();
        out.push(l.get(0).unwrap().parse().unwrap());
    }
    out
}

// AsForDetailsJson model?
