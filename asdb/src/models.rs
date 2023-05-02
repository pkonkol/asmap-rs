use geo_types::Coord;
use ipnetwork::IpNetwork;
use isocountry::CountryCode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
/// Initially based on asrank asns list, then updated with what exactly?
pub struct Asn {
    asn: u32,
    asrank_data: AsrankData,
    //ipnetdb_data: IPNetDBData,
    //whois_data: WhoIsData,
    // georesolvedData??? probably not, for ex. as5550 doesn't have addres fields
}

/// Based on ipnetdb data? or merge ipnetdb with asrank?
/// details from whois, currently only for RIPE
pub struct Organization {
    // georesolved: Option<Coord>
}

/// Based on whois db data
/// currently available only for RIPE
pub struct Person {
    // georesolved: Option<Coord>
}

/// Based on ipnetdb data
/// details from whois, currently only for RIPE
pub struct Prefix {
    cidr: IpNetwork,
    // no address by itself to georesolve, only related persons and orgs have one
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AsrankData {
    rank: u32,
    organisation_long: String,
    country: CountryCode,
    coordinates: Coord,
    degree: AsrankDegree,
    prefixes: u64,
    addresses: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AsrankDegree {
    provider: u32,
    peer: u32,
    customer: u32,
    total: u32,
    transit: u32,
    sibling: u32,
}
