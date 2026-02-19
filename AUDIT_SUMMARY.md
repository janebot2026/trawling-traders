# Audit Remediation Summary

**Date:** 2026-02-19
**Rounds completed:** 4

---

## Overall Status

| Round | Findings | Fixed | Deferred | Source |
|-------|----------|-------|----------|--------|
| Round 1 | 12 | 12 | 0 | Targeted review |
| Round 2 | 18 | 18 | 0 | Targeted review |
| Round 3 | 22 | 22 | 0 | Targeted review (1 false positive, 1 acknowledged) |
| Round 4 | 136 | 134 | 2 | Full codebase audit (`FULL_AUDIT_REPORT.md`) |
| **Total** | **188** | **186** | **2** | |

---

## Round 4: Full Codebase Audit (136 findings)

### By Severity

| Severity | Total | Fixed | Deferred |
|----------|-------|-------|----------|
| Critical | 4     | 4     | 0        |
| High     | 29    | 29    | 0        |
| Medium   | 59    | 58    | 1        |
| Low      | 44    | 43    | 1        |
| **Total**| **136**| **134**| **2**   |

### By Service Area

| Area             | Total | Fixed | Deferred |
|------------------|-------|-------|----------|
| control-plane    | 25    | 24    | 1 (CP-018) |
| bot-runner       | 22    | 22    | 0        |
| data-retrieval   | 21    | 21    | 0        |
| mobile           | 35    | 34    | 1 (MB-032) |
| infrastructure   | 33    | 33    | 0        |

---

## Round 4 — Key Improvements

### Security (15 fixes)

| ID | Fix |
|----|-----|
| CP-001 | Ownership check on openclaw_config handlers — prevents cross-user access (Critical) |
| CP-005 | Constant-time bootstrap token comparison via SHA-256 digest |
| CP-008 | Bootstrap tokens hashed before storage; DB never holds plaintext |
| CP-015 | CSV injection mitigation — strip `\r` in csv_escape |
| CP-006 | Debug endpoints redact internal error strings |
| BR-012 | `#[serde(skip_serializing)]` on llm_api_key field |
| DR-008 | CORS restricted to known origins (configurable via `CORS_ALLOWED_ORIGINS`) |
| MB-011 | `apiKeys` excluded from zustand persisted store |
| MB-012 | LLM API key masked on display; only sent on explicit edit |
| MB-013 | `console.error` guarded behind `__DEV__` for payment errors |
| MB-014 | Raw server errors replaced with user-friendly messages |
| INFRA-007 | Request body redacted from API client error messages |
| INFRA-011 | Docker login via `--password-stdin`; SSH action pinned to SHA (Critical) |
| INFRA-012 | Secrets via `--env-file` (not CLI args); file deleted after start |
| INFRA-022 | `.env` added to `.gitignore` |

### Bug Fixes (31 fixes)

| ID | Fix |
|----|-----|
| CP-002 | Infinite cleanup loop broken — terminal `error` status (Critical) |
| CP-003 | Bot stays `provisioning` until first heartbeat (not prematurely `online`) |
| CP-004 | Real `bot_id` used in `config_versions` INSERT (was `Uuid::nil()`) |
| CP-010 | `is_active` defaults to `false` when no subscription row exists |
| CP-014 | Offline checker fires alert for bots with NULL `last_heartbeat_at` |
| BR-001 | Hold intents skip execution/event emission — no false `trade_blocked` |
| BR-002 | Daily `trade_count` resets alongside PnL at midnight |
| BR-003 | Config acknowledged only after successful `apply_config` |
| BR-004 | Zero-value trades from USD-to-raw overflow caught and blocked |
| BR-020 | `unwrap()` on possibly-None executor replaced with match guard |
| DR-001 | Route shadowing fixed — `/prices/supported` now reachable (Critical) |
| DR-002 | Path param extracted for `/prices/{symbol}` route |
| DR-004 | Batch pricing uses `Decimal` arithmetic (not lossy f64) |
| DR-005 | ORO removed from metals list (no Pyth feed ID) |
| DR-007 | Trailing `&` removed from Pyth batch URL |
| DR-012 | WS disconnect unblocks consumer immediately (sender set to None) |
| MB-003 | `onRefresh` awaits `loadData` |
| MB-004 | Navigation guard ref reset deferred 500ms to let navigation commit |
| MB-007 | `BotSettings` screen registered in navigator |
| MB-009 | Redundant `useEffect` racing with wizard's own effect removed |
| MB-017 | `selectedBotId` removed from `loadBots` deps (stale closure) |
| MB-019 | Sequence counter prevents stale payment config retry overwrites |
| MB-026 | "Create Boat" typo corrected to "Create Bot" |
| MB-033 | `subscriptionCheckedRef` set inside `.then()` (not eagerly) |
| MB-034 | Onboarding step 4 uses `hasFundedBot` instead of `hasBots` |
| MB-035 | `finance_2` added to `ASSETS` constant |
| INFRA-001 | Dead `configureApi` removed (wrote to unused global) |
| INFRA-002 | Enum values aligned: kebab-case changed to snake_case matching Postgres |
| INFRA-006 | 204 No Content no longer parsed as JSON |
| INFRA-014 | `github.event.before` used for change detection (not `HEAD~1`) |
| INFRA-015 | `always()` conditions documented |

