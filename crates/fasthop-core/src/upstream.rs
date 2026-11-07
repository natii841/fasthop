use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;

pub async fn forward(
    client: &Client,
    upstreams: &[String],
    timeout: Duration,
    query: &[u8],
) -> Result<Vec<u8>> {
    for upstream in upstreams {
        let response = match client
            .post(upstream)
            .timeout(timeout)
            .header("content-type", "application/dns-message")
            .body(query.to_vec())
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => response,
            Ok(_) | Err(_) => continue,
        };
        return Ok(response.bytes().await?.to_vec());
    }
    Err(anyhow::anyhow!("all DNS upstreams failed")).context("forwarding DNS query")
}
