use asdb_models::{AsrankAsn, Coord};
use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/asrank/schema.json",
    query_path = "src/asrank/query.graphql",
    response_derives = "Debug"
)]
pub struct AsnsQuery;

//use asns_query::AsnsQueryAsnsEdges;
impl From<asns_query::AsnsQueryAsnsEdges> for asdb_models::As {
    fn from(value: asns_query::AsnsQueryAsnsEdges) -> Self {
        let node = value
            .node
            .expect("AsnsQueryAsnsEdges object should always have node");
        let country = node.country.unwrap();
        let announcing = node.announcing.unwrap();
        Self {
            asn: node.asn.parse().unwrap(),
            asrank_data: Some(AsrankAsn {
                rank: node.rank.unwrap() as u64,
                organization: node.organization.map(|x| x.org_name).flatten(),
                country_iso: country.iso.unwrap(),
                country_name: country.name.unwrap(),
                coordinates: Coord {
                    lat: node.latitude.unwrap(),
                    lon: node.longitude.unwrap(),
                },
                degree: node.asn_degree.unwrap().into(),
                prefixes: announcing.number_prefixes.unwrap() as u64,
                addresses: announcing.number_addresses.unwrap() as u64,
                name: node.asn_name.unwrap(),
            }),
            ..Default::default()
        }
    }
}

impl From<asns_query::AsnsQueryAsnsEdgesNodeAsnDegree> for asdb_models::AsrankDegree {
    fn from(value: asns_query::AsnsQueryAsnsEdgesNodeAsnDegree) -> Self {
        Self {
            provider: value.provider.unwrap() as u32,
            peer: value.peer.unwrap() as u32,
            customer: value.customer.unwrap() as u32,
            total: value.total.unwrap() as u32,
            transit: value.transit.unwrap() as u32,
            sibling: value.sibling.unwrap() as u32,
        }
    }
}
