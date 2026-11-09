// ============================================================================
// sprint_t02.rs — `/55` adversarial argument-position regression.
//
// Focus: eight arguments, distinct positional values, permutations of those
// values, and repeated values. The callee is deliberately non-commutative:
//
//     mix(a,b,c,d,e,f,g,h) = a - b + c - d + e - f + g - h
//
// Every expected value below is derived by hand from that expression. The
// test observes the source driver's modeled result and the emitted AArch64
// argument/parameter register order; it does not obtain expectations from the
// implementation under test and it adds no source-level feature.
// ============================================================================

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct Case {
    name: &'static str,
    args: [i32; 8],
    /// Hand calculation of `a - b + c - d + e - f + g - h`.
    expected: i32,
}

const CASES: &[Case] = &[
    // 1-2+3-4+5-6+7-8 = -4
    Case {
        name: "distinct-ascending",
        args: [1, 2, 3, 4, 5, 6, 7, 8],
        expected: -4,
    },
    // 8-7+6-5+4-3+2-1 = 4
    Case {
        name: "distinct-descending",
        args: [8, 7, 6, 5, 4, 3, 2, 1],
        expected: 4,
    },
    // 2-8+1-6+4-5+3-7 = -16
    Case {
        name: "distinct-permuted",
        args: [2, 8, 1, 6, 4, 5, 3, 7],
        expected: -16,
    },
    // 9-1+9-2+9-3+9-4 = 26
    Case {
        name: "repeated-forward",
        args: [9, 1, 9, 2, 9, 3, 9, 4],
        expected: 26,
    },
    // 4-9+3-9+2-9+1-9 = -26
    Case {
        name: "repeated-reversed",
        args: [4, 9, 3, 9, 2, 9, 1, 9],
        expected: -26,
    },
    // 5-5+5-5+5-5+5-4 = 1
    Case {
        name: "repeated-last-differs",
        args: [5, 5, 5, 5, 5, 5, 5, 4],
        expected: 1,
    },
];

fn source_for(args: [i32; 8]) -> String {
    format!(
        "int mix(int a, int b, int c, int d, int e, int f, int g, int h) {{ return a - b + c - d + e - f + g - h; }}\n\
         int main(void) {{ return mix({}, {}, {}, {}, {}, {}, {}, {}); }}\n",
        args[0], args[1], args[2], args[3], args[4], args[5], args[6], args[7]
    )
}

fn workdir(case: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock moves forward")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "sprint-t02-{}-{}-{}",
        std::process::id(),
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

fn normalized_lines(assembly: &str) -> Vec<&str> {
    assembly
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
}

fn function_body<'a>(lines: &[&'a str], name: &str) -> Vec<&'a str> {
    let label = format!("{name}:");
    let start = lines
        .iter()
        .position(|line| *line == label.as_str())
        .unwrap_or_else(|| panic!("missing function label `{label}`"));
    let mut body = Vec::new();
    for line in &lines[start + 1..] {
        if line.ends_with(':') {
            break;
        }
        body.push(*line);
    }
    body
}

fn assert_argument_and_parameter_order(assembly: &str, context: &str) {
    let lines = normalized_lines(assembly);
    let main = function_body(&lines, "main");
    let mut call_sites = Vec::new();
    for (index, line) in main.iter().enumerate() {
        if *line == "bl mix" {
            call_sites.push(index);
        }
    }
    assert_eq!(
        call_sites.len(),
        1,
        "{context}: expected exactly one direct call to `mix`, got {call_sites:?}:\n{assembly}"
    );
    let call_index = call_sites[0];
    assert!(
        call_index >= 8,
        "{context}: call to `mix` has fewer than eight argument loads before it:\n{assembly}"
    );
    for (register, line) in main[call_index - 8..call_index].iter().enumerate() {
        let prefix = format!("ldr w{register}, [sp, #");
        assert!(
            line.starts_with(prefix.as_str()),
            "{context}: argument {register} must load into w{register} before `bl mix`, got `{line}`:\n{assembly}"
        );
    }

    let mix = function_body(&lines, "mix");
    let spills: Vec<&str> = mix
        .iter()
        .copied()
        .filter(|line| line.starts_with("str w") && line.contains("[sp, #"))
        .take(8)
        .collect();
    assert_eq!(
        spills.len(),
        8,
        "{context}: `mix` must spill eight incoming parameters:\n{assembly}"
    );
    for (register, line) in spills.into_iter().enumerate() {
        let prefix = format!("str w{register}, [sp, #");
        assert!(
            line.starts_with(prefix.as_str()),
            "{context}: parameter {register} must be spilled from w{register}, got `{line}`:\n{assembly}"
        );
    }
}

#[test]
fn eight_argument_permutations_and_repeated_values_keep_their_positions() {
    for case in CASES {
        let dir = workdir(case.name);
        let asm_path = dir.join("main.s");
        let source = source_for(case.args);
        let output = run_candidate(&source, &asm_path);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}: candidate rejected a supported eight-argument call\nsource:\n{source}\nstdout: {}\nstderr: {}",
            case.name,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let expected_magnitude = format!("{:02x}", case.expected.unsigned_abs());
        let expected_negative = case.expected < 0;
        let (magnitude, negative) = modeled_result(&output.stdout);
        assert_eq!(
            (magnitude.as_str(), negative),
            (expected_magnitude.as_str(), expected_negative),
            "{}: modeled value differs from the hand-derived {}{}\nsource:\n{source}\nstdout: {}",
            case.name,
            if expected_negative { "-" } else { "" },
            expected_magnitude,
            String::from_utf8_lossy(&output.stdout)
        );

        let assembly = fs::read_to_string(&asm_path)
            .unwrap_or_else(|error| panic!("{}: cannot read assembly: {error}", case.name));
        assert_argument_and_parameter_order(&assembly, case.name);
    }
}
