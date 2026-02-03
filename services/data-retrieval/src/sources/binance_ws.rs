use crate::types::*;
use futures::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, warn};

/// Binance WebSocket client for real-time price feeds
pub struct BinanceWebSocketClient {
    /// WebSocket stream
    ws_stream: Arc<Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
    /// Channel for receiving price updates
    price_tx: mpsc::Sender<PricePoint>,
    price_rx: Arc<Mutex<mpsc::Receiver<PricePoint>>>,
    /// Subscribed streams
    subscriptions: Arc<RwLock<HashMap<String, String>>>, // symbol -> stream_name
    /// Connection status
    connected: Arc<RwLock<bool>>,
}

impl BinanceWebSocketClient {
    /// Connect to Binance combined stream WebSocket
    pub async fn new() -> Result<Self> {
        // Binance WebSocket URL for combined streams
        let url = "wss://stream.binance.com:9443/ws";
        
        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| DataRetrievalError::ApiError(format!(
                "WebSocket connection failed: {}", e
            )))?;
        
        info!("Connected to Binance WebSocket");
        
        let (price_tx, price_rx) = mpsc::channel(100);
        
        let client = Self {
            ws_stream: Arc::new(Mutex::new(ws_stream)),
            price_tx,
            price_rx: Arc::new(Mutex::new(price_rx)),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            connected: Arc::new(RwLock::new(true)),
        };
        
        // Spawn message handler
        let client_clone = client.clone();
        tokio::spawn(async move {
            client_clone.message_handler().await;
        });
        
