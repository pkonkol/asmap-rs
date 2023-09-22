use leaflet::{FeatureGroup, LayerGroup, Marker};
use wasm_bindgen::prelude::*;

pub mod options;

#[wasm_bindgen]
extern "C" {
    // Is it necessary?
    #[derive(Clone, Debug)]
    #[wasm_bindgen(extends = FeatureGroup)]
    pub type MarkerClusterGroup;

    /// markerClusterGroup()
    #[wasm_bindgen(js_namespace = L)]
    pub fn markerClusterGroup(options: &JsValue) -> MarkerClusterGroup;

    // // May not be necesary as FeatureGroup is just a type from leaflet but extended
    // This seems to be custom method from https://github.com/Leaflet/Leaflet.markercluster/blob/master/src/MarkerClusterGroup.js#L85
    // so it must be imported
    // /// markerClusterGroup().addLayer(L.marker(...))
    // #[wasm_bindgen(method)]
    // pub fn addLayer(this: &MarkerClusterGroup, layer: &LayerGroup);

    #[wasm_bindgen(method)]
    pub fn addLayer(this: &MarkerClusterGroup, layer: &Marker);

    /// layers must come from &[Marker]
    #[wasm_bindgen(method)]
    pub fn addLayers(this: &MarkerClusterGroup, layers: Vec<Marker>);

    #[wasm_bindgen(method, js_name = removeLayer)]
    pub fn removeLayer_marker(this: &MarkerClusterGroup, layer: &Marker);

    #[wasm_bindgen(method)]
    pub fn clearLayers(this: &MarkerClusterGroup);

    // /// markerClusterGroup()
    // #[wasm_bindgen]
    // pub fn markerClusterGroupF() -> FeatureGroup;

    // TODO later
    // markers.refreshClusters();
    // markers.refreshClusters([myMarker0, myMarker33]);
    // markers.refreshClusters({id_0: myMarker0, id_any: myMarker33});
    // markers.refreshClusters(myLayerGroup);
    // markers.refreshClusters(myMarker);
    // #[wasm_bindgen(method)]
    // pub fn removeLayers(this: &MarkerClusterGroup, layer: &Marker);
    // #[wasm_bindgen(method)]
    // pub fn hasLayer(this: &MarkerClusterGroup, layer: &Marker);
    // #[wasm_bindgen(method)]
    // pub fn zoomToShowLayer(this: &MarkerClusterGroup, layer: &Marker);
    // https://github.com/Leaflet/Leaflet.markercluster/tree/master#clusters-methods
}
