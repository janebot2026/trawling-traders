-- Migration: Seed authentication provider toggle keys into platform_config
-- Allows admins to enable/disable auth methods at runtime via the admin dashboard.
-- Only email defaults to enabled; Google/Apple/Solana require client credentials.

INSERT INTO platform_config (key, value, encrypted, description, category) VALUES
    ('email_auth_enabled', 'true', FALSE, 'Enable email/password authentication', 'authentication'),
    ('google_auth_enabled', 'false', FALSE, 'Enable Google OAuth authentication', 'authentication'),
    ('apple_auth_enabled', 'false', FALSE, 'Enable Apple Sign-In authentication', 'authentication'),
    ('solana_auth_enabled', 'false', FALSE, 'Enable Solana wallet authentication', 'authentication'),
    ('block_disposable_emails', 'false', FALSE, 'Block registration from disposable email providers', 'authentication'),
    ('require_email_verification', 'false', FALSE, 'Require email verification before login is allowed', 'authentication'),
    ('google_client_id', '', FALSE, 'Google OAuth client ID (from Google Cloud Console)', 'authentication'),
    ('apple_client_id', '', FALSE, 'Apple Services ID for Sign in with Apple (e.g. com.yourapp.service)', 'authentication'),
    ('apple_team_id', '', FALSE, 'Apple Team ID (10-character identifier from Apple Developer)', 'authentication'),
    ('instant_link_enabled', 'false', FALSE, 'Enable passwordless Instant Link sign-in via email', 'authentication'),
    ('sso_enabled', 'false', FALSE, 'Enable Enterprise SSO (SAML/OIDC) for organizations', 'authentication'),
    ('webauthn_enabled', 'false', FALSE, 'Enable WebAuthn/Passkey authentication', 'authentication'),
    ('webauthn_rp_id', '', FALSE, 'WebAuthn Relying Party ID — your domain (e.g. trawlingtraders.com)', 'authentication'),
    ('webauthn_rp_name', '', FALSE, 'WebAuthn Relying Party display name (e.g. Trawling Traders)', 'authentication'),
    ('webauthn_rp_origin', '', FALSE, 'WebAuthn origin URL (e.g. https://trawlingtraders.com)', 'authentication')
ON CONFLICT (key) DO NOTHING;
