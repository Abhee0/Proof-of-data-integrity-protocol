# Proof of Data Integrity Protocol (PODIP)

> Anchor SHA-256 file hashes on Ethereum. Prove a file existed, unchanged, at a specific point in time.

---

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Project Structure](#project-structure)
- [How It Works](#how-it-works)
- [Prerequisites](#prerequisites)
- [Setup](#setup)
  - [1. Smart Contract Deployment](#1-smart-contract-deployment)
  - [2. Rust Client Setup](#2-rust-client-setup)
- [Usage](#usage)
- [Security Model](#security-model)
- [Gas Costs](#gas-costs)
- [Advanced Topics](#advanced-topics)
- [Potential Improvements](#potential-improvements)

---

## Overview

PODIP answers the question: **"Can I prove this exact file existed before a certain date, without storing its contents anywhere?"**

The answer is a SHA-256 hash stored on Ethereum. The blockchain's immutability and public auditability make it the ideal notary.

**Real-world use cases:**
- Legal documents: prove a contract was signed before a dispute arose
- Research data: timestamp datasets before publication
- Audit trails: anchor log files or build artifacts
- Intellectual property: stake a claim on creative works

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        CLI (podip)                              │
│              store <file>      verify <file>                    │
└────────────────────┬────────────────────┬───────────────────────┘
                     │                    │
          ┌──────────▼──────────┐         │
          │   Hashing Module    │         │
          │  (sha2 crate)       │         │
          │  file → [u8;32]     │         │
          └──────────┬──────────┘         │
                     │                    │
          ┌──────────▼────────────────────▼──────────┐
          │          Blockchain Module                │
          │   ethers-rs + abigen!-generated bindings  │
          │                                           │
          │   store_proof(hash, filename) → tx        │
          │   get_proof_info(hash) → ProofInfo        │
          └──────────────────────┬────────────────────┘
                                 │  HTTPS JSON-RPC
                    ┌────────────▼────────────────┐
                    │     Alchemy / Infura          │
                    │     (Sepolia Testnet)         │
                    └────────────┬─────────────────┘
                                 │
                    ┌────────────▼─────────────────┐
                    │   DataIntegrity.sol           │
                    │   (Solidity 0.8.20)           │
                    │                               │
                    │   mapping(bytes32 => Record)  │
                    │   storeProof / verifyProof    │
                    │   getTimestamp / getRecord    │
                    └───────────────────────────────┘
```

### Data Flow: `store`

```
File on disk
    → SHA-256 (streaming, 64 KiB chunks)
    → bytes32 digest
    → ABI-encode as storeProof(bytes32, string)
    → sign with LocalWallet (ECDSA secp256k1)
    → broadcast to Sepolia via HTTPS RPC
    → mined → receipt → tx hash printed to user
```

### Data Flow: `verify`

```
File on disk
    → SHA-256
    → bytes32 digest
    → verifyProof(bytes32) view call (free, no gas)
    → bool + metadata returned
    → formatted and printed to user
```

---

## Project Structure

```
proof-of-data-integrity/
│
├── contracts/                    # Foundry project
│   ├── src/
│   │   └── DataIntegrity.sol     # Core smart contract
│   ├── script/
│   │   └── Deploy.s.sol          # Foundry deployment script
│   ├── test/
│   │   └── DataIntegrity.t.sol   # Solidity tests (unit + fuzz)
│   └── foundry.toml              # Foundry configuration
│
├── rust-client/                  # Rust CLI application
│   ├── src/
│   │   ├── main.rs               # Entry point: env loading, logging, dispatch
│   │   ├── cli.rs                # clap CLI definition and command handlers
│   │   ├── hashing.rs            # SHA-256 streaming file hashing
│   │   ├── blockchain.rs         # ethers-rs provider, contract bindings, tx
│   │   ├── config.rs             # Environment variable loading & validation
│   │   └── error.rs              # Typed error taxonomy (thiserror)
│   ├── Cargo.toml
│   └── .env.example
│
├── .gitignore
└── README.md
```

---

## How It Works

### The Smart Contract

`DataIntegrity.sol` stores a mapping of `bytes32 → ProofRecord`:

```solidity
struct ProofRecord {
    uint256 timestamp;  // block.timestamp at store time
    address uploader;   // msg.sender
    string  filename;   // human-readable label
}
```

**Key design choices:**
- `bytes32` for the hash: exactly 32 bytes = no dynamic storage overhead
- `timestamp != 0` as existence check: one slot, no extra bool
- Custom errors (EIP-838): ~50 gas cheaper than `require()` + string
- No raw data on-chain: privacy-preserving by design

### The Rust Client

Built with:
| Crate             | Role                                   |
|-------------------|----------------------------------------|
| `ethers`          | Ethereum provider, wallet, ABI binding |
| `sha2`            | Pure-Rust SHA-256                      |
| `clap`            | CLI argument parsing                   |
| `tokio`           | Async runtime                          |
| `thiserror`       | Typed error definitions                |
| `anyhow`          | Error context chaining                 |
| `tracing`         | Structured leveled logging             |
| `dotenvy`         | `.env` file loading                    |

---

## Prerequisites

| Tool         | Version   | Install                                  |
|--------------|-----------|------------------------------------------|
| Rust         | ≥ 1.75    | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Foundry      | latest    | `curl -L https://foundry.paradigm.xyz \| bash && foundryup` |
| Git          | any       | via package manager                       |

You also need:
- A **Sepolia testnet wallet** with some ETH (get from https://sepoliafaucet.com)
- An **Alchemy or Infura API key** (free tier is sufficient)
- An **Etherscan API key** (free, for contract verification)

---

## Setup

### 1. Smart Contract Deployment

```bash
# Clone and enter the project
git clone <repo> proof-of-data-integrity
cd proof-of-data-integrity/contracts

# Install forge-std dependency
forge install foundry-rs/forge-std --no-commit

# Run tests first — always verify before deploying
forge test -vvv

# Check gas usage
forge test --gas-report

# Set environment variables for deployment
export SEPOLIA_RPC_URL="https://eth-sepolia.g.alchemy.com/v2/YOUR_KEY"
export PRIVATE_KEY="0xYOUR_PRIVATE_KEY"
export ETHERSCAN_API_KEY="YOUR_ETHERSCAN_KEY"

# Deploy to Sepolia with automatic source verification
forge script script/Deploy.s.sol:Deploy \
    --rpc-url $SEPOLIA_RPC_URL \
    --private-key $PRIVATE_KEY \
    --broadcast \
    --verify \
    --etherscan-api-key $ETHERSCAN_API_KEY \
    -vvvv
```

**Note the deployed contract address** from the output — you'll need it in the next step.

### 2. Rust Client Setup

```bash
cd ../rust-client

# Copy and fill in the environment file
cp .env.example .env
# Edit .env with your RPC URL, private key, and contract address

# Build in release mode
cargo build --release

# Run tests
cargo test

# Verify the binary works
./target/release/podip --help
```

---

## Usage

### Store a file's proof on-chain

```bash
podip store ./contract_draft.pdf
```

Output:
```
📂  File     : ./contract_draft.pdf
🏷️   Label    : contract_draft.pdf
🔑  SHA-256  : 0x3a4b5c6d...
🔗  Wallet   : 0xAbCd...
📡  Network  : https://eth-sepolia.g.alchemy.com/v2/...
📜  Contract : 0x1234...
⏳  Sending transaction…

✅  Proof anchored on Ethereum!
    Tx Hash  : 0x9f8e7d...
    Block    : #5812340
    Gas used : 68423

💡  Verify later with:
    podip verify ./contract_draft.pdf
```

### Store with a custom label

```bash
podip store ./data.csv --filename "Q3 2024 Sales Report"
```

### Verify a file's integrity

```bash
podip verify ./contract_draft.pdf
```

Output (if found):
```
📂  File     : ./contract_draft.pdf
🔑  SHA-256  : 0x3a4b5c6d...
📡  Querying contract at 0x1234...

✅  VERIFIED — proof exists on-chain
    Stored at : 2024-11-15 14:23:07 UTC (1731677387 unix)
    Uploader  : 0xAbCd...
    Filename  : contract_draft.pdf
```

Output (if not found or file modified):
```
❌  NOT FOUND — no proof exists for this file's hash

    This could mean:
    • The file was never stored via this contract
    • The file has been modified since it was stored
    • You're querying a different contract or network
```

### Compute hash without touching the blockchain

```bash
podip hash ./largefile.bin
# SHA-256(./largefile.bin): 0x3a4b5c6d...
```

### Verbose / debug output

```bash
podip -v store ./file.pdf     # DEBUG level
podip -vv verify ./file.pdf   # TRACE level

# Or via env var (supports module filtering):
RUST_LOG=debug podip store ./file.pdf
RUST_LOG=proof_of_data_integrity=debug,ethers=warn podip store ./file.pdf
```

---

## Security Model

| Threat                    | Mitigation                                                    |
|---------------------------|---------------------------------------------------------------|
| Raw data exposure         | Only the hash is ever sent to the chain — never the data      |
| Private key exposure      | Loaded from env var / .env file, never CLI args or logs       |
| Replay attacks            | Wallet bound to chain ID; transactions are chain-specific     |
| Duplicate storage         | Contract reverts with `DuplicateProof` custom error           |
| Reentrancy                | N/A — no ETH transfers, no external calls in contract         |
| Hash collision            | SHA-256 preimage resistance (2^128 collision resistance)      |
| Timestamp manipulation    | `block.timestamp` can drift ~15s — sufficient for daily-level ordering |

**What this system does NOT provide:**
- Confidentiality (the hash is public; anyone can check if they have the file)
- Hash-to-file lookup (you need the file to verify)
- Deletion (proofs are permanent once stored)

---

## Gas Costs

Measured on Sepolia (approximate, varies with network congestion):

| Operation     | Gas     | USD @ $2000 ETH, 10 gwei |
|---------------|---------|--------------------------|
| `storeProof`  | ~68,000 | ~$0.014                  |
| `verifyProof` | 0       | free (view call)         |
| `getTimestamp`| 0       | free (view call)         |

The contract uses `bytes32` (a single 32-byte slot) rather than `string` for the hash key,
which saves ~20,000 gas vs. dynamic storage for the key path.

---

## Advanced Topics

### Event Listening

The contract emits `ProofStored(bytes32 indexed hash, address indexed uploader, ...)`.

To stream live events using ethers-rs:

```rust
use ethers::providers::{Provider, Ws};

let provider = Provider::<Ws>::connect("wss://eth-sepolia.g.alchemy.com/v2/KEY").await?;
let contract = DataIntegrity::new(address, Arc::new(provider));

let events = contract.event::<ProofStoredFilter>();
let mut stream = events.stream().await?;

while let Some(Ok(event)) = stream.next().await {
    println!("New proof: hash={}", hex::encode(event.hash));
}
```

### Adding IPFS for Full Provenance

Store the file on IPFS → get a CID → store `sha256(file)` + `cid` on-chain:

```solidity
struct ProofRecord {
    uint256 timestamp;
    address uploader;
    string  filename;
    string  ipfsCid;   // e.g. "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi"
}
```

This provides: timestamp + integrity guarantee + retrieval path — a complete provenance chain.

### Zero-Knowledge Proofs (ZK)

For privacy-sensitive use cases, replace the plain SHA-256 hash with a ZK proof:
- **Prove you know a file matching the hash** without revealing the hash
- Use **Poseidon hash** (ZK-friendly) instead of SHA-256
- Libraries: `bellman` (Rust), `snarkjs` (JS), `circom` (circuit language)

This allows: "I can prove this document satisfies conditions X and Y, without revealing the document."

---

## Potential Improvements

| Improvement           | Benefit                                              | Complexity |
|-----------------------|------------------------------------------------------|------------|
| IPFS integration      | Full provenance: timestamp + integrity + retrieval   | Medium     |
| ZK proofs (Poseidon)  | Privacy-preserving verification                      | High       |
| Batch storage         | Multiple files in one tx — amortize gas costs        | Low        |
| Merkle tree of hashes | Store one root for N files — massive gas savings     | Medium     |
| ENS for uploaders     | `0xAbCd...` → `alice.eth` in output                 | Low        |
| Hardware wallet (Ledger) | Never hold private key in software               | Medium     |
| Multi-sig contract    | Require M-of-N approvals before storing             | High       |
| Revocation registry   | Soft-invalidate proofs (can't delete, but can flag) | Low        |
| Cross-chain (L2)      | Optimism/Arbitrum: same security, 100× cheaper gas  | Medium     |

---

## License

MIT
