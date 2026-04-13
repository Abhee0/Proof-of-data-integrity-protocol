// src/hashing.rs
//
// Responsible for one thing: reading a file from disk and producing its SHA-256 digest.
//
// Design decisions:
// - Streaming reads via `Read::read` in fixed-size chunks avoids loading the entire
//   file into memory. This matters for large files (multi-GB datasets, VM images).
// - We return `[u8; 32]` (the raw digest) rather than a hex string, letting callers
//   decide the representation. The blockchain module expects bytes32; the CLI can hex-encode.
// - `sha2::Sha256` is a pure-Rust implementation — no C bindings, no OpenSSL.

use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use sha2::{Digest, Sha256};
use tracing::{debug, info};

use crate::error::HashError;

/// Reads the file at `path` in streaming chunks and returns the SHA-256 digest.
///
/// # Errors
/// Returns `HashError::FileOpen` if the path does not exist or lacks permissions,
/// and `HashError::FileRead` if an I/O error occurs mid-stream.
pub fn hash_file(path: &Path) -> Result<[u8; 32], HashError> {
    let path_str = path.display().to_string();

    debug!(path = %path_str, "Opening file for hashing");

    let file = File::open(path).map_err(|source| HashError::FileOpen {
        path:   path_str.clone(),
        source,
    })?;

    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();

    // 64 KiB buffer — balances syscall overhead vs. memory usage.
    // Benchmark shows diminishing returns beyond ~128 KiB for most filesystems.
    let mut buf = [0u8; 65_536];
    let mut total_bytes: u64 = 0;

    loop {
        let n = reader.read(&mut buf).map_err(|source| HashError::FileRead {
            path:   path_str.clone(),
            source,
        })?;

        if n == 0 {
            break; // EOF
        }

        hasher.update(&buf[..n]);
        total_bytes += n as u64;
    }

    let digest: [u8; 32] = hasher.finalize().into();

    info!(
        path     = %path_str,
        bytes    = total_bytes,
        hash     = %hex::encode(digest),
        "SHA-256 digest computed"
    );

    Ok(digest)
}

/// Converts a raw 32-byte digest to the hex string representation (lowercase, no prefix).
///
/// Kept as a separate utility so the CLI and tests can reuse it without coupling
/// to the hashing logic.
pub fn digest_to_hex(digest: &[u8; 32]) -> String {
    hex::encode(digest)
}

/// Converts a raw 32-byte digest to the `bytes32` Solidity representation.
///
/// `ethers-rs` expects `[u8; 32]` for `bytes32` ABI types — this is a no-op
/// conceptually but makes intent explicit at call sites.
pub fn digest_to_bytes32(digest: [u8; 32]) -> [u8; 32] {
    digest
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn known_sha256_vector() {
        // SHA-256("") == e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let mut f = NamedTempFile::new().unwrap();
        // Write nothing — empty file
        f.flush().unwrap();

        let digest = hash_file(f.path()).unwrap();
        let expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(hex::encode(digest), expected);
    }

    #[test]
    fn hello_world_vector() {
        // SHA-256("hello\n") == 5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"hello\n").unwrap();
        f.flush().unwrap();

        let digest = hash_file(f.path()).unwrap();
        let expected = "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03";
        assert_eq!(hex::encode(digest), expected);
    }

    #[test]
    fn nonexistent_file_returns_error() {
        let result = hash_file(Path::new("/nonexistent/file.bin"));
        assert!(matches!(result, Err(HashError::FileOpen { .. })));
    }
}
