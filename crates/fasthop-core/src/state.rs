use crate::{
    config::Config,
    registry::Registry,
    routing::{Routing, SharedRouting},
    session::Sessions,
};
use anyhow::Result;
use sqlx::PgPool;
use std::sync::Arc;

pub struct AppState {
    pub config: Config,
    pub db: PgPool,
    pub routing: SharedRouting,
    pub registry: Registry,
    pub sessions: Sessions,
}

impl AppState {
    pub async fn new(config: Config) -> Result<Self> {
        let db: PgPool = fasthop_shared::db::connect(&config.database_url, 10).await?;
        fasthop_shared::db::migrate(&db).await?;
        let routing = if let Some(raw) = &config.routing_json {
            Routing::load(raw)?
        } else {
            Routing::from_file(&config.routing_file).unwrap_or_default()
        };
        Ok(Self {
            config,
            db,
            routing: Arc::new(tokio::sync::RwLock::new(routing)),
            registry: Registry::default(),
            sessions: Sessions::default(),
        })
    }
}
