-- Per-bot notification settings (webhook destinations for trader-facing alerts)
CREATE TABLE IF NOT EXISTS bot_notification_settings (
    bot_id                UUID PRIMARY KEY REFERENCES bots(id) ON DELETE CASCADE,
    discord_webhook_url   TEXT,          -- encrypted; NULL = not configured
    email_webhook_url     TEXT,          -- encrypted; NULL = not configured
    notifications_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Reuse trigger function from migration 006
DROP TRIGGER IF EXISTS update_bot_notification_settings_updated_at ON bot_notification_settings;
CREATE TRIGGER update_bot_notification_settings_updated_at
    BEFORE UPDATE ON bot_notification_settings
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
