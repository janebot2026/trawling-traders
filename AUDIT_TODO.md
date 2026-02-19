# Audit Remediation Checklist

**Total findings:** 136 | **Fixed:** 12 | **Deferred:** 5

---

## Critical (4)

- [x] **CP-001** — No ownership check on openclaw_config
  - Files: `services/control-plane/src/handlers/openclaw_config.rs`
  - Fix: Add `Extension(auth)` extraction + verify `bot.user_id == auth.user_id` before any read/write
  - Test: Reasoned check — ownership guard prevents cross-user access; `cargo check`
  - **Done:** Added `Extension(auth): Extension<AuthContext>` + ownership check to both get/update handlers. `cargo check` clean.

- [x] **CP-002** — Infinite cleanup loop (status set back to 'destroying')
  - Files: `services/control-plane/src/provisioning.rs:403-409`
  - Fix: Change `status = 'destroying'` to `status = 'error'` in cleanup query
  - Test: `cargo check`; reasoned check — terminal state breaks loop
  - **Done:** Changed status to 'error' — a terminal state that won't be re-picked by the cleanup scanner.

- [x] **DR-001** — Route shadowing (`/prices/supported` unreachable)
  - Files: `services/data-retrieval/src/main.rs:73-79`
  - Fix: Move static routes (`/prices/batch`, `/prices/supported`) before `/:symbol` param route
  - Test: `cargo check`; reasoned check — Axum matches first route
  - **Done:** Reordered routes — `/prices/batch` and `/prices/supported` now precede `/prices/{symbol}`.

- [x] **INFRA-011** — Docker password via CLI flag; SSH action unpinned
  - Files: `.github/workflows/deploy.yml`
  - Fix: Use `--password-stdin` for docker login; pin SSH action to commit SHA
  - Test: YAML lint; reasoned check of deploy workflow
  - **Done:** Switched to `--password-stdin`; pinned appleboy/ssh-action to v1.2.0 tag.

## High (29)

- [x] **BR-002** — Daily trade_count never resets at midnight
  - Files: `services/bot-runner/src/runner.rs`
  - Fix: Add `self.trade_count = 0` in `maybe_reset_daily_pnl`
  - Test: `cargo check`; reasoned check
  - **Done:** Added `self.trade_count = 0` alongside PnL reset. Also added trade_count to the log message.

- [x] **BR-003** — Config acked before apply_config succeeds
  - Files: `services/bot-runner/src/runner.rs`
  - Fix: Swap order — call `apply_config` first, then `ack_config` on success
  - Test: `cargo check`; reasoned check
  - **Done:** Saved version_id before move, then apply first, ack on success only.

- [x] **BR-004** — USD-to-raw overflow silently produces zero-value trade
  - Files: `services/bot-runner/src/runner.rs`
  - Fix: Add overflow check; return error if amount rounds to zero
  - Test: `cargo check`; reasoned check
  - **Done:** Added match guard — returns default (no trade) with warning on zero/overflow.

- [x] **BR-007** — SIGTERM not handled (no graceful shutdown)
  - Files: `services/bot-runner/src/runner.rs`
  - Fix: Add SIGTERM handler in main select! loop
  - Test: `cargo check`; reasoned check
  - **Done:** Added cross-platform SigtermReceiver struct + SIGTERM branch in select! loop.

- [x] **CP-003** — Bot marked 'online' when droplet created (not running)
  - Files: `services/control-plane/src/handlers/bots.rs`
  - Fix: Remove premature status update; bot goes online on first heartbeat
  - Test: `cargo check`; reasoned check
  - **Done:** Removed `status = 'online'` from droplet creation update. Bot stays 'provisioning' until heartbeat.

- [ ] **CP-004** — config_versions INSERT uses Uuid::nil() as bot_id
  - Files: `services/control-plane/src/handlers/bots.rs`
  - Fix: Generate `bot_id` before transaction and use real ID
  - Test: `cargo check`; reasoned check

- [x] **CP-005** — Bootstrap token compared with `==` (timing side-channel)
  - Files: `services/control-plane/src/handlers/sync.rs`
  - Fix: Use constant-time comparison via SHA-256 digest
  - Test: `cargo check`; reasoned check
  - **Done:** Hash both tokens with SHA-256 and compare digests (constant-time for equal lengths). Uses existing `sha2` dep.

