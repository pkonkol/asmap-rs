use leaflet::FeatureGroup;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // Is it necessary?
    #[derive(Clone, Debug)]
    #[wasm_bindgen(extends = FeatureGroup)]
    pub type MarkerClusterGroup;

    /// markerClusterGroup()
    #[wasm_bindgen]
    pub fn markerClusterGroup() -> MarkerClusterGroup;

    /// markerClusterGroup()
    #[wasm_bindgen]
    pub fn markerClusterGroupF() -> FeatureGroup;

    // May not be necesary as FeatureGroup is just a type from leaflet but extended
    // or
    // /// markerClusterGroup().addLayer(L.marker(...))
    // #[wasm_bindgen(method)]
    // pub fn addLayer(this: &FeatureGroup);

}
