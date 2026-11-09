// ============================================================================
// sprint_a03.rs — independent A03 ABI evidence: leaf/non-leaf LR handling.
//
// A leaf function never executes `bl`, so it must not read or write LR
// (`x30`), even when it owns a stack frame. A non-leaf function must save the
// incoming LR before its first call and reload that same slot after its last
// call, before returning. The fixture also exercises a frame-less leaf and a
// leaf with a parameter/value-slot frame.
//
// Expected values are hand-derived from the source: five()=5, leaf(1)=2,
// bump(2)=4, twice(2)=4+4=8, main=5+2+8=15 (0f). The assertions inspect only
// GNU AArch64 assembly text; emitter internals are not used as an oracle.
// ============================================================================

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

const LR_FIXTURE: &str = "\
int five(void) { return 2 + 3; }\n\
int leaf(int x) { return x + 1; }\n\
int bump(int x) { return x + 2; }\n\
int twice(int x) { return bump(x) + bump(x); }\n\
int main(void) { return five() + leaf(1) + twice(2); }\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LrAccess {
    Save { offset: u32 },
    Restore { offset: u32 },
}

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sprint-a03-{}-{}-{}",
        std::process::id(),
        label,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock moves forward")
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// Compile the fixture once and return the emitted assembly plus the candidate
/// evidence output. A nonzero status is a test-setup failure, not LR evidence.
fn emit_fixture(label: &str) -> (String, Output) {
    let dir = workdir(label);
    let source = dir.join("main.c");
    let asm = dir.join("main.s");
    fs::write(&source, LR_FIXTURE.as_bytes()).expect("source file");

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
        "{label}: candidate failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let assembly = fs::read_to_string(&asm)
        .unwrap_or_else(|error| panic!("{label}: cannot read {}: {error}", asm.display()));
    (assembly, output)
}

fn assert_modeled_fifteen(output: &Output, label: &str) {
    let text = String::from_utf8_lossy(&output.stdout);
    let mut lines = text.lines().filter(|line| line.starts_with("interpret: "));
    let line = lines
        .next()
        .unwrap_or_else(|| panic!("{label}: missing interpret line\nstdout: {text}"));
    assert!(
        lines.next().is_none(),
        "{label}: multiple interpret lines\nstdout: {text}"
    );
    assert_eq!(
        line, "interpret: value=0f negative=false",
        "{label}: hand-derived result is 15 (0f)\nstdout: {text}"
    );
}

/// Extract one function body by its exact label and `.size` terminator. This
/// deliberately does not reuse the emitter's layout or test helpers.
fn function_body<'a>(assembly: &'a str, name: &str) -> Vec<&'a str> {
    let label = format!("{name}:");
    let size = format!(".size {name},");
    let mut body = Vec::new();
    let mut inside = false;
    for line in assembly.lines() {
        let trimmed = line.trim();
        if !inside {
            if trimmed == label {
                inside = true;
            }
            continue;
        }
        if trimmed.starts_with(&size) {
            return body;
        }
        body.push(line);
    }
    panic!("{name}: missing function label or `.size` terminator in:\n{assembly}");
}

/// Split an assembly line into mnemonic and operand text, skipping directives,
/// labels, comments, and blanks. The returned slices borrow from `line`.
fn instruction(line: &str) -> Option<(&str, &str)> {
    let code = line.split("//").next().unwrap_or("").trim();
    if code.is_empty() || code.starts_with('.') || code.ends_with(':') {
        return None;
    }
    let mut parts = code.splitn(2, char::is_whitespace);
    let mnemonic = parts.next()?;
    let operands = parts.next().unwrap_or("").trim();
    Some((mnemonic, operands))
}

fn call_target(line: &str) -> Option<&str> {
    match instruction(line) {
        Some(("bl", target)) => Some(target),
        _ => None,
    }
}

fn mentions_x30(line: &str) -> bool {
    instruction(line)
        .map(|(mnemonic, operands)| {
            format!("{mnemonic} {operands}")
                .split(|character: char| !character.is_ascii_alphanumeric())
                .any(|token| token == "x30")
        })
        .unwrap_or(false)
}

