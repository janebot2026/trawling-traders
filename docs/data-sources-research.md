# Financial Data Sources Research

*Research task: Identify free/cheap high-quality financial data feeds for algorithm building*

## Priority: High-Quality Free Sources

### 1. Cryptocurrency Data (Primary Focus)

#### CoinGecko API ⭐ TOP PRIORITY
- **URL:** https://www.coingecko.com/en/api
- **Free Tier:** 10-30 calls/minute, 10,000+ coins
- **Data:** Price, volume, market cap, OHLC, historical
- **Pros:** No API key required for basic, comprehensive coverage
- **Cons:** Rate limits on free tier
- **Use Case:** Real-time prices, historical backtesting, market cap rankings
- **Integration:** REST API, simple JSON responses

#### CoinMarketCap API
- **URL:** https://coinmarketcap.com/api/
- **Free Tier:** 10,000 calls/month
- **Data:** Price, market data, exchange volumes
- **Pros:** Industry standard, reliable
- **Cons:** Requires API key, stricter limits than CoinGecko
- **Use Case:** Secondary source for validation

#### Binance API ⭐ HIGH PRIORITY
- **URL:** https://binance-docs.github.io/apidocs/
- **Free Tier:** 1200 weight/minute (generous)
- **Data:** Real-time trades, order book (L1/L2), OHLCV (Klines), funding rates
- **Pros:** Exchange-native data, low latency, futures data
- **Cons:** Limited to Binance-listed assets
- **Use Case:** Live trading signals, order flow analysis, futures data
- **Integration:** REST + WebSocket for real-time

#### Coinbase Pro API (Coinbase Advanced Trade)
- **URL:** https://docs.cloud.coinbase.com/advanced-trade-api/docs/welcome
- **Free Tier:** Generous limits
- **Data:** Prices, order book, trades
- **Pros:** US-regulated, high reliability
- **Cons:** Limited coin selection vs Binance
- **Use Case:** US-focused trading, regulatory compliance

### 2. Traditional Finance (Forex, Stocks, Commodities)

#### Alpha Vantage ⭐ RECOMMENDED
- **URL:** https://www.alphavantage.co/
- **Free Tier:** 25 API calls/day
- **Data:** Stocks, forex, crypto, technical indicators
- **Pros:** Clean API, technical indicators built-in
- **Cons:** Very low free tier limit
- **Use Case:** Stock signals, forex pairs, technical analysis

#### Yahoo Finance (unofficial APIs)
- **URL:** https://github.com/ranaroussi/yfinance (Python lib)
- **Free:** Yes (scraping-based)
- **Data:** Stocks, ETFs, forex, historical
- **Pros:** Extensive coverage, free
- **Cons:** Unofficial, can break, TOS concerns
- **Use Case:** Research only, not for production trading

#### Twelve Data
- **URL:** https://twelvedata.com/
- **Free Tier:** 800 API calls/day
- **Data:** Stocks, forex, crypto, ETFs
- **Pros:** Real-time WebSocket, good coverage
- **Cons:** Lower limits than some alternatives
- **Use Case:** Real-time equity data

#### Finnhub
- **URL:** https://finnhub.io/
- **Free Tier:** 60 API calls/minute
- **Data:** Real-time US stocks, forex, crypto
- **Pros:** WebSocket support, fundamentals
- **Cons:** Limited historical on free tier
- **Use Case:** US equity real-time data

### 3. On-Chain Data (Crypto Analytics)

#### Glassnode ⭐ HIGH VALUE
- **URL:** https://glassnode.com/
- **Free Tier:** Limited metrics, 30-day history
- **Data:** On-chain metrics (exchange flows, holder behavior, network activity)
- **Pros:** Unique insights, predictive signals
- **Cons:** Expensive paid tiers, limited free data
- **Use Case:** Whale watching, market sentiment, long-term signals

#### CryptoQuant
- **URL:** https://cryptoquant.com/
- **Free Tier:** Limited metrics
- **Data:** Exchange flows, miner flows, stablecoin flows
- **Pros:** Good exchange flow data
- **Cons:** Limited free tier
- **Use Case:** Exchange inflow/outflow signals

#### Santiment
- **URL:** https://santiment.net/
- **Free Tier:** Some metrics, delayed data
- **Data:** Social sentiment, on-chain, development activity
- **Pros:** Sentiment analysis unique
- **Cons:** Expensive paid plans
- **Use Case:** Sentiment-based signals

#### Dune Analytics (Query-based)
- **URL:** https://dune.com/
- **Free Tier:** Create dashboards, query public data
- **Data:** Custom on-chain queries
- **Pros:** Extremely flexible, community dashboards
- **Cons:** Requires SQL knowledge, not real-time API
- **Use Case:** Custom metrics, research

### 4. Alternative Data / Sentiment

#### LunarCrush ⭐ SOCIAL SENTIMENT
- **URL:** https://lunarcrush.com/
- **Free Tier:** Limited social data
- **Data:** Social mentions, engagement, sentiment for crypto
- **Pros:** Unique social signals
- **Cons:** Limited free tier
- **Use Case:** Hype detection, contrarian signals

