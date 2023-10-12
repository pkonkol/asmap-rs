//! methods for executing and parsing asrank data
mod error;

use asdb::Asdb;
use asdb_models::{As, AsrankAsn, AsrankDegree, Coord};
pub use error::{Error, Result};

use serde_json::Value;
use std::{fs::File, path::Path};

//const API_URL: &str = "https:///api.asrank.caida.org/v2/graphql";

/// Imports asns from "asns.jsonl" file which can be obtained from asrank API using
/// asrank-download.py from https://api.asrank.caida.org/dev/docs
pub async fn import_asns(file: &impl AsRef<Path>, asdb: &Asdb) -> Result<()> {
    let f = File::open(file)?;

    // TODO catch unwind form  this deserializer and return jsonl error
    // why the catch unwind even? don't remember
    let json: Vec<As> = serde_json::Deserializer::from_reader(f)
        .into_iter::<Value>()
        .map(|x| {
            let line = x.unwrap();
            As {
                asn: line["asn"].as_str().unwrap().parse::<u32>().unwrap(),
                asrank_data: Some(AsrankAsn {
                    rank: line["rank"].as_u64().unwrap(),
                    organization: line["organization"]["orgName"]
                        .as_str()
                        .map(|x| x.to_string()),
                    country_iso: line["country"]["iso"].as_str().unwrap().to_string(),
                    country_name: line["country"]["name"].as_str().unwrap().to_string(),
                    coordinates: Coord {
                        lat: line["latitude"].as_f64().unwrap(),
                        lon: line["longitude"].as_f64().unwrap(),
                    },
                    degree: AsrankDegree {
                        provider: line["asnDegree"]["provider"].as_u64().unwrap() as u32,
                        peer: line["asnDegree"]["peer"].as_u64().unwrap() as u32,
                        customer: line["asnDegree"]["customer"].as_u64().unwrap() as u32,
                        total: line["asnDegree"]["total"].as_u64().unwrap() as u32,
                        transit: line["asnDegree"]["transit"].as_u64().unwrap() as u32,
                        sibling: line["asnDegree"]["sibling"].as_u64().unwrap() as u32,
                    },
                    prefixes: line["announcing"]["numberPrefixes"].as_u64().unwrap(),
                    addresses: line["announcing"]["numberAddresses"].as_u64().unwrap(),
                    name: line["asnName"].as_str().unwrap().to_string(),
                }),
                ipnetdb_data: None,
                whois_data: None,
            }
        })
        .collect();

    let insert_result = asdb.insert_ases(&json).await;
    if let Err(e) = insert_result {
        match e {
            asdb::Error::DuplicatesFound(c) => {
                println!("Inserted asns but skipped {c} duplicated entries");
                return Ok(());
            }
            _ => {
                Err(e)?;
            }
        }
    }
    // TODO Result<u64> with n inserted
    Ok(())
}

pub async fn import_orgs() {
    todo!()
}

pub async fn import_asnlinks() {
    todo!()
}

/// download the asns from https://api.asrank.caida.org
/// TODO https://lib.rs/crates/graphql_client use this this to reimplement asrank-download.py
pub async fn download_asns() {
    todo!()
}

pub async fn download_orgs() {
    todo!()
}

pub async fn download_asnlinks() {
    todo!()
}