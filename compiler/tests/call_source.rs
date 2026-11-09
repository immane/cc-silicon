// ============================================================================
// call_source.rs — independent `/55` direct-call source acceptance.
//
// INTEGRATION_PENDING at the `/54` schema baseline: this file compiles, but
// its value and assembly assertions require the `/55` parser, semantic, IR,
// interpreter, and emitter producers. Every expected integer below is derived
// by hand from the modeled signed-`int` subset; no expected value is obtained
// by running the implementation under test.
//
// Native cases are closed-loop checks on a Linux AArch64 host only. They
// verify the host `gcc` target before running, use bounded waits, and print an
// explicit skip when the host is not a suitable AArch64 target. Native process
// exit status is only the low eight bits; the full modeled integer remains the
// primary assertion. These checks are not T00 corpus evidence.
// ============================================================================

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static BOUNDED_RUN_ID: AtomicU64 = AtomicU64::new(0);

const ADD_TWO_THREE: &str = "int add(int a, int b) { return a + b; }\n\
int main(void) { return add(2, 3); }\n";

const ZERO_PARAMETER: &str = "int five(void) { return 2 + 3; }\n\
int main(void) { return five(); }\n";

const TWO_CALLS_IN_EXPRESSION: &str = "int inc(int x) { return x + 1; }\n\
int main(void) { return inc(2) + inc(3); }\n";

const NESTED_CALL_ARGUMENTS: &str = "int inc(int x) { return x + 1; }\n\
int add(int a, int b) { return a + b; }\n\
int main(void) { return add(inc(2), inc(3)); }\n";

const NESTED_CALL_RESULT: &str = "int inc(int x) { return x + 1; }\n\
int main(void) { return inc(inc(2)); }\n";

const NEGATIVE_SUB: &str = "int sub(int a, int b) { return a - b; }\n\
int main(void) { return sub(2, 7); }\n";

const LOCAL_ARGUMENT: &str = "int add(int a, int b) { return a + b; }\n\
int main(void) { int x = 2; return add(x, 3); }\n";

const LOCAL_MUTATED_BY_CALLS: &str = "int inc(int x) { return x + 1; }\n\
int main(void) { int x = 2; x = inc(x); x = inc(x); return x; }\n";

const SAME_PARAMETER_SPELLING: &str = "int inc(int x) { return x + 1; }\n\
int twice(int x) { return x + x; }\n\
int main(void) { return inc(2) + twice(3); }\n";

const REPEATED_LITERAL_CALLS: &str = "int f(int x) { return x + 2; }\n\
int main(void) { return f(2) + f(2); }\n";

const EIGHT_ARGUMENTS: &str = "int mix(int a, int b, int c, int d, int e, int f, int g, int h) {\n\
    return a - b + c - d + e - f + g - h;\n\
}\n\
int main(void) { return mix(1, 2, 3, 4, 5, 6, 7, 8); }\n";

const LIVE_RESULT_ACROSS_CALL: &str = "int add(int a, int b) { return a + b; }\n\
int main(void) { return add(2, 3) + add(4, 5); }\n";

const PARAMETER_SURVIVES_CALL: &str = "int inc(int x) { return x + 1; }\n\
int mix(int a, int b) { return a + inc(b); }\n\
int main(void) { return mix(2, 3); }\n";

const NEGATIVE_DIVISION_REMAINDER: &str = "int idiv(int a, int b) { return a / b; }\n\
int irem(int a, int b) { return a % b; }\n\
int main(void) { return idiv(-7, 2) * 10 + irem(-7, 2); }\n";

const CANONICAL_CONSTANT_RETURN: &str = "int main(void){return 2+3;}\n";

const LOCAL_ASSIGNMENT_REGRESSION: &str = "int main(void){int x=2;x=x+3;return x;}\n";

struct CallCase {
    name: &'static str,
    source: &'static str,
    magnitude_hex: &'static str,
    negative: bool,
    native_exit: i32,
}

