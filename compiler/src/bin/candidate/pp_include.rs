// ============================================================================
// candidate/pp_include.rs — bounded quote-include host policy (B3-03b)
//
// Host-only leaf-include support for the candidate driver. The policy is
// deliberately narrow: quoted header names only; search only the including
// file's directory; no `-I`, angle/system roots, nested includes, include
// guards, or `#pragma once`. Paths are canonicalized, must stay under the
// including directory, and are bounded by the configured source-byte limit.
// Chip-side resolution/splicing stays in PP17/PP18.
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, PpTokenKind};
use cc_silicon_compiler::ids::PpTokenId;
use std::path::{Path, PathBuf};

/// A parsed quoted `#include "..."` line.
pub struct IncludeRequest {
    /// Header-name token (payload for PP17/PP18).
    pub header: PpTokenId,
    /// Inner header spelling without delimiters.
    pub name: Vec<u8>,
}

/// Parse exactly `# include "name"` from committed line tokens.
///
/// Anything other than a three-token line ending in a quoted `HeaderName`
/// is rejected; angle-bracket and macro-generated names stay fail-closed.
pub fn parse_include_line(
    bus: &CompilerBus,
    tokens: &[PpTokenId],
) -> Result<IncludeRequest, String> {
    if tokens.len() != 3 {
        return Err(
            "unsupported include: expected exactly `# include \"name\"` on one line".to_string(),
        );
    }
    let header = tokens[2];
    let body = bus
        .arenas
        .pp_tokens
        .get(header)
        .map_err(|_| format!("include reads missing pp-token {}", header.index()))?;
    if body.kind != PpTokenKind::HeaderName {
        return Err(
            "unsupported include: header name must be a literal `\"...\"` spelling".to_string(),
        );
    }
    let spelling = body.spelling.as_slice();
    if spelling.first() == Some(&b'<') {
        return Err(
            "unsupported include: angle-bracket/system includes need H01-pinned roots".to_string(),
        );
    }
    if spelling.len() < 2 || spelling.first() != Some(&b'"') || spelling.last() != Some(&b'"') {
        return Err("malformed include: expected `\"...\"` delimiters".to_string());
    }
    let name = spelling[1..spelling.len() - 1].to_vec();
    validate_header_name(&name)?;
    Ok(IncludeRequest { header, name })
}

/// Validate the inner header spelling under the bounded policy.
fn validate_header_name(name: &[u8]) -> Result<(), String> {
    if name.is_empty() {
        return Err("malformed include: empty header name".to_string());
    }
    if name.contains(&0) || name.contains(&b'\\') {
        return Err("unsupported include: header name contains NUL or backslash".to_string());
    }
    if name.first() == Some(&b'/') {
        return Err("unsupported include: absolute header paths are refused".to_string());
    }
    let text = std::str::from_utf8(name)
        .map_err(|_| "unsupported include: header name must be valid UTF-8".to_string())?;
    for component in text.split('/') {
        if component.is_empty() || component == "." || component == ".." {
            return Err(
                "unsupported include: empty, `.` or `..` path components are refused".to_string(),
            );
        }
    }
    Ok(())
}

/// Resolve a quoted header against the including file's directory.
///
/// The including directory and candidate file are both canonicalized; the
/// candidate must remain under the including directory, be a regular file,
/// and fit `max_source_bytes`.
pub fn resolve_header_path(
    input: &str,
    name: &[u8],
    max_source_bytes: u64,
) -> Result<PathBuf, String> {
    let text = std::str::from_utf8(name)
        .map_err(|_| "unsupported include: header name must be valid UTF-8".to_string())?;
    let including = Path::new(input);
    let dir = including.parent().unwrap_or_else(|| Path::new("."));
    let dir = dir
        .canonicalize()
        .map_err(|error| format!("cannot resolve including directory: {error}"))?;
    let candidate = dir.join(text);
    let canonical = candidate.canonicalize().map_err(|error| {
        format!(
            "cannot resolve include `{}`: {error}",
            String::from_utf8_lossy(name)
        )
    })?;
    if !canonical.starts_with(&dir) {
        return Err(format!(
            "unsupported include: `{}` escapes the including directory",
            String::from_utf8_lossy(name)
        ));
    }
    let metadata = std::fs::metadata(&canonical)
        .map_err(|error| format!("cannot stat include `{}`: {error}", canonical.display()))?;
    if !metadata.is_file() {
        return Err(format!(
            "unsupported include: `{}` is not a regular file",
            canonical.display()
        ));
    }
    if metadata.len() > max_source_bytes {
        return Err(format!(
            "include `{}` exceeds the configured source-byte limit",
            canonical.display()
        ));
    }
    Ok(canonical)
}
