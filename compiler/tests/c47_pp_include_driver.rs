// ============================================================================
// c47_pp_include_driver.rs — B3-03b bounded quote-include acceptance.
//
// Exercises the candidate driver's leaf-include policy: quoted headers resolve
// only beside the including file; header `#define`/`#undef` and plain text are
// processed; missing/angle/traversal/nested/guard/pragma/after-conditional
// cases stay fail-closed with explicit diagnostics. No `-I`, system roots,
// nested includes, guards, or once semantics are claimed.
// ============================================================================

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "pp-include-{}-{}-{}",
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

fn write(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("scratch subdir");
    }
    std::fs::write(path, bytes).expect("source write");
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

fn assert_fail(output: &Output, needle: &str) {
    assert_eq!(output.status.code(), Some(1));
    let message = stderr(output);
    assert!(
        message.contains(needle),
        "stderr misses `{needle}`:\n{message}"
    );
}

#[test]
fn header_define_expands_after_include() {
    let dir = workdir("define");
    write(&dir.join("h.h"), b"#define X 2+3\n");
    write(
        &dir.join("main.c"),
        b"#include \"h.h\"\nint main(void){return X;}\n",
    );
    let source = dir.join("main.c");
    let evidence = run(&[std::ffi::OsStr::new("--interpret-ir"), source.as_os_str()]);
    assert_value(&evidence, "interpret: value=05 negative=false");

    let emitted = run(&[std::ffi::OsStr::new("-E"), source.as_os_str()]);
    assert_eq!(emitted.status.code(), Some(0), "{}", stderr(&emitted));
    let text = stdout(&emitted);
    assert!(!text.contains('#'), "directive leaked into -E: {text}");
    assert_eq!(text, "int main ( void ) { return 2 + 3 ; }\n");
}

#[test]
fn plain_header_text_is_spliced() {
    let dir = workdir("text");
    write(&dir.join("plain.h"), b"int h;\n");
    write(
        &dir.join("main.c"),
        b"#include \"plain.h\"\nint main(void){return 2+3;}\n",
    );
    let source = dir.join("main.c");
    let emitted = run(&[std::ffi::OsStr::new("-E"), source.as_os_str()]);
    assert_eq!(emitted.status.code(), Some(0), "{}", stderr(&emitted));
    let text = stdout(&emitted);
    assert!(text.contains("int h ;"), "header text missing: {text}");
    assert!(!text.contains("include"), "include leaked: {text}");
}

#[test]
fn header_undef_affects_later_text() {
    let dir = workdir("undef");
    write(&dir.join("undef.h"), b"#undef X\n");
    write(
        &dir.join("main.c"),
        b"#define X 2+3\n#include \"undef.h\"\nint main(void){return X;}\n",
    );
    let source = dir.join("main.c");
    let emitted = run(&[std::ffi::OsStr::new("-E"), source.as_os_str()]);
    assert_eq!(emitted.status.code(), Some(0), "{}", stderr(&emitted));
    assert!(stdout(&emitted).contains("return X"));

    let evidence = run(&[std::ffi::OsStr::new("--interpret-ir"), source.as_os_str()]);
    assert_eq!(evidence.status.code(), Some(1));
    assert!(stderr(&evidence).contains("non-M1 token sequence"));
}

#[test]
fn subdirectory_include_resolves() {
    let dir = workdir("subdir");
    write(&dir.join("sub/z.h"), b"#define Z 2+3\n");
    write(
        &dir.join("main.c"),
        b"#include \"sub/z.h\"\nint main(void){return Z;}\n",
    );
    let source = dir.join("main.c");
    let evidence = run(&[std::ffi::OsStr::new("--interpret-ir"), source.as_os_str()]);
    assert_value(&evidence, "interpret: value=05 negative=false");
}

#[test]
fn duplicate_include_reuses_the_same_source() {
    let dir = workdir("duplicate");
    write(&dir.join("h.h"), b"#define X 2+3\n");
    write(
        &dir.join("main.c"),
        b"#include \"h.h\"\n#include \"h.h\"\nint main(void){return X;}\n",
    );
    let source = dir.join("main.c");
    let evidence = run(&[std::ffi::OsStr::new("--interpret-ir"), source.as_os_str()]);
    assert_value(&evidence, "interpret: value=05 negative=false");
}

