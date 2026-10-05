//! Offline verification of a corpus lock against an explicitly supplied tree.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::diagnostic::{escape_control, Diagnostic, Severity};
use crate::digest::sha256_and_sha512_reader;
use crate::lock::{self, is_filled, AssetFields, LockStatus, ParsedLock, Spanned};
use crate::path::validate_relative_path;

/// Sensible default upper bound on the number of directory entries visited by
/// `--check-unlisted`. Exceeding it is an explicit error, never a silent partial
/// result.
pub const DEFAULT_MAX_TRAVERSAL_ENTRIES: usize = 1_000_000;

#[derive(Clone, Debug)]
pub struct VerifyOptions {
    /// Report files present in the corpus tree but absent from the lock.
    /// Off by default because fetch-on-demand roots may hold staging artifacts.
    pub check_unlisted: bool,
    /// Upper bound on entries visited while checking tree completeness. A bound
    /// of 0 disables the limit (not recommended for untrusted trees).
    pub max_traversal_entries: usize,
}

impl Default for VerifyOptions {
    fn default() -> Self {
        Self {
            check_unlisted: false,
            max_traversal_entries: DEFAULT_MAX_TRAVERSAL_ENTRIES,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Report {
    pub lock_label: String,
    pub root_label: String,
    pub schema: Option<String>,
    pub status: Option<LockStatus>,
    pub assets_listed: usize,
    pub assets_verified: usize,
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    pub fn error_count(&self) -> usize {
        self.count(Severity::Error)
    }

    pub fn warning_count(&self) -> usize {
        self.count(Severity::Warning)
    }

    pub fn info_count(&self) -> usize {
        self.count(Severity::Info)
    }

    fn count(&self, severity: Severity) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == severity)
            .count()
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    }

    /// Exit code contract:
    /// - 0: frozen lock verified (no errors)
    /// - 1: verification failed (errors present)
    /// - 3: valid but not frozen; H00 completion must not be claimed
    pub fn exit_code(&self) -> i32 {
        if self.has_errors() {
            1
        } else {
            match self.status {
                Some(LockStatus::Frozen) => 0,
                Some(LockStatus::Draft) | None => 3,
            }
        }
    }

    pub fn verdict(&self) -> &'static str {
        if self.has_errors() {
            "FAILED"
        } else {
            match self.status {
                Some(LockStatus::Frozen) => "VERIFIED (frozen lock)",
                Some(LockStatus::Draft) => {
                    "NOT VERIFIED (draft/unfrozen lock; H00 completion cannot be claimed)"
                }
                None => "FAILED (lock status is unknown)",
            }
        }
    }

    /// Deterministic, timestamp-free report text. All dynamic labels, paths, and
    /// messages are control-character escaped so untrusted input cannot forge
    /// diagnostic lines.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("torture corpus lock verification\n");
        out.push_str(&format!("lock: {}\n", escape_control(&self.lock_label)));
        out.push_str(&format!("root: {}\n", escape_control(&self.root_label)));
        out.push_str(&format!(
            "schema: {}\n",
            escape_control(self.schema.as_deref().unwrap_or("(missing)"))
        ));
        out.push_str(&format!(
            "status: {}\n",
            self.status.map(LockStatus::as_str).unwrap_or("(missing)")
        ));
        out.push_str(&format!(
            "assets: listed={} verified={}\n",
            self.assets_listed, self.assets_verified
        ));
        out.push_str(
            "note: official release facts are recorded metadata and are not network-verified; only local assets are opened and hashed\n",
        );
        out.push('\n');
        if self.diagnostics.is_empty() {
            out.push_str("diagnostics: none\n");
        } else {
            for diagnostic in &self.diagnostics {
                out.push_str(&format!(
                    "[{}] {} {}: {}\n",
                    diagnostic.severity,
                    diagnostic.code,
                    escape_control(&diagnostic.location),
                    escape_control(&diagnostic.message)
                ));
            }
        }
        out.push('\n');
        out.push_str(&format!(
            "summary: errors={} warnings={} info={}\n",
            self.error_count(),
            self.warning_count(),
            self.info_count()
        ));
        out.push_str(&format!("verdict: {}\n", self.verdict()));
        out
    }

    fn finalize(&mut self) {
        Diagnostic::sort(&mut self.diagnostics);
    }
}

