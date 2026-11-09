// ============================================================================
// sprint_a05.rs — independent ABI evidence: no preserved/scratch register writes.
//
// Static evidence only. The candidate's `--emit-asm` text is inspected; no
// generated program is assembled, linked, or executed. The oracle is AAPCS64
// register ownership, not the emitter's own register choices:
//
//   * x19-x28 (and x29/fp) are callee-saved. This integer bootstrap must
//     leave them untouched; no instruction may use one as a destination.
//     A future save/restore frame design would need an explicit contract
//     change and a preservation-aware test rather than a silent exemption.
//   * x18 is the platform register; this target must not name it.
//   * x16/x17 (IP0/IP1) may be clobbered by linker veneers. The integer
//     slice has no veneer requirement, so the strict scan also rejects any
//     x16/x17 operand: writing them is unnecessary here, and reading them
//     could encode an unverifiable veneer assumption.
//
// The destination-operand parser is exercised against positive controls and
// assembly must contain recognized writes, so an empty or misparsed artifact
// cannot make the absence assertions vacuous.
// ============================================================================

use std::fs;
use std::path::PathBuf;
use std::process::Command;

struct Fixture {
    name: &'static str,
    source: &'static str,
    functions: &'static [&'static str],
    magnitude_hex: &'static str,
    negative: bool,
}

/// Fixtures are the `/55` direct-call subset. Each expected value is derived
/// by hand from the modeled signed-int semantics; the values are not taken
/// from the implementation under test.
const FIXTURES: &[Fixture] = &[
    Fixture {
        name: "canonical-constant",
        // 2 + 3 = 5; leaf with no frame or parameter stores.
        source: "int main(void) { return 2 + 3; }\n",
        functions: &["main"],
        magnitude_hex: "05",
        negative: false,
    },
    Fixture {
        name: "local-assignment",
        // x = 2; x = x + 3; return x -> 5; leaf stack slots.
        source: "int main(void) { int x = 2; x = x + 3; return x; }\n",
        functions: &["main"],
        magnitude_hex: "05",
        negative: false,
    },
    Fixture {
        name: "add-two-three",
        // add(2, 3) = 5; non-leaf caller and two parameter stores.
        source: "int add(int a, int b) { return a + b; }\n\
                 int main(void) { return add(2, 3); }\n",
        functions: &["add", "main"],
        magnitude_hex: "05",
        negative: false,
    },
    Fixture {
        name: "eight-arguments",
        // 1-2+3-4+5-6+7-8 = -4; exercises argument registers w0-w7.
        source: "int mix(int a, int b, int c, int d, int e, int f, int g, int h) {\n\
                 return a - b + c - d + e - f + g - h;\n\
                 }\n\
                 int main(void) { return mix(1, 2, 3, 4, 5, 6, 7, 8); }\n",
        functions: &["mix", "main"],
        magnitude_hex: "04",
        negative: true,
    },
    Fixture {
        name: "nested-call-arguments",
        // add(inc(2), inc(3)) = add(3, 4) = 7; two live call results.
        source: "int inc(int x) { return x + 1; }\n\
                 int add(int a, int b) { return a + b; }\n\
                 int main(void) { return add(inc(2), inc(3)); }\n",
        functions: &["inc", "add", "main"],
        magnitude_hex: "07",
        negative: false,
    },
    Fixture {
        name: "parameter-survives-call",
        // mix(2, 3) = 2 + inc(3) = 2 + 4 = 6; parameter live across a call.
        source: "int inc(int x) { return x + 1; }\n\
                 int mix(int a, int b) { return a + inc(b); }\n\
                 int main(void) { return mix(2, 3); }\n",
        functions: &["inc", "mix", "main"],
        magnitude_hex: "06",
        negative: false,
    },
    Fixture {
        name: "negative-division-remainder",
        // -7/2 = -3 and -7%2 = -1 in the modeled C truncation rules;
        // (-3 * 10) + (-1) = -31 = 0x1f. Exercises sdiv/msub and w2.
        source: "int idiv(int a, int b) { return a / b; }\n\
                 int irem(int a, int b) { return a % b; }\n\
                 int main(void) { return idiv(-7, 2) * 10 + irem(-7, 2); }\n",
        functions: &["idiv", "irem", "main"],
        magnitude_hex: "1f",
        negative: true,
    },
    Fixture {
        name: "local-mutated-by-calls",
        // x = 2; x = inc(x) twice -> 4; local slot live across calls.
        source: "int inc(int x) { return x + 1; }\n\
                 int main(void) { int x = 2; x = inc(x); x = inc(x); return x; }\n",
        functions: &["inc", "main"],
        magnitude_hex: "04",
        negative: false,
    },
    Fixture {
        name: "live-result-across-call",
        // add(2, 3) + add(4, 5) = 5 + 9 = 14 = 0x0e; first result spans a call.
        source: "int add(int a, int b) { return a + b; }\n\
                 int main(void) { return add(2, 3) + add(4, 5); }\n",
        functions: &["add", "main"],
        magnitude_hex: "0e",
        negative: false,
    },
];

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sprint-a05-{}-{}-{}",
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

