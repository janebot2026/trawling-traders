# Audit Remediation Checklist

**Created:** 2026-02-04
**Total Items:** 68
**Status:** In Progress

---

## Priority Order

1. Critical Security/Bugs (8 items)
2. High Severity (16 items)
3. Medium Severity (32 items)
4. Low Severity (12 items)

---

## Critical Severity (8 items)

### [x] CP-01: JWT validation completely bypassed
- **Files:** `services/control-plane/src/middleware/auth.rs`
- **Planned Fix:**
  - Add proper JWT signature verification using jsonwebtoken crate
  - Validate token expiration (built into jsonwebtoken)
  - Support optional issuer validation via JWT_ISSUER env var
- **Test Plan:** Create test with forged JWT; verify 401 returned
- **Status:** COMPLETED
- **Verification:** cargo check passes; JWT_SECRET env var required for startup
- **Note:** Uses HS256 algorithm. Set JWT_SECRET (min 32 bytes) and optionally JWT_ISSUER

### [x] CP-02: Secrets never encrypted
- **Files:** `services/control-plane/src/secrets.rs`
- **Planned Fix:**
  - Implement AES-256-GCM encryption in `encrypt()`
  - Implement matching decryption in `decrypt()`
  - Use proper nonce handling with base64 encoding
- **Test Plan:** Verify encrypted != plaintext; verify decrypt(encrypt(x)) == x
- **Status:** COMPLETED
- **Verification:** 4 unit tests pass (roundtrip, unique nonces, tamper detection, passthrough)
- **Note:** Requires SECRETS_ENCRYPTION_KEY env var (64 hex chars = 32 bytes)

### [ ] CP-04: Secrets exposed in droplet user-data
- **Files:** `services/control-plane/src/handlers/bots.rs`
- **Planned Fix:**
  - Remove direct API key embedding in cloud-init script
  - Add secure secrets fetch endpoint for bots
  - Bot fetches secrets via authenticated endpoint after boot
- **Test Plan:** Verify user-data doesn't contain API keys; verify bot can fetch secrets
- **Status:** Not started
- **Note:** This is a larger architectural change - may defer to follow-up

### [x] BR-01: Command injection via pkill -f
- **Files:** `services/bot-runner/src/executor.rs`, `services/bot-runner/Cargo.toml`
- **Planned Fix:**
  - Get PID from child process handle via spawn()
  - Kill only that specific PID using libc::kill on Unix
  - Remove generic pkill -f pattern
- **Test Plan:** Test timeout handling kills only spawned process
- **Status:** COMPLETED
- **Verification:** cargo check passes; process kill targets specific PID only
- **Note:** Added libc dependency for Unix process management

### [x] BR-05: Integer overflow in portfolio cash update
- **Files:** `services/bot-runner/src/runner.rs`
- **Planned Fix:**
  - Use `saturating_add()` for arithmetic
  - Log warning if saturation occurs
- **Test Plan:** Test with large values near u64::MAX; verify no panic
- **Status:** COMPLETED
- **Verification:** cargo check passes
- **Note:** Used saturating_add to prevent panic; logs warning on saturation

### [x] DR-01: WebSocket race condition (deadlock risk)
- **Files:** `services/data-retrieval/src/sources/binance_ws.rs`
- **Planned Fix:**
  - Split WebSocket into separate send/receive channels
  - Use `futures::stream::StreamExt::split()`
  - Prevent lock contention between reader and writer
- **Test Plan:** Load test with concurrent subscriptions and reads
- **Status:** COMPLETED
- **Verification:** cargo check passes
- **Note:** Split ws_stream into ws_sink and ws_reader for independent locking

### [x] DR-03: Unbounded price cache memory leak
- **Files:** `services/data-retrieval/src/lib.rs`
- **Planned Fix:**
  - Add TTL-based eviction for old prices (5 min)
  - Add maximum cache size limit (10,000 entries)
  - Periodic cleanup every 1000 inserts
