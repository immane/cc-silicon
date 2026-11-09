// ============================================================================
// sprint_t06.rs — T06 unknown, later, self, and mutually recursive callees.
//
// Derived from DIRECT_CALL_SLICE.md: every callee must be an earlier
// file-scope function definition. An undeclared name, a definition that
// follows the caller, self-recursion, and mutual recursion are therefore
// outside the `/55` source subset. Rejection must be explicit, must use the
// candidate diagnostic channel, and must not leave an assembly artifact.
//
// The positive control uses the hand-computed value add(2, 3) = 5. No
// expectation in this file was obtained by running the implementation.
// ============================================================================

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sprint-t06-{}-{}-{}",
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

/// Run the candidate with a finite deadline so a rejection bug cannot hang
/// the test process.
fn invoke(source: &Path, asm: Option<&Path>) -> Output {
    let mut command = Command::new(binary());
    if let Some(path) = asm {
        command.arg("--emit-asm").arg(path);
    }
    command.arg("--interpret-ir").arg(source);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().expect("candidate spawns");
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        match child.try_wait().expect("candidate wait") {
            Some(_) => break,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("candidate exceeded 60 seconds on {}", source.display());
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    child.wait_with_output().expect("candidate output")
}

#[test]
fn earlier_callee_control_is_accepted() {
    let dir = workdir("earlier-control");
    let source = dir.join("main.c");
    fs::write(
        &source,
        "int add(int a, int b) { return a + b; }\n\
int main(void) { return add(2, 3); }\n",
    )
    .expect("source file");

    let output = invoke(&source, None);
    assert_eq!(
        output.status.code(),
        Some(0),
        "earlier definition must remain callable\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("interpret: value=05 negative=false"),
        "hand-derived add(2, 3) must model 5:\n{stdout}"
    );
}

#[test]
fn unknown_later_self_and_mutual_callees_are_rejected() {
    let cases = [
        ("unknown", "int main(void) { return missing(2); }\n"),
        (
            "later",
            "int main(void) { return later(2); }\n\
int later(int x) { return x + 1; }\n",
        ),
        (
            "self",
            "int recurse(int x) { return recurse(x); }\n\
int main(void) { return recurse(1); }\n",
        ),
        (
            "mutual",
            "int ping(int x) { return pong(x); }\n\
int pong(int x) { return ping(x); }\n\
int main(void) { return ping(1); }\n",
        ),
    ];

    for (name, text) in cases {
        let dir = workdir(name);
        let source = dir.join("main.c");
        let asm = dir.join("main.s");
        fs::write(&source, text).expect("source file");

        let output = invoke(&source, Some(&asm));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{name}: source unexpectedly accepted\nstdout: {}\nstderr: {stderr}",
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
    }
}
