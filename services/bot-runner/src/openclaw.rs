//! OpenClaw Gateway Client
//!
//! HTTP client for communicating with the local OpenClaw gateway
//! to request trading decisions.

use crate::types::{DecisionContext, DecisionPlan, GatewayHealth};
use anyhow::{anyhow, Result};
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Default gateway URL
const DEFAULT_GATEWAY_URL: &str = "http://localhost:8090";

/// Default timeout for decision requests (30 seconds)
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// OpenClaw gateway client
pub struct OpenClawClient {
    /// Gateway base URL
    gateway_url: String,
    /// HTTP client with connection pooling
    http_client: Client,
    /// Request timeout
    timeout: Duration,
}

impl OpenClawClient {
    /// Create a new OpenClaw client
    ///
    /// Reads `OPENCLAW_GATEWAY_URL` from environment, defaults to localhost:8090
    pub fn new() -> Self {
        let gateway_url = std::env::var("OPENCLAW_GATEWAY_URL")
            .unwrap_or_else(|_| DEFAULT_GATEWAY_URL.to_string());

        let timeout_secs: u64 = std::env::var("OPENCLAW_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        let http_client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .pool_max_idle_per_host(2)
            .build()
            .expect("Failed to create HTTP client");

        info!(
            "OpenClaw client initialized: url={}, timeout={}s",
            gateway_url, timeout_secs
        );

        Self {
            gateway_url,
            http_client,
            timeout: Duration::from_secs(timeout_secs),
        }
    }

    /// Create client with specific URL (for testing)
    pub fn with_url(gateway_url: String) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .pool_max_idle_per_host(2)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            gateway_url,
            http_client,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        }
    }

    /// Request a trading decision from OpenClaw gateway
    ///
    /// POST /v1/decide with DecisionContext body
    /// Returns DecisionPlan with intents to execute
    pub async fn tick(&self, context: &DecisionContext) -> Result<DecisionPlan> {
        let url = format!("{}/v1/decide", self.gateway_url);

        debug!(
            "Requesting decision from OpenClaw: bot_id={}, portfolio_equity={}",
            context.bot_id, context.portfolio.equity_usd
        );

        let response = self
            .http_client
            .post(&url)
            .json(context)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    anyhow!("OpenClaw decision request timed out after {:?}", self.timeout)
                } else if e.is_connect() {
                    anyhow!("Failed to connect to OpenClaw gateway at {}: {}", url, e)
                } else {
                    anyhow!("OpenClaw request failed: {}", e)
                }
            })?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!(
                "OpenClaw returned error: status={}, body={}",
                status, error_text
            );
            return Err(anyhow!(
                "OpenClaw decision failed with status {}: {}",
                status,
                error_text
            ));
        }

        let plan: DecisionPlan = response.json().await.map_err(|e| {
            anyhow!("Failed to parse OpenClaw decision response: {}", e)
        })?;

        info!(
            "Received decision plan: plan_id={}, intents={}, explanations={}",
            plan.plan_id,
            plan.intents.len(),
            plan.explanations.len()
        );

        Ok(plan)
    }

    /// Check if gateway is healthy
    ///
    /// GET /v1/health
    pub async fn health(&self) -> Result<GatewayHealth> {
        let url = format!("{}/v1/health", self.gateway_url);

        let response = self
            .http_client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| anyhow!("Health check failed: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Gateway health check returned status {}",
                response.status()
            ));
        }

        response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse health response: {}", e))
    }

    /// Check if gateway is available (non-throwing)
    pub async fn is_available(&self) -> bool {
        match self.health().await {
            Ok(health) => health.healthy,
            Err(e) => {
                warn!("OpenClaw gateway not available: {}", e);
                false
            }
        }
    }

    /// Get gateway version
    ///
    /// GET /v1/version
    pub async fn version(&self) -> Result<String> {
        let url = format!("{}/v1/version", self.gateway_url);

        let response = self
            .http_client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| anyhow!("Version check failed: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Gateway version check returned status {}",
                response.status()
            ));
        }

        #[derive(serde::Deserialize)]
        struct VersionResponse {
            version: String,
        }

        let ver: VersionResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse version response: {}", e))?;

        Ok(ver.version)
    }

    /// Get the gateway URL
    pub fn gateway_url(&self) -> &str {
        &self.gateway_url
    }
}

impl Default for OpenClawClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = OpenClawClient::new();
        assert!(client.gateway_url.contains("localhost"));
    }

    #[test]
    fn test_client_with_url() {
        let client = OpenClawClient::with_url("http://custom:9000".to_string());
        assert_eq!(client.gateway_url, "http://custom:9000");
    }
}
