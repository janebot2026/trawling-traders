# OpenClaw Integration Strategy

*How Trawling Traders controls OpenClaw bots without forking*

## The Question

> Do we need to fork OpenClaw so settings like cron jobs read from our centralized database?

**Answer: No fork needed.** OpenClaw already has a desired-state reconciliation pattern we can leverage.

## Current OpenClaw Architecture

OpenClaw uses a **pull-based config system**:

1. Bot starts up with initial config (`openclaw.json`)
2. Bot can query for updates via API
3. Bot posts back config acknowledgments
4. Gateway manages cron schedules locally

## Proposed Integration (No Fork)

Instead of forking OpenClaw, we build a **sync adapter** pattern:

```
┌─────────────────────────────────────────────────────────────┐
│                    Trawling Traders                          │
│                    (Rust Control Plane)                      │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │ Bot Config   │  │ Sync Adapter │  │ OpenClaw Client  │  │
│  │   Service    │──│   (new)      │──│   (new)          │  │
│  └──────────────┘  └──────────────┘  └──────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ HTTPS / WebSocket
                              ▼
                    ┌──────────────────┐
                    │  DigitalOcean    │
                    │     VPS          │
                    └──────────────────┘
                              │
                              │ Local HTTP
                              ▼
                    ┌──────────────────┐
                    │   OpenClaw       │
                    │   (unchanged)    │
                    │   - Gateway      │
                    │   - Cron jobs    │
                    │   - Agent        │
                    └──────────────────┘
```

## How It Works

### 1. Bot Provisioning

When user creates a bot:

```rust
// Control Plane
let bot = create_bot(config).await?;
let droplet = provision_do_droplet(bot.id).await?;

// Generate bootstrap config for OpenClaw
let bootstrap_config = OpenClawConfig {
    agent: AgentConfig {
        name: bot.name,
        // ...
    },
    channels: ChannelsConfig {
        // Use sync adapter as the "brain"
        http_webhook: format!("https://api.trawling-traders.com/bots/{}/webhook", bot.id),
    },
    // Initial cron jobs for config polling
    cron: vec![
        CronJob {
            name: "config-sync",
            schedule: "*/30 * * * * *", // Every 30 seconds
            message: "Read https://api.trawling-traders.com/bots/{bot_id}/config and apply",
        }
    ],
};

// Write to droplet via cloud-init
droplet.set_user_data(cloud_init_script(bootstrap_config));
```

### 2. Config Synchronization

The bot polls for config updates:

```rust
// OpenClaw agent (unchanged)
async fn poll_config() {
    let config = http_get(format!("{}/config", CONTROL_PLANE_URL)).await?;
    let current_hash = hash_local_config();
    
    if config.hash != current_hash {
        // Apply new config
        apply_config(config).await?;
        
        // Acknowledge
        http_post(format!("{}/config_ack", CONTROL_PLANE_URL), json!({
            "version": config.version,
            "hash": config.hash,
            "applied_at": now(),
        })).await?;
    }
}
```

### 3. Cron Job Management

Instead of OpenClaw managing cron jobs locally, the control plane generates the cron schedule:

```rust
// Control Plane generates cron based on bot settings
fn generate_cron_jobs(bot_config: &BotConfig) -> Vec<CronJob> {
    vec![
        // Market data fetch
        CronJob {
            name: "fetch-prices",
            schedule: "* * * * *".to_string(), // Every minute
            message: "Fetch BTC,ETH prices from data-retrieval service".to_string(),
        },
        
        // Trading algorithm execution
        CronJob {
            name: "run-algorithm",
            schedule: "*/5 * * * *".to_string(), // Every 5 minutes
            message: "Run trend-following algorithm on current data".to_string(),
        },
        
        // Health check
        CronJob {
            name: "health-ping",
            schedule: "*/30 * * * * *".to_string(), // Every 30 seconds
            message: "POST heartbeat to control plane".to_string(),
        },
        
        // Config sync (built-in)
        CronJob {
            name: "config-sync",
            schedule: "* * * * *".to_string(), // Every minute
            message: "Pull latest config from control plane".to_string(),
        },
    ]
}

// When user updates config, regenerate cron jobs
async fn update_bot_config(bot_id: &str, new_config: BotConfig) -> Result<()> {
    let cron_jobs = generate_cron_jobs(&new_config);
    
    // Store in database
    db::update_bot_config(bot_id, new_config, cron_jobs.clone()).await?;
    
    // Increment version - bot will pick up on next poll
    db::increment_config_version(bot_id).await?;
    
    Ok(())
}
```

