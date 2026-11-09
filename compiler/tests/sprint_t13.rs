// ============================================================================
// sprint_t13.rs — T13 parentheses, unary operators, calls, and precedence.
//
// Independent `/55` source-level regression. Every expected integer below is
// derived by hand from C precedence/associativity and the frozen signed-`int`
// direct-call subset:
//
//   unary +,-  >  * / %  >  + -
//
// Parentheses create no AST node in `/54`; grouping must remain visible only
// through the modeled result and operand order. Calls are primary expressions
// and may therefore sit on either side of a binary operator, inside a unary
// operator, and inside another call's argument list.
//
// No expectation in this file was obtained by running the implementation.
// Native AArch64 execution is out of scope; modeled values and one emitted
// assembly ordering check run on any host.
// ============================================================================

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Bound one candidate invocation so a regression cannot hang the test suite.
const TIMEOUT: Duration = Duration::from_secs(60);

/// Hand-derived arithmetic cases: `(label, return expression, modeled value)`.
const ARITHMETIC_CASES: &[(&str, &str, i32)] = &[
    ("mul-before-add", "2+3*4", 14),
    ("group-before-mul", "(2+3)*4", 20),
    ("sub-left-assoc", "10-3-2", 5),
    ("grouped-sub", "10-(3-2)", 9),
    ("div-left-assoc", "100/10/5", 2),
    ("grouped-div", "100/(10/5)", 50),
    ("both-products", "2*3+4*5", 26),
    ("grouped-product", "2*(3+4)*5", 70),
    ("unary-mul", "-2*3", -6),
    ("unary-grouped-sum", "-(2+3)", -5),
    ("unary-grouped-product-sum", "-(2+3*4)", -14),
    ("nested-unary", "-(-2)", 2),
    ("unary-plus-minus", "+(-2)", -2),
    ("double-unary-group", "-(+(2+3))", -5),
    ("grouped-negative-sum", "(-2+3)*4", 4),
    ("unary-group-then-mul", "-(2+3)*4", -20),
    ("div-group-product", "10/(2+3)*4", 8),
    ("all-grouped-zero", "10/((2+3)*4)", 0),
    ("add-sub-left-assoc", "7-2+3", 8),
    ("grouped-add-sub", "7-(2+3)", 2),
    ("mixed-chain", "2+3*4-5", 9),
    ("redundant-parens", "((2+3))*4", 20),
];

/// Hand-derived call cases over `add`, `sub`, and the zero-parameter `five`.
const CALL_CASES: &[(&str, &str, i32)] = &[
    ("call-control", "add(2,3)", 5),
    ("call-times-literal", "add(2,3)*4", 20),
    ("literal-times-call", "4*add(2,3)", 20),
    ("call-arg-product", "add(2,3*4)", 14),
    ("call-arg-sum", "add(2+3,4)", 9),
    ("call-arg-group-product", "add((2+3)*4,5)", 25),
    ("neg-call", "-add(2,3)", -5),
    ("neg-first-arg", "add(-2,3)", 1),
    ("neg-second-arg", "add(2,-3)", -1),
    ("neg-both-args", "-add(-2,-3)", 5),
    ("grouped-neg-call", "-(add(2,3))", -5),
    ("outer-neg-inner-neg-call", "-(-add(2,3))", 5),
    ("outer-plus-inner-neg-call", "+(-add(2,3))", -5),
    ("unary-arg-group", "add(-(2+3),+(4*5))", 15),
    ("unary-arg-product", "add(2*(3+4),5)", 19),
    ("redundant-call-parens", "(add(2,3))*4", 20),
    ("call-plus-call-times", "add(2,3)+add(4,5)*2", 23),
    ("grouped-calls-times", "(add(2,3)+add(4,5))*2", 28),
    ("sub-call-arg-product", "sub(10,2*3)", 4),
    ("sub-literal-arg-sum", "sub(10,2-3)", 11),
    ("sub-call-minus", "sub(10,2)-3", 5),
    ("neg-sub-call", "-sub(10,2*3)", -4),
    ("nested-call-unary", "sub(-add(2,3),-add(4,5))", 4),
    ("unary-call-arg", "add(-add(2,3),10)", 5),
    ("zero-arg-call", "five()", 5),
    ("neg-zero-arg-call", "-five()", -5),
    ("zero-arg-call-sum", "add(five(),-five())", 0),
    ("zero-arg-call-product", "add(five()*2,-add(2,3))", 5),
    ("nested-zero-arg-unary", "-(add(2,3*five()))", -17),
    ("call-times-neg-call", "add(2,3)*(-five())", -25),
    ("call-minus-neg-call", "add(2,3)-(-five())", 10),
    ("double-neg-zero-arg", "-(-(five()))", 5),
    ("parens-everywhere", "add((five()),(add(1,2)))", 8),
];

