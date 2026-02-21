# Audit Fix Checklist

Generated from `docs/audit-report.md` on 2026-02-20. Items ordered by severity.

## Critical

- [x] **BR-001** PnL calculation mixes decimal scales for sell trades
  - Files: `services/bot-runner/src/executor.rs`, `services/bot-runner/src/runner.rs`
  - Fix: Added `compute_realized_price()` helper that normalizes raw amounts via token decimals before dividing. Used in both paper and live trade paths.
  - Test: 4 unit tests in `executor::tests` — sell normalization, buy normalization, zero-amount, PnL formula validation. All pass.
  - Verified: `cargo test` — 35/35 pass

- [ ] **DR-001** WebSocket channel replaced on reconnect — consumer stuck on dead channel
  - Files: `services/data-retrieval/src/sources/binance_ws.rs`
  - Fix: Use `tokio::sync::broadcast` instead of `mpsc`, or keep receiver stable across reconnects
  - Test: Disconnect WS, verify prices resume after reconnect

- [ ] **DR-002** Redis cache reconnection can deadlock under concurrent access
  - Files: `services/data-retrieval/src/cache/mod.rs`
  - Fix: Add `tokio::time::timeout()` wrapper on all cache ops; max 2 retries with backoff
  - Test: Simulate Redis down during concurrent requests

- [ ] **DR-003** WebSocket handler aborted via `abort()` — no graceful shutdown
  - Files: `services/data-retrieval/src/sources/binance_ws.rs`
  - Fix: Use `oneshot` shutdown signal instead of `abort()`
  - Test: Rapid reconnections don't corrupt state

## High

- [ ] **CP-001** Timing side-channel in bot auth token comparison
  - Files: `services/control-plane/src/middleware/bot_auth.rs`
  - Fix: Use `subtle::ConstantTimeEq` for token comparison
  - Test: Code review confirms constant-time comparison; cargo check passes

- [ ] **CP-002** Silent decryption failures return empty LLM API keys
  - Files: `services/control-plane/src/handlers/sync.rs`
  - Fix: Return INTERNAL_SERVER_ERROR on decryption failure; log as ERROR
  - Test: Verify error path returns 500 instead of empty string

- [ ] **CP-006** custom_assets has no validation — unbounded array, no format check
  - Files: `services/control-plane/src/models/mod.rs`
  - Fix: Add max 50 items, max 255 chars per item validation
  - Test: Test with 1000+ items; test with 300-char strings

- [x] **BR-002** Portfolio snapshot filters out unpriced positions
  - Files: `services/bot-runner/src/portfolio.rs`
  - Fix: Changed `filter_map` to `map`; unpriced positions included with zero market_value and `price_available: false`
  - Test: `test_snapshot_includes_unpriced_positions` — verifies unpriced position appears in snapshot with correct fields
  - Verified: `cargo test` — 57/57 pass

- [ ] **BR-003** Daily PnL reset uses UTC — bypassable near midnight
  - Files: `services/bot-runner/src/runner.rs`
  - Fix: Add configurable timezone offset; default to UTC with doc note
  - Test: PnL reset boundary test at 23:59 UTC

- [ ] **BR-005** Gateway health check blocks apply_config() for up to 30s
  - Files: `services/bot-runner/src/gateway.rs`
  - Fix: Reduce retry timeout; fail fast if gateway unavailable
  - Test: cargo check passes; review confirms non-blocking behavior

- [ ] **DR-004** Pyth batch requests always return confidence: None
  - Files: `services/data-retrieval/src/sources/pyth.rs`
  - Fix: Extract confidence calculation to helper; reuse in both paths
  - Test: Verify batch and single-price confidence match

- [ ] **DR-005** CoinGecko 429 retry thundering herd
  - Files: `services/data-retrieval/src/sources/coingecko.rs`
  - Fix: Share single retry-after deadline via `Arc<AtomicU64>`
  - Test: Verify concurrent requests share backoff deadline

- [ ] **DR-006** Invalid CORS origins silently filtered
  - Files: `services/data-retrieval/src/main.rs`
  - Fix: Fail loudly — refuse to start if any origin fails to parse
  - Test: Test with invalid origin string in env

- [ ] **MB-001** Token acquisition race condition during startup
  - Files: `apps/mobile/src/api/ApiProvider.tsx`
  - Fix: Use promise-based singleton for token acquisition
  - Test: Verify concurrent getAccessToken() calls return same promise

- [ ] **MB-002** Per-bot metric/event fetch errors completely swallowed
  - Files: `apps/mobile/src/screens/HomeOverviewScreen.tsx`
  - Fix: Add error logging and partial-load indicator
  - Test: Verify errors are logged; UI shows partial-load state