/// Assembly text on an instruction line, with comments and directives
/// removed. Labels and assembler directives are not executable writes.
fn instruction_text(line: &str) -> Option<&str> {
    let text = line.split("//").next().unwrap_or("").trim();
    if text.is_empty() || text.starts_with('.') || text.starts_with('#') || text.ends_with(':') {
        return None;
    }
    Some(text)
}

/// Normalize a register operand for comparison. `!` writeback markers and
/// case are ignored; immediates, symbols, and `sp`/`wzr`/`xzr` are rejected.
fn normalized_register(operand: &str) -> Option<String> {
    let trimmed = operand.trim().trim_end_matches('!');
    let lowered = trimmed.to_ascii_lowercase();
    if lowered == "fp" {
        return Some(lowered);
    }
    let mut characters = lowered.chars();
    match characters.next() {
        Some('x') | Some('w') => {
            let digits = characters.as_str();
            if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            Some(lowered)
        }
        _ => None,
    }
}

fn register_number(register: &str) -> Option<u32> {
    let lowered = register.trim().trim_end_matches('!').to_ascii_lowercase();
    let mut characters = lowered.chars();
    match characters.next() {
        Some('x') | Some('w') => characters.as_str().parse::<u32>().ok(),
        _ => None,
    }
}

/// Callee-saved x19-x28, frame pointer x29, platform x18, and veneer scratch
/// x16/x17 are all forbidden as destinations for emitted values.
fn forbidden_class(register: &str) -> Option<&'static str> {
    if register.eq_ignore_ascii_case("fp") {
        return Some("frame pointer x29 (callee-saved)");
    }
    match register_number(register)? {
        16 | 17 => Some("veneer scratch IP0/IP1 (x16/x17)"),
        18 => Some("platform register x18"),
        19..=28 => Some("callee-saved register x19-x28"),
        29 => Some("frame pointer x29 (callee-saved)"),
        _ => None,
    }
}

fn is_platform_or_veneer(register: &str) -> bool {
    matches!(register_number(register), Some(16) | Some(17) | Some(18))
}

/// Destination registers of one instruction. Loads and general data-processing
/// instructions write their leading operand. Stores and branches write no
/// general-purpose destination here; x30 writes from `bl` are caller-saved and
/// deliberately excluded from this callee-saved/scratch check.
fn destination_registers(line: &str) -> Vec<String> {
    let Some(text) = instruction_text(line) else {
        return Vec::new();
    };
    let mut parts = text.splitn(2, char::is_whitespace);
    let mnemonic = parts.next().unwrap_or("").to_ascii_lowercase();
    let rest = parts.next().unwrap_or("").trim();
    if rest.is_empty() {
        return Vec::new();
    }
    if matches!(
        mnemonic.as_str(),
        "str"
            | "stp"
            | "cmp"
            | "cmn"
            | "tst"
            | "b"
            | "bl"
            | "br"
            | "blr"
            | "ret"
            | "cbz"
            | "cbnz"
            | "tbz"
            | "tbnz"
            | "nop"
    ) {
        return Vec::new();
    }
    let operands: Vec<&str> = rest.split(',').map(str::trim).collect();
    let count = if mnemonic == "ldp" { 2 } else { 1 };
    operands
        .iter()
        .take(count)
        .filter_map(|operand| normalized_register(operand))
        .collect()
}

/// Every register-like token on an instruction line, used only for the strict
/// platform/veneer absence check.
fn register_mentions(line: &str) -> Vec<String> {
    let Some(text) = instruction_text(line) else {
        return Vec::new();
    };
    text.split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .filter_map(normalized_register)
        .collect()
}

