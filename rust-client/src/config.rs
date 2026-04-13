// src/config.rs
//
// Loads and validates all runtime configuration from environment variables.
//
// Why env vars?
// - Private keys must never appear in CLI arguments (they show up in `ps` / shell history).
// - RPC URLs may contain API keys — same concern.
// - `.env` files are gitignored and stay on the developer's machine.
//
// The `Config` struct is created once at startup and passed (by reference or Arc)
// to the modules that need it, avoiding scattered `std::env::var()` calls.

use std::env;

use ethers::types::Address;
use tracing::debug;

use crate::error::ConfigError;

/// Validated runtime configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// HTTPS or WSS Ethereum RPC endpoint (Alchemy / Infura / etc.)
    pub rpc_url: String,

    /// Deployed DataIntegrity contract address (checksummed EIP-55 or lowercase).
    pub contract_address: Address,

    /// Raw hex private key (without 0x prefix) — kept as String, not logged.
    pub private_key: String,

    /// Optional: uploader alias stored in proof metadata. Falls back to address string.
    pub uploader_alias: Option<String>,
}

impl Config {
    /// Reads all required variables from the environment.
    ///
    /// Returns `ConfigError` with the variable name on first failure so the user
    /// knows exactly what's missing without guessing.
    pub fn from_env() -> Result<Self, ConfigError> {
        debug!("Loading configuration from environment");

        let rpc_url = require_env("RPC_URL")?;
        validate_rpc_url(&rpc_url)?;

        let contract_address_str = require_env("CONTRACT_ADDRESS")?;
        let contract_address = contract_address_str
            .parse::<Address>()
            .map_err(|e| ConfigError::InvalidAddress {
                value:  contract_address_str.clone(),
                reason: e.to_string(),
            })?;

        let private_key = require_env("PRIVATE_KEY")?;

        // Optional — won't fail if absent
        let uploader_alias = env::var("UPLOADER_ALIAS").ok();

        Ok(Config {
            rpc_url,
            contract_address,
            private_key,
            uploader_alias,
        })
    }
}

// -------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------

fn require_env(key: &'static str) -> Result<String, ConfigError> {
    env::var(key).map_err(|source| ConfigError::MissingEnvVar { key, source })
}

fn validate_rpc_url(url: &str) -> Result<(), ConfigError> {
    if url.starts_with("https://") || url.starts_with("wss://") || url.starts_with("http://") {
        Ok(())
    } else {
        Err(ConfigError::InvalidRpcUrl {
            value:  url.to_string(),
            reason: "must start with https://, wss://, or http://".to_string(),
        })
    }
}
