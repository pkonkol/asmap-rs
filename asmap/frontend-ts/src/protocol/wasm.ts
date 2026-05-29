import {
    decode_ws_response,
    encode_ws_request
} from "../wasm-protocol/pkg/protocol_wasm";
import type { WSRequest, WSResponse } from "./types";

export function ensureProtocolReady(): Promise<void> {
    return Promise.resolve();
}

export function encodeRequest(request: WSRequest): Uint8Array {
    try {
        return encode_ws_request(request);
    } catch (error) {
        console.error("[protocol] encode_ws_request failed", request, error);
        throw error;
    }
}

export function decodeResponse(bytes: Uint8Array): WSResponse {
    try {
        return decode_ws_response(bytes) as WSResponse;
    } catch (error) {
        console.error("[protocol] decode_ws_response failed", bytes.byteLength, error);
        throw error;
    }
}
