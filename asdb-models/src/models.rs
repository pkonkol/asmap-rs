use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use ipnetwork::IpNetwork;
use isocountry::CountryCode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct As {
    pub asn: u32,
    pub asrank_data: Option<AsrankAsn>,
    pub ipnetdb_data: Option<IPNetDBAsn>,
    pub whois_data: Option<WhoIsAsn>,
    // TODO
    // pub stanford_asdb: Option<StanfordASdb>,
}

// Based on ipnetdb data? or merge ipnetdb with asrank?
// details from whois, currently only for RIPE
// TODO this too was supposed to not be attached to 1 data source. Needed?
//#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
//pub struct Organization {
//    pub name: String,
//    pub registry: Nic,
//    pub whois: Option<WhoIsOrg>,
//    pub georesolved: Option<Coord>,
//}

// TODO is it still needed? The idea was to make generalized prefix struct not attached to single
// data source
//#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
//pub struct Prefix {
//    pub cidr: IpNetwork,
//    pub registry: Nic,
//    pub whois: Option<WhoIsPrefix>,
//    pub ipnetdb_data: Option<IPNetDBPrefix>,
//}

/// Person data is available only in whois data from registries so there is no
/// optional data for it.
/// currently available only for RIPE
// TODO uncomment when ready to use
//#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
//pub struct Person {
//    pub person: String,
//    pub address: String,
//    pub phone: String,
//    pub changed: String,
//    pub source: Nic,
//    pub nic_hdl: Option<String>,
//    pub remarks: Option<String>,
//    pub mnt_by: Option<String>,
//    pub email: Option<String>,
//    pub georesolved: Option<Coord>,
//}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AsrankAsn {
    pub rank: u64,
    pub organization: Option<String>,
    pub country_iso: String,
    pub country_name: String,
    pub coordinates: Coord,
    pub degree: AsrankDegree,
    pub prefixes: u64,
    pub addresses: u64,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AsrankDegree {
    pub provider: u32,
    pub peer: u32,
    pub customer: u32,
    pub total: u32,
    pub transit: u32,
    pub sibling: u32,
}

type Asn = u32;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct IPNetDBAsn {
    pub cc: String,
    pub entity: String,
    pub in_use: bool,
    pub ipv4_prefixes: Vec<IPNetDBPrefix>,
    pub ipv6_prefixes: Vec<IPNetDBPrefix>,
    pub name: String,
    /// stores a list of peer asns
    pub peers: Vec<Asn>,
    pub private: bool,
    pub registry: Registry,
    pub status: String,
    pub ix: Vec<IPNetDBIX>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct IPNetDBIX {
    pub exchange: String,
    // These 2 shoud have proper types but there is some bug with deserialization from mongo
    // thread 'tests::insert_then_get_ipnetdb_as' panicked at 'called `Result::unwrap()` on an `Err` value: Connection("Kind: invalid type: string \"1:2:3:4:5:6:7:8\", expected an array of length 16, labels: {}")
    pub ipv4: [u8; 4],//Ipv4Addr,
    pub ipv6: [u8; 8],//Ipv6Addr,
    pub name: String,
    pub speed: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct IPNetDBPrefix {
    pub range: IpNetwork, // Would it be any better to store it as IpNetwork object?
    pub details: Option<IPNetDBPrefixDetails>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct IPNetDBPrefixDetails {
    pub allocation: IpNetwork,
    pub allocation_cc: String,
    pub allocation_registry: Registry,
    pub prefix_entity: String,
    pub prefix_name: String,
    pub prefix_origin: Vec<Asn>,
    pub prefix_registry: Registry,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WhoIsPrefix {
    pub netname: String,
    pub coutnry: CountryCode,
    pub org: String,
    pub remarks: String,
    pub admin_c: Vec<String>,
    pub tech_c: Vec<String>,
    pub mnt_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WhoIsAsn {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WhoIsOrg {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Registry {
    RIPE,
    ARIN,
    APNIC,
    AFRINIC,
    LACNIC,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Coord {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Bound {
    pub north_east: Coord,
    pub south_west: Coord,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct AsFilters {
    /// 2 letter country code
    pub country_iso: Option<String>,
    /// top left and bottom right corners of the geo bound
    pub bounds: Option<Bound>,
    /// range of addresses, (min, max)
    pub addresses: Option<(i64, i64)>,
    /// range of allowed ranks, (min, max)
    pub rank: Option<(i64, i64)>,
    // pub contry_name (is that even needed? I have to figure out which to use )
    pub has_org: Option<bool>,
}
