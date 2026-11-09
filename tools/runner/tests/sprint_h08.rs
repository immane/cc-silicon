//! Independent H08 CLI tests for H02 status fidelity and report escaping.
//!
//! The real binary from `CARGO_BIN_EXE` is invoked. H05's documented runner
//! exit status is kept (0 success, 1 nonzero child, 2 setup error, 6 signal);
//! the report must additionally disclose the child's actual exit code or
//! terminating signal. Reports and setup errors must stay one key=value line
//! per field when a workdir or program path contains control characters.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const BIN: &str = env!("CARGO_BIN_EXE_cc-silicon-runner");

fn unique(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock moves forward")
        .as_nanos();
    format!("sprint-h08-{}-{label}-{nanos}", std::process::id())
}

fn scratch(label: &str, suffix: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("{}-{suffix}", unique(label)));
    fs::create_dir_all(&dir).expect("scratch directory");
    dir.canonicalize().expect("canonical scratch directory")
}

fn runner(workdir: &Path) -> Command {
    let mut command = Command::new(BIN);
    command
        .arg("--workdir")
        .arg(workdir)
        .args(["--timeout-ms", "2000"])
        .arg("--");
    command
}

fn run_script(workdir: &Path, script: &str) -> Output {
    let mut command = runner(workdir);
    command.arg("sh").arg("-c").arg(script);
    command.output().expect("runner executes")
}

/// Parse the documented report into fields and reject unescaped tab/CR values.
fn report(output: &Output) -> Vec<(String, String)> {
    let text = std::str::from_utf8(&output.stdout).expect("UTF-8 runner report");
    assert!(
        text.ends_with('\n'),
        "report must end with a newline: {text:?}"
    );
    text.lines()
        .map(|line| {
            let (key, value) = line
                .split_once('=')
                .unwrap_or_else(|| panic!("report line is not key=value: {line:?}"));
            assert!(!key.is_empty(), "empty report key: {line:?}");
            assert!(
                !value.contains('\r') && !value.contains('\t'),
                "unescaped control character in report value: {line:?}"
            );
            (key.to_string(), value.to_string())
        })
        .collect()
}

fn field<'a>(fields: &'a [(String, String)], key: &str) -> &'a str {
    let mut found = fields.iter().filter(|(name, _)| name == key);
    let first = found
        .next()
        .unwrap_or_else(|| panic!("missing `{key}` in report {fields:?}"));
    assert!(
        found.next().is_none(),
        "duplicate `{key}` in report {fields:?}"
    );
    first.1.as_str()
}

/// Mirror the documented reversible escaping for paths.
fn escaped(path: &Path) -> String {
    let text = path.to_string_lossy();
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped
}

#[test]
fn child_exit_five_is_completion_not_output_limit() {
    let dir = scratch("exit-5", "plain");
    let output = run_script(&dir, "exit 5");
    assert_eq!(
        output.status.code(),
        Some(1),
        "H05 maps a nonzero child exit to runner status 1: {:?}",
        output.status
    );
    let fields = report(&output);
    assert_eq!(field(&fields, "outcome"), "completed");
    assert_ne!(field(&fields, "outcome"), "output-limit");
    assert_eq!(field(&fields, "exit_code"), "1");
    assert_eq!(field(&fields, "stdout_truncated"), "false");
    assert_eq!(field(&fields, "child_exit"), "5");
    assert_eq!(field(&fields, "signal"), "none");
}

#[test]
fn child_exit_forty_two_is_reported_verbatim() {
    let dir = scratch("exit-42", "plain");
    let output = run_script(&dir, "exit 42");
    assert_eq!(
        output.status.code(),
        Some(1),
        "H05 maps a nonzero child exit to runner status 1: {:?}",
        output.status
    );
    let fields = report(&output);
    assert_eq!(field(&fields, "outcome"), "completed");
    assert_eq!(field(&fields, "exit_code"), "1");
    assert_eq!(field(&fields, "child_exit"), "42");
    assert_eq!(field(&fields, "signal"), "none");
}

#[test]
fn child_signals_are_reported_with_actual_numbers() {
    for (label, script, expected) in [("term", "kill -TERM $$", 15), ("kill", "kill -KILL $$", 9)] {
        let dir = scratch("signal", label);
        let output = run_script(&dir, script);
        assert_eq!(
            output.status.code(),
            Some(6),
            "{label}: H05 maps signal termination to runner status 6: {:?}",
            output.status
        );
        let fields = report(&output);
        assert_eq!(field(&fields, "outcome"), "signaled", "{label}");
        assert_eq!(field(&fields, "exit_code"), "6", "{label}");
        assert_eq!(field(&fields, "child_exit"), "none", "{label}");
        assert_eq!(
            field(&fields, "signal"),
            expected.to_string().as_str(),
            "{label}: report fabricated a signal instead of the child's status"
        );
    }
}

#[test]
fn report_paths_escape_control_characters() {
    let dir = scratch("escape", "line\nbreak\ttab\rreturn\\slash");
    let output = run_script(&dir, "exit 0");
    assert_eq!(output.status.code(), Some(0), "status: {:?}", output.status);
    let fields = report(&output);
    assert_eq!(field(&fields, "workdir"), escaped(&dir).as_str());
    assert_eq!(
        field(&fields, "stdout"),
        escaped(&dir.join("stdout.log")).as_str()
    );
    assert_eq!(
        field(&fields, "stderr"),
        escaped(&dir.join("stderr.log")).as_str()
    );
    assert_eq!(field(&fields, "child_exit"), "0");
}

#[test]
fn setup_error_is_escaped_and_emits_no_report() {
    let dir = scratch("setup", "plain");
    let missing = dir.join("missing\nprogram\ttab\rreturn\\slash");
    let output = runner(&dir)
        .arg(&missing)
        .output()
        .expect("runner executes");
    assert_eq!(
        output.status.code(),
        Some(2),
        "setup errors keep runner status 2: {:?}",
        output.status
    );
    assert!(
        output.stdout.is_empty(),
        "setup error emitted a report: {:?}",
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
    let body = stderr
        .strip_suffix('\n')
        .expect("setup error ends with a newline");
    assert_eq!(
        body.lines().count(),
        1,
        "setup error must remain one escaped line: {body:?}"
    );
    assert!(
        !body.contains('\r') && !body.contains('\t'),
        "setup error contains an unescaped control character: {body:?}"
    );
    assert!(
        body.starts_with("cc-silicon-runner:"),
        "unexpected setup error prefix: {body:?}"
    );
    assert!(
        body.contains(escaped(&missing).as_str()),
        "escaped program path is absent from setup error: {body:?}"
    );
}