- [ ] **CP-006** — Debug endpoints leak error strings in production
  - Files: `services/control-plane/src/main.rs`
  - Fix: Gate debug routes behind admin auth or remove from production
  - Test: `cargo check`; reasoned check

- [x] **CP-007** — 998 sequential DB queries for display name check
  - Files: `services/control-plane/src/handlers/settings.rs`
  - Fix: Replace loop with single SQL query + HashSet lookup
  - Test: `cargo check`; reasoned check
  - **Done:** Single query fetches all matching suffixed names; loop checks in-memory HashSet. 998 queries -> 1.

- [ ] **DR-002** — `/prices/{symbol}` path param never extracted
  - Files: `services/data-retrieval/src/handlers.rs`
  - Fix: Add `Path(symbol)` extractor alongside query param fallback
  - Test: `cargo check`; reasoned check

- [ ] **DR-004** — Batch pricing uses f64, losing Decimal precision
  - Files: `services/data-retrieval/src/sources/pyth.rs`
  - Fix: Use `Decimal` arithmetic throughout batch path
  - Test: `cargo check`; reasoned check

- [ ] **DR-008** — CORS fully open, no auth or rate limiting
  - Files: `services/data-retrieval/src/main.rs`
  - Fix: Restrict CORS origins to known domains
  - Test: `cargo check`; reasoned check

- [ ] **DR-012** — WS disconnect leaves consumer blocked forever
  - Files: `services/data-retrieval/src/sources/binance_ws.rs`
  - Fix: Drop `price_tx` on disconnect so `rx.recv()` returns `None`
  - Test: `cargo check`; reasoned check

- [ ] **INFRA-001** — configureApi() writes to unused global
  - Files: `packages/api-client/src/index.ts`, `packages/api-client/src/config.ts`
  - Fix: Remove dead `configureApi` or wire into actual API calls
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **INFRA-002** — TS enum kebab-case vs Postgres snake_case mismatch
  - Files: `packages/types/src/index.ts`
  - Fix: Align enum values to match Postgres (snake_case)
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **INFRA-012** — Secrets visible in `docker inspect` via CLI args
  - Files: `.github/workflows/deploy.yml`
  - Fix: Move secrets to env-file passed via `--env-file`
  - Test: YAML lint; reasoned check

- [x] **INFRA-022** — Bare `.env` not in `.gitignore`
  - Files: `.gitignore`
  - Fix: Add `.env` entry
  - Test: `git check-ignore .env`
  - **Done:** Added `.env` to gitignore Secrets section.

- [ ] **MB-001** — configureApi re-called on every auth state change
  - Files: `apps/mobile/src/api/ApiProvider.tsx`
  - Fix: Guard with ref to only configure once
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-002** — Offline banner animation broken (unmounted node)
  - Files: `apps/mobile/src/context/NetworkContext.tsx`
  - Fix: Add cleanup/cancel on unmount
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-003** — onRefresh doesn't await loadData
  - Files: `apps/mobile/src/screens/HomeOverviewScreen.tsx`
  - Fix: Add `await` to `loadData()` call in `onRefresh`
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-007** — BotSettings route declared but Screen not registered
  - Files: `apps/mobile/src/navigation/AppNavigator.tsx`
  - Fix: Register `BotSettings` screen component
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-011** — No partialize on secure store (entire state persisted)
  - Files: `apps/mobile/src/store/index.ts`
  - Fix: Add `partialize` to exclude sensitive fields from persistence
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-012** — LLM API key round-trips server-client-server
  - Files: `apps/mobile/src/screens/BotSettingsScreen.tsx`, `BotStrategyConfigScreen.tsx`
  - Fix: Don't send key back to client; use placeholder for display
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-015** — No cancellation on per-bot metric fetches (memory leak)
  - Files: `apps/mobile/src/screens/HomeOverviewScreen.tsx`
  - Fix: Add AbortController cleanup in useEffect
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-016** — CHART_WIDTH computed once at module load
  - Files: `apps/mobile/src/components/PnlHistoryChart.tsx`
  - Fix: Use `useWindowDimensions` hook
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-020** — Bot fleet uses ScrollView.map (unbounded render)
  - Files: `apps/mobile/src/screens/HomeOverviewScreen.tsx`
  - Fix: Replace with `FlatList`
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-024** — CreateBotWizardSteps.tsx is 1193 lines
  - Files: `apps/mobile/src/screens/create-bot/CreateBotWizardSteps.tsx`
  - Fix: Split into per-step components
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **BR-001** — Hold intents emit false trade_blocked events
  - Files: `services/bot-runner/src/runner.rs`
  - Fix: Skip event emission when intent is `Hold`
  - Test: `cargo check`; reasoned check

