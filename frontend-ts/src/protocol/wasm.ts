import {
    decode_ws_response,
    encode_ws_request
} from "../wasm-protocol/pkg/protocol_wasm";
import type { WSRequest, WSResponse } from "./types";

export function ensureProtocolReady(): Promise<void> {
    return Promise.resolve();
}

export function encodeRequest(request: WSRequest): Uint8Array {
    return encode_ws_request(request);
}

export function decodeResponse(bytes: Uint8Array): WSResponse {
    return decode_ws_response(bytes) as WSResponse;
}
