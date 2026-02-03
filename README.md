# Trawling Traders

⚠️ **Work in Progress - Not Ready for Production Use**

This project is actively under development. APIs, schemas, and trading behavior may change without notice.

---

**Deploy Solana OpenClaw traders in minutes.**  
Trawl markets. Tune strategies. Track performance.

## Overview

Trawling Traders lets you spin up personalized virtual employee trading agents on your own DigitalOcean VPS. Each agent is an OpenClaw-powered bot that:
- **Deploys in minutes**: From app tap to live trading
- **Runs your strategy**: Trend, mean reversion, breakout algorithms
- **Stays in your control**: Your VPS, your keys, your profits
- **Reports transparently**: Real-time P&L, trade history, event logs

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
│  (iOS/Android)  │
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
         │           - Pyth (xStocks/Metals)
         ▼
┌─────────────────┐
│  OpenClaw Bots  │  On DigitalOcean VPS
│   (per bot)     │  - Poll config
└─────────────────┘  - Execute trades on Solana
```

## Features

### Trading Personas
- **Set & Forget**: Conservative defaults, paper trading, strict safety
- **Hands-on**: Tune exposure, risk controls, see trade reasons  
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
├── services/
│   ├── control-plane/       # Rust API server
│   └── data-retrieval/      # Price aggregation service
├── packages/
│   ├── types/               # Shared TypeScript types
│   └── api-client/          # API client
└── docs/
```

## Known Issues

### `rust_decimal` + `sqlx 0.8` Integration

**Status:** In Progress

The `Decimal` type from `rust_decimal` does not have native `sqlx 0.8` support via the `db-postgres` feature (that feature targets `tokio-postgres`, not sqlx). We're evaluating options:
- Use `bigdecimal` for database layer with conversion to `rust_decimal` for trading logic
- Implement custom `sqlx::Type/Encode/Decode` traits for `Decimal`

All price calculations use `Decimal` for precision. Database integration is being finalized.

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
- `POST /api/v1/bot/:id/wallet` - Report agent wallet address

## Roadmap

### MVP (In Progress)
- [x] React Native app with ocean theme
- [x] Control plane API (Rust/Axum)
- [x] Data retrieval (CoinGecko + Binance + Pyth)
- [x] Sync adapter pattern
- [x] Agent wallet reporting
- [ ] Cedros Login integration
- [ ] Cedros Pay integration
- [ ] claw-spawn VPS provisioning

### Post-MVP
- [ ] Algorithm backtesting
- [ ] Multi-exchange execution
- [ ] Social features (leaderboards)

## Development

See `docs/` for architecture details.

## License

MIT - See LICENSE file

---

**⚠️ Warning:** This project is a work in progress and not ready for production use. APIs, schemas, and trading behavior may change without notice.
