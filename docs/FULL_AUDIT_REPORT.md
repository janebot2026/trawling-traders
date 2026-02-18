# Full Codebase Audit Report

**Date:** 2026-02-18
**Auditor:** Claude Opus 4.6
**Scope:** Entire repository — control-plane, data-retrieval, bot-runner, mobile app, api-client, types, CI/CD, Docker

---

## 1. Executive Summary

### Build & Test Status
- All 3 Rust services compile cleanly (`cargo check` + `cargo clippy` pass)
- All 34 unit tests pass (3 ignored: network-dependent)
- TypeScript packages (`api-client`, `types`) typecheck cleanly
- 1 compiler warning: unused import `tempfile::tempdir` in bot-runner `gateway.rs:512`

### Top 10 Issues (Ranked by Risk)

| Rank | ID | Summary | Severity |
|------|-----|---------|----------|
| 1 | F-001 | N+1 query loop in bot name availability (up to 998 queries) | High/Perf |
| 2 | F-002 | `realized_pnl_today` never updated — risk rails ineffective | High/Bug |
| 3 | F-003 | Config version increment race (no transaction) | High/Bug |
| 4 | F-004 | Panicking `unwrap()` calls in production trading engine | High/Bug |
| 5 | F-005 | Hardcoded bot limit (4) ignores subscription tier | High/Bug |
| 6 | F-006 | Silent decryption failure returns empty LLM API key | High/Reliability |
| 7 | F-007 | Heartbeat metrics INSERT in a loop (N+1 DB calls) | Medium/Perf |
| 8 | F-008 | Event ingest INSERT in a loop (N+1 DB calls) | Medium/Perf |
| 9 | F-009 | Unsafe `libc::kill()` without error check in executor | Medium/Bug |
| 10 | F-010 | Cache TTL mismatch (30s app vs 60s Redis) in data-retrieval | Medium/Bug |

### Biggest Performance Opportunities
1. Batch INSERT for metrics and events (eliminate N+1 DB calls)
2. Bot name availability: single query with `NOT IN` instead of loop
3. Add DB connection pool query timeout (currently unlimited)

### Highest-Risk Bug
**F-002: `realized_pnl_today` is always zero.** The bot-runner tracks this field but never updates it after trades. This means the daily loss limit check in `validate_intent()` (runner.rs:598) never triggers — a bot could exceed its daily loss cap without any brake engaging.

### Quick Wins (< 1 hour each)
1. Replace name availability loop with single SQL query (~15 min)
2. Add `unwrap_or_default()` to safe Decimal::from_str in engine.rs (~10 min)
3. Batch event/metric INSERTs with `unnest()` (~30 min)
4. Add `idle_timeout` and `statement_timeout` to DB pool (~5 min)
5. Fix unused import in bot-runner gateway.rs (~1 min)
6. Wrap config version increment in a transaction (~15 min)

---

## 2. Findings Table

### F-001 — N+1 Query Loop in Bot Name Availability
| Field | Value |
|-------|-------|
| **Category** | Performance |
| **Severity** | High |
| **Confidence** | High |
| **Location** | `services/control-plane/src/handlers/bots.rs:139-162` |
| **Description** | When a bot name is taken, the handler loops from 2..=999, executing a separate DB query for each candidate name. Worst case: 998 sequential queries. |
| **Impact** | Under load, this can hold a DB connection for several seconds, potentially exhausting the 20-connection pool. |
| **Recommended Fix** | Single query: `SELECT name FROM bots WHERE user_id=$1 AND name LIKE $2 || '-%' AND status != 'destroying'`, then compute next available in Rust. |
| **Test Plan** | Unit test with mock DB: verify correct suggestion when names 2-5 are taken. |
| **Effort** | S |