/// Verify lock text against a corpus root without touching the network.
pub fn verify_lock_text(
    lock_text: &str,
    lock_label: &str,
    root: &Path,
    options: &VerifyOptions,
) -> Report {
    let parsed = lock::parse_lock(lock_text);
    let (status, structural) = lock::validate(&parsed);

    let mut diagnostics = parsed.diagnostics.clone();
    diagnostics.extend(structural);

    let schema = parsed.schema.as_ref().map(|spanned| spanned.value.clone());
    let assets_listed = count_listed_assets(&parsed);
    let mut assets_verified = 0usize;

    if let Some(canonical_root) = resolve_root(root, &mut diagnostics) {
        let mut verifier = AssetVerifier::new(
            root,
            canonical_root,
            options.max_traversal_entries,
            &mut diagnostics,
            &mut assets_verified,
        );
        for asset in &parsed.assets {
            verifier.verify_asset(asset);
        }
        if options.check_unlisted {
            verifier.verify_tree_completeness(&parsed);
        }
    }

    let mut report = Report {
        lock_label: lock_label.to_owned(),
        root_label: root.display().to_string(),
        schema,
        status,
        assets_listed,
        assets_verified,
        diagnostics,
    };
    report.finalize();
    report
}

/// Read and verify a lock file. IO failures are returned to the caller so the
/// CLI can distinguish operational errors from verification failures.
pub fn verify_lock_file(
    lock_path: &Path,
    root: &Path,
    options: &VerifyOptions,
) -> io::Result<Report> {
    let text = fs::read_to_string(lock_path)?;
    Ok(verify_lock_text(
        &text,
        &lock_path.display().to_string(),
        root,
        options,
    ))
}

fn count_listed_assets(parsed: &ParsedLock) -> usize {
    let mut paths = BTreeSet::new();
    if let Some(spanned) = &parsed.release.source_archive {
        if is_filled(&spanned.value) {
            paths.insert(spanned.value.clone());
        }
    }
    for asset in &parsed.assets {
        if let Some(spanned) = &asset.path {
            if is_filled(&spanned.value) {
                paths.insert(spanned.value.clone());
            }
        }
    }
    paths.len()
}

/// Resolve and validate the corpus root. Returns the canonical root only when it
/// exists, is a directory, and can be canonicalized; failures produce exactly one
/// root diagnostic so a bad root does not cascade into per-asset escape errors.
fn resolve_root(root: &Path, diagnostics: &mut Vec<Diagnostic>) -> Option<PathBuf> {
    match fs::metadata(root) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            diagnostics.push(Diagnostic::error(
                "root.missing",
                "root",
                format!("corpus root does not exist: {}", root.display()),
            ));
            return None;
        }
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                "root.unreadable",
                "root",
                format!("cannot inspect corpus root {}: {error}", root.display()),
            ));
            return None;
        }
        Ok(metadata) if !metadata.is_dir() => {
            diagnostics.push(Diagnostic::error(
                "root.not-dir",
                "root",
                format!("corpus root is not a directory: {}", root.display()),
            ));
            return None;
        }
        Ok(_) => {}
    }

    match root.canonicalize() {
        Ok(path) => Some(path),
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                "root.unresolved",
                "root",
                format!(
                    "cannot canonicalize corpus root {}: {error}",
                    root.display()
                ),
            ));
            None
        }
    }
}

struct AssetVerifier<'a> {
    root: &'a Path,
    canonical_root: PathBuf,
    max_traversal_entries: usize,
    diagnostics: &'a mut Vec<Diagnostic>,
    verified: &'a mut usize,
    /// Canonical files already verified. A second asset resolving to the same
    /// file (e.g. through an in-root symlink alias) is a double-count error.
    seen_canonical: BTreeSet<PathBuf>,
}

impl<'a> AssetVerifier<'a> {
    fn new(
        root: &'a Path,
        canonical_root: PathBuf,
        max_traversal_entries: usize,
        diagnostics: &'a mut Vec<Diagnostic>,
        verified: &'a mut usize,
    ) -> Self {
        Self {
            root,
            canonical_root,
            max_traversal_entries,
            diagnostics,
            verified,
            seen_canonical: BTreeSet::new(),
        }
    }

