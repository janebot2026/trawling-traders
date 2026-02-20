# Audit Remediation Checklist — Round 5

**Total findings:** 90 | **Fixed:** 15 | **Deferred:** 0

**Previous rounds:** Rounds 1–4 fixed 186 findings (134 in Round 4, 2 deferred: CP-018 reqwest 0.12, MB-032 Expo SDK).

---

## Critical (2)

- [x] **R5-INFRA-001** — `fetchDataApi` infinite recursion on network errors
  - Files: `packages/api-client/src/http.ts:181-206`
  - Fix: Add `_retryCount` parameter; limit to 1 retry; throw `NetworkError` on exhaustion
  - Test: Reasoned check — verify recursive call passes incremented counter; `npx tsc --noEmit`
  - **Done:** Added `_retryCount` param (default 0); guard retry with `_retryCount < 1`; throw `NetworkError` on exhaustion.

- [x] **R5-BR-001** — Portfolio not updated after trades within tick
  - Files: `services/bot-runner/src/runner.rs:528-556`
  - Fix: Track committed amounts in `HashMap<String, Decimal>` within tick loop; add to existing exposure in `validate_intent`
  - Test: `cargo check`; reasoned check of position-limit enforcement across multiple intents in single tick
  - **Done:** Added `committed_usd` HashMap tracking per-output-mint committed amounts. `validate_intent` now includes tick-committed amounts in position-size check. Also fixes R5-BR-006.

## High (12)

- [x] **R5-CP-001** — Bootstrap token comparison not constant-time
  - Files: `services/control-plane/src/handlers/sync.rs:596-599`, `Cargo.toml`
  - Fix: Add `subtle = "2"` dep; use `ct_eq` for hash comparison
  - Test: `cargo check`; reasoned check — timing oracle eliminated
  - **Done:** Added `subtle = "2"` dep; replaced `!=` with `ct_eq` from `subtle::ConstantTimeEq`.

- [x] **R5-BR-002** — `fetch_price_http` hardcodes SOL decimals (9) and USDC output mint
  - Files: `services/bot-runner/src/executor.rs:455-476`
  - Fix: Look up token decimals from `amount::get_token_info`; use actual `output_mint` parameter
  - Test: `cargo check`; reasoned check — non-SOL tokens get correct decimal scaling
  - **Done:** Uses `get_token_info` for both input/output decimals; uses actual `output_mint` param instead of hardcoded USDC.

- [x] **R5-BR-003** — Executor config never refreshed after creation
  - Files: `services/bot-runner/src/runner.rs:280-310`, `executor.rs`
  - Fix: Add `update_execution_config` method to `TradeExecutor`; call on config change when executor exists
  - Test: `cargo check`; reasoned check — config changes take effect without restart
  - **Done:** Added `update_execution_config` to TradeExecutor; `apply_config` now updates existing executor instead of skipping.

- [x] **R5-BR-004** — Jupiter API key passed as CLI argument (visible in `ps aux`)
  - Files: `services/bot-runner/src/executor.rs:279-280`
  - Fix: Pass via environment variable instead of `--api-key` CLI arg
  - Test: `cargo check`; reasoned check — key no longer in `/proc/[pid]/cmdline`
  - **Done:** Replaced `cmd.arg("--api-key").arg(api_key)` with `cmd.env("JUPITER_API_KEY", api_key)`.

- [x] **R5-INFRA-002** — `createBot` returns unmapped snake_case response
  - Files: `packages/api-client/src/bots.ts:60-87`
  - Fix: Wrap `fetchApi` result with `mapBot(response)`
  - Test: `npx tsc --noEmit`; reasoned check — `bot.userId` no longer `undefined`
  - **Done:** Stored `fetchApi` result in `response`, return `mapBot(response)`.

- [x] **R5-INFRA-003** — Error subclasses `instanceof` broken under Babel
  - Files: `packages/api-client/src/errors.ts:1-59`
  - Fix: Add `Object.setPrototypeOf(this, new.target.prototype)` to each constructor
  - Test: `npx tsc --noEmit`; reasoned check — `instanceof` works with Babel/Metro transpilation
  - **Done:** Added `Object.setPrototypeOf(this, new.target.prototype)` to all 7 error class constructors.

