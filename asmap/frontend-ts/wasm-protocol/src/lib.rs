use wasm_bindgen::prelude::*;

use protocol::{WSRequest, WSResponse};

#[wasm_bindgen]
pub fn encode_ws_request(value: JsValue) -> Result<Vec<u8>, JsValue> {
    let req: WSRequest =
        serde_wasm_bindgen::from_value(value).map_err(|e| JsValue::from_str(&e.to_string()))?;
    bincode::serialize(&req).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn decode_ws_response(bytes: &[u8]) -> Result<JsValue, JsValue> {
    let resp: WSResponse =
        bincode::deserialize(bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&resp).map_err(|e| JsValue::from_str(&e.to_string()))
}