### F-002 — `realized_pnl_today` Never Updated
| Field | Value |
|-------|-------|
| **Category** | Bug |
| **Severity** | High |
| **Confidence** | High |
| **Location** | `services/bot-runner/src/runner.rs:55, 518, 598` |
| **Description** | `realized_pnl_today` is initialized to `Decimal::ZERO` and never mutated after trade execution. It's sent to OpenClaw (line 518) and used in the daily loss limit check (line 598). Since it's always zero, the daily loss circuit breaker never fires. |
| **Impact** | A bot can lose unlimited money in a day without the risk rail engaging. Critical for live trading. |
| **Recommended Fix** | After a confirmed sell trade, compute realized PnL from entry price vs. execution price and add to `self.realized_pnl_today`. Reset daily (e.g., on UTC midnight). |
| **Test Plan** | Add integration test: execute buy then sell, assert `realized_pnl_today` reflects the loss/gain. |
| **Effort** | M |

### F-003 — Config Version Increment Race Condition
| Field | Value |
|-------|-------|
| **Category** | Bug |
| **Severity** | High |
| **Confidence** | High |
| **Location** | `services/control-plane/src/handlers/bots.rs:899-906` |
| **Description** | `update_bot_config` reads `MAX(version)` then inserts `version + 1` without a transaction. Two concurrent requests can read the same version and both insert the same version number. |
| **Impact** | Duplicate config versions or unique constraint violations. |
| **Recommended Fix** | Wrap in a transaction with `FOR UPDATE` on the bot row, or use `INSERT ... SELECT COALESCE(MAX(version), 0) + 1 FROM config_versions WHERE bot_id = $1`. |
| **Test Plan** | Concurrent test: 2 threads update config simultaneously, verify no duplicate versions. |
| **Effort** | S |

### F-004 — Panicking `unwrap()` in Trading Engine
| Field | Value |
|-------|-------|
| **Category** | Bug |
| **Severity** | High |
| **Confidence** | High |
| **Location** | `services/control-plane/src/brain/engine.rs:80-81, 109-110, 156, 175-176, 222-223, 367` |
| **Description** | Multiple `.unwrap()` calls on `Option` and `Decimal::from_str()` results. While the `from_str` calls use constant strings (safe in practice), the pattern is fragile. Line 367: `candles.last().unwrap()` will panic if candles is empty despite earlier check for `< 20`. |
| **Impact** | Unlikely to panic with current constants, but a maintenance hazard. Any future change to the threshold or the constant strings could introduce a crash. |
| **Recommended Fix** | Replace `Decimal::from_str("0.95").unwrap()` with `const` values initialized once. Replace `candles.last().unwrap()` with `.ok_or()`. |
| **Test Plan** | Existing tests + add test with exactly 20 candles and edge cases. |
| **Effort** | S |

### F-005 — Hardcoded Bot Limit Ignores Subscription Tier
| Field | Value |
|-------|-------|
| **Category** | Bug |
| **Severity** | High |
| **Confidence** | High |
| **Location** | `services/control-plane/src/handlers/bots.rs:397` |
| **Description** | `if bot_count >= 4` is hardcoded instead of using the subscription tier's `max_bots()`. The subscription middleware already computes this, but it's ignored here. |
| **Impact** | Free-tier users can create 4 bots instead of their limit. Enterprise users capped at 4 instead of 20. |
| **Recommended Fix** | `if bot_count >= sub.tier.max_bots() as i64 {` |
| **Test Plan** | Test create_bot with different tiers; verify Free blocks at 1, Pro at 4, Enterprise at 20. |
| **Effort** | S |

### F-006 — Silent Decryption Failure for LLM API Key
| Field | Value |
|-------|-------|
| **Category** | Reliability |
| **Severity** | High |
| **Confidence** | High |
| **Location** | `services/control-plane/src/handlers/sync.rs:55-56, 607` |
| **Description** | `state.secrets.decrypt(...).unwrap_or_default()` silently returns empty string on decryption failure. The bot receives an empty API key, LLM calls fail silently, and there's no alert or log entry. |
| **Impact** | Bots silently lose LLM capability. Users see no error. Hard to diagnose. |
| **Recommended Fix** | Log a warning on decryption failure and include a `warnings` field in the config response so the bot can alert. |
| **Test Plan** | Test with corrupted encrypted key, verify warning is logged. |
| **Effort** | S |