- [x] **R5-DR-001** — Redis cache never initialized
  - Files: `services/data-retrieval/src/main.rs:87-95`
  - Fix: Call `.with_cache()` during aggregator initialization when `REDIS_URL` is set
  - Test: `cargo check`; reasoned check — cache used when Redis available
  - **Done:** Added Redis cache initialization when REDIS_URL env var is set; graceful fallback on connection failure.

- [x] **R5-CP-002** — Subscription middleware blocks all GET routes for unpaid users
  - Files: `services/control-plane/src/middleware/subscription.rs:82-153`
  - Fix: Allow GET requests (read-only) through; only block mutating operations (POST/PUT/PATCH/DELETE)
  - Test: `cargo check`; reasoned check — free-tier users can still read their data
  - **Done:** Added `request.method() != Method::GET` guard to subscription check; GET requests pass through for inactive users.

- [x] **R5-BR-005** — Integer overflow in reconciler: `on_chain_amount as i64 - pos.quantity_raw as i64`
  - Files: `services/bot-runner/src/reconciler.rs:207`
  - Fix: Use `i128` for the difference calculation
  - Test: `cargo check`; reasoned check — large token amounts don't overflow
  - **Done:** Changed `diff_raw` field type to `i128`; casts use `as i128` for full u64 range.

- [x] **R5-CP-003** — `get_current_user` fabricates `created_at`/`updated_at` with `Utc::now()`
  - Files: `services/control-plane/src/handlers/bots.rs:1149-1164`
  - Fix: Read actual user record timestamps from DB
  - Test: `cargo check`; reasoned check — timestamps reflect reality
  - **Done:** Replaced in-memory User construction with `sqlx::query_as` from users table; removed fake Utc::now() timestamps.

- [x] **R5-INFRA-004** — GitHub Actions pinned to mutable tags (supply-chain risk)
  - Files: `.github/workflows/deploy.yml`
  - Fix: Pin all actions to full commit SHAs with version comments
  - Test: YAML syntax check; reasoned check
  - **Done:** All 13 action references pinned to full 40-char SHAs with version comments. No mutable tags remain.

- [x] **R5-MB-001** — Behavior config save destroys LLM API key (sends masked value)
  - Files: `apps/mobile/src/screens/BotBehaviorConfigScreen.tsx:67`
  - Fix: Exclude `llmApiKey` from update payload
  - Test: `npx tsc --noEmit`; reasoned check — masked value never sent to server
  - **Done:** Removed `llmApiKey` from the update payload; the server-returned masked value no longer overwrites the real key.

## Medium (35)

- [ ] **R5-CP-004** — Reports query fetches ALL events with no LIMIT
  - Files: `services/control-plane/src/handlers/reports.rs:144-156`
  - Fix: Add `LIMIT 50000` to query
  - Test: `cargo check`; reasoned check

- [ ] **R5-CP-005** — `RotateSecrets` discards plaintext bootstrap token
  - Files: `services/control-plane/src/handlers/bots.rs:1072-1086`
  - Fix: Return the new plaintext token in response before hashing for storage
  - Test: `cargo check`; reasoned check

- [ ] **R5-CP-006** — All unauthenticated requests share single rate-limit bucket
  - Files: `services/control-plane/src/middleware/rate_limit.rs:141-143`
  - Fix: Use IP address as rate-limit key for anonymous requests
  - Test: `cargo check`; reasoned check

- [ ] **R5-CP-007** — LLM chat endpoint: no rate limiting, no cost cap
  - Files: `services/control-plane/src/handlers/chat.rs`
  - Fix: Add per-bot rate limit (e.g., 30 req/hour) via existing rate limiter
  - Test: `cargo check`; reasoned check

