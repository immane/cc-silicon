use cc_silicon_compiler::codec::sha256;
use cc_silicon_compiler::contract::{
    compute_contract_hash, contract_version_file, FrozenSchema, CONTRACT_HASH, CONTRACT_VERSION,
    NORMATIVE_RULES,
};

const VERSION_FILE: &str = include_str!("../contracts/CONTRACT_VERSION");

fn field(name: &str) -> &str {
    VERSION_FILE
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.trim() == name).then(|| value.trim())
        })
        .unwrap_or_else(|| panic!("CONTRACT_VERSION is missing `{name}`"))
}

#[test]
fn frozen_hash_matches_recompute() {
    assert_eq!(compute_contract_hash(), CONTRACT_HASH);
}

#[test]
fn contract_version_file_matches_constants() {
    assert_eq!(VERSION_FILE, contract_version_file());
    assert_eq!(field("version"), CONTRACT_VERSION);
    assert_eq!(field("hash"), CONTRACT_HASH);
    assert_eq!(CONTRACT_VERSION, "t01-c01-c06/5");
}

#[test]
fn target_and_probe_policy_are_recorded() {
    assert_eq!(field("target"), "aarch64-unknown-linux-gnu");
    assert_eq!(field("target_verification"), "unverified");
    assert!(field("probe_substrate").contains("not provisioned"));
    assert!(field("corpus_fetch").contains("hash-locked"));
    let baseline = field("reference_baseline");
    assert!(baseline.contains("authorized"));
    assert!(baseline.contains("not available"));
    assert!(baseline.contains("not candidate evidence"));
    assert!(field("hash_scope").contains("not source code"));
    assert!(field("hash_excludes").contains("runtime-registrations"));
    assert!(field("hash_excludes").contains("group-declared-store-fields"));
    assert!(field("verified_state").contains("not authenticity"));
    assert!(field("verified_state").contains("physical provenance"));
}

#[test]
fn normative_rules_are_identifiers_and_change_the_hash_space() {
    // The rule list is non-trivial and contains no duplicates.
    assert!(NORMATIVE_RULES.len() >= 30);
    let mut sorted = NORMATIVE_RULES.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        NORMATIVE_RULES.len(),
        "rule IDs must be unique"
    );
    for rule in [
        "commit.field-scoped-manifest",
        "commit.inner-task-binding",
        "commit.enqueue-parent-integrity",
        "commit.no-mutation-before-preflight",
        "tasktree.dangling-parent-rejected",
        "task.single-transition-per-batch",
        "commit.proposal-budget-lossless",
        "routing.propagate-mutation-after-read",
        "source.content-hash-computed-internally",
        "commit.enqueue-destination-authorized",
        "commit.result-consume-error-accurate",
        "manifest.config-write-rejected",
        "manifest.registration-validated",
        "probe.wchar-encoding-required",
        "probe.attestation.private-state-not-provenance",
        "config.structurally-immutable",
        "bootstrap.integration-only",
        "snapshot.wire-payloads-encoded",
        "snapshot.reserved-tombstones-visible",
        "limits.every-configured-bound-enforced",
    ] {
        assert!(NORMATIVE_RULES.contains(&rule), "missing rule `{rule}`");
    }
}

#[test]
fn fingerprint_changes_with_a_normative_input() {
    let base = FrozenSchema::current();
    let base_digest = sha256(&base.encode());

    let mut changed = FrozenSchema::current();
    changed.limits.max_records_total = changed.limits.max_records_total.saturating_add(1);
    assert_ne!(base_digest, sha256(&changed.encode()));

    let mut renamed = FrozenSchema::current();
    renamed.version = "t01-c01-c06/test";
    assert_ne!(base_digest, sha256(&renamed.encode()));
}