/// Hand-derived modeled value and low-eight-bit native exit status.
const CALL_CASES: &[CallCase] = &[
    CallCase {
        name: "add-two-three",
        source: ADD_TWO_THREE,
        magnitude_hex: "05",
        negative: false,
        native_exit: 5,
    },
    CallCase {
        name: "zero-parameter-five",
        source: ZERO_PARAMETER,
        magnitude_hex: "05",
        negative: false,
        native_exit: 5,
    },
    CallCase {
        name: "two-calls-in-expression",
        source: TWO_CALLS_IN_EXPRESSION,
        magnitude_hex: "07",
        negative: false,
        native_exit: 7,
    },
    CallCase {
        name: "nested-call-arguments",
        source: NESTED_CALL_ARGUMENTS,
        magnitude_hex: "07",
        negative: false,
        native_exit: 7,
    },
    CallCase {
        name: "nested-call-result",
        source: NESTED_CALL_RESULT,
        magnitude_hex: "04",
        negative: false,
        native_exit: 4,
    },
    CallCase {
        name: "negative-sub",
        source: NEGATIVE_SUB,
        magnitude_hex: "05",
        negative: true,
        native_exit: 251,
    },
    CallCase {
        name: "local-argument",
        source: LOCAL_ARGUMENT,
        magnitude_hex: "05",
        negative: false,
        native_exit: 5,
    },
    CallCase {
        name: "local-mutated-by-calls",
        source: LOCAL_MUTATED_BY_CALLS,
        magnitude_hex: "04",
        negative: false,
        native_exit: 4,
    },
    CallCase {
        name: "same-parameter-spelling",
        source: SAME_PARAMETER_SPELLING,
        magnitude_hex: "09",
        negative: false,
        native_exit: 9,
    },
    CallCase {
        name: "repeated-literal-calls",
        source: REPEATED_LITERAL_CALLS,
        magnitude_hex: "08",
        negative: false,
        native_exit: 8,
    },
    CallCase {
        name: "eight-arguments",
        source: EIGHT_ARGUMENTS,
        magnitude_hex: "04",
        negative: true,
        native_exit: 252,
    },
    CallCase {
        name: "live-result-across-call",
        source: LIVE_RESULT_ACROSS_CALL,
        magnitude_hex: "0e",
        negative: false,
        native_exit: 14,
    },
    CallCase {
        name: "parameter-survives-call",
        source: PARAMETER_SURVIVES_CALL,
        magnitude_hex: "06",
        negative: false,
        native_exit: 6,
    },
    CallCase {
        name: "negative-division-remainder",
        source: NEGATIVE_DIVISION_REMAINDER,
        magnitude_hex: "1f",
        negative: true,
        native_exit: 225,
    },
];

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
        "call-source-{}-{}-{}",
        std::process::id(),
        test,
        unique_suffix()
    ));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// Wait for a child with a finite deadline. Standard Rust has no built-in
/// command timeout, so output is redirected to files and the child is killed
/// when the bound expires.
fn bounded_output(command: &mut Command, limit: Duration, label: &str) -> Output {
    let serial = BOUNDED_RUN_ID.fetch_add(1, Ordering::Relaxed);
    let base = std::env::temp_dir().join(format!(
        "call-source-bound-{}-{}-{}-{}",
        std::process::id(),
        serial,
        label,
        unique_suffix()
    ));
    let stdout_path = base.with_extension("stdout");
    let stderr_path = base.with_extension("stderr");
    let stdout_file = fs::File::create(&stdout_path).expect("bounded stdout file");
    let stderr_file = fs::File::create(&stderr_path).expect("bounded stderr file");
    command
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));
    let mut child = command
        .spawn()
        .unwrap_or_else(|error| panic!("{label} failed to spawn: {error}"));
    let deadline = Instant::now() + limit;
    let status = loop {
        match child.try_wait().expect("bounded child wait") {
            Some(status) => break status,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                let stdout = fs::read_to_string(&stdout_path).unwrap_or_default();
                let stderr = fs::read_to_string(&stderr_path).unwrap_or_default();
                let _ = fs::remove_file(&stdout_path);
                let _ = fs::remove_file(&stderr_path);
                panic!("{label} exceeded {limit:?}\nstdout: {stdout}\nstderr: {stderr}");
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    let stdout = fs::read(&stdout_path).expect("bounded stdout");
    let stderr = fs::read(&stderr_path).expect("bounded stderr");
    let _ = fs::remove_file(&stdout_path);
    let _ = fs::remove_file(&stderr_path);
    Output {
        status,
        stdout,
        stderr,
    }
}

fn run_candidate(source: &str, asm: Option<&Path>) -> Output {
    let dir = asm
        .and_then(|path| path.parent())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| workdir("source"));
    fs::create_dir_all(&dir).expect("source dir");
    let source_path = dir.join("main.c");
    fs::write(&source_path, source.as_bytes()).expect("source file");
    let mut command = Command::new(binary());
    if let Some(path) = asm {
        command.arg("--emit-asm").arg(path);
    }
    command.arg("--interpret-ir").arg(&source_path);
    bounded_output(&mut command, Duration::from_secs(60), "candidate-source")
}

