// ============================================================================
// sprint_l04.rs — source zero initializer/arguments, unary minus, native zero.
//
// Every accepted program below has the hand-derived C value 0, so the modeled
// magnitude is `00` with no negative sign and the AArch64 process exit status
// is 0. `-1 + 1` proves a nonzero unary negation runs rather than being
// constant-folded away. Unsuffixed `0` must normalize to decimal radix before
// the shared constant-fold path accepts it; these cases are expected to be RED
// at the baseline until the L01 zero-radix fix is integrated. Expected values
// are derived from the C programs, not from the candidate implementation.
// ============================================================================

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Case {
    name: &'static str,
    source: &'static str,
}

const CASES: &[Case] = &[
    Case {
        name: "zero-return",
        source: "int main(void){return 0;}\n",
    },
    Case {
        name: "zero-initializer",
        source: "int main(void){int x=0;return x;}\n",
    },
    Case {
        name: "zero-arguments",
        source: "int add(int a, int b){return a+b;}\n\
                 int main(void){return add(0,0);}\n",
    },
    Case {
        name: "unary-minus-zero",
        source: "int main(void){return -0;}\n",
    },
    Case {
        name: "unary-minus-cancels",
        source: "int main(void){return -1+1;}\n",
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
        "sprint-l04-{}-{}-{}",
        std::process::id(),
        test,
        unique_suffix()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn run_candidate(source: &str, asm: Option<&Path>) -> Output {
    let dir = match asm {
        Some(path) => path.parent().expect("assembly parent").to_path_buf(),
        None => workdir("source"),
    };
    std::fs::create_dir_all(&dir).expect("source dir");
    let source_path = dir.join("main.c");
    std::fs::write(&source_path, source.as_bytes()).expect("source file");
    let mut command = Command::new(binary());
    if let Some(path) = asm {
        command.arg("--emit-asm").arg(path);
    }
    command
        .arg("--interpret-ir")
        .arg(&source_path)
        .output()
        .expect("candidate runs")
}

fn assert_modeled_zero(output: &Output, context: &str) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{context}: source unexpectedly rejected\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("interpret: value=00 negative=false"),
        "{context}: expected modeled int zero\nstdout: {stdout}"
    );
}

#[test]
fn zero_sources_model_zero_exactly() {
    for case in CASES {
        let dir = workdir(&format!("model-{}", case.name));
        let asm = dir.join("main.s");
        let output = run_candidate(case.source, Some(&asm));
        assert_modeled_zero(&output, case.name);
        assert!(
            asm.exists(),
            "{}: accepted source produced no assembly artifact",
            case.name
        );
    }
}

#[cfg(target_os = "linux")]
fn aarch64_gcc() -> Option<&'static str> {
    let output = Command::new("gcc")
        .arg("-dumpmachine")
        .output()
        .expect("gcc available");
    assert!(
        output.status.success(),
        "gcc -dumpmachine failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let machine = String::from_utf8_lossy(&output.stdout);
    let machine = machine.trim();
    if !machine.starts_with("aarch64") || machine.contains("aarch64_be") {
        eprintln!("SKIP: native zero execution requires an aarch64-linux gcc; found `{machine}`");
        return None;
    }
    Some("gcc")
}

#[cfg(target_os = "linux")]
#[test]
fn zero_sources_exit_zero_on_aarch64() {
    let Some(gcc) = aarch64_gcc() else {
        return;
    };
    for case in CASES {
        let dir = workdir(&format!("native-{}", case.name));
        let asm = dir.join("main.s");
        let exe = dir.join("main");
        let output = run_candidate(case.source, Some(&asm));
        assert_modeled_zero(&output, case.name);

        let build = Command::new(gcc)
            .arg("-o")
            .arg(&exe)
            .arg(&asm)
            .output()
            .expect("gcc runs");
        assert!(
            build.status.success(),
            "{}: gcc failed to assemble/link\nstdout: {}\nstderr: {}",
            case.name,
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );

        let run = Command::new(&exe).output().expect("program runs");
        assert_eq!(
            run.status.code(),
            Some(0),
            "{}: native exit status is not zero\nstdout: {}\nstderr: {}",
            case.name,
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr)
        );
    }
}