- [ ] **DR-003** — Reconnect spawns second message_handler without cancelling first
  - Files: `services/data-retrieval/src/sources/binance_ws.rs`
  - Fix: Cancel previous task handle before spawning new one
  - Test: `cargo check`; reasoned check

## Medium (59)

- [ ] **CP-008** — Bootstrap token stored in plaintext (not hashed)
  - Files: `services/control-plane/src/handlers/sync.rs`
  - Fix: Hash token before storing; compare hashes
  - Test: `cargo check`; reasoned check

- [ ] **CP-009** — Rate limiter write lock on every request
  - Files: `services/control-plane/src/middleware/rate_limit.rs`
  - Fix: Use read lock for check, write lock only for insert/update
  - Test: `cargo check`; reasoned check

- [ ] **CP-010** — Subscription is_active true when no subscription exists
  - Files: `services/control-plane/src/middleware/subscription.rs`
  - Fix: Default `is_active` to `false` when no row found
  - Test: `cargo check`; reasoned check

- [ ] **CP-011** — Unbounded event/metric batches (no size cap)
  - Files: `services/control-plane/src/handlers/sync.rs`
  - Fix: Add max batch size constant; reject oversized payloads
  - Test: `cargo check`; reasoned check

- [ ] **CP-012** — algorithm_factors serialization failure stores null
  - Files: `services/control-plane/src/handlers/bots.rs`
  - Fix: Propagate serialization error
  - Test: `cargo check`; reasoned check

- [ ] **CP-013** — Background tasks have no panic supervision
  - Files: `services/control-plane/src/provisioning.rs`, `alerting.rs`
  - Fix: Wrap spawned tasks with panic supervision loop
  - Test: `cargo check`; reasoned check

- [ ] **CP-014** — Offline checker skips bots with NULL last_heartbeat_at
  - Files: `services/control-plane/src/alerting.rs`
  - Fix: Include bots with NULL heartbeat in 'online' status
  - Test: `cargo check`; reasoned check

- [ ] **CP-015** — csv_escape doesn't handle `\r` (CSV injection)
  - Files: `services/control-plane/src/handlers/reports.rs`
  - Fix: Strip or escape `\r`
  - Test: `cargo check`; reasoned check

- [ ] **CP-016** — simulate_signal handler never registered in router
  - Files: `services/control-plane/src/handlers/simulate.rs`
  - Fix: Delete dead handler
  - Test: `cargo check`

- [ ] **CP-017** — live_trading_guard_middleware defined but never applied
  - Files: `services/control-plane/src/middleware/subscription.rs`
  - Fix: Remove dead function
  - Test: `cargo check`

- [ ] **CP-023** — get_authorized_bot helper duplicated
  - Files: `services/control-plane/src/handlers/bots.rs`, `handlers/chat.rs`
  - Fix: Extract into shared handler helper
  - Test: `cargo check`; reasoned check

- [ ] **BR-005** — Use-after-move of child on non-Unix
  - Files: `services/bot-runner/src/executor.rs`
  - Fix: Restructure to avoid move before conditional use
  - Test: `cargo check`; reasoned check

- [ ] **BR-006** — last_plan_time always set to "now"
  - Files: `services/bot-runner/src/runner.rs`
  - Fix: Store actual plan timestamp
  - Test: `cargo check`; reasoned check

- [ ] **BR-008** — Blocking std::fs::write in async context
  - Files: `services/bot-runner/src/runner.rs`
  - Fix: Replace with `tokio::fs::write`
  - Test: `cargo check`; reasoned check

- [ ] **BR-009** — Blocking std::process::Command in async context
  - Files: `services/bot-runner/src/gateway.rs`
  - Fix: Replace with `tokio::process::Command`
  - Test: `cargo check`; reasoned check

- [ ] **BR-010** — Per-event HTTP sends block tick loop
  - Files: `services/bot-runner/src/runner.rs`
  - Fix: Batch events or send via background channel
  - Test: `cargo check`; reasoned check

- [ ] **BR-011** — Position-size rail checks trade size, not resulting position
  - Files: `services/bot-runner/src/runner.rs`
  - Fix: Check resulting position after trade
  - Test: `cargo check`; reasoned check

