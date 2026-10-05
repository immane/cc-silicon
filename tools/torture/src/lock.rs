//! Corpus-lock model, parser, and structural validation.
//!
//! The lock is a deterministic line-oriented text artifact under `tools/torture/`.
//! It never embeds corpus content; it pins externally fetched assets by SHA-256.
//! The grammar is intentionally small and strict: unknown records, unknown
//! fields, duplicate fields, and duplicate paths are hard errors so that no lock
//! entry is silently dropped.
//!
//! ```text
//! schema=cc-silicon.torture/lock/v1
//! status=draft
//! release gcc-release="releases/gcc-15.2.0" commit-sha="<40 hex>" source-archive="gcc-15.2.0.tar.xz" source-archive-sha256="<64 hex>"
//! profile name="linux-aarch64-lp64" dialect="gnu17" configuration="-O0" configuration="-O2" suite="compile" suite="execute"
//! target triple="aarch64-unknown-linux-gnu" abi="AAPCS64" format="ELF" data-model="LP64" endianness="little"
//! substrate execution="linux-ci-or-vm" runner="unfilled" toolchain="unfilled" sysroot="unfilled" assembler="unfilled" linker="unfilled"
//! policy acquisition="fetch-on-demand" vendoring="prohibited" hash-lock="sha256" license-distribution="<policy text>"
//! reference label="non-candidate" purpose="reference-only" authorized="true" used-for-candidate-pass-rate="false"
//! asset role="driver" path="gcc/testsuite/lib/c-torture.exp" sha256="<64 hex>" license="GPL-3.0-or-later" provenance="gcc-mirror/gcc@<sha>" suite="compile"
//! ```
//!
//! Lines beginning with `#` are comments. `key=value` assignments are top-level
//! scalars; `record key=value ...` lines build records. Values with spaces must
//! be double-quoted; `\"` and `\\` are the only escapes.

use std::collections::BTreeSet;

use crate::diagnostic::{Diagnostic, Severity};
use crate::digest::{is_commit_sha, is_sha256_hex, is_sha512_hex};
use crate::path::validate_relative_path;

pub const SCHEMA_V1: &str = "cc-silicon.torture/lock/v1";
pub const SUPPORTED_SCHEMAS: &[&str] = &[SCHEMA_V1];

/// Frozen AArch64 GNU/Linux target identity (T00/H01 contract).
pub const FROZEN_TARGET_TRIPLE: &str = "aarch64-unknown-linux-gnu";
pub const FROZEN_TARGET_ABI: &str = "AAPCS64";
pub const FROZEN_TARGET_FORMAT: &str = "ELF";
pub const FROZEN_TARGET_DATA_MODEL: &str = "LP64";
pub const FROZEN_TARGET_ENDIANNESS: &str = "little";

/// Execution substrate policy. Linux CI/VM is chosen, but no concrete runner or
/// toolchain is established yet, so those stay explicit unfilled inputs.
pub const FROZEN_SUBSTRATE_EXECUTION: &str = "linux-ci-or-vm";

/// Corpus acquisition policy: fetch on demand, pin by hash, never vendor.
pub const POLICY_ACQUISITION: &str = "fetch-on-demand";
pub const POLICY_VENDORING: &str = "prohibited";
pub const POLICY_HASH_LOCK: &str = "sha256";

/// Reference-only DejaGnu baseline policy: authorized before candidate
/// implementation, labeled non-candidate, and never candidate pass-rate evidence.
pub const REFERENCE_LABEL: &str = "non-candidate";
pub const REFERENCE_PURPOSE: &str = "reference-only";
pub const REFERENCE_AUTHORIZED: &str = "true";
pub const REFERENCE_CANDIDATE_PASS_RATE: &str = "false";

pub const KNOWN_ROLES: &[&str] = &[
    "suite-test",
    "aux",
    "header",
    "runtime",
    "driver",
    "archive",
    "signature",
    "license",
    "provenance",
    "other",
];

/// T00 requires the compile, execute, and execute/ieee scopes to be counted
/// separately, and the current lock schema is a single profile carrying an
/// explicit suite list (not a per-suite profile decomposition). A frozen lock
/// must therefore name all three baseline suites instead of silently dropping
/// one. If T00 later defines per-suite lock profiles, this rule must be revisited
/// and probably relaxed to a per-profile requirement.
pub const REQUIRED_SUITES: &[&str] = &["compile", "execute", "ieee"];

/// Record kinds that must carry at least one field. A bare record line is
/// malformed rather than silently ignored.
pub const KNOWN_RECORDS: &[&str] = &[
    "release",
    "profile",
    "target",
    "substrate",
    "policy",
    "reference",
    "asset",
];

/// Explicit unresolved sentinels. The set is deliberately narrow so that
/// ordinary values (a runner literally named `example`, a path containing
/// `placeholder`) are not misclassified as unfilled. Both `unfilled` and
/// `<unfilled>` spellings are accepted; the angle-bracket form is unambiguous
/// and never collides with ordinary text.
const UNFILLED_MARKERS: &[&str] = &["unfilled", "tbd", "todo", "fixme", "unresolved"];

/// A parsed value with the 1-based source line for diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spanned {
    pub value: String,
    pub line: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockStatus {
    Draft,
    Frozen,
}

impl LockStatus {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "draft" => Some(LockStatus::Draft),
            "frozen" => Some(LockStatus::Frozen),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            LockStatus::Draft => "draft",
            LockStatus::Frozen => "frozen",
        }
    }
}