### Reliability (16 fixes)

| ID | Fix |
|----|-----|
| CP-011 | Event/metric batch sizes capped at 500 |
| CP-013 | Background tasks wrapped in `catch_unwind` with 5s restart backoff |
| CP-024 | Circuit breaker uses time-windowed failure counting |
| BR-005 | Separate `cfg(unix)`/`cfg(not(unix))` kill paths (no use-after-move) |
| BR-006 | `last_plan_time` stores actual plan timestamp |
| BR-007 | SIGTERM handled for graceful shutdown |
| BR-008 | `std::fs::write` replaced with `tokio::fs::write` (non-blocking) |
| BR-009 | `std::process::Command` replaced with `tokio::process::Command` |
| BR-011 | Position-size rail checks `existing_exposure + trade_amount` |
| BR-013 | Shield defaults to Deny when CLI binary missing |
| BR-014 | SOL balance error logged and skipped; other assets continue |
| DR-003 | Reconnect aborts previous message handler before spawning new |
| DR-006 | 30s stable connection required before backoff reset |
| DR-013 | Pyth batch gets 10s timeout and HealthTracker recording |
| DR-015 | Explicit 10s timeout on Pyth single-symbol fetch |
| INFRA-016 | HTTP health check added to deploy workflow |

### Performance (11 fixes)

| ID | Fix |
|----|-----|
| CP-007 | Display name uniqueness: 998 queries replaced with 1 query + HashSet |
| CP-009 | Rate limiter: read-lock fast path (write-lock only on mutation) |
| BR-010 | Per-tick events collected into Vec, flushed in single HTTP call |
| BR-015 | `portfolio.snapshot()` cached once per tick (was 4+ calls) |
| DR-009 | `get_coin_id` HashMap replaced with `LazyLock` static |
| DR-010 | `get_stock_prices_batch` fetches concurrently via `join_all` |
| DR-011 | Cache eviction uses retain-based cutoff (not drain+rebuild) |
| MB-016 | Chart width uses `useWindowDimensions` (responsive to rotation) |
| MB-020 | Bot fleet: `FlatList` replaces unbounded `ScrollView.map` |
| MB-021 | `onChange` factory memoized with `useCallback` |
| MB-022 | Two `useMemo` merged into single scan pass |

### Code Quality (61 fixes)

**Dead code removal (12 items):**
CP-016 (simulate.rs), CP-017 (live_trading_guard), CP-019 (config crate), CP-020 (jsonwebtoken crate), BR-016 (state.rs), BR-017 (config crate), BR-018 (rand crate), BR-019 (unused lifetime), DR-016 (aggregators + normalizers), MB-027 (BotsListScreen — 551 LOC), MB-028 (AnimatedCard), MB-029 (DashboardHeader)

**File splits (4 items):**
- MB-024: `CreateBotWizardSteps.tsx` 1,193 lines split into 10 per-step components (215 lines orchestrator)
- MB-025: Shared `LLM_MODELS` constant extracted to `config/llmModels.ts`
- INFRA-003/005: `api-client/index.ts` 777 lines split into 7 modules (all under 210 lines)

**Shared extraction:** CP-023 (get_authorized_bot helper), DR-017 (HealthTracker module), DR-020 (CRYPTO_SYMBOLS constant)

