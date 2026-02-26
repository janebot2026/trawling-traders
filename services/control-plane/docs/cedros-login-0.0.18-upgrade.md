# cedros-login 0.0.18 Upgrade Checklist

## Done

- [x] `Cargo.toml`: bumped `cedros-login-server` 0.0.17 -> 0.0.18, `cedros-pay` 1.1.15 -> 1.1.16
- [x] `cargo update -p cedros-login-server -p cedros-pay`
- [x] `src/middleware/auth.rs`: `claims.email_verified.unwrap_or(true)` — old 0.0.17 tokens (field absent) default to `true`
- [x] `src/cedros/login/mod.rs`: added `..Default::default()` to `EmailConfig` for new `custom_blocked_domains` field
- [x] `cargo check` compiles
- [x] `cargo test` — 28 pass

## Post-deploy verification

- [ ] Login with verified user — decode JWT, confirm `email_verified: true`
- [ ] Login with unverified user (if `require_email_verification` is off) — confirm `email_verified: false` in JWT
- [ ] Refresh token — confirm new JWT reflects current DB `email_verified` state
- [ ] Old tokens (minted by 0.0.17) still decode without error

## Optional follow-up: soft enforcement middleware

Once JWT claims carry `email_verified`, we can add a softer enforcement path (allow login but restrict access) instead of the current all-or-nothing block at login:

- [ ] Add a new middleware that reads `require_email_verification` from `platform_config` and checks `auth.email_verified`
- [ ] Return 403 with a `{"error": "email_verification_required"}` body for unverified users
- [ ] Exempt the verification endpoints (`/v1/auth/send-verification`, `/v1/auth/verify-email`) and `/v1/me`
- [ ] Coordinate with frontend to show a "verify your email" interstitial on 403