/// Official release facts recorded from published sources. `source-archive-sha256`
/// is the locally established fetch expectation (no SHA-256 is officially
/// published for GCC 15.2); the other fields are official source facts.
#[derive(Clone, Debug, Default)]
pub struct ReleaseFields {
    pub gcc_release: Option<Spanned>,
    pub release_date: Option<Spanned>,
    pub commit_sha: Option<Spanned>,
    pub source_archive: Option<Spanned>,
    pub source_archive_sha256: Option<Spanned>,
    pub source_archive_sha512: Option<Spanned>,
    pub provenance: Option<Spanned>,
}

#[derive(Clone, Debug, Default)]
pub struct ProfileFields {
    pub name: Option<Spanned>,
    pub dialect: Option<Spanned>,
    pub configurations: Vec<Spanned>,
    pub suites: Vec<Spanned>,
}

#[derive(Clone, Debug, Default)]
pub struct TargetFields {
    pub triple: Option<Spanned>,
    pub abi: Option<Spanned>,
    pub format: Option<Spanned>,
    pub data_model: Option<Spanned>,
    pub endianness: Option<Spanned>,
}

#[derive(Clone, Debug, Default)]
pub struct SubstrateFields {
    pub execution: Option<Spanned>,
    pub runner: Option<Spanned>,
    pub toolchain: Option<Spanned>,
    pub sysroot: Option<Spanned>,
    pub assembler: Option<Spanned>,
    pub linker: Option<Spanned>,
}

#[derive(Clone, Debug, Default)]
pub struct PolicyFields {
    pub acquisition: Option<Spanned>,
    pub vendoring: Option<Spanned>,
    pub hash_lock: Option<Spanned>,
    pub license_distribution: Option<Spanned>,
}

#[derive(Clone, Debug, Default)]
pub struct ReferenceFields {
    pub label: Option<Spanned>,
    pub purpose: Option<Spanned>,
    pub authorized: Option<Spanned>,
    pub used_for_candidate_pass_rate: Option<Spanned>,
}

#[derive(Clone, Debug, Default)]
pub struct AssetFields {
    pub line: usize,
    pub role: Option<Spanned>,
    pub path: Option<Spanned>,
    pub sha256: Option<Spanned>,
    pub sha512: Option<Spanned>,
    pub license: Option<Spanned>,
    pub provenance: Option<Spanned>,
    pub suite: Option<Spanned>,
    pub size: Option<Spanned>,
}

#[derive(Clone, Debug, Default)]
pub struct ParsedLock {
    pub schema: Option<Spanned>,
    pub status: Option<Spanned>,
    pub release: ReleaseFields,
    pub profile: ProfileFields,
    pub target: TargetFields,
    pub substrate: SubstrateFields,
    pub policy: PolicyFields,
    pub reference: ReferenceFields,
    pub assets: Vec<AssetFields>,
    /// Parse-time diagnostics (malformed lines, unknown records/fields, duplicates).
    pub diagnostics: Vec<Diagnostic>,
}

/// True when a value is present, non-blank, and not an explicit unfilled marker.
pub fn is_filled(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    let canonical = trimmed
        .strip_prefix('<')
        .and_then(|inner| inner.strip_suffix('>'))
        .unwrap_or(trimmed);
    !UNFILLED_MARKERS.contains(&canonical.to_ascii_lowercase().as_str())
}

