use yew_router::prelude::*;

#[derive(Routable, PartialEq, Eq, Clone, Debug)]
pub enum Route {
    #[at("/")]
    Map,

    // np. /details/123
    #[at("/details/:id")]
    Details { id: String },

    #[not_found]
    #[at("/404")]
    NotFound,
}
