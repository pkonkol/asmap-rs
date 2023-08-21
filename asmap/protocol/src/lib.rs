use serde::Deserialize;

use asdb_models::{As, Coord};

type Page = u32;

#[derive(Deserialize, Debug)]
pub enum WSRequest {
    AllAs(Page),
    FilteredAS(AsFilters),
}

#[derive(Deserialize, Debug)]
pub enum WSResponse {
    AllAs((Page, Vec<As>)),
    FilteredAS((AsFilters, Vec<As>)),
}

#[derive(Deserialize, Debug)]
pub struct AsFilters {
    /// 2 letter country code
    pub country: Option<String>,
    /// top left and bottom right corners of the geo bound
    pub bounds: Option<(Coord, Coord)>,
}