    fn verify_asset(&mut self, asset: &AssetFields) {
        let Some(path) = asset.path.as_ref() else {
            return;
        };
        if !is_filled(&path.value) || validate_relative_path(&path.value).is_err() {
            return;
        }
        let location = format!("asset:{}", path.value);
        self.verify_path(
            &path.value,
            asset.sha256.as_ref(),
            asset.sha512.as_ref(),
            asset.size.as_ref(),
            &location,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn verify_path(
        &mut self,
        relative: &str,
        expected_digest: Option<&Spanned>,
        expected_sha512: Option<&Spanned>,
        expected_size: Option<&Spanned>,
        location: &str,
    ) {
        let candidate = self.root.join(relative);
        if let Err(error) = fs::symlink_metadata(&candidate) {
            self.report_io_failure(&candidate, location, &error);
            return;
        }

        let canonical = match candidate.canonicalize() {
            Ok(path) => path,
            Err(error) => {
                self.report_io_failure(&candidate, location, &error);
                return;
            }
        };
        if !canonical.starts_with(&self.canonical_root) {
            self.diagnostics.push(Diagnostic::error(
                "asset.symlink-escape",
                location,
                format!(
                    "path resolves outside the corpus root: {} -> {}",
                    candidate.display(),
                    canonical.display()
                ),
            ));
            return;
        }
        if !self.seen_canonical.insert(canonical.clone()) {
            self.diagnostics.push(Diagnostic::error(
                "asset.canonical-duplicate",
                location,
                format!(
                    "path resolves to a file already verified by another asset: {}",
                    canonical.display()
                ),
            ));
            return;
        }

        // Open once and hash the opened handle: path resolution happens only for
        // the boundary check, and the bytes compared are the bytes of this file.
        let file = match fs::File::open(&canonical) {
            Ok(file) => file,
            Err(error) => {
                self.report_io_failure(&canonical, location, &error);
                return;
            }
        };
        let metadata = match file.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                self.report_io_failure(&canonical, location, &error);
                return;
            }
        };
        if !metadata.is_file() {
            self.diagnostics.push(Diagnostic::error(
                "asset.not-regular-file",
                location,
                format!("path is not a regular file: {}", canonical.display()),
            ));
            return;
        }

        if let Some(size) = expected_size {
            if let Some(expected) = parse_size(size) {
                if metadata.len() != expected {
                    self.diagnostics.push(Diagnostic::error(
                        "asset.size-mismatch",
                        location,
                        format!(
                            "expected {} bytes, found {} bytes",
                            expected,
                            metadata.len()
                        ),
                    ));
                }
            }
        }

        let want_sha256 = expected_digest.is_some_and(|spanned| is_filled(&spanned.value));
        let want_sha512 = expected_sha512.is_some_and(|spanned| is_filled(&spanned.value));
        if !want_sha256 && !want_sha512 {
            return;
        }
        match sha256_and_sha512_reader(&file, want_sha256, want_sha512) {
            Ok((actual256, actual512)) => {
                let mut matched = true;
                if let (Some(actual), Some(expected)) = (actual256.as_deref(), expected_digest) {
                    if actual != expected.value {
                        matched = false;
                        self.diagnostics.push(Diagnostic::error(
                            "asset.digest-mismatch",
                            location,
                            format!("expected sha256 {}, found {}", expected.value, actual),
                        ));
                    }
                }
                if let (Some(actual), Some(expected)) = (actual512.as_deref(), expected_sha512) {
                    if actual != expected.value {
                        matched = false;
                        self.diagnostics.push(Diagnostic::error(
                            "asset.sha512-mismatch",
                            location,
                            format!("expected sha512 {}, found {}", expected.value, actual),
                        ));
                    }
                }
                if matched {
                    *self.verified += 1;
                }
            }
            Err(error) => self.report_io_failure(&canonical, location, &error),
        }
    }

    fn report_io_failure(&mut self, path: &Path, location: &str, error: &io::Error) {
        if error.kind() == io::ErrorKind::NotFound {
            self.diagnostics.push(Diagnostic::error(
                "asset.missing",
                location,
                format!(
                    "listed file is missing from the corpus tree: {}",
                    path.display()
                ),
            ));
        } else {
            self.diagnostics.push(Diagnostic::error(
                "asset.unreadable",
                location,
                format!("cannot read {}: {error}", path.display()),
            ));
        }
    }