        Ok(client)
    }
    
    /// Clone for spawning tasks
    fn clone(&self) -> Self {
        Self {
            ws_stream: Arc::clone(&self.ws_stream),
            price_tx: self.price_tx.clone(),
            price_rx: Arc::clone(&self.price_rx),
            subscriptions: Arc::clone(&self.subscriptions),
            connected: Arc::clone(&self.connected),
        }
    }
    
    /// Subscribe to real-time trades for a symbol
    pub async fn subscribe_trades(&self,
        symbol: &str,
    ) -> Result<()> {
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
        
        {
            let mut ws = self.ws_stream.lock().await;
            ws.send(msg).await.map_err(|e| {
                DataRetrievalError::ApiError(format!(
                    "Failed to subscribe: {}", e
                ))
            })?;
        }
        
        {
            let mut subs = self.subscriptions.write().await;
            subs.insert(symbol.to_uppercase(), stream_name);
        }
        
        info!("Subscribed to {} trades", symbol);
        Ok(())
    }
    
    /// Subscribe to 1-minute kline (candlestick) updates
    pub async fn subscribe_klines(
        &self,
        symbol: &str,
        interval: &str, // "1m", "5m", "15m", "1h", "4h", "1d"
    ) -> Result<()> {
        let stream_name = format!("{}@kline_{}", symbol.to_lowercase(), interval);
        
        {
            let subs = self.subscriptions.read().await;
            if subs.values().any(|v| v == &stream_name) {
                return Ok(()); // Already subscribed
            }
        }
        
        let subscribe_msg = serde_json::json!({
            "method": "SUBSCRIBE",
            "params": [&stream_name],
            "id": 2,
        });
        
        let msg = Message::Text(subscribe_msg.to_string());
        
        {
            let mut ws = self.ws_stream.lock().await;
            ws.send(msg).await.map_err(|e| {
                DataRetrievalError::ApiError(format!(
                    "Failed to subscribe to klines: {}", e
                ))
            })?;
        }
        
        {
            let mut subs = self.subscriptions.write().await;
            subs.insert(format!("{}_kline_{}", symbol, interval), stream_name);
        }
        
        info!("Subscribed to {} {} klines", symbol, interval);
        Ok(())
    }
    
    /// Handle incoming WebSocket messages
    async fn message_handler(&self,
    ) {
        loop {
            let msg = {
                let mut ws = self.ws_stream.lock().await;
                ws.next().await
            };
            
            match msg {
                Some(Ok(Message::Text(text))) => {
                    if let Err(e) = self.process_message(&text).await {
                        warn!("Failed to process message: {}", e);
                    }
                }
                Some(Ok(Message::Ping(data))) => {
                    // Send pong
                    let pong = Message::Pong(data);
                    let mut ws = self.ws_stream.lock().await;
                    if let Err(e) = ws.send(pong).await {
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
        
        warn!("WebSocket message handler exited");
    }
    
    /// Process a single message
    async fn process_message(
        &self,
        text: &str,
    ) -> Result<()> {
        let value: Value = serde_json::from_str(text)
            .map_err(|e| DataRetrievalError::InvalidResponse(e.to_string()))?;
        
        // Check if it's a trade or kline message
        if let Some(event_type) = value.get("e").and_then(|v| v.as_str()) {
            match event_type {
                "trade" => {
                    self.process_trade(&value).await?;
                }
                "kline" => {
                    self.process_kline(&value).await?;
                }
                _ => {
                    debug!("Unknown event type: {}", event_type);
                }
            }
        }
        
        Ok(())
    }
    
    /// Process trade message
    async fn process_trade(
        &self,
        value: &Value,
    ) -> Result<()> {
        let symbol = value.get("s")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DataRetrievalError::InvalidResponse(
                "Missing symbol".to_string()
            ))?;
        
        let price_str = value.get("p")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DataRetrievalError::InvalidResponse(
                "Missing price".to_string()
            ))?;
        
        let timestamp_ms = value.get("T")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| DataRetrievalError::InvalidResponse(
                "Missing timestamp".to_string()
            ))?;
        
        let price = price_str.parse::<f64>()
            .map_err(|e| DataRetrievalError::InvalidResponse(e.to_string()))?;
        
        let price_point = PricePoint {
            asset: symbol.replace("USDT", "").replace("USD", ""),
            quote: "USD".to_string(),
            source: "binance".to_string(),
            timestamp: chrono::Utc.timestamp_millis_opt(timestamp_ms).single()
                .unwrap_or_else(chrono::Utc::now),
            price: Decimal::try_from(price)
                .map_err(|e| DataRetrievalError::InvalidResponse(e.to_string()))?,
            volume_24h: None,
            market_cap: None,
            confidence: 0.95, // Binance is real-time exchange data
        };
        
        // Send to channel
        if let Err(e) = self.price_tx.send(price_point).await {
            warn!("Failed to send price update: {}", e);
        }
        
        Ok(())
    }
    
    /// Process kline (candlestick) message
    async fn process_kline(
        &self,
        value: &Value,
    ) -> Result<()> {
        // Kline messages have a nested "k" object
        let kline = value.get("k")
            .ok_or_else(|| DataRetrievalError::InvalidResponse(
                "Missing kline data".to_string()
            ))?;
        
        let symbol = value.get("s")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN");
        
        let is_closed = kline.get("x")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        // Only process closed candles for OHLCV
        if !is_closed {
            return Ok(());
        }
        
        debug!("Kline closed for {}: {:?}", symbol, kline);
        
        // TODO: Send to candle channel for aggregation
        
        Ok(())
    }
    
    /// Receive the next price update
    pub async fn next_price(&self,
    ) -> Option<PricePoint> {
        let mut rx = self.price_rx.lock().await;
        rx.recv().await
    }
    
    /// Get latest price for a symbol (non-blocking)
    pub async fn try_recv_price(&self,
    ) -> Option<PricePoint> {
        let mut rx = self.price_rx.lock().await;
        rx.try_recv().ok()
    }
    
    /// Check if connected
    pub async fn is_connected(&self,
    ) -> bool {
        *self.connected.read().await
    }
    
    /// Reconnect to WebSocket
    pub async fn reconnect(&mut self,
    ) -> Result<()> {
        info!("Reconnecting to Binance WebSocket...");
        
        // Connect new WebSocket
        let (ws_stream, _) = connect_async("wss://stream.binance.com:9443/ws")
            .await
            .map_err(|e| DataRetrievalError::ApiError(format!(
                "Reconnection failed: {}", e
            )))?;
        
        // Replace old stream
        {
            let mut ws = self.ws_stream.lock().await;
            *ws = ws_stream;
        }
        
        // Resubscribe to previous streams
        let subs: Vec<(String, String)> = {
            let s = self.subscriptions.read().await;
            s.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };
        
        for (symbol, stream) in subs {
            // Re-subscribe
            let subscribe_msg = serde_json::json!({
                "method": "SUBSCRIBE",
                "params": [&stream],
                "id": 1,
            });
            
            let msg = Message::Text(subscribe_msg.to_string());
            {
                let mut ws = self.ws_stream.lock().await;
                ws.send(msg).await.map_err(|e| {
                    DataRetrievalError::ApiError(format!(
                        "Resubscription failed: {}", e
                    ))
                })?;
            }
        }
        
        // Mark as connected
        {
            let mut connected = self.connected.write().await;
            *connected = true;
        }
        
        // Restart message handler
        let client_clone = self.clone();
        tokio::spawn(async move {
            client_clone.message_handler().await;
        });
        
        info!("Reconnected to Binance WebSocket");
        Ok(())
    }
    
    /// Close connection gracefully
    pub async fn close(&self,
    ) -> Result<()> {
        info!("Closing Binance WebSocket connection");
        
        {
            let mut ws = self.ws_stream.lock().await;
            ws.close(None).await.map_err(|e| {
                DataRetrievalError::ApiError(format!(
                    "Failed to close WebSocket: {}", e
                ))
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
                println!("Received price: {} = ${}", p.asset, p.price);
                assert_eq!(p.source, "binance");
                assert!(p.price > Decimal::ZERO);
            }
            Ok(None) => println!("Channel closed"),
            Err(_) => println!("Timeout - no trades received"),
        }
        
        client.close().await.unwrap();
    }
}
