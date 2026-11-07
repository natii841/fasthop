use crate::{
    models::{SubStatus, Subscription},
    validate,
};
use chrono::{DateTime, Utc};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    PgPool,
};
use std::str::FromStr;
use std::time::Duration;

pub async fn find_subscription(pool: &PgPool, token: &str) -> sqlx::Result<Option<Subscription>> {
    sqlx::query_as::<_, Subscription>("SELECT telegram_id, token, status, expires_at, created_at FROM subscriptions WHERE token = $1")
        .bind(token).fetch_optional(pool).await
}

pub async fn get_subscription(
    pool: &PgPool,
    telegram_id: i64,
    trial_hours: i64,
) -> sqlx::Result<Subscription> {
    let token = validate::generate_token();
    sqlx::query_as::<_, Subscription>(
        "INSERT INTO subscriptions (telegram_id, token, status, expires_at) VALUES ($1, $2, 'trial', now() + make_interval(hours => $3)) ON CONFLICT (telegram_id) DO UPDATE SET telegram_id = EXCLUDED.telegram_id RETURNING telegram_id, token, status, expires_at, created_at")
        .bind(telegram_id).bind(token).bind(trial_hours).fetch_one(pool).await
}

pub async fn log_query(
    pool: &PgPool,
    token: &str,
    domain: &str,
    client_ip: &str,
    resolved_ip: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO query_logs (user_token, domain, client_ip, resolved_ip) VALUES ($1, $2, $3, $4)")
        .bind(token).bind(domain).bind(client_ip).bind(resolved_ip).execute(pool).await.map(|_| ())
}

pub async fn recent_logs(
    pool: &PgPool,
    limit: i64,
) -> sqlx::Result<Vec<(i64, String, Option<String>, Option<String>, DateTime<Utc>)>> {
    sqlx::query_as("SELECT id, domain, user_token, client_ip, timestamp FROM query_logs ORDER BY timestamp DESC LIMIT $1")
        .bind(limit).fetch_all(pool).await
}

pub async fn connect(url: &str, max_conns: u32) -> sqlx::Result<PgPool> {
    let options = PgConnectOptions::from_str(url)?.statement_cache_capacity(0);
    PgPoolOptions::new()
        .max_connections(max_conns)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await
}

pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

pub async fn list_all(pool: &PgPool) -> sqlx::Result<Vec<Subscription>> {
    sqlx::query_as::<_, Subscription>("SELECT telegram_id, token, status, expires_at, created_at FROM subscriptions ORDER BY created_at DESC")
        .fetch_all(pool).await
}

pub async fn grant_days(pool: &PgPool, telegram_id: i64, days: i32) -> sqlx::Result<Subscription> {
    let token = validate::generate_token();
    sqlx::query_as::<_, Subscription>(
        "INSERT INTO subscriptions (telegram_id, token, status, expires_at) VALUES ($1, $2, 'active', now() + make_interval(days => $3)) ON CONFLICT (telegram_id) DO UPDATE SET status = 'active', expires_at = GREATEST(subscriptions.expires_at, now()) + make_interval(days => $3) RETURNING telegram_id, token, status, expires_at, created_at")
        .bind(telegram_id).bind(token).bind(days).fetch_one(pool).await
}

pub async fn set_status(
    pool: &PgPool,
    telegram_id: i64,
    status: SubStatus,
) -> sqlx::Result<Option<Subscription>> {
    sqlx::query_as::<_, Subscription>("UPDATE subscriptions SET status = $2 WHERE telegram_id = $1 RETURNING telegram_id, token, status, expires_at, created_at")
        .bind(telegram_id).bind(status).fetch_optional(pool).await
}