### F-007 — Heartbeat Metrics N+1 Insert
| Field | Value |
|-------|-------|
| **Category** | Performance |
| **Severity** | Medium |
| **Confidence** | High |
| **Location** | `services/control-plane/src/handlers/sync.rs:245-266` |
| **Description** | Each metric in the batch is inserted with a separate `INSERT INTO metrics` query. A heartbeat with 10 metrics = 10 DB round-trips. |
| **Impact** | With 100 bots sending heartbeats every 30s with 5 metrics each: 500 individual inserts per 30s = ~17/sec. Tolerable now, but won't scale. |
| **Recommended Fix** | Use `INSERT ... SELECT * FROM unnest($1::uuid[], $2::timestamptz[], $3::numeric[], $4::numeric[])` for batch insert. |
| **Test Plan** | Benchmark: measure heartbeat latency before/after batch insert. |
| **Effort** | S |

### F-008 — Event Ingest N+1 Insert
| Field | Value |
|-------|-------|
| **Category** | Performance |
| **Severity** | Medium |
| **Confidence** | High |
| **Location** | `services/control-plane/src/handlers/sync.rs:334-345` |
| **Description** | Same pattern as F-007: each event is inserted individually. |
| **Impact** | Same scaling concern. |
| **Recommended Fix** | Same batch insert approach. |
| **Test Plan** | Same approach. |
| **Effort** | S |

### F-009 — Unsafe `libc::kill()` Without Error Handling
| Field | Value |
|-------|-------|
| **Category** | Bug |
| **Severity** | Medium |
| **Confidence** | Medium |
| **Location** | `services/bot-runner/src/executor.rs:356-358` |
| **Description** | After a timeout, the code calls `unsafe { libc::kill(pid as i32, libc::SIGKILL); }` without checking the return value. If the PID was recycled (rare on short timeouts), this could kill an unrelated process. |
| **Impact** | Low probability, but high consequence if it happens. |
| **Recommended Fix** | Check return value. Also: the `child` handle is consumed by `wait_with_output()`, so the fallback `child.kill()` on line 363 won't compile (it's behind `#[cfg(not(unix))]` but the `child` is moved). Consider restructuring to use `child.kill()` before `wait_with_output()` consumes it. |
| **Test Plan** | Manual review; add integration test with a sleep command that times out. |
| **Effort** | S |

### F-010 — Cache TTL Mismatch in Data Retrieval
| Field | Value |
|-------|-------|
| **Category** | Bug |
| **Severity** | Medium |
| **Confidence** | High |
| **Location** | `services/data-retrieval/src/lib.rs:247` vs `services/data-retrieval/src/cache/mod.rs:47` |
| **Description** | In-memory cache checks `< 30` seconds freshness, but Redis `set_ex` uses 60 seconds. A price that's 35 seconds old will miss the in-memory cache, hit Redis (which considers it fresh), and return stale data. |
| **Impact** | Inconsistent price freshness; could cause trading decisions based on outdated prices. |
| **Recommended Fix** | Align both to the same TTL (30s recommended for trading). |
| **Test Plan** | Test cache flow: insert at t=0, check at t=35, verify miss. |
| **Effort** | S |

### F-011 — Missing DB Pool Timeout and Idle Settings
| Field | Value |
|-------|-------|
| **Category** | Reliability |
| **Severity** | Medium |
| **Confidence** | High |
| **Location** | `services/control-plane/src/db/mod.rs:6-14` |
| **Description** | Pool has 20 connections with 3-second acquire timeout but no `idle_timeout`, `min_connections`, or statement-level timeout. A slow query can hold a connection indefinitely. |
| **Impact** | Under load, one slow query can cascade to pool exhaustion. |
| **Recommended Fix** | Add `.idle_timeout(Duration::from_secs(600))` and consider `sqlx::query(...).timeout(Duration::from_secs(10))`. |
| **Test Plan** | Simulate slow query, verify pool doesn't exhaust. |
| **Effort** | S |

