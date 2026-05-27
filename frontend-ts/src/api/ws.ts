import type { As, AsFilters, AsForFrontend, WSRequest, WSResponse } from "../protocol/types";
import { decodeResponse, encodeRequest, ensureProtocolReady } from "../protocol/wasm";

const API_URL = "ws://[::1]:8080/as";

async function sendWsRequest(request: WSRequest): Promise<WSResponse> {
    await ensureProtocolReady();

    return new Promise((resolve, reject) => {
        const socket = new WebSocket(API_URL);
        socket.binaryType = "arraybuffer";

        socket.onopen = () => {
            try {
                const payload = encodeRequest(request);
                socket.send(payload);
            } catch (error) {
                reject(error);
            }
        };

        socket.onerror = () => {
            reject(new Error("WebSocket error"));
        };

        socket.onmessage = async (event) => {
            try {
                let bytes: Uint8Array;
                if (event.data instanceof ArrayBuffer) {
                    bytes = new Uint8Array(event.data);
                } else if (event.data instanceof Blob) {
                    bytes = new Uint8Array(await event.data.arrayBuffer());
                } else {
                    throw new Error("Unexpected message type");
                }

                const response = decodeResponse(bytes);
                resolve(response);
            } catch (error) {
                reject(error);
            } finally {
                socket.close();
            }
        };
    });
}

export async function getAllAsFiltered(filters: AsFilters): Promise<AsForFrontend[]> {
    const response = await sendWsRequest({ FilteredAS: filters });
    if ("FilteredAS" in response) {
        return response.FilteredAS[1];
    }
    if ("Error" in response) {
        throw new Error(response.Error);
    }
    throw new Error("Unexpected response for filtered ASes");
}

export async function getAsDetails(asn: number): Promise<As> {
    const response = await sendWsRequest({ AsDetails: asn });
    if ("AsDetails" in response) {
        return response.AsDetails;
    }
    if ("Error" in response) {
        throw new Error(response.Error);
    }
    throw new Error("Unexpected response for AS details");
}

export async function fetchAsWhoisData(asn: number): Promise<As["whois_data"]> {
    const response = await sendWsRequest({ FetchWhois: asn });
    if ("WhoisData" in response) {
        return response.WhoisData;
    }
    if ("Error" in response) {
        throw new Error(response.Error);
    }
    throw new Error("Unexpected response for WHOIS data");
}
