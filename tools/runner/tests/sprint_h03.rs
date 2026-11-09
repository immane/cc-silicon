//! Sprint H03: output-cap boundary regressions for the H05 runner.
//!
//! Each test owns a unique temporary workdir and uses only `sh` builtins, so
//! no unrelated host files are read or written. The fast-writer cases let the
//! child exit before the supervisor polls the cap: exceeding a cap must still
//! be classified as `output-limit`, never as an ordinary completion.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cc_silicon_runner::{
    run, CommandSpec, ResourceLimits, RunOutcome, RunReport, RunSpec, StreamName,
};

fn workdir(label: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock moves forward")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "cc-silicon-sprint-h03-{}-{label}-{nonce}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("create sprint H03 workdir");
    dir
}

fn spec(dir: &Path, stdout_limit: usize, stderr_limit: usize, script: &str) -> RunSpec {
    RunSpec {
        workdir: dir.to_path_buf(),
        timeout: Duration::from_millis(2000),
        stdout_limit,
        stderr_limit,
        cancel_file: None,
        limits: ResourceLimits::default(),
        command: CommandSpec {
            program: OsString::from("sh"),
            args: vec![OsString::from("-c"), OsString::from(script)],
        },
    }
}

fn run_script(dir: &Path, stdout_limit: usize, stderr_limit: usize, script: &str) -> RunReport {
    run(&spec(dir, stdout_limit, stderr_limit, script)).expect("runner should not fail setup")
}

#[test]
fn fast_stdout_over_cap_is_output_limit_not_completion() {
    let dir = workdir("fast-stdout-over-cap");
    let payload = "a".repeat(512);
    let report = run_script(&dir, 64, 64, &format!("printf '%s' '{payload}'"));

    assert_eq!(
        report.outcome,
        RunOutcome::OutputLimit {
            stream: StreamName::Stdout
        }
    );
    assert_eq!(report.outcome.exit_code(), 5);
    assert_eq!(report.stdout, vec![b'a'; 64]);
    assert!(report.stdout_truncated);
    assert!(report.stderr.is_empty());
    assert!(!report.stderr_truncated);
    assert_eq!(
        std::fs::read(dir.join("stdout.log")).expect("stdout log"),
        report.stdout
    );
}

#[test]
fn fast_stderr_over_cap_is_output_limit_not_completion() {
    let dir = workdir("fast-stderr-over-cap");
    let payload = "e".repeat(512);
    let report = run_script(&dir, 64, 64, &format!("printf '%s' '{payload}' >&2"));

    assert_eq!(
        report.outcome,
        RunOutcome::OutputLimit {
            stream: StreamName::Stderr
        }
    );
    assert_eq!(report.outcome.exit_code(), 5);
    assert!(report.stdout.is_empty());
    assert!(!report.stdout_truncated);
    assert_eq!(report.stderr, vec![b'e'; 64]);
    assert!(report.stderr_truncated);
    assert_eq!(
        std::fs::read(dir.join("stderr.log")).expect("stderr log"),
        report.stderr
    );
}

#[test]
fn exact_cap_is_not_truncated() {
    let dir = workdir("exact-cap");
    let stdout_payload = "b".repeat(64);
    let stderr_payload = "c".repeat(64);
    let script = format!("printf '%s' '{stdout_payload}'; printf '%s' '{stderr_payload}' >&2");
    let report = run_script(&dir, 64, 64, &script);

    assert_eq!(report.outcome, RunOutcome::Completed { code: 0 });
    assert_eq!(report.outcome.exit_code(), 0);
    assert_eq!(report.stdout, stdout_payload.as_bytes());
    assert_eq!(report.stderr, stderr_payload.as_bytes());
    assert_eq!(report.stdout.len(), 64);
    assert_eq!(report.stderr.len(), 64);
    assert!(!report.stdout_truncated);
    assert!(!report.stderr_truncated);
    assert_eq!(
        std::fs::read(dir.join("stdout.log")).expect("stdout log"),
        report.stdout
    );
    assert_eq!(
        std::fs::read(dir.join("stderr.log")).expect("stderr log"),
        report.stderr
    );
}

#[test]
fn stderr_over_cap_reports_stderr_and_preserves_stdout() {
    let dir = workdir("stderr-over-cap");
    let report = run_script(&dir, 32, 32, "printf out; while :; do printf e >&2; done");

    assert_eq!(
        report.outcome,
        RunOutcome::OutputLimit {
            stream: StreamName::Stderr
        }
    );
    assert_eq!(report.outcome.exit_code(), 5);
    assert_eq!(report.stdout, b"out");
    assert!(!report.stdout_truncated);
    assert_eq!(report.stderr, vec![b'e'; 32]);
    assert!(report.stderr_truncated);
    assert_eq!(
        std::fs::read(dir.join("stdout.log")).expect("stdout log"),
        report.stdout
    );
    assert_eq!(
        std::fs::read(dir.join("stderr.log")).expect("stderr log"),
        report.stderr
    );
}

#[test]
fn both_streams_over_cap_attribute_one_stream_and_keep_outputs_separate() {
    let dir = workdir("both-streams-over-cap");
    let report = run_script(&dir, 32, 32, "while :; do printf o; printf e >&2; done");

    let stream = match &report.outcome {
        RunOutcome::OutputLimit { stream } => *stream,
        other => panic!("expected output-limit, got {other:?}"),
    };
    assert_eq!(report.outcome.exit_code(), 5);
    assert!(report.stdout.len() <= 32);
    assert!(report.stderr.len() <= 32);
    assert!(report.stdout.iter().all(|byte| *byte == b'o'));
    assert!(report.stderr.iter().all(|byte| *byte == b'e'));
    match stream {
        StreamName::Stdout => {
            assert_eq!(report.stdout.len(), 32);
            assert!(report.stdout_truncated);
        }
        StreamName::Stderr => {
            assert_eq!(report.stderr.len(), 32);
            assert!(report.stderr_truncated);
        }
    }
    assert!(report.stdout_truncated || report.stderr_truncated);
    assert_eq!(
        std::fs::read(dir.join("stdout.log")).expect("stdout log"),
        report.stdout
    );
    assert_eq!(
        std::fs::read(dir.join("stderr.log")).expect("stderr log"),
        report.stderr
    );
}
