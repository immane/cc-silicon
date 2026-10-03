mod common;

use std::process::Command;

use common::{
    archived_lock, archived_lock_with, asset_line, TempTree, ARCHIVE_NAME, SHA_ABC,
    SUBSTRATE_UNFILLED,
};

const BIN: &str = env!("CARGO_BIN_EXE_cc-silicon-torture");
const DRIVER: &str = "gcc/testsuite/lib/c-torture.exp";

fn run(args: &[&str]) -> std::process::Output {
    Command::new(BIN)
        .args(args)
        .output()
        .expect("run torture verifier")
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn usage_error_exits_two() {
    let output = run(&[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("expected <LOCK>"));
}

#[test]
fn help_exits_zero() {
    let output = run(&["--help"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).contains("usage:"));
}

#[test]
fn verifies_frozen_lock() {
    let tree = TempTree::new("cli-pass");
    tree.write(ARCHIVE_NAME, "abc");
    tree.write(DRIVER, "abc");
    let lock = archived_lock("frozen", &asset_line(DRIVER, SHA_ABC));
    tree.write("lock.torture", &lock);

    let output = run(&[
        tree.path("lock.torture").to_str().unwrap(),
        tree.root.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    assert!(stdout(&output).contains("verdict: VERIFIED (frozen lock)"));
}

#[test]
fn draft_lock_exits_three() {
    let tree = TempTree::new("cli-draft");
    tree.write(ARCHIVE_NAME, "abc");
    tree.write(DRIVER, "abc");
    let lock = archived_lock_with("draft", SUBSTRATE_UNFILLED, &asset_line(DRIVER, SHA_ABC));
    tree.write("lock.torture", &lock);

    let output = run(&[
        tree.path("lock.torture").to_str().unwrap(),
        tree.root.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(3), "{}", stdout(&output));
    assert!(stdout(&output).contains("draft/unfrozen"));
}

#[test]
fn failed_verification_exits_one() {
    let tree = TempTree::new("cli-fail");
    tree.write(ARCHIVE_NAME, "abc");
    tree.write(DRIVER, "tampered");
    let lock = archived_lock("frozen", &asset_line(DRIVER, SHA_ABC));
    tree.write("lock.torture", &lock);

    let output = run(&[
        tree.path("lock.torture").to_str().unwrap(),
        tree.root.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    assert!(stdout(&output).contains("asset.digest-mismatch"));
}

#[test]
fn invalid_max_unlisted_entries_exits_two() {
    let output = run(&["--max-unlisted-entries", "not-a-number", "lock", "root"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("non-negative integer"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_arguments_do_not_panic() {
    use std::os::unix::ffi::OsStrExt;

    let bad_lock = std::ffi::OsStr::from_bytes(b"\xff\xfe.lock");
    let output = Command::new(BIN)
        .arg(bad_lock)
        .arg(".")
        .output()
        .expect("run torture verifier");

    // A non-UTF-8 path is an operational error (2), never a panic (101).
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
