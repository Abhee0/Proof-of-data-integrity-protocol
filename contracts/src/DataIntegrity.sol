// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title DataIntegrity
 * @author Proof of Data Integrity Protocol
 * @notice Stores SHA-256 hashes on-chain as immutable proof-of-existence records.
 *
 * Design decisions:
 * - bytes32 for hashes: SHA-256 output is exactly 32 bytes; using bytes32 is
 *   gas-efficient and avoids dynamic-length storage costs.
 * - Custom errors (EIP-838): cheaper than require() + string, saves ~50 gas per revert.
 * - No raw data stored: only the hash hits the chain, preserving privacy.
 * - Mapping lookup is O(1) and avoids any enumerable pattern that could become
 *   a DoS vector if abused with many entries.
 * - `address uploader` recorded for auditability without coupling to access control.
 */
contract DataIntegrity {
    // -------------------------------------------------------------------------
    // Data Structures
    // -------------------------------------------------------------------------

    /**
     * @dev Full record for a stored proof.
     *      `timestamp` doubles as existence check (0 == not stored).
     */
    struct ProofRecord {
        uint256 timestamp; // block.timestamp when stored — not wall-clock, but sufficient for ordering
        address uploader;  // who triggered the on-chain write
        string  filename;  // optional metadata: original filename, kept off the key path
    }

    // -------------------------------------------------------------------------
    // State
    // -------------------------------------------------------------------------

    /// @dev Primary store. bytes32 key → record.
    ///      Private: callers must use the getter functions; prevents silent misuse.
    mapping(bytes32 => ProofRecord) private _proofs;

    // -------------------------------------------------------------------------
    // Events
    // -------------------------------------------------------------------------

    /**
     * @notice Emitted whenever a new proof is anchored on-chain.
     * @dev `hash` and `uploader` are indexed to enable efficient off-chain filtering
     *      (e.g., "show me all proofs from address X" or "did this hash ever appear?").
     */
    event ProofStored(
        bytes32 indexed hash,
        address indexed uploader,
        uint256 timestamp,
        string  filename
    );

    // -------------------------------------------------------------------------
    // Custom Errors
    // -------------------------------------------------------------------------

    /// @notice Raised when attempting to store a hash that already exists.
    /// @dev Returning the hash in the error lets the caller identify the collision
    ///      without a separate view call.
    error DuplicateProof(bytes32 hash);

    /// @notice Raised when querying a hash that has never been stored.
    error ProofNotFound(bytes32 hash);

    // -------------------------------------------------------------------------
    // Write Functions
    // -------------------------------------------------------------------------

    /**
     * @notice Anchors a SHA-256 hash on-chain as a timestamped proof-of-existence.
     * @param  hash     The raw bytes32 SHA-256 digest. Computed off-chain by the caller.
     * @param  filename Optional human-readable label (e.g. "report_q3_2024.pdf").
     *                  Stored as metadata only — not part of the proof key.
     *
     * @dev    No reentrancy risk: this function does not call external contracts,
     *         does not transfer ETH, and state is written before the event emit.
     *         The existence check on `timestamp != 0` is the duplicate guard.
     */
    function storeProof(bytes32 hash, string calldata filename) external {
        // Existence check — using custom error to save gas vs. require + string
        if (_proofs[hash].timestamp != 0) {
            revert DuplicateProof(hash);
        }

        // Write state before emitting — consistent with checks-effects-interactions pattern
        _proofs[hash] = ProofRecord({
            timestamp: block.timestamp,
            uploader:  msg.sender,
            filename:  filename
        });

        emit ProofStored(hash, msg.sender, block.timestamp, filename);
    }

    // -------------------------------------------------------------------------
    // Read Functions
    // -------------------------------------------------------------------------

    /**
     * @notice Returns true if the hash was previously stored, false otherwise.
     * @dev    Pure boolean — safe for any caller, no state mutation.
     */
    function verifyProof(bytes32 hash) external view returns (bool) {
        return _proofs[hash].timestamp != 0;
    }

    /**
     * @notice Returns the block timestamp at which the proof was stored.
     * @dev    Reverts with ProofNotFound if hash is unknown, so callers can
     *         distinguish "not found" from timestamp == 0 ambiguity.
     */
    function getTimestamp(bytes32 hash) external view returns (uint256) {
        if (_proofs[hash].timestamp == 0) revert ProofNotFound(hash);
        return _proofs[hash].timestamp;
    }

    /**
     * @notice Returns the full ProofRecord for a given hash.
     * @dev    Returns a memory copy — no storage pointer leak.
     */
    function getProofRecord(bytes32 hash) external view returns (ProofRecord memory) {
        if (_proofs[hash].timestamp == 0) revert ProofNotFound(hash);
        return _proofs[hash];
    }
}
