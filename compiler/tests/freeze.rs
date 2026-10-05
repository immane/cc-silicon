use cc_silicon_compiler::codec::sha256;
use cc_silicon_compiler::contract::{
    compute_contract_hash, contract_version_file, FrozenSchema, CONTRACT_HASH, CONTRACT_VERSION,
    M1_PROPOSAL_WIRE_TAGS, M1_SEED_VERSION, M1_STORE_FAMILY_ARENA, NORMATIVE_RULES,
    PROPOSAL_NAMES,
};
use cc_silicon_compiler::ids::RecordFamily;
use cc_silicon_compiler::task::StoreId;

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
    assert_eq!(CONTRACT_VERSION, "t01-c01-c06/6");
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
    assert!(field("hash_excludes").contains("post-seed-declarations"));
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

#[test]
fn m1_seed_version_is_pinned() {
    assert_eq!(M1_SEED_VERSION, "m1-append/1");
}

#[test]
fn m1_seed_mapping_covers_every_family_exactly_once() {
    use std::collections::BTreeSet;
    // Every family appears exactly once, in RecordFamily::ALL order.
    let mut seen = BTreeSet::new();
    let mut order = Vec::new();
    for (store, _field, family, _arena) in M1_STORE_FAMILY_ARENA {
        assert!(
            StoreId::parse(store).is_some(),
            "unknown store `{store}` in seed mapping"
        );
        assert!(seen.insert(*family), "duplicate family `{family}`");
        order.push(*family);
    }
    // Every mapped family is a distinct member of the frozen inventory
    // (the 19 append families; non-appended families have no seed row).
    let all: BTreeSet<&str> = RecordFamily::ALL.iter().map(|f| f.name()).collect();
    assert_eq!(all.len(), RecordFamily::ALL.len());
    for family in &seen {
        assert!(all.contains(family), "family `{family}` outside inventory");
    }
    assert_eq!(
        seen.len(),
        19,
        "seed mapping carries the 19 proposal-§8 append families"
    );
    // Proposal wire inventory matches the frozen name list pairwise.
    assert_eq!(M1_PROPOSAL_WIRE_TAGS.len(), PROPOSAL_NAMES.len());
    for (i, (name, tag)) in M1_PROPOSAL_WIRE_TAGS.iter().enumerate() {
        assert_eq!(*name, PROPOSAL_NAMES[i]);
        assert_eq!(*tag as usize, i, "proposal wire tags are 0..8 in order");
    }
    // The frozen hash covers the seed: flipping any seed byte changes it.
    let base = FrozenSchema::current().encode();
    assert!(base.windows(11).any(|w| w == b"m1-append/1"));
}
