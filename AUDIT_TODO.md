# Audit Master Checklist

This checklist tracks all findings from the latest full-repo audit.
Order: Critical/High first, then Medium, then Low.

## Critical

- [x] **F-001 — Batch price contract mismatch between bot-runner and data-retrieval**
  - Files touched: `services/bot-runner/src/decision.rs` (and tests)
  - Planned fix:
    - Align batch response parsing with actual `/prices/batch` map response.
    - Keep parsing robust against schema drift where safe.
    - Add regression test for response parsing contract.
  - Test plan:
    - `cargo test` for bot-runner parsing tests and existing suites.
    - Manual repro: verify non-zero parsed prices from sample response payload.
  - Completion note: Added contract-compatible parser for map-shaped payloads with legacy list fallback (`parse_batch_prices_response`), plus two regression tests (`parses_map_shaped_batch_response`, `parses_legacy_list_shaped_batch_response`). Verified via `cd services/bot-runner && cargo test`.

## High

- [x] **F-002 — Real-time cache key mismatch (`/USDT` vs `/USD`) causes cache bypass**
  - Files touched: `services/data-retrieval/src/lib.rs`, `services/data-retrieval/src/sources/binance_ws.rs` (and tests)
  - Planned fix:
    - Normalize websocket-derived symbols to the same quote format used by `get_price_realtime` lookups.
    - Preserve backwards compatibility for existing subscriptions.
  - Test plan:
    - `cargo test` data-retrieval.
    - Add/adjust unit test proving cache hit path for crypto symbol lookup.
  - Completion note: Added `normalize_symbol` to canonicalize Binance symbols to `/USD` and used it in trade processing. Added regression test `normalize_symbol_uses_usd_canonical_quote`. Verified via `cd services/data-retrieval && cargo test`.

- [x] **F-003 — Subscription tier semantics inconsistent for no-subscription users**
  - Files touched: `services/control-plane/src/middleware/subscription.rs` (and tests)
  - Planned fix:
    - Make enforcement match intended Free-tier behavior consistently.
    - Ensure non-paying users are not accidentally treated as fully blocked when policy expects Free-tier operations.
  - Test plan:
    - `cargo test` control-plane middleware tests.
    - Add test coverage for no-subscription user on GET and mutating endpoints.
  - Completion note: Added `should_block_for_inactive_subscription` policy helper and changed enforcement to block only inactive paid tiers on mutating requests; Free tier is no longer blanket-blocked. Added tests `allows_mutating_requests_for_free_tier`, `blocks_mutating_requests_for_inactive_paid_tiers`, and `allows_get_requests_for_all_tiers`. Verified via `cd services/control-plane && cargo test`.

- [ ] **F-004 — Bot registration errors are swallowed and runner continues**
  - Files touched: `services/bot-runner/src/main.rs` (and tests)
  - Planned fix:
    - Only ignore expected idempotent registration outcomes.
    - Propagate true registration failures to fail fast at startup.
  - Test plan:
    - `cargo test` bot-runner.
    - Add unit test for non-idempotent registration failure path.

- [ ] **F-005 — BotRunner panics on OpenClaw client init failure**
  - Files touched: `services/bot-runner/src/runner.rs`, `services/bot-runner/src/main.rs` (and tests)
  - Planned fix:
    - Replace panic path with fallible construction and explicit error propagation.
    - Preserve startup logs and failure context.
  - Test plan:
    - `cargo test` bot-runner.
    - Add unit test ensuring constructor returns `Err` instead of panic on invalid config.

## Medium

- [ ] **F-006 — CSV report generation is unbounded/heavy in request path**
  - Files touched: `services/control-plane/src/handlers/reports.rs` (and tests)
  - Planned fix:
    - Add safe upper bound and explicit truncation/error behavior for large report requests.
    - Keep output schema stable while reducing memory/latency risk.
  - Test plan:
    - `cargo test` control-plane.
    - Add tests for oversized result handling and normal-case behavior.

- [ ] **F-007 — Chat hourly quota check is non-atomic (race window)**
  - Files touched: `services/control-plane/src/handlers/chat.rs` (and tests)
  - Planned fix:
    - Enforce quota atomically in database transaction/lock step.
    - Preserve existing limit values and response semantics.
  - Test plan:
    - `cargo test` control-plane.
    - Add test for quota boundary behavior.

- [ ] **F-008 — Decision tick performs blocking filesystem reads on async path**
  - Files touched: `services/bot-runner/src/decision.rs` (and tests)
  - Planned fix:
    - Move recent-event file reads to non-blocking tokio fs APIs.
    - Keep same filtering/sorting output.
  - Test plan:
    - `cargo test` bot-runner.
    - Regression test for recent-event extraction output consistency.

- [ ] **F-009 — New reqwest client created for each batch price fetch**
  - Files touched: `services/bot-runner/src/runner.rs`, `services/bot-runner/src/decision.rs` (and tests)
  - Planned fix:
    - Reuse a shared HTTP client on `BotRunner`.
    - Keep existing timeouts and request behavior.
  - Test plan:
    - `cargo test` bot-runner.
    - Compile-time and behavior regression checks for price fetch path.

## Low

- [ ] **F-010 — Mobile lint script broken (`eslint` missing)**
  - Files touched: `apps/mobile/package.json` (possibly lockfile) and CI/test docs if needed
  - Planned fix:
    - Ensure lint command has a resolvable eslint binary.
    - Keep lint command behavior unchanged for developers/CI.
  - Test plan:
    - `cd apps/mobile && npm run lint`.
    - Verify no regression in `npm test` and workspace typecheck.
