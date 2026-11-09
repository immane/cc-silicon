// ============================================================================
// sprint_t10.rs — T10 adversarial regression: parameter/local values survive
// callee execution and repeated invocations.
//
// `/55` source slice. Every expected value below is hand-derived from the
// modeled signed-`int` subset; no expectation is read back from the
// implementation under test. The cases cover:
//   * a caller parameter read before and after a direct call;
//   * a caller local used as an argument and again after return;
//   * the same callee invoked twice with distinct parameter values;
//   * a callee local mutated then reinitialized on each invocation;
//   * a callee parameter and local read after a nested call;
//   * a caller parameter and callee parameter with the same spelling;
//   * a caller local and callee local with the same spelling.
// Assembly checks pin one direct-call site per source-level invocation and
// confirm every declared function label is emitted. Native execution is not
// claimed here; the modeled result is the primary assertion.
// ============================================================================

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_WORKDIR: AtomicU64 = AtomicU64::new(0);

struct Case {
    name: &'static str,
    source: &'static str,
    /// Hand-derived magnitude of the modeled signed integer.
    magnitude: u32,
    negative: bool,
    /// Expected static direct-call sites per callee, by callee name.
    calls: &'static [(&'static str, usize)],
}

const CASES: &[Case] = &[
    Case {
        name: "caller-parameter-before-and-after-call",
        // inc(3) = 4; 2 * 10 + 4 + 2 = 26 = 0x1a.
        source: "int inc(int x) { return x + 1; }\n\
int mix(int a, int b) { return a * 10 + inc(b) + a; }\n\
int main(void) { return mix(2, 3); }\n",
        magnitude: 0x1a,
        negative: false,
        calls: &[("inc", 1), ("mix", 1)],
    },
    Case {
        name: "caller-local-as-argument-and-after-call",
        // inc(7) = 8; 8 * 7 = 56 = 0x38.
        source: "int inc(int x) { return x + 1; }\n\
int main(void) { int x = 7; return inc(x) * x; }\n",
        magnitude: 0x38,
        negative: false,
        calls: &[("inc", 1)],
    },
    Case {
        name: "repeated-invocation-rebinds-parameters",
        // pair(1, 2) = 12; pair(3, 4) = 34; 12 + 34 = 46 = 0x2e.
        source: "int pair(int a, int b) { return a * 10 + b; }\n\
int main(void) { return pair(1, 2) + pair(3, 4); }\n",
        magnitude: 0x2e,
        negative: false,
        calls: &[("pair", 2)],
    },
    Case {
        name: "callee-local-fresh-on-each-invocation",
        // bump(2) = (1 + 2) = 3; bump(3) = (1 + 3) = 4; 3 * 10 + 4 = 34.
        source: "int bump(int x) { int y = 1; y = y + x; return y; }\n\
int main(void) { return bump(2) * 10 + bump(3); }\n",
        magnitude: 0x22,
        negative: false,
        calls: &[("bump", 2)],
    },
    Case {
        name: "callee-parameter-and-local-survive-nested-call",
        // wrap(2): y = 10 + inc(2) = 13, 13 * 10 + 2 = 132.
        // wrap(3): y = 10 + inc(3) = 14, 14 * 10 + 3 = 143.
        // 132 + 143 = 275 = 0x113.
        source: "int inc(int x) { return x + 1; }\n\
int wrap(int x) { int y = 10; y = y + inc(x); return y * 10 + x; }\n\
int main(void) { return wrap(2) + wrap(3); }\n",
        magnitude: 0x113,
        negative: false,
        calls: &[("inc", 1), ("wrap", 2)],
    },
    Case {
        name: "same-parameter-spelling-across-frames",
        // inc(5) = 6; inc(6) = 7 = 0x07.
        source: "int inc(int x) { return x + 1; }\n\
int main(void) { int x = 5; x = inc(x); x = inc(x); return x; }\n",
        magnitude: 0x07,
        negative: false,
        calls: &[("inc", 2)],
    },
    Case {
        name: "same-local-spelling-across-frames",
        // wrap(2): y = 10 + 2 = 12; caller y = 3 + 12 = 15 = 0x0f.
        source: "int wrap(int x) { int y = 10; y = y + x; return y; }\n\
int main(void) { int y = 3; y = y + wrap(2); return y; }\n",
        magnitude: 0x0f,
        negative: false,
        calls: &[("wrap", 1)],
    },
];

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir(case: &str) -> PathBuf {
    let serial = NEXT_WORKDIR.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "sprint-t10-{}-{}-{}-{}",
        std::process::id(),
        case,
        serial,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock moves forward")
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn run_candidate(source: &str, asm: &Path) -> std::process::Output {
    let dir = asm.parent().expect("assembly parent");
    let source_path = dir.join("main.c");
    fs::write(&source_path, source.as_bytes()).expect("source file");
    Command::new(binary())
        .arg("--interpret-ir")
        .arg("--emit-asm")
        .arg(asm)
        .arg(&source_path)
        .output()
        .expect("candidate runs")
}

/// Parse the single modeled-result line into its numeric magnitude and sign.
fn modeled_result(stdout: &[u8]) -> (u32, bool) {
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
    let magnitude = u32::from_str_radix(magnitude, 16)
        .unwrap_or_else(|error| panic!("non-hex magnitude `{magnitude}`: {error}"));
    let negative = match negative {
        "true" => true,
        "false" => false,
        other => panic!("malformed sign flag `{other}` in: {line}"),
    };
    (magnitude, negative)
}

fn has_function_label(assembly: &str, name: &str) -> bool {
    let label = format!("{name}:");
    assembly.lines().any(|line| line.trim() == label)
}

fn count_direct_calls(assembly: &str, callee: &str) -> usize {
    assembly
        .lines()
        .filter(|line| {
            let mut tokens = line.split_whitespace();
            tokens.next() == Some("bl") && tokens.next() == Some(callee)
        })
        .count()
}

fn count_all_direct_calls(assembly: &str) -> usize {
    assembly
        .lines()
        .filter(|line| line.split_whitespace().next() == Some("bl"))
        .count()
}

#[test]
fn parameter_and_local_values_survive_callee_execution() {
    for case in CASES {
        let dir = workdir(case.name);
        let asm = dir.join("main.s");
        let output = run_candidate(case.source, &asm);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}: candidate rejected the source\nstdout: {}\nstderr: {}",
            case.name,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            modeled_result(&output.stdout),
            (case.magnitude, case.negative),
            "{}: modeled value disagrees with the hand-derived expectation\nstdout: {}",
            case.name,
            String::from_utf8_lossy(&output.stdout)
        );

        let assembly = fs::read_to_string(&asm).unwrap_or_else(|error| {
            panic!(
                "{}: cannot read emitted assembly {}: {error}",
                case.name,
                asm.display()
            )
        });
        assert!(
            has_function_label(&assembly, "main"),
            "{}: missing main label in:\n{assembly}",
            case.name
        );
        let mut expected_total = 0usize;
        for &(callee, expected) in case.calls {
            assert!(
                has_function_label(&assembly, callee),
                "{}: missing `{callee}` label in:\n{assembly}",
                case.name
            );
            assert_eq!(
                count_direct_calls(&assembly, callee),
                expected,
                "{}: wrong number of `bl {callee}` sites; each source-level \
                 invocation needs its own call site\n{assembly}",
                case.name
            );
            expected_total += expected;
        }
        assert_eq!(
            count_all_direct_calls(&assembly),
            expected_total,
            "{}: unexpected direct-call sites in:\n{assembly}",
            case.name
        );
    }
}
