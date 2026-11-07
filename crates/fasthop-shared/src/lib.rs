pub mod db;
pub mod models;
pub mod validate;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub domain: String,
    pub ips: Vec<String>,
    #[serde(default = "default_ttl")]
    pub ttl: u32,
    #[serde(default = "default_true")]
    pub block_ipv6: bool,
}

fn default_ttl() -> u32 {
    30
}
fn default_true() -> bool {
    true
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("domain is required")]
    EmptyDomain,
    #[error("domain must be an exact name or wildcard subdomain")]
    InvalidDomain,
    #[error("at least one IPv4 address is required")]
    MissingIps,
    #[error("invalid IPv4 address: {0}")]
    InvalidIp(String),
}

pub fn validate_rule(rule: &RoutingRule) -> Result<(), ValidationError> {
    let domain = rule.domain.trim().trim_end_matches('.');
    if domain.is_empty() {
        return Err(ValidationError::EmptyDomain);
    }
    if (domain.starts_with("*.") && domain.len() <= 2)
        || (domain.contains('*') && !domain.starts_with("*."))
    {
        return Err(ValidationError::InvalidDomain);
    }
    if domain.split('.').any(|part| part.is_empty()) {
        return Err(ValidationError::InvalidDomain);
    }
    if rule.ips.is_empty() {
        return Err(ValidationError::MissingIps);
    }
    for ip in &rule.ips {
        if ip.parse::<std::net::Ipv4Addr>().is_err() {
            return Err(ValidationError::InvalidIp(ip.clone()));
        }
    }
    Ok(())
}
