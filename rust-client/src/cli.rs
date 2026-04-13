// src/cli.rs
//
// CLI definition and command dispatch.
//
// `clap` with the `derive` feature lets us declare the entire CLI surface as
// annotated structs. This gives us automatic --help, type coercion, and
// validation for free.
//
// Separation of concerns:
// - This module only: parses args, formats output, orchestrates calls.
// - It does NOT implement hashing or blockchain logic — it delegates to those modules.
// - This makes it easy to unit-test the core logic without simulating CLI invocations.

use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use tracing::info;

use crate::{
    blockchain::BlockchainClient,
    config::Config,
    hashing::{digest_to_bytes32, digest_to_hex, hash_file},
};

// ---------------------------------------------------------------------------
// CLI structure
// ---------------------------------------------------------------------------

/// Proof of Data Integrity Protocol — anchor and verify file hashes on Ethereum.
#[derive(Parser, Debug)]
#[command(
    name    = "podip",
    version = env!("CARGO_PKG_VERSION"),
    author  = "PODIP",
    about   = "Anchor SHA-256 file hashes on Ethereum and verify their integrity later",
    long_about = None,
)]
pub struct Cli {
    /// Increase verbosity. Use -v for DEBUG, -vv for TRACE.
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Hash a file and store its proof on the Ethereum smart contract.
    ///
    /// Example:
    ///   podip store ./report.pdf
    ///   podip store ./data.csv --filename "Q3 Sales Data"
    Store {
        /// Path to the file to hash and anchor on-chain.
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Optional human-readable label stored as metadata.
        /// Defaults to the file's base name.
        #[arg(short, long)]
        filename: Option<String>,
    },

    /// Hash a local file and check whether its proof exists on-chain.
    ///
    /// Example:
    ///   podip verify ./report.pdf
    Verify {
        /// Path to the file to verify.
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    /// Print the SHA-256 hash of a file without touching the blockchain.
    ///
    /// Useful for inspecting what would be stored.
    Hash {
        /// Path to the file to hash.
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
}

// ---------------------------------------------------------------------------
// Command handlers
// ---------------------------------------------------------------------------

/// Dispatches the parsed CLI command to the appropriate handler.
pub async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Store { file, filename } => cmd_store(file, filename).await,
        Commands::Verify { file }          => cmd_verify(file).await,
        Commands::Hash   { file }          => cmd_hash(file),
    }
}

// ---------------------------------------------------------------------------
// `store` handler
// ---------------------------------------------------------------------------

async fn cmd_store(file: PathBuf, filename_override: Option<String>) -> Result<()> {
    // Resolve the display filename: CLI arg → basename → "<unknown>"
    let filename = filename_override.unwrap_or_else(|| {
        file.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>")
            .to_string()
    });

    println!("📂  File     : {}", file.display());
    println!("🏷️   Label    : {filename}");

    // Step 1: Hash the file
    let digest = hash_file(&file)
        .with_context(|| format!("Failed to hash '{}'", file.display()))?;

    let hex_hash = digest_to_hex(&digest);
    println!("🔑  SHA-256  : 0x{hex_hash}");

    // Step 2: Load config and build blockchain client
    let config = Config::from_env()
        .context("Failed to load configuration — check your .env file")?;

    let client = BlockchainClient::new(&config)
        .await
        .context("Failed to connect to Ethereum")?;

    println!("🔗  Wallet   : {:?}", client.address());
    println!("📡  Network  : {}", config.rpc_url);
    println!("📜  Contract : {:?}", config.contract_address);
    println!();
    println!("⏳  Sending transaction…");

    // Step 3: Store proof on-chain
    let result = client
        .store_proof(digest_to_bytes32(digest), &filename)
        .await
        .with_context(|| format!("Failed to store proof for '{}'", file.display()))?;

    // Step 4: Display result
    println!();
    println!("✅  Proof anchored on Ethereum!");
    println!("    Tx Hash  : {:?}", result.tx_hash);
    if let Some(block) = result.block {
        println!("    Block    : #{block}");
    }
    if let Some(gas) = result.gas_used {
        println!("    Gas used : {gas}");
    }
    println!();
    println!("💡  Verify later with:");
    println!("    podip verify {}", file.display());

    info!("store command completed successfully");
    Ok(())
}

// ---------------------------------------------------------------------------
// `verify` handler
// ---------------------------------------------------------------------------

async fn cmd_verify(file: PathBuf) -> Result<()> {
    println!("📂  File     : {}", file.display());

    // Step 1: Hash the local file
    let digest = hash_file(&file)
        .with_context(|| format!("Failed to hash '{}'", file.display()))?;

    let hex_hash = digest_to_hex(&digest);
    println!("🔑  SHA-256  : 0x{hex_hash}");

    // Step 2: Connect and query
    let config = Config::from_env()
        .context("Failed to load configuration")?;

    let client = BlockchainClient::new(&config)
        .await
        .context("Failed to connect to Ethereum")?;

    println!("📡  Querying contract at {:?}…", config.contract_address);
    println!();

    // Step 3: Query the contract
    let info = client
        .get_proof_info(digest_to_bytes32(digest))
        .await
        .context("Failed to query blockchain")?;

    // Step 4: Display result
    if info.exists {
        println!("✅  VERIFIED — proof exists on-chain");

        if let Some(ts) = info.timestamp {
            let dt = DateTime::<Utc>::from_timestamp(ts as i64, 0)
                .unwrap_or_default();
            println!("    Stored at : {} ({ts} unix)", dt.format("%Y-%m-%d %H:%M:%S UTC"));
        }
        if let Some(addr) = info.uploader {
            println!("    Uploader  : {addr:?}");
        }
        if let Some(name) = &info.filename {
            println!("    Filename  : {name}");
        }
    } else {
        println!("❌  NOT FOUND — no proof exists for this file's hash");
        println!();
        println!("    This could mean:");
        println!("    • The file was never stored via this contract");
        println!("    • The file has been modified since it was stored");
        println!("    • You're querying a different contract or network");
    }

    info!("verify command completed successfully");
    Ok(())
}

// ---------------------------------------------------------------------------
// `hash` handler (offline, no blockchain)
// ---------------------------------------------------------------------------

fn cmd_hash(file: PathBuf) -> Result<()> {
    let digest = hash_file(&file)
        .with_context(|| format!("Failed to hash '{}'", file.display()))?;

    println!("SHA-256({}): 0x{}", file.display(), digest_to_hex(&digest));
    Ok(())
}
