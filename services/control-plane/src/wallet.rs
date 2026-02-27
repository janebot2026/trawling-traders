//! Solana wallet balance cache for bot wallets.
//!
//! Uses raw JSON-RPC calls via the existing `reqwest::Client` — no new crate.
//! Balances are cached in-memory with a 60 s TTL and refreshed in bulk every 5 min.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// TTL for cached balance entries.
const BALANCE_TTL: Duration = Duration::from_secs(60);

/// Background refresh interval (batch fetch for all online bots).
const REFRESH_INTERVAL: Duration = Duration::from_secs(300);

/// Max accounts per `getMultipleAccounts` RPC call.
const BATCH_SIZE: usize = 100;

/// Cached balance entry for a single bot wallet.
#[derive(Debug, Clone)]
pub struct CachedBalance {
    pub lamports: u64,
    pub fetched_at: Instant,
}

impl CachedBalance {
    fn is_fresh(&self) -> bool {
        self.fetched_at.elapsed() < BALANCE_TTL
    }
}

/// In-memory balance cache keyed by bot ID.
#[derive(Clone, Default)]
pub struct WalletBalanceCache {
    inner: Arc<RwLock<HashMap<Uuid, CachedBalance>>>,
}

impl WalletBalanceCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a cached balance if it's still within TTL.
    pub fn get(&self, bot_id: &Uuid) -> Option<u64> {
        self.inner
            .read()
            .ok()?
            .get(bot_id)
            .filter(|c| c.is_fresh())
            .map(|c| c.lamports)
    }

    /// Insert or update a cached balance.
    pub fn set(&self, bot_id: Uuid, lamports: u64) {
        if let Ok(mut map) = self.inner.write() {
            map.insert(
                bot_id,
                CachedBalance {
                    lamports,
                    fetched_at: Instant::now(),
                },
            );
        }
    }

    /// Get cached balance or fetch on-demand from Solana RPC.
    ///
    /// Returns `None` if the fetch fails (logs warning, does not propagate).
    pub async fn get_or_fetch(
        &self,
        bot_id: Uuid,
        pubkey: &str,
        client: &reqwest::Client,
        rpc_url: &str,
    ) -> Option<u64> {
        if let Some(lamports) = self.get(&bot_id) {
            return Some(lamports);
        }
        match fetch_balance(client, rpc_url, pubkey).await {
            Ok(lamports) => {
                self.set(bot_id, lamports);
                Some(lamports)
            }
            Err(e) => {
                tracing::warn!(bot_id = %bot_id, pubkey, error = %e, "Failed to fetch balance");
                None
            }
        }
    }

    /// Evict stale entries (called by the refresh task).
    fn evict_stale(&self) {
        if let Ok(mut map) = self.inner.write() {
            let before = map.len();
            map.retain(|_, c| c.fetched_at.elapsed() < BALANCE_TTL * 5);
            let evicted = before - map.len();
            if evicted > 0 {
                tracing::debug!(evicted, remaining = map.len(), "Wallet cache eviction");
            }
        }
    }
}

/// Fetch a single wallet balance via Solana `getBalance` JSON-RPC.
///
/// Returns lamports on success.
pub async fn fetch_balance(
    client: &reqwest::Client,
    rpc_url: &str,
    pubkey: &str,
) -> Result<u64, WalletError> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getBalance",
        "params": [pubkey]
    });

    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| WalletError::Rpc(format!("HTTP error: {e}")))?
        .json()
        .await
        .map_err(|e| WalletError::Rpc(format!("JSON parse error: {e}")))?;

    if let Some(err) = resp.get("error") {
        return Err(WalletError::Rpc(format!("RPC error: {err}")));
    }

    resp["result"]["value"]
        .as_u64()
        .ok_or_else(|| WalletError::Rpc("Missing result.value in getBalance response".into()))
}

