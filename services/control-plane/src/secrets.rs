//! Secrets management with AES-256-GCM encryption
//!
//! Provides secure encryption for sensitive data like API keys.
//!
//! # Environment Variables
//! - `SECRETS_ENCRYPTION_KEY`: 32-byte key in hex format (64 hex characters)
//!
//! # Example
//! ```bash
//! # Generate a secure key:
//! openssl rand -hex 32
//! export SECRETS_ENCRYPTION_KEY=<64-char-hex-string>
//! ```

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::Rng;

/// AES-256-GCM nonce size (96 bits = 12 bytes)
const NONCE_SIZE: usize = 12;

/// Manages secrets and encryption for sensitive data
#[derive(Debug, Clone)]
pub struct SecretsManager {
    /// Encryption key loaded from environment (32 bytes for AES-256)
    encryption_key: Option<Vec<u8>>,
}

impl SecretsManager {
    /// Create a new SecretsManager
    ///
    /// Loads the encryption key from `SECRETS_ENCRYPTION_KEY` environment variable.
    /// Key must be 32 bytes (256 bits) provided as 64 hex characters.
    pub fn new() -> Self {
        let encryption_key = std::env::var("SECRETS_ENCRYPTION_KEY")
            .ok()
            .and_then(|k| {
                let decoded = hex::decode(&k).ok()?;
                if decoded.len() != 32 {
                    tracing::warn!(
                        "SECRETS_ENCRYPTION_KEY must be 32 bytes (64 hex chars), got {} bytes",
                        decoded.len()
                    );
                    return None;
                }
                Some(decoded)
            });

        if encryption_key.is_none() {
            tracing::warn!(
                "SECRETS_ENCRYPTION_KEY not set or invalid - secrets will be stored in plaintext!"
            );
        }

        Self { encryption_key }
    }

    /// Decrypt an encrypted value
    ///
    /// Input format: Base64(nonce || ciphertext || auth_tag)
    ///
    /// # Arguments
    /// * `encrypted` - Base64-encoded encrypted data
    ///
    /// # Returns
    /// * `Ok(String)` - Decrypted plaintext
    /// * `Err` - If decryption fails or data is corrupted
    pub fn decrypt(&self, encrypted: &str) -> Result<String> {
        // If no encryption key is set, assume value is plaintext (dev mode only)
        let key = match &self.encryption_key {
            Some(k) => k,
            None => {
                tracing::debug!("No encryption key - returning value as plaintext");
                return Ok(encrypted.to_string());
            }
        };

        // Decode base64
        let data = BASE64
            .decode(encrypted)
            .map_err(|e| anyhow!("Failed to decode base64: {}", e))?;

        // Validate minimum length (nonce + at least 1 byte + auth tag)
        if data.len() < NONCE_SIZE + 1 + 16 {
            return Err(anyhow!(
                "Encrypted data too short (min {} bytes, got {})",
                NONCE_SIZE + 17,
                data.len()
            ));
        }

        // Split nonce and ciphertext
        let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Create cipher and decrypt
        let cipher =
            Aes256Gcm::new_from_slice(key).map_err(|e| anyhow!("Invalid encryption key: {}", e))?;

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| anyhow!("Decryption failed - data may be corrupted or tampered"))?;

        String::from_utf8(plaintext).map_err(|e| anyhow!("Decrypted data is not valid UTF-8: {}", e))
    }

    /// Encrypt a plaintext value
    ///
    /// Output format: Base64(nonce || ciphertext || auth_tag)
    ///
    /// # Arguments
    /// * `plaintext` - The sensitive data to encrypt
    ///
    /// # Returns
    /// * `Ok(String)` - Base64-encoded encrypted data
    /// * `Err` - If encryption fails
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        // If no encryption key is set, return plaintext (dev mode only)
        let key = match &self.encryption_key {
            Some(k) => k,
            None => {
                tracing::warn!("No encryption key - storing secret in plaintext!");
                return Ok(plaintext.to_string());
            }
        };

        // Generate random nonce (CRITICAL: must be unique for each encryption)
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::thread_rng().fill(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Create cipher and encrypt
        let cipher =
            Aes256Gcm::new_from_slice(key).map_err(|e| anyhow!("Invalid encryption key: {}", e))?;

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;

        // Prepend nonce to ciphertext and encode as base64
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);

        Ok(BASE64.encode(&result))
    }

    /// Check if encryption is active
    pub fn is_encryption_active(&self) -> bool {
        self.encryption_key.is_some()
    }
}

impl Default for SecretsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        // Set up test encryption key (32 bytes = 64 hex chars)
        std::env::set_var(
            "SECRETS_ENCRYPTION_KEY",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );

        let manager = SecretsManager::new();
        assert!(manager.is_encryption_active());

        let plaintext = "sk-secret-api-key-12345";
        let encrypted = manager.encrypt(plaintext).unwrap();

        // Encrypted should be different from plaintext
        assert_ne!(encrypted, plaintext);
        // Encrypted should be base64
        assert!(BASE64.decode(&encrypted).is_ok());

        // Decrypt should return original
        let decrypted = manager.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);

        // Clean up
        std::env::remove_var("SECRETS_ENCRYPTION_KEY");
    }

    #[test]
    fn test_different_encryptions_produce_different_output() {
        std::env::set_var(
            "SECRETS_ENCRYPTION_KEY",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );

        let manager = SecretsManager::new();
        let plaintext = "same-secret";

        let encrypted1 = manager.encrypt(plaintext).unwrap();
        let encrypted2 = manager.encrypt(plaintext).unwrap();

        // Different nonces should produce different ciphertexts
        assert_ne!(encrypted1, encrypted2);

        // Both should decrypt to same value
        assert_eq!(manager.decrypt(&encrypted1).unwrap(), plaintext);
        assert_eq!(manager.decrypt(&encrypted2).unwrap(), plaintext);

        std::env::remove_var("SECRETS_ENCRYPTION_KEY");
    }

    #[test]
    fn test_tampered_data_fails() {
        std::env::set_var(
            "SECRETS_ENCRYPTION_KEY",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );

        let manager = SecretsManager::new();
        let encrypted = manager.encrypt("secret").unwrap();

        // Tamper with the encrypted data
        let mut data = BASE64.decode(&encrypted).unwrap();
        if let Some(byte) = data.get_mut(NONCE_SIZE + 5) {
            *byte ^= 0xFF;
        }
        let tampered = BASE64.encode(&data);

        // Decryption should fail
        assert!(manager.decrypt(&tampered).is_err());

        std::env::remove_var("SECRETS_ENCRYPTION_KEY");
    }

    #[test]
    fn test_no_key_passthrough() {
        // Ensure no key is set
        std::env::remove_var("SECRETS_ENCRYPTION_KEY");

        let manager = SecretsManager::new();
        assert!(!manager.is_encryption_active());

        let plaintext = "unencrypted-secret";

        // Without key, encrypt returns plaintext
        let encrypted = manager.encrypt(plaintext).unwrap();
        assert_eq!(encrypted, plaintext);

        // Without key, decrypt returns input
        let decrypted = manager.decrypt(plaintext).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
