import type {
    As,
    AsFilters,
    AsForFrontend,
    GeocodedAddress,
    UserData,
    WSRequest,
    WSResponse
} from "../protocol/types";
import { decodeResponse, encodeRequest, ensureProtocolReady } from "../protocol/wasm";

const API_URL = "ws://[::1]:8080/as";

async function sendWsRequest(request: WSRequest): Promise<WSResponse> {
    await ensureProtocolReady();

    return new Promise((resolve, reject) => {
        let settled = false;
        let timeoutId = 0;
        const finish = (handler: (value: unknown) => void, value: unknown) => {
            if (settled) {
                return;
            }
            settled = true;
            window.clearTimeout(timeoutId);
            handler(value);
        };

        const socket = new WebSocket(API_URL);
        socket.binaryType = "arraybuffer";

        timeoutId = window.setTimeout(() => {
            finish(reject, new Error("WebSocket timeout"));
            socket.close();
        }, 8000);

        socket.onopen = () => {
            try {
                const payload = encodeRequest(request);
                socket.send(payload);
            } catch (error) {
                console.error("[ws] encode failed", request, error);
                finish(reject, error instanceof Error ? error : new Error(String(error)));
                socket.close();
            }
        };

        socket.onerror = () => {
            console.error("[ws] socket error", request);
            finish(reject, new Error("WebSocket error"));
            socket.close();
        };

        socket.onclose = () => {
            if (!settled) {
                finish(reject, new Error("WebSocket closed without response"));
            }
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
                finish(resolve, response);
            } catch (error) {
                console.error("[ws] decode failed", request, error);
                finish(reject, error instanceof Error ? error : new Error(String(error)));
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

export async function getUserData(asn: number): Promise<UserData> {
    const response = await sendWsRequest({ GetUserData: asn });
    if ("UserData" in response) {
        return response.UserData;
    }
    if ("Error" in response) {
        throw new Error(response.Error);
    }
    throw new Error("Unexpected response for user data");
}

export async function updateUserData(
    asn: number,
    lists?: string[] | null,
    comment?: string | null
): Promise<UserData> {
    const payload: { asn: number; lists?: string[] | null; comment?: string | null } = { asn };
    if (lists !== undefined) {
        payload.lists = lists;
    }
    if (comment !== undefined) {
        payload.comment = comment;
    }
    const response = await sendWsRequest({ UpdateUserData: payload });
    if ("UserData" in response) {
        return response.UserData;
    }
    if ("Error" in response) {
        throw new Error(response.Error);
    }
    throw new Error("Unexpected response for user data update");
}

export async function saveGeocoding(
    asn: number,
    geocoded: GeocodedAddress[]
): Promise<UserData> {
    const response = await sendWsRequest({ SaveGeocoding: { asn, geocoded } });
    if ("UserData" in response) {
        return response.UserData;
    }
    if ("Error" in response) {
        throw new Error(response.Error);
    }
    throw new Error("Unexpected response for geocoding save");
}

export async function getListNames(): Promise<string[]> {
    const response = await sendWsRequest({ GetListNames: null });
    if ("ListNames" in response) {
        return response.ListNames;
    }
    if ("Error" in response) {
        throw new Error(response.Error);
    }
    throw new Error("Unexpected response for list names");
}
