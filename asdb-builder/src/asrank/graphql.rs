use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/asrank/schema.json",
    query_path = "src/asrank/query.graphql",
    response_derives = "Debug"
)]
pub struct AsnsQuery;