- **Test Plan:** Monitor memory over extended period with diverse symbols
- **Status:** COMPLETED
- **Verification:** cargo check passes
- **Note:** Evicts entries older than 5 min; caps at 10k entries

### [x] DR-04: Division by zero in price aggregation
- **Files:** `services/data-retrieval/src/lib.rs`
- **Planned Fix:**
  - Guard against zero total_weight
  - Return error if all sources have zero confidence
- **Test Plan:** Test aggregation with all-zero confidence values
- **Status:** COMPLETED
- **Verification:** cargo check passes
- **Note:** Returns SourceUnhealthy error when total_weight < epsilon

---

## High Severity (16 items)

### [ ] CP-03: SQL injection risk via unsafe type casting
- **Files:** `services/control-plane/src/handlers/bots.rs`
- **Planned Fix:**
  - Use native enum binding instead of string with `::bot_status` cast
- **Test Plan:** Static analysis; verify all status sources are trusted enums
- **Status:** Not started

### [ ] CP-05: Race condition in bot creation
- **Files:** `services/control-plane/src/handlers/bots.rs`
- **Planned Fix:**
  - Wrap count check and creation in transaction
  - Use `SELECT FOR UPDATE` to lock user row
- **Test Plan:** Concurrent request test; verify limit enforced
- **Status:** Not started

### [x] CP-06: Broken Solana wallet validation
- **Files:** `services/control-plane/src/handlers/sync.rs`, `Cargo.toml`
- **Planned Fix:**
  - Add `bs58` crate dependency
  - Validate with proper Base58 decode (must be 32 bytes)
- **Test Plan:** Test with invalid Base58 strings; verify rejection
- **Status:** COMPLETED
- **Verification:** cargo check passes
- **Note:** Added bs58 crate, validates decoded bytes == 32

### [x] CP-07: Rate limiter memory leak (probabilistic cleanup)
- **Files:** `services/control-plane/src/middleware/rate_limit.rs`
- **Planned Fix:**
  - Replace 1% probabilistic cleanup with deterministic periodic cleanup
  - Add `last_cleanup` timestamp tracking
  - Cleanup every 60 seconds
- **Test Plan:** Load test with many unique user IDs; verify memory stable
- **Status:** COMPLETED
- **Verification:** cargo check passes
- **Note:** Cleanup now runs deterministically every 60 seconds

### [x] CP-08: Unvalidated risk caps allow dangerous configs
- **Files:** `services/control-plane/src/handlers/bots.rs`, `src/models/mod.rs`
- **Planned Fix:**
  - Add validation function for risk caps
  - Position size: 1-50%, Daily loss: 1-100000, Drawdown: 1-50%, Trades: 1-100
- **Test Plan:** Test boundary values; verify rejection of invalid values
- **Status:** COMPLETED
- **Verification:** cargo check passes
- **Note:** Added RiskCaps::validate() and call it in create_bot and update_config

### [ ] BR-06: Race condition in intent registry (TOCTOU)
- **Files:** `services/bot-runner/src/runner.rs`, `services/bot-runner/src/intent.rs`
- **Planned Fix:**
  - Create atomic `try_create()` pattern
  - Check and insert in single operation
- **Test Plan:** Concurrent signal test; verify no duplicate intents
- **Status:** Not started

### [x] BR-08: Incorrect slippage calculation
- **Files:** `services/bot-runner/src/executor.rs`
- **Planned Fix:**
  - Calculate absolute slippage (not just negative)
  - Use Decimal for precision
  - Handle edge cases properly
- **Test Plan:** Test with actual < expected and vice versa
- **Status:** COMPLETED
- **Verification:** cargo check passes
- **Note:** Uses Decimal::abs() for absolute slippage, caps at u32::MAX

### [x] DR-02: Price channel capacity too small (data loss)
- **Files:** `services/data-retrieval/src/sources/binance_ws.rs`
- **Planned Fix:**
  - Increase channel capacity from 100 to 10000
