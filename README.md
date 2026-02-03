# Trawling Traders

AI-powered trading bots on Solana. Quality assets only - xStocks, tokenized metals, crypto majors. Not a meme coin casino.

## Overview

Trawling Traders lets you spin up to 4 AI trading bots on your own DigitalOcean VPS. Each bot runs an OpenClaw agent with:
- **Quality asset focus**: Tokenized stocks (xStocks), metals (ORO), BTC/ETH/SOL
- **Professional algorithms**: Trend, mean reversion, breakout
- **Strict risk management**: Position limits, drawdown stops, paper trading
- **Full control**: Your VPS, your keys, your profits

## Quick Start

```bash
# 1. Clone and setup
git clone https://github.com/janebot2026/trawling-traders.git
cd trawling-traders

# 2. Start data retrieval service
cd services/data-retrieval
cargo run

# 3. Start control plane (new terminal)
cd services/control-plane
DATABASE_URL=postgres://localhost/trawling_traders cargo run

# 4. Start mobile app (new terminal)
cd apps/mobile
npm install
npm run ios  # or android
```

## Architecture

```
┌─────────────────┐
│  React Native   │  Mobile app for bot management
│     (iOS/Android) │
└────────┬────────┘
         │ HTTPS
         ▼
┌─────────────────┐
│  Control Plane  │  Rust/Axum API
│  (Rust/Axum)    │  - Bot CRUD
└────────┬────────┘  - Config versioning
         │           - Sync adapter
         ▼
┌─────────────────┐
│  Data Retrieval │  Rust price aggregation
│    (Rust)       │  - CoinGecko (REST)
└────────┬────────┘  - Binance (WebSocket)
         │           - xStocks/Metals (TBD)
         ▼
┌─────────────────┐
│  OpenClaw Bots  │  On DigitalOcean VPS
│   (per bot)     │  - Poll config
└─────────────────┘  - Execute trades on Solana
```

## Features

### Asset Classes
- **Crypto Majors**: BTC, ETH, SOL, etc.
- **Tokenized Stocks (xStocks)**: AAPL, TSLA, SPY, etc. on Solana
- **Tokenized Metals**: Gold, silver (ORO)
- **Custom baskets**: Build your own selection
- **Meme coins**: ⚠️ Explicit opt-in only, not recommended

### Trading Personas
- **Set & Forget**: Conservative defaults, paper trading, strict safety
- **Hands-on**: Tune xStocks exposure, risk controls, see trade reasons
- **Power User**: Signal knobs, custom baskets, full control

### Algorithms
- **Trend**: Ride momentum with confirmations
- **Mean Reversion**: Fade extremes, frequent trades
- **Breakout**: Volume-confirmed breakouts

### Risk Management
- Position sizing (% of portfolio)
- Max daily loss limits
- Max drawdown circuit breakers
- Max trades per day
- Paper trading mode (default)

## Project Structure

```
trawling-traders/
├── apps/
│   └── mobile/              # React Native app
│       ├── src/
│       │   ├── screens/     # 6 MVP screens
│       │   ├── components/  # AnimatedBotCard, etc.
│       │   ├── navigation/  # React Navigation
│       │   └── utils/       # Animations
│       └── package.json
├── services/
│   ├── control-plane/       # Rust API server
│   │   ├── src/
│   │   │   ├── handlers/    # HTTP handlers
│   │   │   ├── models/      # Database models
│   │   │   └── db/          # PostgreSQL
│   │   └── Cargo.toml
│   └── data-retrieval/      # Price aggregation
│       ├── src/
│       │   ├── sources/     # CoinGecko, Binance
│       │   ├── aggregators/ # Multi-source logic
│       │   └── cache/       # Redis
│       └── Cargo.toml
├── packages/
│   ├── types/               # Shared TypeScript types
│   └── api-client/          # API client
└── docs/
    ├── frontend-architecture.md
    ├── data-sources-research.md
    └── openclaw-integration.md
```

## Sync Adapter Pattern

No OpenClaw fork needed. Bots poll the control plane for config updates:

1. **Bot boots** → GET `/bot/:id/config` → downloads config + cron jobs
2. **Config changes** → User updates in app → new version created
3. **Bot polls** → Sees `version != applied_version` → downloads + applies
4. **Bot acks** → POST `/bot/:id/config_ack` → marked as synced

## Environment Variables

### Control Plane
```bash
DATABASE_URL=postgres://user:pass@localhost/trawling_traders
PORT=3000
# CEDROS_API_KEY=... (when available)
```

### Data Retrieval
```bash
REDIS_URL=redis://localhost:6379
# BINANCE_API_KEY=... (optional)
# COINGECKO_API_KEY=... (optional, for higher limits)
```

## API Endpoints

### App-Facing (Mobile App)
- `GET /api/v1/me` - Current user
- `GET /api/v1/bots` - List bots
- `POST /api/v1/bots` - Create bot
- `GET /api/v1/bots/:id` - Get bot details
- `PATCH /api/v1/bots/:id/config` - Update config
- `POST /api/v1/bots/:id/actions` - Pause/resume/redeploy/destroy
- `GET /api/v1/bots/:id/metrics` - Performance data
- `GET /api/v1/bots/:id/events` - Trade events

### Bot-Facing (From VPS)
- `POST /api/v1/bot/:id/register` - First boot registration
- `GET /api/v1/bot/:id/config` - Poll for config
- `POST /api/v1/bot/:id/config_ack` - Confirm config applied
- `POST /api/v1/bot/:id/heartbeat` - Status ping
- `POST /api/v1/bot/:id/events` - Push events

## Roadmap

### MVP (Current)
- [x] React Native app
- [x] Control plane API
- [x] Data retrieval (CoinGecko + Binance)
- [x] Sync adapter pattern
- [ ] ~~Cedros Login~~ (external dependency)
- [ ] ~~Cedros Pay~~ (external dependency)
- [ ] ~~Cedros-open-spawn~~ (external dependency)

### Post-MVP
- [ ] xStocks price feeds (Pyth Network)
- [ ] Tokenized metals (ORO integration)
- [ ] Algorithm backtesting
- [ ] Social features (leaderboards)
- [ ] Multi-exchange execution

## Development

### Adding a New Data Source

1. Create file in `services/data-retrieval/src/sources/`
2. Implement `PriceDataSource` trait
3. Add to aggregator in `lib.rs`

Example:
```rust
pub struct PythClient { ... }

#[async_trait]
impl PriceDataSource for PythClient {
    async fn get_price(&self, asset: &str, quote: &str) -> Result<PricePoint> {
        // Fetch from Pyth oracle
    }
}
```

### Adding a New Screen

1. Create component in `apps/mobile/src/screens/`
2. Add to navigation in `navigation/AppNavigator.tsx`
3. Add route type to `RootStackParamList`

## License

MIT - See LICENSE file

## Credits

Built with:
- [OpenClaw](https://openclaw.ai) - AI agent framework
- [React Native](https://reactnative.dev/) - Mobile framework
- [Axum](https://github.com/tokio-rs/axum) - Rust web framework
- [CoinGecko](https://coingecko.com) - Price data
- [Binance](https://binance.com) - Real-time feeds

---

**Note:** This is a work in progress. MVP features are complete, but some integrations (Cedros Login/Pay, xStocks data) are pending external dependencies.
