// ============================================================================
// sprint_t28.rs — `/55` replay repeatability for result, assembly, snapshot
// and trace.
//
// One source is compiled by two fresh candidate processes with different
// assembly destinations. The modeled result is hand-derived from the C
// expression `(2 + 3) * 2 = 10`; the expected assembly lines are derived from
// the committed local-load/store plus Add and Mul IR shape. Replay requires
// the evidence report, assembly bytes, and both semantic hashes to repeat.
//
// No expected value below was produced by running the implementation.
// ============================================================================

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SOURCE: &str = "int main(void){int x=2;return (x+3)*2;}\n";

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sprint-t28-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock moves forward")
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn compile(source: &Path, asm: &Path) -> Output {
    Command::new(binary())
        .arg("--emit-asm")
        .arg(asm)
        .arg("--emit-ir-snapshot")
        .arg("--emit-trace")
        .arg("--interpret-ir")
        .arg(source)
        .output()
        .expect("candidate runs")
}

fn evidence_line<'a>(report: &'a str, prefix: &str) -> &'a str {
    report
        .lines()
        .find(|line| line.starts_with(prefix))
        .unwrap_or_else(|| panic!("missing {prefix:?} in:\n{report}"))
}

#[test]
fn replay_repeats_result_assembly_snapshot_and_trace() {
    let dir = workdir();
    let source = dir.join("main.c");
    fs::write(&source, SOURCE).expect("source file");

    // Distinct destinations prove the assembly bytes do not depend on where
    // the host stores them.
    let first_asm = dir.join("first.s");
    let second_asm = dir.join("second.s");
    let first = compile(&source, &first_asm);
    let second = compile(&source, &second_asm);

    for (label, output) in [("first", &first), ("second", &second)] {
        assert_eq!(
            output.status.code(),
            Some(0),
            "{label} replay run failed; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // A fresh process per run: the whole report must be byte-identical.
    assert_eq!(first.stdout, second.stdout, "evidence report diverged");
    assert_eq!(first.stderr, second.stderr, "diagnostics diverged");

    let first_assembly = fs::read(&first_asm).expect("first assembly");
    let second_assembly = fs::read(&second_asm).expect("second assembly");
    assert_eq!(
        first_assembly, second_assembly,
        "assembly bytes diverged across replay"
    );

    let report = String::from_utf8(first.stdout).expect("utf-8 evidence report");
    assert_eq!(
        evidence_line(&report, "interpret: "),
        "interpret: value=0a negative=false",
        "(2 + 3) * 2 is hand-derived as 10 = 0x0a"
    );

    for prefix in ["snapshot: ", "trace: "] {
        let value = evidence_line(&report, prefix)
            .strip_prefix(prefix)
            .expect("line was selected by its prefix");
        assert_eq!(value.len(), 64, "{prefix}{value}");
        assert!(
            value
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
            "{prefix}{value}"
        );
    }

    let assembly = String::from_utf8(first_assembly).expect("utf-8 assembly");
    for instruction in [
        "sub sp, sp, #",
        "add sp, sp, #",
        "movz w0, #2",
        "movz w0, #3",
        "add w0, w0, w1",
        "mul w0, w0, w1",
        "str w0, [sp, #0]",
        "ldr w0, [sp, #0]",
        "    ret\n",
    ] {
        assert!(
            assembly.contains(instruction),
            "hand-derived instruction {instruction:?} missing from assembly:\n{assembly}"
        );
    }
}