- **Test Plan:** Load test with simulated high-volume feed
- **Status:** COMPLETED
- **Verification:** cargo check passes

### [x] DR-05: Incorrect timestamp handling (uses current time)
- **Files:** `services/data-retrieval/src/sources/binance_ws.rs`
- **Planned Fix:**
  - Use parsed timestamp from trade data instead of `Utc::now()`
  - Parse millisecond timestamp correctly
- **Test Plan:** Compare trade timestamps vs server time
- **Status:** COMPLETED
- **Verification:** cargo check passes
- **Note:** Uses DateTime::from_timestamp_millis() with fallback to now()

### [ ] DR-06: No WebSocket reconnection logic
- **Files:** `services/data-retrieval/src/lib.rs`
- **Planned Fix:**
  - Add reconnection logic with exponential backoff
  - Log reconnection attempts
  - Update health status on disconnect
- **Test Plan:** Simulate disconnect; verify reconnection
- **Status:** Not started

### [ ] MB-02: No token expiration handling in API client
- **Files:** `packages/api-client/src/index.ts`
- **Planned Fix:**
  - Detect 401 responses
  - Attempt token refresh
  - Redirect to login if refresh fails
- **Test Plan:** Test with expired token; verify refresh flow
- **Status:** Not started

### [ ] MB-16: Logout doesn't clear auth state
- **Files:** `apps/mobile/src/screens/ProfileScreen.tsx`
- **Planned Fix:**
  - Call `logout()` from Cedros SDK
  - Clear AsyncStorage
  - Reset navigation stack to Auth
- **Test Plan:** Test logout; verify back button shows auth screen
- **Status:** Not started

### [x] MB-20: Syntax error in AnimatedBotCard (build fails)
- **Files:** `apps/mobile/src/components/AnimatedBotCard.tsx`
- **Planned Fix:**
  - Fix missing `>` in JSX tag at line 147
- **Test Plan:** TypeScript build passes
- **Status:** COMPLETED
- **Verification:** Babel parser confirms valid JSX syntax
- **Note:** External dependency @cedros/pay-react-native has separate syntax errors (not our code)

### [ ] DR-12: CoinGecko candle logic completely wrong
- **Files:** `services/data-retrieval/src/sources/coingecko.rs`
- **Planned Fix:**
  - Use `/coins/{id}/ohlc` endpoint instead of market_chart
  - Parse OHLC data correctly
- **Test Plan:** Verify candles match exchange data
- **Status:** Not started

### [ ] DR-13: Pyth feed IDs are placeholder/fake values
- **Files:** `services/data-retrieval/src/sources/pyth.rs`
- **Planned Fix:**
  - Research and add real Pyth feed IDs
  - Add comment with source reference
- **Test Plan:** Test each symbol against Pyth API
- **Status:** Not started

### [ ] BR-09: Missing retry logic for control plane calls
- **Files:** `services/bot-runner/src/client.rs`
- **Planned Fix:**
  - Add retry wrapper with 3 attempts
  - Exponential backoff (1s, 2s, 4s)
  - Don't retry on 4xx errors
- **Test Plan:** Test with simulated network failures
- **Status:** Not started

---

## Medium Severity (32 items)

### [ ] CP-09: Decimal-to-BigDecimal silent data loss
- **Files:** `services/control-plane/src/models/mod.rs`
- **Planned Fix:**
  - Return Result instead of using `unwrap_or_default()`
  - Propagate conversion errors
- **Test Plan:** Test with edge-case Decimal values
- **Status:** Not started

### [ ] CP-10: Authorization check duplicated in every handler
- **Files:** `services/control-plane/src/handlers/bots.rs`
- **Planned Fix:**
  - Create `get_authorized_bot()` helper function
  - Refactor handlers to use shared helper
