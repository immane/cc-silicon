// ============================================================================
// h04_candidate.rs — H04 candidate driver (Part A) acceptance.
//
// Exercises the `candidate` binary end to end: the M1 source produces
// snapshot/trace evidence with the modeled return `5`, repeated runs are
// byte-identical, invalid input yields a structured diagnostic with no
// evidence (exit 1), and target emission (`-S`) plus unknown flags fail
// closed (exit 2). Part B adds host-only `-E` preprocessed emission through
// the frozen PP28 worker (golden bytes, determinism, `-o` file, `#error`
// exit 1, evidence-flag clash exit 2). H04 `-D`/`-U` seeds the frozen macro
// table in flag order (bare `-D` defaults to `1`, values expand, `-U`
// undefines, last-wins; bad names/values fail closed with exit 2). This is
// host tooling: no contract version is bumped and no frozen hash changes here.
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
const M1_ERROR: &[u8] = b"#error boom\nint main(void){return 2+3;}\n";
/// Source returning the `FOO` macro: `-D` seeds expand it, `-U` (or no
/// flag) leaves the spelling untouched.
const M1_USE_FOO: &[u8] = b"int main(void){return FOO;}\n";
/// Golden PP28 emission of [`M1_USE_FOO`] under a bare `-D FOO` (default
/// replacement `1`).
const M1_FOO_ONE_GOLDEN: &[u8] = b"int main ( void ) { return 1 ; }\n";
/// Conditional source distinguishing defined-as-empty (`-D FOO=`, first
/// branch) from undefined (`-U FOO`, second branch).
const M1_IFDEF_FOO: &[u8] =
    b"#ifdef FOO\nint main(void){return 2+3;}\n#else\nint main(void){return 0;}\n#endif\n";
/// Golden PP28 emission of [`M1_MAIN`]: single-line source, one space
/// between tokens, terminal newline.
const M1_EMIT_GOLDEN: &[u8] = b"int main ( void ) { return 2 + 3 ; }\n";

#[test]
fn candidate_emit_golden() {
    let dir = workdir("emit-golden");
    let source = write_source(&dir, "main.c", M1_MAIN);
    let output = Command::new(binary())
        .arg("-E")
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert!(output.status.success());
    assert_eq!(output.stdout, M1_EMIT_GOLDEN);
    assert!(output.stderr.is_empty());
}

#[test]
fn candidate_emit_is_deterministic() {
    let dir = workdir("emit-determinism");
    let source = write_source(&dir, "main.c", M1_MAIN);
    let first = Command::new(binary())
        .arg("-E")
        .arg(&source)
        .output()
        .expect("candidate runs");
    let second = Command::new(binary())
        .arg("-E")
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert!(first.status.success() && second.status.success());
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stdout, M1_EMIT_GOLDEN);
}

#[test]
fn candidate_emit_writes_to_output_path() {
    let dir = workdir("emit-output");
    let source = write_source(&dir, "main.c", M1_MAIN);
    let emitted = dir.join("main.i");
    let output = Command::new(binary())
        .arg("-E")
        .arg("-o")
        .arg(&emitted)
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    let text = std::fs::read(&emitted).expect("emitted file written");
    assert_eq!(text, M1_EMIT_GOLDEN);
}

#[test]
fn candidate_emit_rejects_error_directive() {
    let dir = workdir("emit-error");
    let source = write_source(&dir, "bad.c", M1_ERROR);
    let emitted = dir.join("bad.i");
    let output = Command::new(binary())
        .arg("-E")
        .arg("-o")
        .arg(&emitted)
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert_eq!(output.status.code(), Some(1));
    assert!(!emitted.exists());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("candidate diagnostic: "));
    assert!(stderr.contains("boom"));
}

