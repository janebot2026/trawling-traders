-- Migration: 005_bot_openclaw_config.sql
-- Purpose: Add OpenClaw configuration table for LLM and channel integrations
-- This separates OpenClaw-specific config from trading config for cleaner encryption handling

CREATE TABLE IF NOT EXISTS bot_openclaw_config (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bot_id UUID NOT NULL REFERENCES bots(id) ON DELETE CASCADE,

    -- LLM Configuration
    llm_provider TEXT NOT NULL,                    -- 'openai', 'anthropic', 'venice', 'openrouter'
    llm_model TEXT NOT NULL DEFAULT '',            -- 'gpt-4o', 'claude-3-5-sonnet', etc.
    encrypted_llm_api_key TEXT NOT NULL,           -- AES-256-GCM encrypted

    -- Telegram Integration
    telegram_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    encrypted_telegram_bot_token TEXT,             -- AES-256-GCM encrypted

    -- Discord Integration (columns added for V2, API deferred)
    discord_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    encrypted_discord_bot_token TEXT,              -- AES-256-GCM encrypted

    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- One OpenClaw config per bot
    UNIQUE(bot_id)
);

-- Index for efficient bot lookups
CREATE INDEX IF NOT EXISTS idx_bot_openclaw_config_bot_id ON bot_openclaw_config(bot_id);

-- Migrate existing LLM data from config_versions
-- Takes the latest config version for each bot
INSERT INTO bot_openclaw_config (bot_id, llm_provider, llm_model, encrypted_llm_api_key)
SELECT DISTINCT ON (cv.bot_id)
    cv.bot_id,
    COALESCE(cv.llm_provider, 'openai'),
    '',  -- llm_model is new field
    COALESCE(cv.encrypted_llm_api_key, '')
FROM config_versions cv
INNER JOIN bots b ON cv.bot_id = b.id
WHERE cv.bot_id IS NOT NULL
ORDER BY cv.bot_id, cv.version DESC
ON CONFLICT (bot_id) DO NOTHING;

-- Comment for documentation
COMMENT ON TABLE bot_openclaw_config IS 'OpenClaw gateway configuration including LLM and channel integrations. Secrets are AES-256-GCM encrypted.';
COMMENT ON COLUMN bot_openclaw_config.encrypted_llm_api_key IS 'AES-256-GCM encrypted LLM API key. Format: base64(nonce || ciphertext || tag)';
COMMENT ON COLUMN bot_openclaw_config.encrypted_telegram_bot_token IS 'AES-256-GCM encrypted Telegram bot token from @BotFather';
COMMENT ON COLUMN bot_openclaw_config.encrypted_discord_bot_token IS 'AES-256-GCM encrypted Discord bot token (V2 feature)';
