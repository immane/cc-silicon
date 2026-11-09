// ============================================================================
// runner.rs — H05 isolated-runner acceptance tests.
//
// Exercises completion, timeout, output caps, cancellation, signal
// classification, workdir setup, and (Linux-only) prlimit bounds. The tests
// use `/bin/sh` as a controllable child; they do not invoke a compiler.
// ============================================================================

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use cc_silicon_runner::{run, CommandSpec, ResourceLimits, RunOutcome, RunSpec, SetupError};

fn workdir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "h05-runner-{}-{}-{}",
        std::process::id(),
        test,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock moves forward")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn spec(dir: PathBuf, timeout_ms: u64, script: &str) -> RunSpec {
    RunSpec {
        workdir: dir,
        timeout: Duration::from_millis(timeout_ms),
        stdout_limit: 1024,
        stderr_limit: 1024,
        cancel_file: None,
        limits: ResourceLimits::default(),
        command: CommandSpec {
            program: OsString::from("sh"),
            args: vec![OsString::from("-c"), OsString::from(script)],
        },
    }
}

#[test]
fn completes_with_code_and_logs() {
    let dir = workdir("complete");
    let report = run(&spec(
        dir.clone(),
        1000,
        "printf hello; printf error >&2; exit 7",
    ))
    .expect("runner");
    assert_eq!(report.outcome, RunOutcome::Completed { code: 7 });
    assert_eq!(report.outcome.exit_code(), 1);
    assert_eq!(report.stdout, b"hello");
    assert_eq!(report.stderr, b"error");
    assert_eq!(
        std::fs::read(dir.join("stdout.log")).unwrap(),
        b"hello".to_vec()
    );
    assert_eq!(
        std::fs::read(dir.join("stderr.log")).unwrap(),
        b"error".to_vec()
    );
}

#[test]
fn timeout_terminates_the_child() {
    let dir = workdir("timeout");
    let started = std::time::Instant::now();
    let report = run(&spec(dir, 50, "sleep 2")).expect("runner");
    assert_eq!(report.outcome, RunOutcome::Timeout);
    assert_eq!(report.outcome.exit_code(), 3);
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn output_cap_terminates_and_truncates() {
    let dir = workdir("output");
    let mut run_spec = spec(dir.clone(), 2000, "while true; do printf x; done");
    run_spec.stdout_limit = 32;
    let report = run(&run_spec).expect("runner");
    assert_eq!(
        report.outcome,
        RunOutcome::OutputLimit {
            stream: cc_silicon_runner::StreamName::Stdout
        }
    );
    assert_eq!(report.outcome.exit_code(), 5);
    assert_eq!(report.stdout.len(), 32);
    assert!(report.stdout_truncated);
}

#[test]
fn cancellation_file_terminates() {
    let dir = workdir("cancel");
    let cancel = dir.join("cancel");
    let mut run_spec = spec(dir, 5000, "sleep 2");
    run_spec.cancel_file = Some(cancel.clone());
    let handle = {
        let cancel = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            std::fs::write(cancel, b"cancel").expect("cancel marker");
        })
    };
    let report = run(&run_spec).expect("runner");
    handle.join().unwrap();
    assert_eq!(report.outcome, RunOutcome::Cancelled);
    assert_eq!(report.outcome.exit_code(), 4);
}

#[cfg(unix)]
#[test]
fn signal_is_classified() {
    let dir = workdir("signal");
    let report = run(&spec(dir, 1000, "kill -TERM $$")).expect("runner");
    match report.outcome {
        RunOutcome::Signaled { signal } => assert_eq!(signal, 15),
        other => panic!("expected signaled, got {other:?}"),
    }
}

#[test]
fn missing_program_is_a_setup_error() {
    let dir = workdir("missing");
    let mut run_spec = spec(dir, 1000, "true");
    run_spec.command.program = OsString::from("/definitely/not/a/program");
    match run(&run_spec) {
        Err(SetupError::Spawn(message)) => assert!(message.contains("definitely")),
        other => panic!("expected spawn error, got {other:?}"),
    }
}

#[test]
fn workdir_file_is_rejected() {
    let dir = workdir("workdir-file");
    let path = dir.join("not-a-dir");
    std::fs::write(&path, b"x").unwrap();
    let run_spec = spec(path, 1000, "true");
    assert!(matches!(run(&run_spec), Err(SetupError::Workdir(_))));
}

#[test]
fn environment_is_cleared_to_the_documented_minimum() {
    let dir = workdir("env");
    let report = run(&spec(
        dir,
        1000,
        "printf '%s|%s|%s' \"$HOME\" \"$TMPDIR\" \"$LC_ALL\"",
    ))
    .expect("runner");
    let text = String::from_utf8(report.stdout).unwrap();
    let parts: Vec<&str> = text.split('|').collect();
    assert_eq!(parts.len(), 3);
    assert!(parts[0].contains("h05-runner-"));
    assert!(parts[0].contains("env"));
    assert!(parts[1].ends_with("tmp"));
    assert_eq!(parts[2], "C");
}

#[cfg(target_os = "linux")]
#[test]
fn timeout_stops_the_process_group() {
    let dir = workdir("process-group");
    let marker = dir.join("late-marker");
    let script = format!("sleep 0.3; touch {}", marker.display());
    let report = run(&spec(dir, 50, &script)).expect("runner");
    assert_eq!(report.outcome, RunOutcome::Timeout);
    std::thread::sleep(Duration::from_millis(400));
    assert!(
        !marker.exists(),
        "descendant survived group termination: {}",
        marker.display()
    );
}

#[cfg(target_os = "linux")]
#[test]
fn prlimit_bounds_are_applied_on_linux() {
    use std::process::Command;
    let available = Command::new("prlimit")
        .arg("--version")
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if !available {
        return;
    }
    let dir = workdir("prlimit");
    let limit = 200_000_000u64;
    let mut run_spec = spec(dir, 1000, "grep 'Max address space' /proc/self/limits");
    run_spec.limits.address_space = Some(limit);
    let report = run(&run_spec).expect("runner");
    let text = String::from_utf8_lossy(&report.stdout);
    assert!(
        text.contains(&limit.to_string()),
        "limit not visible: {text}"
    );
}