- [ ] **MB-003** Linking.openURL() called without URL validation
  - Files: `apps/mobile/src/screens/AuthScreen.tsx`, `apps/mobile/src/screens/BillingScreen.tsx`
  - Fix: Validate URLs against HTTPS whitelist before opening
  - Test: Verify non-HTTPS URLs are rejected

- [ ] **MB-010** Token expiry not checked before returning
  - Files: `apps/mobile/src/api/ApiProvider.tsx`
  - Fix: Check expiry; preemptively refresh within grace period
  - Test: Near-expired token triggers refresh

- [ ] **MB-011** 8 TypeScript errors in raw-types.ts
  - Files: `packages/api-client/src/raw-types.ts`
  - Fix: Add `?? ''` fallbacks for string|undefined fields
  - Test: `tsc --noEmit` passes clean

- [ ] **DR-008** Price cache eviction uses O(n log n) sort
  - Files: `services/data-retrieval/src/lib.rs`
  - Fix: Use `lru` crate or `indexmap` for O(1) eviction
  - Test: Benchmark with 10k entries

## Medium

- [ ] **CP-003** Subscription cache stores divergable is_active boolean
  - Files: `services/control-plane/src/middleware/subscription.rs`
  - Fix: Derive is_active from expires_at at serve time
  - Test: Cache with expired expires_at but is_active=true

- [ ] **CP-004** EventInput.event_type not validated against DB enum
  - Files: `services/control-plane/src/handlers/sync.rs`
  - Fix: Validate against allowed enum list before INSERT
  - Test: Request with invalid event_type returns 400

- [ ] **CP-005** Subscription middleware query may lack index
  - Files: `services/control-plane/migrations/`
  - Fix: Add migration for `idx_bots_user_id` if missing
  - Test: EXPLAIN ANALYZE confirms index usage

- [ ] **CP-007** AlgorithmFactorInput accepts NaN/Infinity weights
  - Files: `services/control-plane/src/models/mod.rs`
  - Fix: Validate `weight.is_finite()` and bounds [-100, 100]
  - Test: Test with NaN, Infinity, -200.0

- [ ] **CP-008** Bot registration status check is not atomic
  - Files: `services/control-plane/src/handlers/sync.rs`
  - Fix: Use `UPDATE ... WHERE status = 'provisioning'` and check rows_affected
  - Test: Verify concurrent registrations handled

- [ ] **CP-009** CORS accepts x-csrf-token but no CSRF validation
  - Files: `services/control-plane/src/main.rs`
  - Fix: Remove x-csrf-token from CORS allowed headers (not used)
  - Test: Verify CORS config no longer includes it

- [ ] **CP-011** Config fetch has no retry logic for DB blips
  - Files: `services/control-plane/src/handlers/sync.rs`
  - Fix: Wrap config fetch in retry (3 attempts with backoff)
  - Test: Verify retry behavior on transient error

- [ ] **BR-004** Portfolio snapshot called 3+ times per tick
  - Files: `services/bot-runner/src/runner.rs`
  - Fix: Compute once; pass through functions
  - Test: Profile tick confirms single snapshot call

- [ ] **BR-006** Malformed JSON propagates error through entire decision_tick
  - Files: `services/bot-runner/src/client.rs`
  - Fix: Catch parsing errors; skip update; log error
  - Test: Malformed JSON doesn't crash decision_tick

- [ ] **BR-007** Config ack failure causes re-application on restart
  - Files: `services/bot-runner/src/runner.rs`
  - Fix: Track applied version_id; skip re-apply if already applied
  - Test: Ack failure then retry doesn't duplicate init

- [x] **BR-008** OpenClaw intents not validated (same mint, negative amount)
  - Files: `services/bot-runner/src/runner.rs`
  - Fix: Added structural validation before risk rail checks: reject same-mint trades and non-positive amounts. Hold intents exempt.
  - Verified: `cargo test` — 57/57 pass; `cargo check` clean

- [x] **BR-009** claw-trader path not checked for execute permissions
  - Files: `services/bot-runner/src/executor.rs`
  - Fix: Added unix permission check for execute bit during initialization; warns if file exists but is not executable
  - Verified: `cargo test` — 57/57 pass

- [ ] **DR-007** Rate limiter uses proxy IP instead of client IP
  - Files: `services/data-retrieval/src/rate_limit.rs`
  - Fix: Extract from X-Forwarded-For if trusted proxy configured
  - Test: Verify client IP extracted behind proxy

