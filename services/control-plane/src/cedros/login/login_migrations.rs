//! Migration workarounds for cedros-login-server v0.0.5 bugs.
//!
//! cedros-login has 55+ non-idempotent DDL statements. On every startup we wipe
//! all cedros-login migration entries and tables (except `users`), then re-run
//! from scratch. This module handles specific migrations that cannot run through
//! sqlx's migrator at all.

use sqlx::PgPool;
use tracing::info;

/// Drop ALL cedros-login tables for clean-slate migration reset.
/// Excludes `users` (shared with our app). Uses CASCADE to handle FK deps.
pub async fn drop_all_cedros_tables(pool: &PgPool) {
    // Tables WITHOUT IF NOT EXISTS (would fail on re-creation)
    for table in [
        "outbox_events",
        "user_credentials",
        "webauthn_credentials",
        "webauthn_challenges",
        "deposit_sessions",
        "deposit_webhook_events",
        "privacy_notes",
        "credit_balances",
        "credit_transactions",
        "deposit_config",
        "credit_holds",
        "pending_spl_deposits",
        "withdrawal_history",
        "treasury_config",
        "pending_wallet_recovery",
    ] {
        sqlx::query(&format!("DROP TABLE IF EXISTS {table} CASCADE"))
            .execute(pool)
            .await
            .ok();
    }
    // Tables WITH IF NOT EXISTS but having non-idempotent ALTER TABLE/ADD CONSTRAINT.
    // Must drop so columns+constraints get recreated cleanly. Excludes `users`.
    for table in [
        "sessions",
        "solana_nonces",
        "verification_tokens",
        "login_attempts",
        "totp_secrets",
        "totp_recovery_codes",
        "custom_roles",
        "abac_policies",
        "organizations",
        "memberships",
        "invites",
        "audit_logs",
        "sso_providers",
        "sso_auth_states",
        "api_keys",
        "system_settings",
        "solana_wallet_material",
        "credit_refund_requests",
    ] {
        sqlx::query(&format!("DROP TABLE IF EXISTS {table} CASCADE"))
            .execute(pool)
            .await
            .ok();
    }
}

/// Drop orphaned idx_* indexes on cedros-login tables (not our app tables).
/// These indexes survive crash loops when their parent tables use IF NOT EXISTS,
/// but block re-migration because CREATE INDEX lacks IF NOT EXISTS.
pub async fn drop_orphaned_cedros_indexes(pool: &PgPool) {
    let app_tables = [
        "bots",
        "config_versions",
        "metrics",
        "events",
        "users",
        "platform_config",
        "config_audit_log",
        "bot_openclaw_config",
    ];
    let exclude = app_tables
        .iter()
        .map(|t| format!("'{t}'"))
        .collect::<Vec<_>>()
        .join(",");
    let query = format!(
        "SELECT indexname::text FROM pg_indexes \
         WHERE schemaname = 'public' \
         AND indexname LIKE 'idx_%' \
         AND tablename NOT IN ({exclude})"
    );
    let orphaned_indexes: Vec<String> = sqlx::query_scalar(&query)
        .fetch_all(pool)
        .await
        .unwrap_or_default();
    for idx in &orphaned_indexes {
        sqlx::query(&format!("DROP INDEX IF EXISTS \"{idx}\""))
            .execute(pool)
            .await
            .ok();
    }
    if !orphaned_indexes.is_empty() {
        info!(
            "Dropped {} orphaned cedros-login indexes",
            orphaned_indexes.len()
        );
    }
}

