use serde::{Deserialize, Serialize};

use asdb_models::{As, Bound};
// TODO remove pub and switch references to asdb_models
pub use asdb_models::AsForFrontend;

type Asn = u32;

#[derive(Serialize, Deserialize, Debug)]
pub enum WSRequest {
    /// requests all ases that match given filter
    FilteredAS(AsFilters),
    /// details for single As
    AsDetails(Asn),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum WSResponse {
    /// returnes vec of ases matching the filters along the original filters requested
    FilteredAS((AsFilters, Vec<AsForFrontend>)),
    /// details for single As
    AsDetails(As),
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AsFiltersHasOrg {
    Yes,
    No,
    Both,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AsFilters {
    /// 2 letter country code
    pub country: Option<String>,
    // Bound comes from asdb_models so it's kinda mixed in into the protocol but should do for now
    pub exclude_country: bool,
    /// top left and bottom right corners of the geo bound
    pub bounds: Option<Bound>,
    /// range of addresses, (min, max)
    pub addresses: Option<(i64, i64)>,
    /// range of rank, (min, max)
    pub rank: Option<(i64, i64)>,
    /// some ases have no organisation in asrank data
    pub has_org: AsFiltersHasOrg,
    /// layer 1 category from stanford asdb
    pub category: Vec<String>,
}

impl From<AsFilters> for asdb_models::AsFilters {
    fn from(value: AsFilters) -> Self {
        let has_org = match value.has_org {
            AsFiltersHasOrg::Both => None,
            AsFiltersHasOrg::No => Some(false),
            AsFiltersHasOrg::Yes => Some(true),
        };
        asdb_models::AsFilters {
            country_iso: value.country,
            exclude_country: value.exclude_country,
            bounds: value.bounds,
            addresses: value.addresses,
            rank: value.rank,
            has_org,
            category: value.category,
            // ..Default::default()
        }
    }
}

impl Default for AsFilters {
    fn default() -> Self {
        Self {
            country: None,
            exclude_country: false,
            bounds: None,
            addresses: None,
            rank: None,
            has_org: AsFiltersHasOrg::Both,
            category: vec![],
        }
    }
}
