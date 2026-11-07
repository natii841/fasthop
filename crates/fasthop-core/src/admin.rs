use crate::{routing::Routing, state::AppState};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use fasthop_shared::{db, models::SubStatus, validate};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
struct GrantRequest {
    telegram_id: i64,
    days: i32,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/admin", get(index))
        .route("/api/admin/subscriptions", get(list_subscriptions))
        .route("/api/admin/logs", get(list_logs))
        .route("/api/admin/grant", post(grant))
        .route(
            "/api/admin/subscriptions/{telegram_id}/{status}",
            post(set_status),
        )
        .route("/api/admin/routing/reload", post(reload_routing))
}

async fn index() -> &'static str {
    "FastHop admin API"
}

fn authorized(state: &AppState, headers: &HeaderMap) -> bool {
    let Some(expected) = &state.config.admin_token else {
        return false;
    };
    headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == format!("Bearer {expected}"))
}

async fn list_subscriptions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !authorized(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    match db::list_all(&state.db).await {
        Ok(rows) => Json(rows).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn list_logs(State(state): State<Arc<AppState>>, headers: HeaderMap) -> impl IntoResponse {
    if !authorized(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    match db::recent_logs(&state.db, 100).await {
        Ok(rows) => Json(rows).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn grant(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<GrantRequest>,
) -> impl IntoResponse {
    if !authorized(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let Ok(days) = validate::validate_days(request.days) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    match db::grant_days(&state.db, request.telegram_id, days).await {
        Ok(subscription) => {
            state.registry.invalidate(&subscription.token).await;
            Json(subscription).into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn set_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((telegram_id, status)): Path<(i64, String)>,
) -> impl IntoResponse {
    if !authorized(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let Some(status) = (match status.as_str() {
        "trial" => Some(SubStatus::Trial),
        "active" => Some(SubStatus::Active),
        "paused" => Some(SubStatus::Paused),
        "expired" => Some(SubStatus::Expired),
        _ => None,
    }) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    match db::set_status(&state.db, telegram_id, status).await {
        Ok(Some(subscription)) => {
            state.registry.invalidate(&subscription.token).await;
            state.sessions.invalidate(&subscription.token).await;
            Json(subscription).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn reload_routing(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !authorized(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let loaded = if let Some(raw) = &state.config.routing_json {
        Routing::load(raw)
    } else {
        Routing::from_file(&state.config.routing_file)
    };
    match loaded {
        Ok(routing) => {
            *state.routing.write().await = routing;
            StatusCode::NO_CONTENT.into_response()
        }
        Err(_) => StatusCode::BAD_REQUEST.into_response(),
    }
}
