// ============================================================================
// sprint_t04.rs — T04 local/parameter shadowing of a function name.
//
// In C's ordinary namespace an automatic object and a file-scope function may
// share a spelling; the inner object hides the outer function. A call whose
// callee identifier resolves to that object is invalid: an object designator
// is not a function designator. The `/55` direct-call subset must report a
// typed rejection rather than quietly selecting the hidden file-scope
// function.
//
// The accepted controls use the hand-derived function f(x) = x + 1:
// f(2) = 3, and g(5) = f(5) = 6 when main passes its hidden object value 5.
// No expectation below was obtained by running the implementation.
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
        "sprint-t04-{}-{}-{}",
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

/// A shadowing object must be rejected as a callee, not skipped in favour of
/// the file-scope function of the same spelling.
fn assert_shadow_rejected(name: &str, text: &str) {
    let dir = workdir(name);
    let source = dir.join("main.c");
    let asm = dir.join("main.s");
    fs::write(&source, text.as_bytes()).expect("source file");

    let output = invoke(&source, Some(&asm));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{name}: shadowed object must not be callable\nstdout: {}\nstderr: {stderr}",
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
        stderr.contains("object"),
        "{name}: expected the diagnostic to name the object target: {stderr}"
    );
    assert!(
        stderr.contains("not a function"),
        "{name}: expected an object-is-not-callable diagnostic: {stderr}"
    );
    assert!(
        !asm.exists(),
        "{name}: rejected source left an assembly artifact at {}",
        asm.display()
    );
}

#[test]
fn local_and_parameter_objects_shadow_the_function_and_are_not_callable() {
    // main's automatic `f` hides file-scope `f`; without the shadow this call
    // would be f(1) = 2.
    assert_shadow_rejected(
        "local-shadow",
        "int f(int x) { return x + 1; }\n\
int main(void) { int f = 2; return f(1); }\n",
    );

    // g's parameter `f` hides file-scope `f`; without the shadow this call
    // would be f(1) = 2.
    assert_shadow_rejected(
        "parameter-shadow",
        "int f(int x) { return x + 1; }\n\
int g(int f) { return f(1); }\n\
int main(void) { return g(2); }\n",
    );
}

#[test]
fn shadowing_is_lexically_scoped_to_the_declaring_function() {
    // Hand-derived results: f(2) = 3, and g(5) = f(5) = 6 when main passes
    // its hidden object value 5 to the sibling body.
    let controls = [
        (
            "unshadowed",
            "int f(int x) { return x + 1; }\n\
int main(void) { int x = 2; return f(x); }\n",
            "03",
        ),
        (
            "sibling-local",
            // main's local f hides the function only in main: main passes the
            // object value 5 to g, and g's body still calls file-scope f.
            "int f(int x) { return x + 1; }\n\
int g(int x) { return f(x); }\n\
int main(void) { int f = 5; return g(f); }\n",
            "06",
        ),
    ];

    for (name, text, expected) in controls {
        let dir = workdir(name);
        let source = dir.join("main.c");
        let asm = dir.join("main.s");
        fs::write(&source, text.as_bytes()).expect("source file");

        let output = invoke(&source, Some(&asm));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{name}: unshadowed call must be accepted\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        let expected_line = format!("interpret: value={expected} negative=false");
        assert!(
            stdout.contains(expected_line.as_str()),
            "{name}: hand-derived result must model {expected}:\n{stdout}"
        );
        assert!(
            asm.is_file(),
            "{name}: accepted source must emit the requested assembly at {}",
            asm.display()
        );
    }
}