- [ ] **R5-CP-008** — `list_bots` returns `total = bots.len()` but query uses LIMIT
  - Files: `services/control-plane/src/handlers/bots.rs:210`
  - Fix: Add a separate `COUNT(*)` query for true total, or document as page count
  - Test: `cargo check`; reasoned check

- [ ] **R5-CP-009** — Cedros-login middleware logs first 25 chars of auth headers
  - Files: `services/control-plane/src/main.rs:434-451`
  - Fix: Remove or fully redact the auth header from log output
  - Test: `cargo check`; reasoned check

- [ ] **R5-DR-002** — `get_stock_prices_batch` calls individual `get_price` in loop
  - Files: `services/data-retrieval/src/lib.rs:399-421`
  - Fix: Use Pyth batch endpoint for stock/metals symbols
  - Test: `cargo check`; reasoned check

- [ ] **R5-DR-003** — Health endpoint always returns 200 even when degraded
  - Files: `services/data-retrieval/src/handlers.rs:168-181`
  - Fix: Return 503 when all upstream sources are failing
  - Test: `cargo check`; reasoned check

- [ ] **R5-DR-004** — Redis connection stored once; no reconnection strategy
  - Files: `services/data-retrieval/src/cache/mod.rs:1-58`
  - Fix: Add reconnection attempt on transient failure; re-initialize connection
  - Test: `cargo check`; reasoned check

- [ ] **R5-DR-005** — `get_coin_id` calls CoinGecko `/search` on every non-static symbol
  - Files: `services/data-retrieval/src/sources/coingecko.rs:161-195`
  - Fix: Add in-memory cache for dynamic coin ID lookups (TTL 24h)
  - Test: `cargo check`; reasoned check

- [ ] **R5-DR-006** — No rate limiting on data-retrieval HTTP endpoints
  - Files: `services/data-retrieval/src/main.rs:104-115`
  - Fix: Add basic rate limiter middleware (e.g., tower-governor or manual bucket)
  - Test: `cargo check`; reasoned check

- [ ] **R5-DR-007** — Stock symbol lists duplicated in 3 places
  - Files: `services/data-retrieval/src/lib.rs:52-55`, `pyth.rs:294-308`
  - Fix: Single source of truth constant in `lib.rs`; reference from other modules
  - Test: `cargo check`; reasoned check

- [x] **R5-BR-006** — All intents validated against same pre-trade snapshot within tick
  - Files: `services/bot-runner/src/runner.rs:486-488`
  - Fix: (Addressed together with R5-BR-001) — committed amounts tracked per tick
  - Test: `cargo check`; reasoned check
  - **Done:** Fixed by R5-BR-001 — committed amounts tracked in tick loop.

- [ ] **R5-BR-007** — Position-size check applies to sell intents (meaningless)
  - Files: `services/bot-runner/src/runner.rs:700-714`
  - Fix: Skip position-size check for sell intents
  - Test: `cargo check`; reasoned check

- [ ] **R5-BR-008** — Paper trading applies maximum slippage (systematically pessimistic)
  - Files: `services/bot-runner/src/executor.rs:656-658`
  - Fix: Use average (half) of max slippage for paper trading
  - Test: `cargo check`; reasoned check

- [ ] **R5-BR-009** — Initial config poll has no retry
  - Files: `services/bot-runner/src/runner.rs:155-157`
  - Fix: Add retry loop (3 attempts with backoff) on initial config fetch
  - Test: `cargo check`; reasoned check

- [ ] **R5-BR-010** — Secret fields (`llm_api_key`, `telegram_bot_token`) derive `Debug`
  - Files: `services/bot-runner/src/config.rs:77-80`
  - Fix: Implement custom `Debug` that redacts secret fields
  - Test: `cargo check`; reasoned check

- [ ] **R5-BR-011** — `check_gateway_health` subprocess has no timeout
  - Files: `services/bot-runner/src/gateway.rs:366-374`
  - Fix: Add `tokio::time::timeout(Duration::from_secs(10), ...)` wrapper
  - Test: `cargo check`; reasoned check

