# Trawling Traders - Full Codebase Audit Report

**Date:** 2026-02-21
**Scope:** Entire repository (control-plane, bot-runner, data-retrieval, mobile app, packages, CI/CD)
**Build Status:** All Rust services compile clean. api-client has 8 TypeScript errors.

---

## 1. Executive Summary

### Top 10 Issues (Ranked by Risk)

| # | Area | Issue | Severity |
|---|------|-------|----------|
| 1 | bot-runner | PnL calculation uses wrong price dimension for sell trades | Critical |
| 2 | data-retrieval | WebSocket price channel replaced on reconnect — consumer stuck on dead channel | Critical |
| 3 | data-retrieval | Redis cache reconnection can deadlock under concurrent access | Critical |
| 4 | control-plane | Silent decryption failures return empty LLM API keys to bots | High |
| 5 | bot-runner | Portfolio snapshot filters out positions without current prices — OpenClaw sees incomplete state | High |
| 6 | bot-runner | Daily PnL reset uses UTC — risk caps bypassable near midnight in non-UTC timezones | High |
| 7 | data-retrieval | Pyth batch requests always return `confidence: None` — aggregation skewed | High |
| 8 | control-plane | Timing side-channel in bot auth token comparison | High |
| 9 | api-client | 8 TypeScript type errors in raw-types.ts (string\|undefined assigned to string) | High |
| 10 | mobile | Race condition in token acquisition during startup | High |

### Biggest Performance Opportunities

1. **bot-runner:** Portfolio snapshot computed 3+ times per tick — cache for tick duration
2. **data-retrieval:** In-memory price cache eviction uses O(n log n) sort — use LRU
3. **data-retrieval:** CoinGecko 429 retry causes thundering herd — share retry deadline
4. **control-plane:** Subscription middleware query on every authed request (60s cache miss)

### Quick Wins (< 1 hour each)

1. Fix api-client TypeScript errors in `raw-types.ts` (add `?? ''` fallbacks)
2. Fix bot-runner clippy warnings (remove `clone()` on Copy type, remove redundant import)
3. Add `confidence` to Pyth batch responses (extract to helper, reuse)
4. Fix "Failed to create boat" typo in `useBots.ts:190`
5. Remove hardcoded `#d6eefb` in `AppTabBar.tsx` — use theme color
6. Add `maxToRenderPerBatch` to mobile FlatList for large bot lists
7. Validate event_type enum values on sync endpoint before DB insert
8. Log decryption failures as ERROR not WARN in control-plane sync handler

---

## 2. Findings Table

### Control-Plane Service

