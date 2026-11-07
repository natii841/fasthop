use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

#[derive(Default)]
pub struct Sessions {
    inner: Mutex<HashMap<String, (String, Instant)>>,
}

impl Sessions {
    pub async fn allow(&self, token: &str, network: String, switch_after: Duration) -> bool {
        let mut sessions = self.inner.lock().await;
        match sessions.get_mut(token) {
            None => {
                sessions.insert(token.into(), (network, Instant::now()));
                true
            }
            Some((current, last_seen)) if *current == network => {
                *last_seen = Instant::now();
                true
            }
            Some((current, last_seen)) if last_seen.elapsed() > switch_after => {
                *current = network;
                *last_seen = Instant::now();
                true
            }
            Some(_) => false,
        }
    }

    pub async fn invalidate(&self, token: &str) {
        self.inner.lock().await.remove(token);
    }
}