- [ ] **R5-BR-012** — `restart_gateway`/`reload_gateway` subprocesses have no timeout
  - Files: `services/bot-runner/src/gateway.rs:284-296,316-328`
  - Fix: Add 30s timeout wrapper
  - Test: `cargo check`; reasoned check

- [ ] **R5-BR-013** — `reqwest::Client::new()` without default timeout
  - Files: `services/bot-runner/src/executor.rs:253`
  - Fix: Build client with `.timeout(Duration::from_secs(30))`
  - Test: `cargo check`; reasoned check

- [ ] **R5-INFRA-005** — `updateBotConfig` returns raw response without `mapBotConfig()`
  - Files: `packages/api-client/src/bots.ts:99-107`
  - Fix: Add `mapBotConfig(response)` call
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **R5-INFRA-006** — `fetchApi` overwrites caller's `AbortSignal`
  - Files: `packages/api-client/src/http.ts:95-105`
  - Fix: Chain caller's signal with timeout controller using `addEventListener('abort', ...)`
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **R5-INFRA-007** — Stale `types/dist/` committed (diverges from source)
  - Files: `packages/types/dist/index.d.ts`
  - Fix: Add `dist/` to `.gitignore` for types package; delete stale dist
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **R5-INFRA-008** — Deploy pulls `:latest` causing unnecessary restarts
  - Files: `.github/workflows/deploy.yml:229-244`
  - Fix: Only pull/restart services whose build job succeeded
  - Test: YAML syntax check; reasoned check

- [ ] **R5-MB-002** — API keys not cleared from Zustand store on logout
  - Files: `apps/mobile/src/screens/settings/AiProviderSettings.tsx:29,35`
  - Fix: Clear `apiKeys` state in logout handler
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **R5-MB-003** — Subscription check unawaited promise on auth
  - Files: `apps/mobile/src/screens/AuthScreen.tsx:97-127`
  - Fix: Await the subscription check; guard setState with mounted ref
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **R5-MB-004** — Drawer logout navigates without clearing tokens
  - Files: `apps/mobile/src/navigation/AppNavigator.tsx:340-344`
  - Fix: Call `clearAuth()` before navigating to auth screen
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **R5-MB-005** — `fetchCedrosPayConfig()` has no timeout
  - Files: `apps/mobile/src/config/api.ts:43-63`
  - Fix: Add `AbortController` with 10s timeout
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **R5-MB-006** — Telegram fields collected but never sent in create bot
  - Files: `apps/mobile/src/screens/CreateBotScreen.tsx:292-296`
  - Fix: Include `telegramUserId` and `telegramPairingCode` in request payload
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **R5-MB-007** — Clearing API key field sends empty string (deletes key)
  - Files: `apps/mobile/src/screens/BotSettingsScreen.tsx:380-396`
  - Fix: Validate API key field; prevent sending empty string
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **R5-MB-008** — "Forgot password?" shows dev placeholder Alert
  - Files: `apps/mobile/src/screens/AuthScreen.tsx:335-340`
  - Fix: Link to actual password reset URL via `Linking.openURL`
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **R5-MB-009** — "Manage Subscription" shows raw URL in Alert
  - Files: `apps/mobile/src/screens/BillingScreen.tsx:71-77`
  - Fix: Open URL directly via `Linking.openURL` instead of showing in Alert
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **R5-MB-010** — `key={paragraph}` causes duplicate paragraphs to be dropped
  - Files: `apps/mobile/src/screens/DocsScreen.tsx:272-274`
  - Fix: Use index-based key `key={`p-${idx}`}`
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **R5-MB-011** — Per-bot parallel API calls (2xN) on every focus
  - Files: `apps/mobile/src/screens/HomeOverviewScreen.tsx:65-86`
  - Fix: Add staleness check; skip refresh if data is < 30s old
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **R5-MB-012** — Most interactive elements lack `accessibilityLabel`
  - Files: Multiple mobile screen files
  - Fix: DEFERRED — large cross-cutting concern; best done as dedicated accessibility pass
  - Test: N/A

## Low (45)

