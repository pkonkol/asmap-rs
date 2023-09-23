use serde::Serialize;

#[derive(Serialize)]
pub struct CsvAs<'a> {
    pub asn: &'a u32,
    pub rank: &'a u64,
    pub name: &'a str,
    pub organization: &'a str,
}

impl<'a> From<&'a asdb_models::As> for CsvAs<'a> {
    fn from(value: &'a asdb_models::As) -> Self {
        const DEFAULT: &str = "";
        let rank = value
            .asrank_data
            .as_ref().map(|x| &x.rank)
            .unwrap();
        let name = &value.asrank_data.as_ref().unwrap().name;
        let organization = value
            .asrank_data
            .as_ref()
            .unwrap()
            .organization
            .as_ref()
            .map_or(DEFAULT, |s| s.as_ref());
        Self {
            asn: &value.asn,
            rank,
            name,
            organization,
        }
    }
}
