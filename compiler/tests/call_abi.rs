// ============================================================================
// call_abi.rs — independent `/55` direct-call ABI acceptance.
//
// INTEGRATION_PENDING at the `/54` schema baseline: the feasible same-TU
// checks need the `/55` source/IR/emitter producers. The two-way mixed-link
// directions are recorded as blocked, not passed, because neither has a
// frozen boundary:
//
//   * candidate callee + reference caller: `--emit-asm` emits the complete
//     source TU including `main`, so a reference caller's `main` would
//     duplicate the entry symbol. No function-only emission path is frozen.
//   * candidate caller + reference callee: normal `/55` source rejects an
//     unknown/forward/external callee, and no test-only external-callee
//     IR/artifact boundary exists at `/54`.
//
// The feasible evidence here is same-TU ABI behavior: a candidate-defined
// `main` plus candidate helper functions, assembled/linked by external GCC on
// Linux AArch64. `nm`/`readelf` are not used; emitted-text and execution
// checks are the semantic evidence. Native runs verify the actual GCC target
// before executing, use bounded waits, and print an explicit skip otherwise.
// This is not T00 corpus evidence.
// ============================================================================

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static BOUNDED_RUN_ID: AtomicU64 = AtomicU64::new(0);

const EIGHT_ARGUMENTS: &str = "int mix(int a, int b, int c, int d, int e, int f, int g, int h) {\n\
    return a - b + c - d + e - f + g - h;\n\
}\n\
int main(void) { return mix(1, 2, 3, 4, 5, 6, 7, 8); }\n";

#[cfg(target_os = "linux")]
const NESTED_CALLS: &str = "int inc(int x) { return x + 1; }\n\
int add(int a, int b) { return a + b; }\n\
int main(void) { return add(inc(2), inc(3)); }\n";

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
        "call-abi-{}-{}-{}",
        std::process::id(),
        test,
        unique_suffix()
    ));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// Wait for a child with a finite deadline. Standard Rust has no built-in
/// command timeout, so output is redirected to files and the child is killed
/// when the bound expires.
fn bounded_output(command: &mut Command, limit: Duration, label: &str) -> Output {
    let serial = BOUNDED_RUN_ID.fetch_add(1, Ordering::Relaxed);
    let base = std::env::temp_dir().join(format!(
        "call-abi-bound-{}-{}-{}-{}",
        std::process::id(),
        serial,
        label,
        unique_suffix()
    ));
    let stdout_path = base.with_extension("stdout");
    let stderr_path = base.with_extension("stderr");
    let stdout_file = fs::File::create(&stdout_path).expect("bounded stdout file");
    let stderr_file = fs::File::create(&stderr_path).expect("bounded stderr file");
    command
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));
    let mut child = command
        .spawn()
        .unwrap_or_else(|error| panic!("{label} failed to spawn: {error}"));
    let deadline = Instant::now() + limit;
    let status = loop {
        match child.try_wait().expect("bounded child wait") {
            Some(status) => break status,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                let stdout = fs::read_to_string(&stdout_path).unwrap_or_default();
                let stderr = fs::read_to_string(&stderr_path).unwrap_or_default();
                let _ = fs::remove_file(&stdout_path);
                let _ = fs::remove_file(&stderr_path);
                panic!("{label} exceeded {limit:?}\nstdout: {stdout}\nstderr: {stderr}");
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    let stdout = fs::read(&stdout_path).expect("bounded stdout");
    let stderr = fs::read(&stderr_path).expect("bounded stderr");
    let _ = fs::remove_file(&stdout_path);
    let _ = fs::remove_file(&stderr_path);
    Output {
        status,
        stdout,
        stderr,
    }
}

fn run_candidate(source: &str, asm: &Path) -> Output {
    let dir = asm.parent().expect("assembly parent");
    fs::create_dir_all(dir).expect("source dir");
    let source_path = dir.join("main.c");
    fs::write(&source_path, source.as_bytes()).expect("source file");
    let mut command = Command::new(binary());
    command
        .arg("--emit-asm")
        .arg(asm)
        .arg("--interpret-ir")
        .arg(&source_path);
    bounded_output(&mut command, Duration::from_secs(60), "candidate-abi")
}

fn read_asm(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read emitted assembly {}: {error}", path.display()))
}

fn assert_modeled(output: &Output, magnitude_hex: &str, negative: bool, context: &str) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{context}\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    let mut lines = text.lines().filter(|line| line.starts_with("interpret: "));
    let line = lines
        .next()
        .unwrap_or_else(|| panic!("{context}: missing interpret line\nstdout: {text}"));
    assert!(
        lines.next().is_none(),
        "{context}: multiple interpret lines: {text}"
    );
    let rest = line
        .strip_prefix("interpret: value=")
        .unwrap_or_else(|| panic!("{context}: malformed interpret line: {line}"));
    let (magnitude, negative_flag) = rest
        .split_once(" negative=")
        .unwrap_or_else(|| panic!("{context}: malformed interpret line: {line}"));
    assert_eq!(
        (magnitude, negative_flag == "true"),
        (magnitude_hex, negative),
        "{context}\nstdout: {text}"
    );
}