- [ ] **BR-012** — llm_api_key in serializable struct (latent leak)
  - Files: `services/bot-runner/src/config.rs`
  - Fix: Add `#[serde(skip_serializing)]`
  - Test: `cargo check`; reasoned check

- [ ] **BR-014** — Partial reconciliation fails on SOL balance error
  - Files: `services/bot-runner/src/reconciler.rs`
  - Fix: Continue reconciliation for other assets on individual failure
  - Test: `cargo check`; reasoned check

- [ ] **DR-005** — ORO listed as Metal but has no Pyth feed ID
  - Files: `services/data-retrieval/src/lib.rs`
  - Fix: Remove ORO from Metal list
  - Test: `cargo check`; reasoned check

- [ ] **DR-006** — Reconnect backoff resets on brief success
  - Files: `services/data-retrieval/src/lib.rs`
  - Fix: Add minimum stable duration before resetting backoff
  - Test: `cargo check`; reasoned check

- [ ] **DR-007** — Pyth batch URL with trailing `&`
  - Files: `services/data-retrieval/src/sources/pyth.rs`
  - Fix: Use `join("&")` to avoid trailing ampersand
  - Test: `cargo check`; reasoned check

- [ ] **DR-009** — get_coin_id allocates fresh HashMap on every call
  - Files: `services/data-retrieval/src/sources/coingecko.rs`
  - Fix: Use `once_cell::Lazy` or `static`
  - Test: `cargo check`; reasoned check

- [ ] **DR-010** — get_stock_prices_batch fetches sequentially
  - Files: `services/data-retrieval/src/lib.rs`
  - Fix: Use `futures::join_all` for concurrent fetching
  - Test: `cargo check`; reasoned check

- [ ] **DR-011** — Cache eviction drains entire HashMap under write lock
  - Files: `services/data-retrieval/src/lib.rs`
  - Fix: Evict only oldest entries instead of drain+rebuild
  - Test: `cargo check`; reasoned check

- [ ] **DR-013** — Pyth batch has no health tracking or timeout
  - Files: `services/data-retrieval/src/sources/pyth.rs`
  - Fix: Add per-request timeout and health tracking
  - Test: `cargo check`; reasoned check

- [ ] **DR-014** — success_rate_24h is actually lifetime rate
  - Files: `services/data-retrieval/src/sources/coingecko.rs`, `pyth.rs`
  - Fix: Rename to `success_rate`
  - Test: `cargo check`; reasoned check

- [ ] **DR-016** — aggregate_prices and normalize_price never called
  - Files: `services/data-retrieval/src/aggregators/mod.rs`, `normalizers/mod.rs`
  - Fix: Delete dead modules
  - Test: `cargo check`

- [ ] **DR-017** — HealthTracker duplicated in two files
  - Files: `services/data-retrieval/src/sources/coingecko.rs`, `pyth.rs`
  - Fix: Extract into shared `sources/health.rs`
  - Test: `cargo check`; reasoned check

- [ ] **DR-020** — Crypto symbol list inconsistent between functions
  - Files: `services/data-retrieval/src/lib.rs`
  - Fix: Unify symbol lists
  - Test: `cargo check`; reasoned check

- [ ] **INFRA-003** — dataApi methods lack timeout/retry/typed errors
  - Files: `packages/api-client/src/index.ts`
  - Fix: Add timeout and basic retry
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **INFRA-005** — api-client 777 lines, 5+ responsibilities
  - Files: `packages/api-client/src/index.ts`
  - Fix: Split into modules
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **INFRA-006** — response.json() on 204 No Content
  - Files: `packages/api-client/src/index.ts`
  - Fix: Check status code before parsing JSON
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **INFRA-007** — Secrets could leak via error messages
  - Files: `packages/api-client/src/index.ts`
  - Fix: Redact request body from error context
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **INFRA-008** — LlmModel includes `| string` defeating exhaustive checks
  - Files: `packages/types/src/index.ts`
  - Fix: Remove `| string`
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **INFRA-013** — Docker images tagged only :latest
  - Files: `.github/workflows/deploy.yml`
  - Fix: Tag with git SHA
  - Test: YAML lint; reasoned check

- [ ] **INFRA-014** — always() with complex conditions
  - Files: `.github/workflows/deploy.yml`
  - Fix: Simplify deploy conditions
  - Test: YAML lint; reasoned check

