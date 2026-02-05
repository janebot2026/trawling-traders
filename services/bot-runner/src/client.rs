//! Control Plane API Client

use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::config::BotConfig;

/// Maximum retry attempts for transient failures
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff (doubles each retry)
const BASE_DELAY_MS: u64 = 1000;

/// Client for communicating with the control plane
pub struct ControlPlaneClient {
    client: Client,
    base_url: String,
    bot_id: Uuid,
}

impl ControlPlaneClient {
    /// Create new control plane client
    pub fn new(base_url: &str, bot_id: Uuid) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            bot_id,
        })
    }

    /// Execute request with retry logic for transient failures
    ///
    /// Retries up to MAX_RETRIES times with exponential backoff.
    /// Does NOT retry on 4xx client errors (except 429 Too Many Requests).
    async fn with_retry<F, Fut>(&self, operation: &str, make_request: F) -> anyhow::Result<Response>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<Response, reqwest::Error>>,
    {
        let mut last_error = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let delay = Duration::from_millis(BASE_DELAY_MS * (1 << (attempt - 1)));
                warn!(
                    "{} failed (attempt {}/{}), retrying in {:?}",
                    operation, attempt, MAX_RETRIES, delay
                );
                tokio::time::sleep(delay).await;
            }

            match make_request().await {
                Ok(response) => {
                    let status = response.status();
                    // Don't retry on client errors (4xx) except 429
                    if status.is_client_error() && status != StatusCode::TOO_MANY_REQUESTS {
                        return Ok(response);
                    }
                    // Retry on 5xx server errors and 429
                    if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS {
                        last_error = Some(anyhow::anyhow!("{} returned {}", operation, status));
                        continue;
                    }
                    return Ok(response);
                }
                Err(e) => {
                    // Network errors are retryable
                    last_error = Some(anyhow::anyhow!("{} network error: {}", operation, e));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("{} failed after {} retries", operation, MAX_RETRIES)))
    }

    /// Register bot with control plane on first boot
    pub async fn register(
        &self,
        wallet_address: Option<String>,
    ) -> anyhow::Result<RegistrationResponse> {
        let url = format!("{}/v1/bot/{}/register", self.base_url, self.bot_id);

        let req = RegisterRequest {
            agent_wallet: wallet_address.unwrap_or_default(),
        };

        let response = self
            .with_retry("register", || self.client.post(&url).json(&req).send())
            .await?;

        if response.status().is_success() {
            let resp: RegistrationResponse = response.json().await?;
            info!("Bot registered: {:?}", resp);
            Ok(resp)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!("Registration failed: {} - {}", status, text))
        }
    }

    /// Report wallet address to control plane (for post-registration update)
    pub async fn report_wallet(&self, wallet_address: &str) -> anyhow::Result<()> {
        let url = format!("{}/v1/bot/{}/wallet", self.base_url, self.bot_id);

        let req = WalletReportRequest {
            wallet_address: wallet_address.to_string(),
        };

        let response = self
            .with_retry("report_wallet", || self.client.post(&url).json(&req).send())
            .await?;

        if response.status().is_success() {
            info!("Wallet address reported: {}", wallet_address);
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!("Wallet report failed: {} - {}", status, text))
        }
    }

    /// Poll for config updates
    pub async fn get_config(&self) -> anyhow::Result<Option<BotConfig>> {
        let url = format!("{}/v1/bot/{}/config", self.base_url, self.bot_id);

        debug!("Polling config from {}", url);

        let response = self
            .with_retry("get_config", || self.client.get(&url).send())
            .await?;

        match response.status() {
            StatusCode::OK => {
                let config: BotConfigResponse = response.json().await?;
                debug!("Received config version: {}", config.version);
                Ok(Some(BotConfig::from_response(config)?))
            }
            StatusCode::NOT_MODIFIED => {
                debug!("Config unchanged");
                Ok(None)
            }
            _ => {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                Err(anyhow::anyhow!("Config poll failed: {} - {}", status, text))
            }
        }
    }

    /// Acknowledge config version
    pub async fn ack_config(&self, version_id: Uuid) -> anyhow::Result<()> {
        let url = format!("{}/v1/bot/{}/config_ack", self.base_url, self.bot_id);

        let req = ConfigAckRequest {
            version: format!("v{}", version_id),
            hash: version_id.to_string(),
            applied_at: chrono::Utc::now(),
        };

        let response = self
            .with_retry("ack_config", || self.client.post(&url).json(&req).send())
            .await?;

        if response.status().is_success() {
            info!("✓ Config version {} acknowledged", version_id);
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!("Config ack failed: {} - {}", status, text))
        }
    }

    /// Send heartbeat
    pub async fn heartbeat(
        &self,
        status: &str,
        metrics: Option<Vec<MetricInput>>,
    ) -> anyhow::Result<HeartbeatResponse> {
        let url = format!("{}/v1/bot/{}/heartbeat", self.base_url, self.bot_id);

        let req = HeartbeatRequest {
            status: status.to_string(),
            timestamp: chrono::Utc::now(),
            metrics,
        };

        let response = self
            .with_retry("heartbeat", || self.client.post(&url).json(&req).send())
            .await?;

        if response.status().is_success() {
            let resp: HeartbeatResponse = response.json().await?;
            Ok(resp)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!("Heartbeat failed: {} - {}", status, text))
        }
    }

    /// Send events
    pub async fn send_events(&self, events: Vec<EventInput>) -> anyhow::Result<()> {
        let url = format!("{}/v1/bot/{}/events", self.base_url, self.bot_id);

        let req = EventsBatchRequest { events };

        let response = self
            .with_retry("send_events", || self.client.post(&url).json(&req).send())
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(anyhow::anyhow!("Events send failed: {} - {}", status, text))
        }
    }
}

// Request/Response types

#[derive(Debug, Clone, Serialize)]
struct RegisterRequest {
    agent_wallet: String,
}

#[derive(Debug, Clone, Serialize)]
struct WalletReportRequest {
    wallet_address: String,
}

#[derive(Debug, Deserialize)]
pub struct RegistrationResponse {
    pub bot_id: String,
    pub status: String,
    pub config_url: String,
}

#[derive(Debug, Deserialize)]
pub struct BotConfigResponse {
    pub version_id: String,
    pub version: i32,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
struct ConfigAckRequest {
    version: String,
    hash: String,
    applied_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
struct HeartbeatRequest {
    status: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    metrics: Option<Vec<MetricInput>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetricInput {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub equity: rust_decimal::Decimal,
    pub pnl: rust_decimal::Decimal,
}

#[derive(Debug, Deserialize)]
pub struct HeartbeatResponse {
    pub needs_config_update: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
struct EventsBatchRequest {
    events: Vec<EventInput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventInput {
    pub event_type: String,
    pub message: String,
    pub metadata: Option<serde_json::Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
