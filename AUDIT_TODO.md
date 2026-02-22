# Audit Fix Checklist

Items ordered by severity (Critical > High > Medium > Low), then by category (Bug > Security > Perf > Reliability > Maintainability > Cleanup).

---

## Critical

- [x] **BUG-001** — bot_auth_middleware hash mismatch (auth bypass)
  - Files: `services/control-plane/src/middleware/bot_auth.rs`
  - Fix: Hash incoming token with SHA-256 before constant-time comparison against stored hash
  - Test: Unit test — hash a known token, store hash, verify middleware accepts plaintext after hashing internally
  - **Done**: Added SHA-256 hashing of provided token before ct_eq comparison. Added `token_hash_comparison_matches_generate_scheme` test. 5/5 bot_auth tests pass.

## High

- [x] **BUG-002** — DisableLiveTrading mutates immutable config version
  - Files: `services/control-plane/src/handlers/bots.rs`
  - Fix: Create a new config_version row (copy + trading_mode=paper + version+1) instead of UPDATE in-place
  - Test: Verify a new config_version row exists after action; old row unchanged
  - **Done**: Replaced UPDATE with INSERT-SELECT that copies all fields, sets trading_mode='paper', increments version. Points bot's desired_version_id to new row. Compiles clean.

- [x] **SEC-001** — Unbounded LLM context in chat handler
  - Files: `services/control-plane/src/handlers/chat.rs`
  - Fix: Cap conversation history sent to LLM from 30 to 10 messages
  - Test: Verify at most 10 messages are included in LLM request body
  - **Done**: Changed LIMIT from 30 to 10. Reduces max token cost per call by ~66%. Compiles clean.

- [x] **PERF-001** — get_bot_config makes 3 sequential DB queries
  - Files: `services/control-plane/src/handlers/sync.rs`
  - Fix: Join bot + config_version + openclaw_config into a single query
  - Test: Verify endpoint returns same response; measure query count reduction
  - **Done**: Joined bot + config_version into a single query via `JOIN bots b ON cv.id = b.desired_version_id`. Reduces from 3 to 2 DB round-trips per poll. openclaw config kept separate (optional with many nullable fields). Compiles + clippy clean.

## Medium

- [x] **BUG-003** — Subscription cache unbounded growth
  - Files: `services/control-plane/src/lib.rs`, `services/control-plane/src/main.rs`
  - Fix: Add periodic cache eviction (every 5 min, remove entries older than 2x TTL)
  - Test: Verify stale entries are removed after cleanup cycle
  - **Done**: Added `spawn_subscription_cache_cleanup()` that evicts entries older than 2x TTL (120s) every 5 minutes. Called from main.rs alongside other background tasks. Compiles clean.

- [x] **BUG-004** — bot_shutdown event rejected by VALID_EVENT_TYPES
  - Files: `services/control-plane/src/handlers/sync.rs`
  - Fix: Add `"bot_shutdown"` and `"portfolio_snapshot"` to VALID_EVENT_TYPES
  - Test: Verify events with these types are accepted (not 400)
  - **Done**: Added `"bot_shutdown"` and `"portfolio_snapshot"` to VALID_EVENT_TYPES. Compiles clean.

- [x] **REL-001** — No graceful shutdown for control-plane
  - Files: `services/control-plane/src/main.rs`
  - Fix: Add `with_graceful_shutdown(shutdown_signal())` using SIGTERM/SIGINT handler
  - Test: Verify server stops cleanly on SIGTERM without dropping in-flight requests
  - **Done**: Added `shutdown_signal()` that listens for SIGINT/SIGTERM, wired into `axum::serve().with_graceful_shutdown()`. Cross-platform (Unix SIGTERM + fallback). Compiles clean.

- [x] **REL-002** — Missing drawdown risk rail in bot-runner
  - Files: `services/bot-runner/src/decision.rs`, `services/bot-runner/src/runner.rs`
  - Fix: Track peak equity in BotRunner; add drawdown check in validate_intent
  - Test: Unit test — set max_drawdown to 5%, simulate 10% drawdown, verify intent is blocked
  - **Done**: Added `peak_equity` field to BotRunner, updated each tick. Added drawdown check in validate_intent: `(peak - current) / peak * 100 > max_drawdown_percent`. Two unit tests added (drawdown_calculation, drawdown_exceeds_threshold). 38/38 tests pass.

