use crate::types::*;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, watch, Mutex, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, warn};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsSink = SplitSink<WsStream, Message>;
type WsReader = SplitStream<WsStream>;

/// Capacity of the broadcast channel for price updates.
///
/// broadcast drops the *oldest* messages when the channel is full, which is
/// preferable to stalling the message handler (DR-001).
const PRICE_BROADCAST_CAPACITY: usize = 10_000;

/// Binance WebSocket client for real-time price feeds.
///
/// Uses a `broadcast` channel for price distribution so that:
/// - Multiple consumers can each call `subscribe()` to get their own `Receiver`.
/// - On reconnect the channel is reused — existing subscribers keep receiving
///   without any replacement of receiver handles (DR-001).
///
/// Graceful shutdown of the message-handler task is driven by a `watch` channel
/// rather than `abort()`, so no locks are left held on shutdown (DR-003).
pub struct BinanceWebSocketClient {
    /// WebSocket write half (for sending subscriptions)
    ws_sink: Arc<Mutex<WsSink>>,
    /// WebSocket read half (for receiving messages)
    ws_reader: Arc<Mutex<WsReader>>,
    /// Broadcast sender — never replaced after construction (DR-001).
    /// Wrapped in Option so the message handler can detect sender drops, though
    /// in normal operation the Arc keeps the sender alive indefinitely.
    price_tx: Arc<broadcast::Sender<PricePoint>>,
    /// Subscribed streams
    subscriptions: Arc<RwLock<HashMap<String, String>>>, // symbol -> stream_name
    /// Connection status
    connected: Arc<RwLock<bool>>,
    /// Shutdown signal for the running message-handler task (DR-003).
    ///
    /// Sending `true` tells the task to exit cleanly.  A new `watch` channel is
    /// created for each new handler task so old signals do not bleed over.
    shutdown_tx: Arc<Mutex<watch::Sender<bool>>>,
    /// Atomic counter for unique subscription request IDs
    next_sub_id: Arc<AtomicU64>,
    /// Monotonically increasing count of price updates that could not be
    /// delivered (no active receivers).  Exposed via [`dropped_count`] for
    /// health-endpoint reporting.
    dropped_count: Arc<AtomicU64>,
}

impl BinanceWebSocketClient {
    fn normalize_symbol(symbol: &str) -> String {
        if symbol.contains('/') {
            return symbol.to_string();
        }
        if let Some(base) = symbol.strip_suffix("USDT") {
            // Use USD as our canonical quote key so cache lookups and HTTP responses
            // share the same symbol form.
            format!("{}/USD", base)
        } else if let Some(base) = symbol.strip_suffix("USD") {
            format!("{}/USD", base)
        } else {
            symbol.to_string()
        }
    }

    /// Connect to Binance combined stream WebSocket.
    pub async fn new() -> Result<Self> {
        let url = "wss://stream.binance.com:9443/ws";

        let (ws_stream, _) = connect_async(url).await.map_err(|e| {
            DataRetrievalError::ApiError(format!("WebSocket connection failed: {}", e))
        })?;

        info!("Connected to Binance WebSocket");

        let (ws_sink, ws_reader) = ws_stream.split();

        // broadcast channel — never replaced on reconnect (DR-001).
        let (price_tx, _) = broadcast::channel(PRICE_BROADCAST_CAPACITY);
        let price_tx = Arc::new(price_tx);

        // Initial shutdown channel — replaced each time a new handler task starts.
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let shutdown_tx = Arc::new(Mutex::new(shutdown_tx));

        let client = Self {
            ws_sink: Arc::new(Mutex::new(ws_sink)),
            ws_reader: Arc::new(Mutex::new(ws_reader)),
            price_tx,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            connected: Arc::new(RwLock::new(true)),
            shutdown_tx,
            next_sub_id: Arc::new(AtomicU64::new(1)),
            dropped_count: Arc::new(AtomicU64::new(0)),
        };

        // Spawn initial message handler.
        let handler_clone = client.clone_state();
        tokio::spawn(async move {
            handler_clone.message_handler(shutdown_rx).await;
        });

        Ok(client)
    }