/// Invalid or outside-subset parenthesis/call forms; all must fail closed.
const REJECT_CASES: &[(&str, &str)] = &[
    (
        "grouped-callee",
        "int add(int a, int b) { return a + b; }\n\
         int main(void) { return (add)(2, 3); }\n",
    ),
    ("unclosed-group", "int main(void) { return (2 + 3; }\n"),
    (
        "unclosed-call-arg-group",
        "int add(int a, int b) { return a + b; }\n\
         int main(void) { return add(2, (3); }\n",
    ),
    ("empty-group", "int main(void) { return (); }\n"),
    ("extra-close", "int main(void) { return 2 + 3); }\n"),
];

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sprint-t13-{}-{}-{}",
        std::process::id(),
        label,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock moves forward")
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn bounded_output(command: &mut Command, label: &str) -> Output {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .unwrap_or_else(|error| panic!("{label}: candidate failed to spawn: {error}"));
    let deadline = Instant::now() + TIMEOUT;
    loop {
        match child.try_wait().expect("candidate wait") {
            Some(_) => break,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("{label}: candidate exceeded {TIMEOUT:?}");
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    child
        .wait_with_output()
        .unwrap_or_else(|error| panic!("{label}: candidate output: {error}"))
}

fn invoke(source_text: &str, label: &str) -> (Output, PathBuf) {
    let dir = workdir(label);
    let source = dir.join("main.c");
    let asm = dir.join("main.s");
    fs::write(&source, source_text.as_bytes()).expect("source file");
    let mut command = Command::new(binary());
    command
        .arg("--emit-asm")
        .arg(&asm)
        .arg("--interpret-ir")
        .arg(&source);
    (bounded_output(&mut command, label), asm)
}

fn read_asm(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read emitted assembly {}: {error}", path.display()))
}

fn arithmetic_program(expression: &str) -> String {
    format!("int main(void) {{ return {expression}; }}\n")
}

fn call_program(expression: &str) -> String {
    format!(
        "int add(int a, int b) {{ return a + b; }}\n\
         int sub(int a, int b) {{ return a - b; }}\n\
         int five(void) {{ return 2 + 3; }}\n\
         int main(void) {{ return {expression}; }}\n"
    )
}

fn expected_result(value: i32) -> (String, bool) {
    (format!("{:02x}", value.unsigned_abs()), value < 0)
}

fn modeled_result(output: &Output, label: &str) -> (String, bool) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{label}: expected success\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    let mut lines = text.lines().filter(|line| line.starts_with("interpret: "));
    let line = lines
        .next()
        .unwrap_or_else(|| panic!("{label}: missing interpret line\nstdout: {text}"));
    assert!(
        lines.next().is_none(),
        "{label}: multiple interpret lines\nstdout: {text}"
    );
    let rest = line
        .strip_prefix("interpret: value=")
        .unwrap_or_else(|| panic!("{label}: malformed interpret line: {line}"));
    let (magnitude, negative) = rest
        .split_once(" negative=")
        .unwrap_or_else(|| panic!("{label}: malformed interpret line: {line}"));
    assert!(
        negative == "true" || negative == "false",
        "{label}: malformed negative flag: {line}"
    );
    (magnitude.to_string(), negative == "true")
}

/// Require a successful model of `expected` and return the emitted assembly.
fn run_model(label: &str, source_text: &str, expected: i32) -> String {
    let (output, asm) = invoke(source_text, label);
    let actual = modeled_result(&output, label);
    let expected_pair = expected_result(expected);
    assert_eq!(
        actual,
        expected_pair,
        "{label}: modeled value mismatch for source:\n{source_text}\nstdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        asm.is_file(),
        "{label}: missing emitted assembly {}",
        asm.display()
    );
    read_asm(&asm)
}

#[test]
fn arithmetic_parentheses_unary_and_precedence_match_hand_values() {
    for &(label, expression, expected) in ARITHMETIC_CASES {
        let source = arithmetic_program(expression);
        let _ = run_model(label, &source, expected);
    }
}

#[test]
fn call_parentheses_unary_and_precedence_match_hand_values() {
    for &(label, expression, expected) in CALL_CASES {
        let source = call_program(expression);
        let _ = run_model(label, &source, expected);
    }
}

#[test]
fn unary_call_result_is_negated_after_the_direct_call() {
    let source = call_program("-add(2, 3)");
    let assembly = run_model("emit-unary-call", &source, -5);
    let call = assembly
        .find("bl add\n")
        .unwrap_or_else(|| panic!("missing `bl add` in:\n{assembly}"));
    let neg = assembly
        .find("neg w0, w0\n")
        .unwrap_or_else(|| panic!("missing `neg w0, w0` in:\n{assembly}"));
    assert!(
        call < neg,
        "the negation must consume the call result, not precede the call:\n{assembly}"
    );
}

#[test]
fn malformed_parenthesis_and_callee_forms_fail_without_artifacts() {
    for &(label, source_text) in REJECT_CASES {
        let (output, asm) = invoke(source_text, label);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{label}: source unexpectedly accepted\nstdout: {stdout}\nstderr: {stderr}"
        );
        assert!(
            stdout.trim().is_empty(),
            "{label}: rejected source published evidence: {stdout}"
        );
        assert!(
            stderr.starts_with("candidate: "),
            "{label}: rejection must use the candidate stderr channel: {stderr}"
        );
        assert!(
            stderr.contains("candidate diagnostic: "),
            "{label}: rejection must be a structured diagnostic: {stderr}"
        );
        assert!(
            !asm.exists(),
            "{label}: rejected source left an assembly artifact at {}",
            asm.display()
        );
    }
}
