use fasthop_shared::{db, models::Subscription};
use sqlx::PgPool;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

#[derive(Clone, Default)]
pub struct Registry {
    tokens: Arc<RwLock<HashMap<String, (Subscription, Instant)>>>,
}

impl Registry {
    pub async fn get(&self, pool: &PgPool, token: &str) -> sqlx::Result<Option<Subscription>> {
        if let Some((subscription, loaded)) = self.tokens.read().await.get(token) {
            if loaded.elapsed() < Duration::from_secs(30) {
                return Ok(Some(subscription.clone()));
            }
        }
        let subscription = db::find_subscription(pool, token).await?;
        if let Some(value) = &subscription {
            self.tokens
                .write()
                .await
                .insert(token.to_owned(), (value.clone(), Instant::now()));
        }
        Ok(subscription)
    }

    pub async fn invalidate(&self, token: &str) {
        self.tokens.write().await.remove(token);
    }
}
