// ============================================================================
// sprint_t15.rs — T15 checked arithmetic limits at the modeled-int boundary.
//
// Signed overflow and division/remainder by zero are undefined in C, so no
// expectation here is a native compiler or native-run result. The covered M1
// model defines `int` as signed 32-bit arithmetic, so the successful cases are
// hand-evaluated i32 values and the failing cases must be diagnosed by the
// model instead of silently wrapping.
//
// The unsuffixed `0` spelling is still an octal literal outside the
// decimal-only fold gate at this baseline (`/55`). The zero-divisor cases
// therefore synthesize zero with `x = x - 1` rather than spelling `0`; this
// keeps the assertion on VF12's checked arithmetic instead of the known
// zero-literal limitation.
// ============================================================================

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock moves forward")
        .as_nanos()
}

fn workdir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sprint-t15-{}-{}-{}",
        std::process::id(),
        test,
        unique_suffix()
    ));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn run_candidate(test: &str, source: &str) -> Output {
    let dir = workdir(test);
    let source_path = dir.join("main.c");
    fs::write(&source_path, source.as_bytes()).expect("source file");
    Command::new(binary())
        .arg("--interpret-ir")
        .arg(&source_path)
        .output()
        .expect("candidate runs")
}

fn assert_models(test: &str, source: &str, magnitude: &str, negative: bool) {
    let output = run_candidate(test, source);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{test}\nstdout: {stdout}\nstderr: {stderr}"
    );
    let mut lines = stdout
        .lines()
        .filter(|line| line.starts_with("interpret: "));
    let line = lines
        .next()
        .unwrap_or_else(|| panic!("{test}: missing interpret line\nstdout: {stdout}"));
    assert!(lines.next().is_none(), "{test}: multiple interpret lines");
    let rest = line
        .strip_prefix("interpret: value=")
        .unwrap_or_else(|| panic!("{test}: malformed interpret line `{line}`"));
    let (actual_magnitude, actual_negative) = rest
        .split_once(" negative=")
        .unwrap_or_else(|| panic!("{test}: malformed interpret line `{line}`"));
    assert_eq!(
        (actual_magnitude, actual_negative),
        (magnitude, if negative { "true" } else { "false" }),
        "{test}\nstdout: {stdout}\nstderr: {stderr}"
    );
}

fn assert_model_fails(test: &str, source: &str, expected: &str) {
    let output = run_candidate(test, source);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{test}: expected a typed model failure\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        !stdout.contains("interpret: "),
        "{test}: a failing model must not publish a value\nstdout: {stdout}"
    );
    // The failure must come from the interpreter, not from a native/emitter
    // side path or a silently accepted program.
    assert!(
        stderr.contains("vf12:"),
        "{test}: failure did not originate in the IR model\nstderr: {stderr}"
    );
    assert!(
        stderr.contains(expected),
        "{test}: missing `{expected}` diagnosis\nstderr: {stderr}"
    );
}

#[test]
fn checked_boundaries_model_exact_values() {
    let cases = [
        // 2147483646 + 1 = 2147483647 = 0x7fffffff.
        (
            "int main(void){int x=2147483646;return x+1;}\n",
            "7fffffff",
            false,
        ),
        // -2147483647 - 1 = -2147483648 = -(0x80000000).
        (
            "int main(void){int x=1;x=-2147483647;return x-1;}\n",
            "80000000",
            true,
        ),
        // 46340 * 46340 = 2147395600 = 0x7ffea810, below INT_MAX.
        (
            "int main(void){int x=46340;return x*46340;}\n",
            "7ffea810",
            false,
        ),
        // INT_MIN / 2 truncates toward zero to -1073741824 = -(0x40000000).
        (
            "int main(void){int x=1;x=-2147483647;x=x-1;return x/2;}\n",
            "40000000",
            true,
        ),
    ];
    for (index, &(source, magnitude, negative)) in cases.iter().enumerate() {
        assert_models(&format!("boundary-{index}"), source, magnitude, negative);
    }
}

#[test]
fn add_sub_and_mul_overflow_fail_closed() {
    // INT_MAX + 1 = 2147483648, one past the modeled signed range.
    assert_model_fails(
        "add-overflow",
        "int main(void){int x=2147483647;return x+1;}\n",
        "overflow",
    );
    // -2147483647 - 2 = -2147483649, one below INT_MIN.
    assert_model_fails(
        "sub-underflow",
        "int main(void){int x=1;x=-2147483647;return x-2;}\n",
        "overflow",
    );
    // 46341 * 46341 = 2147488281, one beyond INT_MAX.
    assert_model_fails(
        "mul-overflow",
        "int main(void){int x=46341;return x*46341;}\n",
        "overflow",
    );
}

#[test]
fn computed_zero_divisors_fail_closed() {
    // x is 1, then x - 1 is 0 without spelling a zero literal.
    assert_model_fails(
        "division-by-zero",
        "int main(void){int x=1;x=x-1;return 10/x;}\n",
        "division by zero",
    );
    assert_model_fails(
        "remainder-by-zero",
        "int main(void){int x=1;x=x-1;return 10%x;}\n",
        "remainder by zero",
    );
}

#[test]
fn minimum_int_special_cases_fail_closed() {
    // INT_MIN / -1 and INT_MIN % -1 are the two signed-division overflow
    // cases; INT_MIN's negation is the corresponding unary overflow.
    let build_minimum = "int main(void){int x=1;x=-2147483647;x=x-1;";
    assert_model_fails(
        "minimum-div-minus-one",
        &format!("{build_minimum}return x/-1;}}\n"),
        "overflow",
    );
    assert_model_fails(
        "minimum-rem-minus-one",
        &format!("{build_minimum}return x%-1;}}\n"),
        "overflow",
    );
    assert_model_fails(
        "minimum-negation",
        &format!("{build_minimum}return -x;}}\n"),
        "Neg overflow",
    );
}