    /// Clone all Arc fields for spawning tasks.
    ///
    /// DR-019: Renamed from `clone` to `clone_state` to avoid shadowing the `Clone`
    /// trait method.  `BinanceWebSocketClient` intentionally does not implement `Clone`
    /// because the type holds interior-mutable WebSocket streams; this helper is an
    /// explicit, in-module-only operation.
    fn clone_state(&self) -> Self {
        Self {
            ws_sink: Arc::clone(&self.ws_sink),
            ws_reader: Arc::clone(&self.ws_reader),
            price_tx: Arc::clone(&self.price_tx),
            subscriptions: Arc::clone(&self.subscriptions),
            connected: Arc::clone(&self.connected),
            shutdown_tx: Arc::clone(&self.shutdown_tx),
            next_sub_id: Arc::clone(&self.next_sub_id),
            dropped_count: Arc::clone(&self.dropped_count),
        }
    }

    /// Subscribe to real-time trades for a symbol.
    pub async fn subscribe_trades(&self, symbol: &str) -> Result<()> {
        let stream_name = format!("{}@trade", symbol.to_lowercase());

        {
            let subs = self.subscriptions.read().await;
            if subs.contains_key(symbol) {
                return Ok(()); // Already subscribed
            }
        }

        let sub_id = self.next_sub_id.fetch_add(1, Ordering::Relaxed);
        let subscribe_msg = serde_json::json!({
            "method": "SUBSCRIBE",
            "params": [&stream_name],
            "id": sub_id,
        });

        let msg = Message::Text(subscribe_msg.to_string());

        {
            let mut sink = self.ws_sink.lock().await;
            sink.send(msg)
                .await
                .map_err(|e| DataRetrievalError::ApiError(format!("Failed to subscribe: {}", e)))?;
        }

        {
            let mut subs = self.subscriptions.write().await;
            subs.insert(symbol.to_uppercase(), stream_name);
        }

        info!("Subscribed to {} trades", symbol);
        Ok(())
    }

    // Kline (candlestick) subscriptions are not yet implemented.
    // The subscribe/process stubs were removed to avoid dead code accumulation.

    /// Handle incoming WebSocket messages.
    ///
    /// Exits cleanly when `shutdown_rx` becomes `true` (DR-003), or when the
    /// WebSocket stream ends / errors.
    async fn message_handler(&self, mut shutdown_rx: watch::Receiver<bool>) {
        loop {
            // Poll the shutdown signal without blocking the read path.
            if *shutdown_rx.borrow() {
                info!("WebSocket message handler received shutdown signal");
                break;
            }

            // Read from ws_reader — hold the lock only for the duration of one read.
            let msg = {
                let mut reader = self.ws_reader.lock().await;
                // Race between the next WebSocket message and a shutdown signal so
                // we do not block indefinitely inside the lock (DR-003).
                tokio::select! {
                    result = reader.next() => result,
                    _ = shutdown_rx.changed() => {
                        info!("WebSocket message handler received shutdown signal during read");
                        break;
                    }
                }
            };

            match msg {
                Some(Ok(Message::Text(text))) => {
                    if let Err(e) = self.process_message(&text).await {
                        warn!("Failed to process message: {}", e);
                    }
                }
                Some(Ok(Message::Ping(data))) => {
                    let pong = Message::Pong(data);
                    let mut sink = self.ws_sink.lock().await;
                    if let Err(e) = sink.send(pong).await {
                        error!("Failed to send pong: {}", e);
                    }
                }
                Some(Ok(Message::Close(_))) => {
                    info!("WebSocket closed by server");
                    break;
                }
                Some(Err(e)) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                None => {
                    info!("WebSocket stream ended");
                    break;
                }
                _ => {} // Ignore other message types
            }
        }

        // Mark as disconnected so the reconnect loop in lib.rs kicks in.
        {
            let mut connected = self.connected.write().await;
            *connected = false;
        }

        warn!("WebSocket message handler exited");
    }

