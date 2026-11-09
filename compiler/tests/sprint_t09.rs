// ============================================================================
// sprint_t09.rs — T09: two call results live across a third call.
//
// `main` evaluates three calls left to right and passes all three results to
// `digits`. When the third call runs, the first two results are still live.
// Under AAPCS64 a callee may freely clobber w0..w3, so a caller that merely
// keeps those results in scratch registers across `bl inc` corrupts them.
//
// Hand-derived model: inc(1) = 2, inc(2) = 3, inc(3) = 4, and
// digits(2, 3, 4) = 2 * 100 + 3 * 10 + 4 = 234 = 0xea.
//
// The assembly check is structural evidence for the host-only emitter, not
// native execution or T00 evidence. It requires each call result to be stored
// to its own stack slot before the next call and the outer call to reload
// exactly those slots into w0/w1/w2 after the third call.
// ============================================================================

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

const SOURCE: &str = "int inc(int x) { return x + 1; }\n\
int digits(int hundreds, int tens, int units) {\n\
    return hundreds * 100 + tens * 10 + units;\n\
}\n\
int main(void) { return digits(inc(1), inc(2), inc(3)); }\n";

const EXPECTED_INTERPRET: &str = "interpret: value=ea negative=false";

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
        "sprint-t09-{}-{}-{}",
        std::process::id(),
        test,
        unique_suffix()
    ));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn run_candidate(source: &str, asm: &Path) -> Output {
    let dir = asm.parent().expect("assembly path has a parent");
    fs::create_dir_all(dir).expect("scratch dir");
    let source_path = dir.join("main.c");
    fs::write(&source_path, source.as_bytes()).expect("write source");
    let stdout_path = dir.join("candidate.stdout");
    let stderr_path = dir.join("candidate.stderr");
    let mut command = Command::new(binary());
    command
        .arg("--emit-asm")
        .arg(asm)
        .arg("--interpret-ir")
        .arg(&source_path)
        .stdout(Stdio::from(
            fs::File::create(&stdout_path).expect("stdout file"),
        ))
        .stderr(Stdio::from(
            fs::File::create(&stderr_path).expect("stderr file"),
        ));
    let mut child = command
        .spawn()
        .unwrap_or_else(|error| panic!("candidate failed to spawn: {error}"));
    let deadline = Instant::now() + Duration::from_secs(60);
    let status = loop {
        match child.try_wait().expect("candidate wait") {
            Some(status) => break status,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                let stdout = fs::read_to_string(&stdout_path).unwrap_or_default();
                let stderr = fs::read_to_string(&stderr_path).unwrap_or_default();
                panic!("candidate exceeded 60s\nstdout: {stdout}\nstderr: {stderr}");
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    Output {
        status,
        stdout: fs::read(&stdout_path).expect("candidate stdout"),
        stderr: fs::read(&stderr_path).expect("candidate stderr"),
    }
}

/// Body lines of one emitted function, from its label to `.size`.
fn function_body<'a>(assembly: &'a str, name: &str) -> Vec<&'a str> {
    let lines: Vec<&'a str> = assembly.lines().collect();
    let label = format!("{name}:");
    let start = lines
        .iter()
        .position(|line| line.trim() == label.as_str())
        .unwrap_or_else(|| panic!("missing function label `{label}` in:\n{assembly}"));
    let mut body = Vec::new();
    for &line in lines.iter().skip(start + 1) {
        let trimmed = line.trim();
        if trimmed.starts_with(".size ") {
            break;
        }
        if !trimmed.is_empty() {
            body.push(trimmed);
        }
    }
    body
}

fn call_target(line: &str) -> Option<&str> {
    let mut parts = line.split_whitespace();
    if parts.next() != Some("bl") {
        return None;
    }
    parts.next()
}

/// Parse one direct `mnemonic register, [sp, #slot]` frame access.
fn slot_access(line: &str, mnemonic: &str, register: &str) -> Option<u32> {
    let compact: String = line
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    let prefix = format!("{mnemonic}{register},[sp,#");
    compact
        .strip_prefix(&prefix)?
        .strip_suffix(']')?
        .parse()
        .ok()
}

