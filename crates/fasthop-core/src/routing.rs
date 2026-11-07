use anyhow::{Context, Result};
use fasthop_shared::{validate_rule, RoutingRule};
use std::{collections::HashMap, fs, sync::Arc};

#[derive(Clone, Debug)]
pub struct CompiledRule {
    pub ips: Vec<std::net::Ipv4Addr>,
    pub ttl: u32,
    pub block_ipv6: bool,
}

#[derive(Clone, Default)]
pub struct Routing {
    exact: HashMap<String, CompiledRule>,
    wildcard: HashMap<String, CompiledRule>,
}

impl Routing {
    pub fn load(raw: &str) -> Result<Self> {
        let rules: Vec<RoutingRule> = serde_json::from_str(raw).context("invalid routing JSON")?;
        let mut routing = Self::default();
        for rule in rules {
            validate_rule(&rule)
                .with_context(|| format!("invalid routing rule {}", rule.domain))?;
            let domain = rule
                .domain
                .to_ascii_lowercase()
                .trim_end_matches('.')
                .to_owned();
            let compiled = CompiledRule {
                ips: rule
                    .ips
                    .iter()
                    .map(|ip| ip.parse())
                    .collect::<Result<_, _>>()?,
                ttl: rule.ttl.clamp(1, 86400),
                block_ipv6: rule.block_ipv6,
            };
            if let Some(wildcard) = domain.strip_prefix("*.") {
                routing.wildcard.insert(wildcard.into(), compiled);
            } else {
                routing.exact.insert(domain, compiled);
            }
        }
        Ok(routing)
    }
    pub fn from_file(path: &str) -> Result<Self> {
        Self::load(&fs::read_to_string(path).with_context(|| format!("reading {path}"))?)
    }
    pub fn find(&self, name: &str) -> Option<&CompiledRule> {
        let name = name.trim_end_matches('.').to_ascii_lowercase();
        self.exact.get(&name).or_else(|| {
            name.split('.')
                .skip(1)
                .map(|_| ())
                .enumerate()
                .find_map(|(i, _)| {
                    let suffix = name.split('.').skip(i + 1).collect::<Vec<_>>().join(".");
                    self.wildcard.get(&suffix)
                })
        })
    }
}

pub type SharedRouting = Arc<tokio::sync::RwLock<Routing>>;