| ID | Category | Severity | Confidence | Location | Description | Impact | Fix | Test Plan | Effort |
|----|----------|----------|------------|----------|-------------|--------|-----|-----------|--------|
| CP-001 | Security | High | High | middleware/bot_auth.rs:57 | Direct `==` comparison of tokens is not constant-time. sync.rs uses `subtle::ConstantTimeEq` but bot_auth.rs does not. | Timing side-channel could allow brute-forcing bootstrap tokens | Use `subtle::ConstantTimeEq` for token comparison | Unit test measuring timing variance | S |
| CP-002 | Reliability | High | High | handlers/sync.rs:54-84 | Decryption failures for LLM API keys return `String::new()` instead of erroring. `unwrap_or_default()` at lines 666-673 does the same. | Bots receive empty API keys and silently fail LLM integration | Return `INTERNAL_SERVER_ERROR` on decryption failure | Integration test with invalid encryption key | S |
| CP-003 | Bug | Medium | High | middleware/subscription.rs:105-125 | Cache stores independent `is_active` boolean that can diverge from `expires_at` computation | Downgraded users briefly retain access (up to 60s cache TTL) | Derive `is_active` from `expires_at` at serve time | Test cache with expired `expires_at` but `is_active=true` | S |
| CP-004 | Security | Medium | Medium | handlers/sync.rs:378 | `EventInput.event_type` is `String` without validation against DB enum values. Invalid values cause 500 instead of 400. | Confuses monitoring; bots get opaque errors | Validate against allowed enum list before INSERT | Test with invalid event_type string | S |
| CP-005 | Performance | Medium | Medium | middleware/subscription.rs:122-129 | LEFT JOIN between `subscriptions` and `bots` on `user_id` — verify index exists | Cache-miss queries are O(n) table scans on every authenticated request | Verify/add `idx_bots_user_id` index | Check EXPLAIN ANALYZE output | S |
| CP-006 | Security | High | Medium | models/mod.rs:520 | `custom_assets: Option<Vec<String>>` has no validation — unbounded array size, no format check | Memory DoS; invalid asset configs silently fail downstream | Add max 50 items, max 255 chars per item | Test with 1000+ items; test with 300-char strings | S |
| CP-007 | Security | Medium | Medium | models/mod.rs:571-574 | `AlgorithmFactorInput` accepts NaN/Infinity/out-of-range weights | Trading algorithm malfunction or crash | Validate `weight.is_finite()` and bounds [-100, 100] | Test with NaN, Infinity, -200.0 | S |
| CP-008 | Bug | Medium | High | handlers/sync.rs:458-469 | Bot registration status check is not atomic (SELECT then UPDATE) | Benign race: concurrent registrations both succeed | Use `UPDATE ... WHERE status = 'provisioning'` and check rows_affected | Test concurrent registration calls | S |
| CP-009 | Security | Medium | Low | main.rs:140-146 | CORS accepts `x-csrf-token` header but no middleware validates it | CSRF attacks if frontend auth is session-based | Add CSRF validation middleware for state-changing endpoints | Manual test with cross-origin POST | M |
| CP-010 | Security | Low | High | main.rs:396-406 | Debug endpoint exposes first 20 chars of Authorization header | Token prefix leaked in logs/responses if debug routes enabled in prod | Replace with `[REDACTED] (N chars)` | Verify debug endpoint output | S |
| CP-011 | Reliability | Medium | Medium | handlers/sync.rs:24-37 | Config fetch has no retry logic — brief DB blips cause bot failures | Bot heartbeat failures and status transitions to "Offline" | Wrap in `with_retry` (3 attempts) | Integration test with DB timeout | S |
| CP-012 | Maintainability | Low | High | Multiple handlers | Inconsistent return types: some `Result<Json<T>>`, others `Result<StatusCode>` | Harder to follow patterns; inconsistent API responses | Standardize on `Result<Json<T>>` with error response type | N/A | M |

### Bot-Runner Service

