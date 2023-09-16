use ipnetwork::IpNetwork;
use isocountry::CountryCode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Nic {
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
pub struct As {
    pub asn: u32,
    pub asrank_data: Option<AsrankAsn>,
    pub ipnetdb_data: Option<IPNetDBAsn>,
    pub whois_data: Option<WhoIsAsn>,
}

/// Based on ipnetdb data? or merge ipnetdb with asrank?
/// details from whois, currently only for RIPE
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Organization {
    pub name: String,
    pub registry: Nic,
    pub whois: Option<WhoIsOrg>,
    pub georesolved: Option<Coord>,
}

/// Represents
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Prefix {
    pub cidr: IpNetwork,
    pub registry: Nic,
    pub whois: Option<WhoIsPrefix>,
    pub ipnetdb_data: Option<IPNetDBPrefix>,
}

/// Person data is available only in whois data from registries so there is no
/// optional data for it.
/// currently available only for RIPE
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Person {
    pub person: String,
    pub address: String,
    pub phone: String,
    pub changed: String,
    pub source: Nic,
    pub nic_hdl: Option<String>,
    pub remarks: Option<String>,
    pub mnt_by: Option<String>,
    pub email: Option<String>,
    pub georesolved: Option<Coord>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AsrankAsn {
    // pub name: String, TODO
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct IPNetDBAsn {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WhoIsAsn {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WhoIsOrg {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct IPNetDBPrefix {
    pub prefix: IpNetwork,
    pub allocation: IpNetwork,
    pub allocation_cc: CountryCode,
    pub allocation_registry: Nic,
    pub asn: u32,
    pub as_cc: CountryCode,
    pub as_entity: String,
    pub as_name: String,
    pub as_registry: Nic,
    pub prefix_entity: String,
    pub prefix_name: String,
    pub prefix_origin: Vec<u32>,
    pub prefix_registry: Nic,
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
pub struct AsFilters {
    /// 2 letter country code
    pub country_iso: Option<String>,
    /// top left and bottom right corners of the geo bound
    pub bounds: Option<(Coord, Coord)>,
    /// range of addresses, (min, max)
    pub addresses: Option<(i64, i64)>,
    /// range of allowed ranks, (min, max)
    pub rank: Option<(i64, i64)>,
    // pub contry_name (is that even needed? I have to figure out which to use )
    pub has_org: bool,
}

impl Default for AsFilters {
    fn default() -> Self {
        Self {
            country_iso: None,
            bounds: None,
            addresses: None,
            rank: None,
            has_org: false,
        }
    }
}