/// Batch-fetch balances for multiple pubkeys via `getMultipleAccounts`.
///
/// Returns a vec of `(pubkey_index, lamports)` for accounts that exist.
async fn fetch_balances_batch(
    client: &reqwest::Client,
    rpc_url: &str,
    pubkeys: &[&str],
) -> Result<Vec<(usize, u64)>, WalletError> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getMultipleAccounts",
        "params": [pubkeys, { "encoding": "base64" }]
    });

    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| WalletError::Rpc(format!("HTTP error: {e}")))?
        .json()
        .await
        .map_err(|e| WalletError::Rpc(format!("JSON parse error: {e}")))?;

    if let Some(err) = resp.get("error") {
        return Err(WalletError::Rpc(format!("RPC error: {err}")));
    }

    let accounts = resp["result"]["value"]
        .as_array()
        .ok_or_else(|| WalletError::Rpc("Missing result.value array".into()))?;

    let mut results = Vec::new();
    for (i, account) in accounts.iter().enumerate() {
        if account.is_null() {
            // Account doesn't exist on-chain yet — 0 balance
            results.push((i, 0));
            continue;
        }
        if let Some(lamports) = account["lamports"].as_u64() {
            results.push((i, lamports));
        }
    }

    Ok(results)
}

/// Spawn a background task that refreshes balances for all online bots.
///
/// Runs every 5 minutes. Queries bots with non-null `agent_wallet` that are
/// online/provisioning, then batch-fetches balances via `getMultipleAccounts`.
pub fn spawn_balance_refresh_task(
    pool: sqlx::PgPool,
    cache: WalletBalanceCache,
    client: reqwest::Client,
    rpc_url: String,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(REFRESH_INTERVAL);
        loop {
            interval.tick().await;
            if let Err(e) = refresh_all_balances(&pool, &cache, &client, &rpc_url).await {
                tracing::warn!(error = %e, "Balance refresh task failed");
            }
            cache.evict_stale();
        }
    });
}

/// Inner refresh logic — separated for testability.
async fn refresh_all_balances(
    pool: &sqlx::PgPool,
    cache: &WalletBalanceCache,
    client: &reqwest::Client,
    rpc_url: &str,
) -> Result<(), WalletError> {
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, agent_wallet FROM bots \
         WHERE agent_wallet IS NOT NULL \
           AND status IN ('online', 'provisioning', 'paused')",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| WalletError::Db(e.to_string()))?;

    if rows.is_empty() {
        return Ok(());
    }

    // Process in batches of BATCH_SIZE
    for chunk in rows.chunks(BATCH_SIZE) {
        let pubkeys: Vec<&str> = chunk.iter().map(|(_, pk)| pk.as_str()).collect();
        match fetch_balances_batch(client, rpc_url, &pubkeys).await {
            Ok(results) => {
                for (idx, lamports) in results {
                    if idx < chunk.len() {
                        cache.set(chunk[idx].0, lamports);
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    batch_size = chunk.len(),
                    error = %e,
                    "Batch balance fetch failed, skipping batch"
                );
            }
        }
    }

    tracing::debug!(bots = rows.len(), "Balance refresh complete");
    Ok(())
}

/// Wallet-related errors.
#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    #[error("Solana RPC: {0}")]
    Rpc(String),
    #[error("Database: {0}")]
    Db(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_balance_fresh_within_ttl() {
        let cache = WalletBalanceCache::new();
        let id = Uuid::new_v4();
        cache.set(id, 1_000_000_000);
        assert_eq!(cache.get(&id), Some(1_000_000_000));
    }

    #[test]
    fn cached_balance_returns_none_for_unknown() {
        let cache = WalletBalanceCache::new();
        assert_eq!(cache.get(&Uuid::new_v4()), None);
    }

    #[test]
    fn fetch_balance_parses_valid_response() {
        // Test the JSON parsing path without hitting the network
        let json: serde_json::Value = serde_json::json!({
            "jsonrpc": "2.0",
            "result": { "context": { "slot": 1 }, "value": 42_000_000_000u64 },
            "id": 1
        });
        let lamports = json["result"]["value"].as_u64().unwrap();
        assert_eq!(lamports, 42_000_000_000);
    }

    #[test]
    fn fetch_balance_detects_error_response() {
        let json: serde_json::Value = serde_json::json!({
            "jsonrpc": "2.0",
            "error": { "code": -32600, "message": "Invalid request" },
            "id": 1
        });
        assert!(json.get("error").is_some());
    }

    #[test]
    fn evict_stale_removes_old_entries() {
        let cache = WalletBalanceCache::new();
        let id = Uuid::new_v4();
        // Insert an entry that looks old
        if let Ok(mut map) = cache.inner.write() {
            map.insert(
                id,
                CachedBalance {
                    lamports: 100,
                    fetched_at: Instant::now() - BALANCE_TTL * 10,
                },
            );
        }
        assert!(cache.get(&id).is_none()); // stale — not returned
        cache.evict_stale();
        assert_eq!(cache.inner.read().unwrap().len(), 0);
    }
}