| ID | Category | Severity | Confidence | Location | Description | Impact | Fix | Test Plan | Effort |
|----|----------|----------|------------|----------|-------------|--------|-----|-----------|--------|
| BR-001 | Bug | Critical | High | runner.rs:692-716, executor.rs:847-849 | `realized_price = out_amount / in_amount` for sells mixes decimal scales (raw USDC / raw tokens). PnL formula then uses this wrong price. | Daily loss limits incorrect; trading decisions based on wrong PnL | Normalize: `USDC_per_token = (out_usdc / 10^6) / (in_tokens / 10^decimals)` | Unit test: sell 5 SOL at $200, verify PnL = (200-entry)*5 | M |
| BR-002 | Bug | High | High | portfolio.rs:174-214 | Positions with `current_price_usdc == None` are silently filtered from snapshot | OpenClaw sees incomplete portfolio; may suggest trades on invisible positions | Include positions with unknown prices as zero-value, with `price_missing` flag | Test snapshot with mix of priced/unpriced positions | S |
| BR-003 | Bug | High | Medium | runner.rs:673-684 | Daily PnL reset uses `Utc::now().date_naive()` — not timezone-aware | Users can bypass daily loss limits by trading near UTC midnight | Add configurable timezone offset to config | Test PnL reset at 23:59 UTC for EST user | S |
| BR-004 | Perf | Medium | High | runner.rs:518,620,1106 | `portfolio.snapshot()` called 3+ times per tick (decision, context, heartbeat) | Unnecessary computation every 60s | Cache snapshot for tick duration; pass through functions | Profile tick latency before/after | S |
| BR-005 | Reliability | High | Medium | gateway.rs:377-390 | Gateway health check timeout (10s) blocks `apply_config()`, freezing bot for up to 30s | Hung gateway freezes all trading | Move health checks to background task; fail fast | Test with hung gateway process | M |
| BR-006 | Reliability | Medium | High | client.rs:165-180 | Malformed JSON from control-plane propagates error through entire decision_tick | Bot locked in error state; no fallback to previous config | Log parsing errors; skip update; emit event | Test with malformed JSON response | S |
| BR-007 | Bug | Medium | Medium | runner.rs:298-301 | If `apply_config` succeeds but `ack_config` fails, control-plane will re-send config | Config re-applied on restart; duplicate initialization | Make ack idempotent by version_id | Test ack failure then retry | S |
| BR-008 | Security | Medium | High | runner.rs:530-555 | OpenClaw intents accepted without validating: same mint buy/sell, negative amounts, absurd sizes | Nonsensical or malicious trades could execute | Add intent validation: `input != output`, `amount > 0`, bounds check | Test with self-trade and negative amount | S |
| BR-009 | Reliability | Medium | Medium | executor.rs:254-265 | claw-trader path checked for existence but not execute permissions | Init succeeds but all trades fail with "Permission denied" | Check `fs::metadata().permissions().mode()` for execute bit | Test with non-executable file at path | S |
| BR-010 | Validation | Low | High | config.rs:156-161 | Risk caps deserialized without bounds validation (e.g., max_position_size_percent: 200) | Invalid configs accepted silently | Validate 0-100% range on position size, positive values for loss/drawdown | Test with out-of-range values | S |
| BR-011 | Cleanup | Low | High | intent.rs:365, runner.rs:316 | Clippy warnings: redundant `use uuid;`, `.clone()` on Copy type | Code clarity | Remove redundant import; remove `.clone()` call | `cargo clippy` passes clean | S |
| BR-012 | Maintainability | Low | High | runner.rs (1100+ lines) | Runner struct exceeds 500-line file limit from CLAUDE.md | Hard to navigate and maintain | Split into orchestrator, config_manager, decision_engine, event_collector | Build compiles after split | L |

### Data-Retrieval Service

| ID | Category | Severity | Confidence | Location | Description | Impact | Fix | Test Plan | Effort |
|----|----------|----------|------------|----------|-------------|--------|-----|-----------|--------|
| DR-001 | Bug | Critical | High | sources/binance_ws.rs:354-363 | On WebSocket reconnect, `price_rx` is replaced with new channel. Old consumer holds reference to dead channel. | After reconnect, real-time prices stop flowing permanently | Use `tokio::sync::broadcast` instead of `mpsc`, or don't replace receiver | Test: disconnect WS, verify prices resume after reconnect | M |
| DR-002 | Bug | Critical | High | cache/mod.rs:41-52 | Redis reconnect acquires write lock; retry acquires read lock. Partial failure can deadlock. No timeout on cache ops. | Cache operations hang indefinitely; requests timeout | Add `tokio::time::timeout()` wrapper; max 2 retries with backoff | Test with Redis down during concurrent requests | M |
| DR-003 | Bug | Critical | Medium | sources/binance_ws.rs:365-378 | Old message handler task aborted via `old_handle.abort()` — no graceful shutdown. May leave locks held. | Corrupted internal state; dropped price updates | Use `oneshot` shutdown signal instead of `abort()` | Test rapid reconnections don't corrupt state | M |
| DR-004 | Bug | High | High | sources/pyth.rs:249-279 | `get_prices_batch` always sets `confidence: None` while `get_price` computes it properly | Batch requests have artificially low confidence; aggregation skewed | Extract confidence calculation to helper; reuse in both paths | Test batch vs single price confidence equality | S |
| DR-005 | Reliability | High | Medium | sources/coingecko.rs:125-140 | Multiple concurrent requests hitting 429 all sleep independently then retry simultaneously | Thundering herd on recovery; exacerbates rate limiting | Share single retry-after deadline across concurrent requests | Load test with rate-limited responses | M |
| DR-006 | Security | High | High | main.rs:27-47 | Invalid CORS origins silently filtered out; operator may not notice | Misconfigured CORS policies silently degrade | Fail loudly: refuse to start if any origin fails to parse | Test with mix of valid/invalid origins | S |
| DR-007 | Security | Medium | High | rate_limit.rs:84-92 | Rate limiter uses `ConnectInfo<SocketAddr>` — behind reverse proxy, always sees proxy IP | Rate limiting ineffective or blocks legitimate NAT users | Extract from `X-Forwarded-For` if trusted proxy configured | Test behind Nginx proxy | S |
| DR-008 | Perf | High | Medium | lib.rs:227-238 | Price cache eviction uses `sort_unstable()` on full timestamp vec — O(n log n) | CPU spikes during high throughput | Use `lru` crate or `indexmap` for efficient eviction | Benchmark with 10k entries, 1000 updates/sec | M |
| DR-009 | Bug | Medium | Medium | sources/pyth.rs:159-171 | Exponent clamped to [-38, 38] without warning — extreme values silently corrupted | Prices for extreme-value assets off by orders of magnitude | Log warning when clamping; consider error instead | Test with expo=-50 | S |
| DR-010 | Reliability | Medium | Medium | sources/binance_ws.rs:267-276 | Dropped price updates (channel full) only logged as warning — no metric counter | Silent data loss; operators unaware prices are stale | Add atomic counter; expose in health endpoint | Monitor counter during load test | S |
| DR-011 | Testing | Medium | High | Multiple files | Critical paths untested: reconnection, rate limiting, cache eviction, error handling | Bugs in error paths go undetected until production | Add unit tests for reconnection, rate limiter, cache eviction | N/A | L |