fn stack_offset(address: &str) -> Option<u32> {
    let inner = address.strip_prefix('[')?.strip_suffix(']')?;
    let (base, offset) = inner.split_once(',')?;
    if base.trim() != "sp" {
        return None;
    }
    offset.trim().strip_prefix('#')?.parse().ok()
}

fn lr_access(line: &str) -> Option<LrAccess> {
    let (mnemonic, operands) = instruction(line)?;
    if mnemonic != "str" && mnemonic != "ldr" {
        return None;
    }
    let (register, address) = operands.split_once(',')?;
    if register.trim() != "x30" {
        return None;
    }
    let offset = stack_offset(address.trim())?;
    if mnemonic == "str" {
        Some(LrAccess::Save { offset })
    } else {
        Some(LrAccess::Restore { offset })
    }
}

/// Find one `sub sp, sp, #imm` / `add sp, sp, #imm` and report its position in
/// the function body plus the immediate. Other arithmetic is ignored.
fn stack_adjust(body: &[&str], mnemonic: &str) -> Option<(usize, u32)> {
    body.iter().enumerate().find_map(|(position, line)| {
        let (op, operands) = instruction(line)?;
        if op != mnemonic {
            return None;
        }
        let compact: String = operands
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect();
        let immediate = compact.strip_prefix("sp,sp,#")?.parse::<u32>().ok()?;
        Some((position, immediate))
    })
}

fn body_text(body: &[&str]) -> String {
    body.join("\n")
}

#[test]
fn leaf_functions_do_not_touch_lr() {
    let (assembly, output) = emit_fixture("leaf");
    assert_modeled_fifteen(&output, "leaf fixture");

    for name in ["five", "leaf", "bump"] {
        let body = function_body(&assembly, name);
        assert!(!body.is_empty(), "{name}: empty function body");
        assert!(
            body.iter().all(|line| call_target(line).is_none()),
            "{name}: leaf must not execute a call:\n{}",
            body_text(&body)
        );
        assert!(
            body.iter().all(|line| !mentions_x30(line)),
            "{name}: leaf must leave LR untouched, but mentions x30:\n{}",
            body_text(&body)
        );
        let returns = body
            .iter()
            .filter(|line| matches!(instruction(line), Some(("ret", _))))
            .count();
        assert_eq!(
            returns,
            1,
            "{name}: leaf must return exactly once:\n{}",
            body_text(&body)
        );
    }

    // `five` is the canonical constant-return shape: no frame at all.
    let five = function_body(&assembly, "five");
    assert!(
        stack_adjust(&five, "sub").is_none(),
        "five: constant leaf must not reserve a frame:\n{}",
        body_text(&five)
    );
    assert!(
        stack_adjust(&five, "add").is_none(),
        "five: constant leaf must not tear down a frame:\n{}",
        body_text(&five)
    );

    // `leaf`/`bump` have parameter and value slots, so they do own a frame.
    // That frame must not make the leaf touch LR.
    let leaf = function_body(&assembly, "leaf");
    let (_, frame) = stack_adjust(&leaf, "sub")
        .unwrap_or_else(|| panic!("leaf: expected a stack frame:\n{}", body_text(&leaf)));
    assert!(frame > 0, "leaf: frame must be nonempty");
    assert_eq!(
        stack_adjust(&leaf, "add").map(|(_, delta)| delta),
        Some(frame),
        "leaf: frame teardown must match reservation:\n{}",
        body_text(&leaf)
    );
}

