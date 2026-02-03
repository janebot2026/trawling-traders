-- Migration: Add agent_wallet column to bots table
ALTER TABLE bots ADD COLUMN IF NOT EXISTS agent_wallet TEXT;

-- Index for quick lookups by wallet
CREATE INDEX IF NOT EXISTS idx_bots_agent_wallet ON bots(agent_wallet) WHERE agent_wallet IS NOT NULL;
