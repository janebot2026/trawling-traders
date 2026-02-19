-- Migration: Add missing indexes for common query patterns

-- INFRA-027: docs_analytics_events is queried by event_type for analytics
-- aggregation, but had no index on that column.
CREATE INDEX IF NOT EXISTS idx_docs_analytics_events_event_type
    ON docs_analytics_events(event_type);

-- INFRA-033: config_versions is frequently queried by (bot_id, version DESC)
-- for "latest config" lookups. The existing idx_config_versions_bot_id only
-- covers bot_id; adding version DESC avoids a sort in common queries.
CREATE INDEX IF NOT EXISTS idx_config_versions_bot_id_version
    ON config_versions(bot_id, version DESC);