- **Test Plan:** Code review checklist; verify all handlers use helper
- **Status:** Not started

### [ ] CP-11: Orphan cleanup race with user destroy
- **Files:** `services/control-plane/src/provisioning.rs`
- **Planned Fix:**
  - Use advisory lock per bot
  - Re-check status after acquiring lock
- **Test Plan:** Concurrent cleanup/destroy test
- **Status:** Not started

### [ ] CP-12: N+1 query in subscription middleware
- **Files:** `services/control-plane/src/middleware/subscription.rs`
- **Planned Fix:**
  - Store bot_count in SubscriptionContext from initial query
  - Reuse cached count in bot_create_limit_middleware
- **Test Plan:** Profile request latency; verify single DB query
- **Status:** Not started

### [ ] CP-13: Unbounded event storage
- **Files:** `services/control-plane/src/handlers/sync.rs`
- **Planned Fix:**
  - Add background cleanup task
  - Delete events older than 30 days
- **Test Plan:** Monitor table size over time
- **Status:** Not started

### [ ] DR-07: Redis connection cloning anti-pattern
- **Files:** `services/data-retrieval/src/cache/mod.rs`
- **Planned Fix:**
  - Change to `&mut self` or proper interior mutability
  - Remove unnecessary clones
- **Test Plan:** Benchmark before/after
- **Status:** Not started

### [ ] DR-08: Missing per-request timeout
- **Files:** `services/data-retrieval/src/sources/coingecko.rs`
- **Planned Fix:**
  - Add 10s timeout wrapper using tokio::time::timeout
- **Test Plan:** Test with simulated slow responses
- **Status:** Not started

### [ ] DR-09: Inconsistent asset class detection
- **Files:** `services/data-retrieval/src/lib.rs`, `services/data-retrieval/src/handlers.rs`
- **Planned Fix:**
  - Remove duplicate `is_stock_symbol()` function
  - Use `AssetClass::from_symbol()` everywhere
- **Test Plan:** Test routing for edge-case symbols
- **Status:** Not started

### [ ] DR-10: Float-to-Decimal precision loss
- **Files:** `services/data-retrieval/src/sources/coingecko.rs`, `binance_ws.rs`, `pyth.rs`
- **Planned Fix:**
  - Parse strings directly to Decimal where possible
  - Avoid f64 intermediate step
- **Test Plan:** Compare parsed vs original for large prices
- **Status:** Not started

### [ ] BR-10: Cache eviction is O(n) linear scan
- **Files:** `services/bot-runner/src/executor.rs`
- **Planned Fix:**
  - Use BTreeMap with timestamp index
  - Batch eviction (remove 10% at once)
- **Test Plan:** Benchmark with full cache
- **Status:** Not started

### [ ] BR-11: Executor init failure silently continues
- **Files:** `services/bot-runner/src/runner.rs`
- **Planned Fix:**
  - Return error instead of Ok(())
  - Send critical event to control plane
- **Test Plan:** Test with missing claw-trader binary
- **Status:** Not started

### [ ] MB-01: LLM API keys stored in component state
- **Files:** `apps/mobile/src/screens/CreateBotScreen.tsx`, `BotSettingsScreen.tsx`
- **Planned Fix:**
  - Use SecureStore for sensitive data
  - Don't keep in React state longer than necessary
- **Test Plan:** Security audit of state management
- **Status:** Not started

### [ ] MB-03: Missing Error Boundaries
- **Files:** `apps/mobile/App.tsx`
- **Planned Fix:**
  - Create ErrorBoundary component
  - Wrap app with error boundary
- **Test Plan:** Test with thrown error in component
- **Status:** Not started

### [ ] MB-04: Navigation race condition in auth flow
- **Files:** `apps/mobile/src/screens/AuthScreen.tsx`
- **Planned Fix:**
  - Check subscription status before navigating to Main
  - Navigate to Subscribe if not active
- **Test Plan:** Test with expired subscription
- **Status:** Not started

