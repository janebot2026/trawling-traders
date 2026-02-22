# Trawling Traders - Comprehensive Code Audit

**Date:** 2026-02-22
**Auditor:** Claude Code (Opus 4.6)
**Scope:** Full repository (control-plane, data-retrieval, bot-runner, mobile app, api-client, types, CI/CD, infra)

---

## 1. Executive Summary

### Build & Test Status
- All 3 Rust services compile cleanly (`cargo check` passes)
- 21 control-plane tests pass, 36 bot-runner tests pass
- Clippy clean on control-plane and data-retrieval; 4 `too_many_arguments` warnings on bot-runner
- TypeScript packages typecheck successfully

### Top 10 Issues (Ranked by Risk)

| Rank | ID | Category | Severity | Summary |
|------|-----|----------|----------|---------|
| 1 | BUG-001 | Bug | Critical | `bot_auth_middleware` compares hash-to-plaintext (auth bypass for bots) |
| 2 | SEC-001 | Security | High | Chat handler sends full conversation history to LLM (unbounded token cost + data leak) |
| 3 | BUG-002 | Bug | High | `DisableLiveTrading` mutates immutable config version in-place |
| 4 | PERF-001 | Perf | High | `get_bot_config` makes 3 sequential DB queries per 30s poll per bot |
| 5 | BUG-003 | Bug | Medium | Subscription cache never evicts entries (unbounded HashMap growth) |
| 6 | REL-001 | Reliability | Medium | No graceful shutdown for control-plane (background tasks orphaned) |
| 7 | SEC-002 | Security | Medium | LLM API key stored plaintext in bot-runner process memory |
| 8 | PERF-002 | Perf | Medium | `models/mod.rs` at 877 LOC, `bots.rs` at 1232 LOC exceed size limits |
| 9 | BUG-004 | Bug | Medium | `bot_shutdown` event type not in `VALID_EVENT_TYPES` allow-list |
| 10 | MAINT-001 | Maintainability | Medium | `executor.rs` at 1032 LOC, multiple functions >60 LOC |

### Biggest Performance Opportunities
1. Batch the 3 queries in `get_bot_config` into a single joined query (~3x fewer DB round-trips per bot per 30s)
2. Add indexes on `bot_chat_messages(bot_id, created_at)` if not present in migrations
3. Cache tradeable assets and assistant options (static data queried on every page load)

### Highest-Risk Bug
**BUG-001**: The `bot_auth_middleware` stores a SHA-256 hash of the bootstrap token in the DB, but compares it directly against the plaintext token from the request header using constant-time comparison. This means **no bot can ever authenticate** after the token is hashed, unless the `get_bot_secrets` endpoint (which correctly hashes before comparing) is the only path that works. The middleware likely only works because secrets endpoint validates via body JSON while the middleware checks the Authorization header — but if both use the same stored hash, the middleware comparison is broken.

### Quick Wins (< 1 hour each)
1. Fix `bot_shutdown` event type rejection (add to `VALID_EVENT_TYPES`)
2. Add subscription cache eviction (periodic cleanup in background task)
3. Cap chat history context window sent to LLM (last 10 messages, not 30)
4. Add `Content-Length` limit on bot event/metrics batch endpoints

---

## 2. Findings Table

### BUG-001: bot_auth_middleware hash mismatch
| Field | Value |
|-------|-------|
| **Category** | Bug / Security |
| **Severity** | Critical |
| **Confidence** | High |
| **Location** | `services/control-plane/src/middleware/bot_auth.rs:47-67` |
| **Description** | The `bot_auth_middleware` reads `bootstrap_token` from the `bots` table (which stores a SHA-256 hash per `bots.rs:491`) and compares it directly to the plaintext bearer token using `ct_eq`. The `get_bot_secrets` endpoint (`sync.rs:673`) correctly hashes the request token before comparing, but the middleware does not. This means the middleware either (a) always fails (if DB has hash), or (b) the token stored is actually plaintext (contradicting `generate_bootstrap_token`). |
| **Impact** | If the DB stores hashes: all bot-facing endpoints (heartbeat, config, events, wallet, register) fail auth. If plaintext: the secrets endpoint breaks instead. One code path is definitely wrong. |
| **Recommended Fix** | Hash the incoming token in `bot_auth_middleware` before comparison, matching `get_bot_secrets`. |
| **Test Plan** | Unit test: hash a known token, store hash, verify middleware accepts the plaintext token after hashing it internally. |
| **Effort** | S |

