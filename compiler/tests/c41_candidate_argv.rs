// ============================================================================
// c41_candidate_argv.rs — H04 candidate driver argv hardening (D-03).
//
// Regression coverage for the `args_os` conversion in `candidate`: a
// non-UTF-8 argv element must produce a structured `candidate:` driver
// diagnostic on stderr (exit 2) instead of the previous `std::env::args()`
// panic (exit 101 with a Rust panic message). A valid invocation must still
// follow the existing parse path and produce M1 evidence.
// ============================================================================

#![cfg(unix)]

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "candidate-argv-{}-{}-{}",
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

fn write_source(dir: &Path) -> PathBuf {
    let path = dir.join("main.c");
    std::fs::write(&path, b"int main(void){return 2+3;}\n").expect("source write");
    path
}

#[test]
fn candidate_rejects_non_utf8_argument() {
    let dir = workdir("non-utf8");
    let source = write_source(&dir);
    // `bad-\xff.c` has no `String` representation; the driver must reject
    // it by argv position before flag parsing.
    let invalid = OsString::from_vec(b"bad-\xff.c".to_vec());
    let output = Command::new(binary())
        .arg("-E")
        .arg(&source)
        .arg(&invalid)
        .output()
        .expect("candidate runs");

    assert_eq!(
        output.status.code(),
        Some(2),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.starts_with("candidate:"), "stderr: {stderr:?}");
    assert!(stderr.contains("argv[3]"), "stderr: {stderr:?}");
    assert!(stderr.contains("not valid UTF-8"), "stderr: {stderr:?}");
    assert!(!stderr.contains("panicked"), "stderr: {stderr:?}");
    assert!(
        !stderr.to_ascii_lowercase().contains("backtrace"),
        "stderr: {stderr:?}"
    );
    assert!(
        output.stdout.is_empty(),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn candidate_accepts_valid_argv() {
    let dir = workdir("valid");
    let source = write_source(&dir);
    let output = Command::new(binary())
        .arg(&source)
        .output()
        .expect("candidate runs");

    assert!(
        output.status.success(),
        "status: {:?}, stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 evidence");
    assert!(stdout.contains("candidate evidence"), "stdout: {stdout:?}");
    assert!(
        stdout.contains("interpret: value=05 negative=false"),
        "stdout: {stdout:?}"
    );
}