fn read_asm(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read emitted assembly {}: {error}", path.display()))
}

fn assert_success(output: &Output, context: &str) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{context}\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Parse the single modeled-result line into its exact magnitude and sign.
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
    (magnitude.to_string(), negative == "true")
}

fn assert_modeled(output: &Output, magnitude_hex: &str, negative: bool, context: &str) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{context}\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let (actual_magnitude, actual_negative) = modeled_result(&output.stdout);
    assert_eq!(
        (actual_magnitude.as_str(), actual_negative),
        (magnitude_hex, negative),
        "{context}\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn function_body(assembly: &str, name: &str, function_names: &[&str]) -> String {
    let lines: Vec<&str> = assembly.lines().collect();
    let label = format!("{name}:");
    let start = lines
        .iter()
        .position(|line| line.trim() == label)
        .unwrap_or_else(|| panic!("missing function label `{label}` in:\n{assembly}"));
    let mut body = Vec::new();
    for line in lines.iter().skip(start + 1) {
        let trimmed = line.trim();
        if trimmed.starts_with(".size ") {
            break;
        }
        if let Some(candidate) = trimmed.strip_suffix(':') {
            if function_names.contains(&candidate) {
                break;
            }
        }
        body.push(*line);
    }
    body.join("\n")
}

fn assert_label(assembly: &str, name: &str, context: &str) {
    let label = format!("{name}:");
    assert!(
        assembly.lines().any(|line| line.trim() == label),
        "{context}: missing function label `{label}` in:\n{assembly}"
    );
}

fn assert_direct_call(assembly: &str, callee: &str, context: &str) {
    assert!(
        assembly.lines().any(|line| {
            let mut tokens = line.split_whitespace();
            tokens.next() == Some("bl") && tokens.next() == Some(callee)
        }),
        "{context}: missing direct call `bl {callee}` in:\n{assembly}"
    );
}

fn destination_registers(line: &str) -> Vec<String> {
    let text = line.split("//").next().unwrap_or(line).trim();
    if text.is_empty() || text.starts_with('.') || text.starts_with('#') || text.ends_with(':') {
        return Vec::new();
    }
    let mut parts = text.splitn(2, char::is_whitespace);
    let mnemonic = parts.next().unwrap_or("").to_ascii_lowercase();
    let rest = parts.next().unwrap_or("").trim();
    if matches!(
        mnemonic.as_str(),
        "str"
            | "stp"
            | "bl"
            | "b"
            | "br"
            | "blr"
            | "ret"
            | "cbz"
            | "cbnz"
            | "tbz"
            | "tbnz"
            | "cmp"
            | "cmn"
            | "tst"
            | "nop"
    ) || rest.is_empty()
    {
        return Vec::new();
    }
    let take = if mnemonic == "ldp" { 2 } else { 1 };
    rest.split(',')
        .map(str::trim)
        .take(take)
        .filter(|operand| operand.starts_with('w') || operand.starts_with('x'))
        .map(|operand| operand.trim_start_matches('!').to_ascii_lowercase())
        .collect()
}

fn is_callee_saved(register: &str) -> bool {
    let digits = register
        .strip_prefix('x')
        .or_else(|| register.strip_prefix('w'));
    digits
        .and_then(|digits| digits.parse::<u32>().ok())
        .map(|number| (19..=28).contains(&number))
        .unwrap_or(false)
}

fn stack_adjust_immediate(line: &str, mnemonic: &str) -> Option<u32> {
    let text = line.split("//").next().unwrap_or(line).trim();
    let compact: String = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    let prefix = format!("{mnemonic}sp,sp,#");
    compact.strip_prefix(&prefix)?.parse::<u32>().ok()
}

