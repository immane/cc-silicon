// ============================================================================
// sprint_t30.rs — T30 canonical and `/49`–`/53` compatibility plus the
// negative artifact contract.
//
// Independent end-to-end coverage through the `candidate` CLI; no shared
// helpers with existing suites. The success matrix covers the frozen old
// source slices: canonical constant return, `/49` local return, `/50` local
// add, `/51` operators, `/52` expression trees, and `/53` assignments. Every
// expected integer is computed by hand from the source. The negative matrix
// covers parser, semantic, and VF12 model rejection: each must exit 1 with a
// structured diagnostic and leave neither the requested assembly nor the
// requested evidence report behind.
// ============================================================================

use std::path::Path;
use std::process::{Command, Output};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir(test: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sprint-t30-{}-{}-{}",
        std::process::id(),
        test,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock moves forward")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn run_source(dir: &Path, source: &str, asm: Option<&Path>, report: Option<&Path>) -> Output {
    let source_path = dir.join("main.c");
    std::fs::write(&source_path, source.as_bytes()).expect("source write");
    let mut command = Command::new(binary());
    if let Some(path) = asm {
        command.arg("--emit-asm").arg(path);
    }
    if let Some(path) = report {
        command.arg("-o").arg(path);
    }
    command
        .arg("--interpret-ir")
        .arg(&source_path)
        .output()
        .expect("candidate runs")
}

/// `(name, source, modeled line, needs frame, distinguishing instruction)`.
const COMPATIBILITY: &[(&str, &str, &str, bool, &str)] = &[
    (
        "canonical",
        "int main(void){return 2+3;}\n",
        "value=05 negative=false",
        false,
        "movz w0, #5",
    ),
    (
        "49-local-return",
        "int main(void){int x=2;return x;}\n",
        "value=02 negative=false",
        true,
        "ldr w0, [sp, #0]",
    ),
    (
        "50-local-add",
        "int main(void){int x=2;return x+3;}\n",
        "value=05 negative=false",
        true,
        "add w0, w0, w1",
    ),
    (
        "51-local-sub",
        "int main(void){int x=2;return x-3;}\n",
        "value=01 negative=true",
        true,
        "sub w0, w0, w1",
    ),
    (
        "51-local-mul",
        "int main(void){int x=2;return x*3;}\n",
        "value=06 negative=false",
        true,
        "mul w0, w0, w1",
    ),
    (
        "51-local-div",
        "int main(void){int x=2;return x/3;}\n",
        "value=00 negative=false",
        true,
        "sdiv w0, w0, w1",
    ),
    (
        "51-local-rem",
        "int main(void){int x=2;return x%3;}\n",
        "value=02 negative=false",
        true,
        "msub w0, w2, w1, w0",
    ),
    (
        "52-precedence",
        "int main(void){int x=2;return x+3*2;}\n",
        "value=08 negative=false",
        true,
        "mul w0, w0, w1",
    ),
    (
        "52-parentheses",
        "int main(void){int x=2;return (x+3)*2;}\n",
        "value=0a negative=false",
        true,
        "add w0, w0, w1",
    ),
    (
        "52-literal-left",
        "int main(void){int x=2;return 10/x;}\n",
        "value=05 negative=false",
        true,
        "sdiv w0, w0, w1",
    ),
    (
        "52-unary-minus",
        "int main(void){int x=2;return -x;}\n",
        "value=02 negative=true",
        true,
        "neg w0, w0",
    ),
    (
        "52-unary-plus",
        "int main(void){int x=2;return +(x+1);}\n",
        "value=03 negative=false",
        true,
        "add w0, w0, w1",
    ),
    (
        "53-one-assignment",
        "int main(void){int x=2;x=x+3;return x;}\n",
        "value=05 negative=false",
        true,
        "add w0, w0, w1",
    ),
    (
        "53-two-assignments",
        "int main(void){int x=2;x=x+3;x=x*2;return x;}\n",
        "value=0a negative=false",
        true,
        "str w0, [sp, #0]",
    ),
];

