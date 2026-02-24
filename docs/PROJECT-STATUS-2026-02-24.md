# Project Status Report — 2026-02-24

## TL;DR

cedros-pay bumped to v1.1.15 and deployed successfully. cedros-login admin auth is now working end-to-end. **cedros-pay admin routes still return 401** — the `@cedros/pay-react` SDK is not sending auth headers (same class of bug as the cedros-login issue that was just fixed). A diagnostic logging middleware has been added to cedros-pay routes to aid debugging. The backend is healthy and correctly rejects unauthenticated requests.

---

## 1. What Changed This Session

### 1a. cedros-pay v1.1.14 → v1.1.15

**Commits**: `24b315e6`, `9663969f`

| File | Change |
|------|--------|
| `services/control-plane/Cargo.toml` | `cedros-pay = "1.1.14"` → `"1.1.15"` |
| `services/control-plane/Cargo.lock` | Updated lockfile |
| All 3 services | `cargo fmt` formatting fixes |

- CI pipeline passed (lint + tests + build)
- Deploy succeeded, both containers healthy on 178.63.98.99
- cedros-pay running in full integration mode (not placeholder)

### 1b. Turbopack env-var inlining bug (web app — diagnosed, not our code)

**Root cause**: Next.js 16 Turbopack only inlines `process.env.NEXT_PUBLIC_*` when it appears as a bare standalone expression. ANY wrapping — `?.trim()`, `?? ""`, `|| ""` — breaks the SWC pattern match.

**Evidence**: In the same HTTP 500 response from the proxy:
- CSP header (using bare `process.env.NEXT_PUBLIC_TT_API_URL` in `csp.ts`) correctly contained `api.trawlingtraders.com`
- proxy.ts (using `(process.env.NEXT_PUBLIC_TT_API_URL ?? "").trim()`) got `undefined`

**Fix applied by web app team**: Split into two statements:
```typescript
const raw = process.env.NEXT_PUBLIC_TT_API_URL;
const apiBase = raw || "";
```

### 1c. cedros-login admin auth fixed

**Problem**: All `/v1/auth/admin/*` requests arrived with `has_auth=false` → 401.
**Root cause**: `@cedros/login-react` AdminShell's `getAccessToken` callback wasn't providing the JWT.
**Fix**: Applied by web app team in their SDK configuration.
**Verified**: Server logs now show `has_auth=true` → 200 OK for all cedros-login admin routes.

### 1d. cedros-pay admin auth still broken (CURRENT ISSUE)

**Problem**: ALL cedros-pay admin routes return 401:
- `/admin/transactions`, `/admin/stats`, `/admin/products`
- `/admin/subscriptions`, `/admin/coupons`, `/admin/refunds`
- `/admin/credits/refunds`, `/admin/stripe/refunds`

**Server logs**:
```
WARN cedros_pay::middleware::auth: Admin authentication failed: no valid auth method
```

**Frontend SDK logs**: `{serverUrl: 'https://api.trawling...', hasApiKey: false}`

**Backend config** (from `pay.rs`):
```rust
cfg.cedros_login.enabled = true;
cfg.cedros_login.base_url = "http://127.0.0.1:3001/v1/auth";
cfg.cedros_login.jwt_issuer = Some("cedros-login");
cfg.cedros_login.jwt_audience = Some("cedros-app");
```

**Diagnosis**: Same class of bug as cedros-login. The pay SDK isn't attaching `Authorization: Bearer <jwt>` to its admin requests. The backend is configured to accept JWTs validated via cedros-login JWKS — it just never receives one.

**Action added**: Diagnostic logging middleware on cedros-pay routes (logs `has_auth` and `has_api_key` for `/admin` requests). This will show definitively whether the auth header is absent or present-but-invalid once the frontend is fixed.

---

## 2. What Needs Immediate Attention

### 2a. Fix cedros-pay admin auth in web app (BLOCKING)

