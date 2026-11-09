// ============================================================================
// c56_source_locals.rs — `/49` M1 source-locals vertical slice.
//
// The candidate parses `int main(void){int x=2;return x;}`, commits an
// automatic local, lowers Constant/LocalStore/LocalLoad/Return, models the
// returned initializer, emits AArch64, and — on native AArch64 Linux —
// assembles, links, and runs the result. The canonical constant-return
// source remains a regression fixture.
// ============================================================================

use std::path::PathBuf;
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "source-locals-{}-{}-{}",
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

fn run_candidate(source: &str, asm: Option<&PathBuf>) -> std::process::Output {
    let dir = asm
        .map(|path| path.parent().expect("asm parent").to_path_buf())
        .unwrap_or_else(|| workdir("source"));
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

#[test]
fn locals_shape_lowers_to_load_store_ir_and_models_two() {
    let dir = workdir("emit");
    let asm = dir.join("main.s");
    let output = run_candidate("int main(void){int x=2;return x;}\n", Some(&asm));
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("interpret: value=02"), "{stdout}");
    let assembly = String::from_utf8(std::fs::read(&asm).expect("assembly file")).unwrap();
    assert!(assembly.contains("sub sp, sp, #16"), "{assembly}");
    assert!(assembly.contains("str w0, [sp, #0]"), "{assembly}");
    assert!(assembly.contains("ldr w0, [sp, #0]"), "{assembly}");
    assert!(assembly.contains("add sp, sp, #16"), "{assembly}");
    assert!(assembly.contains("    ret\n"), "{assembly}");
}

#[test]
fn local_add_lowers_to_arithmetic_ir_and_models_five() {
    let dir = workdir("add-emit");
    let asm = dir.join("main.s");
    let output = run_candidate("int main(void){int x=2;return x+3;}\n", Some(&asm));
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("interpret: value=05"), "{stdout}");
    let assembly = String::from_utf8(std::fs::read(&asm).expect("assembly file")).unwrap();
    assert!(assembly.contains("add w0, w0, w1"), "{assembly}");
    assert!(assembly.contains("str w0, [sp, #0]"), "{assembly}");
    assert!(assembly.contains("ldr w0, [sp, #0]"), "{assembly}");
    assert!(assembly.contains("    ret\n"), "{assembly}");
}

#[test]
fn local_operators_lower_to_the_matching_ir() {
    let cases = [
        ("-", "value=01 negative=true", "sub w0, w0, w1"),
        ("*", "value=06 negative=false", "mul w0, w0, w1"),
        ("/", "value=00 negative=false", "sdiv w0, w0, w1"),
        ("%", "value=02 negative=false", "msub w0, w2, w1, w0"),
    ];
    for (operator, expected, instruction) in cases {
        let source = format!("int main(void){{int x=2;return x{operator}3;}}\n");
        let dir = workdir(&format!("op-{}", operator.bytes().next().unwrap()));
        let asm = dir.join("main.s");
        let output = run_candidate(&source, Some(&asm));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{operator}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains(expected), "{operator}: {stdout}");
        let assembly = String::from_utf8(std::fs::read(&asm).expect("assembly file")).unwrap();
        assert!(assembly.contains(instruction), "{operator}: {assembly}");
    }
}

#[test]
fn canonical_constant_return_still_models_five() {
    let dir = workdir("canonical");
    let asm = dir.join("main.s");
    let output = run_candidate("int main(void){return 2+3;}\n", Some(&asm));
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("interpret: value=05"), "{stdout}");
    let assembly = String::from_utf8(std::fs::read(&asm).expect("assembly file")).unwrap();
    assert!(assembly.contains("movz w0, #5"), "{assembly}");
    assert!(!assembly.contains("sub sp, sp"), "{assembly}");
}

#[test]
fn locals_slice_replays_deterministically() {
    let dir = workdir("replay");
    let source = dir.join("main.c");
    std::fs::write(&source, b"int main(void){int x=2;return x;}\n").expect("source file");
    let run = || {
        Command::new(binary())
            .arg("--interpret-ir")
            .arg(&source)
            .output()
            .expect("candidate runs")
    };
    let first = run();
    let second = run();
    assert_eq!(first.status.code(), Some(0));
    assert_eq!(second.status.code(), Some(0));
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
}