### F-012 — Pyth Price Conversion Loses Precision via f64
| Field | Value |
|-------|-------|
| **Category** | Bug |
| **Severity** | Medium |
| **Confidence** | High |
| **Location** | `services/data-retrieval/src/sources/pyth.rs:189-199` |
| **Description** | Pyth integer price is converted to `f64` then to `Decimal`. For BTC at ~$50,000 with 8 decimal exponent, f64 loses sub-cent precision. |
| **Impact** | Price inaccuracy up to ~$0.01 for high-value assets. Matters for large trades. |
| **Recommended Fix** | Convert directly: `Decimal::from(price_int) * Decimal::TEN.powi(expo)`. |
| **Test Plan** | Test with known Pyth feed data, compare f64 vs Decimal path. |
| **Effort** | S |

### F-013 — No Backpressure on Binance WebSocket Channel
| Field | Value |
|-------|-------|
| **Category** | Reliability |
| **Severity** | Medium |
| **Confidence** | Medium |
| **Location** | `services/data-retrieval/src/sources/binance_ws.rs:53` |
| **Description** | 10,000-entry bounded channel. If consumer falls behind, `send()` blocks the message handler, stalling all price updates. No drop-oldest semantics. |
| **Impact** | During high volatility, stale prices could accumulate. |
| **Recommended Fix** | Use `try_send()` with logging on channel full, or switch to `broadcast::channel` for automatic overflow. |
| **Test Plan** | Load test: simulate slow consumer, verify no deadlock. |
| **Effort** | S |

### F-014 — No Jitter in Reconnection Backoff
| Field | Value |
|-------|-------|
| **Category** | Reliability |
| **Severity** | Low |
| **Confidence** | High |
| **Location** | `services/data-retrieval/src/lib.rs:140-167` |
| **Description** | Exponential backoff is deterministic. Multiple instances disconnecting simultaneously will all reconnect in lockstep (thundering herd). |
| **Impact** | Could overwhelm Binance WebSocket servers during outage recovery. |
| **Recommended Fix** | Add random jitter: `delay += rand::thread_rng().gen_range(0..delay/4)`. |
| **Test Plan** | Manual: verify random variance in reconnect timing. |
| **Effort** | S |

### F-015 — Mobile: `usePrices` Hook Dependency Issue
| Field | Value |
|-------|-------|
| **Category** | Bug |
| **Severity** | Medium |
| **Confidence** | Medium |
| **Location** | `apps/mobile/src/hooks/usePrices.ts:75` |
| **Description** | The `useEffect` dependency uses `symbols.join(',')` for stabilization, which is correct. However, `fetchPrices` (a `useCallback`) closes over `symbols` but doesn't list it as a dependency — if `symbols` reference changes but content is the same, `fetchPrices` is recreated but the effect may not re-run if the joined string is identical. This is actually correct behavior but fragile. |
| **Impact** | Low — current implementation works but is confusing to maintain. |
| **Recommended Fix** | Add a comment explaining the stabilization pattern, or use `useMemo` for the key. |
| **Test Plan** | Manual: verify prices update when switching symbol sets. |
| **Effort** | S |

### F-016 — Mobile: Silent Error Swallowing in BotDetailScreen
| Field | Value |
|-------|-------|
| **Category** | Reliability |
| **Severity** | Medium |
| **Confidence** | High |
| **Location** | `apps/mobile/src/screens/BotDetailScreen.tsx:52` |
| **Description** | `.catch(() => ({ messages: [] }))` silently swallows all errors including auth failures. User gets empty messages with no indication of failure. |
| **Impact** | Chat feature silently broken; user has no way to know. |
| **Recommended Fix** | Log the error: `.catch((err) => { console.warn('Chat fetch failed:', err.message); return { messages: [] }; })` |
| **Test Plan** | Test with network offline, verify console warning. |
| **Effort** | S |

