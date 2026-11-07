mod admin;
mod config;
mod doh;
mod registry;
mod routing;
mod session;
mod state;
mod static_files;
mod upstream;

use anyhow::Result;
use axum::{routing::get, Router};
use config::Config;
use state::AppState;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    let config = Config::from_env()?;
    let state = Arc::new(AppState::new(config.clone()).await?);
    if let Some(token) = config.bot_token.clone() {
        let bot_db = state.db.clone();
        let bot_base_url = state.config.base_url.clone();
        let bot_trial_hours = state.config.trial_hours;
        tokio::spawn(async move {
            if let Err(error) =
                fasthop_bot::spawn(token, bot_db, bot_base_url, bot_trial_hours).await
            {
                tracing::error!(%error, "telegram bot stopped");
            }
        });
    }
    let app = Router::new()
        .route(
            "/healthz",
            get(|| async { axum::Json(serde_json::json!({"ok": true})) }),
        )
        .merge(doh::router())
        .merge(admin::router())
        .merge(static_files::router())
        .with_state(state);
    let addr: SocketAddr = ([0, 0, 0, 0], config.port).into();
    tracing::info!(%addr, "fasthop listening");
    axum::serve(TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
