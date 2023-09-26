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

#[derive(Deserialize, Debug)]
pub struct LatLng {
    pub lat: f64,
    pub lng: f64,
}
#[derive(Deserialize, Debug)]
pub struct LatLngBounds {
    pub _southWest: LatLng,
    pub _northEast: LatLng,
}
