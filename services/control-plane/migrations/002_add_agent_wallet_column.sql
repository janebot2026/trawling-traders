-- Migration: Add agent_wallet column to bots table
-- NOTE: This migration is redundant — agent_wallet was already defined in
-- 001_initial_schema.sql (line 82) and indexed there. The ADD COLUMN IF NOT
-- EXISTS is a safe no-op. Kept for migration history continuity.
ALTER TABLE bots ADD COLUMN IF NOT EXISTS agent_wallet TEXT;

-- Note: Index idx_bots_agent_wallet already created in 001_initial_schema.sql
