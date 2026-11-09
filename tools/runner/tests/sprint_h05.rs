//! Independent H05 supervision tests for external-tool resolution and limits.
//!
//! `setsid` and `prlimit` are external programs, so these tests place fake
//! versions on `PATH`. That makes missing, failing, and working cases
//! deterministic without depending on the host's Linux tooling. `PATH` is
//! process-global, so every test holds `PATH_LOCK` for its duration.
//!
//! Contract under test: a requested rlimit must either be applied or fail the
//! run as a setup error; it must never be silently omitted. A supervision
//! helper that cannot run must never be reported as a child outcome.

#![cfg(unix)]

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use cc_silicon_runner::{run, CommandSpec, ResourceLimits, RunOutcome, RunSpec};

static PATH_LOCK: Mutex<()> = Mutex::new(());

const PASS_SETSID: &str = r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  exit 0
fi
if [ "$1" = "--" ]; then
  shift
fi
exec "$@"
"#;

const PASS_PRLIMIT: &str = r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  exit 0
fi
printf '%s\n' "$*"
while [ "$#" -gt 0 ] && [ "$1" != "--" ]; do
  shift
done
if [ "$#" -gt 0 ]; then
  shift
fi
"$@"
"#;

const BROKEN_TOOL: &str = r#"#!/bin/sh
exit 9
"#;

const LIMIT_FLAGS: [&str; 4] = ["--as=123456789", "--cpu=7", "--nofile=41", "--nproc=13"];

const MARKER_SCRIPT: &str = r#"printf child-ran; printf child-ran > "$TMPDIR/ran""#;

fn workdir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "h05-sprint-{}-{}-{}",
        std::process::id(),
        test,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock moves forward")
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn scratch(test: &str) -> (PathBuf, PathBuf) {
    let root = workdir(test);
    let bin = root.join("bin");
    fs::create_dir_all(&bin).expect("fake tool directory");
    (root, bin)
}

fn write_tool(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    fs::write(&path, body).expect("write fake tool");
    let mut permissions = fs::metadata(&path)
        .expect("fake tool metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make fake tool executable");
}

struct PathRestore {
    original: Option<OsString>,
}

impl Drop for PathRestore {
    fn drop(&mut self) {
        match self.original.take() {
            Some(path) => std::env::set_var("PATH", path),
            None => std::env::remove_var("PATH"),
        }
    }
}

fn fake_path(bin: &Path) -> (MutexGuard<'static, ()>, PathRestore) {
    let lock = PATH_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let original = std::env::var_os("PATH");
    std::env::set_var("PATH", bin);
    (lock, PathRestore { original })
}

fn spec(dir: PathBuf, script: &str) -> RunSpec {
    RunSpec {
        workdir: dir,
        timeout: Duration::from_millis(2_000),
        stdout_limit: 4_096,
        stderr_limit: 4_096,
        cancel_file: None,
        limits: ResourceLimits::default(),
        command: CommandSpec {
            program: OsString::from("/bin/sh"),
            args: vec![OsString::from("-c"), OsString::from(script)],
        },
    }
}

fn all_requested_limits() -> ResourceLimits {
    ResourceLimits {
        address_space: Some(123_456_789),
        cpu_seconds: Some(7),
        nofile: Some(41),
        nproc: Some(13),
    }
}

fn assert_limit_flags(text: &str, context: &str) {
    for flag in LIMIT_FLAGS {
        assert!(
            text.contains(flag),
            "{context}: requested limit {flag} was silently dropped; captured output: {text:?}"
        );
    }
}

