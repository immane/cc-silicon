//! SHA-256 digests and canonical lowercase-hex validation.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use sha2::{Digest, Sha256, Sha512};

pub const SHA256_HEX_LEN: usize = 64;
pub const SHA512_HEX_LEN: usize = 128;
pub const COMMIT_SHA_HEX_LEN: usize = 40;

/// Lowercase hex SHA-256 of a byte slice.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    to_lower_hex(&hasher.finalize())
}

/// Lowercase hex SHA-512 of a byte slice.
pub fn sha512_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha512::new();
    hasher.update(bytes);
    to_lower_hex(&hasher.finalize())
}

/// Compute the requested digests in a single streaming pass over one reader, so
/// an asset is read once even when it carries both a fetched SHA-256 and a
/// published SHA-512.
pub fn sha256_and_sha512_reader<R: Read>(
    mut reader: R,
    want_sha256: bool,
    want_sha512: bool,
) -> io::Result<(Option<String>, Option<String>)> {
    let mut hasher256 = want_sha256.then(Sha256::new);
    let mut hasher512 = want_sha512.then(Sha512::new);
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        if let Some(hasher) = hasher256.as_mut() {
            hasher.update(&buffer[..read]);
        }
        if let Some(hasher) = hasher512.as_mut() {
            hasher.update(&buffer[..read]);
        }
    }
    Ok((
        hasher256.map(|hasher| to_lower_hex(&hasher.finalize())),
        hasher512.map(|hasher| to_lower_hex(&hasher.finalize())),
    ))
}

/// Lowercase hex SHA-256 of an arbitrary reader, streamed so large archives stay
/// memory-safe. Verifying from an already-opened handle avoids re-resolving the
/// path (see the TOCTOU note in the README).
pub fn sha256_reader<R: Read>(mut reader: R) -> io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(to_lower_hex(&hasher.finalize()))
}

/// Lowercase hex SHA-256 of a file, streamed so large archives stay memory-safe.
pub fn sha256_file(path: &Path) -> io::Result<String> {
    sha256_reader(File::open(path)?)
}

fn to_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn is_lower_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// A canonical SHA-256 asset/archive digest: exactly 64 lowercase hex chars.
pub fn is_sha256_hex(value: &str) -> bool {
    is_lower_hex(value, SHA256_HEX_LEN)
}

/// A canonical SHA-512 published digest: exactly 128 lowercase hex chars.
pub fn is_sha512_hex(value: &str) -> bool {
    is_lower_hex(value, SHA512_HEX_LEN)
}

/// A canonical full GCC commit SHA: exactly 40 lowercase hex chars.
pub fn is_commit_sha(value: &str) -> bool {
    is_lower_hex(value, COMMIT_SHA_HEX_LEN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_nist_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha512_matches_nist_vectors() {
        assert_eq!(
            sha512_hex(b""),
            "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
        );
        assert_eq!(
            sha512_hex(b"abc"),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
    }

    #[test]
    fn combined_reader_matches_individual_digests() {
        let (sha256, sha512) = sha256_and_sha512_reader(&b"abc"[..], true, true).unwrap();
        assert_eq!(sha256.as_deref(), Some(sha256_hex(b"abc").as_str()));
        assert_eq!(sha512.as_deref(), Some(sha512_hex(b"abc").as_str()));
    }

    #[test]
    fn hex_validators_reject_uppercase_and_wrong_length() {
        assert!(is_sha256_hex(&sha256_hex(b"abc")));
        assert!(!is_sha256_hex("ABC"));
        assert!(!is_sha256_hex(""));
        assert!(is_sha512_hex(&sha512_hex(b"abc")));
        assert!(!is_sha512_hex("ABC"));
        assert!(!is_sha512_hex(&sha256_hex(b"abc")));
        assert!(is_commit_sha("0123456789abcdef0123456789abcdef01234567"));
        assert!(!is_commit_sha("0123456789ABCDEF0123456789abcdef01234567"));
    }
}