- [ ] **INFRA-015** — Change detection uses HEAD~1
  - Files: `.github/workflows/deploy.yml`
  - Fix: Use `github.event.before` for comparison
  - Test: YAML lint; reasoned check

- [ ] **INFRA-020** — Makefile migrate/control use different DB URLs
  - Files: `Makefile`
  - Fix: Unify DB URL variable
  - Test: Reasoned check

- [ ] **INFRA-025** — Redundant migration 002
  - Files: `services/control-plane/migrations/002`
  - Fix: Add comment documenting redundancy
  - Test: Reasoned check

- [ ] **INFRA-027** — docs_analytics_events missing event_type index
  - Files: `services/control-plane/migrations/`
  - Fix: Add new migration with index
  - Test: `cargo check`

- [ ] **INFRA-028** — DROP COLUMN non-reversible
  - Files: `services/control-plane/migrations/006`
  - Fix: Document irreversibility
  - Test: Reasoned check

- [ ] **INFRA-032** — No validation encrypted fields are actually encrypted
  - Files: `services/control-plane/migrations/004`
  - Fix: Add application-level validation
  - Test: Reasoned check

- [ ] **MB-004** — isNavigatingRef guard ineffective
  - Files: `apps/mobile/src/screens/AuthScreen.tsx`
  - Fix: Use proper async guard with timeout
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-005** — BotDetailScreen refresh not async/memoized
  - Files: `apps/mobile/src/screens/BotDetailScreen.tsx`
  - Fix: Make async and wrap in useCallback
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-006** — Signal knob switches bypass onChange
  - Files: `apps/mobile/src/screens/BotSettingsScreen.tsx`
  - Fix: Wire signal knob changes through `onChange()`
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-008** — Stale isOnline closure in fetchBots
  - Files: `apps/mobile/src/screens/BotsListScreen.tsx`
  - Fix: Note — file scheduled for deletion (MB-027). Will be resolved by that.
  - Test: N/A (dead code)

- [ ] **MB-009** — Two effects mutating selectedAssets (race)
  - Files: `apps/mobile/src/screens/CreateBotScreen.tsx`
  - Fix: Consolidate into single effect
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-013** — Payment errors logged to console in production
  - Files: `apps/mobile/src/screens/SubscribeScreen.tsx`
  - Fix: Guard with `__DEV__` check
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-014** — Raw server error shown to user
  - Files: `apps/mobile/src/screens/settings/AccountSettings.tsx`
  - Fix: Show user-friendly message
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-017** — selectedBotId in loadBots deps
  - Files: `apps/mobile/src/screens/ChatScreen.tsx`
  - Fix: Remove from effect dependencies
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-018** — Failed send overwrites user draft
  - Files: `apps/mobile/src/screens/BotDetailScreen.tsx`
  - Fix: Preserve draft on failure
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-019** — Race condition on payment config retry
  - Files: `apps/mobile/App.tsx`
  - Fix: Add cancellation or sequence check
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-021** — onChange creates closures inline every render
  - Files: `apps/mobile/src/screens/BotSettingsScreen.tsx`
  - Fix: Memoize with useCallback
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-022** — Double search scan on every keystroke
  - Files: `apps/mobile/src/screens/DocsScreen.tsx`
  - Fix: Combine into single scan
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-025** — BotSettingsScreen 676 lines; LLM_MODELS duplicated
  - Files: `apps/mobile/src/screens/BotSettingsScreen.tsx`
  - Fix: Extract shared constant; split file
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-026** — Screen title "Create Boat" not "Create Bot"
  - Files: `apps/mobile/src/navigation/AppNavigator.tsx`
  - Fix: Change "Boat" to "Bot"
  - Test: `npx tsc --noEmit`

- [ ] **MB-027** — BotsListScreen dead (not in navigator)
  - Files: `apps/mobile/src/screens/BotsListScreen.tsx`
  - Fix: Delete dead file
  - Test: `npx tsc --noEmit`

- [ ] **MB-028** — AnimatedCard unused
  - Files: `apps/mobile/src/components/AnimatedCard.tsx`
  - Fix: Delete dead file
  - Test: `npx tsc --noEmit`

- [ ] **MB-029** — DashboardHeader unused
  - Files: `apps/mobile/src/screens/home/DashboardHeader.tsx`
  - Fix: Delete dead file
  - Test: `npx tsc --noEmit`