**Other:** Error propagation (CP-012, CP-021), env-based config (CP-022), Dockerfile fixes (CP-025, INFRA-017), naming (DR-014, DR-019), type tightening (INFRA-004, INFRA-008, INFRA-010), Docker image tagging (INFRA-013), build reproducibility (INFRA-023), migration documentation (INFRA-025, INFRA-028, INFRA-029, INFRA-032), new indexes (INFRA-027, INFRA-033), triggers (INFRA-031), stable keys (MB-010), dead params (MB-030), UI polish (MB-031, MB-002)

---

## Cumulative Improvements (Rounds 1-4)

### Security
1. Bot-facing endpoints protected by auth middleware (R1)
2. Debug routes gated behind explicit env var (R1)
3. Encryption failures return 500, not silent empty strings (R2)
4. Production rejects missing encryption key — fail-closed (R2)
5. LLM API keys in encrypted SecureStore, not AsyncStorage (R2)
6. Free-tier users blocked from live trading mode (R2)
7. Ownership check on openclaw_config (R4)
8. Constant-time + hashed bootstrap token comparison (R4)
9. CORS restricted to known origins (R4)
10. Docker secrets via env-file, not CLI args (R4)
11. API key never round-tripped to client (R4)

### Financial Correctness
1. PnL tracking accumulates from confirmed sells with UTC midnight reset (R3)
2. Daily trade count resets at midnight alongside PnL (R4)
3. Config version increment is atomic — no race condition (R3)
4. Zero-value trades from overflow blocked (R4)
5. Position-size checks include existing exposure (R4)
6. Pyth pricing uses Decimal arithmetic throughout (R3, R4)

### Performance
1. Bot name uniqueness: 1 query, was 998 (R3, R4)
2. Heartbeat metrics + event ingest: batch INSERT (R3)
3. Report generation: 1 joined query, was N+1 (R1)
4. API key auth: 1 JOIN query, was 3 round-trips (R2)
5. Rate limiter: read-lock fast path (R4)
6. Stock prices: concurrent fetch via join_all (R4)
7. Cache eviction: retain-based, not drain+rebuild (R4)
8. Event flush: single HTTP call per tick (R4)

### Reliability
1. Panic supervision on all background tasks (R2, R4)
2. SIGTERM handling for graceful shutdown (R4)
3. WebSocket backpressure and disconnect handling (R3, R4)
4. Reconnection backoff with jitter and stability gate (R3, R4)
5. DB pool configured with idle_timeout and max_lifetime (R3)
6. Circuit breaker with time-windowed failure counting (R4)
7. Time-windowed batch size caps (R4)

---

## Files Touched (Round 4)

**New files:** 20
- 10 wizard step components (`apps/mobile/src/screens/create-bot/*.tsx`)
- 7 api-client modules (`packages/api-client/src/*.ts`)
- `config/llmModels.ts`, `sources/health.rs`, `handlers/helpers.rs`

**Files deleted:** 7
- `BotsListScreen.tsx` (551 LOC), `AnimatedCard.tsx` (97 LOC), `DashboardHeader.tsx` (81 LOC)
- `simulate.rs`, `state.rs`, `aggregators/mod.rs`, `normalizers/mod.rs`

**Net LOC removed:** ~1,200+ lines of dead/redundant code

**Migrations added:** 2
- `017_add_missing_indexes.sql` — indexes on bots, bot_events, bot_metrics, bot_config_versions
- `018_updated_at_triggers.sql` — triggers on bot_events, bot_metrics, bot_config_versions, platform_config

---

## Deferred Items

| ID | Severity | Reason |
|----|----------|--------|
| CP-018 | Low | reqwest 0.11 to 0.12 migration — large dependency update affecting all HTTP call sites in control-plane. Requires coordinated testing. |
| MB-032 | Medium | Expo SDK 49 is past EOL; upgrading to SDK 51+ is a major version bump touching navigation, build tooling, and native dependencies. Should be a dedicated effort. |

---

## Verification

- All three Rust services (`control-plane`, `bot-runner`, `data-retrieval`) compile clean
- TypeScript passes with zero errors (`npx tsc --noEmit` in `apps/mobile`)
- No secrets introduced; no external behavior changes beyond proven bug fixes
- Full audit checklist tracked in `AUDIT_TODO.md` (134/136 fixed, 2 deferred)
