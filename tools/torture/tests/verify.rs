mod common;

use std::fs;

use cc_silicon_torture::verify::VerifyOptions;

use common::{
    archive_asset_line, archived_lock, archived_lock_with, asset_line, codes, complete_lock,
    lock_with, verify, verify_with, TempTree, ARCHIVE_NAME, COMMIT_SHA, RELEASE_DATE,
    RELEASE_PROVENANCE, SHA512_ABC, SHA512_EMPTY, SHA_ABC, SHA_EMPTY, SUBSTRATE_FILLED,
    SUBSTRATE_UNFILLED,
};

const DRIVER: &str = "gcc/testsuite/lib/c-torture.exp";

fn frozen_tree(tag: &str) -> TempTree {
    let tree = TempTree::new(tag);
    tree.write(ARCHIVE_NAME, "abc");
    tree.write(DRIVER, "abc");
    tree
}

#[test]
fn accepts_frozen_lock_with_known_hashes() {
    let tree = frozen_tree("pass");
    let lock = archived_lock("frozen", &asset_line(DRIVER, SHA_ABC));
    let report = verify(&lock, &tree.root);

    assert_eq!(report.error_count(), 0, "{}", report.render());
    assert_eq!(report.exit_code(), 0);
    // The release archive and its cross-linked asset are the same file: listed
    // and verified exactly once.
    assert_eq!(report.assets_listed, 2);
    assert_eq!(report.assets_verified, 2);
    assert!(
        report.verdict().starts_with("VERIFIED"),
        "{}",
        report.verdict()
    );
}

