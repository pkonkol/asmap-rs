use asdb_models::As;
use protocol::AsForFrontend;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct CsvAs<'a> {
    pub asn: &'a u32,
    pub rank: &'a u32,
    pub name: &'a str,
    pub organization: &'a str,
}

impl<'a> From<&'a AsForFrontend> for CsvAs<'a> {
    fn from(value: &'a AsForFrontend) -> Self {
        const DEFAULT: &str = "";
        let rank = &value.rank;
        let name = &value.name;
        let organization = value.organization.as_ref().map_or(DEFAULT, |s| s.as_ref());
        Self {
            asn: &value.asn,
            rank,
            name,
            organization,
        }
    }
}

#[derive(Serialize)]
pub struct CsvAsDetailed<'a> {
    pub asn: &'a u32,
    pub rank: &'a u32,
    pub name: &'a str,
    pub organization: &'a str,
}

impl<'a> From<&'a As> for CsvAsDetailed<'a> {
    fn from(value: &'a As) -> Self {
        const DEFAULT: &str = "";
        let asrank = value.asrank_data.as_ref().unwrap();
        let rank = &asrank.rank;
        let name = &asrank.name;
        let organization = asrank.organization.as_ref().map_or(DEFAULT, |s| s.as_ref());
        Self {
            asn: &value.asn,
            rank,
            name,
            organization,
        }
    }
}

pub enum DownloadableCsvInput<'a> {
    Simple(Box<dyn Iterator<Item = &'a AsForFrontend> + 'a>),
    Detailed(Box<dyn Iterator<Item = &'a As> + 'a>),
}

#[derive(Deserialize, Debug)]
pub struct LatLng {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Deserialize, Debug)]
pub struct LatLngBounds {
    #[serde(alias = "_southWest")]
    pub _south_west: LatLng,
    #[serde(alias = "_northEast")]
    pub _north_east: LatLng,
}

// TODO is it needed? It would allow to use a bit less unwraps when forking with the frontend inputs
// pub struct FrontendFilters { }
