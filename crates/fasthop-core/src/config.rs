use anyhow::{bail, Context, Result};

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub bot_token: Option<String>,
    pub base_url: String,
    pub routing_file: String,
    pub routing_json: Option<String>,
    pub upstreams: Vec<String>,
    pub upstream_timeout_ms: u64,
    pub device_switch_seconds: u64,
    pub admin_token: Option<String>,
    pub trial_hours: i64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL is required")?;
        let base_url = std::env::var("BASE_URL")
            .context("BASE_URL is required")?
            .trim_end_matches('/')
            .to_owned();
        let upstreams = std::env::var("UPSTREAMS")
            .unwrap_or_else(|_| "https://1.1.1.1/dns-query,https://8.8.8.8/dns-query".into())
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "8080".into())
            .parse()
            .context("PORT must be a number")?;
        if port == 0 {
            bail!("PORT must be greater than zero");
        }
        Ok(Self {
            port,
            database_url,
            base_url,
            upstreams,
            bot_token: std::env::var("BOT_TOKEN").ok().filter(|v| !v.is_empty()),
            routing_file: std::env::var("ROUTING_FILE").unwrap_or_else(|_| "routing.json".into()),
            routing_json: std::env::var("ROUTING_JSON").ok(),
            upstream_timeout_ms: std::env::var("UPSTREAM_TIMEOUT_MS")
                .unwrap_or_else(|_| "2000".into())
                .parse()
                .context("UPSTREAM_TIMEOUT_MS must be a number")?,
            device_switch_seconds: std::env::var("DEVICE_SWITCH_SECONDS")
                .unwrap_or_else(|_| "120".into())
                .parse()
                .context("DEVICE_SWITCH_SECONDS must be a number")?,
            admin_token: std::env::var("ADMIN_TOKEN").ok().filter(|v| !v.is_empty()),
            trial_hours: std::env::var("TRIAL_HOURS")
                .unwrap_or_else(|_| "5".into())
                .parse()
                .context("TRIAL_HOURS must be a number")?,
        })
    }
}