fn assert_argument_register_setup(
    assembly: &str,
    caller: &str,
    callee: &str,
    argument_count: usize,
    function_names: &[&str],
    context: &str,
) {
    let body = function_body(assembly, caller, function_names);
    let lines: Vec<&str> = body.lines().collect();
    let call_prefix = format!("bl {callee}");
    let call_index = lines
        .iter()
        .position(|line| {
            let mut tokens = line.split_whitespace();
            tokens.next() == Some("bl") && tokens.next() == Some(callee)
        })
        .unwrap_or_else(|| panic!("{context}: `{caller}` does not call `{callee}`:\n{body}"));
    let mut written = std::collections::BTreeSet::new();
    for line in &lines[..call_index] {
        for register in destination_registers(line) {
            written.insert(register);
        }
    }
    for index in 0..argument_count {
        let register = format!("w{index}");
        assert!(
            written.contains(&register),
            "{context}: call to `{callee}` does not set `{register}` before `{call_prefix}`:\n{body}"
        );
    }
}

fn assert_aapcs64_frame_shape(assembly: &str, function_names: &[&str], context: &str) {
    for &name in function_names {
        let body = function_body(assembly, name, function_names);
        for line in body.lines() {
            for register in destination_registers(line) {
                assert!(
                    !is_callee_saved(&register),
                    "{context}: `{name}` writes callee-saved register `{register}`: {line}\n{body}"
                );
            }
        }
        let mut saw_aligned_frame = false;
        for line in body.lines() {
            if let Some(immediate) = stack_adjust_immediate(line, "sub") {
                saw_aligned_frame = true;
                assert_eq!(
                    immediate % 16,
                    0,
                    "{context}: `{name}` uses a non-16-byte-aligned frame: {line}"
                );
            }
        }
        let non_leaf = body
            .lines()
            .any(|line| line.split_whitespace().next() == Some("bl"));
        assert!(
            body.lines()
                .any(|line| line.split_whitespace().next() == Some("ret")),
            "{context}: `{name}` has no return instruction:\n{body}"
        );
        if non_leaf {
            assert!(
                saw_aligned_frame,
                "{context}: non-leaf `{name}` does not reserve a 16-byte-aligned stack frame:\n{body}"
            );
            let lr_mentions = body.matches("x30").count();
            assert!(
                lr_mentions >= 2,
                "{context}: non-leaf `{name}` does not save and restore LR (`x30` occurrences: {lr_mentions}):\n{body}"
            );
        }
    }
}

#[test]
fn models_direct_call_values_exactly() {
    for case in CALL_CASES {
        let magnitude = u32::from_str_radix(case.magnitude_hex, 16).expect("hex magnitude");
        let expected_low_byte = if case.negative {
            (256 - (magnitude % 256)) % 256
        } else {
            magnitude % 256
        };
        assert_eq!(
            case.native_exit as u32, expected_low_byte,
            "{}: native exit table disagrees with the hand-derived modeled value",
            case.name
        );
        let dir = workdir(case.name);
        let asm = dir.join("main.s");
        let output = run_candidate(case.source, Some(&asm));
        assert_modeled(&output, case.magnitude_hex, case.negative, case.name);
        assert!(
            asm.is_file(),
            "{}: missing emitted assembly {}",
            case.name,
            asm.display()
        );
    }
}

#[test]
fn emits_multi_function_labels_direct_calls_and_argument_registers() {
    let dir = workdir("call-shape");
    let asm = dir.join("main.s");
    let output = run_candidate(EIGHT_ARGUMENTS, Some(&asm));
    assert_success(&output, "eight-argument assembly");
    let assembly = read_asm(&asm);
    assert_label(&assembly, "main", "eight-argument assembly");
    assert_label(&assembly, "mix", "eight-argument assembly");
    assert_direct_call(&assembly, "mix", "eight-argument assembly");
    assert_argument_register_setup(
        &assembly,
        "main",
        "mix",
        8,
        &["mix", "main"],
        "eight-argument assembly",
    );
    assert_aapcs64_frame_shape(&assembly, &["mix", "main"], "eight-argument assembly");
}

#[test]
fn nested_and_repeated_calls_use_distinct_direct_call_sites() {
    let dir = workdir("nested-call-shape");
    let asm = dir.join("main.s");
    let output = run_candidate(NESTED_CALL_ARGUMENTS, Some(&asm));
    assert_success(&output, "nested call assembly");
    let assembly = read_asm(&asm);
    for name in ["inc", "add", "main"] {
        assert_label(&assembly, name, "nested call assembly");
    }
    assert_direct_call(&assembly, "inc", "nested call assembly");
    assert_direct_call(&assembly, "add", "nested call assembly");
    assert_aapcs64_frame_shape(&assembly, &["inc", "add", "main"], "nested call assembly");
}

