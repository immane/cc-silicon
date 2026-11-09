//! Sprint H07: cancellation, stdin EOF, and concurrent workdir isolation.
//!
//! These are finite host-supervision probes for the existing H05 runner API.
//! Each child is a short-lived `/bin/sh` command with no intentionally
//! descendant-held pipes, so every test is bounded by the runner timeout and
//! an outer test-suite deadline. They do not claim kernel sandboxing.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

use cc_silicon_runner::{run, CommandSpec, ResourceLimits, RunOutcome, RunSpec};

static NEXT_WORKDIR: AtomicU64 = AtomicU64::new(0);

fn unique_workdir(label: &str) -> PathBuf {
    let serial = NEXT_WORKDIR.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock moves forward")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "h07-runner-{}-{}-{}-{}",
        std::process::id(),
        label,
        nanos,
        serial
    ));
    std::fs::create_dir_all(&dir).expect("create scratch workdir");
    dir
}

fn shell_spec(dir: PathBuf, timeout_ms: u64, script: &str) -> RunSpec {
    RunSpec {
        workdir: dir,
        timeout: Duration::from_millis(timeout_ms),
        stdout_limit: 4096,
        stderr_limit: 4096,
        cancel_file: None,
        limits: ResourceLimits::default(),
        command: CommandSpec {
            program: OsString::from("sh"),
            args: vec![OsString::from("-c"), OsString::from(script)],
        },
    }
}

#[test]
fn cancellation_file_before_start_terminates_run() {
    let dir = unique_workdir("cancel-before-start");
    let cancel = dir.join("cancel");
    std::fs::write(&cancel, b"cancel").expect("write cancellation marker");
    let mut run_spec = shell_spec(dir, 8_000, "exec sleep 5");
    run_spec.cancel_file = Some(cancel.clone());

    let started = Instant::now();
    let report = run(&run_spec).expect("runner");

    assert_eq!(report.outcome, RunOutcome::Cancelled);
    assert_eq!(report.outcome.label(), "cancelled");
    assert_eq!(report.outcome.exit_code(), 4);
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "pre-existing marker was not honored: {:?}",
        started.elapsed()
    );
    assert_eq!(
        std::fs::read(&cancel).expect("marker remains"),
        b"cancel".to_vec()
    );
}

#[test]
fn cancellation_file_during_run_terminates_child() {
    let dir = unique_workdir("cancel-during");
    let cancel = dir.join("cancel");
    let mut run_spec = shell_spec(dir, 8_000, "exec sleep 5");
    run_spec.cancel_file = Some(cancel.clone());

    let writer = {
        let cancel = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            std::fs::write(cancel, b"cancel").expect("write cancellation marker");
        })
    };

    let started = Instant::now();
    let report = run(&run_spec).expect("runner");
    writer.join().expect("cancel writer thread");

    assert_eq!(report.outcome, RunOutcome::Cancelled);
    assert_eq!(report.outcome.exit_code(), 4);
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "cancellation did not stop the child promptly: {:?}",
        started.elapsed()
    );
}

#[test]
fn child_stdin_is_eof_and_does_not_block() {
    let dir = unique_workdir("stdin-eof");
    // `cat` exits only if stdin reaches EOF; an inherited open stdin would
    // either echo unexpected bytes or block until the runner timeout.
    let report = run(&shell_spec(dir, 2_000, "cat; printf 'stdin-eof'")).expect("runner");

    assert_eq!(report.outcome, RunOutcome::Completed { code: 0 });
    assert_eq!(report.outcome.exit_code(), 0);
    assert_eq!(report.stdout, b"stdin-eof".to_vec());
    assert!(report.stderr.is_empty());
}

#[test]
fn concurrent_runs_keep_separate_workdirs_and_outputs() {
    let dir_a = unique_workdir("parallel-a");
    let dir_b = unique_workdir("parallel-b");
    let canonical_a = dir_a.canonicalize().expect("canonical workdir a");
    let canonical_b = dir_b.canonicalize().expect("canonical workdir b");
    assert_ne!(
        canonical_a, canonical_b,
        "concurrent instances must not share a workdir"
    );

    let barrier = Arc::new(Barrier::new(2));
    let spec_a = shell_spec(
        dir_a.clone(),
        5_000,
        "printf alpha > marker.txt; sleep 0.2; cat marker.txt; printf '|%s' \"$(pwd -P)\"",
    );
    let spec_b = shell_spec(
        dir_b.clone(),
        5_000,
        "printf beta > marker.txt; sleep 0.2; cat marker.txt; printf '|%s' \"$(pwd -P)\"",
    );

    let handle_a = {
        let barrier = Arc::clone(&barrier);
        std::thread::spawn(move || {
            barrier.wait();
            run(&spec_a)
        })
    };
    let handle_b = {
        let barrier = Arc::clone(&barrier);
        std::thread::spawn(move || {
            barrier.wait();
            run(&spec_b)
        })
    };

    let report_a = handle_a.join().expect("thread a").expect("runner a");
    let report_b = handle_b.join().expect("thread b").expect("runner b");

    assert_eq!(report_a.outcome, RunOutcome::Completed { code: 0 });
    assert_eq!(report_b.outcome, RunOutcome::Completed { code: 0 });
    assert_eq!(report_a.workdir, canonical_a);
    assert_eq!(report_b.workdir, canonical_b);
    assert_eq!(
        String::from_utf8(report_a.stdout.clone()).expect("utf8 stdout a"),
        format!("alpha|{}", canonical_a.display())
    );
    assert_eq!(
        String::from_utf8(report_b.stdout.clone()).expect("utf8 stdout b"),
        format!("beta|{}", canonical_b.display())
    );
    assert_eq!(
        std::fs::read(dir_a.join("marker.txt")).expect("marker a"),
        b"alpha".to_vec()
    );
    assert_eq!(
        std::fs::read(dir_b.join("marker.txt")).expect("marker b"),
        b"beta".to_vec()
    );
    assert_eq!(
        std::fs::read(dir_a.join("stdout.log")).expect("log a"),
        report_a.stdout
    );
    assert_eq!(
        std::fs::read(dir_b.join("stdout.log")).expect("log b"),
        report_b.stdout
    );
}
