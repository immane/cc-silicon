// ============================================================================
// sprint_r02.rs — legacy `/52` local-expression depth budget.
//
// The single-local source path uses the legacy expression parser. It must
// enforce the same 32-level nesting budget as the general parser: 32 levels
// are accepted, 33 fail typed and append no AST/artifact.
// ============================================================================

use std::path::PathBuf;
use std::process::Command;

fn candidate() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sprint-r02-{}-{test}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock moves forward")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn source(levels: usize, unary: bool) -> String {
    let open: String = if unary {
        "- ".repeat(levels)
    } else {
        "(".repeat(levels)
    };
    let close: String = if unary {
        String::new()
    } else {
        ")".repeat(levels)
    };
    format!("int main(void){{int x=1;return {open}x{close};}}\n")
}

fn run(levels: usize, unary: bool) -> (Option<i32>, String, String) {
    let dir = workdir(if unary { "unary" } else { "group" });
    let path = dir.join("main.c");
    std::fs::write(&path, source(levels, unary)).expect("source write");
    let output = Command::new(candidate())
        .arg("--interpret-ir")
        .arg(&path)
        .output()
        .expect("candidate runs");
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn legacy_local_expression_depth_is_bounded() {
    for unary in [false, true] {
        let (status, stdout, stderr) = run(32, unary);
        assert_eq!(status, Some(0), "32 levels: {stderr}");
        assert!(
            stdout.contains("interpret: value=01 negative=false"),
            "{stdout}"
        );

        let (status, stdout, stderr) = run(33, unary);
        assert_eq!(status, Some(1), "33 levels: {stderr}");
        assert!(!stdout.contains("interpret: "), "{stdout}");
        assert!(
            stderr.contains("candidate diagnostic:"),
            "expected typed diagnostic: {stderr}"
        );
    }
}