### F-017 — API Client: `any` Types in Map Functions
| Field | Value |
|-------|-------|
| **Category** | Maintainability |
| **Severity** | Low |
| **Confidence** | High |
| **Location** | `packages/api-client/src/index.ts:398-453` |
| **Description** | `mapBot`, `mapBotConfig`, `mapChatMessage` all accept `raw: any`. No runtime validation. If the backend changes a field name, the mapping silently produces `undefined` values. |
| **Impact** | Type safety gap at the API boundary. |
| **Recommended Fix** | Use `zod` or runtime type guards for API responses. |
| **Test Plan** | Add snapshot tests for API response shapes. |
| **Effort** | M |

### F-018 — Unused Import Warning
| Field | Value |
|-------|-------|
| **Category** | Cleanup |
| **Severity** | Low |
| **Confidence** | High |
| **Location** | `services/bot-runner/src/gateway.rs:512` |
| **Description** | `use tempfile::tempdir;` is imported but unused. |
| **Impact** | Compiler warning. |
| **Recommended Fix** | Remove the import. |
| **Test Plan** | `cargo check` confirms no warning. |
| **Effort** | S |

### F-019 — `HomeOverviewScreen`: Promise.all Fails Atomically
| Field | Value |
|-------|-------|
| **Category** | Reliability |
| **Severity** | Medium |
| **Confidence** | High |
| **Location** | `apps/mobile/src/screens/HomeOverviewScreen.tsx:64-85` |
| **Description** | `Promise.all` for fetching metrics/events per bot. If one bot's request fails, all results are lost. |
| **Impact** | One bot's transient error causes entire overview to fail. |
| **Recommended Fix** | Use `Promise.allSettled` or individual `.catch()` wrappers. |
| **Test Plan** | Mock one bot's metrics endpoint to fail, verify other bots still display. |
| **Effort** | S |

### F-020 — Dead Code: `get_holdings()` Returns Empty Vec
| Field | Value |
|-------|-------|
| **Category** | Cleanup |
| **Severity** | Low |
| **Confidence** | High |
| **Location** | `services/bot-runner/src/executor.rs:836-843` |
| **Description** | `get_holdings()` is `#[deprecated]` and always returns `Ok(vec![])`. Marked dead but kept. |
| **Impact** | Code clutter. If accidentally called, returns incorrect data. |
| **Recommended Fix** | Remove entirely or change to return `Err("not implemented")`. |
| **Test Plan** | Grep for callers, verify none exist, remove. |
| **Effort** | S |

### F-021 — `QuoteCache` Excessive `#[allow(dead_code)]` Annotations
| Field | Value |
|-------|-------|
| **Category** | Cleanup |
| **Severity** | Low |
| **Confidence** | High |
| **Location** | `services/bot-runner/src/executor.rs:53, 129, 144, 150` |
| **Description** | Multiple methods marked `#[allow(dead_code)] // WIP`. `cleanup()`, `size()`, `spawn_cleanup_task()`, and `with_max_size()` are never called. |
| **Impact** | Dead code. `spawn_cleanup_task()` would actually be useful to call — expired entries are never cleaned. |
| **Recommended Fix** | Either call `spawn_cleanup_task()` in executor init, or remove the dead methods. |
| **Test Plan** | If enabling cleanup: verify cache size stays bounded over time. |
| **Effort** | S |

### F-022 — Docker Compose Uses Default Postgres Credentials
| Field | Value |
|-------|-------|
| **Category** | Security |
| **Severity** | Low |
| **Confidence** | High |
| **Location** | `docker-compose.yml:9-11` |
| **Description** | `POSTGRES_USER: postgres`, `POSTGRES_PASSWORD: postgres`. Clearly marked as "DEVELOPMENT ONLY" with comments, which is appropriate. |
| **Impact** | None if not used in production (comments indicate awareness). |
| **Recommended Fix** | Already handled with comments. Could add a startup check that refuses to start in production mode with default credentials. |
| **Test Plan** | N/A |
| **Effort** | S |

