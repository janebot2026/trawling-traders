-- Migration: Add agent_wallet column to bots table
ALTER TABLE bots ADD COLUMN IF NOT EXISTS agent_wallet TEXT;

-- Note: Index idx_bots_agent_wallet already created in 001_initial_schema.sql