    fn verify_tree_completeness(&mut self, parsed: &ParsedLock) {
        let mut listed = BTreeSet::new();
        if let Some(spanned) = &parsed.release.source_archive {
            if is_filled(&spanned.value) {
                listed.insert(spanned.value.clone());
            }
        }
        for asset in &parsed.assets {
            if let Some(spanned) = &asset.path {
                if is_filled(&spanned.value) {
                    listed.insert(spanned.value.clone());
                }
            }
        }

        let enumeration = collect_files(self.root, self.max_traversal_entries);
        for relative in &enumeration.files {
            if !listed.contains(relative) {
                self.diagnostics.push(Diagnostic::error(
                    "tree.unlisted",
                    format!("asset:{relative}"),
                    "file exists in the corpus tree but is not listed in the lock",
                ));
            }
        }
        // Enumeration failures are surfaced rather than swallowed: a partial
        // listing must never be reported as a clean completeness check.
        for (path, error) in &enumeration.failures {
            self.diagnostics.push(Diagnostic::error(
                "tree.enumeration-incomplete",
                "root",
                format!(
                    "could not enumerate {}: {error}; unlisted-file detection is incomplete",
                    path.display()
                ),
            ));
        }
        if enumeration.limit_exceeded {
            self.diagnostics.push(Diagnostic::error(
                "tree.traversal-limit",
                "root",
                format!(
                    "corpus tree traversal exceeded the configured limit of {} entries; unlisted-file detection is incomplete",
                    self.max_traversal_entries
                ),
            ));
        }
    }
}

fn parse_size(spanned: &Spanned) -> Option<u64> {
    if !is_filled(&spanned.value) {
        return None;
    }
    spanned.value.parse::<u64>().ok()
}

struct Enumeration {
    files: Vec<String>,
    failures: Vec<(PathBuf, io::Error)>,
    limit_exceeded: bool,
}

/// Iteratively collect regular-file relative paths with `/` separators. Symlinks
/// are skipped to avoid cycles; escaping symlinks are already rejected during
/// asset verification. Every read_dir/entry/metadata failure is recorded so the
/// caller can fail closed instead of silently skipping an incomplete subtree. A
/// non-zero `max_entries` bounds the number of entries visited so a hostile or
/// runaway tree cannot make the walk unbounded; exceeding it stops the walk and
/// is reported rather than treated as a clean result.
fn collect_files(root: &Path, max_entries: usize) -> Enumeration {
    let mut files = Vec::new();
    let mut failures = Vec::new();
    let mut limit_exceeded = false;
    let mut visited = 0usize;
    let mut stack = vec![root.to_path_buf()];

    'walk: while let Some(dir) = stack.pop() {
        if max_entries != 0 {
            visited += 1;
            if visited > max_entries {
                limit_exceeded = true;
                break;
            }
        }
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) => {
                failures.push((dir, error));
                continue;
            }
        };
        let mut entries = match entries.collect::<Result<Vec<_>, io::Error>>() {
            Ok(entries) => entries,
            Err(error) => {
                failures.push((dir, error));
                continue;
            }
        };
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            if max_entries != 0 {
                visited += 1;
                if visited > max_entries {
                    limit_exceeded = true;
                    break 'walk;
                }
            }
            let path = entry.path();
            match fs::symlink_metadata(&path) {
                Err(error) => failures.push((path, error)),
                Ok(metadata) if metadata.file_type().is_symlink() => {}
                Ok(metadata) if metadata.is_dir() => stack.push(path),
                Ok(metadata) if metadata.is_file() => match path.strip_prefix(root) {
                    Ok(relative) => files.push(relative.to_string_lossy().replace('\\', "/")),
                    Err(_) => failures.push((
                        path,
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "enumerated path is not under the corpus root",
                        ),
                    )),
                },
                Ok(_) => {}
            }
        }
    }

    files.sort();
    failures.sort_by(|a, b| a.0.cmp(&b.0));
    Enumeration {
        files,
        failures,
        limit_exceeded,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn missing_dir(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "cc-silicon-torture-{tag}-{}-missing",
            std::process::id()
        ))
    }

    #[test]
    fn collect_files_reports_missing_root() {
        let missing = missing_dir("collect");
        let _ = fs::remove_dir_all(&missing);
        let enumeration = collect_files(&missing, DEFAULT_MAX_TRAVERSAL_ENTRIES);
        assert!(enumeration.files.is_empty());
        assert!(!enumeration.failures.is_empty());
        assert!(!enumeration.limit_exceeded);
    }

    #[test]
    fn collect_files_honors_the_entry_limit() {
        let root =
            std::env::temp_dir().join(format!("cc-silicon-torture-limit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create root");
        for index in 0..5 {
            fs::write(root.join(format!("file-{index}.c")), b"x").expect("write file");
        }
        let enumeration = collect_files(&root, 2);
        assert!(enumeration.limit_exceeded);
        let _ = fs::remove_dir_all(&root);
    }
}
