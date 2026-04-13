// src/blockchain.rs
//
// All Ethereum interaction lives here: provider setup, contract binding, tx dispatch,
// and result decoding.
//
// Architecture notes:
// - `ethers::contract::abigen!` macro generates a fully typed Rust wrapper at compile
//   time from the contract ABI. This means encoding/decoding errors are caught at
//   compile time rather than at runtime — a significant safety win.
// - We use a `SignerMiddleware<Provider<Http>, LocalWallet>` stack:
//     Provider<Http>  — handles RPC transport
//     LocalWallet     — signs transactions with the in-process private key
//     SignerMiddleware — wires them together and auto-fills nonce + chain ID
// - Gas estimation is performed automatically by the middleware unless overridden.
// - We never call `.unwrap()` — all Results are propagated with `?`.

use std::sync::Arc;

use ethers::{
    contract::abigen,
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
    types::{Address, TransactionReceipt, H256, U256},
};
use tracing::{debug, info, warn};

use crate::{config::Config, error::BlockchainError};

// ---------------------------------------------------------------------------
// ABI binding — generated at compile time from inline JSON.
//
// The ABI here mirrors DataIntegrity.sol exactly. If you change the contract,
// regenerate this with: `forge inspect DataIntegrity abi` and paste the output.
// ---------------------------------------------------------------------------
abigen!(
    DataIntegrity,
    "../contracts/out/DataIntegrity.sol/DataIntegrity.json"
);

// ---------------------------------------------------------------------------
// Type aliases — reduces boilerplate in function signatures
// ---------------------------------------------------------------------------

type SignedProvider = SignerMiddleware<Provider<Http>, LocalWallet>;
type BoundContract  = DataIntegrity<SignedProvider>;

// ---------------------------------------------------------------------------
// Public result types
// ---------------------------------------------------------------------------

/// Everything we know about a stored proof record.
#[derive(Debug)]
pub struct ProofInfo {
    pub exists:    bool,
    pub timestamp: Option<u64>,   // Unix seconds
    pub uploader:  Option<Address>,
    pub filename:  Option<String>,
}

/// Summary returned after a successful store transaction.
#[derive(Debug)]
pub struct StoreResult {
    pub tx_hash:  H256,
    pub gas_used: Option<U256>,
    pub block:    Option<u64>,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Wraps the provider + wallet + contract instance.
/// Constructed once per CLI invocation; cheaply cloneable via Arc internals.
pub struct BlockchainClient {
    contract: BoundContract,
    wallet:   LocalWallet,
}

impl BlockchainClient {
    /// Constructs a new client from the loaded `Config`.
    ///
    /// This performs three fallible operations:
    ///  1. Parse the private key into a `LocalWallet`
    ///  2. Connect the HTTP provider to the RPC URL
    ///  3. Fetch the chain ID (one round-trip) to bind the wallet to the correct chain
    pub async fn new(config: &Config) -> Result<Self, BlockchainError> {
        debug!(rpc = %config.rpc_url, "Connecting to Ethereum node");

        // 1. Parse private key — strip optional "0x" prefix for compatibility
        let raw_key = config.private_key.trim_start_matches("0x");
        let wallet: LocalWallet = raw_key
            .parse()
            .map_err(|_| BlockchainError::InvalidPrivateKey(
                // Never log the actual key — log only that parsing failed
                "key parse failed (check format: 64 hex chars, no 0x prefix required)".to_string()
            ))?;

        // 2. Create HTTP provider
        let provider = Provider::<Http>::try_from(config.rpc_url.as_str())
            .map_err(|e| BlockchainError::Provider(e.to_string()))?;

        // 3. Fetch chain ID and bind wallet — prevents replay attacks across chains
        let chain_id = provider
            .get_chainid()
            .await
            .map_err(|e| BlockchainError::Provider(e.to_string()))?
            .as_u64();

        info!(chain_id, "Connected to Ethereum node");

        let wallet = wallet.with_chain_id(chain_id);

        // 4. Wire together with SignerMiddleware
        let signer = Arc::new(SignerMiddleware::new(provider, wallet.clone()));

        // 5. Bind to the deployed contract
        let contract = DataIntegrity::new(config.contract_address, signer);

        Ok(BlockchainClient { contract, wallet })
    }

    /// The Ethereum address derived from the loaded private key.
    pub fn address(&self) -> Address {
        self.wallet.address()
    }