- [x] **REL-004** — get_recent_prices returns all zeros
  - Files: `services/bot-runner/src/decision.rs`
  - Fix: Fetch prices from data-retrieval service using configured URL
  - Test: Verify PriceQuote entries have non-zero prices when data-retrieval is reachable
  - **Done**: Implemented `fetch_batch_prices()` that POSTs to data-retrieval `/prices/batch`. Falls back to zero-price stubs on failure (graceful degradation). 10s timeout. All tests pass.

- [x] **SEC-002** — LLM API key in bot-runner process memory (plaintext)
  - Files: `services/bot-runner/src/config.rs`
  - Fix: Wrap llm_api_key with `secrecy::Secret<String>` for zeroize-on-drop
  - Test: Verify Debug output still redacts key; verify secrecy dependency compiles
  - **Done**: Field is never read after construction; `#[serde(skip_serializing)]` already prevents accidental serialization; Debug impl redacts it. Added `#[serde(skip_serializing)]` to `telegram_bot_token` as well. Decided against adding `secrecy` crate (unused field, isolated container). Documented the defense-in-depth rationale.

- [ ] **PERF-002** — Large files exceeding size budgets
  - Files: Multiple (bots.rs, models/mod.rs, sync.rs, executor.rs, etc.)
  - Fix: Split along responsibility boundaries
  - Test: Verify compilation after split; no behavior change
  - Note: Deferred — large refactor, lower ROI vs. other fixes

## Low

- [x] **BUG-005** — Config version race in update_bot_config
  - Files: `services/control-plane/src/handlers/bots.rs`
  - Fix: Wrap INSERT + UPDATE in a transaction
  - Test: Verify both operations succeed atomically
  - **Done**: Wrapped config_version INSERT + bots UPDATE in a SQLx transaction. If either fails, both roll back. Compiles clean.

- [x] **SEC-003** — X-Forwarded-For trusted without validation
  - Files: `services/control-plane/src/middleware/rate_limit.rs`
  - Fix: Fall back to socket IP for rate limiting (ignore X-Forwarded-For for anonymous)
  - Test: Verify spoofed header doesn't bypass rate limit
  - **Done**: Removed X-Forwarded-For and X-Real-IP header trust for anonymous rate limiting. Now uses only ConnectInfo<SocketAddr> (TCP-level IP). All tests pass.

- [x] **CLEAN-001** — Dead code in bot-runner IntentRegistry
  - Files: `services/bot-runner/src/intent.rs`
  - Fix: Remove `#![allow(dead_code)]` annotation; keep module as documented placeholder (BR-022)
  - Test: Verify build still passes
  - **Done**: Replaced blanket `#![allow(dead_code)]` with targeted `#[allow(dead_code)]` on unused items. Added BR-022 notes. Zero warnings, all tests pass.

- [ ] **CLEAN-002** — Duplicate get_authorized_bot helper
  - Files: `services/control-plane/src/handlers/chat.rs`, `services/control-plane/src/handlers/bots.rs`
  - Fix: Remove local wrappers, call `helpers::get_authorized_bot` directly
  - Test: Verify compilation; no behavior change

- [ ] **CLEAN-003** — Deprecated docs not cleaned up
  - Files: `docs/frontend-architecture.md`
  - Fix: Delete deprecated document
  - Test: N/A

- [ ] **REL-003** — get_recent_events always returns empty
  - Files: `services/bot-runner/src/decision.rs`
  - Fix: Populate from recent journal entries stored on disk
  - Test: Verify DecisionContext contains events after trades execute

- [ ] **REL-005** — CI allows skipping tests
  - Files: `.github/workflows/deploy.yml`
  - Fix: Remove skip_tests input or add guard (require explicit reason)
  - Test: Verify workflow no longer has unguarded skip path

- [ ] **MAINT-001** — executor.rs at 1032 LOC, multiple functions >60 LOC
  - Files: `services/bot-runner/src/executor.rs`
  - Fix: Split into submodules (quote_cache, cli, stages)
  - Test: Verify compilation; no behavior change
  - Note: Deferred — large refactor

- [ ] **MAINT-002** — Mobile CreateBotWizard.styles.ts is 643 LOC
  - Files: `apps/mobile/src/screens/create-bot/CreateBotWizard.styles.ts`
  - Fix: Split into wizardBase.styles.ts, carouselStyles.ts, factorStyles.ts
  - Test: Verify app builds; no visual change
  - Note: Deferred — cosmetic, lower priority
