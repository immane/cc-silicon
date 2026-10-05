// ============================================================================
// h04_candidate.rs — H04 candidate driver (Part A) acceptance.
//
// Exercises the `candidate` binary end to end: the M1 source produces
// snapshot/trace evidence with the modeled return `5`, repeated runs are
// byte-identical, invalid input yields a structured diagnostic with no
// evidence (exit 1), and target emission (`-S`) plus unknown flags fail
// closed (exit 2). This is host tooling: no contract version is bumped
// and no frozen hash changes here.
// ============================================================================

use std::path::PathBuf;
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "candidate-{}-{}-{}",
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

fn write_source(dir: &std::path::Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("source write");
    path
}

const M1_MAIN: &[u8] = b"int main(void){return 2+3;}\n";
const M1_BAD: &[u8] = b"int main(void){return foo;}\n";

#[test]
fn candidate_models_m1_return_5() {
    let dir = workdir("model");
    let source = write_source(&dir, "main.c", M1_MAIN);
    let output = Command::new(binary())
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 evidence");
    assert!(stdout.contains("candidate evidence (t01-c01-c06/34)"));
    assert!(stdout.contains("interpret: value=05 negative=false"));
    assert!(stdout.contains("snapshot: "));
    assert!(stdout.contains("trace: "));
}

#[test]
fn candidate_evidence_is_deterministic() {
    let dir = workdir("determinism");
    let source = write_source(&dir, "main.c", M1_MAIN);
    let first = Command::new(binary())
        .arg(&source)
        .output()
        .expect("candidate runs");
    let second = Command::new(binary())
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert!(first.status.success() && second.status.success());
    assert_eq!(first.stdout, second.stdout);
}

#[test]
fn candidate_rejects_invalid_input_with_diagnostic() {
    let dir = workdir("invalid");
    let source = write_source(&dir, "bad.c", M1_BAD);
    let output = Command::new(binary())
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(!stdout.contains("snapshot: "));
    assert!(!stdout.contains("interpret: "));
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("candidate diagnostic: "));
}

#[test]
fn candidate_refuses_target_emission() {
    let dir = workdir("refuse");
    let source = write_source(&dir, "main.c", M1_MAIN);
    let output = Command::new(binary())
        .arg("-S")
        .arg("-o")
        .arg(dir.join("main.s"))
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert_eq!(output.status.code(), Some(2));
    assert!(!dir.join("main.s").exists());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("fail-closed"));
}

#[test]
fn candidate_rejects_unknown_flags() {
    let dir = workdir("flags");
    let source = write_source(&dir, "main.c", M1_MAIN);
    let output = Command::new(binary())
        .arg("--optimize-everything")
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn candidate_writes_evidence_to_output_path() {
    let dir = workdir("output");
    let source = write_source(&dir, "main.c", M1_MAIN);
    let report = dir.join("evidence.txt");
    let output = Command::new(binary())
        .arg("-o")
        .arg(&report)
        .arg("--interpret-ir")
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    let text = std::fs::read_to_string(&report).expect("report written");
    assert!(text.contains("interpret: value=05 negative=false"));
    assert!(!text.contains("snapshot: "));
}
