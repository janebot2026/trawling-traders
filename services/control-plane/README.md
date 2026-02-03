# Trawling Traders Control Plane

## Environment Variables

```bash
# Database
DATABASE_URL=postgres://postgres:postgres@localhost:5432/trawling_traders

# Server
PORT=3000

# Security (for production)
# JWT_SECRET=your-secret-here
# ENCRYPTION_KEY=your-32-byte-key-here

# Cedros Pay (optional for now)
# STRIPE_SECRET_KEY=sk_test_...
# STRIPE_WEBHOOK_SECRET=whsec_...
```

## Local Development Setup

1. **Start PostgreSQL** (using Docker):
   ```bash
   docker run -d \
     --name trawling-traders-db \
     -e POSTGRES_PASSWORD=postgres \
     -e POSTGRES_DB=trawling_traders \
     -p 5432:5432 \
     postgres:15
   ```

2. **Run migrations**:
   ```bash
   cargo sqlx migrate run
   ```

3. **Start the server**:
   ```bash
   cargo run
   ```

## API Endpoints

- `GET /v1/health` - Health check
- `GET /v1/me` - Current user (auth required)
- `GET /v1/bots` - List bots (auth required)
- `POST /v1/bots` - Create bot (auth required)
- `GET /v1/bots/:id` - Get bot details (auth required)
- `GET /v1/bots/:id/metrics` - Get bot metrics (auth required)
- `GET /v1/bots/:id/events` - Get bot events (auth required)

## Bot-facing endpoints (from VPS)

- `GET /v1/bot/:id/config` - Get config
- `POST /v1/bot/:id/heartbeat` - Heartbeat
- `POST /v1/bot/:id/events` - Push events