/// A weak heuristic for "strong revision linkage": provenance should embed an
/// immutable revision such as a commit SHA or tag digest. This is intentionally
/// a warning, because provenance conventions vary and the tool must not pretend
/// to verify legal truth.
fn has_revision_token(value: &str) -> bool {
    let mut run = 0usize;
    for character in value.chars() {
        if character.is_ascii_hexdigit() {
            run += 1;
            if run >= 7 {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

pub fn lock_location(line: usize) -> String {
    format!("lock:{line}")
}

pub fn parse_lock(text: &str) -> ParsedLock {
    let mut parsed = ParsedLock::default();
    let mut release_seen = BTreeSet::new();
    let mut profile_seen = BTreeSet::new();
    let mut target_seen = BTreeSet::new();
    let mut substrate_seen = BTreeSet::new();
    let mut policy_seen = BTreeSet::new();
    let mut reference_seen = BTreeSet::new();

    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if first_word(trimmed).contains('=') {
            apply_top_level(&mut parsed, trimmed, line);
            continue;
        }
        let (kind, rest) = split_kind(trimmed);
        let pairs = match tokenize_pairs(rest) {
            Ok(pairs) => pairs,
            Err(message) => {
                parsed.diagnostics.push(Diagnostic::error(
                    "lock.line.malformed",
                    lock_location(line),
                    message,
                ));
                continue;
            }
        };
        if pairs.is_empty() && KNOWN_RECORDS.contains(&kind) {
            parsed.diagnostics.push(Diagnostic::error(
                "lock.record.empty",
                lock_location(line),
                format!("record '{kind}' has no fields"),
            ));
            continue;
        }
        let diagnostics = &mut parsed.diagnostics;
        match kind {
            "release" => apply_release(
                &mut parsed.release,
                diagnostics,
                pairs,
                line,
                &mut release_seen,
            ),
            "profile" => apply_profile(
                &mut parsed.profile,
                diagnostics,
                pairs,
                line,
                &mut profile_seen,
            ),
            "target" => apply_target(
                &mut parsed.target,
                diagnostics,
                pairs,
                line,
                &mut target_seen,
            ),
            "substrate" => apply_substrate(
                &mut parsed.substrate,
                diagnostics,
                pairs,
                line,
                &mut substrate_seen,
            ),
            "policy" => apply_policy(
                &mut parsed.policy,
                diagnostics,
                pairs,
                line,
                &mut policy_seen,
            ),
            "reference" => apply_reference(
                &mut parsed.reference,
                diagnostics,
                pairs,
                line,
                &mut reference_seen,
            ),
            "asset" => apply_asset(&mut parsed.assets, diagnostics, pairs, line),
            other => diagnostics.push(Diagnostic::error(
                "lock.record.unknown",
                lock_location(line),
                format!("unknown record kind '{other}'"),
            )),
        }
    }
    parsed
}

fn first_word(line: &str) -> &str {
    line.split_whitespace().next().unwrap_or("")
}

fn split_kind(line: &str) -> (&str, &str) {
    match line.find(char::is_whitespace) {
        Some(index) => (&line[..index], line[index..].trim_start()),
        None => (line, ""),
    }
}

fn apply_top_level(parsed: &mut ParsedLock, line: &str, line_number: usize) {
    match tokenize_pairs(line) {
        Ok(mut pairs) if pairs.len() == 1 => {
            let (key, value) = pairs.remove(0);
            let spanned = Spanned {
                value,
                line: line_number,
            };
            match key.as_str() {
                "schema" => {
                    if parsed.schema.is_some() {
                        parsed.diagnostics.push(Diagnostic::error(
                            "lock.field.duplicate",
                            lock_location(line_number),
                            "duplicate top-level field 'schema'",
                        ));
                    } else {
                        parsed.schema = Some(spanned);
                    }
                }
                "status" => {
                    if parsed.status.is_some() {
                        parsed.diagnostics.push(Diagnostic::error(
                            "lock.field.duplicate",
                            lock_location(line_number),
                            "duplicate top-level field 'status'",
                        ));
                    } else {
                        parsed.status = Some(spanned);
                    }
                }
                other => parsed.diagnostics.push(Diagnostic::error(
                    "lock.field.unknown",
                    lock_location(line_number),
                    format!("unknown top-level field '{other}'"),
                )),
            }
        }
        Ok(_) => parsed.diagnostics.push(Diagnostic::error(
            "lock.line.malformed",
            lock_location(line_number),
            "expected exactly one top-level key=value assignment per line",
        )),
        Err(message) => parsed.diagnostics.push(Diagnostic::error(
            "lock.line.malformed",
            lock_location(line_number),
            message,
        )),
    }
}

fn apply_release(
    release: &mut ReleaseFields,
    diagnostics: &mut Vec<Diagnostic>,
    pairs: Vec<(String, String)>,
    line: usize,
    seen: &mut BTreeSet<String>,
) {
    for (key, value) in pairs {
        if !seen.insert(key.clone()) {
            diagnostics.push(Diagnostic::error(
                "lock.field.duplicate",
                lock_location(line),
                format!("duplicate release field '{key}'"),
            ));
            continue;
        }
        apply_named(
            diagnostics,
            line,
            "release",
            &key,
            value,
            &mut [
                ("gcc-release", &mut release.gcc_release),
                ("release-date", &mut release.release_date),
                ("commit-sha", &mut release.commit_sha),
                ("source-archive", &mut release.source_archive),
                ("source-archive-sha256", &mut release.source_archive_sha256),
                ("source-archive-sha512", &mut release.source_archive_sha512),
                ("provenance", &mut release.provenance),
            ],
        );
    }
}

fn apply_profile(
    profile: &mut ProfileFields,
    diagnostics: &mut Vec<Diagnostic>,
    pairs: Vec<(String, String)>,
    line: usize,
    seen: &mut BTreeSet<String>,
) {
    for (key, value) in pairs {
        let spanned = Spanned { value, line };
        match key.as_str() {
            "configuration" => profile.configurations.push(spanned),
            "suite" => profile.suites.push(spanned),
            _ => {
                if !seen.insert(key.clone()) {
                    diagnostics.push(Diagnostic::error(
                        "lock.field.duplicate",
                        lock_location(line),
                        format!("duplicate profile field '{key}'"),
                    ));
                    continue;
                }
                apply_named(
                    diagnostics,
                    line,
                    "profile",
                    &key,
                    spanned.value,
                    &mut [
                        ("name", &mut profile.name),
                        ("dialect", &mut profile.dialect),
                    ],
                );
            }
        }
    }
}

fn apply_target(
    target: &mut TargetFields,
    diagnostics: &mut Vec<Diagnostic>,
    pairs: Vec<(String, String)>,
    line: usize,
    seen: &mut BTreeSet<String>,
) {
    for (key, value) in pairs {
        if !seen.insert(key.clone()) {
            diagnostics.push(Diagnostic::error(
                "lock.field.duplicate",
                lock_location(line),
                format!("duplicate target field '{key}'"),
            ));
            continue;
        }
        apply_named(
            diagnostics,
            line,
            "target",
            &key,
            value,
            &mut [
                ("triple", &mut target.triple),
                ("abi", &mut target.abi),
                ("format", &mut target.format),
                ("data-model", &mut target.data_model),
                ("endianness", &mut target.endianness),
            ],
        );
    }
}

fn apply_substrate(
    substrate: &mut SubstrateFields,
    diagnostics: &mut Vec<Diagnostic>,
    pairs: Vec<(String, String)>,
    line: usize,
    seen: &mut BTreeSet<String>,
) {
    for (key, value) in pairs {
        if !seen.insert(key.clone()) {
            diagnostics.push(Diagnostic::error(
                "lock.field.duplicate",
                lock_location(line),
                format!("duplicate substrate field '{key}'"),
            ));
            continue;
        }
        apply_named(
            diagnostics,
            line,
            "substrate",
            &key,
            value,
            &mut [
                ("execution", &mut substrate.execution),
                ("runner", &mut substrate.runner),
                ("toolchain", &mut substrate.toolchain),
                ("sysroot", &mut substrate.sysroot),
                ("assembler", &mut substrate.assembler),
                ("linker", &mut substrate.linker),
            ],
        );
    }
}

fn apply_policy(
    policy: &mut PolicyFields,
    diagnostics: &mut Vec<Diagnostic>,
    pairs: Vec<(String, String)>,
    line: usize,
    seen: &mut BTreeSet<String>,
) {
    for (key, value) in pairs {
        if !seen.insert(key.clone()) {
            diagnostics.push(Diagnostic::error(
                "lock.field.duplicate",
                lock_location(line),
                format!("duplicate policy field '{key}'"),
            ));
            continue;
        }
        apply_named(
            diagnostics,
            line,
            "policy",
            &key,
            value,
            &mut [
                ("acquisition", &mut policy.acquisition),
                ("vendoring", &mut policy.vendoring),
                ("hash-lock", &mut policy.hash_lock),
                ("license-distribution", &mut policy.license_distribution),
            ],
        );
    }
}

fn apply_reference(
    reference: &mut ReferenceFields,
    diagnostics: &mut Vec<Diagnostic>,
    pairs: Vec<(String, String)>,
    line: usize,
    seen: &mut BTreeSet<String>,
) {
    for (key, value) in pairs {
        if !seen.insert(key.clone()) {
            diagnostics.push(Diagnostic::error(
                "lock.field.duplicate",
                lock_location(line),
                format!("duplicate reference field '{key}'"),
            ));
            continue;
        }
        apply_named(
            diagnostics,
            line,
            "reference",
            &key,
            value,
            &mut [
                ("label", &mut reference.label),
                ("purpose", &mut reference.purpose),
                ("authorized", &mut reference.authorized),
                (
                    "used-for-candidate-pass-rate",
                    &mut reference.used_for_candidate_pass_rate,
                ),
            ],
        );
    }
}

fn apply_asset(
    assets: &mut Vec<AssetFields>,
    diagnostics: &mut Vec<Diagnostic>,
    pairs: Vec<(String, String)>,
    line: usize,
) {
    let mut asset = AssetFields {
        line,
        ..AssetFields::default()
    };
    let mut seen = BTreeSet::new();
    for (key, value) in pairs {
        if !seen.insert(key.clone()) {
            diagnostics.push(Diagnostic::error(
                "lock.field.duplicate",
                lock_location(line),
                format!("duplicate asset field '{key}'"),
            ));
            continue;
        }
        apply_named(
            diagnostics,
            line,
            "asset",
            &key,
            value,
            &mut [
                ("role", &mut asset.role),
                ("path", &mut asset.path),
                ("sha256", &mut asset.sha256),
                ("sha512", &mut asset.sha512),
                ("license", &mut asset.license),
                ("provenance", &mut asset.provenance),
                ("suite", &mut asset.suite),
                ("size", &mut asset.size),
            ],
        );
    }
    assets.push(asset);
}

/// Assign a parsed key/value to one of a record's known fields, or report it as
/// unknown. Keeping this in one place guarantees all records reject unknown
/// fields instead of silently ignoring them.
fn apply_named(
    diagnostics: &mut Vec<Diagnostic>,
    line: usize,
    record: &str,
    key: &str,
    value: String,
    fields: &mut [(&str, &mut Option<Spanned>)],
) {
    match fields.iter_mut().find(|(name, _)| *name == key) {
        Some((_, slot)) => {
            **slot = Some(Spanned { value, line });
        }
        None => diagnostics.push(Diagnostic::error(
            "lock.field.unknown",
            lock_location(line),
            format!("unknown {record} field '{key}'"),
        )),
    }
}

fn tokenize_pairs(input: &str) -> Result<Vec<(String, String)>, String> {
    let mut chars = input.chars().peekable();
    let mut pairs = Vec::new();
    loop {
        while matches!(chars.peek(), Some(c) if c.is_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        let mut key = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c.is_whitespace() {
                break;
            }
            key.push(c);
            chars.next();
        }
        if key.is_empty() {
            return Err("expected a field name before '='".to_owned());
        }
        if chars.next() != Some('=') {
            return Err(format!("expected '=' after field '{key}'"));
        }
        let value = if chars.peek() == Some(&'"') {
            chars.next();
            let mut value = String::new();
            let mut closed = false;
            while let Some(c) = chars.next() {
                match c {
                    '"' => {
                        closed = true;
                        break;
                    }
                    '\\' => match chars.next() {
                        Some('"') => value.push('"'),
                        Some('\\') => value.push('\\'),
                        Some(other) => {
                            return Err(format!("unsupported escape '\\{other}' in quoted value"))
                        }
                        None => return Err("unterminated escape in quoted value".to_owned()),
                    },
                    _ => value.push(c),
                }
            }
            if !closed {
                return Err("unterminated quoted value".to_owned());
            }
            value
        } else {
            let mut value = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                value.push(c);
                chars.next();
            }
            if value.is_empty() {
                return Err(format!("missing value for field '{key}'"));
            }
            value
        };
        if let Some(&next) = chars.peek() {
            if !next.is_whitespace() {
                return Err(format!(
                    "field '{key}' must be followed by whitespace or end of line"
                ));
            }
        }
        pairs.push((key, value));
    }
    Ok(pairs)
}

/// Structural validation of a parsed lock. Physical verification happens later;
/// this rejects malformed identifiers, digests, paths, and policy mismatches.
///
/// Returns the parsed status (when valid) together with all diagnostics.
pub fn validate(parsed: &ParsedLock) -> (Option<LockStatus>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();

    validate_schema_and_status(parsed, &mut diagnostics);
    let status = parsed
        .status
        .as_ref()
        .and_then(|spanned| LockStatus::parse(&spanned.value));
    let frozen = status == Some(LockStatus::Frozen);

    validate_release(parsed, frozen, &mut diagnostics);
    validate_profile(parsed, frozen, &mut diagnostics);
    validate_target(parsed, frozen, &mut diagnostics);
    validate_substrate(parsed, frozen, &mut diagnostics);
    validate_policy(parsed, frozen, &mut diagnostics);
    validate_reference(parsed, frozen, &mut diagnostics);
    validate_assets(parsed, frozen, &mut diagnostics);
    validate_archive_cross_link(parsed, frozen, &mut diagnostics);

    (status, diagnostics)
}

fn validate_schema_and_status(parsed: &ParsedLock, diagnostics: &mut Vec<Diagnostic>) {
    match parsed.schema.as_ref() {
        None => diagnostics.push(Diagnostic::error(
            "lock.schema.missing",
            "lock",
            "the 'schema' field is required",
        )),
        Some(spanned) if !SUPPORTED_SCHEMAS.contains(&spanned.value.as_str()) => {
            diagnostics.push(Diagnostic::error(
                "lock.schema.unsupported",
                lock_location(spanned.line),
                format!(
                    "unsupported schema '{}'; supported: {}",
                    spanned.value,
                    SUPPORTED_SCHEMAS.join(", ")
                ),
            ));
        }
        Some(_) => {}
    }

    match parsed.status.as_ref() {
        None => diagnostics.push(Diagnostic::error(
            "lock.status.missing",
            "lock",
            "the 'status' field is required",
        )),
        Some(spanned) => {
            if LockStatus::parse(&spanned.value).is_none() {
                diagnostics.push(Diagnostic::error(
                    "lock.status.invalid",
                    lock_location(spanned.line),
                    format!(
                        "status must be 'draft' or 'frozen'; found '{}'",
                        spanned.value
                    ),
                ));
            }
        }
    }
}

fn validate_release(parsed: &ParsedLock, frozen: bool, diagnostics: &mut Vec<Diagnostic>) {
    let release = &parsed.release;
    check_filled(
        frozen,
        diagnostics,
        "release.gcc-release",
        &release.gcc_release,
    );
    check_filled(
        frozen,
        diagnostics,
        "release.commit-sha",
        &release.commit_sha,
    );
    check_filled(
        frozen,
        diagnostics,
        "release.source-archive",
        &release.source_archive,
    );
    check_filled(
        frozen,
        diagnostics,
        "release.source-archive-sha256",
        &release.source_archive_sha256,
    );
    check_filled(
        frozen,
        diagnostics,
        "release.source-archive-sha512",
        &release.source_archive_sha512,
    );
    check_filled(
        frozen,
        diagnostics,
        "release.provenance",
        &release.provenance,
    );

    if let Some(spanned) = &release.release_date {
        if !is_filled(&spanned.value) {
            report_unfilled(frozen, diagnostics, "release.release-date", spanned);
        }
    }
    if let Some(spanned) = &release.provenance {
        if is_filled(&spanned.value) && !spanned.value.contains("://") {
            diagnostics.push(Diagnostic::warning(
                "release.provenance.weak",
                lock_location(spanned.line),
                format!(
                    "release provenance '{}' does not look like a source URL; use the official source location",
                    spanned.value
                ),
            ));
        }
    }

    if let Some(spanned) = &release.commit_sha {
        if is_filled(&spanned.value) && !is_commit_sha(&spanned.value) {
            diagnostics.push(Diagnostic::error(
                "release.commit-sha.invalid",
                lock_location(spanned.line),
                "commit SHA must be exactly 40 lowercase hex characters",
            ));
        }
    }
    if let Some(spanned) = &release.source_archive_sha256 {
        if is_filled(&spanned.value) && !is_sha256_hex(&spanned.value) {
            diagnostics.push(Diagnostic::error(
                "digest.invalid",
                lock_location(spanned.line),
                format!(
                    "source archive sha256 must be 64 lowercase hex characters; found '{}'",
                    spanned.value
                ),
            ));
        }
    }
    if let Some(spanned) = &release.source_archive_sha512 {
        if is_filled(&spanned.value) && !is_sha512_hex(&spanned.value) {
            diagnostics.push(Diagnostic::error(
                "digest.invalid",
                lock_location(spanned.line),
                format!(
                    "published source archive sha512 must be 128 lowercase hex characters; found '{}'",
                    spanned.value
                ),
            ));
        }
    }
    if let Some(spanned) = &release.source_archive {
        if is_filled(&spanned.value) {
            check_relative_path(
                diagnostics,
                &spanned.value,
                spanned.line,
                "release.source-archive",
            );
        }
    }
}

fn validate_profile(parsed: &ParsedLock, frozen: bool, diagnostics: &mut Vec<Diagnostic>) {
    let profile = &parsed.profile;
    check_filled(frozen, diagnostics, "profile.name", &profile.name);
    check_filled(frozen, diagnostics, "profile.dialect", &profile.dialect);

    // The option matrix must be explicit: an absent list is a warning on a draft
    // and an error on a frozen lock, matching the README rule.
    if profile.configurations.is_empty() {
        diagnostics.push(Diagnostic::new(
            active_severity(frozen),
            "profile.configuration.missing",
            "lock",
            "no option matrix configuration is listed; a frozen lock may not assume a default matrix",
        ));
    }
    // A frozen lock must name each baseline suite separately; omitting one would
    // silently shrink the accepted scope without a T00 profile decision.
    let suite_severity = active_severity(frozen);
    for required in REQUIRED_SUITES {
        if !profile
            .suites
            .iter()
            .any(|spanned| spanned.value == *required)
        {
            diagnostics.push(Diagnostic::new(
                suite_severity,
                "profile.suite.missing",
                "lock",
                format!(
                    "the separate '{required}' suite must be declared; T00 defines no per-suite lock profile decomposition"
                ),
            ));
        }
    }
    for spanned in &profile.configurations {
        if !is_filled(&spanned.value) {
            report_unfilled(frozen, diagnostics, "profile.configuration", spanned);
        }
    }
    for spanned in &profile.suites {
        if !is_filled(&spanned.value) {
            report_unfilled(frozen, diagnostics, "profile.suite", spanned);
        }
    }
}

fn validate_target(parsed: &ParsedLock, frozen: bool, diagnostics: &mut Vec<Diagnostic>) {
    let target = &parsed.target;
    check_fixed(
        frozen,
        diagnostics,
        "target.triple",
        &target.triple,
        FROZEN_TARGET_TRIPLE,
        "the frozen H00 target is AArch64 GNU/Linux",
    );
    check_fixed(
        frozen,
        diagnostics,
        "target.abi",
        &target.abi,
        FROZEN_TARGET_ABI,
        "the frozen H00 ABI is AAPCS64",
    );
    check_fixed(
        frozen,
        diagnostics,
        "target.format",
        &target.format,
        FROZEN_TARGET_FORMAT,
        "the frozen H00 object format is ELF",
    );
    check_fixed(
        frozen,
        diagnostics,
        "target.data-model",
        &target.data_model,
        FROZEN_TARGET_DATA_MODEL,
        "the frozen H00 data model is LP64",
    );
    check_fixed(
        frozen,
        diagnostics,
        "target.endianness",
        &target.endianness,
        FROZEN_TARGET_ENDIANNESS,
        "the frozen H00 endianness is little-endian",
    );
}

fn validate_substrate(parsed: &ParsedLock, frozen: bool, diagnostics: &mut Vec<Diagnostic>) {
    let substrate = &parsed.substrate;
    check_fixed(
        frozen,
        diagnostics,
        "substrate.execution",
        &substrate.execution,
        FROZEN_SUBSTRATE_EXECUTION,
        "the chosen execution substrate is Linux CI/VM",
    );
    // The concrete runner/toolchain is not established yet; it must be present
    // and filled before a lock may freeze. `unfilled` is a draft-only marker.
    check_filled(frozen, diagnostics, "substrate.runner", &substrate.runner);
    check_filled(
        frozen,
        diagnostics,
        "substrate.toolchain",
        &substrate.toolchain,
    );
    check_filled(frozen, diagnostics, "substrate.sysroot", &substrate.sysroot);
    check_filled(
        frozen,
        diagnostics,
        "substrate.assembler",
        &substrate.assembler,
    );
    check_filled(frozen, diagnostics, "substrate.linker", &substrate.linker);
}

fn validate_policy(parsed: &ParsedLock, frozen: bool, diagnostics: &mut Vec<Diagnostic>) {
    let policy = &parsed.policy;
    check_fixed(
        frozen,
        diagnostics,
        "policy.acquisition",
        &policy.acquisition,
        POLICY_ACQUISITION,
        "corpus assets are fetched on demand, never vendored",
    );
    check_fixed(
        frozen,
        diagnostics,
        "policy.vendoring",
        &policy.vendoring,
        POLICY_VENDORING,
        "vendoring corpus assets into the repository is prohibited",
    );
    check_fixed(
        frozen,
        diagnostics,
        "policy.hash-lock",
        &policy.hash_lock,
        POLICY_HASH_LOCK,
        "corpus assets are pinned by SHA-256",
    );
    check_filled(
        frozen,
        diagnostics,
        "policy.license-distribution",
        &policy.license_distribution,
    );
}

fn validate_reference(parsed: &ParsedLock, frozen: bool, diagnostics: &mut Vec<Diagnostic>) {
    let reference = &parsed.reference;
    check_fixed(
        frozen,
        diagnostics,
        "reference.label",
        &reference.label,
        REFERENCE_LABEL,
        "the reference DejaGnu baseline is labeled non-candidate",
    );
    check_fixed(
        frozen,
        diagnostics,
        "reference.purpose",
        &reference.purpose,
        REFERENCE_PURPOSE,
        "the reference baseline is reference-only",
    );
    check_fixed(
        frozen,
        diagnostics,
        "reference.authorized",
        &reference.authorized,
        REFERENCE_AUTHORIZED,
        "a reference-only baseline is authorized before candidate implementation",
    );
    check_fixed(
        frozen,
        diagnostics,
        "reference.used-for-candidate-pass-rate",
        &reference.used_for_candidate_pass_rate,
        REFERENCE_CANDIDATE_PASS_RATE,
        "the reference baseline must never be used as candidate pass-rate evidence",
    );
}

fn validate_assets(parsed: &ParsedLock, frozen: bool, diagnostics: &mut Vec<Diagnostic>) {
    if parsed.assets.is_empty() {
        let severity = if frozen {
            Severity::Error
        } else {
            Severity::Warning
        };
        diagnostics.push(Diagnostic::new(
            severity,
            "asset.empty",
            "lock",
            "the lock lists no assets; an empty lock verifies nothing",
        ));
    }

    let mut seen_paths = BTreeSet::new();
    for asset in &parsed.assets {
        validate_asset(asset, frozen, &mut seen_paths, diagnostics);
    }
}

/// The source archive is a corpus asset. Requiring an `asset role="archive"`
/// record that cross-links to `release.source-archive` keeps it inside the
/// per-asset license/provenance checks and the duplicate-path check, so it cannot
/// be smuggled in as release metadata only.
fn validate_archive_cross_link(
    parsed: &ParsedLock,
    frozen: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let release_path = parsed
        .release
        .source_archive
        .as_ref()
        .filter(|spanned| is_filled(&spanned.value));
    let release_digest = parsed
        .release
        .source_archive_sha256
        .as_ref()
        .filter(|spanned| is_filled(&spanned.value));
    let release_sha512 = parsed
        .release
        .source_archive_sha512
        .as_ref()
        .filter(|spanned| is_filled(&spanned.value));
    let archives: Vec<&AssetFields> = parsed
        .assets
        .iter()
        .filter(|asset| {
            asset
                .role
                .as_ref()
                .is_some_and(|role| role.value == "archive")
        })
        .collect();

    let Some(path) = release_path else {
        if let Some(archive) = archives.first() {
            diagnostics.push(Diagnostic::new(
                active_severity(frozen),
                "archive.release-missing",
                asset_location(archive),
                "an asset role=\"archive\" record exists but release.source-archive is not filled",
            ));
        }
        return;
    };

    // A non-archive asset claiming the release-archive path is a collision: it
    // would double-list (and potentially double-verify) the same file.
    for asset in &parsed.assets {
        let is_archive = asset
            .role
            .as_ref()
            .is_some_and(|role| role.value == "archive");
        if is_archive {
            continue;
        }
        if let Some(asset_path) = &asset.path {
            if is_filled(&asset_path.value) && asset_path.value == path.value {
                diagnostics.push(Diagnostic::error(
                    "archive.path-collision",
                    asset_location(asset),
                    format!(
                        "asset path '{}' collides with release.source-archive but its role is not 'archive'",
                        asset_path.value
                    ),
                ));
            }
        }
    }

    match archives.as_slice() {
        [] => diagnostics.push(Diagnostic::new(
            active_severity(frozen),
            "archive.asset.missing",
            lock_location(path.line),
            "release.source-archive must also appear as an asset record with role=\"archive\" so that it carries per-asset license/provenance and participates in duplicate-path detection",
        )),
        [archive] => {
            if let Some(asset_path) = &archive.path {
                if is_filled(&asset_path.value) && asset_path.value != path.value {
                    diagnostics.push(Diagnostic::error(
                        "archive.path-mismatch",
                        asset_location(archive),
                        format!(
                            "archive asset path '{}' must equal release.source-archive '{}'",
                            asset_path.value, path.value
                        ),
                    ));
                }
            }
            if let (Some(asset_digest), Some(release_digest)) = (&archive.sha256, release_digest) {
                if is_filled(&asset_digest.value) && asset_digest.value != release_digest.value {
                    diagnostics.push(Diagnostic::error(
                        "archive.digest-mismatch",
                        asset_location(archive),
                        format!(
                            "archive asset sha256 '{}' must equal release.source-archive-sha256 '{}'",
                            asset_digest.value, release_digest.value
                        ),
                    ));
                }
            }
            if let Some(release_sha512) = release_sha512 {
                match archive.sha512.as_ref().filter(|spanned| is_filled(&spanned.value)) {
                    None => diagnostics.push(Diagnostic::new(
                        active_severity(frozen),
                        "archive.sha512-missing",
                        asset_location(archive),
                        "release.source-archive-sha512 is published, so the archive asset must also record the published sha512 for local verification",
                    )),
                    Some(asset_sha512) if asset_sha512.value != release_sha512.value => {
                        diagnostics.push(Diagnostic::error(
                            "archive.sha512-mismatch",
                            asset_location(archive),
                            format!(
                                "archive asset sha512 '{}' must equal release.source-archive-sha512 '{}'",
                                asset_sha512.value, release_sha512.value
                            ),
                        ));
                    }
                    Some(_) => {}
                }
            }
        }
        _ => diagnostics.push(Diagnostic::error(
            "archive.asset.multiple",
            "lock",
            format!(
                "expected exactly one asset role=\"archive\", found {}",
                archives.len()
            ),
        )),
    }
}

fn validate_asset(
    asset: &AssetFields,
    frozen: bool,
    seen_paths: &mut BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let location = asset_location(asset);
    check_filled(frozen, diagnostics, "asset.role", &asset.role);
    check_filled(frozen, diagnostics, "asset.path", &asset.path);
    check_filled(frozen, diagnostics, "asset.sha256", &asset.sha256);
    check_filled(frozen, diagnostics, "asset.license", &asset.license);
    check_filled(frozen, diagnostics, "asset.provenance", &asset.provenance);

    if let Some(spanned) = &asset.role {
        if is_filled(&spanned.value) && !KNOWN_ROLES.contains(&spanned.value.as_str()) {
            diagnostics.push(Diagnostic::error(
                "role.unknown",
                location.clone(),
                format!(
                    "unknown asset role '{}'; known roles: {}",
                    spanned.value,
                    KNOWN_ROLES.join(", ")
                ),
            ));
        }
    }
    if let Some(spanned) = &asset.path {
        match validate_relative_path(&spanned.value) {
            Ok(()) => {
                if !seen_paths.insert(spanned.value.clone()) {
                    diagnostics.push(Diagnostic::error(
                        "path.duplicate",
                        location.clone(),
                        format!("duplicate asset path '{}'", spanned.value),
                    ));
                }
            }
            Err(problem) => diagnostics.push(Diagnostic::error(
                problem.code(),
                location.clone(),
                format!("asset path '{}': {}", spanned.value, problem.describe()),
            )),
        }
    }
    if let Some(spanned) = &asset.sha256 {
        if is_filled(&spanned.value) && !is_sha256_hex(&spanned.value) {
            diagnostics.push(Diagnostic::error(
                "digest.invalid",
                location.clone(),
                format!(
                    "asset sha256 must be 64 lowercase hex characters; found '{}'",
                    spanned.value
                ),
            ));
        }
    }
    if let Some(spanned) = &asset.sha512 {
        if !is_filled(&spanned.value) {
            // A supplied but unresolved optional field must not be silently
            // ignored on a frozen lock.
            report_unfilled(frozen, diagnostics, "asset.sha512", spanned);
        } else if !is_sha512_hex(&spanned.value) {
            diagnostics.push(Diagnostic::error(
                "digest.invalid",
                location.clone(),
                format!(
                    "asset sha512 must be 128 lowercase hex characters; found '{}'",
                    spanned.value
                ),
            ));
        }
    }
    if let Some(spanned) = &asset.provenance {
        if is_filled(&spanned.value) && !has_revision_token(&spanned.value) {
            diagnostics.push(Diagnostic::warning(
                "provenance.revision-unresolved",
                location.clone(),
                format!(
                    "provenance '{}' embeds no revision token (>=7 hex characters); strong revision linkage is expected",
                    spanned.value
                ),
            ));
        }
    }
    if let Some(spanned) = &asset.size {
        if !is_filled(&spanned.value) {
            report_unfilled(frozen, diagnostics, "asset.size", spanned);
        } else if spanned.value.parse::<u64>().is_err() {
            diagnostics.push(Diagnostic::error(
                "size.invalid",
                location,
                format!(
                    "asset size must be a non-negative decimal integer; found '{}'",
                    spanned.value
                ),
            ));
        }
    }
}

fn asset_location(asset: &AssetFields) -> String {
    match &asset.path {
        Some(spanned) if !spanned.value.trim().is_empty() => format!("asset:{}", spanned.value),
        _ => lock_location(asset.line),
    }
}

fn check_relative_path(diagnostics: &mut Vec<Diagnostic>, raw: &str, line: usize, label: &str) {
    if let Err(problem) = validate_relative_path(raw) {
        diagnostics.push(Diagnostic::error(
            problem.code(),
            lock_location(line),
            format!("{label}: {}", problem.describe()),
        ));
    }
}

fn check_filled(
    frozen: bool,
    diagnostics: &mut Vec<Diagnostic>,
    label: &str,
    value: &Option<Spanned>,
) {
    match value {
        None => diagnostics.push(Diagnostic::new(
            active_severity(frozen),
            "field.missing",
            "lock",
            format!("required field '{label}' is absent"),
        )),
        Some(spanned) if !is_filled(&spanned.value) => {
            report_unfilled(frozen, diagnostics, label, spanned)
        }
        Some(_) => {}
    }
}

fn check_fixed(
    frozen: bool,
    diagnostics: &mut Vec<Diagnostic>,
    label: &str,
    value: &Option<Spanned>,
    expected: &str,
    requirement: &str,
) {
    match value {
        None => diagnostics.push(Diagnostic::new(
            active_severity(frozen),
            "field.missing",
            "lock",
            format!("required field '{label}' is absent; {requirement}"),
        )),
        Some(spanned) if !is_filled(&spanned.value) => diagnostics.push(Diagnostic::new(
            active_severity(frozen),
            "field.unfilled",
            lock_location(spanned.line),
            format!("required field '{label}' is unfilled; {requirement}"),
        )),
        Some(spanned) if spanned.value != expected => diagnostics.push(Diagnostic::error(
            "value.unsupported",
            lock_location(spanned.line),
            format!(
                "field '{label}' must equal '{expected}'; found '{}'; {requirement}",
                spanned.value
            ),
        )),
        Some(_) => {}
    }
}

fn report_unfilled(
    frozen: bool,
    diagnostics: &mut Vec<Diagnostic>,
    label: &str,
    spanned: &Spanned,
) {
    diagnostics.push(Diagnostic::new(
        active_severity(frozen),
        "field.unfilled",
        lock_location(spanned.line),
        format!(
            "required field '{label}' is unfilled (value '{}')",
            spanned.value
        ),
    ));
}

fn active_severity(frozen: bool) -> Severity {
    if frozen {
        Severity::Error
    } else {
        Severity::Warning
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_quoted_and_bare_values() {
        let pairs = tokenize_pairs(r#"name="hello world" count=3"#).unwrap();
        assert_eq!(
            pairs,
            vec![
                ("name".to_owned(), "hello world".to_owned()),
                ("count".to_owned(), "3".to_owned())
            ]
        );
    }

    #[test]
    fn rejects_unterminated_and_bad_escape() {
        assert!(tokenize_pairs("name=\"oops").is_err());
        assert!(tokenize_pairs("name=\"bad\\n\"").is_err());
        assert!(tokenize_pairs("name").is_err());
    }

    #[test]
    fn requires_separator_after_quoted_value() {
        assert!(tokenize_pairs("sha256=\"abc\"size=1").is_err());
        assert!(tokenize_pairs("sha256=\"abc\"\tsize=1").is_ok());
        assert!(tokenize_pairs("sha256=\"abc\" size=1").is_ok());
    }

    #[test]
    fn placeholder_detection_is_narrow_and_explicit() {
        assert!(is_filled("example"));
        assert!(is_filled("unknown"));
        assert!(is_filled("placeholder"));
        assert!(!is_filled(""));
        assert!(!is_filled("unfilled"));
        assert!(!is_filled("<unfilled>"));
        assert!(!is_filled("TBD"));
    }

    #[test]
    fn unknown_records_and_fields_are_errors() {
        let parsed =
            parse_lock("schema=cc-silicon.torture/lock/v1\nstatus=draft\nwidget foo=bar\n");
        assert!(parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "lock.record.unknown"));

        let parsed =
            parse_lock("schema=cc-silicon.torture/lock/v1\nstatus=draft\nrelease bogus=\"x\"\n");
        assert!(parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "lock.field.unknown"));
    }
}
