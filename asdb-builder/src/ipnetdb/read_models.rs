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

impl TryFrom<IPNetDBAsn> for asdb_models::IPNetDBAsn {
    type Error = &'static str;

    fn try_from(value: IPNetDBAsn) -> Result<Self, Self::Error> {
        //let r = value.registry;
        Ok(Self {
            cc: value.cc,
            entity: value.entity,
            in_use: value.in_use,
            ipv4_prefixes: value
                .ipv4_prefixes
                .into_iter()
                .map(|x| asdb_models::IPNetDBPrefix {
                    range: x,
                    details: None,
                })
                .collect(),
            ipv6_prefixes: value
                .ipv6_prefixes
                .unwrap_or(vec![])
                .into_iter()
                .map(|x| asdb_models::IPNetDBPrefix {
                    range: x,
                    details: None,
                })
                .collect(),
            name: value.name.unwrap_or("".to_string()),
            peers: value.peers.unwrap_or(vec![]),
            private: value.private.unwrap_or(false),
            registry: asdb_models::Registry::try_from(value.registry.unwrap().as_str()).unwrap(),
            status: value.status.unwrap_or("".to_string()),
            ix: value
                .ix
                .unwrap_or(vec![])
                .into_iter()
                .map(|x| asdb_models::IPNetDBIX::try_from(x).unwrap())
                .collect(),
        })
    }
}

impl TryFrom<IPNetDBIX> for asdb_models::IPNetDBIX {
    type Error = &'static str;

    fn try_from(value: IPNetDBIX) -> Result<Self, Self::Error> {
        let ipv4: Option<Ipv4Addr> = value.ipv4.parse().ok();
        if ipv4.is_none() {
            println!("IPNetDBIX couldn't parse ipv4 '{}'", value.ipv4);
        }
        let ipv6: Option<Ipv6Addr> = value.ipv6.parse().ok();
        if ipv6.is_none() {
            println!("IPNetDBIX couldn't parse ipv6 '{}'", value.ipv6);
        }
        Ok(Self {
            exchange: value.exchange,
            ipv4: ipv4.map(|x| x.octets()),
            ipv6: ipv6.map(|x| x.octets()),
            name: value.name.unwrap_or("".to_string()),
            speed: value.speed,
        })
    }
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
    //pub allocation: Option<IpNetwork>,
    pub allocation: String, // IpNetwork had deserialization problems
    pub allocation_cc: String,
    pub allocation_registry: String,
    pub allocation_status: String,
    pub prefix_entity: String,
    pub prefix_name: String,
    pub prefix_origins: Option<Vec<u32>>,
    pub prefix_registry: String,
    pub prefix_asset: Option<Vec<u32>>,
    pub prefix_assignment: Option<String>,
    pub prefix_bogon: bool,
    pub prefix_cc: String,
    pub rpki_status: Option<String>,
    //pub ix: Option<serde_json::Value>,
    pub ix: Option<PrefixIPNetDBIX>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PrefixIPNetDBIX {
    pub exchange: Option<String>,
    // These 2 shoud have proper types but there is some bug with deserialization from mongo
    // thread 'tests::insert_then_get_ipnetdb_as' panicked at 'called `Result::unwrap()` on an `Err` value: Connection("Kind: invalid type: string \"1:2:3:4:5:6:7:8\", expected an array of length 16, labels: {}")
    pub ipv4: Option<String>, //Ipv4Addr,//[u8; 4],//Ipv4Addr,
    pub ipv6: Option<String>, //Ipv6Addr,//[u8; 8],//Ipv6Addr,
    pub name: Option<String>,
    pub speed: Option<u32>,
}

impl TryFrom<IPNetDBPrefix> for asdb_models::IPNetDBPrefixDetails {
    type Error = &'static str;

    fn try_from(value: IPNetDBPrefix) -> Result<Self, Self::Error> {
        Ok(Self {
            allocation: value.allocation.parse().ok(),
            allocation_cc: if value.allocation_cc.len() >= 2 {
                Some(value.allocation_cc)
            } else {
                None
            },
            allocation_registry: asdb_models::Registry::try_from(
                value.allocation_registry.as_str(),
            )
            .ok(),
            prefix_entity: value.prefix_entity,
            prefix_name: value.prefix_name,
            prefix_origins: value.prefix_origins.unwrap_or(vec![]),
            prefix_registry: asdb_models::Registry::try_from(value.prefix_registry.as_str())
                .unwrap(),
        })
    }
}
