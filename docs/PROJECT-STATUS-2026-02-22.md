# Project Status Report — 2026-02-22

## TL;DR

The codebase is in good shape. A comprehensive audit was completed and 14 of 17 findings were fixed across 18 commits. All Rust services compile and pass tests. **22 commits are sitting locally unpushed.** There's a minor `cargo fmt` issue that will block CI. Three large refactors were deferred. The mobile app is feature-complete for MVP but has zero tests.

---

## 1. What's Done (Completely)

### Audit Remediation (14/17 fixed)

Every Critical and High severity finding is resolved:

| ID | Severity | What was fixed |
|----|----------|----------------|
| BUG-001 | **Critical** | Auth bypass — bot token compared plaintext vs hash. Now hashes before compare. |
| BUG-002 | High | DisableLiveTrading mutated immutable config. Now creates new version row. |
| SEC-001 | High | LLM context was 30 messages (cost + data leak). Capped to 10. |
| PERF-001 | High | 3 sequential DB queries per bot poll. Joined to 2. |
| BUG-003 | Medium | Subscription cache grew unbounded. Added 5-min eviction. |
| BUG-004 | Medium | `bot_shutdown`/`portfolio_snapshot` events rejected. Added to allowlist. |
| REL-001 | Medium | No graceful shutdown. Added SIGTERM/SIGINT handler. |
| REL-002 | Medium | No drawdown risk rail. Added max_drawdown_percent enforcement. |
| REL-004 | Medium | Prices always zero. Now fetches from data-retrieval service. |
| SEC-002 | Medium | LLM API key serialization leak. Added skip_serializing. |
| BUG-005 | Low | Config update race condition. Wrapped in transaction. |
| SEC-003 | Low | Rate limiter trusted X-Forwarded-For. Now uses socket IP only. |
| CLEAN-001 | Low | Blanket `#![allow(dead_code)]`. Replaced with targeted annotations. |
| CLEAN-002 | Low | Duplicate `get_authorized_bot` in 2 files. Consolidated to helpers.rs. |
| CLEAN-003 | Low | Deleted deprecated `docs/frontend-architecture.md`. |
| REL-003 | Low | `get_recent_events` always empty. Now reads journal entries from disk. |
| REL-005 | Low | CI `skip_tests` bypass. Removed entirely — tests always gate builds. |

### Backend Services

| Service | Compiles | Tests | Status |
|---------|----------|-------|--------|
| control-plane (Rust/Axum) | Yes | 22/22 pass | Production-ready |
| data-retrieval (Rust/Axum) | Yes | 10/10 pass (5 network tests skipped) | Production-ready |
| bot-runner (Rust) | Yes | 38/38 pass | Production-ready |

### Infrastructure

- CI/CD pipeline: Tests mandatory, Docker build + push, SSH deploy, health checks
- Database: 18 migrations, schema stable, triggers for `updated_at`
- Docker: Multi-stage builds for both deployed services
- Server: Running at 178.63.98.99 with health checks

### TypeScript Packages

- `@trawling-traders/types` — compiles clean
- `@trawling-traders/api-client` — compiles clean, full endpoint coverage

---

## 2. What Needs Immediate Attention

### 2a. Push the audit commits (BLOCKING)

**22 commits are local-only.** Nothing has been pushed to origin/master. This is the entire audit remediation work. If something happens to your machine, it's all gone.

```
git push origin master
```

### 2b. Fix `cargo fmt` before pushing (BLOCKING CI)

The control-plane has formatting drift in ~6 files. CI runs `cargo fmt --check` and will reject the push. This is a 30-second fix:

```
cd services/control-plane && cargo fmt
```

Then commit and push. The formatting changes are trivial (line wrapping).

---

## 3. What Was Deferred (Intentionally)

These are large refactors with low ROI relative to the functional fixes. None affect correctness or security.

| ID | What | Why deferred |
|----|------|-------------|
| PERF-002 | Split oversized files (bots.rs 1252 LOC, models/mod.rs 877 LOC) | Multi-file split, high churn, no behavioral benefit |
| MAINT-001 | Split executor.rs (1032 LOC) into submodules | Large refactor, functions >60 LOC but working correctly |
| MAINT-002 | Split CreateBotWizard.styles.ts (642 LOC) | Cosmetic, styles-only file, no logic impact |

**Recommendation**: Tackle these during a dedicated refactoring session when there's nothing more pressing. They violate the 500-line file budget but don't cause bugs.

---

## 4. What Hasn't Been Started / Gaps

### Mobile App Testing — Zero Tests

