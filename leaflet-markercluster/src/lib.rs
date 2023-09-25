use leaflet::{FeatureGroup, Marker};
use wasm_bindgen::prelude::*;

pub mod options;

#[wasm_bindgen]
extern "C" {
    #[derive(Clone, Debug)]
    #[wasm_bindgen(extends = FeatureGroup)]
    pub type MarkerClusterGroup;

    /// markerClusterGroup()
    #[wasm_bindgen(js_namespace = L)]
    pub fn markerClusterGroup(options: &JsValue) -> MarkerClusterGroup;

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
