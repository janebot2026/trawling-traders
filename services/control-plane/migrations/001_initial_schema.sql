-- Initial schema for Trawling Traders Control Plane

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Enums
DO $$ BEGIN
    CREATE TYPE subscription_status AS ENUM ('active', 'cancelled', 'past_due');
EXCEPTION WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE bot_status AS ENUM ('provisioning', 'online', 'offline', 'paused', 'error', 'destroying');
EXCEPTION WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE config_status AS ENUM ('pending', 'applied', 'failed');
EXCEPTION WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE persona AS ENUM ('beginner', 'tweaker', 'quant_lite');
EXCEPTION WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE asset_focus AS ENUM ('majors', 'tokenized_equities', 'tokenized_metals', 'memes', 'custom');
EXCEPTION WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE algorithm_mode AS ENUM ('trend', 'mean_reversion', 'breakout');
EXCEPTION WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE strictness AS ENUM ('low', 'medium', 'high');
EXCEPTION WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE trading_mode AS ENUM ('paper', 'live');
EXCEPTION WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE event_type AS ENUM ('trade_opened', 'trade_closed', 'stop_triggered', 'config_applied', 'config_failed', 'error', 'status_change');
EXCEPTION WHEN duplicate_object THEN null;
END $$;

-- Users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email TEXT NOT NULL UNIQUE,
    cedros_user_id TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Subscriptions table
CREATE TABLE IF NOT EXISTS subscriptions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status subscription_status NOT NULL DEFAULT 'active',
    max_bots INTEGER NOT NULL DEFAULT 4,
    current_period_start TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    current_period_end TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Bots table
CREATE TABLE IF NOT EXISTS bots (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    status bot_status NOT NULL DEFAULT 'provisioning',
    persona persona NOT NULL,
    droplet_id BIGINT,
    region TEXT NOT NULL DEFAULT 'nyc1',
    ip_address INET,
    agent_wallet TEXT,
    desired_version_id UUID NOT NULL,
    applied_version_id UUID,
    config_status config_status NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_heartbeat_at TIMESTAMPTZ
);

-- Config versions table
CREATE TABLE IF NOT EXISTS config_versions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bot_id UUID NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
    version INTEGER NOT NULL,
    name TEXT NOT NULL,
    persona persona NOT NULL,
    asset_focus asset_focus NOT NULL,
    custom_assets JSONB,
    algorithm_mode algorithm_mode NOT NULL,
    strictness strictness NOT NULL,
    max_position_size_percent INTEGER NOT NULL,
    max_daily_loss_usd INTEGER NOT NULL,
    max_drawdown_percent INTEGER NOT NULL,
    max_trades_per_day INTEGER NOT NULL,
    trading_mode trading_mode NOT NULL,
    llm_provider TEXT NOT NULL,
    encrypted_llm_api_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(bot_id, version)
);

-- Metrics table
CREATE TABLE IF NOT EXISTS metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bot_id UUID NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL,
    equity DECIMAL(20, 8) NOT NULL,
    pnl DECIMAL(20, 8) NOT NULL
);

-- Events table
CREATE TABLE IF NOT EXISTS events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bot_id UUID NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
    event_type event_type NOT NULL,
    message TEXT NOT NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_bots_user_id ON bots(user_id);
CREATE INDEX IF NOT EXISTS idx_bots_status ON bots(status);
CREATE INDEX IF NOT EXISTS idx_bots_agent_wallet ON bots(agent_wallet) WHERE agent_wallet IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_config_versions_bot_id ON config_versions(bot_id);
CREATE INDEX IF NOT EXISTS idx_metrics_bot_id_timestamp ON metrics(bot_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_events_bot_id_created_at ON events(bot_id, created_at);
