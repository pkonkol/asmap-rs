use std::{net::SocketAddr, num::NonZeroU32};

use axum::{
    extract::{
        ConnectInfo, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use nonzero_ext::nonzero;
use tracing::{debug, info, trace, warn};

use crate::state::ServerState;
use protocol::{AsFilters, WSRequest, WSResponse};

pub async fn as_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<ServerState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_as_socket(socket, addr, state))
}

#[tracing::instrument(skip(state, socket))]
pub async fn handle_as_socket(mut socket: WebSocket, addr: SocketAddr, state: ServerState) {
    loop {
        trace!("handle_as_socket loop start");
        let msg = if let Some(Ok(msg)) = socket.recv().await {
            msg
        } else {
            continue;
        };
        match msg {
            Message::Binary(b) => {
                let req: WSRequest = bincode::deserialize(&b).unwrap();
                match req {
                    WSRequest::FilteredAS(filters) => {
                        info!(
                            "reveived WSRequest::FilteredAs with filters {filters:?} from {}",
                            addr.ip()
                        );
                        let resp = filtered_as(filters, addr, &state).await;
                        socket.send(Message::Binary(resp.into())).await.unwrap();
                        socket.send(Message::Close(None)).await.unwrap();
                        break;
                    }
                    WSRequest::AsDetails(asn) => {
                        info!(
                            "reveived WSRequest::AsDetails for asn {asn} from {}",
                            addr.ip()
                        );
                        let resp = as_details(asn, addr, &state).await;
                        socket.send(Message::Binary(resp.into())).await.unwrap();
                        socket.send(Message::Close(None)).await.unwrap();
                        break;
                    }
                    WSRequest::FetchWhois(asn) => {
                        info!(
                            "received WSRequest::FetchWhois for asn {asn} from {}",
                            addr.ip()
                        );
                        let resp = fetch_whois(asn, addr, &state).await;
                        socket.send(Message::Binary(resp.into())).await.unwrap();
                        socket.send(Message::Close(None)).await.unwrap();
                        break;
                    }
                    WSRequest::GetWhois(asn) => {
                        info!(
                            "received WSRequest::GetWhois for asn {asn} from {}",
                            addr.ip()
                        );
                        let resp = get_whois(asn, addr, &state).await;
                        socket.send(Message::Binary(resp.into())).await.unwrap();
                        socket.send(Message::Close(None)).await.unwrap();
                        break;
                    }
                };
            }
            Message::Close(_x) => {
                info!("reveived websocket Message::Close from {}", addr.ip());
                break;
            }
            _ => {
                info!(
                    "Received unsupported message type: {msg:?} from {}",
                    addr.ip()
                );
                socket.send(Message::Close(None)).await.unwrap();
                break;
            }
        };
    }
}

/// returns WsResponse containing ases that match certain filters encoded using bincode
#[tracing::instrument(skip(state))]
async fn filtered_as(filters: AsFilters, addr: SocketAddr, state: &ServerState) -> Vec<u8> {
    let ases_count = state
        .asdb
        .count_ases_filtered(&asdb_models::AsFilters::from(filters.clone()))
        .await
        .unwrap();
    debug!("ases count for current filters is {ases_count}");

    state
        .simple_limiter
        .check_key_n(&addr.ip(), NonZeroU32::new(ases_count as u32).unwrap())
        .unwrap()
        .unwrap();

    let ases = state
        .asdb
        .get_ases_filtered(&asdb_models::AsFilters::from(filters.clone()))
        .await
        .unwrap();
    let resp = WSResponse::FilteredAS((filters.clone(), ases));
    let serialized = bincode::serialize(&resp).unwrap();
    debug!("successfuly encoded ases filtered by {filters:?} ");
    serialized
}

/// returns WsResponse containing details for single AS encoded using bincode
#[tracing::instrument(skip(state))]
async fn as_details(asn: u32, addr: SocketAddr, state: &ServerState) -> Vec<u8> {
    // separate limiter for detailed request? would be best TODO
    state
        .detailed_limiter
        .check_key_n(&addr.ip(), nonzero!(1u32))
        .unwrap()
        .unwrap();

    let as_ = state.asdb.get_as(asn).await.unwrap();
    let resp = WSResponse::AsDetails(as_);
    let serialized = bincode::serialize(&resp).unwrap();
    debug!("successfuly encoded AS{asn} details");
    serialized
}

/// Fetches WHOIS data from RIPE API, caches it in the database, and returns it
#[tracing::instrument(skip(state))]
async fn fetch_whois(asn: u32, addr: SocketAddr, state: &ServerState) -> Vec<u8> {
    // Use detailed limiter for WHOIS requests (rate limited)
    if let Err(e) = state.detailed_limiter.check_key_n(&addr.ip(), nonzero!(1u32)) {
        warn!("Rate limit exceeded for WHOIS fetch from {}: {:?}", addr.ip(), e);
        let resp = WSResponse::Error("Rate limit exceeded".to_string());
        return bincode::serialize(&resp).unwrap();
    }

    // Fetch from RIPE API
    let whois_result = state.whois_client.get_as_whois_data(asn).await;

    let resp = match whois_result {
        Ok(data) => {
            // Convert to our model and cache
            let whois_data = asdb_models::WhoIsAsn {
                as_name: data.aut_num.as_name,
                descr: data.aut_num.descr,
                org_id: data.aut_num.org,
                admin_c: data.aut_num.admin_c,
                tech_c: data.aut_num.tech_c,
                abuse_c: data.aut_num.abuse_c,
                country: data.aut_num.country,
                organisation: data.organisation.map(|o| asdb_models::WhoIsOrg {
                    org_id: o.org_id,
                    org_name: o.org_name,
                    org_type: o.org_type,
                    address: o.address,
                    country: o.country,
                    phone: o.phone,
                    email: o.email,
                }),
                contacts: data
                    .contacts
                    .into_iter()
                    .map(|p| asdb_models::WhoIsPerson {
                        nic_hdl: p.nic_hdl,
                        name: p.name,
                        address: p.address,
                        phone: p.phone,
                        email: p.email,
                    })
                    .collect(),
                fetched_at: Some(chrono::Utc::now().to_rfc3339()),
            };

            // Cache in database (ignore errors)
            if let Err(e) = state.asdb.update_whois_data(asn, &whois_data).await {
                warn!("Failed to cache WHOIS data for AS{}: {:?}", asn, e);
            } else {
                debug!("Cached WHOIS data for AS{}", asn);
            }

            WSResponse::WhoisData(Some(whois_data))
        }
        Err(e) => {
            warn!("Failed to fetch WHOIS data for AS{}: {:?}", asn, e);
            WSResponse::WhoisData(None)
        }
    };

    let serialized = bincode::serialize(&resp).unwrap();
    debug!("Successfully encoded WHOIS response for AS{}", asn);
    serialized
}

/// Returns cached WHOIS data from database (without fetching from API)
#[tracing::instrument(skip(state))]
async fn get_whois(asn: u32, addr: SocketAddr, state: &ServerState) -> Vec<u8> {
    state
        .detailed_limiter
        .check_key_n(&addr.ip(), nonzero!(1u32))
        .unwrap()
        .unwrap();

    let resp = match state.asdb.get_whois_data(asn).await {
        Ok(whois_data) => WSResponse::WhoisData(whois_data),
        Err(e) => {
            warn!("Failed to get cached WHOIS data for AS{}: {:?}", asn, e);
            WSResponse::WhoisData(None)
        }
    };

    let serialized = bincode::serialize(&resp).unwrap();
    debug!("Successfully encoded cached WHOIS response for AS{}", asn);
    serialized
}