```rust
// bot_auth.rs:57-67 — BEFORE
match stored_token {
    Some(token) => {
        let is_equal = token.as_bytes().ct_eq(provided_token.as_bytes());
        // ...

// AFTER
match stored_token {
    Some(stored_hash) => {
        use sha2::{Digest, Sha256};
        let provided_hash = hex::encode(Sha256::digest(provided_token.as_bytes()));
        let is_equal = stored_hash.as_bytes().ct_eq(provided_hash.as_bytes());
        // ...
```

---

### BUG-002: DisableLiveTrading mutates immutable config version
| Field | Value |
|-------|-------|
| **Category** | Bug |
| **Severity** | High |
| **Confidence** | High |
| **Location** | `services/control-plane/src/handlers/bots.rs:1091-1111` |
| **Description** | `BotAction::DisableLiveTrading` does `UPDATE config_versions SET trading_mode = 'paper' WHERE id = (SELECT desired_version_id ...)`. Config versions are designed to be **immutable** (new versions are created on updates, with incrementing version numbers). Mutating in-place breaks audit trail and config hash validation (`sync.rs:181`). |
| **Impact** | Config hash becomes invalid for bots that already acked this version. Bot may repeatedly re-apply the "same" version because the hash changed server-side. Audit trail lost. |
| **Recommended Fix** | Create a new config version (copy existing, change `trading_mode` to `paper`, increment version) instead of mutating in-place. |
| **Test Plan** | Test that after DisableLiveTrading, a new config_version row exists with version = old + 1. |
| **Effort** | S |

---

### BUG-003: Subscription cache unbounded growth
| Field | Value |
|-------|-------|
| **Category** | Bug |
| **Severity** | Medium |
| **Confidence** | High |
| **Location** | `services/control-plane/src/lib.rs:86`, `src/middleware/subscription.rs:166-176` |
| **Description** | `subscription_cache: HashMap<Uuid, SubscriptionCacheEntry>` grows unboundedly. Entries are inserted on every unique user request but never evicted — only the TTL check skips stale reads, but the HashMap entries remain forever. |
| **Impact** | Memory leak proportional to unique users over time. With 10K users, ~1.6MB (acceptable). With 1M+, problematic. More critically, the `RwLock` write path scans an ever-growing map. |
| **Recommended Fix** | Add periodic eviction (e.g., every 5 minutes, remove entries older than 2x TTL) in a background task, similar to rate limiter cleanup. |
| **Test Plan** | Load test with many unique user IDs, verify memory stabilizes. |
| **Effort** | S |

---

### BUG-004: `bot_shutdown` event rejected by VALID_EVENT_TYPES
| Field | Value |
|-------|-------|
| **Category** | Bug |
| **Severity** | Medium |
| **Confidence** | High |
| **Location** | `services/control-plane/src/handlers/sync.rs:393-401`, `services/bot-runner/src/runner.rs:253` |
| **Description** | Bot-runner sends `event_type: "bot_shutdown"` during graceful shutdown, but `VALID_EVENT_TYPES` in `sync.rs` does not include `"bot_shutdown"`. Also missing: `"portfolio_snapshot"` (sent from `runner.rs:422`), `"config_applied"` events from bot-runner (already in list but the bot sends slightly different types). |
| **Impact** | Shutdown and portfolio events are silently rejected with 400 Bad Request. The bot logs a warning but the control plane never records these events. |
| **Recommended Fix** | Add `"bot_shutdown"` and `"portfolio_snapshot"` to `VALID_EVENT_TYPES`. |
| **Test Plan** | Trigger bot shutdown, verify event appears in the `events` table. |
| **Effort** | S |

