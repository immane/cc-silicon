// ============================================================================
// sprint_t14.rs — `/55` adversarial signed division and remainder regression.
//
// Focus: for defined negative inputs, C99 signed division truncates toward
// zero and the remainder satisfies n == q * d + r with r's sign equal to the
// dividend's (or r == 0). Every (n, d) pair below has a nonzero divisor and
// excludes INT_MIN / -1. The expected q and r values are hand calculations;
// they are not obtained by running the implementation under test.
//
// The test observes both the source driver's modeled sign/magnitude and the
// emitted AArch64 `sdiv`/`msub` sequence. It adds no source-level feature.
// ============================================================================

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct Case {
    name: &'static str,
    numerator: i32,
    denominator: i32,
    /// Hand calculation of trunc(numerator / denominator).
    quotient: i32,
    /// Hand calculation of numerator - quotient * denominator.
    remainder: i32,
}

const CASES: &[Case] = &[
    Case {
        name: "negative-over-positive",
        numerator: -7,
        denominator: 3,
        quotient: -2,
        remainder: -1,
    },
    Case {
        name: "positive-over-negative",
        numerator: 7,
        denominator: -3,
        quotient: -2,
        remainder: 1,
    },
    Case {
        name: "negative-over-negative",
        numerator: -7,
        denominator: -3,
        quotient: 2,
        remainder: -1,
    },
    Case {
        name: "positive-control",
        numerator: 7,
        denominator: 3,
        quotient: 2,
        remainder: 1,
    },
    Case {
        name: "truncate-not-floor",
        numerator: -9,
        denominator: 4,
        quotient: -2,
        remainder: -1,
    },
    Case {
        name: "negative-magnitude",
        numerator: -70000,
        denominator: 3,
        quotient: -23333,
        remainder: -1,
    },
    Case {
        name: "exact-negative",
        numerator: -8,
        denominator: 4,
        quotient: -2,
        remainder: 0,
    },
    Case {
        name: "negative-dividend-negative-divisor-remainder",
        numerator: -8,
        denominator: -3,
        quotient: 2,
        remainder: -2,
    },
    Case {
        name: "zero-quotient-negative",
        numerator: -1,
        denominator: 2,
        quotient: 0,
        remainder: -1,
    },
    Case {
        name: "zero-quotient-positive-divisor",
        numerator: 1,
        denominator: -2,
        quotient: 0,
        remainder: 1,
    },
    Case {
        name: "zero-over-negative",
        numerator: 0,
        denominator: -3,
        quotient: 0,
        remainder: 0,
    },
];

fn workdir(operation: &str, case: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock moves forward")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "sprint-t14-{}-{}-{}-{}",
        std::process::id(),
        operation,
        case,
        stamp
    ));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn run_candidate(source: &str, asm: &Path) -> Output {
    let source_path = asm
        .parent()
        .expect("assembly path has a parent")
        .join("main.c");
    fs::write(&source_path, source.as_bytes()).expect("source file");
    Command::new(env!("CARGO_BIN_EXE_candidate"))
        .arg("--emit-asm")
        .arg(asm)
        .arg("--interpret-ir")
        .arg(&source_path)
        .output()
        .expect("candidate runs")
}

fn modeled_result(stdout: &[u8]) -> (String, bool) {
    let text = String::from_utf8_lossy(stdout);
    let mut lines = text.lines().filter(|line| line.starts_with("interpret: "));
    let line = lines
        .next()
        .unwrap_or_else(|| panic!("missing interpret line\nstdout: {text}"));
    assert!(lines.next().is_none(), "multiple interpret lines: {text}");
    let rest = line
        .strip_prefix("interpret: value=")
        .unwrap_or_else(|| panic!("malformed interpret line: {line}"));
    let (magnitude, negative) = rest
        .split_once(" negative=")
        .unwrap_or_else(|| panic!("malformed interpret line: {line}"));
    let negative = match negative {
        "true" => true,
        "false" => false,
        other => panic!("malformed negative field `{other}` in: {line}"),
    };
    (magnitude.to_string(), negative)
}

fn assert_modeled(output: &Output, expected: i32, context: &str) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{context}: candidate rejected a defined negative division/remainder\n\
         stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let expected_magnitude = format!("{:02x}", expected.unsigned_abs());
    let expected_negative = expected < 0;
    let (magnitude, negative) = modeled_result(&output.stdout);
    assert_eq!(
        (magnitude.as_str(), negative),
        (expected_magnitude.as_str(), expected_negative),
        "{context}: modeled value differs from hand-derived {expected}\n\
         stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

fn read_assembly(asm: &Path, context: &str) -> String {
    fs::read_to_string(asm)
        .unwrap_or_else(|error| panic!("{context}: cannot read assembly: {error}"))
}

#[test]
fn quotient_truncates_toward_zero_for_negative_operands() {
    for case in CASES {
        // The table itself must satisfy the C identity before it is used.
        assert_eq!(
            case.numerator,
            case.quotient * case.denominator + case.remainder,
            "{}: quotient/remainder table is inconsistent",
            case.name
        );

        let source = format!(
            "int main(void) {{ return {} / {}; }}\n",
            case.numerator, case.denominator
        );
        let dir = workdir("quotient", case.name);
        let asm_path = dir.join("main.s");
        let output = run_candidate(&source, &asm_path);
        assert_modeled(&output, case.quotient, case.name);

        let assembly = read_assembly(&asm_path, case.name);
        assert!(
            assembly.contains("    sdiv w0, w0, w1\n"),
            "{}: quotient must use signed division:\n{assembly}",
            case.name
        );
        assert!(
            !assembly.contains("udiv"),
            "{}: quotient must not use unsigned division:\n{assembly}",
            case.name
        );
    }
}

#[test]
fn remainder_follows_the_dividend_sign_for_negative_operands() {
    for case in CASES {
        let source = format!(
            "int main(void) {{ return {} % {}; }}\n",
            case.numerator, case.denominator
        );
        let dir = workdir("remainder", case.name);
        let asm_path = dir.join("main.s");
        let output = run_candidate(&source, &asm_path);
        assert_modeled(&output, case.remainder, case.name);

        let assembly = read_assembly(&asm_path, case.name);
        assert!(
            assembly.contains("    sdiv w2, w0, w1\n"),
            "{}: remainder must use signed division for its quotient:\n{assembly}",
            case.name
        );
        assert!(
            assembly.contains("    msub w0, w2, w1, w0\n"),
            "{}: remainder must subtract quotient times divisor:\n{assembly}",
            case.name
        );
        assert!(
            !assembly.contains("udiv"),
            "{}: remainder must not use unsigned division:\n{assembly}",
            case.name
        );
    }
}
