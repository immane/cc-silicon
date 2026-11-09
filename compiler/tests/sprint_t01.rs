// ============================================================================
// sprint_t01.rs — independent `/55` arity and ordering regression.
//
// T01 focus: zero, one, two, and eight parameters bind to the caller's
// arguments in declaration/source order, and the next parameter or argument
// is rejected. Every expectation below is derived by hand from the frozen
// `/55` direct-call subset; no expected value was obtained by running the
// implementation under test.
//
// The eight-parameter case deliberately declares the names in reverse
// alphabetical order and assigns each position a distinct decimal weight.
// A name-sorted binding, a reversed binding, or a dropped parameter changes
// the modeled value.
// ============================================================================

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct AcceptedCase {
    name: &'static str,
    source: &'static str,
    magnitude_hex: &'static str,
    negative: bool,
}

struct RejectedCase {
    name: &'static str,
    source: &'static str,
    diagnostic: &'static str,
}

const ACCEPTED: &[AcceptedCase] = &[
    AcceptedCase {
        name: "zero-parameters",
        // seven() = 7; 7 + 35 = 42.
        source: "int seven(void) { return 7; }\n\
                 int main(void) { return seven() + 35; }\n",
        magnitude_hex: "2a",
        negative: false,
    },
    AcceptedCase {
        name: "one-parameter",
        // plus_one(41) = 41 + 1 = 42.
        source: "int plus_one(int x) { return x + 1; }\n\
                 int main(void) { return plus_one(41); }\n",
        magnitude_hex: "2a",
        negative: false,
    },
    AcceptedCase {
        name: "two-parameters",
        // subtract(50, 8) = 50 - 8 = 42, not 8 - 50.
        source: "int subtract(int left, int right) { return left - right; }\n\
                 int main(void) { return subtract(50, 8); }\n",
        magnitude_hex: "2a",
        negative: false,
    },
    AcceptedCase {
        name: "eight-parameters",
        // h,g,...,a receive 1,2,...,8 in declaration order:
        // 1 + 2*10 + 3*100 + 4*1000 + 5*10000 + 6*100000
        //   + 7*1000000 + 8*10000000 = 87654321 = 0x05397fb1.
        source: "int digits(int h, int g, int f, int e, int d, int c, int b, int a) {\n\
                 \x20   return h + g * 10 + f * 100 + e * 1000 + d * 10000\n\
                 \x20       + c * 100000 + b * 1000000 + a * 10000000;\n\
                 }\n\
                 int main(void) { return digits(1, 2, 3, 4, 5, 6, 7, 8); }\n",
        magnitude_hex: "05397fb1",
        negative: false,
    },
];

const REJECTED: &[RejectedCase] = &[
    RejectedCase {
        name: "nine-parameters",
        // Nine declared parameters exceed the eight-parameter parser budget;
        // the call supplies only eight arguments so the parameter boundary is
        // the first violated limit.
        source: "int too_many(int a, int b, int c, int d, int e, int f, int g, int h, int i) {\n\
                 \x20   return i;\n\
                 }\n\
                 int main(void) { return too_many(1, 2, 3, 4, 5, 6, 7, 8); }\n",
        diagnostic: "eight-parameter budget",
    },
    RejectedCase {
        name: "nine-arguments",
        // Nine argument expressions exceed the eight-argument parser budget.
        source: "int capped(int a, int b, int c, int d, int e, int f, int g, int h) {\n\
                 \x20   return a;\n\
                 }\n\
                 int main(void) { return capped(1, 2, 3, 4, 5, 6, 7, 8, 9); }\n",
        diagnostic: "eight-argument budget",
    },
];

fn workdir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sprint-t01-{}-{}-{}",
        std::process::id(),
        test,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock moves forward")
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn run_candidate(source: &str, label: &str, asm: Option<&Path>) -> Output {
    let dir = asm
        .and_then(|path| path.parent())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| workdir(label));
    fs::create_dir_all(&dir).expect("source dir");
    let source_path = dir.join("main.c");
    fs::write(&source_path, source.as_bytes()).expect("source file");
    let mut command = Command::new(env!("CARGO_BIN_EXE_candidate"));
    if let Some(path) = asm {
        command.arg("--emit-asm").arg(path);
    }
    command
        .arg("--interpret-ir")
        .arg(&source_path)
        .output()
        .expect("candidate runs")
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
    let negative = match negative {
        "true" => true,
        "false" => false,
        other => panic!("malformed negative field `{other}` in: {line}"),
    };
    (magnitude.to_string(), negative)
}

#[test]
fn parameters_and_arguments_keep_source_order_at_zero_one_two_and_eight() {
    for case in ACCEPTED {
        let output = run_candidate(case.source, case.name, None);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}: supported arity rejected\nsource:\n{}\nstdout: {}\nstderr: {}",
            case.name,
            case.source,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let (magnitude, negative) = modeled_result(&output.stdout);
        assert_eq!(
            (magnitude.as_str(), negative),
            (case.magnitude_hex, case.negative),
            "{}: modeled value differs from the hand-derived 0x{}\nsource:\n{}\nstdout: {}",
            case.name,
            case.magnitude_hex,
            case.source,
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

#[test]
fn ninth_parameter_and_ninth_argument_are_rejected_without_artifacts() {
    for case in REJECTED {
        let dir = workdir(case.name);
        let asm = dir.join("main.s");
        let output = run_candidate(case.source, case.name, Some(&asm));
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert_eq!(
            output.status.code(),
            Some(1),
            "{}: source unexpectedly accepted\nsource:\n{}\nstdout: {}\nstderr: {stderr}",
            case.name,
            case.source,
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(
            stderr.starts_with("candidate: "),
            "{}: rejection must use the candidate stderr channel: {stderr}",
            case.name
        );
        assert!(
            stderr.contains("candidate diagnostic: "),
            "{}: rejection must be a structured diagnostic: {stderr}",
            case.name
        );
        assert!(
            stderr.contains(case.diagnostic),
            "{}: expected the `{}` boundary diagnostic, got: {stderr}",
            case.name,
            case.diagnostic
        );
        assert!(
            !asm.exists(),
            "{}: rejected source left an assembly artifact at {}",
            case.name,
            asm.display()
        );
    }
}