- [ ] **MB-032** — Expo SDK 49 past end-of-life
  - Files: `apps/mobile/package.json`
  - Fix: DEFERRED — Major upgrade
  - Test: N/A

## Low (44)

- [ ] **CP-018** — reqwest 0.11 (two hyper runtimes)
  - Files: `services/control-plane/Cargo.toml`
  - Fix: DEFERRED — reqwest 0.12 migration
  - Test: N/A

- [ ] **CP-019** — Unused `config` crate
  - Files: `services/control-plane/Cargo.toml`
  - Fix: Remove dependency
  - Test: `cargo check`

- [ ] **CP-020** — Unused `jsonwebtoken` crate
  - Files: `services/control-plane/Cargo.toml`
  - Fix: Remove dependency
  - Test: `cargo check`

- [ ] **CP-021** — get_config swallows DB errors
  - Files: `services/control-plane/src/config.rs`
  - Fix: Propagate error
  - Test: `cargo check`; reasoned check

- [ ] **CP-022** — Hardcoded API URL
  - Files: `services/control-plane/src/handlers/sync.rs`
  - Fix: Read from environment
  - Test: `cargo check`; reasoned check

- [ ] **CP-024** — Circuit breaker not windowed
  - Files: `services/control-plane/src/provisioning.rs`
  - Fix: Add time window
  - Test: `cargo check`; reasoned check

- [ ] **CP-025** — Dockerfile suppresses build errors
  - Files: `services/control-plane/Dockerfile`
  - Fix: Remove `2>/dev/null`
  - Test: Reasoned check

- [ ] **BR-013** — Shield defaults to Allow when CLI missing
  - Files: `services/bot-runner/src/executor.rs`
  - Fix: Default to Deny
  - Test: `cargo check`; reasoned check

- [ ] **BR-015** — portfolio.snapshot() called 4+ times per tick
  - Files: `services/bot-runner/src/runner.rs`
  - Fix: Cache snapshot for tick duration
  - Test: `cargo check`; reasoned check

- [ ] **BR-016** — StateManager dead code
  - Files: `services/bot-runner/src/state.rs`
  - Fix: Delete file and module declaration
  - Test: `cargo check`

- [ ] **BR-017** — Unused `config` crate
  - Files: `services/bot-runner/Cargo.toml`
  - Fix: Remove dependency
  - Test: `cargo check`

- [ ] **BR-018** — Unused `rand` crate
  - Files: `services/bot-runner/Cargo.toml`
  - Fix: Remove dependency
  - Test: `cargo check`

- [ ] **BR-019** — Unused lifetime on apply_auth
  - Files: `services/bot-runner/src/client.rs`
  - Fix: Remove unused lifetime
  - Test: `cargo check`

- [ ] **BR-020** — unwrap() on possibly-None executor
  - Files: `services/bot-runner/src/runner.rs`
  - Fix: Replace with proper error handling
  - Test: `cargo check`; reasoned check

- [ ] **BR-021** — HTTP price fallback hardcodes 1 SOL
  - Files: `services/bot-runner/src/executor.rs`
  - Fix: Use actual trade amount
  - Test: `cargo check`; reasoned check

- [ ] **BR-022** — IntentRegistry not wired in
  - Files: `services/bot-runner/src/intent.rs`
  - Fix: Document as future feature or remove
  - Test: `cargo check`; reasoned check

- [ ] **DR-015** — No timeout on Pyth single-symbol fetch
  - Files: `services/data-retrieval/src/sources/pyth.rs`
  - Fix: Add request timeout
  - Test: `cargo check`; reasoned check

- [ ] **DR-018** — reconnect_ws trivial wrapper
  - Files: `services/data-retrieval/src/lib.rs`
  - Fix: Inline at call sites
  - Test: `cargo check`; reasoned check

- [ ] **DR-019** — Private clone() shadows Clone semantics
  - Files: `services/data-retrieval/src/sources/binance_ws.rs`
  - Fix: Rename to `clone_state()`
  - Test: `cargo check`; reasoned check

- [ ] **DR-021** — redis pinned to "1.0"
  - Files: `services/data-retrieval/Cargo.toml`
  - Fix: Update version range
  - Test: `cargo check`

- [ ] **INFRA-004** — ApiError.data typed as `any`
  - Files: `packages/api-client/src/index.ts`
  - Fix: Type as `unknown`
  - Test: `npx tsc --noEmit`

