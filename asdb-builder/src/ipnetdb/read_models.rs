use std::net::{Ipv4Addr, Ipv6Addr};

use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct IPNetDBAsn {
    #[serde(alias = "as")]
    pub as_: u32,
    pub cc: String,
    pub entity: String,
    pub in_use: bool,
    pub ipv4_prefixes: Vec<IpNetwork>,
    pub ipv6_prefixes: Option<Vec<IpNetwork>>,
    pub name: Option<String>,
    pub peers: Option<Vec<u32>>,
    pub private: Option<bool>,
    pub registry: Option<String>,
    pub status: Option<String>,
    pub ix: Option<Vec<IPNetDBIX>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct IPNetDBIX {
    pub exchange: String,
    // These 2 shoud have proper types but there is some bug with deserialization from mongo
    // thread 'tests::insert_then_get_ipnetdb_as' panicked at 'called `Result::unwrap()` on an `Err` value: Connection("Kind: invalid type: string \"1:2:3:4:5:6:7:8\", expected an array of length 16, labels: {}")
    pub ipv4: String, //Ipv4Addr,//[u8; 4],//Ipv4Addr,
    pub ipv6: String, //Ipv6Addr,//[u8; 8],//Ipv6Addr,
    pub name: Option<String>,
    pub speed: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct IPNetDBPrefix {
    #[serde(alias = "as")]
    pub as_: u32,
    pub as_cc: String,
    pub as_entity: String,
    pub as_name: String,
    pub as_private: bool,
    pub as_registry: String,
    pub allocation: IpNetwork,
    pub allocation_cc: String,
    pub allocation_registry: String,
    pub allocation_status: String,
    pub prefix_entity: String,
    pub prefix_name: String,
    pub prefix_origin: Option<Vec<u32>>,
    pub prefix_registry: String,
    pub prefix_asset: Option<Vec<String>>,
    pub prefix_assignment: Option<String>,
    pub prefix_bogon: bool,
    pub prefix_cc: String,
    pub rpki_status: Option<String>,
    pub ix: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PrefixIPNetDBIX {
    pub exchange: String,
    // These 2 shoud have proper types but there is some bug with deserialization from mongo
    // thread 'tests::insert_then_get_ipnetdb_as' panicked at 'called `Result::unwrap()` on an `Err` value: Connection("Kind: invalid type: string \"1:2:3:4:5:6:7:8\", expected an array of length 16, labels: {}")
    pub ipv4: String, //Ipv4Addr,//[u8; 4],//Ipv4Addr,
    pub ipv6: String, //Ipv6Addr,//[u8; 8],//Ipv6Addr,
    pub name: Option<String>,
    pub speed: u32,
}
