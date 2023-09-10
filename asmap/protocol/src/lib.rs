use serde::{Deserialize, Serialize};

use asdb_models::{As, Coord};

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
    AllAs((Page, TotalPages, Vec<As>)),
    /// returnes vec of ases matching the filters along the original filters requested
    FilteredAS((AsFilters, Vec<As>)),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AsFilters {
    /// 2 letter country code
    pub country: Option<String>,
    /// top left and bottom right corners of the geo bound
    pub bounds: Option<(Coord, Coord)>,
    // pub addresses
    // pub rank
}

impl From<AsFilters> for asdb_models::AsFilters {
    fn from(value: AsFilters) -> Self {
        asdb_models::AsFilters {
            country_iso: value.country,
            bounds: value.bounds,
            ..Default::default()
        }
    }
}