- [ ] **R5-CP-010** — `alert_state`/`trade_failures` HashMaps grow without bound
  - Files: `services/control-plane/src/alerting.rs:118-121`
  - Fix: Add eviction when map exceeds 10K entries
  - Test: `cargo check`; reasoned check

- [ ] **R5-CP-011** — `IdempotencyKey` struct defined but never used
  - Files: `services/control-plane/src/provisioning.rs:605-625`
  - Fix: Delete dead struct
  - Test: `cargo check`

- [ ] **R5-CP-012** — `derive_default_persona` duplicated in two files
  - Files: `services/control-plane/src/handlers/settings.rs:76-83`, `bots.rs:301-307`
  - Fix: Extract to shared helper
  - Test: `cargo check`; reasoned check

- [ ] **R5-CP-013** — `WebhookNotifier` creates own `reqwest::Client`
  - Files: `services/control-plane/src/webhook.rs:34-44`
  - Fix: Accept shared `reqwest::Client` via constructor
  - Test: `cargo check`; reasoned check

- [ ] **R5-CP-014** — Subscription query on EVERY authenticated request
  - Files: `services/control-plane/src/middleware/subscription.rs:97-113`
  - Fix: Cache subscription status per user_id with 30s TTL
  - Test: `cargo check`; reasoned check

- [ ] **R5-CP-015** — `bots.rs` exceeds 500-line limit (1180 lines)
  - Files: `services/control-plane/src/handlers/bots.rs`
  - Fix: DEFERRED — large refactor, split into sub-handlers
  - Test: N/A

- [ ] **R5-CP-016** — `sync.rs` exceeds 500-line limit (727 lines)
  - Files: `services/control-plane/src/handlers/sync.rs`
  - Fix: DEFERRED — large refactor
  - Test: N/A

- [ ] **R5-CP-017** — `provisioning.rs` exceeds 500-line limit (626 lines)
  - Files: `services/control-plane/src/provisioning.rs`
  - Fix: DEFERRED — large refactor
  - Test: N/A

- [ ] **R5-CP-018** — `admin.rs` exceeds 500-line limit (532 lines)
  - Files: `services/control-plane/src/handlers/admin.rs`
  - Fix: DEFERRED — large refactor
  - Test: N/A

- [ ] **R5-CP-019** — `Decimal::from_str().unwrap()` fragile pattern
  - Files: `services/control-plane/src/algorithms/mod.rs:100-104`
  - Fix: Replace with `Decimal::from_str().unwrap_or_default()` or handle error
  - Test: `cargo check`; reasoned check

- [ ] **R5-CP-020** — Duplicate types (`Candle`, `Signal`) in algorithms + brain modules
  - Files: `services/control-plane/src/algorithms/`, `brain/`
  - Fix: DEFERRED — requires deeper module unification
  - Test: N/A

- [ ] **R5-DR-008** — `NoOpCache` struct defined but never used
  - Files: `services/data-retrieval/src/cache/mod.rs:61-68`
  - Fix: Delete dead struct
  - Test: `cargo check`

- [ ] **R5-DR-009** — Hardcoded `"usd"` check in coingecko response
  - Files: `services/data-retrieval/src/sources/coingecko.rs:216-220`
  - Fix: Use constant; add doc comment explaining CoinGecko always returns lowercase currency
  - Test: `cargo check`; reasoned check

- [ ] **R5-DR-010** — `Candle`, `OnChainMetric`, `SentimentData` types unused
  - Files: `services/data-retrieval/src/types.rs:48-133`
  - Fix: Delete unused types
  - Test: `cargo check`

- [ ] **R5-DR-011** — `(-expo) as u32` overflow on extreme Pyth exponent values
  - Files: `services/data-retrieval/src/sources/pyth.rs:160-164`
  - Fix: Clamp exponent to safe range before cast
  - Test: `cargo check`; reasoned check

