use crate::types::*;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, warn};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsSink = SplitSink<WsStream, Message>;
type WsReader = SplitStream<WsStream>;

/// Binance WebSocket client for real-time price feeds
///
/// Uses split read/write channels to prevent deadlock between
/// sending subscriptions and receiving messages.
pub struct BinanceWebSocketClient {
    /// WebSocket write half (for sending subscriptions)
    ws_sink: Arc<Mutex<WsSink>>,
    /// WebSocket read half (for receiving messages)
    ws_reader: Arc<Mutex<WsReader>>,
    /// Channel sender — wrapped in Option so the message handler can drop it on
    /// disconnect, which causes `price_rx.recv()` to return `None` (DR-012).
    price_tx: Arc<Mutex<Option<mpsc::Sender<PricePoint>>>>,
    price_rx: Arc<Mutex<mpsc::Receiver<PricePoint>>>,
    /// Subscribed streams
    subscriptions: Arc<RwLock<HashMap<String, String>>>, // symbol -> stream_name
    /// Connection status
    connected: Arc<RwLock<bool>>,
    /// Handle to the running message-handler task.
    /// Stored so reconnect can abort the old task before spawning a new one (DR-003).
    handler_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl BinanceWebSocketClient {
    /// Connect to Binance combined stream WebSocket
    pub async fn new() -> Result<Self> {
        // Binance WebSocket URL for combined streams
        let url = "wss://stream.binance.com:9443/ws";

        let (ws_stream, _) = connect_async(url).await.map_err(|e| {
            DataRetrievalError::ApiError(format!("WebSocket connection failed: {}", e))
        })?;

        info!("Connected to Binance WebSocket");

        // Split into read/write halves to prevent deadlock
        let (ws_sink, ws_reader) = ws_stream.split();

        // Larger buffer to prevent data loss during price spikes
        // 10k entries = ~10 seconds of high-volume crypto trading
        let (price_tx, price_rx) = mpsc::channel(10000);

        let client = Self {
            ws_sink: Arc::new(Mutex::new(ws_sink)),
            ws_reader: Arc::new(Mutex::new(ws_reader)),
            price_tx: Arc::new(Mutex::new(Some(price_tx))),
            price_rx: Arc::new(Mutex::new(price_rx)),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            connected: Arc::new(RwLock::new(true)),
            handler_task: Arc::new(Mutex::new(None)),
        };

        // Spawn message handler and store the handle
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            client_clone.message_handler().await;
        });
        *client.handler_task.lock().await = Some(handle);