### Mobile App (React Native)

| ID | Category | Severity | Confidence | Location | Description | Impact | Fix | Test Plan | Effort |
|----|----------|----------|------------|----------|-------------|--------|-----|-----------|--------|
| MB-001 | Bug | High | Medium | api/ApiProvider.tsx:15-30 | Token acquisition uses hardcoded retry timing (120ms, 150ms) — race condition on concurrent calls | Auth may fail intermittently during startup | Use promise-based approach that caches token acquisition | Test rapid concurrent getAccessToken() calls | M |
| MB-002 | Reliability | High | High | screens/HomeOverviewScreen.tsx:72-88 | Per-bot metric/event fetch errors completely swallowed — no logging, no user notification | Users see partial fleet data without knowing some bots failed | Add explicit error handling with retry; show partial-load indicator | Test with one bot returning 500 | S |
| MB-003 | Security | High | High | screens/AuthScreen.tsx:456, BillingScreen.tsx:72 | `Linking.openURL()` called on URLs from API responses without validation | Phishing attacks if API response is intercepted/modified | Validate URLs against whitelist before opening | Test with malicious URL in response | S |
| MB-004 | Security | Medium | High | screens/settings/AiProviderSettings.tsx:29-42 | API keys stored in local component state — vulnerable to screen recording, memory dumps | API keys exposed via screenshots or crash logs | Use SecureStore; only show masked versions in UI | Verify keys not in memory dump | M |
| MB-005 | Bug | Medium | Medium | hooks/useBots.ts:60 | `useBot` sets loading=true before checking if botId is defined — loading stuck forever if null | Spinner shown indefinitely for invalid botId | Check `if (!botId) return;` before setting loading | Test with undefined botId | S |
| MB-006 | Perf | Medium | Medium | screens/HomeOverviewScreen.tsx:206-227 | FlatList without `maxToRenderPerBatch` — renders all bots on initial load | Jank on screens with 50+ bots | Add `maxToRenderPerBatch={10}` | Profile render time with 100 bots | S |
| MB-007 | Perf | Medium | Medium | components/AnimatedBotCard.tsx:34-53 | Animation loops for provisioning status can stack on rapid re-renders | Memory leak in long-running sessions; CPU waste | Ensure cleanup always fires; deduplicate pulse loops | Test rapid mount/unmount cycles | S |
| MB-008 | Bug | Low | High | hooks/useBots.ts:190 | Error message says "Failed to create **boat**" instead of "bot" | Confusing user-facing error | Change to "Failed to create bot" | Manual verification | S |
| MB-009 | Maintainability | Medium | High | Multiple files | Refresh intervals hardcoded (30s, 120ms, 150ms) across 6+ files | Difficult to tune; inconsistent behavior | Centralize in `src/config/intervals.ts` | Build compiles after refactor | S |
| MB-010 | Security | High | Medium | api/ApiProvider.tsx:15-34 | `getToken`/`refreshToken` don't check token expiry before returning | Users get cryptic 401 errors during token refresh window | Check expiry; preemptively refresh within grace period | Test with near-expired token | M |
| MB-011 | TypeSafety | High | High | api-client/src/raw-types.ts:181-198 | 8 TypeScript errors: `string | undefined` assigned to `string` fields | Build broken with strict TypeScript | Add `?? ''` fallbacks or make fields optional | `tsc --noEmit` passes | S |