fn function_body(assembly: &str, name: &str, function_names: &[&str]) -> String {
    let lines: Vec<&str> = assembly.lines().collect();
    let label = format!("{name}:");
    let start = lines
        .iter()
        .position(|line| line.trim() == label)
        .unwrap_or_else(|| panic!("missing function label `{label}` in:\n{assembly}"));
    let mut body = Vec::new();
    for line in lines.iter().skip(start + 1) {
        let trimmed = line.trim();
        if trimmed.starts_with(".size ") {
            break;
        }
        if let Some(candidate) = trimmed.strip_suffix(':') {
            if function_names.contains(&candidate) {
                break;
            }
        }
        body.push(*line);
    }
    body.join("\n")
}

fn assert_label(assembly: &str, name: &str, context: &str) {
    let label = format!("{name}:");
    assert!(
        assembly.lines().any(|line| line.trim() == label),
        "{context}: missing function label `{label}` in:\n{assembly}"
    );
}

fn destination_registers(line: &str) -> Vec<String> {
    let text = line.split("//").next().unwrap_or(line).trim();
    if text.is_empty() || text.starts_with('.') || text.starts_with('#') || text.ends_with(':') {
        return Vec::new();
    }
    let mut parts = text.splitn(2, char::is_whitespace);
    let mnemonic = parts.next().unwrap_or("").to_ascii_lowercase();
    let rest = parts.next().unwrap_or("").trim();
    if matches!(
        mnemonic.as_str(),
        "str"
            | "stp"
            | "bl"
            | "b"
            | "br"
            | "blr"
            | "ret"
            | "cbz"
            | "cbnz"
            | "tbz"
            | "tbnz"
            | "cmp"
            | "cmn"
            | "tst"
            | "nop"
    ) || rest.is_empty()
    {
        return Vec::new();
    }
    let take = if mnemonic == "ldp" { 2 } else { 1 };
    rest.split(',')
        .map(str::trim)
        .take(take)
        .filter(|operand| operand.starts_with('w') || operand.starts_with('x'))
        .map(|operand| operand.trim_start_matches('!').to_ascii_lowercase())
        .collect()
}

fn is_callee_saved(register: &str) -> bool {
    let digits = register
        .strip_prefix('x')
        .or_else(|| register.strip_prefix('w'));
    digits
        .and_then(|digits| digits.parse::<u32>().ok())
        .map(|number| (19..=28).contains(&number))
        .unwrap_or(false)
}

fn stack_adjust_immediate(line: &str, mnemonic: &str) -> Option<u32> {
    let text = line.split("//").next().unwrap_or(line).trim();
    let compact: String = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    let prefix = format!("{mnemonic}sp,sp,#");
    compact.strip_prefix(&prefix)?.parse::<u32>().ok()
}

fn assert_argument_register_setup(
    assembly: &str,
    caller: &str,
    callee: &str,
    argument_count: usize,
    function_names: &[&str],
    context: &str,
) {
    let body = function_body(assembly, caller, function_names);
    let lines: Vec<&str> = body.lines().collect();
    let call_prefix = format!("bl {callee}");
    let call_index = lines
        .iter()
        .position(|line| {
            let mut tokens = line.split_whitespace();
            tokens.next() == Some("bl") && tokens.next() == Some(callee)
        })
        .unwrap_or_else(|| panic!("{context}: `{caller}` does not call `{callee}`:\n{body}"));
    let mut written = std::collections::BTreeSet::new();
    for line in &lines[..call_index] {
        for register in destination_registers(line) {
            written.insert(register);
        }
    }
    for index in 0..argument_count {
        let register = format!("w{index}");
        assert!(
            written.contains(&register),
            "{context}: call to `{callee}` does not set `{register}` before `{call_prefix}`:\n{body}"
        );
    }
}

fn assert_aapcs64_frame_shape(assembly: &str, function_names: &[&str], context: &str) {
    for &name in function_names {
        let body = function_body(assembly, name, function_names);
        for line in body.lines() {
            for register in destination_registers(line) {
                assert!(
                    !is_callee_saved(&register),
                    "{context}: `{name}` writes callee-saved register `{register}`: {line}\n{body}"
                );
            }
        }
        let mut saw_aligned_frame = false;
        for line in body.lines() {
            if let Some(immediate) = stack_adjust_immediate(line, "sub") {
                saw_aligned_frame = true;
                assert_eq!(
                    immediate % 16,
                    0,
                    "{context}: `{name}` uses a non-16-byte-aligned frame: {line}"
                );
            }
        }
        let non_leaf = body
            .lines()
            .any(|line| line.split_whitespace().next() == Some("bl"));
        assert!(
            body.lines()
                .any(|line| line.split_whitespace().next() == Some("ret")),
            "{context}: `{name}` has no return instruction:\n{body}"
        );
        if non_leaf {
            assert!(
                saw_aligned_frame,
                "{context}: non-leaf `{name}` does not reserve a 16-byte-aligned stack frame:\n{body}"
            );
            let lr_mentions = body.matches("x30").count();
            assert!(
                lr_mentions >= 2,
                "{context}: non-leaf `{name}` does not save and restore LR (`x30` occurrences: {lr_mentions}):\n{body}"
            );
        }
    }
}