        Ok(client)
    }

    /// Clone for spawning tasks
    fn clone(&self) -> Self {
        Self {
            ws_sink: Arc::clone(&self.ws_sink),
            ws_reader: Arc::clone(&self.ws_reader),
            price_tx: Arc::clone(&self.price_tx),
            price_rx: Arc::clone(&self.price_rx),
            subscriptions: Arc::clone(&self.subscriptions),
            connected: Arc::clone(&self.connected),
            handler_task: Arc::clone(&self.handler_task),
        }
    }

    /// Subscribe to real-time trades for a symbol
    pub async fn subscribe_trades(&self, symbol: &str) -> Result<()> {
        let stream_name = format!("{}@trade", symbol.to_lowercase());

        {
            let subs = self.subscriptions.read().await;
            if subs.contains_key(symbol) {
                return Ok(()); // Already subscribed
            }
        }

        let subscribe_msg = serde_json::json!({
            "method": "SUBSCRIBE",
            "params": [&stream_name],
            "id": 1,
        });

        let msg = Message::Text(subscribe_msg.to_string());

        // Use ws_sink (write half) - doesn't block message handler
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

    /// Handle incoming WebSocket messages
    ///
    /// Uses the read half (ws_reader) so subscriptions can be sent
    /// concurrently without blocking.
    async fn message_handler(&self) {
        loop {
            // Read from ws_reader (read half) - doesn't block subscriptions
            let msg = {
                let mut reader = self.ws_reader.lock().await;
                reader.next().await
            };

            match msg {
                Some(Ok(Message::Text(text))) => {
                    if let Err(e) = self.process_message(&text).await {
                        warn!("Failed to process message: {}", e);
                    }
                }
                Some(Ok(Message::Ping(data))) => {
                    // Send pong using write half
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

        // Mark as disconnected
        {
            let mut connected = self.connected.write().await;
            *connected = false;
        }

        // Drop the sender so any caller blocked on price_rx.recv() gets None (DR-012).
        {
            let mut tx = self.price_tx.lock().await;
            *tx = None;
        }

        warn!("WebSocket message handler exited");
    }

    /// Process a single message
    async fn process_message(&self, text: &str) -> Result<()> {
        let value: Value = serde_json::from_str(text)
            .map_err(|e| DataRetrievalError::InvalidResponse(e.to_string()))?;

        // Check if it's a trade or kline message
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

    /// Process trade message
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

        // Convert millisecond timestamp to DateTime<Utc>
        let timestamp =
            chrono::DateTime::from_timestamp_millis(timestamp_ms).unwrap_or_else(chrono::Utc::now);

        // Parse directly to Decimal to avoid f64 precision loss
        let price = Decimal::from_str(price_str)
            .map_err(|e| DataRetrievalError::InvalidResponse(format!("Invalid price: {}", e)))?;

        // Format symbol as BTC/USDT from BTCUSDT
        let formatted_symbol = if let Some(base) = symbol.strip_suffix("USDT") {
            format!("{}/USDT", base)
        } else if let Some(base) = symbol.strip_suffix("USD") {
            format!("{}/USD", base)
        } else {
            symbol.to_string()
        };

        let price_point = PricePoint {
            symbol: formatted_symbol,
            price,
            source: "binance".to_string(),
            timestamp,
            confidence: Some(0.95), // Binance is real-time exchange data
        };

        // Non-blocking send — drop update rather than stalling the message handler
        let tx_guard = self.price_tx.lock().await;
        if let Some(tx) = tx_guard.as_ref() {
            match tx.try_send(price_point) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    warn!("Price channel full (10k entries). Dropping update — consumer may be behind.");
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    warn!("Price channel closed. Consumer disconnected.");
                }
            }
        }

        Ok(())
    }

    /// Receive the next price update
    pub async fn next_price(&self) -> Option<PricePoint> {
        let mut rx = self.price_rx.lock().await;
        rx.recv().await
    }

    /// Get latest price for a symbol (non-blocking)
    pub async fn try_recv_price(&self) -> Option<PricePoint> {
        let mut rx = self.price_rx.lock().await;
        rx.try_recv().ok()
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Reconnect to WebSocket
    ///
    /// Uses interior mutability (Arc<Mutex>) so this can be called from shared references.
    pub async fn reconnect(&self) -> Result<()> {
        info!("Reconnecting to Binance WebSocket...");

        // Connect new WebSocket
        let (ws_stream, _) = connect_async("wss://stream.binance.com:9443/ws")
            .await
            .map_err(|e| DataRetrievalError::ApiError(format!("Reconnection failed: {}", e)))?;

        // Split into read/write halves
        let (ws_sink, ws_reader) = ws_stream.split();

        // Replace old streams
        {
            let mut sink = self.ws_sink.lock().await;
            *sink = ws_sink;
        }
        {
            let mut reader = self.ws_reader.lock().await;
            *reader = ws_reader;
        }

        // Resubscribe to previous streams
        let subs: Vec<(String, String)> = {
            let s = self.subscriptions.read().await;
            s.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        for (_symbol, stream) in subs {
            // Re-subscribe using write half
            let subscribe_msg = serde_json::json!({
                "method": "SUBSCRIBE",
                "params": [&stream],
                "id": 1,
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

        // Restore the sender so process_trade can forward messages again (DR-012).
        // The receiver end is unchanged — callers continue to hold the same price_rx.
        {
            let mut tx = self.price_tx.lock().await;
            // Only create a fresh channel when the old sender was dropped.
            // If it is still present the existing channel is reusable.
            if tx.is_none() {
                let (new_tx, new_rx) = mpsc::channel(10000);
                *tx = Some(new_tx);
                *self.price_rx.lock().await = new_rx;
            }
        }

        // Abort the previous message-handler task before starting a new one (DR-003).
        {
            let mut task = self.handler_task.lock().await;
            if let Some(old_handle) = task.take() {
                old_handle.abort();
            }
        }

        // Restart message handler and store the new handle
        let client_clone = self.clone();
        let handle = tokio::spawn(async move {
            client_clone.message_handler().await;
        });
        *self.handler_task.lock().await = Some(handle);

        info!("Reconnected to Binance WebSocket");
        Ok(())
    }

    /// Close connection gracefully
    pub async fn close(&self) -> Result<()> {
        info!("Closing Binance WebSocket connection");

        // Close via write half
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

        // Subscribe to BTC
        client.subscribe_trades("BTCUSDT").await.unwrap();

        // Wait a bit for connection
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Try to receive a trade
        let timeout = tokio::time::Duration::from_secs(10);
        let price = tokio::time::timeout(timeout, client.next_price()).await;

        match price {
            Ok(Some(p)) => {
                println!("Received price: {} = ${}", p.symbol, p.price);
                assert_eq!(p.source, "binance");
                assert!(p.price > Decimal::ZERO);
            }
            Ok(None) => println!("Channel closed"),
            Err(_) => println!("Timeout - no trades received"),
        }

        client.close().await.unwrap();
    }
}
