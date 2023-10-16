//! methods for executing and parsing asrank data
mod error;
mod graphql;

use asdb::Asdb;
use asdb_models::{As, AsrankAsn, AsrankDegree, Coord};
pub use error::{Error, Result};

use indicatif::ProgressIterator;
use serde_json::Value;
use std::io::prelude::*;
use std::io::BufReader;
use std::{fs::File, path::Path};

use crate::asrank::graphql::asns_query;

const API_URL: &str = "https:///api.asrank.caida.org/v2/graphql";
const PAGE_SIZE: i64 = 10000;

/// load asns either from
pub async fn load(asdb: &Asdb, file: Option<impl AsRef<Path>>) -> Result<()> {
    let ases = if let Some(f) = file {
        import_asns(f).await?
    } else {
        download_asns().await?
    };
    write_to_db(&ases, asdb).await
}

/// Imports asns from "asns.jsonl" file which can be obtained from asrank API using
/// asrank-download.py from https://api.asrank.caida.org/dev/docs
pub async fn import_asns(file: impl AsRef<Path>) -> Result<Vec<As>> {
    println!("importingh asrank data from file");

    let len = BufReader::new(File::open(&file)?).lines().count() as u64;
    // TODO catch unwind form  this deserializer and return jsonl error
    // why the catch unwind even? don't remember
    let json: Vec<As> = serde_json::Deserializer::from_reader(File::open(&file)?)
        .into_iter::<Value>()
        .progress_count(len)
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
                ..Default::default()
            }
        })
        .collect();
    return Ok(json);
}

pub async fn write_to_db(ases: &Vec<As>, asdb: &Asdb) -> Result<()> {
    println!("Inserting asrank data into the database");
    let insert_result = asdb.insert_ases(&ases).await;
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

/// download the asns from https://api.asrank.caida.org
/// TODO https://lib.rs/crates/graphql_client use this this to reimplement asrank-download.py
pub async fn download_asns() -> Result<Vec<As>> {
    use graphql::AsnsQuery;
    use graphql_client::{GraphQLQuery, Response};
    println!("starting download of asns");

    let client = reqwest::Client::new();
    let request_body = AsnsQuery::build_query(asns_query::Variables {
        first: 1,
        offset: 0,
    });
    let res = client.post(API_URL).json(&request_body).send().await?;
    let response_body: Response<asns_query::ResponseData> = res.json().await?;
    let bar = indicatif::ProgressBar::new(
        response_body
            .data
            .expect("response should always have data field")
            .asns
            .total_count as u64,
    );
    let mut out = vec![];
    bar.inc(1);
    let mut page: i64 = 0;
    loop {
        let variables = asns_query::Variables {
            first: PAGE_SIZE,
            offset: 0 + PAGE_SIZE * page,
        };
        let request_body = AsnsQuery::build_query(variables);
        let res = client
            .post("https://api.asrank.caida.org/v2/graphql")
            .json(&request_body)
            .send()
            .await?;
        let response_body: Response<asns_query::ResponseData> = res.json().await?;
        let data = response_body
            .data
            .expect("response should always have data field");
        let edges = data.asns.edges.expect("response should always have edges");
        let mut ases = edges
            .into_iter()
            .map(|x| As::from(x.unwrap()))
            .collect::<Vec<_>>();
        out.append(&mut ases);

        if let Some(true) = data.asns.page_info.has_next_page {
            bar.inc(PAGE_SIZE as u64);
            page = page + 1;
            continue;
        }
        bar.finish();
        break;
    }

    Ok(out)
}
