use anyhow::{anyhow, Result};

/// Manages secrets and encryption for sensitive data
#[derive(Debug, Clone)]
pub struct SecretsManager {
    /// Encryption key loaded from environment
    encryption_key: Option<Vec<u8>>,
}

impl SecretsManager {
    /// Create a new SecretsManager
    pub fn new() -> Self {
        let encryption_key = std::env::var("SECRETS_ENCRYPTION_KEY")
            .ok()
            .and_then(|k| hex::decode(&k).ok());

        Self { encryption_key }
    }

    /// Decrypt an encrypted value
    pub fn decrypt(&self, encrypted: &str) -> Result<String> {
        // If no encryption key is set, assume value is plaintext (dev mode)
        if self.encryption_key.is_none() {
            return Ok(encrypted.to_string());
        }

        // For now, return plaintext - implement actual decryption as needed
        // In production, use something like AES-GCM
        Ok(encrypted.to_string())
    }

    /// Encrypt a plaintext value
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        // If no encryption key is set, return plaintext (dev mode)
        if self.encryption_key.is_none() {
            return Ok(plaintext.to_string());
        }

        // For now, return plaintext - implement actual encryption as needed
        Ok(plaintext.to_string())
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