- [ ] **DR-009** Pyth exponent clamped without warning
  - Files: `services/data-retrieval/src/sources/pyth.rs`
  - Fix: Log warning when clamping extreme exponents
  - Test: Test with expo=-50 logs warning

- [ ] **DR-010** Dropped WS price updates only logged as warning
  - Files: `services/data-retrieval/src/sources/binance_ws.rs`
  - Fix: Add atomic counter; expose in health endpoint
  - Test: Verify counter increments on channel-full

- [ ] **MB-004** API keys stored in component state (screen recording risk)
  - Files: `apps/mobile/src/screens/settings/AiProviderSettings.tsx`
  - Fix: Use SecureStore; only show masked versions
  - Test: Verify keys not visible in component state

- [ ] **MB-005** useBot loading stuck forever if botId is null
  - Files: `apps/mobile/src/hooks/useBots.ts`
  - Fix: Early return before setting loading if botId is undefined
  - Test: Verify loading=false when botId is null

- [ ] **MB-006** FlatList without maxToRenderPerBatch
  - Files: `apps/mobile/src/screens/HomeOverviewScreen.tsx`
  - Fix: Add `maxToRenderPerBatch={10}` and `updateCellsBatchingPeriod={50}`
  - Test: Profile render time with many bots

- [ ] **MB-007** Animation loops can stack on rapid re-renders
  - Files: `apps/mobile/src/components/AnimatedBotCard.tsx`
  - Fix: Use ref to prevent duplicate animation loops
  - Test: Rapid mount/unmount cycles don't leak

- [ ] **MB-009** Refresh intervals hardcoded across 6+ files
  - Files: Multiple mobile files
  - Fix: Centralize in `src/config/intervals.ts`
  - Test: Build compiles; all files use centralized constants

- [ ] **CI-001** Docker images tagged :latest only
  - Files: `.github/workflows/deploy.yml`
  - Fix: Tag with `${{ github.sha }}` in addition to latest
  - Test: Verify deploy tags with SHA

- [ ] **CI-002** Docker password shell expansion risk
  - Files: `.github/workflows/deploy.yml`
  - Fix: Ensure --password-stdin used correctly; add safety comment
  - Test: Review CI logs for leakage

- [ ] **DR-011** Critical paths untested
  - Files: `services/data-retrieval/` (multiple)
  - Fix: Add unit tests for reconnection, rate limiter, cache eviction
  - Test: New tests pass

## Low

- [ ] **CP-010** Debug endpoint exposes auth header prefix
  - Files: `services/control-plane/src/main.rs`
  - Fix: Replace with `[REDACTED] (N chars)`
  - Test: Verify debug endpoint output

- [ ] **CP-012** Inconsistent handler return types
  - Files: Multiple control-plane handlers
  - Fix: Standardize on `Result<Json<T>>` with error response type
  - Test: cargo check passes

- [x] **BR-010** Risk caps deserialized without bounds validation
  - Files: `services/bot-runner/src/config.rs`
  - Fix: Added `RiskCaps::validate()` method: position_size 1-100%, drawdown 1-100%, positive daily_loss and trades_per_day. Called in `from_response`.
  - Verified: `cargo test` — 57/57 pass

- [x] **BR-011** Clippy warnings: redundant import, clone on Copy
  - Files: `services/bot-runner/src/intent.rs`, `services/bot-runner/src/runner.rs`
  - Fix: Removed `use uuid;` (redundant single-component import); removed `.clone()` on Copy type `ExecutionConfig`
  - Verified: `cargo clippy --lib` — only 4 `too_many_arguments` warnings remain (structural, acceptable)

- [ ] **BR-012** runner.rs exceeds 500-line file limit
  - Files: `services/bot-runner/src/runner.rs`
  - Fix: Split into orchestrator, config_manager, decision_engine modules
  - Test: cargo build compiles after split

- [ ] **MB-008** "Failed to create boat" typo
  - Files: `apps/mobile/src/hooks/useBots.ts`
  - Fix: Change "boat" to "bot"
  - Test: Manual verification

- [ ] **CI-003** Dual camelCase/snake_case fallback pattern
  - Files: `packages/api-client/src/raw-types.ts`
  - Fix: Standardize naming convention from API
  - Test: tsc --noEmit passes

- [ ] **CI-004** Dev DB credentials committed in docker-compose.yml
  - Files: `docker-compose.yml`
  - Fix: Move to `.env` file (gitignored); add `.env.example`
  - Test: Verify .env not in git

---

**Total items: 48**
**Progress: 0/48 complete**
