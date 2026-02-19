-- Migration: Apply updated_at auto-update trigger to all tables that have
-- an updated_at column. Previously only the users table had this trigger
-- (created in 006_cedros_login_integration.sql).
--
-- The function update_updated_at_column() was created in migration 006.

-- bots
DROP TRIGGER IF EXISTS update_bots_updated_at ON bots;
CREATE TRIGGER update_bots_updated_at
    BEFORE UPDATE ON bots
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- platform_config
DROP TRIGGER IF EXISTS update_platform_config_updated_at ON platform_config;
CREATE TRIGGER update_platform_config_updated_at
    BEFORE UPDATE ON platform_config
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- bot_openclaw_config
DROP TRIGGER IF EXISTS update_bot_openclaw_config_updated_at ON bot_openclaw_config;
CREATE TRIGGER update_bot_openclaw_config_updated_at
    BEFORE UPDATE ON bot_openclaw_config
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- docs_categories
DROP TRIGGER IF EXISTS update_docs_categories_updated_at ON docs_categories;
CREATE TRIGGER update_docs_categories_updated_at
    BEFORE UPDATE ON docs_categories
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- docs_articles
DROP TRIGGER IF EXISTS update_docs_articles_updated_at ON docs_articles;
CREATE TRIGGER update_docs_articles_updated_at
    BEFORE UPDATE ON docs_articles
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
