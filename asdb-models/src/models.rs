use std::fmt::Display;

use ipnetwork::IpNetwork;
use isocountry::CountryCode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct As {
    pub asn: u32,
    pub asrank_data: Option<AsrankAsn>,
    pub ipnetdb_data: Option<IPNetDBAsn>,
    pub whois_data: Option<WhoIsAsn>,
    pub stanford_asdb: Vec<StanfordASdbCategory>,
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
    pub rank: u32,
    pub organization: Option<String>,
    pub country_iso: String,
    pub country_name: String,
    pub coordinates: Coord,
    pub degree: AsrankDegree,
    pub prefixes: u32,
    pub addresses: u32,
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

impl Display for AsrankDegree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "provider:{},peer:{},customer:{},total:{},transit:{},sibling:{}",
            self.provider, self.peer, self.customer, self.total, self.transit, self.sibling
        )
    }
}

type Asn = u32;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct IPNetDBAsn {
    pub cc: String,
    pub entity: String,
    pub in_use: bool,
    pub ipv4_prefixes: Vec<IPNetDBPrefix>,
    pub ipv6_prefixes: Vec<IPNetDBPrefix>,
    pub name: Option<String>,
    /// stores a list of peer asns
    pub peers: Vec<Asn>,
    pub private: bool,
    pub registry: InternetRegistry,
    pub status: Option<String>,
    pub ix: Vec<IPNetDBIX>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct IPNetDBIX {
    pub exchange: String,
    // These 2 shoud have proper types but there is some bug with deserialization from mongo
    // thread 'tests::insert_then_get_ipnetdb_as' panicked at 'called `Result::unwrap()` on an `Err` value: Connection("Kind: invalid type: string \"1:2:3:4:5:6:7:8\", expected an array of length 16, labels: {}")
    pub ipv4: Option<[u8; 4]>,  //Ipv4Addr,
    pub ipv6: Option<[u8; 16]>, //Ipv6Addr,
    pub name: Option<String>,
    pub speed: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct IPNetDBPrefix {
    pub range: IpNetwork, // Would it be any better to store it as IpNetwork object?
    pub details: Option<IPNetDBPrefixDetails>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct IPNetDBPrefixDetails {
    pub allocation: Option<IpNetwork>,
    pub allocation_cc: Option<String>,
    pub allocation_registry: Option<InternetRegistry>,
    pub prefix_entity: String,
    pub prefix_name: String,
    pub prefix_origins: Vec<Asn>,
    pub prefix_registry: String,
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
pub enum InternetRegistry {
    RIPE,
    ARIN,
    APNIC,
    AFRINIC,
    LACNIC,
    LOCAL(String),
    EMPTY,
}

impl From<&str> for InternetRegistry {
    //type Error = String;

    fn from(value: &str) -> Self {
        let value = value.trim();
        if value.eq_ignore_ascii_case("ripe") {
            return Self::RIPE;
        } else if value.eq_ignore_ascii_case("arin") {
            return Self::ARIN;
        } else if value.eq_ignore_ascii_case("apnic") {
            return Self::APNIC;
        } else if value.eq_ignore_ascii_case("afrinic") {
            return Self::AFRINIC;
        } else if value.eq_ignore_ascii_case("lacnic") {
            return Self::LACNIC;
        }
        if value.is_empty() {
            return Self::EMPTY;
        }
        println!("unknown (local?) registry {value}");
        Self::LOCAL(value.to_string())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct StanfordASdbCategory {
    pub layer1: String,
    pub layer2: String,
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
    // should the country_iso field be excluded from the resuts or included
    pub exclude_country: bool,
    /// top left and bottom right corners of the geo bound
    pub bounds: Option<Bound>,
    /// range of addresses, (min, max)
    pub addresses: Option<(i64, i64)>,
    /// range of allowed ranks, (min, max)
    pub rank: Option<(i64, i64)>,
    // pub contry_name (is that even needed? I have to figure out which to use )
    pub has_org: Option<bool>,
    /// layer 1 category from stanford asdb
    pub category: Vec<String>,
}

// TODO use if manual impl wont work
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AsForFrontendFromDB {
    pub asn: u32,
    #[serde(alias = "asrank_data")]
    pub asrank: AsForFrontendFromDBAsrank,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AsForFrontendFromDBAsrank {
    pub rank: u32,
    pub name: String,
    pub country_iso: String,
    pub organization: Option<String>,
    pub prefixes: u32,
    pub addresses: u32,
    pub coordinates: Coord,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AsForFrontend {
    pub asn: u32,
    pub rank: u32,
    pub name: String,
    pub country_code: String,
    pub organization: Option<String>,
    pub prefixes: u32,
    pub addresses: u32,
    pub coordinates: Coord,
}

// prob 1000 times easier
impl From<AsForFrontendFromDB> for AsForFrontend {
    fn from(value: AsForFrontendFromDB) -> Self {
        Self {
            asn: value.asn,
            rank: value.asrank.rank,
            name: value.asrank.name,
            country_code: value.asrank.country_iso,
            organization: value.asrank.organization,
            prefixes: value.asrank.prefixes,
            addresses: value.asrank.addresses,
            coordinates: value.asrank.coordinates,
        }
    }
}

impl From<As> for AsForFrontend {
    fn from(value: As) -> Self {
        let asrank_data = value.asrank_data.unwrap();
        Self {
            asn: value.asn,
            rank: asrank_data.rank,
            name: asrank_data.name,
            country_code: asrank_data.country_iso,
            organization: asrank_data.organization,
            prefixes: asrank_data.prefixes,
            addresses: asrank_data.addresses,
            coordinates: Coord {
                lat: asrank_data.coordinates.lat,
                lon: asrank_data.coordinates.lon,
            },
        }
    }
}
