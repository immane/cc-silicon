// ============================================================================
// c55_target_m1.rs — M1 AArch64 host-only target-emission prototype.
//
// Candidate emits GNU AArch64 assembly for the committed M1 Constant+Return
// shape. On a native AArch64 Linux host the test also assembles, links, and
// executes it. This is not a general T11 code generator.
// ============================================================================

use std::path::PathBuf;
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "m1-target-{}-{}-{}",
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

#[test]
fn candidate_emits_m1_aarch64_assembly() {
    let dir = workdir("emit");
    let source = dir.join("main.c");
    let asm = dir.join("main.s");
    std::fs::write(&source, b"int main(void){return 2+3;}\n").expect("source");
    let output = Command::new(binary())
        .arg("--emit-asm")
        .arg(&asm)
        .arg("--interpret-ir")
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(std::fs::read(&asm).expect("assembly file")).unwrap();
    assert!(text.contains(".globl main"));
    assert!(text.contains("main:"));
    assert!(text.contains("movz w0, #5"));
    assert!(text.contains("    ret"));
    assert!(text.contains(".size main, .-main"));
}

#[cfg(target_os = "linux")]
#[test]
fn emitted_assembly_assembles_links_and_returns_five() {
    let machine = Command::new("gcc")
        .arg("-dumpmachine")
        .output()
        .expect("gcc available");
    let machine = String::from_utf8_lossy(&machine.stdout);
    if !machine.contains("aarch64") {
        return;
    }
    let dir = workdir("run");
    let source = dir.join("main.c");
    let asm = dir.join("main.s");
    let exe = dir.join("main");
    std::fs::write(&source, b"int main(void){return 2+3;}\n").expect("source");
    let output = Command::new(binary())
        .arg("--emit-asm")
        .arg(&asm)
        .arg("--interpret-ir")
        .arg(&source)
        .output()
        .expect("candidate runs");
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