---

### SEC-001: Unbounded LLM context in chat handler
| Field | Value |
|-------|-------|
| **Category** | Security / Cost |
| **Severity** | High |
| **Confidence** | High |
| **Location** | `services/control-plane/src/handlers/chat.rs:397-410` |
| **Description** | The chat handler sends the last 30 messages to the LLM provider. With 4000-char messages, this is up to 120K characters per request. At ~4 chars/token, that's ~30K tokens per call. With the user's API key, this could cost $0.30-$3.00 per message depending on provider. A malicious user could rack up significant costs on their own key. More importantly, the control-plane is making the LLM call synchronously — blocking a handler thread for 5-30 seconds. |
| **Impact** | High LLM cost per request, potential for abuse (30 requests/hour * 30K tokens = 900K tokens/hour per bot). Thread starvation under load. |
| **Recommended Fix** | (1) Cap context to last 10 messages. (2) Add token count estimation before calling LLM. (3) Consider making LLM calls async with a response webhook pattern instead of blocking. |
| **Test Plan** | Verify context is capped. Load test chat endpoint under concurrent requests. |
| **Effort** | S (cap) / M (async refactor) |

---

### SEC-002: LLM API key in bot-runner process memory
| Field | Value |
|-------|-------|
| **Category** | Security |
| **Severity** | Medium |
| **Confidence** | Medium |
| **Location** | `services/bot-runner/src/config.rs` (BotConfig.llm_api_key field) |
| **Description** | The LLM API key is fetched via bootstrap secrets and held as a plain `String` in memory. While the `Debug` impl redacts it (good), a core dump or `/proc/pid/mem` read could expose it. |
| **Impact** | Low probability (requires root on VPS), but secrets in memory are a defense-in-depth concern. |
| **Recommended Fix** | Use `secrecy::Secret<String>` wrapper to zero memory on drop and prevent accidental logging. |
| **Test Plan** | Verify `Debug` output redacts key. Verify key is zeroed after config update. |
| **Effort** | S |

---

### PERF-001: get_bot_config makes 3 sequential DB queries
| Field | Value |
|-------|-------|
| **Category** | Performance |
| **Severity** | High |
| **Confidence** | High |
| **Location** | `services/control-plane/src/handlers/sync.rs:48-121` |
| **Description** | Every 30 seconds, each bot polls `GET /bot/:id/config`. This handler makes 3 sequential queries: (1) SELECT bot, (2) SELECT config_version, (3) SELECT bot_openclaw_config. With 100 bots, that's 600 queries/minute for config polling alone. |
| **Impact** | DB connection pool pressure. At 20 max connections, 600 queries/min is manageable but fragile. |
| **Recommended Fix** | Join all 3 queries into one: `SELECT b.*, cv.*, oc.* FROM bots b JOIN config_versions cv ON ... LEFT JOIN bot_openclaw_config oc ON ...`. |
| **Test Plan** | Benchmark before/after with 100 simulated bots. Target: <50% query count. |
| **Effort** | M |

---

### PERF-002: Large files exceeding size budgets
| Field | Value |
|-------|-------|
| **Category** | Maintainability |
| **Severity** | Medium |
| **Confidence** | High |
| **Location** | Multiple files |
| **Description** | Per CLAUDE.md: "File hard limit: 500 lines. Split at ~400." Several files exceed this: `bots.rs` (1232), `models/mod.rs` (877), `sync.rs` (822), `executor.rs` (1032), `provisioning.rs` (603), `alerting.rs` (525), `admin.rs` (531), `intent.rs` (581), `gateway.rs` (558). |
| **Impact** | Reduced readability, harder to review PRs, increased merge conflicts. |
| **Recommended Fix** | Split along responsibility boundaries (e.g., `bots.rs` → `bots/create.rs`, `bots/actions.rs`, `bots/queries.rs`). |
| **Test Plan** | Verify compilation after split. No behavior change. |
| **Effort** | M-L |