#[test]
fn emitted_assembly_never_writes_preserved_platform_or_veneer_registers() {
    let mut total_destination_writes = 0usize;
    for fixture in FIXTURES {
        let name = fixture.name;
        let dir = workdir(name);
        let source_path = dir.join("main.c");
        let asm_path = dir.join("module.s");
        fs::write(&source_path, fixture.source.as_bytes()).expect("write fixture source");

        let output = Command::new(binary())
            .arg("--emit-asm")
            .arg(&asm_path)
            .arg("--interpret-ir")
            .arg(&source_path)
            .output()
            .expect("candidate runs");
        assert_eq!(
            output.status.code(),
            Some(0),
            "{name}: candidate failed\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let expected = format!(
            "interpret: value={} negative={}",
            fixture.magnitude_hex, fixture.negative
        );
        let interpretations: Vec<&str> = stdout
            .lines()
            .filter(|line| line.starts_with("interpret: "))
            .collect();
        assert_eq!(
            interpretations.len(),
            1,
            "{name}: expected one interpret line\nstdout: {stdout}"
        );
        assert_eq!(
            interpretations[0],
            expected.as_str(),
            "{name}: hand-derived model mismatch\nstdout: {stdout}"
        );

        let assembly = fs::read_to_string(&asm_path)
            .unwrap_or_else(|error| panic!("{name}: cannot read emitted assembly: {error}"));
        assert!(
            !assembly.trim().is_empty(),
            "{name}: emitted assembly is empty"
        );
        for &function in fixture.functions {
            let label = format!("{function}:");
            assert!(
                assembly.lines().any(|line| line.trim() == label),
                "{name}: missing function label `{label}` in:\n{assembly}"
            );
        }

        let mut destination_writes = 0usize;
        let mut returns = 0usize;
        for line in assembly.lines() {
            if let Some(text) = instruction_text(line) {
                if text.split_whitespace().next() == Some("ret") {
                    returns += 1;
                }
            }
            for register in register_mentions(line) {
                assert!(
                    !is_platform_or_veneer(&register),
                    "{name}: emitted code names platform/veneer register `{register}`: {line}\n{assembly}"
                );
            }
            for register in destination_registers(line) {
                if let Some(class) = forbidden_class(&register) {
                    panic!(
                        "{name}: emitted code writes forbidden register `{register}` ({class}): {line}\n{assembly}"
                    );
                }
                destination_writes += 1;
            }
        }
        assert!(
            destination_writes > 0,
            "{name}: no destination operands recognized; assembly:\n{assembly}"
        );
        assert!(
            returns >= fixture.functions.len(),
            "{name}: expected at least one return per function\n{assembly}"
        );
        total_destination_writes += destination_writes;
    }
    assert!(
        total_destination_writes >= FIXTURES.len(),
        "destination analysis did not observe enough writes: {total_destination_writes}"
    );
}

#[test]
fn destination_analysis_and_scratch_scan_reject_positive_controls() {
    let forbidden_writes = [
        "mov x19, x0",
        "mov w28, w1",
        "ldr x20, [sp, #0]",
        "ldp w21, w22, [sp, #0]",
        "add x29, sp, #0",
        "mov x16, x0",
        "add x17, x0, x1",
        "ldr x18, [sp, #0]",
    ];
    for line in forbidden_writes {
        let destinations = destination_registers(line);
        assert!(
            !destinations.is_empty(),
            "parser missed destination in `{line}`"
        );
        assert!(
            destinations
                .iter()
                .any(|register| forbidden_class(register).is_some()),
            "detector missed forbidden destination in `{line}`: {destinations:?}"
        );
    }

    for register in ["x16", "w16", "x17", "w17", "x18", "w18"] {
        assert!(
            is_platform_or_veneer(register),
            "platform/veneer scan missed `{register}`"
        );
    }
    assert!(
        register_mentions("ldr w0, [x18, #0]")
            .iter()
            .any(|register| is_platform_or_veneer(register)),
        "mention scan missed x18 in a memory operand"
    );

    // Stores read their register operands; only a load/data-processing write
    // should count. These also prove directives, labels, and branches are
    // excluded rather than blindly treated as destinations.
    assert!(destination_registers("str x19, [sp, #0]").is_empty());
    assert!(destination_registers("stp x29, x30, [sp, #-16]!").is_empty());
    assert!(destination_registers("ldr x30, [sp, #8]")
        .iter()
        .all(|register| forbidden_class(register).is_none()));
    assert!(destination_registers(".globl main").is_empty());
    assert!(destination_registers(".size main, .-main").is_empty());
    assert!(destination_registers("main:").is_empty());
    assert!(destination_registers("b main").is_empty());
    assert!(destination_registers("cmp w0, w1").is_empty());
    assert!(destination_registers("# mov x19, x0").is_empty());
    assert!(destination_registers("// mov x19, x0").is_empty());
}