#[test]
fn non_leaf_calls_save_lr_and_keep_frames_aligned() {
    let dir = workdir("frame-shape");
    let asm = dir.join("main.s");
    let output = run_candidate(ADD_TWO_THREE, Some(&asm));
    assert_success(&output, "frame assembly");
    let assembly = read_asm(&asm);
    let main = function_body(&assembly, "main", &["add", "main"]);
    let call_count = main
        .lines()
        .filter(|line| {
            let mut tokens = line.split_whitespace();
            tokens.next() == Some("bl") && tokens.next() == Some("add")
        })
        .count();
    assert_eq!(call_count, 1, "expected one call to add:\n{main}");
    assert!(
        main.matches("x30").count() >= 2,
        "non-leaf main must save and restore LR:\n{main}"
    );
    assert!(
        main.lines()
            .any(|line| stack_adjust_immediate(line, "sub").is_some()),
        "non-leaf main must reserve an aligned stack frame:\n{main}"
    );
    assert_aapcs64_frame_shape(&assembly, &["add", "main"], "frame assembly");
}

#[test]
fn canonical_and_local_assignment_regressions_are_preserved() {
    let constant_dir = workdir("canonical-regression");
    let constant_asm = constant_dir.join("main.s");
    let constant = run_candidate(CANONICAL_CONSTANT_RETURN, Some(&constant_asm));
    assert_modeled(&constant, "05", false, "canonical constant return");
    let constant_text = read_asm(&constant_asm);
    assert!(
        constant_text.contains("movz w0, #5"),
        "canonical return must materialize 5:\n{constant_text}"
    );
    assert!(
        !constant_text
            .lines()
            .any(|line| line.split_whitespace().next() == Some("bl")),
        "canonical return must not emit a call:\n{constant_text}"
    );
    assert!(
        !constant_text
            .lines()
            .any(|line| stack_adjust_immediate(line, "sub").is_some()),
        "canonical return must remain frame-free:\n{constant_text}"
    );

    let local_dir = workdir("local-assignment-regression");
    let local_asm = local_dir.join("main.s");
    let local = run_candidate(LOCAL_ASSIGNMENT_REGRESSION, Some(&local_asm));
    assert_modeled(&local, "05", false, "/53 local assignment");
    let local_text = read_asm(&local_asm);
    assert!(
        local_text.contains("str w0, [sp")
            && local_text
                .lines()
                .any(|line| line.trim_start().starts_with("ldr w0, [sp")),
        "/53 local assignment must still load and store the local:\n{local_text}"
    );
    assert!(
        !local_text
            .lines()
            .any(|line| line.split_whitespace().next() == Some("bl")),
        "/53 local assignment must not emit a call:\n{local_text}"
    );
}

const REJECTED_SOURCES: &[(&str, &str)] = &[
    (
        "nine-parameters",
        "int nine(int a, int b, int c, int d, int e, int f, int g, int h, int i) { return a; }\n\
int main(void) { return nine(1, 2, 3, 4, 5, 6, 7, 8, 9); }\n",
    ),
    (
        "nine-arguments",
        "int eight(int a, int b, int c, int d, int e, int f, int g, int h) {\n\
    return a + b + c + d + e + f + g + h;\n\
}\n\
int main(void) { return eight(1, 2, 3, 4, 5, 6, 7, 8, 9); }\n",
    ),
    ("unknown-callee", "int main(void) { return missing(2); }\n"),
    (
        "forward-callee",
        "int main(void) { return later(2); }\n\
int later(int x) { return x + 1; }\n",
    ),
    (
        "duplicate-parameter",
        "int duplicate(int x, int x) { return x; }\n\
int main(void) { return duplicate(1, 2); }\n",
    ),
    (
        "duplicate-function",
        "int duplicate(void) { return 1; }\n\
int duplicate(void) { return 2; }\n\
int main(void) { return duplicate(); }\n",
    ),
    ("call-to-main", "int main(void) { return main(); }\n"),
    (
        "recursion",
        "int recurse(int x) { return recurse(x); }\n\
int main(void) { return recurse(1); }\n",
    ),
    (
        "malformed-empty-argument",
        "int f(int x) { return x; }\n\
int main(void) { return f(, 2); }\n",
    ),
    (
        "malformed-missing-comma",
        "int f(int a, int b) { return a + b; }\n\
int main(void) { return f(2 3); }\n",
    ),
    (
        "malformed-missing-close",
        "int f(int x) { return x; }\n\
int main(void) { return f(2; }\n",
    ),
    (
        "empty-parameter-parentheses",
        "int f() { return 1; }\n\
int main(void) { return f(); }\n",
    ),
    (
        "non-int-parameter-char",
        "int f(char x) { return x; }\n\
int main(void) { return f(1); }\n",
    ),
    (
        "non-int-parameter-long",
        "int f(long x) { return x; }\n\
int main(void) { return f(1); }\n",
    ),
    (
        "prototype-only",
        "int f(int x);\n\
int main(void) { return f(2); }\n",
    ),
    (
        "prototype-and-definition",
        "int f(int x);\n\
int f(int x) { return x; }\n\
int main(void) { return f(2); }\n",
    ),
];

