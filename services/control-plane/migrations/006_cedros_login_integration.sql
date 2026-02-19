-- Align users table with cedros-login-server's expected schema.
-- cedros-login's CREATE TABLE IF NOT EXISTS will be a no-op since
-- our migration 001 already created the users table. This migration
-- adds all columns cedros-login expects so its later ALTER TABLE
-- migrations succeed.

-- Drop cedros_user_id (not needed with embedded auth)
-- WARNING: DROP COLUMN is non-reversible. Data in cedros_user_id cannot be
-- recovered after this migration runs. A down migration would need to
-- re-populate the column from an external source.
ALTER TABLE users DROP COLUMN IF EXISTS cedros_user_id;

-- Make email nullable (cedros-login allows wallet-only users)
-- NOTE: With email nullable, the UNIQUE constraint on email still allows
-- multiple NULL values (PostgreSQL treats NULLs as distinct for UNIQUE).
-- This is intentional: wallet-only users have no email. If only one NULL
-- email is desired, add a partial unique index:
--   CREATE UNIQUE INDEX ... ON users(email) WHERE email IS NOT NULL;
ALTER TABLE users ALTER COLUMN email DROP NOT NULL;

-- Columns from cedros-login-server's initial schema
ALTER TABLE users ADD COLUMN IF NOT EXISTS email_verified BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS password_hash VARCHAR(255);
ALTER TABLE users ADD COLUMN IF NOT EXISTS name VARCHAR(255);
ALTER TABLE users ADD COLUMN IF NOT EXISTS picture TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS wallet_address VARCHAR(64) UNIQUE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS google_id VARCHAR(255) UNIQUE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS auth_methods TEXT[] NOT NULL DEFAULT '{}';

-- Columns added by later cedros-login migrations
ALTER TABLE users ADD COLUMN IF NOT EXISTS is_system_admin BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS apple_id VARCHAR(255) UNIQUE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS stripe_customer_id VARCHAR(255);
ALTER TABLE users ADD COLUMN IF NOT EXISTS last_login_at TIMESTAMPTZ;

-- Indexes from cedros-login's schema (IF NOT EXISTS is safe for idempotency)
CREATE INDEX IF NOT EXISTS idx_users_wallet ON users(wallet_address);
CREATE INDEX IF NOT EXISTS idx_users_google ON users(google_id);

-- Create the update_updated_at trigger function that cedros-login expects
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

DROP TRIGGER IF EXISTS update_users_updated_at ON users;
CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
