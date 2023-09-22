use leaflet::{FeatureGroup, LayerGroup, Marker};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // Is it necessary?
    #[derive(Clone, Debug)]
    #[wasm_bindgen(extends = FeatureGroup)]
    pub type MarkerClusterGroup;

    /// markerClusterGroup()
    #[wasm_bindgen(js_namespace = L)]
    pub fn markerClusterGroup() -> MarkerClusterGroup;

    // // May not be necesary as FeatureGroup is just a type from leaflet but extended
    // This seems to be custom method from https://github.com/Leaflet/Leaflet.markercluster/blob/master/src/MarkerClusterGroup.js#L85
    // so it must be imported
    // /// markerClusterGroup().addLayer(L.marker(...))
    // #[wasm_bindgen(method)]
    // pub fn addLayer(this: &MarkerClusterGroup, layer: &LayerGroup);

    #[wasm_bindgen(method)]
    pub fn addLayer(this: &MarkerClusterGroup, layer: &Marker);

    #[wasm_bindgen(method, js_name = removeLayer)]
    pub fn removeLayer_marker(this: &MarkerClusterGroup, layer: &Marker);

    // /// markerClusterGroup()
    // #[wasm_bindgen]
    // pub fn markerClusterGroupF() -> FeatureGroup;

}