#[test]
fn rejects_unsupported_call_sources_without_artifacts() {
    for (index, (name, source)) in REJECTED_SOURCES.iter().enumerate() {
        let dir = workdir(&format!("reject-{name}"));
        let asm = dir.join(format!("rejected-{index}.s"));
        let output = run_candidate(source, Some(&asm));
        assert_eq!(
            output.status.code(),
            Some(1),
            "{name}: source unexpectedly accepted\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.starts_with("candidate: "),
            "{name}: rejection must use the candidate stderr channel: {stderr}"
        );
        assert!(
            stderr.contains("candidate diagnostic: "),
            "{name}: rejection must be a structured diagnostic: {stderr}"
        );
        assert!(
            !asm.exists(),
            "{name}: rejected source left an assembly artifact at {}",
            asm.display()
        );
    }
}

#[test]
fn identical_call_source_replays_identical_model_and_assembly() {
    let dir = workdir("replay");
    let first_asm = dir.join("first.s");
    let second_asm = dir.join("second.s");
    let first = run_candidate(NESTED_CALL_ARGUMENTS, Some(&first_asm));
    let second = run_candidate(NESTED_CALL_ARGUMENTS, Some(&second_asm));
    assert_modeled(&first, "07", false, "replay first run");
    assert_modeled(&second, "07", false, "replay second run");
    let first_bytes = fs::read(&first_asm).expect("first assembly");
    let second_bytes = fs::read(&second_asm).expect("second assembly");
    assert_eq!(
        first_bytes, second_bytes,
        "identical source produced different assembly bytes"
    );
}

#[cfg(target_os = "linux")]
fn aarch64_gcc() -> Option<&'static str> {
    let mut command = Command::new("gcc");
    command.arg("-dumpmachine");
    let output = bounded_output(&mut command, Duration::from_secs(15), "gcc-dumpmachine");
    assert!(
        output.status.success(),
        "gcc -dumpmachine failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let machine = String::from_utf8_lossy(&output.stdout);
    let machine = machine.trim();
    if !machine.starts_with("aarch64") || machine.contains("aarch64_be") {
        eprintln!(
            "SKIP: native AArch64 execution requires an aarch64-linux gcc; found `{machine}`"
        );
        return None;
    }
    Some("gcc")
}

#[cfg(target_os = "linux")]
#[test]
fn call_results_execute_on_aarch64() {
    let Some(gcc) = aarch64_gcc() else {
        return;
    };
    for case in CALL_CASES {
        let dir = workdir(&format!("native-{}", case.name));
        let asm = dir.join("main.s");
        let exe = dir.join("main");
        let output = run_candidate(case.source, Some(&asm));
        assert_modeled(&output, case.magnitude_hex, case.negative, case.name);

        let mut build = Command::new(gcc);
        build.arg("-o").arg(&exe).arg(&asm);
        let build = bounded_output(&mut build, Duration::from_secs(60), "gcc-link");
        assert!(
            build.status.success(),
            "{}: gcc failed to assemble/link\nstdout: {}\nstderr: {}",
            case.name,
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );

        let mut run = Command::new(&exe);
        let run = bounded_output(&mut run, Duration::from_secs(30), "native-run");
        assert_eq!(
            run.status.code(),
            Some(case.native_exit),
            "{}: native exit status differs (modeled {}{} expected low byte {})\nstdout: {}\nstderr: {}",
            case.name,
            if case.negative { "-" } else { "" },
            case.magnitude_hex,
            case.native_exit,
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr)
        );
    }
}