### Packages / CI / Infrastructure

| ID | Category | Severity | Confidence | Location | Description | Impact | Fix | Test Plan | Effort |
|----|----------|----------|------------|----------|-------------|--------|-----|-----------|--------|
| CI-001 | Security | Medium | High | .github/workflows/deploy.yml:27,29 | Docker images tagged with `:latest` only — no immutable version tags | Can't rollback to specific version; cache invalidation issues | Tag with `${{ github.sha }}` in addition to latest | Verify deploy with SHA tag | S |
| CI-002 | Reliability | Medium | Medium | .github/workflows/deploy.yml:221 | Docker password passed via `echo '${{ secrets.DOCKER_PASSWORD }}'` — shell expansion risk | Secret could leak if shell escaping fails | Use `--password-stdin` with heredoc or file-based approach | Review CI logs for leakage | S |
| CI-003 | Maintainability | Low | High | api-client/src/raw-types.ts | Dual camelCase/snake_case fallback pattern (`response.todayPnl ?? response.today_pnl`) repeated everywhere | Fragile; maintenance burden; easy to miss fields | Standardize on one naming convention from API | N/A | M |
| CI-004 | Security | Low | High | docker-compose.yml | Dev DB credentials (postgres/postgres) committed — labeled "DEV ONLY" but still present | Risk of accidental production use | Move to `.env` file (gitignored) | Verify .env not in git | S |

---

## 3. Proposed Patch Set

### Patch 1: Dead Code Removal & Dependency Cleanup

**Scope:** Bot-runner clippy fixes, unused imports, dead_code audit

**Files:**
- `services/bot-runner/src/intent.rs` — Remove redundant `use uuid;` (line 365)
- `services/bot-runner/src/runner.rs` — Remove `.clone()` on Copy type (line 316)
- `apps/mobile/src/components/OceanBackground.tsx` — Remove unused `useColorScheme` import
- `apps/mobile/src/store/index.ts` — Remove unused `useAppStore` if confirmed dead

**Effort:** S (< 30 min)
**Risk:** None

### Patch 2: Bug Fixes (grouped by area)

#### 2a: Critical — PnL Calculation Fix (bot-runner)
```rust
// executor.rs: Fix realized_price to use proper USDC/token dimension
// Before:
realized_price: Decimal::from(out_amount) / Decimal::from(in_amount)
// After:
realized_price: {
    let decimals = if side == TradeSide::Sell { output_decimals } else { input_decimals };
    let normalized_out = Decimal::from(out_amount) / Decimal::from(10u64.pow(output_decimals as u32));
    let normalized_in = Decimal::from(in_amount) / Decimal::from(10u64.pow(input_decimals as u32));
    if normalized_in > Decimal::ZERO { normalized_out / normalized_in } else { Decimal::ZERO }
}
```

#### 2b: Critical — WebSocket Channel Fix (data-retrieval)
Replace `mpsc` with `broadcast` channel for price distribution, or stop replacing receiver on reconnect.