/// Pre-apply cedros-login migrations that cannot run through sqlx's migrator:
/// - 4 migrations use CREATE INDEX CONCURRENTLY (can't run in transactions)
/// - 20260123000001: duplicate index name conflict
///
/// Creates indexes without CONCURRENTLY and inserts migration entries with correct
/// SHA-384 checksums so the migrator skips them.
pub async fn pre_apply_buggy_migrations(pool: &PgPool) {
    // (version, description, checksum_hex, pre_ddl[])
    let skip_migrations: &[(i64, &str, &str, &[&str])] = &[
        (
            20260108000001,
            "webauthn user credentials index",
            "2130da06130dd2f1c0b793db54ff8e9a4e1850a4b96857de08019ab181a6fe3076ce64970ccae53bc6c3a56bde2a4163",
            &["CREATE INDEX IF NOT EXISTS idx_webauthn_credentials_user_all \
               ON webauthn_credentials(user_id, created_at DESC)"],
        ),
        (
            20260108000002,
            "audit session composite index",
            "8527299795dd0a7dc4cdab698925bd8d32bb458890df2987bc99d8a84eaf2882a5233ceddc27b71675fd49f80ddfb8c2",
            &["CREATE INDEX IF NOT EXISTS idx_audit_logs_session_time \
               ON audit_logs(session_id, created_at DESC) WHERE session_id IS NOT NULL"],
        ),
        (
            20260109000002,
            "drop redundant audit index",
            "ca26b4793d69df067770936b7f78870995cc0555cb20ff47e10878d3b303bd16eb32217d8a18b78b950f4f0569d98079",
            &[], // no-op migration (SELECT 1)
        ),
        (
            20260110000001,
            "memberships composite indexes",
            "34bfb195fba47d4e58a3ed237c0b70e54b07201bddc28f0390a25605a61b7feebd8578e2e89fba8397d4098436c9e3bc",
            &[
                "CREATE INDEX IF NOT EXISTS idx_memberships_user_joined \
                 ON memberships(user_id, joined_at DESC)",
                "CREATE INDEX IF NOT EXISTS idx_memberships_org_joined \
                 ON memberships(org_id, joined_at ASC)",
            ],
        ),
        (
            // Bug: creates idx_credit_transactions_idempotency which already exists
            // from migration 20260121000002. DDL applied post-migration instead.
            20260123000001,
            "spl deposit tracking",
            "811c22129f27a2ad379838831cd280fa1bd730b03deac466f0d122437af042302e25d1431ad92b2400a70aa0073a892f",
            &[], // DDL applied in post_apply_skipped_migrations()
        ),
        // 20260130000001 duplicate version fixed in v0.0.5 (treasury_config renamed to 20260130000002)
    ];

    for (version, description, checksum_hex, pre_ddl) in skip_migrations {
        for ddl in *pre_ddl {
            sqlx::query(ddl).execute(pool).await.ok();
        }

        let insert = format!(
            "INSERT INTO _sqlx_migrations (version, description, installed_on, success, checksum, execution_time) \
             VALUES ({version}, '{description}', now(), true, decode('{checksum_hex}', 'hex'), 0) \
             ON CONFLICT (version) DO NOTHING"
        );
        sqlx::query(&insert).execute(pool).await.ok();
    }
}

/// Post-apply DDL for migrations skipped via pre_apply_buggy_migrations.
/// Tables referenced here are created by earlier migrations that ran normally.
pub async fn post_apply_skipped_migrations(pool: &PgPool) {
    post_apply_spl_deposit_tracking(pool).await;
    info!("Applied post-migration DDL for skipped migrations");
}

/// DDL from 20260123000001_spl_deposit_tracking.sql (duplicate index bug).
async fn post_apply_spl_deposit_tracking(pool: &PgPool) {
    let stmts = [
        "ALTER TABLE deposit_sessions ADD COLUMN IF NOT EXISTS input_token_mint TEXT",
        "ALTER TABLE deposit_sessions ADD COLUMN IF NOT EXISTS input_token_amount BIGINT",
        "ALTER TABLE deposit_sessions ADD COLUMN IF NOT EXISTS swap_tx_signature TEXT",
        "ALTER TABLE deposit_sessions ADD COLUMN IF NOT EXISTS credit_currency TEXT",
        "ALTER TABLE deposit_sessions ADD COLUMN IF NOT EXISTS credit_amount BIGINT",
        "ALTER TABLE credit_balances ADD COLUMN IF NOT EXISTS held_balance BIGINT NOT NULL DEFAULT 0",
        "ALTER TABLE credit_transactions ADD COLUMN IF NOT EXISTS idempotency_key TEXT",
        "ALTER TABLE credit_transactions ADD COLUMN IF NOT EXISTS reference_type TEXT",
        "ALTER TABLE credit_transactions ADD COLUMN IF NOT EXISTS reference_id UUID",
        "ALTER TABLE credit_transactions ADD COLUMN IF NOT EXISTS hold_id UUID",
    ];
    for stmt in &stmts {
        sqlx::query(stmt).execute(pool).await.ok();
    }
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS pending_spl_deposits (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES users(id),
            wallet_address TEXT NOT NULL,
            token_mint TEXT NOT NULL,
            token_amount_raw TEXT NOT NULL,
            token_amount BIGINT,
            tx_signature TEXT NOT NULL UNIQUE,
            status TEXT NOT NULL DEFAULT 'pending',
            deposit_session_id UUID REFERENCES deposit_sessions(id),
            error_message TEXT,
            processed_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            expires_at TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '7 days')
        )",
    )
    .execute(pool)
    .await
    .ok();
    let indexes = [
        "CREATE INDEX IF NOT EXISTS idx_deposit_sessions_input_token \
         ON deposit_sessions(input_token_mint) WHERE input_token_mint IS NOT NULL",
        // Skip idx_credit_transactions_idempotency - already created by migration 20260121000002
        "CREATE INDEX IF NOT EXISTS idx_pending_spl_deposits_user ON pending_spl_deposits(user_id)",
        "CREATE INDEX IF NOT EXISTS idx_pending_spl_deposits_wallet ON pending_spl_deposits(wallet_address)",
        "CREATE INDEX IF NOT EXISTS idx_pending_spl_deposits_status ON pending_spl_deposits(status)",
        "CREATE INDEX IF NOT EXISTS idx_pending_spl_deposits_pending \
         ON pending_spl_deposits(user_id, status) WHERE status = 'pending'",
    ];
    for stmt in &indexes {
        sqlx::query(stmt).execute(pool).await.ok();
    }
}