---

## 3. Proposed Patch Set

### Patch 1: Dead Code Removal & Cleanup
**Files touched:** 3 | **Risk:** None | **Effort:** S

1. Remove unused `tempfile::tempdir` import in `bot-runner/src/gateway.rs:512`
2. Remove deprecated `get_holdings()` from `bot-runner/src/executor.rs:836-843`
3. Remove unused `#[allow(dead_code)]` methods from `QuoteCache` that aren't needed, OR wire up `spawn_cleanup_task()` in executor initialization

### Patch 2: Bug Fixes
**Files touched:** 5 | **Risk:** Low | **Effort:** M

1. **F-002: Fix realized_pnl_today** — In `runner.rs`, after a confirmed sell trade, compute and accumulate realized PnL. Add UTC midnight reset logic.
2. **F-003: Fix config version race** — Wrap `update_bot_config` version increment in a transaction with `FOR UPDATE`.
3. **F-005: Fix hardcoded bot limit** — Replace `>= 4` with `>= sub.tier.max_bots()`.
4. **F-006: Fix silent decryption** — Add `warn!` logging when `decrypt()` fails in `sync.rs`.
5. **F-010: Fix cache TTL mismatch** — Align Redis TTL to 30 seconds in `data-retrieval/src/cache/mod.rs`.

### Patch 3: Performance Improvements
**Files touched:** 2 | **Risk:** Low | **Effort:** S

1. **F-001: Batch name availability** — Replace loop with single query returning all existing names matching prefix.
2. **F-007 + F-008: Batch inserts** — Use `unnest()` for metrics and events batch inserts.
3. **F-011: Pool settings** — Add `idle_timeout`, consider `min_connections`.

### Patch 4: Reliability & Hardening
**Files touched:** 4 | **Risk:** Low | **Effort:** M

1. **F-004: Replace unwrap() in engine.rs** — Use `const` Decimal values and proper error returns.
2. **F-009: Fix unsafe kill** — Check return value of `libc::kill`, add error logging.
3. **F-012: Fix Pyth precision** — Use Decimal arithmetic instead of f64 intermediate.
4. **F-013: Add backpressure** — Use `try_send()` with warning log on channel full.
5. **F-014: Add jitter** — Random jitter to reconnection backoff.

---

## 4. Summary Statistics

| Category | Critical | High | Medium | Low | Total |
|----------|----------|------|--------|-----|-------|
| Bug | 0 | 4 | 2 | 0 | 6 |
| Performance | 0 | 1 | 2 | 0 | 3 |
| Reliability | 0 | 1 | 3 | 0 | 4 |
| Security | 0 | 0 | 0 | 1 | 1 |
| Cleanup | 0 | 0 | 0 | 4 | 4 |
| Maintainability | 0 | 0 | 1 | 1 | 2 |
| **Total** | **0** | **6** | **8** | **6** | **22** |

### What's Working Well
- No SQL injection risk: all queries use parameterized bindings
- Proper authorization checks via `get_authorized_bot()` helper
- AES-256-GCM encryption for secrets at rest
- Bootstrap token has atomic one-time-use protection (race condition handled)
- Good concurrency control: semaphore for provisioning, advisory locks for cleanup
- Comprehensive event/metric pipeline with structured logging
- Well-designed paper trading simulation for safe testing
- Strong type safety with Rust enums for state machines
- Process-safety comments and targeted PID-based kill (vs `pkill`)

### Risks & Assumptions
- **No integration tests** requiring a live database — correctness of SQL queries depends on manual/staging testing
- **No load testing** evidence found — scalability of N+1 patterns untested
- **Mobile app has no ESLint config** — code quality enforcement is manual
- **No rate limiting on bot-facing endpoints** (`/bot/:id/config`, `/bot/:id/heartbeat`) — a compromised bot could DoS the control plane