The `@cedros/pay-react` SDK's `hostContext.cedrosPay` needs to provide auth credentials. Currently it only passes `serverUrl`. The fix is the same pattern used for cedros-login: provide a `getAccessToken` callback that returns the user's JWT.

In `unified-admin-shell.tsx`, the cedrosPay hostContext should include the same auth mechanism as cedrosLogin.

### 2b. Deploy logging middleware (RECOMMENDED)

The diagnostic logging middleware added to cedros-pay routes needs to be committed, pushed, and deployed. This will confirm whether the frontend is sending auth headers or not.

---

## 3. Backend Health Status

### Endpoint verification (from server via SSH)

| Endpoint | Status | Notes |
|----------|--------|-------|
| `GET /v1/auth/.well-known/jwks.json` | 200 OK | JWKS available internally and externally |
| `POST /v1/auth/login` | Works | Returns JWT with admin claims |
| `GET /v1/auth/admin/users?limit=20` | 200 OK | After frontend fix |
| `GET /v1/auth/admin/settings` | 200 OK | After frontend fix |
| `GET /v1/auth/admin/deposits` | 200 OK | After frontend fix |
| `GET /admin/transactions` | 401 | No auth header sent by pay SDK |
| Our admin routes (`/v1/admin/bots`) | 200 OK | Working correctly |

### Container status

| Container | Image Tag | Status |
|-----------|-----------|--------|
| trawling-traders-control-plane | `9663969f...` | Up ~1h (healthy) |
| trawling-traders-data-retrieval | `9663969f...` | Up ~1h (healthy) |

---

## 4. Architecture Clarification: Route Paths

Understanding the route layout is important for debugging:

```
/v1/                     → app routes (bots, subscriptions, etc.)
/v1/admin/               → our admin routes (admin_bots, provisioning, audit)
/v1/auth/                → cedros-login (login, register, JWKS, etc.)
/v1/auth/admin/          → cedros-login admin (users, settings, deposits)
/admin/                  → cedros-pay admin (transactions, stats, products)
                           (pay route_prefix="" + internal /admin/ prefix)
/paywall/v1/             → cedros-pay user-facing routes
```

**Key detail**: cedros-pay admin routes live at `/admin/*` (top-level), not under `/v1/pay/admin/*`. This is because `cfg.server.route_prefix = ""` and cedros-pay applies its own internal `/admin/` prefix.

---

## 5. Commits This Session

| Hash | Message |
|------|---------|
| `24b315e6` | `chore(deps): bump cedros-pay 1.1.14 -> 1.1.15` |
| `9663969f` | `style: apply cargo fmt across all services` |
| (pending) | Diagnostic logging middleware for cedros-pay admin routes |

---

## 6. Risks & Assumptions

1. **cedros-pay admin auth method**: We configured `cedros_login.enabled = true` so the pay server should accept JWTs validated via cedros-login JWKS. The "no valid auth method" error means either (a) no auth header is being sent, or (b) the JWT doesn't have the right claims. The logging middleware will disambiguate.

2. **Route prefix confusion**: The web app might be sending requests to the wrong path. The pay SDK sends to `serverUrl + /admin/transactions`. If `serverUrl` is `https://api.trawlingtraders.com`, requests go to `/admin/transactions` which is correct.

3. **Turbopack env-var pattern**: The split-variable workaround is fragile. Future refactors could break it again if someone consolidates the two lines. A comment in proxy.ts explaining the constraint would help.

---

## 7. Recommended Next Steps

1. **Web app team**: Add auth to `hostContext.cedrosPay` in `unified-admin-shell.tsx` — same `getAccessToken` pattern as cedrosLogin
2. **Deploy logging middleware**: Commit + push the cedros-pay diagnostic logging to aid further debugging
3. **Once frontend fixed**: Verify cedros-pay v1.1.15 admin JWT claim fix actually works end-to-end