- [ ] **R5-DR-012** — All WS subscriptions use `"id": 1` (can't correlate failures)
  - Files: `services/data-retrieval/src/sources/binance_ws.rs:108-112`
  - Fix: Use incremented subscription ID
  - Test: `cargo check`; reasoned check

- [ ] **R5-DR-013** — No graceful shutdown for WS/spawned tasks in data-retrieval
  - Files: `services/data-retrieval/src/main.rs`
  - Fix: Add SIGTERM handler with graceful shutdown of spawned tasks
  - Test: `cargo check`; reasoned check

- [ ] **R5-DR-014** — Unused deps: `dotenvy`, `config`, `url`, `wiremock`
  - Files: `services/data-retrieval/Cargo.toml`
  - Fix: Remove unused dependencies
  - Test: `cargo check`

- [ ] **R5-DR-015** — CORS failures silent; no warning logged
  - Files: `services/data-retrieval/src/handlers.rs`
  - Fix: Add tracing warning on CORS origin rejection
  - Test: `cargo check`; reasoned check

- [ ] **R5-BR-014** — Malformed claw-trader response silently yields `out_amount: 0`
  - Files: `services/bot-runner/src/executor.rs:418-429`
  - Fix: Return error on parse failure instead of defaulting to 0
  - Test: `cargo check`; reasoned check

- [ ] **R5-BR-015** — `intent.rs` (583 lines) mostly dead code (`IntentRegistry`)
  - Files: `services/bot-runner/src/intent.rs`
  - Fix: DEFERRED — marked as future feature in R4; keep for now
  - Test: N/A

- [ ] **R5-BR-016** — Quote cache eviction sorts 10K entries on hot path
  - Files: `services/bot-runner/src/executor.rs:90-101`
  - Fix: Use retain-based eviction with timestamp cutoff (same pattern as DR-011 R4)
  - Test: `cargo check`; reasoned check

- [ ] **R5-BR-017** — Cleanup task has no cancellation — leaked on recreate
  - Files: `services/bot-runner/src/executor.rs:134-142`
  - Fix: Store `JoinHandle`; abort previous before spawning new
  - Test: `cargo check`; reasoned check

- [ ] **R5-BR-018** — Base58 validation accepts invalid characters
  - Files: `services/bot-runner/src/amount.rs:157`
  - Fix: Use proper Base58 decode for validation
  - Test: `cargo check`; reasoned check

- [ ] **R5-BR-019** — `runner.rs` exceeds 500-line limit (1069 lines)
  - Files: `services/bot-runner/src/runner.rs`
  - Fix: DEFERRED — large refactor
  - Test: N/A

- [ ] **R5-BR-020** — `executor.rs` exceeds 500-line limit (898 lines)
  - Files: `services/bot-runner/src/executor.rs`
  - Fix: DEFERRED — large refactor
  - Test: N/A

- [ ] **R5-BR-021** — `expect()` panics if HTTP client builder fails
  - Files: `services/bot-runner/src/openclaw.rs:46`
  - Fix: Replace `expect()` with `?` error propagation
  - Test: `cargo check`; reasoned check

- [ ] **R5-BR-022** — `get_recent_prices` returns zero for all assets
  - Files: `services/bot-runner/src/runner.rs:805-832`
  - Fix: Implement actual recent price lookup from state/portfolio
  - Test: `cargo check`; reasoned check

- [ ] **R5-BR-023** — `get_recent_events` always returns empty vec
  - Files: `services/bot-runner/src/runner.rs:835-840`
  - Fix: Implement actual event history retrieval
  - Test: `cargo check`; reasoned check

- [ ] **R5-INFRA-009** — Dead file: `client.ts` (circular re-export)
  - Files: `packages/api-client/src/client.ts`
  - Fix: Delete file
  - Test: `npx tsc --noEmit`

- [ ] **R5-INFRA-010** — Dead `ApiClientConfig` interface
  - Files: `packages/api-client/src/config.ts`
  - Fix: Delete file
  - Test: `npx tsc --noEmit`

- [ ] **R5-INFRA-011** — `DATA_API_URL` uses `process.env` not `EXPO_PUBLIC_` prefix
  - Files: `packages/api-client/src/http.ts:13`
  - Fix: Add `EXPO_PUBLIC_DATA_API_URL` fallback
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **R5-INFRA-012** — `fetchApi` returns `Promise<any>` (zero type safety)
  - Files: `packages/api-client/src/http.ts:76-81`
  - Fix: DEFERRED — changing to `Promise<unknown>` requires updating all call sites (M effort)
  - Test: N/A

- [ ] **R5-INFRA-013** — PostgreSQL bound to `0.0.0.0` in docker-compose
  - Files: `docker-compose.yml:14`
  - Fix: Bind to `127.0.0.1:5432:5432`
  - Test: Reasoned check

- [ ] **R5-INFRA-014** — `lint`/`typecheck` scripts identical; do neither
  - Files: `package.json:13-14`
  - Fix: Rename to `build:packages` or add real lint tooling
  - Test: Reasoned check

- [ ] **R5-INFRA-015** — `postinstall` runs `npx patch-package` with no patches
  - Files: `package.json:15`
  - Fix: Remove postinstall script
  - Test: Reasoned check

- [ ] **R5-INFRA-016** — Stale duplicate audit report in `docs/`
  - Files: `docs/FULL_AUDIT_REPORT.md`
  - Fix: Delete stale copy
  - Test: N/A

- [ ] **R5-INFRA-017** — `docs/frontend-architecture.md` references removed APIs
  - Files: `docs/frontend-architecture.md`
  - Fix: Add deprecation notice at top of file
  - Test: N/A

- [ ] **R5-MB-013** — Data-loading duplicated between `loadData` and `useFocusEffect`
  - Files: `apps/mobile/src/screens/HomeOverviewScreen.tsx:54-118`
  - Fix: Consolidate into single loading path
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **R5-MB-014** — Chat navigate uses `as never` (likely wrong route)
  - Files: `apps/mobile/src/screens/home/BotFleetCard.tsx:125`
  - Fix: Use correct typed navigation route
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **R5-MB-015** — `setLoading(true)` on background refresh causes UI flicker
  - Files: `apps/mobile/src/hooks/useBots.ts:15-26`
  - Fix: Only set loading on initial fetch, not on refresh
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **R5-MB-016** — Terms/Privacy links show dev placeholders
  - Files: `apps/mobile/src/screens/AuthScreen.tsx:450-460`
  - Fix: Link to actual URLs or hide until available
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **R5-MB-017** — Default trading mode is `'live'` for new bots
  - Files: `apps/mobile/src/screens/CreateBotScreen.tsx:281`
  - Fix: Default to `'paper'`
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **R5-MB-018** — 3 files exceed 500-line limit
  - Files: `BotSettingsScreen.tsx` (693), `CreateBotScreen.tsx` (607), `CreateBotWizard.styles.ts` (643)
  - Fix: DEFERRED — large refactor for each
  - Test: N/A

- [ ] **R5-MB-019** — `useBotsStore`, `usePricesStore` never used
  - Files: `apps/mobile/src/store/index.ts:34-63,98-122`
  - Fix: Delete dead stores
  - Test: `npx tsc --noEmit`

---

## Deferred Items

| ID | Severity | Reason |
|----|----------|--------|
| R5-CP-015 | Low | bots.rs 1180 lines — large handler split |
| R5-CP-016 | Low | sync.rs 727 lines — large handler split |
| R5-CP-017 | Low | provisioning.rs 626 lines — large handler split |
| R5-CP-018 | Low | admin.rs 532 lines — large handler split |
| R5-CP-020 | Low | Duplicate types across modules — deep unification |
| R5-BR-015 | Low | IntentRegistry — marked future feature in R4 |
| R5-BR-019 | Low | runner.rs 1069 lines — large handler split |
| R5-BR-020 | Low | executor.rs 898 lines — large handler split |
| R5-INFRA-012 | Low | fetchApi `Promise<any>` → `Promise<unknown>` — all call sites need updating |
| R5-MB-012 | Medium | Accessibility labels — cross-cutting dedicated pass |
| R5-MB-018 | Low | 3 mobile files > 500 lines — per-file split effort |