#### 2c: High — Silent Decryption Failures (control-plane)
```rust
// Before:
let key = state.secrets.decrypt(&cfg.encrypted_llm_api_key).unwrap_or_default();
// After:
let key = state.secrets.decrypt(&cfg.encrypted_llm_api_key)
    .map_err(|e| {
        tracing::error!(bot_id = %bot_id, "Failed to decrypt LLM API key: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Decryption failure".to_string())
    })?;
```

#### 2d: High — TypeScript Type Errors (api-client)
```typescript
// raw-types.ts: Add ?? '' fallbacks for all string|undefined fields
todayPnl: Number(raw.todayPnl ?? raw.today_pnl ?? 0),
status: raw.status ?? '',  // was: raw.status (string | undefined)
```

#### 2e: High — Pyth Batch Confidence (data-retrieval)
Extract confidence calculation from `get_price()` into `compute_confidence()` helper, call from both `get_price()` and `get_prices_batch()`.

#### 2f: Medium — Bot Auth Timing Side-Channel (control-plane)
```rust
use subtle::ConstantTimeEq;
let is_equal = stored_token.as_bytes().ct_eq(provided_token.as_bytes());
if !bool::from(is_equal) {
    return Err(StatusCode::UNAUTHORIZED);
}
```

#### 2g: Low — Typo Fix (mobile)
```typescript
// useBots.ts:190 — "boat" → "bot"
```

**Effort:** M (2-4 hours total)
**Risk:** Low-Medium (PnL fix needs careful testing)

### Patch 3: Performance Improvements

#### 3a: Cache Portfolio Snapshot (bot-runner)
```rust
// Compute once per tick, pass to all consumers
let tick_snapshot = self.portfolio.snapshot();
// Pass &tick_snapshot to decision_tick, build_decision_context, send_heartbeat
```

#### 3b: LRU Price Cache (data-retrieval)
Replace `HashMap` + sort-based eviction with `lru` crate for O(1) eviction.

#### 3c: CoinGecko Thundering Herd Fix (data-retrieval)
Share retry-after deadline via `Arc<AtomicU64>` across concurrent requests.

#### 3d: Mobile FlatList Optimization
```tsx
<FlatList
  maxToRenderPerBatch={10}
  updateCellsBatchingPeriod={50}
  // ... existing props
/>
```

**Effort:** M (2-3 hours total)
**Risk:** Low

### Patch 4: Refactors & Tests

#### 4a: Input Validation (control-plane + bot-runner)
- Validate `custom_assets` array size and string length
- Validate `AlgorithmFactorInput.weight` is finite and bounded
- Validate `event_type` against allowed enum values
- Validate OpenClaw intent: input != output mint, amount > 0

#### 4b: Redis Cache Timeout (data-retrieval)
Wrap all Redis operations in `tokio::time::timeout(Duration::from_secs(5), ...)`.

#### 4c: Graceful WebSocket Shutdown (data-retrieval)
Replace `old_handle.abort()` with `oneshot` shutdown signal.

#### 4d: Test Coverage
Priority test additions:
- Bot-runner: PnL calculation with sell trades (various decimal scales)
- Bot-runner: Risk rail boundary conditions (position size at exactly 100%)
- Data-retrieval: Redis reconnection under concurrent access
- Data-retrieval: WebSocket reconnection price flow continuity
- Control-plane: Event type validation

**Effort:** L (4-8 hours total)
**Risk:** Low

---

## Appendix: Build & Lint Results

### Rust (cargo check + clippy)
- **control-plane:** Clean (0 warnings)
- **data-retrieval:** Clean (0 warnings)
- **bot-runner:** 6 warnings
  - `too_many_arguments` (3x in intent.rs — structural, acceptable)
  - `clone_on_copy` (runner.rs:316 — easy fix)
  - `single_component_path_imports` (intent.rs:365 — easy fix)

### TypeScript (tsc --noEmit)
- **packages/types:** Clean (0 errors)
- **packages/api-client:** 8 errors in `raw-types.ts` (lines 181-198)
  - All are `string | undefined` not assignable to `string`