---

### REL-001: No graceful shutdown for control-plane
| Field | Value |
|-------|-------|
| **Category** | Reliability |
| **Severity** | Medium |
| **Confidence** | High |
| **Location** | `services/control-plane/src/main.rs:94-99` |
| **Description** | `axum::serve` is called without a graceful shutdown signal. When the process receives SIGTERM (Docker stop), in-flight requests are dropped, background tasks (cleanup, retention, offline checker) are killed mid-operation, and DB connections may not be properly returned. |
| **Impact** | Orphaned DB connections, incomplete operations during deployment. |
| **Recommended Fix** | Use `axum::serve(...).with_graceful_shutdown(shutdown_signal())` with a SIGTERM/SIGINT handler. |
| **Test Plan** | Send SIGTERM to running server, verify in-flight requests complete. |
| **Effort** | S |

---

### REL-002: Missing drawdown risk rail in bot-runner
| Field | Value |
|-------|-------|
| **Category** | Bug |
| **Severity** | Medium |
| **Confidence** | High |
| **Location** | `services/bot-runner/src/decision.rs:302-376` |
| **Description** | `validate_intent` checks `max_trades_per_day`, `max_position_size_percent`, and `max_daily_loss_usd`, but does **not** check `max_drawdown_percent` even though it's part of `RiskCaps` and is sent to the bot. The drawdown rail is defined but never enforced. |
| **Impact** | Bots can exceed their configured maximum drawdown without being stopped. This is a safety-critical omission for a trading system. |
| **Recommended Fix** | Add drawdown calculation: `(peak_equity - current_equity) / peak_equity * 100` and compare against `max_drawdown_percent`. Requires tracking peak equity in BotRunner state. |
| **Test Plan** | Unit test: set max_drawdown to 5%, simulate 10% drawdown, verify intent is blocked. |
| **Effort** | M |

---

### CLEAN-001: Dead code in bot-runner IntentRegistry
| Field | Value |
|-------|-------|
| **Category** | Cleanup |
| **Severity** | Low |
| **Confidence** | High |
| **Location** | `services/bot-runner/src/intent.rs` (581 LOC) |
| **Description** | `IntentRegistry` is instantiated in `BotRunner::new()` but its core methods (`try_create`, `find_equivalent`) are never called from the execution path. The file has `#![allow(dead_code)]` at the top. BR-022 documents this as a placeholder. |
| **Impact** | 581 lines of unused code. |
| **Recommended Fix** | Either wire it into the execution path or remove it and add a TODO issue. Keeping 581 LOC of dead code in a production binary increases compile time and cognitive load. |
| **Test Plan** | N/A (removal) or wire into execution path and test. |
| **Effort** | S |

---

### CLEAN-002: Duplicate `get_authorized_bot` helper
| Field | Value |
|-------|-------|
| **Category** | Cleanup |
| **Severity** | Low |
| **Confidence** | High |
| **Location** | `bots.rs:168-176`, `chat.rs:46-54` |
| **Description** | Both `bots.rs` and `chat.rs` define their own `get_authorized_bot` wrapper that delegates to `helpers::get_authorized_bot`. The wrapper is identical in both files. |
| **Impact** | Trivial duplication. |
| **Recommended Fix** | Use `helpers::get_authorized_bot` directly or make the wrapper a shared function. |
| **Effort** | S |

---