#[test]
fn non_leaf_functions_save_and_restore_lr_around_every_call() {
    let (assembly, output) = emit_fixture("nonleaf");
    assert_modeled_fifteen(&output, "nonleaf fixture");

    for name in ["twice", "main"] {
        let body = function_body(&assembly, name);
        let calls: Vec<usize> = body
            .iter()
            .enumerate()
            .filter(|(_, line)| call_target(line).is_some())
            .map(|(position, _)| position)
            .collect();
        assert!(
            !calls.is_empty(),
            "{name}: expected at least one call:\n{}",
            body_text(&body)
        );

        let x30_lines: Vec<usize> = body
            .iter()
            .enumerate()
            .filter(|(_, line)| mentions_x30(line))
            .map(|(position, _)| position)
            .collect();
        assert_eq!(
            x30_lines.len(),
            2,
            "{name}: non-leaf must touch LR exactly for one save and one restore:\n{}",
            body_text(&body)
        );

        let accesses: Vec<(usize, LrAccess)> = body
            .iter()
            .enumerate()
            .filter_map(|(position, line)| lr_access(line).map(|access| (position, access)))
            .collect();
        assert_eq!(
            accesses.len(),
            x30_lines.len(),
            "{name}: every LR mention must be a parsed str/ldr access:\n{}",
            body_text(&body)
        );

        let mut save: Option<(usize, u32)> = None;
        let mut restore: Option<(usize, u32)> = None;
        for (position, access) in accesses {
            match access {
                LrAccess::Save { offset } => {
                    assert!(
                        save.is_none(),
                        "{name}: duplicate LR save at line {position}:\n{}",
                        body_text(&body)
                    );
                    save = Some((position, offset));
                }
                LrAccess::Restore { offset } => {
                    assert!(
                        restore.is_none(),
                        "{name}: duplicate LR restore at line {position}:\n{}",
                        body_text(&body)
                    );
                    restore = Some((position, offset));
                }
            }
        }
        let (save_pos, save_offset) =
            save.unwrap_or_else(|| panic!("{name}: missing LR save:\n{}", body_text(&body)));
        let (restore_pos, restore_offset) =
            restore.unwrap_or_else(|| panic!("{name}: missing LR restore:\n{}", body_text(&body)));

        let first_call = *calls.first().expect("nonempty call list");
        let last_call = *calls.last().expect("nonempty call list");
        assert!(
            save_pos < first_call,
            "{name}: LR must be saved before the first call:\n{}",
            body_text(&body)
        );
        assert!(
            restore_pos > last_call,
            "{name}: LR must be restored after the last call:\n{}",
            body_text(&body)
        );
        assert!(
            save_pos < restore_pos,
            "{name}: save must precede restore:\n{}",
            body_text(&body)
        );

        assert_eq!(
            save_offset,
            restore_offset,
            "{name}: restore must use the same LR slot as the save:\n{}",
            body_text(&body)
        );
        assert_eq!(
            save_offset % 8,
            0,
            "{name}: LR slot must be 8-byte aligned:\n{}",
            body_text(&body)
        );

        let (sub_pos, frame) = stack_adjust(&body, "sub")
            .unwrap_or_else(|| panic!("{name}: non-leaf has no frame:\n{}", body_text(&body)));
        let (add_pos, teardown) = stack_adjust(&body, "add").unwrap_or_else(|| {
            panic!(
                "{name}: non-leaf has no frame teardown:\n{}",
                body_text(&body)
            )
        });
        assert_eq!(
            frame,
            teardown,
            "{name}: frame teardown must match reservation:\n{}",
            body_text(&body)
        );
        assert_eq!(
            frame % 16,
            0,
            "{name}: frame must be 16-byte aligned:\n{}",
            body_text(&body)
        );
        assert!(
            save_pos > sub_pos,
            "{name}: LR save must be inside the reserved frame:\n{}",
            body_text(&body)
        );
        assert!(
            save_offset + 8 <= frame,
            "{name}: LR slot {save_offset} does not fit frame {frame}:\n{}",
            body_text(&body)
        );
        assert!(
            restore_pos < add_pos,
            "{name}: LR restore must precede frame teardown:\n{}",
            body_text(&body)
        );

        let returns = body
            .iter()
            .filter(|line| matches!(instruction(line), Some(("ret", _))))
            .count();
        assert_eq!(
            returns,
            1,
            "{name}: non-leaf must return exactly once:\n{}",
            body_text(&body)
        );
        let last_instruction = body
            .iter()
            .filter_map(|line| instruction(line))
            .next_back()
            .unwrap_or_else(|| panic!("{name}: empty instruction stream"));
        assert_eq!(
            last_instruction.0,
            "ret",
            "{name}: function must end in ret:\n{}",
            body_text(&body)
        );
        let ret_pos = body
            .iter()
            .enumerate()
            .filter(|(_, line)| matches!(instruction(line), Some(("ret", _))))
            .map(|(position, _)| position)
            .next_back()
            .expect("checked ret above");
        assert!(
            add_pos < ret_pos,
            "{name}: frame teardown must precede ret:\n{}",
            body_text(&body)
        );
    }
}