#[test]
fn detects_altered_file() {
    let tree = TempTree::new("altered");
    tree.write(ARCHIVE_NAME, "abc");
    tree.write(DRIVER, "abd");
    let lock = archived_lock("frozen", &asset_line(DRIVER, SHA_ABC));
    let report = verify(&lock, &tree.root);

    assert!(codes(&report).contains(&"asset.digest-mismatch"));
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn detects_missing_file() {
    let tree = TempTree::new("missing");
    tree.write(ARCHIVE_NAME, "abc");
    let lock = archived_lock("frozen", &asset_line(DRIVER, SHA_ABC));
    let report = verify(&lock, &tree.root);

    assert!(codes(&report).contains(&"asset.missing"));
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn detects_malformed_commit_sha() {
    let tree = TempTree::new("bad-commit");
    tree.write(ARCHIVE_NAME, "abc");
    let lock = format!(
        "{}{}",
        complete_lock("frozen").replace(COMMIT_SHA, "not-a-valid-sha"),
        archive_asset_line()
    );
    let report = verify(&lock, &tree.root);

    assert!(codes(&report).contains(&"release.commit-sha.invalid"));
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn detects_malformed_digest() {
    let tree = frozen_tree("bad-digest");
    let lock = archived_lock("frozen", &asset_line(DRIVER, "deadbeef"));
    let report = verify(&lock, &tree.root);

    assert!(codes(&report).contains(&"digest.invalid"));
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn detects_duplicate_paths() {
    let tree = frozen_tree("duplicate");
    let lock = format!(
        "{}{}{}{}",
        complete_lock("frozen"),
        archive_asset_line(),
        asset_line(DRIVER, SHA_ABC),
        asset_line(DRIVER, SHA_ABC)
    );
    let report = verify(&lock, &tree.root);

    assert!(codes(&report).contains(&"path.duplicate"));
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn rejects_traversal_and_absolute_paths() {
    let tree = TempTree::new("unsafe");
    tree.write(ARCHIVE_NAME, "abc");

    let traversal = format!(
        "{}{}",
        complete_lock("frozen"),
        asset_line("../escape.c", SHA_ABC)
    );
    let report = verify(&traversal, &tree.root);
    assert!(
        codes(&report).contains(&"path.traversal"),
        "{}",
        report.render()
    );

    let absolute = format!(
        "{}{}",
        complete_lock("frozen"),
        asset_line("/etc/passwd", SHA_ABC)
    );
    let report = verify(&absolute, &tree.root);
    assert!(
        codes(&report).contains(&"path.absolute"),
        "{}",
        report.render()
    );
}

#[cfg(unix)]
#[test]
fn rejects_symlink_escaping_the_root() {
    use std::os::unix::fs::symlink;

    let tree = TempTree::new("symlink-escape");
    tree.write(ARCHIVE_NAME, "abc");
    let outside = TempTree::new("symlink-outside");
    outside.write("secret.c", "abc");
    symlink(outside.path("secret.c"), tree.path("escape.c")).expect("create symlink");

    let lock = archived_lock("frozen", &asset_line("escape.c", SHA_ABC));
    let report = verify(&lock, &tree.root);

    assert!(
        codes(&report).contains(&"asset.symlink-escape"),
        "{}",
        report.render()
    );
    assert_eq!(report.exit_code(), 1);
}

#[cfg(unix)]
#[test]
fn accepts_symlink_that_stays_within_the_root() {
    use std::os::unix::fs::symlink;

    let tree = TempTree::new("symlink-inside");
    tree.write(ARCHIVE_NAME, "abc");
    tree.write("real.c", "abc");
    symlink(tree.path("real.c"), tree.path("link.c")).expect("create symlink");

    let lock = archived_lock("frozen", &asset_line("link.c", SHA_ABC));
    let report = verify(&lock, &tree.root);

    assert_eq!(report.error_count(), 0, "{}", report.render());
    assert_eq!(report.exit_code(), 0);
}

#[test]
fn detects_license_and_provenance_omissions() {
    let tree = frozen_tree("license");
    let no_license = format!(
        "{}{}asset role=\"driver\" path=\"{DRIVER}\" sha256=\"{SHA_ABC}\" provenance=\"gcc-mirror\"\n",
        complete_lock("frozen"),
        archive_asset_line()
    );
    let report = verify(&no_license, &tree.root);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "field.missing" && d.message.contains("asset.license")),
        "{}",
        report.render()
    );

    let no_provenance = format!(
        "{}{}asset role=\"driver\" path=\"{DRIVER}\" sha256=\"{SHA_ABC}\" license=\"GPL-3.0-or-later\"\n",
        complete_lock("frozen"),
        archive_asset_line()
    );
    let report = verify(&no_provenance, &tree.root);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "field.missing" && d.message.contains("asset.provenance")),
        "{}",
        report.render()
    );
}

#[test]
fn frozen_lock_with_unfilled_required_field_is_rejected() {
    let tree = TempTree::new("unfilled");
    tree.write(ARCHIVE_NAME, "abc");
    let lock = archived_lock_with("frozen", SUBSTRATE_UNFILLED, "");
    let report = verify(&lock, &tree.root);

    let unfilled = report
        .diagnostics
        .iter()
        .filter(|d| d.code == "field.unfilled")
        .count();
    assert!(unfilled >= 5, "{}", report.render());
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn frozen_lock_with_unresolved_commit_sha_is_rejected() {
    let tree = TempTree::new("unfilled-sha");
    tree.write(ARCHIVE_NAME, "abc");
    let lock = format!(
        "{}{}",
        complete_lock("frozen").replace(
            &format!("commit-sha=\"{COMMIT_SHA}\""),
            "commit-sha=\"unfilled\""
        ),
        archive_asset_line()
    );
    let report = verify(&lock, &tree.root);

    assert!(codes(&report).contains(&"field.unfilled"));
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn draft_lock_is_not_reported_as_verified() {
    let tree = frozen_tree("draft");
    let lock = archived_lock_with("draft", SUBSTRATE_UNFILLED, &asset_line(DRIVER, SHA_ABC));
    let report = verify(&lock, &tree.root);

    assert_eq!(report.error_count(), 0, "{}", report.render());
    assert_eq!(report.exit_code(), 3);
    assert!(report.verdict().contains("draft"), "{}", report.verdict());
    assert!(report.verdict().contains("cannot be claimed"));
}

#[test]
fn empty_frozen_lock_is_rejected() {
    let tree = TempTree::new("empty-frozen");
    tree.write(ARCHIVE_NAME, "abc");
    let report = verify(&complete_lock("frozen"), &tree.root);

    assert!(codes(&report).contains(&"asset.empty"));
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn empty_draft_lock_makes_no_false_claim() {
    let tree = TempTree::new("empty-draft");
    tree.write(ARCHIVE_NAME, "abc");
    let report = verify(&complete_lock("draft"), &tree.root);

    assert_eq!(report.error_count(), 0, "{}", report.render());
    assert_eq!(report.exit_code(), 3);
    assert!(report.verdict().contains("draft"));
}

#[test]
fn rejects_reference_baseline_used_for_candidate_pass_rate() {
    let tree = TempTree::new("reference");
    tree.write(ARCHIVE_NAME, "abc");
    let lock = complete_lock("frozen").replace(
        "used-for-candidate-pass-rate=\"false\"",
        "used-for-candidate-pass-rate=\"true\"",
    );
    let report = verify(&lock, &tree.root);

    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "value.unsupported" && d.message.contains("candidate pass-rate")),
        "{}",
        report.render()
    );
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn rejects_target_identity_not_matching_the_frozen_target() {
    let tree = TempTree::new("target");
    tree.write(ARCHIVE_NAME, "abc");
    let lock =
        complete_lock("frozen").replace("aarch64-unknown-linux-gnu", "x86_64-unknown-linux-gnu");
    let report = verify(&lock, &tree.root);

    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "value.unsupported"
                && d.message.contains("aarch64-unknown-linux-gnu")),
        "{}",
        report.render()
    );
}

#[test]
fn rejects_unknown_record_and_field() {
    let tree = TempTree::new("unknown");
    tree.write(ARCHIVE_NAME, "abc");

    let unknown_record = format!("{}widget foo=bar\n", complete_lock("frozen"));
    let report = verify(&unknown_record, &tree.root);
    assert!(codes(&report).contains(&"lock.record.unknown"));

    let unknown_field = format!("{}release bogus=\"x\"\n", complete_lock("frozen"));
    let report = verify(&unknown_field, &tree.root);
    assert!(codes(&report).contains(&"lock.field.unknown"));
}

#[test]
fn detects_size_mismatch() {
    let tree = frozen_tree("size");
    let lock = format!(
        "{}{}asset role=\"driver\" path=\"{DRIVER}\" sha256=\"{SHA_ABC}\" license=\"GPL-3.0-or-later\" provenance=\"gcc-mirror/gcc@{COMMIT_SHA}\" suite=\"compile\" size=1\n",
        complete_lock("frozen"),
        archive_asset_line()
    );
    let report = verify(&lock, &tree.root);

    assert!(
        codes(&report).contains(&"asset.size-mismatch"),
        "{}",
        report.render()
    );
}

#[test]
fn reports_unlisted_tree_files_when_requested() {
    let tree = frozen_tree("unlisted");
    tree.write("extra/unreferenced.c", "abc");
    let lock = archived_lock("frozen", &asset_line(DRIVER, SHA_ABC));

    let quiet = verify(&lock, &tree.root);
    assert!(!codes(&quiet).contains(&"tree.unlisted"));

    let options = VerifyOptions {
        check_unlisted: true,
        ..VerifyOptions::default()
    };
    let strict = verify_with(&lock, &tree.root, &options);
    assert!(
        codes(&strict).contains(&"tree.unlisted"),
        "{}",
        strict.render()
    );
}

#[test]
fn report_is_deterministic_and_order_independent() {
    let tree = TempTree::new("determinism");
    tree.write(ARCHIVE_NAME, "abc");
    let first = format!(
        "{}{}{}{}",
        complete_lock("frozen"),
        archive_asset_line(),
        asset_line("missing/first.c", SHA_ABC),
        asset_line("missing/second.c", SHA_ABC)
    );
    let second = format!(
        "{}{}{}{}",
        complete_lock("frozen"),
        archive_asset_line(),
        asset_line("missing/second.c", SHA_ABC),
        asset_line("missing/first.c", SHA_ABC)
    );

    let report_a = verify(&first, &tree.root);
    let report_b = verify(&first, &tree.root);
    let report_c = verify(&second, &tree.root);

    assert_eq!(report_a.render(), report_b.render());
    assert_eq!(report_a.diagnostics, report_c.diagnostics);
    assert_eq!(report_a.render(), report_c.render());
}

#[test]
fn missing_corpus_root_yields_a_single_root_error_not_an_asset_flood() {
    let tree = TempTree::new("root");
    let missing = tree.path("does-not-exist");
    let lock = archived_lock("frozen", &asset_line(DRIVER, SHA_ABC));
    let report = verify(&lock, &missing);

    assert!(codes(&report).contains(&"root.missing"));
    let root_errors = report
        .diagnostics
        .iter()
        .filter(|d| d.location == "root")
        .count();
    assert_eq!(root_errors, 1, "{}", report.render());
    assert!(!codes(&report).contains(&"asset.symlink-escape"));
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn frozen_lock_requires_archive_asset_record() {
    let tree = frozen_tree("archive-missing");
    let lock = format!("{}{}", complete_lock("frozen"), asset_line(DRIVER, SHA_ABC));
    let report = verify(&lock, &tree.root);

    assert!(
        codes(&report).contains(&"archive.asset.missing"),
        "{}",
        report.render()
    );
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn archive_asset_digest_must_match_release() {
    let tree = frozen_tree("archive-digest");
    let archive = format!(
        "asset role=\"archive\" path=\"{ARCHIVE_NAME}\" sha256=\"{SHA_EMPTY}\" license=\"GPL-3.0-or-later\" provenance=\"gcc-mirror/gcc@{COMMIT_SHA}\"\n"
    );
    let lock = format!(
        "{}{}{}",
        complete_lock("frozen"),
        archive,
        asset_line(DRIVER, SHA_ABC)
    );
    let report = verify(&lock, &tree.root);

    assert!(
        codes(&report).contains(&"archive.digest-mismatch"),
        "{}",
        report.render()
    );
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn archive_asset_path_must_match_release() {
    let tree = frozen_tree("archive-path");
    let archive = format!(
        "asset role=\"archive\" path=\"elsewhere.tar.xz\" sha256=\"{SHA_ABC}\" license=\"GPL-3.0-or-later\" provenance=\"gcc-mirror/gcc@{COMMIT_SHA}\"\n"
    );
    let lock = format!(
        "{}{}{}",
        complete_lock("frozen"),
        archive,
        asset_line(DRIVER, SHA_ABC)
    );
    let report = verify(&lock, &tree.root);

    assert!(
        codes(&report).contains(&"archive.path-mismatch"),
        "{}",
        report.render()
    );
}

#[test]
fn non_archive_asset_cannot_claim_the_release_archive_path() {
    let tree = frozen_tree("archive-collision");
    let lock = format!(
        "{}{}",
        complete_lock("frozen"),
        asset_line(ARCHIVE_NAME, SHA_ABC)
    );
    let report = verify(&lock, &tree.root);

    assert!(
        codes(&report).contains(&"archive.path-collision"),
        "{}",
        report.render()
    );
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn archive_asset_participates_in_duplicate_path_detection() {
    let tree = frozen_tree("archive-duplicate");
    let lock = format!(
        "{}{}{}",
        complete_lock("frozen"),
        archive_asset_line(),
        asset_line(ARCHIVE_NAME, SHA_ABC)
    );
    let report = verify(&lock, &tree.root);

    assert!(
        codes(&report).contains(&"path.duplicate"),
        "{}",
        report.render()
    );
}

#[test]
fn archive_asset_requires_license_and_provenance() {
    let tree = frozen_tree("archive-license");
    let archive = format!(
        "asset role=\"archive\" path=\"{ARCHIVE_NAME}\" sha256=\"{SHA_ABC}\" provenance=\"gcc-mirror\"\n"
    );
    let lock = format!(
        "{}{}{}",
        complete_lock("frozen"),
        archive,
        asset_line(DRIVER, SHA_ABC)
    );
    let report = verify(&lock, &tree.root);

    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "field.missing" && d.message.contains("asset.license")),
        "{}",
        report.render()
    );
}

#[test]
fn multiple_archive_assets_are_rejected() {
    let tree = frozen_tree("archive-multiple");
    let lock = format!(
        "{}{}{}{}",
        complete_lock("frozen"),
        archive_asset_line(),
        archive_asset_line(),
        asset_line(DRIVER, SHA_ABC)
    );
    let report = verify(&lock, &tree.root);

    assert!(
        codes(&report).contains(&"archive.asset.multiple"),
        "{}",
        report.render()
    );
}

#[test]
fn frozen_lock_requires_all_baseline_suites() {
    let tree = frozen_tree("suites");
    let lock = format!(
        "{}{}{}",
        complete_lock("frozen").replace(" suite=\"ieee\"", ""),
        archive_asset_line(),
        asset_line(DRIVER, SHA_ABC)
    );
    let report = verify(&lock, &tree.root);

    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "profile.suite.missing" && d.message.contains("ieee")),
        "{}",
        report.render()
    );
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn draft_lock_missing_suite_is_a_warning_not_an_error() {
    let tree = frozen_tree("suites-draft");
    let lock = format!(
        "{}{}{}",
        lock_with("draft", SUBSTRATE_FILLED).replace(" suite=\"ieee\"", ""),
        archive_asset_line(),
        asset_line(DRIVER, SHA_ABC)
    );
    let report = verify(&lock, &tree.root);

    assert!(codes(&report).contains(&"profile.suite.missing"));
    assert_eq!(report.error_count(), 0, "{}", report.render());
    assert_eq!(report.exit_code(), 3);
}

#[test]
fn ordinary_words_are_not_treated_as_placeholders() {
    let tree = frozen_tree("ordinary");
    let substrate = "substrate execution=\"linux-ci-or-vm\" runner=\"example\" toolchain=\"unknown\" sysroot=\"/opt/example\" assembler=\"gas\" linker=\"ld\"";
    let lock = archived_lock_with("draft", substrate, &asset_line(DRIVER, SHA_ABC));
    let report = verify(&lock, &tree.root);

    assert!(
        !codes(&report).contains(&"field.unfilled"),
        "{}",
        report.render()
    );
    assert_eq!(report.error_count(), 0, "{}", report.render());
}

#[test]
fn control_characters_are_escaped_in_the_report() {
    let tree = TempTree::new("control");
    tree.write(ARCHIVE_NAME, "abc");
    let lock = format!(
        "{}{}asset role=\"driver\" path=\"a\u{7}b.c\" sha256=\"{SHA_ABC}\" license=\"GPL-3.0-or-later\" provenance=\"gcc-mirror/gcc@{COMMIT_SHA}\"\n",
        complete_lock("draft"),
        archive_asset_line()
    );
    let report = verify(&lock, &tree.root);
    let rendered = report.render();

    assert!(!rendered.contains('\u{7}'), "{rendered}");
    assert!(rendered.contains("a\\x07b.c"), "{rendered}");
}

#[cfg(unix)]
#[test]
fn incomplete_enumeration_is_reported_not_silently_ignored() {
    use std::os::unix::fs::PermissionsExt;

    let tree = frozen_tree("enumeration");
    let locked = tree.path("locked");
    fs::create_dir_all(&locked).expect("create locked dir");
    fs::write(locked.join("hidden.c"), "abc").expect("write hidden file");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).expect("chmod");

    // Running as root bypasses directory permissions; skip the strict assertion
    // in that case rather than producing a false failure.
    let unreadable = fs::read_dir(&locked).is_err();

    let lock = archived_lock("frozen", &asset_line(DRIVER, SHA_ABC));
    let options = VerifyOptions {
        check_unlisted: true,
        ..VerifyOptions::default()
    };
    let report = verify_with(&lock, &tree.root, &options);

    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).ok();

    if unreadable {
        assert!(
            codes(&report).contains(&"tree.enumeration-incomplete"),
            "{}",
            report.render()
        );
        assert_eq!(report.exit_code(), 1);
    }
}