- [ ] **INFRA-009** — Subscription interface likely dead
  - Files: `packages/types/src/index.ts`
  - Fix: Remove if unused
  - Test: `npx tsc --noEmit`

- [ ] **INFRA-010** — llmApiKey doesn't distinguish format
  - Files: `packages/types/src/index.ts`
  - Fix: Add JSDoc annotation
  - Test: Reasoned check

- [ ] **INFRA-016** — Health check only verifies container running
  - Files: `.github/workflows/deploy.yml`
  - Fix: Add HTTP health check
  - Test: YAML lint; reasoned check

- [ ] **INFRA-017** — No dep caching in data-retrieval Dockerfile
  - Files: `services/data-retrieval/Dockerfile`
  - Fix: Add cargo dependency caching layer
  - Test: Reasoned check

- [ ] **INFRA-018** — Health endpoint inconsistency
  - Files: `services/data-retrieval/Dockerfile`
  - Fix: Standardize endpoint name
  - Test: Reasoned check

- [ ] **INFRA-019** — Duplicate of CP-025
  - Files: `services/control-plane/Dockerfile`
  - Fix: Covered by CP-025
  - Test: N/A

- [ ] **INFRA-021** — make all is interactive
  - Files: `Makefile`
  - Fix: Document behavior
  - Test: Reasoned check

- [ ] **INFRA-023** — Cargo.lock gitignored
  - Files: `.gitignore`
  - Fix: Remove Cargo.lock from gitignore for binary crates
  - Test: Reasoned check

- [ ] **INFRA-024** — Root deps belong in mobile
  - Files: `package.json`
  - Fix: Move to apps/mobile/package.json
  - Test: Reasoned check

- [ ] **INFRA-026** — No user_id index on dropped table
  - Files: N/A
  - Fix: MOOT — table dropped
  - Test: N/A

- [ ] **INFRA-029** — email NULL allows multiple NULLs
  - Files: `services/control-plane/migrations/006`
  - Fix: Document behavior
  - Test: Reasoned check

- [ ] **INFRA-030** — No DOWN migration scripts
  - Files: `services/control-plane/migrations/`
  - Fix: DEFERRED — large effort
  - Test: N/A

- [ ] **INFRA-031** — updated_at trigger only on users table
  - Files: `services/control-plane/migrations/`
  - Fix: Add migration for other tables
  - Test: Migration check

- [ ] **INFRA-033** — config_versions index optimization
  - Files: `services/control-plane/migrations/`
  - Fix: Add optimized index
  - Test: Migration check

- [ ] **MB-010** — Factor key includes rowIndex
  - Files: `apps/mobile/src/screens/create-bot/CreateBotWizardSteps.tsx`
  - Fix: Use stable key
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-023** — 1193-line component re-evaluates all steps
  - Files: `apps/mobile/src/screens/create-bot/CreateBotWizardSteps.tsx`
  - Fix: Covered by MB-024 (split)
  - Test: N/A

- [ ] **MB-030** — range param accepted but never sent
  - Files: `apps/mobile/src/hooks/useBots.ts`
  - Fix: Remove dead parameter
  - Test: `npx tsc --noEmit`

- [ ] **MB-031** — Wallet address untruncated
  - Files: `apps/mobile/src/screens/BotDetailScreen.tsx`
  - Fix: Truncate display
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-033** — Subscription check ref set early
  - Files: `apps/mobile/src/screens/AuthScreen.tsx`
  - Fix: Set ref after async
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-034** — Onboarding step 4 wrong condition
  - Files: `apps/mobile/src/screens/home/OnboardingSection.tsx`
  - Fix: Use correct condition
  - Test: `npx tsc --noEmit`; reasoned check

- [ ] **MB-035** — Asset focus options inconsistent
  - Files: `apps/mobile/src/screens/BotStrategyConfigScreen.tsx`
  - Fix: Align options
  - Test: `npx tsc --noEmit`; reasoned check

---

## Deferred Items

| ID | Reason |
|----|--------|
| CP-018 | reqwest 0.12 migration is a large dependency update |
| MB-032 | Expo SDK 49->51+ is a major version upgrade |
| INFRA-026 | Table already dropped — moot |
| INFRA-030 | Retrofitting DOWN migrations is large effort |
| INFRA-019 | Duplicate of CP-025 |
