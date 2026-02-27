-- Embedded wallet support: per-bot wallet ID from cedros-login SSS
ALTER TABLE bots ADD COLUMN IF NOT EXISTS embedded_wallet_id TEXT;

CREATE INDEX IF NOT EXISTS idx_bots_embedded_wallet_id
    ON bots(embedded_wallet_id) WHERE embedded_wallet_id IS NOT NULL;
