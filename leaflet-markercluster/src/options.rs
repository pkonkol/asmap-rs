use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkerClusterGroupOptions {
    pub disable_clustering_at_zoom: u64,
    pub spiderfy_on_every_zoom: bool,
    pub spiderfy_on_max_zoom: bool,
    pub spiderfy_distance_multiplier: f64,
    // this way may be impossible to pass through serde into JS
    //pub max_cluster_radius: Box<dyn Fn(f64) -> f64>,
    // pub max_cluster_radius: Closure<dyn Fn(f64) -> f64>,
    pub max_cluster_radius: u64,
    pub chunked_loading: bool,
}

impl Default for MarkerClusterGroupOptions {
    fn default() -> Self {
        Self {
            disable_clustering_at_zoom: 20,
            spiderfy_on_every_zoom: false,
            spiderfy_on_max_zoom: true,
            spiderfy_distance_multiplier: 1.,
            max_cluster_radius: 80,
            chunked_loading: true,
        }
    }
}
