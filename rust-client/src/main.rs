// src/main.rs
//
// Application entry point.
//
// Responsibilities:
//  1. Load .env file (dotenvy) before anything else reads env vars
//  2. Parse CLI arguments
//  3. Configure the tracing subscriber (structured logging)
//  4. Dispatch to the CLI runner
//  5. Convert errors to human-readable output and set the exit code
//
// Nothing domain-specific lives here — main.rs is intentionally thin.

mod blockchain;
mod cli;
mod config;
mod error;
mod hashing;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

use crate::cli::{Cli, run};

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Load .env before parsing env vars anywhere
    //    `ok()` silences the error if .env doesn't exist — that's fine in CI/CD
    //    environments where vars are injected directly into the process environment.
    dotenvy::dotenv().ok();

    // 2. Parse CLI args first so we have the verbosity level for logging setup
    let cli = Cli::parse();

    // 3. Configure tracing subscriber
    //
    //    Verbosity levels:
    //      (default) → INFO and above
    //      -v        → DEBUG and above
    //      -vv       → TRACE and above
    //
    //    The RUST_LOG env var can override this: `RUST_LOG=ethers=debug podip store file.pdf`
    let level = match cli.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    // Compose a filter that respects RUST_LOG if set, otherwise falls back to `level`
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    fmt()
        .with_env_filter(filter)
        // Compact format for CLI: no timestamps in normal mode, more readable
        .with_target(cli.verbose > 0)  // show module path only in verbose mode
        .with_thread_ids(false)
        .compact()
        .init();

    // 4. Run the command — errors propagate through anyhow and are printed by the runtime
    run(cli).await.map_err(|err| {
        // Print a clean error chain (not a Rust debug representation)
        eprintln!("\n🚨  Error: {err}");
        for cause in err.chain().skip(1) {
            eprintln!("   caused by: {cause}");
        }
        // Suppress the default panic/error output from the `?` propagation
        std::process::exit(1);
    })
}