#[test]
fn same_tu_call_assembly_is_aapcs64_shaped() {
    let dir = workdir("shape");
    let asm = dir.join("main.s");
    let output = run_candidate(EIGHT_ARGUMENTS, &asm);
    assert_modeled(&output, "04", true, "eight-argument ABI shape");
    let assembly = read_asm(&asm);
    assert!(
        assembly.contains(".globl main"),
        "candidate program must expose the main entry:\n{assembly}"
    );
    assert_label(&assembly, "main", "eight-argument ABI shape");
    assert_label(&assembly, "mix", "eight-argument ABI shape");
    assert!(
        assembly.lines().any(|line| {
            let mut tokens = line.split_whitespace();
            tokens.next() == Some("bl") && tokens.next() == Some("mix")
        }),
        "candidate main must call mix directly:\n{assembly}"
    );
    assert_argument_register_setup(
        &assembly,
        "main",
        "mix",
        8,
        &["mix", "main"],
        "eight-argument ABI shape",
    );
    assert_aapcs64_frame_shape(&assembly, &["mix", "main"], "eight-argument ABI shape");
}

#[cfg(target_os = "linux")]
fn aarch64_gcc() -> Option<&'static str> {
    let mut command = Command::new("gcc");
    command.arg("-dumpmachine");
    let output = bounded_output(&mut command, Duration::from_secs(15), "gcc-dumpmachine");
    assert!(
        output.status.success(),
        "gcc -dumpmachine failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let machine = String::from_utf8_lossy(&output.stdout);
    let machine = machine.trim();
    if !machine.starts_with("aarch64") || machine.contains("aarch64_be") {
        eprintln!(
            "SKIP: native AArch64 execution requires an aarch64-linux gcc; found `{machine}`"
        );
        return None;
    }
    Some("gcc")
}

#[cfg(target_os = "linux")]
fn build_and_run_candidate_program(
    source: &str,
    magnitude_hex: &str,
    negative: bool,
    expected_exit: i32,
    context: &str,
) {
    let Some(gcc) = aarch64_gcc() else {
        return;
    };
    let dir = workdir(context);
    let asm = dir.join("main.s");
    let exe = dir.join("main");
    let output = run_candidate(source, &asm);
    assert_modeled(&output, magnitude_hex, negative, context);

    let mut build = Command::new(gcc);
    build.arg("-o").arg(&exe).arg(&asm);
    let build = bounded_output(&mut build, Duration::from_secs(60), "gcc-link");
    assert!(
        build.status.success(),
        "{context}: external GCC failed to assemble/link\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let mut run = Command::new(&exe);
    let run = bounded_output(&mut run, Duration::from_secs(30), "native-run");
    assert_eq!(
        run.status.code(),
        Some(expected_exit),
        "{context}: native exit status differs (modeled {}{} expected low byte {})\nstdout: {}\nstderr: {}",
        if negative { "-" } else { "" },
        magnitude_hex,
        expected_exit,
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}

#[cfg(target_os = "linux")]
#[test]
fn same_tu_eight_argument_program_links_and_returns_minus_four() {
    build_and_run_candidate_program(EIGHT_ARGUMENTS, "04", true, 252, "same-tu-eight-argument");
}

#[cfg(target_os = "linux")]
#[test]
fn same_tu_nested_call_program_links_and_returns_seven() {
    build_and_run_candidate_program(NESTED_CALLS, "07", false, 7, "same-tu-nested-calls");
}

#[test]
#[ignore = "BLOCKED (not a pass): `/55` --emit-asm emits the complete source TU including `main`; there is no frozen function-only emission path, so a reference caller's `main` would duplicate the candidate entry symbol at link time. Removing or renaming the candidate entry would change the artifact under test."]
fn blocked_candidate_callee_with_reference_caller() {
    panic!(
        "BLOCKED (not a pass): candidate callee + reference caller remains unverified. \
         `/55` --emit-asm always emits the source program's `main`, and no frozen \
         function-only or test-only mixed-link boundary exists at `/54`. A duplicate \
         `main` link cannot be treated as ABI evidence."
    );
}

#[test]
#[ignore = "BLOCKED (not a pass): normal `/55` source rejects unknown/forward/external callees, and no frozen test-only external-callee IR/artifact boundary exists at `/54`. A candidate caller cannot name a reference C callee without broadening the frontend or inventing an unapproved fixture boundary."]
fn blocked_candidate_caller_with_reference_callee() {
    panic!(
        "BLOCKED (not a pass): candidate caller + reference callee remains unverified. \
         The source definition-order rule rejects external callees, and no frozen \
         test-only external-callee boundary exists at `/54`. Normal source support \
         must not be silently broadened to manufacture this evidence."
    );
}