#[test]
fn frozen_lock_requires_release_provenance() {
    let tree = frozen_tree("release-provenance");
    let lock = format!(
        "{}{}{}",
        complete_lock("frozen").replace(&format!(" provenance=\"{RELEASE_PROVENANCE}\""), ""),
        archive_asset_line(),
        asset_line(DRIVER, SHA_ABC)
    );
    let report = verify(&lock, &tree.root);

    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "field.missing" && d.message.contains("release.provenance")),
        "{}",
        report.render()
    );
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn release_date_is_optional() {
    let tree = frozen_tree("release-date");
    let lock = format!(
        "{}{}{}",
        complete_lock("frozen").replace(&format!(" release-date=\"{RELEASE_DATE}\""), ""),
        archive_asset_line(),
        asset_line(DRIVER, SHA_ABC)
    );
    let report = verify(&lock, &tree.root);

    assert_eq!(report.error_count(), 0, "{}", report.render());
}

#[test]
fn published_sha512_is_verified_against_the_local_archive() {
    let tree = frozen_tree("sha512-local");
    // Released and recorded sha512 are the (wrong) empty-string value, but the
    // local archive bytes hash to the "abc" vector, so the local check fails.
    let lock = format!(
        "{}{}{}",
        complete_lock("frozen"),
        archive_asset_line().replace(SHA512_ABC, SHA512_EMPTY),
        asset_line(DRIVER, SHA_ABC)
    );
    let report = verify(&lock, &tree.root);

    assert!(
        codes(&report).contains(&"asset.sha512-mismatch"),
        "{}",
        report.render()
    );
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn archive_sha512_must_match_the_release_record() {
    let tree = frozen_tree("sha512-cross");
    let lock = format!(
        "{}{}{}",
        complete_lock("frozen"),
        archive_asset_line().replace(SHA512_ABC, SHA512_EMPTY),
        asset_line(DRIVER, SHA_ABC)
    );
    let report = verify(&lock, &tree.root);

    assert!(
        codes(&report).contains(&"archive.sha512-mismatch"),
        "{}",
        report.render()
    );
}

#[test]
fn archive_sha512_is_required_when_the_release_publishes_one() {
    let tree = frozen_tree("sha512-missing");
    let archive = format!(
        "asset role=\"archive\" path=\"{ARCHIVE_NAME}\" sha256=\"{SHA_ABC}\" license=\"GPL-3.0-or-later\" provenance=\"gcc-mirror/gcc@{COMMIT_SHA}\"\n"
    );
    let lock = format!(
        "{}{}{}",
        complete_lock("frozen"),
        archive,
        asset_line(DRIVER, SHA_ABC)
    );
    let report = verify(&lock, &tree.root);

    assert!(
        codes(&report).contains(&"archive.sha512-missing"),
        "{}",
        report.render()
    );
}

#[test]
fn malformed_published_sha512_is_rejected() {
    let tree = frozen_tree("sha512-malformed");
    let lock = format!(
        "{}{}{}",
        complete_lock("frozen").replace(SHA512_ABC, "not-a-sha512"),
        archive_asset_line().replace(SHA512_ABC, SHA512_EMPTY),
        asset_line(DRIVER, SHA_ABC)
    );
    let report = verify(&lock, &tree.root);

    assert!(
        codes(&report).contains(&"digest.invalid"),
        "{}",
        report.render()
    );
}

#[test]
fn detached_signature_asset_is_supported() {
    let tree = frozen_tree("signature");
    tree.write("gcc-15.2.0.tar.xz.sig", "abc");
    let signature = format!(
        "asset role=\"signature\" path=\"gcc-15.2.0.tar.xz.sig\" sha256=\"{SHA_ABC}\" license=\"GPL-3.0-or-later\" provenance=\"gcc-mirror/gcc@{COMMIT_SHA}\"\n"
    );
    let lock = format!(
        "{}{}{}{}",
        complete_lock("frozen"),
        archive_asset_line(),
        asset_line(DRIVER, SHA_ABC),
        signature
    );
    let report = verify(&lock, &tree.root);

    assert_eq!(report.error_count(), 0, "{}", report.render());
    assert_eq!(report.assets_verified, 3, "{}", report.render());
}

#[test]
fn report_distinguishes_official_facts_from_local_assets() {
    let tree = frozen_tree("official");
    let lock = archived_lock("frozen", &asset_line(DRIVER, SHA_ABC));
    let report = verify(&lock, &tree.root);
    let rendered = report.render();

    assert!(rendered.contains("not network-verified"), "{rendered}");
    assert!(
        rendered.contains("only local assets are opened and hashed"),
        "{rendered}"
    );
}

const NO_MATRIX: &str = " configuration=\"-O0\" configuration=\"-O1\"";

#[test]
fn draft_lock_without_option_matrix_warns() {
    let tree = frozen_tree("no-matrix-draft");
    let lock = format!(
        "{}{}{}",
        complete_lock("draft").replace(NO_MATRIX, ""),
        archive_asset_line(),
        asset_line(DRIVER, SHA_ABC)
    );
    let report = verify(&lock, &tree.root);

    assert!(
        codes(&report).contains(&"profile.configuration.missing"),
        "{}",
        report.render()
    );
    assert_eq!(report.error_count(), 0, "{}", report.render());
    assert_eq!(report.exit_code(), 3);
}

#[test]
fn frozen_lock_without_option_matrix_is_rejected() {
    let tree = frozen_tree("no-matrix-frozen");
    let lock = format!(
        "{}{}{}",
        complete_lock("frozen").replace(NO_MATRIX, ""),
        archive_asset_line(),
        asset_line(DRIVER, SHA_ABC)
    );
    let report = verify(&lock, &tree.root);

    assert!(
        codes(&report).contains(&"profile.configuration.missing"),
        "{}",
        report.render()
    );
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn bare_known_records_are_malformed() {
    let tree = frozen_tree("bare-records");
    for kind in [
        "release",
        "profile",
        "target",
        "substrate",
        "policy",
        "reference",
        "asset",
    ] {
        let lock = format!("{}{}\n", complete_lock("draft"), kind);
        let report = verify(&lock, &tree.root);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code == "lock.record.empty" && d.message.contains(kind)),
            "record '{kind}' was not rejected: {}",
            report.render()
        );
    }
}

