// src/error.rs
//
// Centralised error taxonomy for the proof-of-data-integrity client.
//
// Design rationale:
// - `thiserror::Error` generates `std::error::Error` + `Display` impls automatically.
// - We use typed errors at module boundaries (hashing, blockchain, config) so that
//   callers can pattern-match on failure causes without string parsing.
// - At the application layer (main / CLI), we convert to `anyhow::Error` via `?`,
//   which attaches context and produces human-readable error chains.

use thiserror::Error;

/// Errors that can occur during file I/O or SHA-256 hashing.
#[derive(Debug, Error)]
pub enum HashError {
    #[error("Failed to open file '{path}': {source}")]
    FileOpen {
        path:   String,
        source: std::io::Error,
    },

    #[error("Failed to read file '{path}': {source}")]
    FileRead {
        path:   String,
        source: std::io::Error,
    },
}

/// Errors originating from blockchain interaction.
#[derive(Debug, Error)]
pub enum BlockchainError {
    /// Wallet / signer construction failures.
    #[error("Failed to parse private key: {0}")]
    InvalidPrivateKey(String),

    /// The contract function reverted (custom Solidity error decoded).
    #[error("Contract reverted: {reason}")]
    ContractRevert { reason: String },

    /// Transaction broadcast or confirmation failure.
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    /// Low-level ethers provider / transport error.
    #[error("Provider error: {0}")]
    Provider(String),

    /// ABI encoding / decoding mismatch.
    #[error("ABI error: {0}")]
    Abi(String),
}

/// Errors from configuration / environment loading.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing required environment variable '{key}': {source}")]
    MissingEnvVar {
        key:    &'static str,
        source: std::env::VarError,
    },

    #[error("Invalid contract address '{value}': {reason}")]
    InvalidAddress { value: String, reason: String },

    #[error("Invalid RPC URL '{value}': {reason}")]
    InvalidRpcUrl { value: String, reason: String },
}