#### The Graph (GRT)
- **URL:** https://thegraph.com/
- **Free Tier:** Query decentralized subgraphs
- **Data:** DeFi protocol data, custom blockchain queries
- **Pros:** Decentralized, DeFi-focused
- **Cons:** Complex GraphQL, requires subgraph knowledge
- **Use Case:** DeFi-specific strategies

### 5. Economic / Macro Data

#### FRED (Federal Reserve Economic Data)
- **URL:** https://fred.stlouisfed.org/
- **Free:** Yes
- **Data:** Interest rates, inflation, employment, GDP
- **Pros:** Authoritative macro data
- **Cons:** Not real-time, crypto correlation limited
- **Use Case:** Macro regime detection, risk-off signals

#### TradingView (unofficial)
- **URL:** https://www.tradingview.com/
- **Free:** Charts are free, data via widgets
- **Data:** All markets, technical indicators
- **Pros:** Best charting, community scripts
- **Cons:** Not a data API directly
- **Use Case:** Visualization, strategy validation

## Data Retrieval Module Architecture (Proposed)

### Module Structure
```
data-retrieval/
├── src/
│   ├── sources/
│   │   ├── coingecko.rs       # Price, market data
│   │   ├── binance.rs         # Real-time trades, order book
│   │   ├── glassnode.rs       # On-chain metrics
│   │   ├── alphavantage.rs    # Stocks, forex
│   │   └── lunarcrush.rs      # Social sentiment
│   ├── normalizers/
│   │   ├── price.rs           # Normalize price feeds
│   │   ├── volume.rs          # Normalize volume
│   │   └── sentiment.rs       # Normalize sentiment scores
│   ├── aggregators/
│   │   ├── multi_source.rs    # Combine multiple feeds
│   │   └── confidence.rs      # Score data reliability
│   ├── cache/
│   │   └── redis.rs           # Hot cache for real-time
│   └── lib.rs
```

### Normalized Data Schema

```rust
// Universal price data point
pub struct PricePoint {
    pub asset: String,           // "BTC", "ETH", etc.
    pub quote: String,           // "USD", "USDT"
    pub source: String,          // "binance", "coingecko"
    pub timestamp: DateTime<Utc>,
    pub price: Decimal,
    pub volume_24h: Option<Decimal>,
    pub confidence: f64,         // 0.0 - 1.0 based on source quality
}

// OHLCV candle
pub struct Candle {
    pub asset: String,
    pub timeframe: TimeFrame,    // 1m, 5m, 15m, 1h, 4h, 1d
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub timestamp: DateTime<Utc>,
}

// On-chain metric
pub struct OnChainMetric {
    pub asset: String,
    pub metric_type: MetricType, // ExchangeFlow, MinerFlow, etc.
    pub value: f64,
    pub timestamp: DateTime<Utc>,
}

// Social sentiment
pub struct SentimentData {
    pub asset: String,
    pub platform: String,        // "twitter", "reddit", "lunarcrush"
    pub sentiment_score: f64,    // -1.0 to 1.0
    pub volume: u64,             // Mention count
    pub timestamp: DateTime<Utc>,
}
```

### Multi-Source Aggregation Strategy

1. **Primary Price Feed:** Binance (real-time, low latency)
2. **Validation Feed:** CoinGecko (price sanity checks)
3. **On-Chain Overlay:** Glassnode (exchange flows as signal)
4. **Sentiment Overlay:** LunarCrush (social momentum)

**Conflict Resolution:**
- If sources differ >1%, flag for review
- Use median of 3+ sources when available
- Confidence score weighted by source reliability

## Implementation Priority

### Phase 1 (MVP - Week 1)
- [ ] CoinGecko integration (prices, historical)
- [ ] Binance integration (real-time via WebSocket)
- [ ] Basic normalization layer
- [ ] Redis cache for hot data

### Phase 2 (Enhancement - Week 2)
- [ ] Glassnode on-chain metrics
- [ ] Multi-source aggregation
- [ ] Confidence scoring
- [ ] Data quality monitoring

### Phase 3 (Advanced - Week 3)
- [ ] LunarCrush sentiment
- [ ] Alpha Vantage (stocks/forex)
- [ ] Custom Dune queries
- [ ] Backfill historical data

## API Keys Needed

| Source | Free Tier | API Key Required | Priority |
|--------|-----------|------------------|----------|
| CoinGecko | 10-30/min | No | P0 |
| Binance | 1200 weight/min | No | P0 |
| Glassnode | Limited | Yes | P1 |
| Alpha Vantage | 25/day | Yes | P2 |
| LunarCrush | Limited | Yes | P2 |
| Finnhub | 60/min | Yes | P2 |

## Next Steps

1. **Start implementing:** CoinGecko price fetcher
2. **Add:** Binance WebSocket real-time feed
3. **Build:** Normalization layer (unify formats)
4. **Cache:** Redis for sub-100ms lookups
5. **Monitor:** Data quality, source health
6. **Document:** API response schemas, rate limits

---

*Research ongoing. Update as sources are integrated and tested.*
