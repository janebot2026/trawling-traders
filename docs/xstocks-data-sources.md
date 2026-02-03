# Data Sources for xStocks (Tokenized Stocks on Solana)

## The Problem

We need price feeds for tokenized stocks on Solana (xStocks like AAPL, TSLA, SPY) and tokenized metals (ORO for gold).

Regular crypto price APIs (CoinGecko, Binance) don't cover these Solana-specific assets well.

## Potential Sources

### 1. Pyth Network (Recommended)
- **What**: Institutional-grade oracle network
- **Coverage**: 400+ price feeds including stocks, FX, commodities
- **Latency**: ~400ms updates
- **Cost**: Free (with rate limits)
- **Solana Native**: Yes, built for Solana
- **Docs**: https://docs.pyth.network/

**API Example**:
```
https://hermes.pyth.network/v2/updates/price/latest?ids[]=0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43
```

**Stock Feeds Available**:
- AAPL/USD
- TSLA/USD
- GOOGL/USD
- AMZN/USD
- MSFT/USD
- NVDA/USD
- META/USD
- SPY (S&P 500 ETF)
- QQQ (Nasdaq ETF)
- And more...

### 2. Switchboard
- **What**: Permissionless oracle network
- **Coverage**: Custom feeds, community-driven
- **Solana Native**: Yes
- **Cost**: Pay per request

### 3. Chainlink (on Solana)
- **What**: Industry standard oracle
- **Coverage**: Limited on Solana vs Ethereum
- **Solana Native**: Yes, but fewer feeds

### 4. Direct xStocks Integration
- **What**: If xStocks protocol exposes their own price oracle
- **Pros**: Direct from source
- **Cons**: Single point of failure

## Recommendation

**Primary**: Pyth Network
- Best coverage for stocks/metals
- Solana-native
- Free tier sufficient for our use case
- Sub-500ms latency

**Fallback**: Switchboard for assets Pyth doesn't cover

## Implementation Plan

1. Add `PythClient` to data-retrieval service
2. Map common stocks to Pyth feed IDs
3. Cache prices in Redis (5-10 second TTL)
4. Use same aggregation logic as crypto

## Pyth Feed IDs (Sample)

| Asset | Pyth Feed ID |
|-------|--------------|
| AAPL | e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43 |
| TSLA | c5e0a3cbf1fc4e49bf7fbdc6a5009d8c98a7d9ba2d293b8ab0f0e8a1c4e3d8f9 |
| GOOGL | (lookup required) |
| ORO (Gold) | (lookup required) |

Full list: https://pyth.network/price-feeds