#[test]
fn identifier_chip_manifest_is_write_scoped() {
    use cc_silicon_compiler::chips::SeIdentifierChip;
    use cc_silicon_compiler::chips::Worker;
    use cc_silicon_compiler::manifest::SE_ID_CHIP;
    use cc_silicon_compiler::task::{StoreId, TaskKind};

    let manifest = SeIdentifierChip.manifest();
    assert_eq!(manifest.id, SE_ID_CHIP);
    assert_eq!(
        manifest.task_kinds,
        vec![TaskKind::SEMANTIC_IDENTIFIER_EXPR]
    );
    assert_eq!(
        manifest.writes,
        vec![cc_silicon_compiler::manifest::FieldPath::new(
            StoreId::Sem,
            "records"
        )]
    );
    assert!(manifest.declares_read(StoreId::Symbols, "symbols"));
    assert!(manifest.declares_read(StoreId::Symbols, "scopes"));
    assert!(manifest.declares_read(StoreId::Parse, "nodes"));
}

#[test]
fn locals_shape_rejects_mismatched_names_and_extra_statements() {
    for source in [
        "int main(void){int x=2;return y;}\n",
        "int main(void){int x=2;return x^3;}\n",
        "int main(void){int x;return x;}\n",
        "int main(void){return x;}\n",
    ] {
        let output = run_candidate(source, None);
        assert_eq!(
            output.status.code(),
            Some(1),
            "source unexpectedly accepted: {source}\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("candidate diagnostic"), "{stderr}");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn emitted_locals_assembly_runs_on_aarch64() {
    let machine = Command::new("gcc")
        .arg("-dumpmachine")
        .output()
        .expect("gcc available");
    if !String::from_utf8_lossy(&machine.stdout).contains("aarch64") {
        return;
    }
    let dir = workdir("run");
    let asm = dir.join("main.s");
    let exe = dir.join("main");
    let output = run_candidate("int main(void){int x=2;return x;}\n", Some(&asm));
    assert_eq!(output.status.code(), Some(0));
    let build = Command::new("gcc")
        .arg("-o")
        .arg(&exe)
        .arg(&asm)
        .output()
        .expect("gcc runs");
    assert!(
        build.status.success(),
        "gcc failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&exe).output().expect("program runs");
    assert_eq!(run.status.code(), Some(2));
}

#[cfg(target_os = "linux")]
#[test]
fn emitted_local_add_runs_on_aarch64() {
    let machine = Command::new("gcc")
        .arg("-dumpmachine")
        .output()
        .expect("gcc available");
    if !String::from_utf8_lossy(&machine.stdout).contains("aarch64") {
        return;
    }
    let dir = workdir("run-add");
    let asm = dir.join("main.s");
    let exe = dir.join("main");
    let output = run_candidate("int main(void){int x=2;return x+3;}\n", Some(&asm));
    assert_eq!(output.status.code(), Some(0));
    let build = Command::new("gcc")
        .arg("-o")
        .arg(&exe)
        .arg(&asm)
        .output()
        .expect("gcc runs");
    assert!(
        build.status.success(),
        "gcc failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&exe).output().expect("program runs");
    assert_eq!(run.status.code(), Some(5));
}

#[cfg(target_os = "linux")]
#[test]
fn emitted_local_operators_run_on_aarch64() {
    let machine = Command::new("gcc")
        .arg("-dumpmachine")
        .output()
        .expect("gcc available");
    if !String::from_utf8_lossy(&machine.stdout).contains("aarch64") {
        return;
    }
    for (operator, expected) in [("-", 255), ("*", 6), ("/", 0), ("%", 2)] {
        let dir = workdir(&format!("run-op-{}", operator.bytes().next().unwrap()));
        let asm = dir.join("main.s");
        let exe = dir.join("main");
        let source = format!("int main(void){{int x=2;return x{operator}3;}}\n");
        let output = run_candidate(&source, Some(&asm));
        assert_eq!(output.status.code(), Some(0));
        let build = Command::new("gcc")
            .arg("-o")
            .arg(&exe)
            .arg(&asm)
            .output()
            .expect("gcc runs");
        assert!(
            build.status.success(),
            "gcc failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let run = Command::new(&exe).output().expect("program runs");
        assert_eq!(run.status.code(), Some(expected), "operator {operator}");
    }
}