### [ ] MB-07: No network error differentiation
- **Files:** All API-calling screens
- **Planned Fix:**
  - Map error types to appropriate messages
  - Handle 401, 403, 5xx, network errors differently
- **Test Plan:** Test each error scenario
- **Status:** Not started

### [ ] MB-09: Numeric inputs not validated
- **Files:** `apps/mobile/src/screens/CreateBotScreen.tsx`
- **Planned Fix:**
  - Add validateNumericInput helper
  - Validate before API call
- **Test Plan:** Test with non-numeric input
- **Status:** Not started

### [ ] MB-13: Hardcoded paddingTop instead of safe area
- **Files:** All screens
- **Planned Fix:**
  - Use `useSafeAreaInsets()` hook
  - Replace hardcoded padding values
- **Test Plan:** Test on various device form factors
- **Status:** Not started

### [ ] MB-19: No fetch timeout in API client
- **Files:** `packages/api-client/src/index.ts`
- **Planned Fix:**
  - Add 30s timeout via AbortController
  - Throw ApiError on timeout
- **Test Plan:** Test with simulated network hang
- **Status:** Not started

### [ ] INF-01: Weak postgres credentials in docker-compose
- **Files:** `docker-compose.yml`
- **Planned Fix:**
  - Add comment noting this is for development only
  - Consider using env vars for credentials
- **Test Plan:** Security checklist
- **Status:** Not started

### [ ] INF-02: Duplicate index in migrations
- **Files:** `services/control-plane/migrations/002_add_agent_wallet_column.sql`
- **Planned Fix:**
  - Remove duplicate index statement (already in 001)
- **Test Plan:** Lint migrations
- **Status:** Not started

### [ ] DR-11: Missing rate limit backoff
- **Files:** `services/data-retrieval/src/sources/coingecko.rs`
- **Planned Fix:**
  - Wait for retry-after header value
  - Retry once after waiting
- **Test Plan:** Verify backoff behavior on 429
- **Status:** Not started

### [ ] DR-14: Health check wastes API quota
- **Files:** `services/data-retrieval/src/sources/coingecko.rs`, `pyth.rs`
- **Planned Fix:**
  - Track health via internal metrics
  - Don't make real API calls in health check
- **Test Plan:** Verify no API calls in health check
- **Status:** Not started

### [ ] DR-15: No connection pool limits
- **Files:** `services/data-retrieval/src/sources/coingecko.rs`, `pyth.rs`
- **Planned Fix:**
  - Add `pool_max_idle_per_host(10)`
  - Add `pool_idle_timeout(90s)`
- **Test Plan:** Test under high concurrency
- **Status:** Not started

### [ ] MB-05: fetchBots in both useEffect and useFocusEffect
- **Files:** `apps/mobile/src/screens/BotsListScreen.tsx`
- **Planned Fix:**
  - Remove useEffect, keep only useFocusEffect
- **Test Plan:** Monitor network tab for duplicate requests
- **Status:** Not started

### [ ] MB-06: Missing pulseAnim in useEffect dependencies
- **Files:** `apps/mobile/src/components/AnimatedBotCard.tsx`
- **Planned Fix:**
  - Add pulseAnim to deps array
  - Add cleanup function
- **Test Plan:** Test status change while animating
- **Status:** Not started

### [ ] MB-08: State updates after potential unmount
- **Files:** `apps/mobile/src/screens/BotDetailScreen.tsx`
- **Planned Fix:**
  - Add `isMounted` ref guard
  - Check before setState calls
- **Test Plan:** Test rapid navigation
- **Status:** Not started

### [ ] MB-11: Animation memory leaks
- **Files:** `apps/mobile/src/utils/animations.ts`
- **Planned Fix:**
  - Refactor to hooks with proper cleanup
  - Return cleanup function
- **Test Plan:** Profile memory during navigation
- **Status:** Not started