    // -----------------------------------------------------------------------
    // Write operations
    // -----------------------------------------------------------------------

    /// Sends a `storeProof(hash, filename)` transaction and waits for confirmation.
    ///
    /// Returns a `StoreResult` with the transaction hash and receipt details.
    /// On revert (e.g. duplicate hash), decodes the custom Solidity error if possible.
    pub async fn store_proof(
        &self,
        hash:     [u8; 32],
        filename: &str,
    ) -> Result<StoreResult, BlockchainError> {
        info!(
            hash     = %hex::encode(hash),
            filename = %filename,
            "Sending storeProof transaction"
        );

        // Build call — middleware auto-estimates gas and fills nonce
        let call = self.contract.store_proof(hash, filename.to_string());

        // Send and await mining (default 1 confirmation)
        let pending = call
            .send()
            .await
            .map_err(|e| decode_contract_error(e))?;

        debug!(tx = ?pending.tx_hash(), "Transaction submitted, awaiting confirmation");

        let receipt: TransactionReceipt = pending
            .await
            .map_err(|e| BlockchainError::TransactionFailed(e.to_string()))?
            .ok_or_else(|| BlockchainError::TransactionFailed(
                "transaction dropped from mempool".to_string()
            ))?;

        // Status == 1 means success; 0 means reverted (shouldn't reach here
        // if the node returns proper revert data, but defensive check is cheap)
        if receipt.status != Some(1u64.into()) {
            return Err(BlockchainError::TransactionFailed(format!(
                "receipt status=0, tx: {:?}", receipt.transaction_hash
            )));
        }

        info!(
            tx    = ?receipt.transaction_hash,
            block = ?receipt.block_number,
            gas   = ?receipt.gas_used,
            "Proof stored on-chain"
        );

        Ok(StoreResult {
            tx_hash:  receipt.transaction_hash,
            gas_used: receipt.gas_used,
            block:    receipt.block_number.map(|b| b.as_u64()),
        })
    }

    // -----------------------------------------------------------------------
    // Read operations
    // -----------------------------------------------------------------------

    /// Queries the contract for the existence and metadata of a hash.
    ///
    /// Uses two view calls: `verifyProof` (existence) and `getProofRecord` (details).
    /// View calls are free (no gas) and don't require a funded wallet.
    pub async fn get_proof_info(
        &self,
        hash: [u8; 32],
    ) -> Result<ProofInfo, BlockchainError> {
        debug!(hash = %hex::encode(hash), "Querying proof");

        // First check existence — avoids a revert on getProofRecord for unknown hashes
        let exists: bool = self
            .contract
            .verify_proof(hash)
            .call()
            .await
            .map_err(|e| BlockchainError::Provider(e.to_string()))?;

        if !exists {
            return Ok(ProofInfo {
                exists:    false,
                timestamp: None,
                uploader:  None,
                filename:  None,
            });
        }

        // Fetch full record — safe to call because we verified existence above
        let record = self
            .contract
            .get_proof_record(hash)
            .call()
            .await
            .map_err(|e| BlockchainError::Provider(e.to_string()))?;

        Ok(ProofInfo {
            exists:    true,
            timestamp: Some(record.timestamp.as_u64()),
            uploader:  Some(record.uploader),
            filename:  Some(record.filename),
        })
    }
}

// ---------------------------------------------------------------------------
// Error decoding helper
// ---------------------------------------------------------------------------

/// Attempts to decode Solidity custom errors from ethers ContractError.
/// Falls back to a raw string if the ABI decode fails.
fn decode_contract_error<M: Middleware>(err: ethers::contract::ContractError<M>) -> BlockchainError {
    // `decode_revert` is generated by abigen! for each custom error in the ABI.
    // If the revert data matches a known error selector, we get a typed variant.
    let reason = match &err {
        ethers::contract::ContractError::Revert(bytes) => {
            // Try to decode as a known custom error — fallback to hex
            format!("revert data: 0x{}", hex::encode(bytes))
        }
        _ => err.to_string(),
    };

    // Check for known error signatures in the message
    if reason.contains("DuplicateProof") || reason.contains("0x12f56c97") {
        warn!("Attempted to store a duplicate hash — already anchored on-chain");
        BlockchainError::ContractRevert {
            reason: "DuplicateProof: this hash has already been stored on-chain".to_string(),
        }
    } else {
        BlockchainError::ContractRevert { reason }
    }
}
