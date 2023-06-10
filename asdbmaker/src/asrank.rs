//! methods for executing and parsing asrank data
mod error;

use asdb::Asdb;
use asdb_models::{As, AsrankAsn, AsrankDegree, Coord};
pub use error::{Error, Result};

use isocountry::CountryCode;
use serde_json::Value;
use std::{fs::File, path::Path};

const API_URL: &str = "https://api.asrank.caida.org/v2/graphql";

pub async fn import_asns(asns: &impl AsRef<Path>, asdb: &Asdb) -> Result<()> {
    // TODO Result<u64> with n inserted
    let f = File::open(asns)?;
    // let json: Vec<Value> = serde_json::from_reader(f)?;
    // TODO catch unwind form  this deserializer and return jsonl error
    let json: Vec<As> = serde_json::Deserializer::from_reader(f)
        .into_iter::<Value>()
        .map(|x| {
            let line = x.unwrap();
            println!("line: {line:#?}");
            As {
                asn: line["asn"].as_str().unwrap().parse::<u32>().unwrap(),
                asrank_data: Some(AsrankAsn {
                    rank: line["rank"].as_u64().unwrap(),
                    organization: line["organization"]["orgName"]
                        .as_str()
                        .map_or(None, |x| Some(x.to_string())),
                    country_iso: line["country"]["iso"].as_str().unwrap().to_string(),
                    country_name: line["country"]["name"].as_str().unwrap().to_string(),
                    coordinates: Coord {
                        lat: line["longitude"].as_f64().unwrap(),
                        lon: line["latitude"].as_f64().unwrap(),
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
                }),
                ipnetdb_data: None,
                whois_data: None,
            }
        })
        .collect();
    println!("value is {json:#?}");
    asdb.insert_ases(&json).await?;
    // TODO return number inserted
    Ok(())
}

pub async fn import_orgs() {
    todo!()
}

pub async fn import_asnlinks() {
    todo!()
}

pub async fn download_asns() {
    todo!("Reimplement graphQL requests based on caida provided asrank-download.py")
}

pub async fn download_orgs() {
    todo!()
}

pub async fn download_asnlinks() {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    // const ASNS_PATH: &str = "asrank/asns.jsonl";
    async fn import_asns() {
        // let path = PathBuf::from(ASNS_PATH);
        // import_asns(&path, "").await.unwrap();
        //check DB
        // where to store db connection/db pool?
    }
}