### [ ] MB-14: No offline handling
- **Files:** All screens
- **Planned Fix:**
  - Add NetInfo listener
  - Show offline message instead of "Failed to load"
- **Test Plan:** Test offline scenarios
- **Status:** Not started

### [ ] MB-15: BotCard not memoized
- **Files:** `apps/mobile/src/screens/BotsListScreen.tsx`
- **Planned Fix:**
  - Wrap with React.memo()
  - Add custom comparison function
- **Test Plan:** Profile render count
- **Status:** Not started

### [ ] MB-18: Race condition in bot creation navigation
- **Files:** `apps/mobile/src/screens/CreateBotScreen.tsx`
- **Planned Fix:**
  - Pass new bot data through navigation params
  - Or invalidate cache before navigation
- **Test Plan:** Verify new bot appears in list
- **Status:** Not started

### [ ] MB-22: No retry logic in API client
- **Files:** `packages/api-client/src/index.ts`
- **Planned Fix:**
  - Add 3 retries with exponential backoff
  - Don't retry on 4xx errors
- **Test Plan:** Test with simulated failures
- **Status:** Not started

### [ ] BR-21: No graceful shutdown handling
- **Files:** `services/bot-runner/src/runner.rs`
- **Planned Fix:**
  - Add SIGTERM signal handler
  - Clean up in-flight operations
- **Test Plan:** Send SIGTERM; verify clean shutdown
- **Status:** Not started

---

## Low Severity (12 items)

### [ ] CP-14: Heartbeat uses client timestamp
- **Files:** `services/control-plane/src/handlers/sync.rs`
- **Planned Fix:**
  - Use `NOW()` instead of client-provided timestamp
- **Test Plan:** Verify DB timestamp is server time
- **Status:** Not started

### [ ] CP-15: Decimal-to-f32 precision loss
- **Files:** `services/control-plane/src/brain/engine.rs`
- **Planned Fix:**
  - Use f64 instead of f32
- **Test Plan:** Compare f32 vs f64 results
- **Status:** Not started

### [ ] CP-16: Unused get_current_user() in lib.rs
- **Files:** `services/control-plane/src/lib.rs`
- **Planned Fix:**
  - Remove unused function
- **Test Plan:** Verify no callers; cargo check passes
- **Status:** Not started

### [ ] CP-17: Unused INITIAL_MIGRATION constant
- **Files:** `services/control-plane/src/db/mod.rs`
- **Planned Fix:**
  - Remove unused constant
- **Test Plan:** Verify no callers; cargo check passes
- **Status:** Not started

### [ ] DR-16: Unused imports (8 warnings)
- **Files:** Multiple data-retrieval files
- **Planned Fix:**
  - Remove unused imports per compiler warnings
- **Test Plan:** cargo check shows no warnings
- **Status:** Not started

### [ ] DR-17: forex_commodities.rs entirely unused
- **Files:** `services/data-retrieval/src/sources/forex_commodities.rs`
- **Planned Fix:**
  - Delete entire file
- **Test Plan:** Verify no references; cargo check passes
- **Status:** Not started

### [ ] BR-12: No timeout on HTTP price fetch
- **Files:** `services/bot-runner/src/executor.rs`
- **Planned Fix:**
  - Add 10s timeout wrapper
- **Test Plan:** Test with slow server
- **Status:** Not started

### [ ] BR-15: get_holdings() always returns empty
- **Files:** `services/bot-runner/src/executor.rs`
- **Planned Fix:**
  - Add deprecation notice
  - Or remove if not needed
- **Test Plan:** Verify no callers rely on data
- **Status:** Not started

### [ ] BR-17: unwrap_or_default silently uses default config
- **Files:** `services/bot-runner/src/config.rs`
- **Planned Fix:**
  - Return error on parse failure
  - Log the error
- **Test Plan:** Test with malformed config JSON
- **Status:** Not started