The React Native app has **no tests whatsoever**. Jest is in package.json but there's no config and no test files. 19 functional screens, complex navigation, Zustand state — all untested.

**Risk**: High. Any refactor or dependency upgrade is flying blind.

### Stub Screens in Mobile

Three bottom-tab screens are empty placeholders:
- **Research** — "Coming Soon"
- **Leaderboard** — "Coming Soon"
- **Community** — "Coming Soon"

These are visible to users and make the app feel unfinished.

### API Response Format Inconsistency (CI-003)

The api-client has a dual camelCase/snake_case fallback pattern for parsing API responses. The backend sometimes returns snake_case, sometimes camelCase. This is documented but unfixed — the client works around it with fallback mappers.

### No Mobile App README

There's no setup guide for the mobile app. Anyone onboarding would have to reverse-engineer the Expo + workspace setup.

### Stale Branch

`codex/mobile-vision-chat` exists locally but hasn't been merged or cleaned up.

### Feature Flag: `feat/token-registry`

Remote branch `feat/token-registry` exists on origin — unclear if it's stale or in-progress.

---

## 5. Architecture Overview (Refresher)

```
                    ┌──────────────┐
                    │  Mobile App  │  React Native (Expo)
                    │  (Zustand)   │  Cedros Login/Pay SDKs
                    └──────┬───────┘
                           │ HTTPS
                    ┌──────▼───────┐
                    │ control-plane│  Rust/Axum 0.8, port 3000
                    │  (API + Auth │  Cedros Login + Pay embedded
                    │  + Payments) │  PostgreSQL via SQLx
                    └──┬───────┬───┘
                       │       │
              ┌────────▼──┐  ┌─▼──────────────┐
              │data-retrvl│  │  DigitalOcean   │
              │ port 8080 │  │  Droplets (VPS) │
              │ CoinGecko │  │  ┌────────────┐ │
              │ Binance WS│  │  │ bot-runner  │ │
              │ Pyth      │  │  │ (per bot)   │ │
              └───────────┘  │  └────────────┘ │
                             └─────────────────┘
```

**Key flows**:
- User creates bot via mobile → control-plane provisions DigitalOcean droplet
- bot-runner on droplet polls control-plane for config, executes trades via OpenClaw/Jupiter
- bot-runner pushes heartbeats + events back to control-plane
- data-retrieval provides real-time prices (CoinGecko, Binance WS, Pyth)
- Cedros handles auth (JWT) and payments (Stripe)

---

## 6. Recommended Next Steps (Prioritized)

### Today — Quick Wins

1. **Fix formatting + push all commits** (~5 min)
   - `cargo fmt` in control-plane, commit, push 23 commits to origin

2. **Clean up stale branches** (~2 min)
   - Delete `codex/mobile-vision-chat` if merged/abandoned
   - Check `feat/token-registry` status on origin

### This Week — High Value

3. **Mobile app tests** — at minimum, snapshot tests for critical screens and unit tests for Zustand stores. The app is the user-facing product and has zero test coverage.

4. **API response format standardization (CI-003)** — pick snake_case, update all endpoints, remove the dual-format fallback in api-client. This is tech debt that makes every new endpoint harder.

5. **Tackle PERF-002 / MAINT-001 file splits** — the 1000+ LOC files are hard to navigate and review. Breaking them up now prevents them from getting worse.

### When Ready

6. **Implement stub screens** (Research, Leaderboard, Community) or remove them from navigation to avoid "Coming Soon" impression.

7. **Mobile app README** — document setup, env vars, build commands, architecture for anyone else touching the code.

8. **End-to-end test** — verify the full flow: create bot → provision droplet → heartbeat → trade → event shows in app. This is currently only tested manually.

---

## 7. File Locations (Quick Reference)

| What | Where |
|------|-------|
| Audit report | `docs/AUDIT-REPORT.md` |
| Audit checklist | `AUDIT_TODO.md` |
| Audit summary | `AUDIT_SUMMARY.md` |
| CI pipeline | `.github/workflows/deploy.yml` |
| Control-plane routes | `services/control-plane/src/main.rs` |
| Bot handlers | `services/control-plane/src/handlers/bots.rs` |
| Bot-runner entry | `services/bot-runner/src/runner.rs` |
| Risk rails | `services/bot-runner/src/decision.rs` |
| Price aggregation | `services/data-retrieval/src/` |
| Mobile screens | `apps/mobile/src/screens/` |
| API client | `packages/api-client/src/` |
| Shared types | `packages/types/src/index.ts` |
| DB migrations | `services/control-plane/migrations/` |
| Docker compose | `docker-compose.yml` |
| Coding standards | `CLAUDE.md` |
