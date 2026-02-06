//! User data generation for bot droplet provisioning
//!
//! Generates cloud-init user_data scripts with modern Node.js 20 LTS + pnpm support.
//! Based on claw-spawn's bootstrap pattern with Trawling Traders specific configuration.

use uuid::Uuid;

/// Configuration for user_data generation
#[derive(Debug, Clone)]
pub struct UserDataConfig {
    pub control_plane_url: String,
    /// Node.js major version (default: 20)
    pub node_major: u8,
    /// Install pnpm via corepack (default: true)
    pub install_pnpm: bool,
    /// Optional specific pnpm version
    pub pnpm_version: String,
    /// Install Rust toolchain (default: true)
    pub install_rust: bool,
    /// Rust toolchain (default: stable)
    pub rust_toolchain: String,
    /// Extra apt packages to install
    pub extra_apt_packages: String,
    /// Downrigger repository URL
    pub downrigger_repo_url: String,
    /// Downrigger git ref (branch/tag/SHA)
    pub downrigger_ref: String,
}

impl Default for UserDataConfig {
    fn default() -> Self {
        Self {
            control_plane_url: "https://api.trawling-traders.com".to_string(),
            node_major: 20,
            install_pnpm: true,
            pnpm_version: String::new(),
            install_rust: true,
            rust_toolchain: "stable".to_string(),
            extra_apt_packages: String::new(),
            downrigger_repo_url: "https://github.com/janebot2026/downrigger.git".to_string(),
            downrigger_ref: "main".to_string(),
        }
    }
}

/// Generate cloud-init user_data script for droplet provisioning
///
/// SECURITY: Secrets are NOT embedded in user-data. Instead, the bot uses a
/// one-time bootstrap token to fetch secrets from the control plane API after boot.
pub fn generate_user_data(
    bot_id: Uuid,
    bot_name: &str,
    bootstrap_token: &str,
    config: &UserDataConfig,
) -> String {
    let bootstrap_script = include_str!("../scripts/trawler-bootstrap.sh");

    format!(
        r##"#!/bin/bash
# Trawling Traders Bot Bootstrap for Bot {}
set -e

# NOTE: Do not enable `set -x` (xtrace). This user-data includes secrets
# (bootstrap token) and xtrace would leak them into cloud-init logs.

export BOT_ID="{}"
export BOT_NAME="{}"
export CONTROL_PLANE_URL="{}"
export BOOTSTRAP_TOKEN="{}"

# Toolchain configuration
export TOOLCHAIN_NODE_MAJOR="{}"
export TOOLCHAIN_INSTALL_PNPM="{}"
export TOOLCHAIN_PNPM_VERSION="{}"
export TOOLCHAIN_INSTALL_RUST="{}"
export TOOLCHAIN_RUST_TOOLCHAIN="{}"
export TOOLCHAIN_EXTRA_APT_PACKAGES="{}"

# Downrigger configuration
export DOWNRIGGER_REPO_URL="{}"
export DOWNRIGGER_REF="{}"

# Start of embedded bootstrap script
{}
"##,
        bot_id,
        bot_id,
        sanitize_bot_name(bot_name),
        config.control_plane_url,
        bootstrap_token,
        config.node_major,
        config.install_pnpm,
        config.pnpm_version,
        config.install_rust,
        config.rust_toolchain,
        config.extra_apt_packages,
        config.downrigger_repo_url,
        config.downrigger_ref,
        bootstrap_script
    )
}

/// Maximum length for sanitized bot names
const MAX_BOT_NAME_LENGTH: usize = 64;

/// Sanitize user-provided bot name to prevent injection/truncation issues
fn sanitize_bot_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | ' ' | '-' | '_' => c,
            _ => '_',
        })
        .collect();

    let trimmed = sanitized.trim();
    if trimmed.len() > MAX_BOT_NAME_LENGTH {
        trimmed[..MAX_BOT_NAME_LENGTH].to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_data_does_not_contain_xtrace() {
        let config = UserDataConfig::default();
        let user_data = generate_user_data(Uuid::new_v4(), "TestBot", "test-token", &config);

        // Ensure xtrace is not enabled (would leak secrets)
        assert!(!user_data.lines().any(|l| l.trim() == "set -x"));
    }

    #[test]
    fn test_user_data_exports_toolchain_config() {
        let config = UserDataConfig {
            node_major: 20,
            install_pnpm: true,
            ..Default::default()
        };

        let user_data = generate_user_data(Uuid::new_v4(), "TestBot", "test-token", &config);

        assert!(user_data.contains("TOOLCHAIN_NODE_MAJOR=\"20\""));
        assert!(user_data.contains("TOOLCHAIN_INSTALL_PNPM=\"true\""));
    }

    #[test]
    fn test_sanitize_bot_name() {
        assert_eq!(sanitize_bot_name("My Bot"), "My Bot");
        assert_eq!(sanitize_bot_name("Bot<script>"), "Bot_script_");
        assert_eq!(sanitize_bot_name("  Spaces  "), "Spaces");

        // Test truncation
        let long_name = "a".repeat(100);
        assert_eq!(sanitize_bot_name(&long_name).len(), MAX_BOT_NAME_LENGTH);
    }
}