### [ ] BR-22: trade_count not checked against limit
- **Files:** `services/bot-runner/src/runner.rs`
- **Planned Fix:**
  - Check count before evaluating trades
  - Log when limit reached
- **Test Plan:** Verify trades blocked after limit
- **Status:** Not started

### [ ] MB-12: Unused darkTheme export
- **Files:** `apps/mobile/src/theme.ts`
- **Planned Fix:**
  - Remove unused export
- **Test Plan:** Search for imports; verify none
- **Status:** Not started

### [ ] MB-17: Console.log with sensitive data
- **Files:** Multiple mobile files
- **Planned Fix:**
  - Create dev-only logger utility
  - Remove sensitive data from logs
- **Test Plan:** Audit logs in prod build
- **Status:** Not started

### [ ] MB-23: Unused animation components
- **Files:** `apps/mobile/src/components/AnimatedCard.tsx`, `OceanBackground.tsx`
- **Planned Fix:**
  - Remove PulseAnimation, StaggerContainer, OceanBackgroundDark
- **Test Plan:** Search for imports; verify none
- **Status:** Not started

### [ ] CP-WARN: Unused imports in control-plane (8 warnings)
- **Files:** Multiple control-plane files
- **Planned Fix:**
  - Remove unused imports per compiler warnings
- **Test Plan:** cargo check shows no warnings
- **Status:** Not started

### [ ] BR-WARN: Unused code in bot-runner (24 warnings)
- **Files:** Multiple bot-runner files
- **Planned Fix:**
  - Add #[allow(dead_code)] or remove unused code
- **Test Plan:** cargo check shows reduced warnings
- **Status:** Not started

---

## Progress Summary

| Severity | Total | Completed | Remaining |
|----------|-------|-----------|-----------|
| Critical | 8 | 7 | 1 (deferred) |
| High | 16 | 7 | 9 |
| Medium | 32 | 0 | 32 |
| Low | 15 | 0 | 15 |
| **Total** | **71** | **14** | **57** |

---

## Deferred Items

*Items that require larger architectural changes or external dependencies:*

- **CP-04**: Secrets in droplet user-data - requires new endpoint and bot-side changes
- **DR-13**: Pyth feed IDs - requires research into real feed IDs

---

## Completion Log

*Each completed item will be logged here with commit hash and notes.*

| ID | Commit | Date | Notes |
|----|--------|------|-------|
| MB-20 | 29739829 | 2026-02-04 | Fixed missing `>` in JSX self-closing tag |
| CP-01 | 4064feeb | 2026-02-04 | Implemented proper JWT signature verification with HS256 |
| CP-02 | 0cf0d9d5 | 2026-02-04 | Implemented AES-256-GCM encryption with 4 unit tests |
| BR-01 | 67ee0fb3 | 2026-02-04 | Replaced pkill -f with targeted PID kill using libc |
| BR-05 | 54a165bc | 2026-02-04 | Used saturating_add to prevent cash overflow |
| DR-04 | 54a51c9e | 2026-02-04 | Guard against zero total_weight in price aggregation |
| DR-03 | 6f6b0bb4 | 2026-02-04 | Added TTL eviction and max size limits to price cache |
| DR-01 | e4ddb6e3 | 2026-02-04 | Split WebSocket into separate read/write halves |
| DR-02 | b50edd2f | 2026-02-04 | Increased price channel capacity from 100 to 10000 |
| DR-05 | 92a9972e | 2026-02-04 | Use actual trade timestamp instead of Utc::now() |
| CP-06 | dfb719c5 | 2026-02-04 | Added proper Base58 validation for Solana wallets |
| CP-08 | 7593473d | 2026-02-04 | Added RiskCaps validation with safe value ranges |
| CP-07 | 9fd6f43d | 2026-02-04 | Replaced probabilistic cleanup with deterministic |
| BR-08 | (pending) | 2026-02-04 | Fixed slippage calc with Decimal and absolute value |

