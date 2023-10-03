use serde::{Deserialize, Serialize};

use asdb_models::{As, Bound, Coord};

type Page = u32;
type TotalPages = u32;

#[derive(Serialize, Deserialize, Debug)]
pub enum WSRequest {
    /// requests a single page of ases without filters
    AllAs(Page),
    /// requests all ases that match given filter
    FilteredAS(AsFilters),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum WSResponse {
    /// returns requested vec of ases along with total number of pages and requested page number
    AllAs((Page, TotalPages, Vec<AsForFrontend>)),
    /// returnes vec of ases matching the filters along the original filters requested
    FilteredAS((AsFilters, Vec<AsForFrontend>)),
}
/*
asn:42151, country:Russia, name: VESTER-AS, rank: 48141, org: Some("Planeta, Ltd"), prefixes: 1, addresses: 512, bgp.he, bgpview
*/
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AsForFrontend {
    pub asn: u32,
    pub rank: u32,
    pub name: String,
    pub country_code: String,
    pub organization: Option<String>,
    pub prefixes: u16,
    pub addresses: u32,
    pub coordinates: Coord,
}

impl From<As> for AsForFrontend {
    fn from(value: As) -> Self {
        let asrank_data = value.asrank_data.unwrap();
        Self {
            asn: value.asn,
            rank: asrank_data.rank as u32,
            name: asrank_data.name,
            country_code: asrank_data.country_iso,
            organization: asrank_data.organization,
            prefixes: asrank_data.prefixes as u16,
            addresses: asrank_data.addresses as u32,
            coordinates: Coord {
                lat: asrank_data.coordinates.lat,
                lon: asrank_data.coordinates.lon,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AsFilters {
    /// 2 letter country code
    pub country: Option<String>,
    // Bound comes from asdb_models so it's kinda mixed in into the protocol but should do for now
    /// top left and bottom right corners of the geo bound
    pub bounds: Option<Bound>,
    /// range of addresses, (min, max)
    pub addresses: Option<(i64, i64)>,
    /// range of rank, (min, max)
    pub rank: Option<(i64, i64)>,
    /// some ases have no organisation in asrank data
    pub has_org: Option<bool>,
}

impl From<AsFilters> for asdb_models::AsFilters {
    fn from(value: AsFilters) -> Self {
        asdb_models::AsFilters {
            country_iso: value.country,
            bounds: value.bounds,
            addresses: value.addresses,
            rank: value.rank,
            has_org: value.has_org,
            ..Default::default()
        }
    }
}

impl Default for AsFilters {
    fn default() -> Self {
        Self {
            country: None,
            bounds: None,
            addresses: None,
            rank: None,
            has_org: None,
        }
    }
}
