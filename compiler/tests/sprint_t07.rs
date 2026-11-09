// ============================================================================
// sprint_t07.rs — T07 entry-selection regression.
//
// Independent source-level checks for the frozen `/55` entry rule: exactly one
// non-parameterized `main` definition selects the program entry, and `main` is
// not callable. Expectations come from that rule, not from another run: in the
// accepted source the middle function is `main` and returns 3 while its
// neighbours return 7 and 9, so selecting any other function changes the
// modeled value.
// ============================================================================

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_WORKDIR: AtomicU64 = AtomicU64::new(0);

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir(tag: &str) -> PathBuf {
    let serial = NEXT_WORKDIR.fetch_add(1, Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("sprint-t07-{}-{tag}-{serial}", std::process::id()));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// Run the candidate with `--emit-asm` so a rejected source must also leave no
/// assembly artifact.
fn run(source: &str, tag: &str) -> (Output, PathBuf) {
    let dir = workdir(tag);
    let source_path = dir.join("case.c");
    let asm_path = dir.join("case.s");
    fs::write(&source_path, source).expect("source write");
    let output = Command::new(binary())
        .arg("--emit-asm")
        .arg(&asm_path)
        .arg(&source_path)
        .output()
        .expect("candidate runs");
    (output, asm_path)
}

fn assert_rejected(source: &str, tag: &str, message: &str) {
    let (output, asm) = run(source, tag);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{tag}: source unexpectedly accepted\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "{tag}: rejected source wrote evidence"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("candidate: "),
        "{tag}: rejection must use the candidate stderr channel: {stderr}"
    );
    assert!(
        stderr.contains(message),
        "{tag}: expected `{message}` in: {stderr}"
    );
    assert!(
        !asm.exists(),
        "{tag}: rejected source left an assembly artifact at {}",
        asm.display()
    );
}

#[test]
fn entry_selects_the_middle_function_named_main() {
    let source = "int first(void) { return 7; }\n\
                  int main(void) { return 3; }\n\
                  int last(void) { return 9; }\n";
    let (output, asm) = run(source, "middle-main");
    assert!(
        output.status.success(),
        "accepted entry source failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "accepted source wrote stderr");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("candidate evidence (t01-c01-c06/55)\n"),
        "missing evidence header: {stdout}"
    );
    assert!(
        stdout.contains("interpret: value=03 negative=false"),
        "entry must be the middle `main` returning 3: {stdout}"
    );
    let assembly = fs::read_to_string(&asm).expect("assembly written");
    for name in ["first", "main", "last"] {
        assert!(
            assembly.contains(&format!("{name}:")),
            "missing {name} label"
        );
    }
}

#[test]
fn entry_absent_main_is_rejected() {
    assert_rejected("int helper(void) { return 1; }\n", "absent-main", "no main");
}

#[test]
fn entry_duplicate_main_is_rejected() {
    assert_rejected(
        "int main(void) { return 1; }\n\
         int main(void) { return 2; }\n",
        "duplicate-main",
        "duplicate",
    );
}

#[test]
fn entry_parameterized_main_is_rejected() {
    assert_rejected(
        "int main(int x) { return x; }\n",
        "parameterized-main",
        "main must not have parameters",
    );
}

#[test]
fn entry_rejects_self_call_to_main() {
    assert_rejected(
        "int main(void) { return main(); }\n",
        "self-call-main",
        "se-call to `main`",
    );
}

#[test]
fn entry_rejects_call_to_main_from_a_later_function() {
    assert_rejected(
        "int main(void) { return 1; }\n\
         int helper(void) { return main(); }\n",
        "later-call-main",
        "se-call to `main`",
    );
}