#[test]
fn missing_header_fails() {
    let dir = workdir("missing");
    write(
        &dir.join("main.c"),
        b"#include \"missing.h\"\nint main(void){return 2+3;}\n",
    );
    let source = dir.join("main.c");
    let output = run(&[std::ffi::OsStr::new("--interpret-ir"), source.as_os_str()]);
    assert_fail(&output, "cannot resolve include");
}

#[test]
fn angle_and_traversal_are_refused() {
    let dir = workdir("refused");
    write(
        &dir.join("angle.c"),
        b"#include <stdio.h>\nint main(void){return 2+3;}\n",
    );
    let angle = run(&[
        std::ffi::OsStr::new("--interpret-ir"),
        dir.join("angle.c").as_os_str(),
    ]);
    assert_fail(&angle, "angle-bracket/system includes");

    write(
        &dir.join("trav.c"),
        b"#include \"../outside.h\"\nint main(void){return 2+3;}\n",
    );
    let traversal = run(&[
        std::ffi::OsStr::new("--interpret-ir"),
        dir.join("trav.c").as_os_str(),
    ]);
    assert_fail(&traversal, "`..` path components");
}

#[test]
fn nested_include_is_refused() {
    let dir = workdir("nested");
    write(&dir.join("inner.h"), b"int inner;\n");
    write(&dir.join("outer.h"), b"#include \"inner.h\"\n");
    write(
        &dir.join("main.c"),
        b"#include \"outer.h\"\nint main(void){return 2+3;}\n",
    );
    let source = dir.join("main.c");
    let output = run(&[std::ffi::OsStr::new("--interpret-ir"), source.as_os_str()]);
    assert_fail(&output, "nested includes");
}

#[test]
fn header_guard_and_pragma_are_refused() {
    let dir = workdir("guard");
    write(&dir.join("guard.h"), b"#ifndef G\n#define G 1\n#endif\n");
    write(
        &dir.join("guard.c"),
        b"#include \"guard.h\"\nint main(void){return 2+3;}\n",
    );
    let guard = run(&[
        std::ffi::OsStr::new("--interpret-ir"),
        dir.join("guard.c").as_os_str(),
    ]);
    assert_fail(&guard, "header conditionals");

    write(&dir.join("once.h"), b"#pragma once\n#define Y 2+3\n");
    write(
        &dir.join("once.c"),
        b"#include \"once.h\"\nint main(void){return Y;}\n",
    );
    let once = run(&[
        std::ffi::OsStr::new("--interpret-ir"),
        dir.join("once.c").as_os_str(),
    ]);
    assert_fail(&once, "header directive `pragma`");
}

#[test]
fn include_after_conditional_is_refused() {
    let dir = workdir("after-conditional");
    write(&dir.join("h.h"), b"#define X 2+3\n");
    write(
        &dir.join("main.c"),
        b"#if 1\n#include \"h.h\"\n#endif\nint main(void){return 2+3;}\n",
    );
    let source = dir.join("main.c");
    let output = run(&[std::ffi::OsStr::new("--interpret-ir"), source.as_os_str()]);
    assert_fail(&output, "after the first conditional");
}

#[test]
fn include_evidence_is_deterministic() {
    let dir = workdir("deterministic");
    write(&dir.join("h.h"), b"#define X 2+3\n");
    write(
        &dir.join("main.c"),
        b"#include \"h.h\"\nint main(void){return X;}\n",
    );
    let source = dir.join("main.c");
    let args = [
        std::ffi::OsStr::new("--emit-ir-snapshot"),
        std::ffi::OsStr::new("--emit-trace"),
        std::ffi::OsStr::new("--interpret-ir"),
        source.as_os_str(),
    ];
    let first = run(&args);
    let second = run(&args);
    assert_eq!(first.status.code(), Some(0), "{}", stderr(&first));
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
}

#[cfg(unix)]
#[test]
fn symlink_escape_is_refused() {
    use std::os::unix::fs::symlink;
    let dir = workdir("symlink");
    let outside = dir
        .parent()
        .expect("scratch parent")
        .join(format!("outside-{}.h", std::process::id()));
    write(&outside, b"#define X 2+3\n");
    symlink(&outside, dir.join("link.h")).expect("symlink");
    write(
        &dir.join("main.c"),
        b"#include \"link.h\"\nint main(void){return X;}\n",
    );
    let source = dir.join("main.c");
    let output = run(&[std::ffi::OsStr::new("--interpret-ir"), source.as_os_str()]);
    assert_fail(&output, "escapes the including directory");
}