#[test]
fn candidate_emit_clashes_with_evidence_selection() {
    let dir = workdir("emit-clash");
    let source = write_source(&dir, "main.c", M1_MAIN);
    for flag in ["--emit-ir-snapshot", "--emit-trace", "--interpret-ir"] {
        let output = Command::new(binary())
            .arg("-E")
            .arg(flag)
            .arg(&source)
            .output()
            .expect("candidate runs");
        assert_eq!(output.status.code(), Some(2), "flag: {flag}");
        let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
        assert!(stderr.contains("clashes"), "flag: {flag}");
    }
}
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
    assert!(stdout.contains("candidate evidence (t01-c01-c06/36)"));
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

#[test]
fn candidate_define_bare_defaults_to_one() {
    let dir = workdir("define-bare");
    let source = write_source(&dir, "main.c", M1_USE_FOO);
    let output = Command::new(binary())
        .arg("-E")
        .arg("-D")
        .arg("FOO")
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert!(output.status.success());
    assert_eq!(output.stdout, M1_FOO_ONE_GOLDEN);
}

#[test]
fn candidate_define_value_expands() {
    let dir = workdir("define-value");
    let source = write_source(&dir, "main.c", M1_USE_FOO);
    let output = Command::new(binary())
        .arg("-E")
        .arg("-DFOO=2+3")
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert!(output.status.success());
    assert_eq!(output.stdout, M1_EMIT_GOLDEN);
}

#[test]
fn candidate_undef_removes_prior_define() {
    let dir = workdir("undef");
    let source = write_source(&dir, "main.c", M1_USE_FOO);
    let output = Command::new(binary())
        .arg("-E")
        .arg("-DFOO=1")
        .arg("-U")
        .arg("FOO")
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 emission");
    assert!(stdout.contains("FOO"), "undef leaves the spelling: {stdout}");
}

#[test]
fn candidate_define_last_wins() {
    let dir = workdir("last-wins");
    let source = write_source(&dir, "main.c", M1_USE_FOO);
    let output = Command::new(binary())
        .arg("-E")
        .arg("-DFOO=1")
        .arg("-DFOO=2")
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert!(output.status.success());
    assert_eq!(output.stdout, b"int main ( void ) { return 2 ; }\n");
    let output = Command::new(binary())
        .arg("-E")
        .arg("-UFOO")
        .arg("-DFOO=7")
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert!(output.status.success());
    assert_eq!(output.stdout, b"int main ( void ) { return 7 ; }\n");
}

#[test]
fn candidate_empty_define_stays_defined() {
    let dir = workdir("define-empty");
    let source = write_source(&dir, "main.c", M1_IFDEF_FOO);
    let defined = Command::new(binary())
        .arg("-E")
        .arg("-DFOO=")
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert!(defined.status.success());
    assert_eq!(defined.stdout, M1_EMIT_GOLDEN);
    let undefined = Command::new(binary())
        .arg("-E")
        .arg("-UFOO")
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert!(undefined.status.success());
    assert_eq!(undefined.stdout, b"int main ( void ) { return 0 ; }\n");
}

#[test]
fn candidate_rejects_bad_macro_flags() {
    let dir = workdir("bad-macro-flag");
    let source = write_source(&dir, "main.c", M1_MAIN);
    // Each case is a driver error (exit 2): bad name, bad value, and a
    // dangling separate-argument `-D`.
    let cases: &[&[&str]] = &[
        &["-D9LIVES=1"],
        &["-DFOO-BAR=1"],
        &["-UFOO=1"],
        &["-DFOO=@"],
        &["-D"],
    ];
    for case in cases {
        let mut command = Command::new(binary());
        command.arg("-E");
        for flag in *case {
            command.arg(flag);
        }
        let output = command.arg(&source).output().expect("candidate runs");
        assert_eq!(output.status.code(), Some(2), "case: {case:?}");
        let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
        assert!(stderr.contains("-D") || stderr.contains("-U"), "case: {case:?}");
    }
}

#[test]
fn candidate_define_leaves_m1_evidence_intact() {
    let dir = workdir("define-evidence");
    let source = write_source(&dir, "main.c", M1_MAIN);
    let output = Command::new(binary())
        .arg("-DFOO=1")
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 evidence");
    assert!(stdout.contains("interpret: value=05 negative=false"));
}
