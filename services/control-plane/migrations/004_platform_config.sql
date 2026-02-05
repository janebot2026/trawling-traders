-- Migration: Platform configuration table for admin-managed settings
-- Allows runtime configuration without environment variable changes

CREATE TABLE IF NOT EXISTS platform_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    encrypted BOOLEAN NOT NULL DEFAULT FALSE,
    description TEXT,
    category TEXT NOT NULL DEFAULT 'general',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by TEXT
);

-- Insert default configuration entries (values will come from env vars on first boot)
-- These act as documentation and allow override via admin dashboard

-- DigitalOcean provisioning
INSERT INTO platform_config (key, value, encrypted, description, category) VALUES
    ('digitalocean_token', '', TRUE, 'DigitalOcean API token for droplet provisioning', 'provisioning'),
    ('droplet_region', 'nyc3', FALSE, 'Default region for new droplets', 'provisioning'),
    ('droplet_size', 's-1vcpu-2gb', FALSE, 'Default droplet size', 'provisioning'),
    ('droplet_image', 'ubuntu-22-04-x64', FALSE, 'Default droplet OS image', 'provisioning')
ON CONFLICT (key) DO NOTHING;

-- Trading configuration
INSERT INTO platform_config (key, value, encrypted, description, category) VALUES
    ('jupiter_api_key', '', TRUE, 'Jupiter aggregator API key', 'trading'),
    ('solana_rpc_url', 'https://api.devnet.solana.com', FALSE, 'Solana RPC endpoint URL', 'trading'),
    ('default_slippage_bps', '50', FALSE, 'Default slippage in basis points', 'trading'),
    ('paper_trading_default', 'true', FALSE, 'Enable paper trading by default for new bots', 'trading')
ON CONFLICT (key) DO NOTHING;

-- Service URLs
INSERT INTO platform_config (key, value, encrypted, description, category) VALUES
    ('control_plane_url', '', FALSE, 'Public URL of this control plane service', 'services'),
    ('data_retrieval_url', '', FALSE, 'URL of the data retrieval service', 'services')
ON CONFLICT (key) DO NOTHING;

-- Alerting configuration
INSERT INTO platform_config (key, value, encrypted, description, category) VALUES
    ('discord_webhook_url', '', TRUE, 'Discord webhook URL for alerts', 'alerting'),
    ('email_webhook_url', '', TRUE, 'Email service webhook URL', 'alerting'),
    ('alert_email_to', 'alerts@trawlingtraders.com', FALSE, 'Default email recipient for alerts', 'alerting'),
    ('alerts_enabled', 'true', FALSE, 'Enable/disable all alerting', 'alerting')
ON CONFLICT (key) DO NOTHING;

-- Limits and quotas
INSERT INTO platform_config (key, value, encrypted, description, category) VALUES
    ('max_bots_per_user', '5', FALSE, 'Maximum bots allowed per user', 'limits'),
    ('max_concurrent_provisions', '3', FALSE, 'Maximum concurrent droplet provisions', 'limits'),
    ('rate_limit_requests_per_minute', '60', FALSE, 'API rate limit per user per minute', 'limits')
ON CONFLICT (key) DO NOTHING;

-- Create index for category-based queries
CREATE INDEX IF NOT EXISTS idx_platform_config_category ON platform_config(category);

-- Audit log for config changes
CREATE TABLE IF NOT EXISTS config_audit_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    config_key TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT,
    changed_by TEXT NOT NULL,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ip_address TEXT
);

CREATE INDEX IF NOT EXISTS idx_config_audit_log_key ON config_audit_log(config_key);
CREATE INDEX IF NOT EXISTS idx_config_audit_log_time ON config_audit_log(changed_at DESC);