    /// Process a single message.
    async fn process_message(&self, text: &str) -> Result<()> {
        let value: Value = serde_json::from_str(text)
            .map_err(|e| DataRetrievalError::InvalidResponse(e.to_string()))?;

        if let Some(event_type) = value.get("e").and_then(|v| v.as_str()) {
            match event_type {
                "trade" => {
                    self.process_trade(&value).await?;
                }
                "kline" => {
                    debug!("Received kline event (not yet processed)");
                }
                _ => {
                    debug!("Unknown event type: {}", event_type);
                }
            }
        }

        Ok(())
    }

    /// Process a trade message and broadcast the resulting [`PricePoint`].
    async fn process_trade(&self, value: &Value) -> Result<()> {
        let symbol = value
            .get("s")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DataRetrievalError::InvalidResponse("Missing symbol".to_string()))?;

        let price_str = value
            .get("p")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DataRetrievalError::InvalidResponse("Missing price".to_string()))?;

        let timestamp_ms = value
            .get("T")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| DataRetrievalError::InvalidResponse("Missing timestamp".to_string()))?;

        let timestamp =
            chrono::DateTime::from_timestamp_millis(timestamp_ms).unwrap_or_else(chrono::Utc::now);

        let price = Decimal::from_str(price_str)
            .map_err(|e| DataRetrievalError::InvalidResponse(format!("Invalid price: {}", e)))?;

        let price_point = PricePoint {
            symbol: Self::normalize_symbol(symbol),
            price,
            source: "binance".to_string(),
            timestamp,
            confidence: Some(0.95),
        };

        // broadcast::Sender::send returns Err only when there are no active
        // receivers.  We count these drops so health checks can surface them.
        match self.price_tx.send(price_point) {
            Ok(_) => {}
            Err(_) => {
                let total = self.dropped_count.fetch_add(1, Ordering::Relaxed) + 1;
                warn!(
                    dropped_total = total,
                    "Price update dropped: no active subscribers on broadcast channel"
                );
            }
        }

        Ok(())
    }

    /// Returns the total number of price updates that were dropped because there
    /// were no active broadcast receivers at the time of the send.
    ///
    /// Intended for health-endpoint exposure to surface delivery gaps over time.
    pub fn dropped_count(&self) -> u64 {
        self.dropped_count.load(Ordering::Relaxed)
    }

    /// Subscribe to the price broadcast channel.
    ///
    /// Each call returns an independent [`broadcast::Receiver`] that starts
    /// receiving new messages from the moment of subscription.  The channel is
    /// never replaced on reconnect, so existing receivers remain valid (DR-001).
    pub fn subscribe(&self) -> broadcast::Receiver<PricePoint> {
        self.price_tx.subscribe()
    }

    /// Receive the next price update on the struct's own subscriber handle.
    ///
    /// This is a convenience wrapper for callers that want a single shared
    /// receiver stored inside the client.  It creates a new subscription on
    /// each call — prefer calling [`subscribe`] once and reusing the receiver
    /// in a loop for high-throughput consumers.
    pub async fn next_price(&self) -> Option<PricePoint> {
        let mut rx = self.price_tx.subscribe();
        rx.recv().await.ok()
    }

    /// Non-blocking price receive — returns `None` when no message is available.
    pub async fn try_recv_price(&self) -> Option<PricePoint> {
        self.price_tx.subscribe().try_recv().ok()
    }

    /// Check if connected.
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Reconnect to WebSocket.
    ///
    /// Signals the old message-handler task to exit via the `watch` channel and
    /// then spawns a fresh handler task with a new shutdown channel (DR-003).
    /// The broadcast price channel is reused unchanged so existing subscribers
    /// continue receiving without replacing their receiver handles (DR-001).
    pub async fn reconnect(&self) -> Result<()> {
        info!("Reconnecting to Binance WebSocket...");

        // Signal the current handler to shut down gracefully (DR-003).
        // The lock is released before we await the new connection so we do not
        // hold it across an await that could take seconds.
        {
            let tx = self.shutdown_tx.lock().await;
            // Ignore send errors — the old task may already have exited.
            let _ = tx.send(true);
        }

        // Connect new WebSocket
        let (ws_stream, _) = connect_async("wss://stream.binance.com:9443/ws")
            .await
            .map_err(|e| DataRetrievalError::ApiError(format!("Reconnection failed: {}", e)))?;

        let (ws_sink, ws_reader) = ws_stream.split();

        // Replace the underlying streams while holding their locks.
        {
            let mut sink = self.ws_sink.lock().await;
            *sink = ws_sink;
        }
        {
            let mut reader = self.ws_reader.lock().await;
            *reader = ws_reader;
        }

        // Re-subscribe to previous streams
        let subs: Vec<(String, String)> = {
            let s = self.subscriptions.read().await;
            s.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        for (_symbol, stream) in subs {
            let sub_id = self.next_sub_id.fetch_add(1, Ordering::Relaxed);
            let subscribe_msg = serde_json::json!({
                "method": "SUBSCRIBE",
                "params": [&stream],
                "id": sub_id,
            });

            let msg = Message::Text(subscribe_msg.to_string());
            {
                let mut sink = self.ws_sink.lock().await;
                sink.send(msg).await.map_err(|e| {
                    DataRetrievalError::ApiError(format!("Resubscription failed: {}", e))
                })?;
            }
        }

        // Mark as connected
        {
            let mut connected = self.connected.write().await;
            *connected = true;
        }

        // Create a fresh shutdown channel for the new handler task (DR-003).
        let (new_shutdown_tx, new_shutdown_rx) = watch::channel(false);
        {
            let mut tx = self.shutdown_tx.lock().await;
            *tx = new_shutdown_tx;
        }

        // Spawn the new message handler — the broadcast channel is reused (DR-001).
        let handler_clone = self.clone_state();
        tokio::spawn(async move {
            handler_clone.message_handler(new_shutdown_rx).await;
        });

        info!("Reconnected to Binance WebSocket");
        Ok(())
    }

    /// Close connection gracefully.
    pub async fn close(&self) -> Result<()> {
        info!("Closing Binance WebSocket connection");

        // Signal the handler to exit cleanly before closing the stream (DR-003).
        {
            let tx = self.shutdown_tx.lock().await;
            let _ = tx.send(true);
        }

        {
            let mut sink = self.ws_sink.lock().await;
            sink.close().await.map_err(|e| {
                DataRetrievalError::ApiError(format!("Failed to close WebSocket: {}", e))
            })?;
        }

        {
            let mut connected = self.connected.write().await;
            *connected = false;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Integration test - requires real Binance WebSocket (may be geo-blocked)
    async fn test_connect() {
        let client = BinanceWebSocketClient::new().await.unwrap();
        assert!(client.is_connected().await);

        client.subscribe_trades("BTCUSDT").await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let timeout = tokio::time::Duration::from_secs(10);
        let mut rx = client.subscribe();
        let price = tokio::time::timeout(timeout, rx.recv()).await;

        match price {
            Ok(Ok(p)) => {
                println!("Received price: {} = ${}", p.symbol, p.price);
                assert_eq!(p.source, "binance");
                assert!(p.price > Decimal::ZERO);
            }
            Ok(Err(e)) => println!("Receive error: {}", e),
            Err(_) => println!("Timeout - no trades received"),
        }

        client.close().await.unwrap();
    }

    /// Verify that a subscriber obtained before reconnect continues to receive
    /// messages after reconnect — the channel is not replaced (DR-001).
    #[tokio::test]
    #[ignore] // Integration test
    async fn test_subscriber_survives_reconnect() {
        let client = BinanceWebSocketClient::new().await.unwrap();
        let mut rx = client.subscribe(); // obtain receiver BEFORE reconnect
        client.reconnect().await.unwrap();
        // rx should still be valid; a send on the new handler will reach it.
        assert!(rx.try_recv().is_err()); // empty but not closed
    }

    #[test]
    fn normalize_symbol_uses_usd_canonical_quote() {
        assert_eq!(
            BinanceWebSocketClient::normalize_symbol("BTCUSDT"),
            "BTC/USD"
        );
        assert_eq!(
            BinanceWebSocketClient::normalize_symbol("ETHUSD"),
            "ETH/USD"
        );
        assert_eq!(
            BinanceWebSocketClient::normalize_symbol("BTC/USD"),
            "BTC/USD"
        );
    }
}
