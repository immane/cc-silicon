// ============================================================================
// sprint_l05.rs — source divide/remainder by zero is a compiler diagnostic.
//
// C leaves integer division and remainder by zero undefined, so this test never
// runs native code as an oracle. The candidate must reject the source with a
// typed diagnostic before writing either the assembly or the evidence report.
//
// The nonzero control values are hand-derived: 7 / 2 == 3 and 7 % 2 == 1.
// The direct-literal zero cases depend on the accepted zero normalization
// (L01); before it, `0` decodes with octal radix and the fold rejects it.
// ============================================================================

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_CASE: AtomicU64 = AtomicU64::new(0);

struct Run {
    output: Output,
    asm: PathBuf,
    report: PathBuf,
}

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir(case: &str) -> PathBuf {
    let serial = NEXT_CASE.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock moves forward")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "sprint-l05-{}-{}-{}-{}",
        std::process::id(),
        serial,
        nanos,
        case
    ));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn run_candidate(case: &str, source: &str) -> Run {
    let dir = workdir(case);
    let source_path = dir.join("main.c");
    fs::write(&source_path, source.as_bytes()).expect("source file");
    let asm = dir.join("main.s");
    let report = dir.join("evidence.txt");
    let output = Command::new(binary())
        .arg("--emit-asm")
        .arg(&asm)
        .arg("-o")
        .arg(&report)
        .arg("--interpret-ir")
        .arg(&source_path)
        .output()
        .expect("candidate runs");
    Run {
        output,
        asm,
        report,
    }
}

fn assert_diagnosed(run: &Run, diagnostic: &str) {
    assert_eq!(
        run.output.status.code(),
        Some(1),
        "{diagnostic}: stderr: {}",
        String::from_utf8_lossy(&run.output.stderr)
    );
    let stderr = String::from_utf8_lossy(&run.output.stderr);
    assert!(
        stderr.contains(diagnostic),
        "expected {diagnostic:?} in stderr, got {stderr}"
    );
    assert!(
        stderr.contains("candidate:"),
        "expected a driver diagnostic, got {stderr}"
    );
    let stdout = String::from_utf8_lossy(&run.output.stdout);
    assert!(
        !stdout.contains("candidate evidence") && !stdout.contains("interpret:"),
        "failed compilation must not report success: {stdout}"
    );
    assert!(
        !run.asm.exists(),
        "assembly must not be written after failure: {}",
        run.asm.display()
    );
    assert!(
        !run.report.exists(),
        "evidence report must not be written after failure: {}",
        run.report.display()
    );
}

fn assert_published(run: &Run, magnitude_hex: &str) {
    assert_eq!(
        run.output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&run.output.stderr)
    );
    assert!(
        run.asm.exists(),
        "assembly artifact missing: {}",
        run.asm.display()
    );
    assert!(
        run.report.exists(),
        "evidence report missing: {}",
        run.report.display()
    );
    let bytes = fs::read(&run.report).expect("evidence report");
    let report = String::from_utf8_lossy(&bytes);
    assert!(
        report.contains("candidate evidence"),
        "evidence report is not a success report: {report}"
    );
    let expected = format!("interpret: value={magnitude_hex} negative=false");
    assert!(
        report.contains(expected.as_str()),
        "expected modeled value {magnitude_hex}, got {report}"
    );
}

#[test]
fn literal_zero_denominators_are_diagnosed_before_artifacts() {
    for (case, source, diagnostic) in [
        (
            "div-zero-literal",
            "int main(void){return 7/0;}\n",
            "division by zero",
        ),
        (
            "rem-zero-literal",
            "int main(void){return 7%0;}\n",
            "remainder by zero",
        ),
    ] {
        let run = run_candidate(case, source);
        assert_diagnosed(&run, diagnostic);
    }
}

#[test]
fn computed_zero_denominators_are_diagnosed_before_artifacts() {
    for (case, source, diagnostic) in [
        (
            "div-computed-zero",
            "int main(void){return 7/(3-3);}\n",
            "division by zero",
        ),
        (
            "rem-computed-zero",
            "int main(void){return 7%(3-3);}\n",
            "remainder by zero",
        ),
    ] {
        let run = run_candidate(case, source);
        assert_diagnosed(&run, diagnostic);
    }
}

#[test]
fn defined_division_and_remainder_publish_artifacts() {
    for (case, source, magnitude_hex) in [
        ("div-defined", "int main(void){return 7/2;}\n", "03"),
        ("rem-defined", "int main(void){return 7%2;}\n", "01"),
    ] {
        let run = run_candidate(case, source);
        assert_published(&run, magnitude_hex);
    }
}