#[test]
fn two_call_results_live_across_third_call_without_scratch_reuse() {
    let dir = workdir("live-across-third");
    let asm = dir.join("main.s");
    let output = run_candidate(SOURCE, &asm);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(0),
        "candidate rejected the T09 source\nstdout: {stdout}\nstderr: {stderr}"
    );
    let interpret_lines: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("interpret: "))
        .collect();
    assert_eq!(
        interpret_lines,
        vec![EXPECTED_INTERPRET],
        "the modeled value must be the hand-derived 234 (0xea)\nstdout: {stdout}"
    );

    let assembly = fs::read_to_string(&asm)
        .unwrap_or_else(|error| panic!("cannot read emitted assembly {}: {error}", asm.display()));
    let body = function_body(&assembly, "main");
    let calls: Vec<(usize, &str)> = body
        .iter()
        .enumerate()
        .filter_map(|(index, line)| call_target(line).map(|target| (index, target)))
        .collect();
    let targets: Vec<&str> = calls.iter().map(|(_, target)| *target).collect();
    assert_eq!(
        targets,
        vec!["inc", "inc", "inc", "digits"],
        "main must evaluate the three inc calls before digits:\n{assembly}"
    );

    let first_three = calls.get(..3).expect("three inc calls");
    let mut spill_slots = Vec::new();
    for &(index, target) in first_three {
        assert_eq!(target, "inc");
        let next = body.get(index + 1).copied().unwrap_or_else(|| {
            panic!("missing instruction after `bl inc` at body index {index}:\n{assembly}")
        });
        let slot = slot_access(next, "str", "w0").unwrap_or_else(|| {
            panic!(
                "each inc result must be spilled immediately after its call; found `{next}`:\n{assembly}"
            )
        });
        spill_slots.push(slot);
    }
    assert_eq!(spill_slots.len(), 3);
    assert_ne!(
        spill_slots[0], spill_slots[1],
        "the first two live call results must not share one slot:\n{assembly}"
    );
    assert_ne!(
        spill_slots[0], spill_slots[2],
        "the first and third call results must not share one slot:\n{assembly}"
    );
    assert_ne!(
        spill_slots[1], spill_slots[2],
        "the second and third call results must not share one slot:\n{assembly}"
    );

    let (digits_index, digits_target) = calls[3];
    assert_eq!(digits_target, "digits");
    assert!(digits_index >= 3);
    let argument_setup = &body[digits_index - 3..digits_index];
    assert_eq!(
        slot_access(argument_setup[0], "ldr", "w0"),
        Some(spill_slots[0]),
        "digits argument 0 must reload the first inc result after the third call:\n{assembly}"
    );
    assert_eq!(
        slot_access(argument_setup[1], "ldr", "w1"),
        Some(spill_slots[1]),
        "digits argument 1 must reload the second inc result after the third call:\n{assembly}"
    );
    assert_eq!(
        slot_access(argument_setup[2], "ldr", "w2"),
        Some(spill_slots[2]),
        "digits argument 2 must reload the third inc result after its call:\n{assembly}"
    );

    let store_line = body.get(digits_index + 1).copied().unwrap_or_else(|| {
        panic!("missing instruction after `bl digits` at body index {digits_index}:\n{assembly}")
    });
    let result_slot = slot_access(store_line, "str", "w0").unwrap_or_else(|| {
        panic!(
            "the digits result must be spilled after its call; found `{store_line}`:\n{assembly}"
        )
    });
    assert!(
        !spill_slots.contains(&result_slot),
        "the outer result must not overwrite a still-live call-result slot:\n{assembly}"
    );
    let tail = body
        .get(digits_index + 2..)
        .unwrap_or_else(|| panic!("missing return path after `bl digits`:\n{assembly}"));
    assert!(
        tail.iter()
            .any(|line| slot_access(line, "ldr", "w0") == Some(result_slot)),
        "the return path must reload the outer result slot:\n{assembly}"
    );
}
