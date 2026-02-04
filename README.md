# Trawling Traders 🎣

**Deploy Solana trading agents in minutes.**

Trawl markets. Tune strategies. Track performance.

---

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
make setup          # Install dependencies

# 2. Configure environment
make env            # Creates .env file - edit with your values
cp .env services/control-plane/.env

# 3. Start PostgreSQL
make db             # Or start manually: sudo systemctl start postgresql

# 4. Run migrations
cd services/control-plane
DATABASE_URL=postgres://postgres:postgres@localhost/trawling_traders cargo sqlx migrate run

# 5. Start all services (choose one)
make dev            # Shows commands to run in separate terminals
make dev-tmux       # Runs everything in tmux panes

# Or manually:
# Terminal 1: make data      # Data retrieval service (port 8080)
# Terminal 2: make control   # Control plane API (port 3000)
# Terminal 3: make mobile    # Mobile app
```

## Architecture

```
┌─────────────────┐
│  React Native   │  Mobile app for bot management
│  (iOS/Android)  │  - Cedros Login/Pay integration
└────────┬────────┘  - Bot creation wizard
         │ HTTPS     - Real-time metrics
         ▼
┌─────────────────┐
│  Control Plane  │  Rust/Axum API
│  (Rust/Axum)    │  - Bot CRUD with subscription tiers
└────────┬────────┘  - Config versioning
         │           - Sync adapter for bots
         │           - Secrets encryption (AES-256-GCM)
         │           - Rate limiting (100/120 req/min)
         │           - Observability + alerting
         ▼
┌─────────────────┐
│  Data Retrieval │  Rust price aggregation
│    (Rust)       │  - CoinGecko (REST)
└────────┬────────┘  - Binance (WebSocket)
         │           - Pyth (xStocks/Metals)
         ▼
┌─────────────────┐
│  Bot Runner     │  On DigitalOcean VPS
│  (per bot)      │  - Poll config from control plane
└─────────────────┘  - Execute trades via claw-trader-cli
                     - Paper trading (default)
                     - Real-time event reporting
```

## Project Structure

```
trawling-traders/
├── apps/
│   └── mobile/              # React Native app (Cedros Login/Pay)
├── services/
│   ├── control-plane/       # Rust API server (auth, bots, config)
│   ├── data-retrieval/      # Price aggregation service
│   └── bot-runner/          # Trading agent (runs on VPS)
├── packages/
│   └── downrigger/          # Setup CLI for trading agents
├── claw-trader-cli/         # Jupiter Ultra CLI tool
└── Makefile                 # Development commands
```

## Development Setup

### Prerequisites

- **Rust** (latest stable) - [rustup.rs](https://rustup.rs/)
- **Node.js** 18+ - [nodejs.org](https://nodejs.org/)
- **PostgreSQL** 14+
- **DigitalOcean** account (for VPS provisioning)

### Environment Variables

Create `.env` in the project root:

```bash
# Database (required)
DATABASE_URL=postgres://postgres:postgres@localhost/trawling_traders

# Control Plane (required)
PORT=3000
CONTROL_PLANE_URL=https://api.trawlingtraders.com
DATA_RETRIEVAL_URL=https://data.trawlingtraders.com

# Secrets (generate for production)
# Generate: openssl rand -base64 32
SECRETS_ENCRYPTION_KEY=your-base64-encoded-32-byte-key

# External APIs (optional, for full functionality)
DIGITALOCEAN_TOKEN=your-digitalocean-token
JUPITER_API_KEY=your-jupiter-api-key

# Alert webhooks (optional)
DISCORD_ALERT_WEBHOOK=https://discord.com/api/webhooks/...
EMAIL_ALERT_WEBHOOK=https://your-email-service.com/webhook
```

### Database Setup

```bash
# Start PostgreSQL
sudo systemctl start postgresql

# Create database (if not exists)
sudo -u postgres createdb trawling_traders

# Run migrations
cd services/control-plane
cargo sqlx migrate run
```

### Running Services

```bash
# Quick start - all services in tmux
make dev-tmux

# Or run individually:
# Terminal 1 - Data retrieval
cd services/data-retrieval
cargo run

# Terminal 2 - Control plane
cd services/control-plane
DATABASE_URL=postgres://postgres:postgres@localhost/trawling_traders cargo run

# Terminal 3 - Mobile app
cd apps/mobile
npm install
npm run ios  # or android
```

## API Endpoints

### App-Facing (Mobile App)
All routes require Cedros-issued JWT token.

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/v1/me` | Current user |
| GET | `/v1/bots` | List bots |
| POST | `/v1/bots` | Create bot (subscription limits apply) |
| GET | `/v1/bots/:id` | Get bot details |
| PATCH | `/v1/bots/:id/config` | Update config |
| POST | `/v1/bots/:id/actions` | Pause/resume/redeploy/destroy |
| GET | `/v1/bots/:id/metrics` | Performance data (7 days) |
| GET | `/v1/bots/:id/events` | Trade events (last 100) |
| POST | `/v1/simulate-signal` | Dry-run algorithm |