### CLEAN-003: Deprecated docs not cleaned up
| Field | Value |
|-------|-------|
| **Category** | Cleanup |
| **Severity** | Low |
| **Confidence** | High |
| **Location** | `docs/frontend-architecture.md` |
| **Description** | Marked as "DEPRECATED (2026-02-20)" on line 3 but still in the repo. References outdated "6 MVP screens" while the app now has ~25 screens. |
| **Impact** | Confusion for new contributors. |
| **Recommended Fix** | Delete or move to `docs/archive/`. |
| **Effort** | S |

---

### REL-003: get_recent_events always returns empty
| Field | Value |
|-------|-------|
| **Category** | Reliability |
| **Severity** | Low |
| **Confidence** | High |
| **Location** | `services/bot-runner/src/decision.rs:455-457` |
| **Description** | `get_recent_events()` always returns `Vec::new()`. The `DecisionContext` sent to OpenClaw never contains event history, so the AI cannot learn from recent trade outcomes. |
| **Impact** | OpenClaw makes decisions without historical context, potentially repeating bad patterns. |
| **Recommended Fix** | Populate from journal entries or from control-plane event history. |
| **Effort** | M |

---

### REL-004: get_recent_prices returns all zeros
| Field | Value |
|-------|-------|
| **Category** | Reliability |
| **Severity** | Medium |
| **Confidence** | High |
| **Location** | `services/bot-runner/src/decision.rs:429-452` |
| **Description** | `get_recent_prices()` creates stub `PriceQuote` entries with `price_usd: Decimal::ZERO` and `source: "pending"`. The bot-runner never fetches actual prices from data-retrieval; it relies entirely on OpenClaw to fill them in. |
| **Impact** | If OpenClaw doesn't replace these stubs, all risk calculations using price data will be based on $0 prices. Position sizing based on equity * position_size_percent would be $0. |
| **Recommended Fix** | Fetch prices from the data-retrieval service URL configured in `Config`. |
| **Effort** | M |

---

### MAINT-002: Mobile app CreateBotWizard.styles.ts is 643 LOC
| Field | Value |
|-------|-------|
| **Category** | Maintainability |
| **Severity** | Low |
| **Confidence** | High |
| **Location** | `apps/mobile/src/screens/create-bot/CreateBotWizard.styles.ts` |
| **Description** | Exceeds the 500-line file limit. Contains styles for wizard, carousels, factors, dropdowns, and summary — multiple UI concerns in one file. |
| **Impact** | Hard to navigate. Risk of style conflicts. |
| **Recommended Fix** | Split into `wizardBase.styles.ts`, `carouselStyles.ts`, `factorStyles.ts`. |
| **Effort** | S |

---

### SEC-003: X-Forwarded-For trusted without validation
| Field | Value |
|-------|-------|
| **Category** | Security |
| **Severity** | Low |
| **Confidence** | Medium |
| **Location** | `services/control-plane/src/middleware/rate_limit.rs:141-161` |
| **Description** | Rate limiter trusts `X-Forwarded-For` header for anonymous rate limiting. An attacker can spoof this header to bypass rate limits by sending a different IP on each request. |
| **Impact** | Rate limiting for anonymous users can be bypassed. Authenticated rate limiting (by user_id) is unaffected. |
| **Recommended Fix** | Only trust `X-Forwarded-For` when behind a known proxy (configure trusted proxy list). |
| **Effort** | S |

---

### BUG-005: Config version race in update_bot_config
| Field | Value |
|-------|-------|
| **Category** | Bug |
| **Severity** | Low |
| **Confidence** | Medium |
| **Location** | `services/control-plane/src/handlers/bots.rs:951-1000` |
| **Description** | The config version INSERT uses `(SELECT COALESCE(MAX(version), 0) + 1)` which is safe against races at the SQL level (serialized within the INSERT). However, the subsequent `UPDATE bots SET desired_version_id = $1` is outside the INSERT, creating a brief window where `desired_version_id` points to a stale version if two concurrent config updates race. |
| **Impact** | Very unlikely in practice (single user updating one bot), but could cause a config update to be lost if two requests arrive simultaneously. |
| **Recommended Fix** | Use a transaction for the INSERT + UPDATE pair. |
| **Effort** | S |

