// ============================================================================
// c46_pp_driver.rs — B3-03a bounded candidate PP driver acceptance.
//
// Exercises the host line walk end to end: leading source `#define`/`#undef`
// affect later text and conditionals, inactive post-conditional definitions
// are dropped by PP19, active post-conditional macro mutation is rejected,
// benign redefinition succeeds and non-benign redefinition fails, consumed
// directive lines never reach the lexer, and `#error`/continuation cases stay
// fail-closed. Include behavior is covered by c47. This is not a full
// preprocessor claim.
// ============================================================================

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "pp-driver-{}-{}-{}",
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

fn source(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("source write");
    path
}

fn run(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(binary())
        .args(args)
        .output()
        .expect("candidate runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("utf8 stdout")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("utf8 stderr")
}

fn assert_value(output: &Output, needle: &str) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected success, stderr: {}",
        stderr(output)
    );
    let text = stdout(output);
    assert!(text.contains(needle), "stdout misses `{needle}`:\n{text}");
}

#[test]
fn define_expands_later_text_and_line_is_consumed() {
    let dir = workdir("define");
    let path = source(
        &dir,
        "main.c",
        b"#define X 2+3\nint main(void){return X;}\n",
    );
    let evidence = run(&[std::ffi::OsStr::new("--interpret-ir"), path.as_os_str()]);
    assert_value(&evidence, "interpret: value=05 negative=false");
    assert!(
        !stdout(&evidence).contains("#"),
        "directive token leaked into evidence"
    );

    let emitted = run(&[std::ffi::OsStr::new("-E"), path.as_os_str()]);
    assert_eq!(emitted.status.code(), Some(0), "{}", stderr(&emitted));
    let text = stdout(&emitted);
    assert!(!text.contains('#'), "directive leaked into -E: {text}");
    assert!(!text.contains("define"), "directive name leaked: {text}");
    assert_eq!(text, "int main ( void ) { return\n2 + 3\n; }\n");
}

#[test]
fn use_before_define_is_not_expanded() {
    let dir = workdir("use-before");
    let path = source(
        &dir,
        "main.c",
        b"int x = X;\n#define X 2+3\nint main(void){return 2+3;}\n",
    );
    let emitted = run(&[std::ffi::OsStr::new("-E"), path.as_os_str()]);
    assert_eq!(emitted.status.code(), Some(0), "{}", stderr(&emitted));
    let text = stdout(&emitted);
    assert!(text.contains("x = X"), "pre-define use expanded: {text}");
    assert!(text.contains("2 + 3"));
}

#[test]
fn undef_prevents_later_expansion() {
    let dir = workdir("undef");
    let path = source(
        &dir,
        "main.c",
        b"#define X 2+3\n#undef X\nint main(void){return X;}\n",
    );
    let emitted = run(&[std::ffi::OsStr::new("-E"), path.as_os_str()]);
    assert_eq!(emitted.status.code(), Some(0), "{}", stderr(&emitted));
    let text = stdout(&emitted);
    assert!(
        text.contains("return X"),
        "undefined X was expanded: {text}"
    );

    let evidence = run(&[std::ffi::OsStr::new("--interpret-ir"), path.as_os_str()]);
    assert_eq!(evidence.status.code(), Some(1));
    assert!(stderr(&evidence).contains("non-M1 token sequence"));
}

#[test]
fn benign_redefine_succeeds_and_nonbenign_fails() {
    let dir = workdir("redefine");
    let ok = source(
        &dir,
        "ok.c",
        b"#define X 2+3\n#define X 2+3\nint main(void){return X;}\n",
    );
    let ok_run = run(&[std::ffi::OsStr::new("--interpret-ir"), ok.as_os_str()]);
    assert_value(&ok_run, "interpret: value=05 negative=false");

    let bad = source(
        &dir,
        "bad.c",
        b"#define X 2+3\n#define X 3+3\nint main(void){return X;}\n",
    );
    let bad_run = run(&[std::ffi::OsStr::new("--interpret-ir"), bad.as_os_str()]);
    assert_eq!(bad_run.status.code(), Some(1));
    assert!(
        stderr(&bad_run).contains("redefine mismatch"),
        "{}",
        stderr(&bad_run)
    );
}

#[test]
fn leading_define_drives_conditional_branch() {
    let dir = workdir("conditional-leading");
    let true_branch = source(
        &dir,
        "true.c",
        b"#define X 1\n#if X\nint main(void){return 2+3;}\n#else\nint main(void){return 0;}\n#endif\n",
    );
    let true_run = run(&[
        std::ffi::OsStr::new("--interpret-ir"),
        true_branch.as_os_str(),
    ]);
    assert_value(&true_run, "interpret: value=05 negative=false");

    let false_branch = source(
        &dir,
        "false.c",
        b"#define X 0\n#if X\nint main(void){return 0;}\n#else\nint main(void){return 2+3;}\n#endif\n",
    );
    let false_run = run(&[
        std::ffi::OsStr::new("--interpret-ir"),
        false_branch.as_os_str(),
    ]);
    assert_value(&false_run, "interpret: value=05 negative=false");
}

#[test]
fn inactive_post_conditional_define_is_dropped() {
    let dir = workdir("inactive-define");
    let path = source(
        &dir,
        "main.c",
        b"#if 0\n#define X 2+3\n#endif\nint main(void){return 2+3;}\n",
    );
    let evidence = run(&[std::ffi::OsStr::new("--interpret-ir"), path.as_os_str()]);
    assert_value(&evidence, "interpret: value=05 negative=false");
}

#[test]
fn active_post_conditional_define_is_rejected() {
    let dir = workdir("active-define");
    let path = source(
        &dir,
        "main.c",
        b"#if 1\n#define X 2+3\n#endif\nint main(void){return 2+3;}\n",
    );
    let evidence = run(&[std::ffi::OsStr::new("--interpret-ir"), path.as_os_str()]);
    assert_eq!(evidence.status.code(), Some(1));
    let message = stderr(&evidence);
    assert!(
        message.contains("after the first conditional"),
        "unexpected diagnostic: {message}"
    );
}

#[test]
fn error_remains_fail_closed() {
    let dir = workdir("fail-closed");
    let error = source(
        &dir,
        "error.c",
        b"#error boom\nint main(void){return 2+3;}\n",
    );
    let error_run = run(&[std::ffi::OsStr::new("--interpret-ir"), error.as_os_str()]);
    assert_eq!(error_run.status.code(), Some(1));
    assert!(
        stderr(&error_run).contains("boom"),
        "{}",
        stderr(&error_run)
    );
}

#[test]
fn continued_directive_is_rejected() {
    let dir = workdir("continuation");
    let path = source(
        &dir,
        "main.c",
        b"#define X 2 + \\\n3\nint main(void){return X;}\n",
    );
    let evidence = run(&[std::ffi::OsStr::new("--interpret-ir"), path.as_os_str()]);
    assert_eq!(evidence.status.code(), Some(1));
    assert!(
        stderr(&evidence).contains("spans multiple physical lines"),
        "{}",
        stderr(&evidence)
    );
}

#[test]
fn evidence_is_deterministic_across_runs() {
    let dir = workdir("deterministic");
    let path = source(
        &dir,
        "main.c",
        b"#define X 2+3\nint main(void){return X;}\n",
    );
    let args = [
        std::ffi::OsStr::new("--emit-ir-snapshot"),
        std::ffi::OsStr::new("--emit-trace"),
        std::ffi::OsStr::new("--interpret-ir"),
        path.as_os_str(),
    ];
    let first = run(&args);
    let second = run(&args);
    assert_eq!(first.status.code(), Some(0));
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
}