#[test]
fn canonical_and_local_slices_remain_compatible() {
    for &(name, source, expected, framed, marker) in COMPATIBILITY {
        let dir = workdir(name);
        let asm = dir.join("main.s");
        let output = run_source(&dir, source, Some(asm.as_path()), None);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{name}: source rejected\nstdout: {stdout}\nstderr: {stderr}"
        );
        assert!(stderr.is_empty(), "{name}: unexpected stderr: {stderr}");
        assert!(
            stdout.contains("candidate evidence (t01-c01-c06/55)"),
            "{name}: evidence header is not the frozen /55 marker: {stdout}"
        );
        assert!(
            stdout.contains(expected),
            "{name}: expected modeled `{expected}` in: {stdout}"
        );

        let assembly = std::fs::read_to_string(&asm).unwrap_or_else(|error| {
            panic!(
                "{name}: cannot read emitted assembly {}: {error}",
                asm.display()
            )
        });
        for required in [
            ".globl main\n",
            "\nmain:\n",
            "    ret\n",
            ".size main, .-main\n",
        ] {
            assert!(
                assembly.contains(required),
                "{name}: missing `{required}` in assembly:\n{assembly}"
            );
        }
        assert!(
            assembly.contains(marker),
            "{name}: missing `{marker}` in assembly:\n{assembly}"
        );
        assert_eq!(
            assembly.contains("sub sp, sp"),
            framed,
            "{name}: unexpected frame shape:\n{assembly}"
        );
    }
}

/// `(name, source)` — rejected in the parser, semantic checker, or VF12 model.
const REJECTED: &[(&str, &str)] = &[
    ("unknown-identifier", "int main(void){return y;}\n"),
    ("undeclared-identifier", "int main(void){return x;}\n"),
    ("uninitialized-local", "int main(void){int x;return x;}\n"),
    (
        "unsupported-operator",
        "int main(void){int x=2;return x^3;}\n",
    ),
    (
        "mismatched-assignment-name",
        "int main(void){int x=2;y=x+3;return x;}\n",
    ),
    (
        "unclosed-parenthesis",
        "int main(void){int x=2;return (x+3;}\n",
    ),
    ("division-by-zero", "int main(void){int x=2;return x/0;}\n"),
    ("remainder-by-zero", "int main(void){int x=2;return x%0;}\n"),
];

#[test]
fn failed_compilation_emits_no_assembly_or_report() {
    for &(name, source) in REJECTED {
        let dir = workdir(name);
        let asm = dir.join("main.s");
        let report = dir.join("evidence.txt");
        let output = run_source(&dir, source, Some(asm.as_path()), Some(report.as_path()));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{name}: expected a candidate diagnostic, not acceptance\nstdout: {}\nstderr: {stderr}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(
            output.stdout.is_empty(),
            "{name}: rejected source wrote evidence to stdout: {}",
            String::from_utf8_lossy(&output.stdout)
        );
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
        assert!(
            !report.exists(),
            "{name}: rejected source left a report artifact at {}",
            report.display()
        );
    }
}

#[test]
fn failed_compilation_preserves_preexisting_outputs() {
    let dir = workdir("preserve");
    let asm = dir.join("main.s");
    let report = dir.join("evidence.txt");
    std::fs::write(&asm, b"previous assembly\n").expect("sentinel assembly");
    std::fs::write(&report, b"previous report\n").expect("sentinel report");

    let output = run_source(
        &dir,
        "int main(void){int x=2;return x/0;}\n",
        Some(asm.as_path()),
        Some(report.as_path()),
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "division-by-zero must fail closed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(&asm).expect("assembly sentinel remains"),
        "previous assembly\n",
        "failed compilation modified the pre-existing assembly artifact"
    );
    assert_eq!(
        std::fs::read_to_string(&report).expect("report sentinel remains"),
        "previous report\n",
        "failed compilation modified the pre-existing evidence artifact"
    );
}