---

### REL-005: CI allows skipping tests
| Field | Value |
|-------|-------|
| **Category** | Reliability |
| **Severity** | Low |
| **Confidence** | High |
| **Location** | `.github/workflows/deploy.yml` (workflow_dispatch skip_tests input) |
| **Description** | The deploy workflow allows skipping tests via manual dispatch. While flagged with a caution message, this creates a path for deploying untested code to production. |
| **Impact** | Broken code can reach production if tests are skipped. |
| **Recommended Fix** | Remove the skip option, or require additional approval (e.g., require two approvers for skip-test deploys). |
| **Effort** | S |

---

## 3. Proposed Patch Set

### Patch 1: Bug Fixes (Critical + High)

**Files touched:**
- `services/control-plane/src/middleware/bot_auth.rs` — Fix hash comparison (BUG-001)
- `services/control-plane/src/handlers/bots.rs` — Fix DisableLiveTrading to create new version (BUG-002)
- `services/control-plane/src/handlers/sync.rs` — Add missing event types (BUG-004)
- `services/bot-runner/src/decision.rs` — Add max_drawdown check (REL-002)

**Justification:** These are correctness issues affecting authentication, data integrity, and safety.

### Patch 2: Security + Reliability

**Files touched:**
- `services/control-plane/src/handlers/chat.rs` — Cap context to 10 messages (SEC-001)
- `services/control-plane/src/main.rs` — Add graceful shutdown handler (REL-001)
- `services/control-plane/src/middleware/subscription.rs` + `lib.rs` — Add cache eviction (BUG-003)

**Justification:** Prevents cost abuse, improves deployment reliability, fixes memory leak.

### Patch 3: Performance

**Files touched:**
- `services/control-plane/src/handlers/sync.rs` — Join 3 queries into 1 for get_bot_config (PERF-001)

**Justification:** Reduces DB pressure from bot polling by ~66%.

### Patch 4: Cleanup + Maintenance

**Files touched:**
- `services/bot-runner/src/intent.rs` — Remove or gate behind feature flag (CLEAN-001)
- `services/control-plane/src/handlers/chat.rs` + `bots.rs` — Deduplicate helper (CLEAN-002)
- `docs/frontend-architecture.md` — Delete deprecated doc (CLEAN-003)

**Justification:** Reduces dead code and duplication.

---

## Appendix: Files Reviewed

| Service | Files | LOC |
|---------|-------|-----|
| control-plane (Rust) | ~35 | ~14,000 |
| bot-runner (Rust) | 16 | ~5,700 |
| data-retrieval (Rust) | 10 | ~2,200 |
| mobile (React Native) | ~58 | ~6,000 |
| api-client (TypeScript) | 9 | ~650 |
| types (TypeScript) | 1 | ~360 |
| CI/CD + infra | ~10 | ~800 |
| **Total** | **~139** | **~29,700** |

### Commands Run
- `cargo check` — all 3 services pass
- `cargo test` — 21 + 36 tests pass (0 failures)
- `cargo clippy` — clean (control-plane, data-retrieval), 4 warnings (bot-runner: too_many_arguments)
- `npx tsc --noEmit` — types and api-client pass

### Risks & Assumptions
- BUG-001 severity depends on whether the DB actually stores hashes or plaintext. The code in `generate_bootstrap_token()` clearly produces hashes, so the middleware is likely broken.
- Performance estimates are rough; actual impact depends on bot fleet size.
- Mobile app was reviewed at the structural level; no runtime testing was performed.

### TODOs
1. Verify BUG-001 in a running environment (check what `bootstrap_token` column actually contains)
2. Load test `get_bot_config` with simulated bot fleet to quantify PERF-001
3. Review cedros-login and cedros-pay library internals (opaque dependencies, not audited)