### Bot-Facing (From VPS)
These endpoints are for bot runners to communicate with control plane.

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/v1/bot/:id/register` | First boot registration |
| GET | `/v1/bot/:id/config` | Poll for config updates |
| POST | `/v1/bot/:id/config_ack` | Confirm config applied |
| POST | `/v1/bot/:id/heartbeat` | Status + metrics ping |
| POST | `/v1/bot/:id/events` | Push trade events |
| POST | `/v1/bot/:id/wallet` | Report agent wallet address |

### Health Checks (No Auth)

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/v1/healthz` | Load balancer health check |
| GET | `/v1/readyz` | Readiness probe (checks DB) |

## Features

### Trading Personas
- **Set & Forget**: Conservative defaults, paper trading, strict safety
- **Hands-on**: Tune exposure, risk controls, see trade reasons  
- **Power User**: Signal knobs, custom baskets, full control

### Algorithms
- **Trend**: SMA crossover, momentum confirmations
- **Mean Reversion**: Z-score based, fade extremes
- **Breakout**: Volume-confirmed range breaks

### Risk Management
- Position sizing (% of portfolio)
- Max daily loss limits
- Max drawdown circuit breakers
- Max trades per day
- **Paper trading mode (default)**
- Shield checks before every trade

### Subscription Tiers
| Tier | Bots | Trades/Day | Features |
|------|------|------------|----------|
| Free | 1 | 10 | Basic algorithms |
| Pro | 4 | 100 | Advanced tuning |
| Enterprise | 20 | 1000 | Custom strategies |

## Sync Adapter Pattern

No OpenClaw fork needed. Bots poll the control plane for config updates:

1. **Bot boots** → GET `/bot/:id/config` → downloads config + cron jobs
2. **Config changes** → User updates in app → new version created
3. **Bot polls** → Sees `hash != applied_hash` → downloads + applies
4. **Bot acks** → POST `/bot/:id/config_ack` → marked as synced

## Deployment

### Control Plane (Staging)

```bash
# Build release
cd services/control-plane
cargo build --release

# Set environment variables
export DATABASE_URL=...
export SECRETS_ENCRYPTION_KEY=...

# Run migrations
./target/release/control-plane migrate

# Start server
./target/release/control-plane
```

### Bot Runner (On VPS)

The `claw-spawn` library provisions DigitalOcean droplets with cloud-init that:
1. Installs Rust, Node.js, OpenClaw
2. Clones downrigger and claw-trader-cli
3. Sets up systemd services
4. Registers bot with control plane
5. Starts bot-runner agent

## Development Commands

```bash
make help           # Show all commands
make setup          # Install dependencies
make db             # Start PostgreSQL
make migrate        # Run migrations
make dev            # Show startup commands
make dev-tmux       # Start in tmux
make check          # Cargo check all services
make test           # Run all tests
make clean          # Clean build artifacts
make env            # Generate .env template
make status         # Check service status
```

## Key Technologies

- **Control Plane**: Rust, Axum 0.8, SQLx 0.8, PostgreSQL
- **Data Retrieval**: Rust, WebSockets, REST APIs
- **Bot Runner**: Rust, tokio, claw-trader-cli
- **Mobile App**: React Native, TypeScript, Cedros SDK
- **Infrastructure**: DigitalOcean, cloud-init, systemd

## Production Checklist

- [ ] Set `SECRETS_ENCRYPTION_KEY` (32-byte base64)
- [ ] Configure `DIGITALOCEAN_TOKEN` for provisioning
- [ ] Set up `JUPITER_API_KEY` for trading
- [ ] Configure webhook URLs for alerts (Discord/email)
- [ ] Set up PostgreSQL with backups
- [ ] Enable SSL/TLS on control plane
- [ ] Configure firewall rules for bot VPS
- [ ] Set up log aggregation (Loki/Grafana)
- [ ] Enable paper trading mode initially

## Known Limitations

- **Solana only** - No EVM or other chains yet
- **USDC pairs** - Primary trading against USDC
- **In-memory rate limiting** - Not distributed across instances
- **Single region** - DO droplets in nyc3 only

## Related Projects

- **[claw-trader-cli](https://github.com/janebot2026/claw-trader-cli)** - Jupiter Ultra CLI for execution
- **[downrigger](https://github.com/janebot2026/downrigger)** - Trading agent setup CLI
- **[claw-spawn](https://crates.io/crates/claw-spawn)** - DigitalOcean provisioning library

## License

MIT - See LICENSE file

---

**⚠️ Note:** This is actively developed software. APIs and behavior may change. Always test with paper trading before live deployment.