### 4. The Sync Adapter

A lightweight service running alongside the control plane:

```rust
// sync-adapter/src/main.rs

#[derive(Debug, Clone)]
struct BotConfigPayload {
    version: String,
    hash: String,
    agent_config: OpenClawAgentConfig,
    cron_jobs: Vec<CronJob>,
    trading_params: TradingParams,
    llm_config: LlmConfig,
}

// Endpoint: GET /bots/{id}/config
async fn get_config(Path(bot_id): Path<String>) -> Json<BotConfigPayload> {
    let config = db::get_desired_config(&bot_id).await?;
    Json(config)
}

// Endpoint: POST /bots/{id}/config_ack
async fn ack_config(
    Path(bot_id): Path<String>,
    Json(ack): Json<ConfigAck>,
) -> Result<(), AppError> {
    db::mark_config_applied(&bot_id, &ack.version, &ack.hash).await?;
    Ok(())
}

// Endpoint: POST /bots/{id}/heartbeat
async fn heartbeat(
    Path(bot_id): Path<String>,
    Json(status): Json<BotStatus>,
) -> Result<(), AppError> {
    db::update_bot_status(&bot_id, &status).await?;
    
    // Check if config needs update
    let desired = db::get_config_version(&bot_id, "desired").await?;
    let applied = db::get_config_version(&bot_id, "applied").await?;
    
    if desired != applied {
        // Return 409 Conflict to signal update needed
        // Bot will fetch immediately
    }
    
    Ok(())
}

// Endpoint: POST /bots/{id}/metrics
async fn ingest_metrics(
    Path(bot_id): Path<String>,
    Json(metrics): Json<Vec<MetricPoint>>,
) -> Result<(), AppError> {
    db::store_metrics(&bot_id, metrics).await?;
    Ok(())
}

// Endpoint: POST /bots/{id}/events
async fn ingest_events(
    Path(bot_id): Path<String>,
    Json(events): Json<Vec<BotEvent>>,
) -> Result<(), AppError> {
    db::store_events(&bot_id, events).await?;
    Ok(())
}
```

## Config Flow

```
User updates bot settings in app
           │
           ▼
┌──────────────────────┐
│  Control Plane API   │
│  PATCH /bots/{id}    │
└──────────────────────┘
           │
           ▼
┌──────────────────────┐
│  Generate new config │
│  + cron jobs         │
│  + version increment │
└──────────────────────┘
           │
           ▼
┌──────────────────────┐
│  Database            │
│  desired_version = 2 │
│  applied_version = 1 │
└──────────────────────┘
           │
           │ (bot polls every 30s)
           ▼
┌──────────────────────┐
│  Bot: GET /config    │
│  Sees version 2      │
│  Downloads + applies │
│  POST /config_ack    │
└──────────────────────┘
           │
           ▼
┌──────────────────────┐
│  Database            │
│  applied_version = 2 │
│  status = "synced"   │
└──────────────────────┘
           │
           ▼
┌──────────────────────┐
│  App shows           │
│  "Config Applied"    │
└──────────────────────┘
```

## Why Not Fork?

| Approach | Pros | Cons |
|----------|------|------|
| **Fork OpenClaw** | Full control | Maintenance burden, diverges from upstream, security updates |
| **Sync Adapter** | Clean separation, OpenClaw updates automatically, simpler | Requires polling (30s latency max) |
| **Push Notifications** | Instant updates | More complex, requires WebSocket infra |

**Recommended:** Sync Adapter with 30s polling for MVP. Can add WebSocket push later.

## Open Questions

1. **Config Size:** If config is large (>100KB), use diff/patch instead of full replace
2. **Rollback:** Keep last N config versions for rollback capability  
3. **Validation:** Bot should validate config before applying (schema check)
4. **Secrets:** API keys should be encrypted in transit and at rest

## Implementation Plan

1. **Build sync-adapter service** (Rust, Axum or Actix)
2. **Add config endpoints** to control plane
3. **Generate OpenClaw bootstrap configs** with sync URLs
4. **Build config reconciliation loop** in OpenClaw agent (or use existing cron)
5. **Test end-to-end:** update config in app → see bot pick it up → verify applied

## Code Location

```
trawling-traders/
├── services/
│   ├── data-retrieval/     ✅ Done
│   ├── control-plane/      🔄 Next (after sync-adapter design)
│   └── sync-adapter/       🆕 New (lightweight, single purpose)
```

---

**Decision:** Use sync adapter pattern. No OpenClaw fork needed.
