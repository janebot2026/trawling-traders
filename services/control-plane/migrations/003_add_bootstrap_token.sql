-- Migration: Add bootstrap_token column for secure secrets retrieval
-- This allows bots to authenticate once after provisioning to fetch secrets
-- instead of having secrets embedded in cloud-init user-data.

ALTER TABLE bots ADD COLUMN IF NOT EXISTS bootstrap_token TEXT;
ALTER TABLE bots ADD COLUMN IF NOT EXISTS bootstrap_token_used_at TIMESTAMPTZ;

-- Index for token lookup
CREATE INDEX IF NOT EXISTS idx_bots_bootstrap_token ON bots(bootstrap_token) WHERE bootstrap_token IS NOT NULL;

COMMENT ON COLUMN bots.bootstrap_token IS 'One-time token for bot to fetch secrets after provisioning';
COMMENT ON COLUMN bots.bootstrap_token_used_at IS 'Timestamp when bootstrap token was consumed (null = not yet used)';