#[test]
fn unknown_bare_record_is_still_unknown() {
    let tree = frozen_tree("bare-unknown");
    let lock = format!("{}widget\n", complete_lock("draft"));
    let report = verify(&lock, &tree.root);

    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "lock.record.unknown" && d.message.contains("widget")),
        "{}",
        report.render()
    );
}

#[test]
fn frozen_lock_requires_published_sha512() {
    let tree = frozen_tree("no-published-sha512");
    let lock = format!(
        "{}{}{}",
        complete_lock("frozen").replace(&format!(" source-archive-sha512=\"{SHA512_ABC}\""), ""),
        archive_asset_line(),
        asset_line(DRIVER, SHA_ABC)
    );
    let report = verify(&lock, &tree.root);

    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "field.missing"
                && d.message.contains("release.source-archive-sha512")),
        "{}",
        report.render()
    );
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn frozen_supplied_optional_markers_are_rejected() {
    let tree = frozen_tree("optional-markers");
    let driver = format!(
        "asset role=\"driver\" path=\"{DRIVER}\" sha256=\"{SHA_ABC}\" sha512=\"unfilled\" license=\"GPL-3.0-or-later\" provenance=\"gcc-mirror/gcc@{COMMIT_SHA}\" size=\"unfilled\"\n"
    );
    let lock = format!(
        "{}{}{}",
        complete_lock("frozen"),
        archive_asset_line(),
        driver
    );
    let report = verify(&lock, &tree.root);

    let unfilled = report
        .diagnostics
        .iter()
        .filter(|d| d.code == "field.unfilled")
        .count();
    assert!(unfilled >= 2, "{}", report.render());
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn unicode_line_separators_cannot_forge_report_lines() {
    let tree = TempTree::new("u2028");
    tree.write(ARCHIVE_NAME, "abc");
    let lock = format!(
        "{}{}{}",
        complete_lock("draft"),
        archive_asset_line(),
        asset_line("a\u{2028}b.c", SHA_ABC)
    );
    let report = verify(&lock, &tree.root);
    let rendered = report.render();

    assert!(!rendered.contains('\u{2028}'), "{rendered}");
    assert!(!rendered.contains('\u{2029}'), "{rendered}");
    assert!(rendered.contains("\\u{2028}"), "{rendered}");
}

#[test]
fn traversal_limit_is_reported_when_requested() {
    let tree = frozen_tree("traversal-limit");
    let lock = archived_lock("frozen", &asset_line(DRIVER, SHA_ABC));
    let options = VerifyOptions {
        check_unlisted: true,
        max_traversal_entries: 1,
    };
    let report = verify_with(&lock, &tree.root, &options);

    assert!(
        codes(&report).contains(&"tree.traversal-limit"),
        "{}",
        report.render()
    );
    assert_eq!(report.exit_code(), 1);
}

#[cfg(unix)]
#[test]
fn canonical_symlink_alias_is_rejected() {
    use std::os::unix::fs::symlink;

    let tree = TempTree::new("canonical-alias");
    tree.write(ARCHIVE_NAME, "abc");
    tree.write("real.c", "abc");
    symlink(tree.path("real.c"), tree.path("link.c")).expect("create symlink");

    let lock = archived_lock(
        "frozen",
        &format!(
            "{}{}",
            asset_line("real.c", SHA_ABC),
            asset_line("link.c", SHA_ABC)
        ),
    );
    let report = verify(&lock, &tree.root);

    assert!(
        codes(&report).contains(&"asset.canonical-duplicate"),
        "{}",
        report.render()
    );
    assert_eq!(report.exit_code(), 1);
}
