use crate::{state::AppState, upstream};
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use fasthop_shared::{db, validate};
use hickory_proto::{
    op::{Message, MessageType},
    rr::{rdata::A, Name, RData, Record, RecordType},
};
use std::{collections::HashMap, net::IpAddr, sync::Arc, time::Duration};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/b/{token}", get(get_query).post(post_query))
        .route("/dns-query/{token}", get(get_query).post(post_query))
}

async fn get_query(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(token): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let Some(encoded) = query.get("dns") else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    match URL_SAFE_NO_PAD.decode(encoded).ok() {
        Some(bytes) => serve(state, headers, token, bytes).await,
        None => StatusCode::BAD_REQUEST.into_response(),
    }
}

async fn post_query(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(token): Path<String>,
    body: axum::body::Bytes,
) -> Response {
    serve(state, headers, token, body.to_vec()).await
}

async fn serve(
    state: Arc<AppState>,
    headers: HeaderMap,
    token: String,
    query: Vec<u8>,
) -> Response {
    if !validate::is_well_formed_token(&token) || query.len() < 12 || query.len() > 4096 {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(subscription) = state.registry.get(&state.db, &token).await.ok().flatten() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !subscription.status.grants_access() || subscription.expires_at <= chrono::Utc::now() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let client_ip = client_ip(&headers);
    if !state
        .sessions
        .allow(
            &token,
            network_key(&client_ip),
            Duration::from_secs(state.config.device_switch_seconds),
        )
        .await
    {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [(
                "retry-after",
                state.config.device_switch_seconds.to_string(),
            )],
        )
            .into_response();
    }
    let Ok(message) = Message::from_vec(&query) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Some(question) = message.query() else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let name = question.name().to_ascii();
    let routing = state.routing.read().await;
    if let Some(rule) = routing.find(&name) {
        if question.query_type() == RecordType::A {
            let mut answer = Message::new();
            answer
                .set_id(message.id())
                .set_message_type(MessageType::Response);
            for query in message.queries() {
                answer.add_query(query.clone());
            }
            for ip in &rule.ips {
                answer.add_answer(Record::from_rdata(
                    Name::from_ascii(&name).unwrap_or_else(|_| Name::root()),
                    rule.ttl,
                    RData::A(A(*ip)),
                ));
            }
            let response = answer.to_vec().unwrap_or_default();
            let resolved = rule
                .ips
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let _ = db::log_query(&state.db, &token, &name, &client_ip, Some(&resolved)).await;
            return dns_response(response, rule.ttl);
        }
        if question.query_type() == RecordType::AAAA && rule.block_ipv6 {
            let mut answer = Message::new();
            answer
                .set_id(message.id())
                .set_message_type(MessageType::Response);
            for query in message.queries() {
                answer.add_query(query.clone());
            }
            let response = answer.to_vec().unwrap_or_default();
            let _ = db::log_query(
                &state.db,
                &token,
                &name,
                &client_ip,
                Some("AAAA-suppressed"),
            )
            .await;
            return dns_response(response, rule.ttl);
        }
    }
    drop(routing);
    let client = reqwest::Client::new();
    match upstream::forward(
        &client,
        &state.config.upstreams,
        Duration::from_millis(state.config.upstream_timeout_ms),
        &query,
    )
    .await
    {
        Ok(response) => {
            let _ = db::log_query(&state.db, &token, &name, &client_ip, None).await;
            dns_response(response, 30)
        }
        Err(_) => StatusCode::BAD_GATEWAY.into_response(),
    }
}

fn dns_response(body: Vec<u8>, ttl: u32) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/dns-message"),
        )
        .header(header::CACHE_CONTROL, format!("private, max-age={ttl}"))
        .body(axum::body::Body::from(body))
        .unwrap()
}

fn client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(str::trim)
        .filter(|v| v.parse::<IpAddr>().is_ok())
        .unwrap_or("127.0.0.1")
        .to_owned()
}

fn network_key(ip: &str) -> String {
    match ip.parse::<IpAddr>() {
        Ok(IpAddr::V4(value)) => {
            let octets = value.octets();
            format!("{}.{}.{}", octets[0], octets[1], octets[2])
        }
        Ok(IpAddr::V6(value)) => value.segments()[..4]
            .iter()
            .map(|part| format!("{part:04x}"))
            .collect::<Vec<_>>()
            .join(":"),
        Err(_) => ip.to_owned(),
    }
}
