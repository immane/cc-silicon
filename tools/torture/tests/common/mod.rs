#![allow(dead_code)]
//! Shared, test-generated fixtures. These are synthetic files only; no GCC
//! corpus content or GPL material is copied into this repository.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use cc_silicon_torture::verify::{verify_lock_text, Report, VerifyOptions};

/// SHA-256 of the ASCII bytes `abc`, used as an independently known vector.
pub const SHA_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
/// SHA-256 of the empty string, used as a second known vector for mismatches.
pub const SHA_EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
/// SHA-512 of the ASCII bytes `abc`, used as an independently known vector.
pub const SHA512_ABC: &str = "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f";
/// SHA-512 of the empty string.
pub const SHA512_EMPTY: &str = "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e";

/// Official-source release provenance URL used by the synthetic fixtures.
pub const RELEASE_PROVENANCE: &str = "https://gcc.gnu.org/pub/gcc/releases/gcc-15.2.0/";
pub const RELEASE_DATE: &str = "2025-08-08";
pub const COMMIT_SHA: &str = "0123456789abcdef0123456789abcdef01234567";
pub const ARCHIVE_NAME: &str = "gcc-15.2.0.tar.xz";

pub const SUBSTRATE_FILLED: &str = "substrate execution=\"linux-ci-or-vm\" runner=\"linux-ci-runner\" toolchain=\"gcc-reference\" sysroot=\"linux-sysroot\" assembler=\"gas\" linker=\"ld\"";
pub const SUBSTRATE_UNFILLED: &str = "substrate execution=\"linux-ci-or-vm\" runner=\"unfilled\" toolchain=\"unfilled\" sysroot=\"unfilled\" assembler=\"unfilled\" linker=\"unfilled\"";

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct TempTree {
    pub root: PathBuf,
}

impl TempTree {
    pub fn new(tag: &str) -> Self {
        let unique = format!(
            "cc-silicon-torture-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let root = std::env::temp_dir().join(unique);
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp root");
        TempTree { root }
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    pub fn write(&self, relative: &str, contents: &str) {
        let path = self.path(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent directory");
        }
        fs::write(path, contents).expect("write fixture file");
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// A structurally complete lock. The represented policy is fetch-on-demand with
/// hash locking, a non-candidate reference-only baseline, and the frozen AArch64
/// LP64/AAPCS64 target identity.
pub fn lock_with(status: &str, substrate: &str) -> String {
    format!(
        "schema={SCHEMA}\n\
status={status}\n\
release gcc-release=\"releases/gcc-15.2.0\" release-date=\"{RELEASE_DATE}\" commit-sha=\"{COMMIT_SHA}\" source-archive=\"{ARCHIVE_NAME}\" source-archive-sha256=\"{SHA_ABC}\" source-archive-sha512=\"{SHA512_ABC}\" provenance=\"{RELEASE_PROVENANCE}\"\n\
profile name=\"linux-aarch64-lp64\" dialect=\"gnu17\" configuration=\"-O0\" configuration=\"-O1\" suite=\"compile\" suite=\"execute\" suite=\"ieee\"\n\
target triple=\"aarch64-unknown-linux-gnu\" abi=\"AAPCS64\" format=\"ELF\" data-model=\"LP64\" endianness=\"little\"\n\
{substrate}\n\
policy acquisition=\"fetch-on-demand\" vendoring=\"prohibited\" hash-lock=\"sha256\" license-distribution=\"GCC assets remain GPL and are not redistributed in the MIT tree\"\n\
reference label=\"non-candidate\" purpose=\"reference-only\" authorized=\"true\" used-for-candidate-pass-rate=\"false\"\n",
        SCHEMA = cc_silicon_torture::SCHEMA_V1,
    )
}

pub fn complete_lock(status: &str) -> String {
    lock_with(status, SUBSTRATE_FILLED)
}

pub fn asset_line(path: &str, digest: &str) -> String {
    format!(
        "asset role=\"driver\" path=\"{path}\" sha256=\"{digest}\" license=\"GPL-3.0-or-later\" provenance=\"gcc-mirror/gcc@{COMMIT_SHA}\" suite=\"compile\"\n"
    )
}

/// The canonical cross-linked source-archive asset. `release.source-archive` must
/// be accompanied by exactly this record so it carries license/provenance and is
/// subject to duplicate-path detection.
pub fn archive_asset_line() -> String {
    format!(
        "asset role=\"archive\" path=\"{ARCHIVE_NAME}\" sha256=\"{SHA_ABC}\" sha512=\"{SHA512_ABC}\" license=\"GPL-3.0-or-later\" provenance=\"gcc-mirror/gcc@{COMMIT_SHA}\"\n"
    )
}

pub fn archived_lock(status: &str, assets: &str) -> String {
    format!(
        "{}{}{}",
        complete_lock(status),
        archive_asset_line(),
        assets
    )
}

pub fn archived_lock_with(status: &str, substrate: &str, assets: &str) -> String {
    format!(
        "{}{}{}",
        lock_with(status, substrate),
        archive_asset_line(),
        assets
    )
}

pub fn verify(lock: &str, root: &Path) -> Report {
    verify_with(lock, root, &VerifyOptions::default())
}

pub fn verify_with(lock: &str, root: &Path, options: &VerifyOptions) -> Report {
    verify_lock_text(lock, "fixture.lock", root, options)
}

pub fn codes(report: &Report) -> Vec<&'static str> {
    report
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}