#[test]
fn missing_prlimit_is_a_setup_error_and_does_not_run_the_command() {
    let (root, bin) = scratch("missing-prlimit");
    write_tool(&bin, "setsid", PASS_SETSID);
    let (_lock, _restore) = fake_path(&bin);
    let mut run_spec = spec(root.clone(), r#"printf ran > "$TMPDIR/ran""#);
    run_spec.limits = all_requested_limits();

    let error = run(&run_spec).expect_err("requested limits without prlimit must fail closed");
    assert!(
        !root.join("tmp").join("ran").exists(),
        "run returned a setup error but still executed the command: {error}"
    );
}

#[test]
fn missing_setsid_does_not_silently_drop_requested_limits() {
    let (root, bin) = scratch("missing-setsid-limits");
    write_tool(&bin, "prlimit", PASS_PRLIMIT);
    let (_lock, _restore) = fake_path(&bin);
    let mut run_spec = spec(root.clone(), MARKER_SCRIPT);
    run_spec.limits = all_requested_limits();

    match run(&run_spec) {
        Ok(report) => {
            assert_eq!(
                report.outcome,
                RunOutcome::Completed { code: 0 },
                "unexpected outcome with prlimit available: {report:?}"
            );
            assert!(
                root.join("tmp").join("ran").exists(),
                "runner reported success without executing the command"
            );
            let text = String::from_utf8_lossy(&report.stdout);
            assert_limit_flags(&text, "missing setsid");
        }
        Err(error) => {
            // Failing closed is acceptable; running unbounded is not.
            assert!(
                !root.join("tmp").join("ran").exists(),
                "runner failed after executing the command unbounded: {error}"
            );
        }
    }
}

#[test]
fn failing_setsid_is_not_reported_as_a_child_exit() {
    let (root, bin) = scratch("failing-setsid");
    write_tool(&bin, "setsid", BROKEN_TOOL);
    let (_lock, _restore) = fake_path(&bin);
    let run_spec = spec(root.clone(), MARKER_SCRIPT);

    match run(&run_spec) {
        Ok(report) => {
            assert_eq!(
                report.outcome,
                RunOutcome::Completed { code: 0 },
                "failed setsid probe became a child outcome: {report:?}"
            );
            assert!(
                String::from_utf8_lossy(&report.stdout).contains("child-ran"),
                "successful outcome did not execute the requested command"
            );
            assert!(
                root.join("tmp").join("ran").exists(),
                "successful outcome did not execute the requested command"
            );
        }
        Err(error) => {
            // Treating an unusable helper as a setup failure is also fail-closed.
            assert!(
                !root.join("tmp").join("ran").exists(),
                "runner reported a setup error after executing the command: {error}"
            );
        }
    }
}

#[test]
fn failing_prlimit_is_a_setup_error_and_does_not_run_the_command() {
    let (root, bin) = scratch("failing-prlimit");
    write_tool(&bin, "setsid", PASS_SETSID);
    write_tool(&bin, "prlimit", BROKEN_TOOL);
    let (_lock, _restore) = fake_path(&bin);
    let mut run_spec = spec(root.clone(), r#"printf ran > "$TMPDIR/ran""#);
    run_spec.limits = all_requested_limits();

    let error = run(&run_spec).expect_err("failing prlimit must fail closed");
    assert!(
        !root.join("tmp").join("ran").exists(),
        "failing prlimit still executed the command (setup error: {error})"
    );
}

#[test]
fn requested_limits_reach_prlimit_when_both_tools_work() {
    let (root, bin) = scratch("applied-limits");
    write_tool(&bin, "setsid", PASS_SETSID);
    write_tool(&bin, "prlimit", PASS_PRLIMIT);
    let (_lock, _restore) = fake_path(&bin);
    let mut run_spec = spec(root.clone(), MARKER_SCRIPT);
    run_spec.limits = all_requested_limits();

    let report = run(&run_spec).expect("working setsid and prlimit");
    assert_eq!(report.outcome, RunOutcome::Completed { code: 0 });
    assert!(
        root.join("tmp").join("ran").exists(),
        "runner did not execute the command"
    );
    let text = String::from_utf8_lossy(&report.stdout);
    assert_limit_flags(&text, "complete helper set");
}
