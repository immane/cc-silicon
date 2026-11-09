// ============================================================================
// sprint_a07.rs — A07 nested/repeated calls and caller-value preservation.
//
// Independent oracle:
//   * `add(inc(2), inc(3))` = (2 + 1) + (3 + 1) = 7.
//   * `sum3(inc(1), inc(2), inc(3))` = (1 + 1) + (2 + 1) + (3 + 1) = 9.
//   * `combine(4, 3)` = 4 * 10 + (3 + 3) = 46.
//   * Three `x = inc(x)` assignments starting at 2 reach 5.
//   * `bump(pair(bump(1), bump(2)))` = ((1 + 1) * 10 + (2 + 1)) + 1 = 24.
//
// The structural assertions inspect the emitted AArch64 text directly and
// track caller frame slots. They are independent of the interpreter output
// consumed by the value assertion.
// ============================================================================

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const NESTED_TWO_ARGUMENTS: &str = "int inc(int x) { return x + 1; }\n\
int add(int a, int b) { return a + b; }\n\
int main(void) { return add(inc(2), inc(3)); }\n";

const NESTED_THREE_ARGUMENTS: &str = "int sum3(int a, int b, int c) { return a + b + c; }\n\
int inc(int x) { return x + 1; }\n\
int main(void) { return sum3(inc(1), inc(2), inc(3)); }\n";

const INTERMEDIATE_ACROSS_CALL: &str = "int twice(int x) { return x + x; }\n\
int combine(int a, int b) { return a * 10 + twice(b); }\n\
int main(void) { return combine(4, 3); }\n";

const REPEATED_LOCAL_UPDATES: &str = "int inc(int x) { return x + 1; }\n\
int main(void) { int x = 2; x = inc(x); x = inc(x); x = inc(x); return x; }\n";

const REPEATED_NESTED: &str = "int bump(int x) { return x + 1; }\n\
int pair(int a, int b) { return a * 10 + b; }\n\
int main(void) { return bump(pair(bump(1), bump(2))); }\n";

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
        "sprint-a07-{}-{}-{}",
        std::process::id(),
        test,
        unique_suffix()
    ));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn run_candidate(source: &str, asm: &Path) -> Output {
    let dir = asm.parent().expect("assembly parent");
    fs::create_dir_all(dir).expect("source dir");
    let source_path = dir.join("main.c");
    fs::write(&source_path, source.as_bytes()).expect("source file");
    let output = Command::new(binary())
        .arg("--emit-asm")
        .arg(asm)
        .arg("--interpret-ir")
        .arg(&source_path)
        .output()
        .expect("candidate runs");
    assert_eq!(
        output.status.code(),
        Some(0),
        "candidate failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn read_asm(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read emitted assembly {}: {error}", path.display()))
}

fn assert_interpret(output: &Output, magnitude: &str, negative: bool) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("interpret: "))
        .collect();
    assert_eq!(
        lines.len(),
        1,
        "expected exactly one interpret line\nstdout: {stdout}"
    );
    let rest = lines[0]
        .strip_prefix("interpret: value=")
        .unwrap_or_else(|| panic!("malformed interpret line: {}", lines[0]));
    let (value, sign) = rest
        .split_once(" negative=")
        .unwrap_or_else(|| panic!("malformed interpret line: {}", lines[0]));
    assert_eq!(value, magnitude, "modeled magnitude\nstdout: {stdout}");
    assert_eq!(
        sign.parse::<bool>().expect("negative flag is a boolean"),
        negative,
        "modeled sign\nstdout: {stdout}"
    );
}

fn function_body(assembly: &str, name: &str) -> Vec<String> {
    let lines: Vec<&str> = assembly.lines().collect();
    let label = format!("{name}:");
    let start = lines
        .iter()
        .position(|line| line.trim() == label)
        .unwrap_or_else(|| panic!("missing function `{name}` in:\n{assembly}"));
    let mut body = Vec::new();
    for line in lines.iter().skip(start + 1) {
        if line.trim().starts_with(".size ") {
            break;
        }
        body.push((*line).to_string());
    }
    assert!(!body.is_empty(), "empty body for `{name}`:\n{assembly}");
    body
}

fn call_target(line: &str) -> Option<&str> {
    let text = line.split("//").next().unwrap_or(line).trim();
    let mut parts = text.split_whitespace();
    if parts.next() != Some("bl") {
        return None;
    }
    parts.next()
}

fn calls_in(body: &[String]) -> Vec<(usize, &str)> {
    body.iter()
        .enumerate()
        .filter_map(|(index, line)| call_target(line).map(|target| (index, target)))
        .collect()
}

fn stack_slot(line: &str, mnemonic: &str, register: &str) -> Option<u32> {
    let text = line.split("//").next().unwrap_or(line).trim();
    let prefix = format!("{mnemonic} {register}, [sp, #");
    let rest = text.strip_prefix(&prefix)?;
    rest.strip_suffix(']')?.parse().ok()
}

fn stack_adjust(line: &str, mnemonic: &str) -> Option<u32> {
    let text = line.split("//").next().unwrap_or(line).trim();
    let rest = text.strip_prefix(mnemonic)?.trim_start();
    rest.strip_prefix("sp, sp, #")?.parse().ok()
}

fn spilled_result(body: &[String], call_index: usize) -> u32 {
    let line = body
        .get(call_index + 1)
        .unwrap_or_else(|| panic!("missing spill after call at line {call_index}"));
    stack_slot(line, "str", "w0")
        .unwrap_or_else(|| panic!("call at line {call_index} does not spill w0: {line}"))
}

fn first_store_after(body: &[String], index: usize) -> (usize, u32) {
    for (store_index, line) in body.iter().enumerate().skip(index + 1) {
        if let Some(slot) = stack_slot(line, "str", "w0") {
            return (store_index, slot);
        }
    }
    panic!("no store after line {index}:\n{}", body.join("\n"));
}

fn slot_accesses(body: &[String], mnemonic: &str, register: &str, slot: u32) -> usize {
    body.iter()
        .filter(|line| stack_slot(line, mnemonic, register) == Some(slot))
        .count()
}

fn assert_distinct(slots: &[u32], context: &str) {
    for (index, slot) in slots.iter().enumerate() {
        for other in &slots[index + 1..] {
            assert_ne!(slot, other, "{context}: shared frame slot {slot}");
        }
    }
}

fn frame_bounds(body: &[String], context: &str) -> (usize, usize) {
    let mut sub = None;
    let mut add = None;
    for (index, line) in body.iter().enumerate() {
        if let Some(amount) = stack_adjust(line, "sub") {
            assert!(
                sub.is_none(),
                "{context}: duplicate stack subtraction:\n{}",
                body.join("\n")
            );
            sub = Some((index, amount));
        }
        if let Some(amount) = stack_adjust(line, "add") {
            assert!(
                add.is_none(),
                "{context}: duplicate stack restoration:\n{}",
                body.join("\n")
            );
            add = Some((index, amount));
        }
    }
    let (sub_index, sub_amount) =
        sub.unwrap_or_else(|| panic!("{context}: missing stack subtraction"));
    let (add_index, add_amount) =
        add.unwrap_or_else(|| panic!("{context}: missing stack restoration"));
    assert!(
        sub_index < add_index,
        "{context}: stack restored before use:\n{}",
        body.join("\n")
    );
    assert_eq!(
        sub_amount,
        add_amount,
        "{context}: unbalanced stack frame:\n{}",
        body.join("\n")
    );
    (sub_index, add_index)
}

#[test]
fn nested_two_argument_call_spills_both_results_before_the_outer_call() {
    let dir = workdir("nested-two");
    let assembly_path = dir.join("main.s");
    let output = run_candidate(NESTED_TWO_ARGUMENTS, &assembly_path);
    assert_interpret(&output, "07", false);

    let assembly = read_asm(&assembly_path);
    let main = function_body(&assembly, "main");
    let calls = calls_in(&main);
    assert_eq!(
        calls.iter().map(|(_, target)| *target).collect::<Vec<_>>(),
        vec!["inc", "inc", "add"],
        "nested call order:\n{assembly}"
    );

    let nested = [
        spilled_result(&main, calls[0].0),
        spilled_result(&main, calls[1].0),
    ];
    assert_distinct(&nested, "nested call results");
    assert!(
        calls[0].0 < calls[1].0 && calls[1].0 < calls[2].0,
        "nested calls reordered:\n{assembly}"
    );

    let outer = calls[2].0;
    assert!(outer >= 2, "outer call lacks argument setup:\n{assembly}");
    assert_eq!(
        stack_slot(&main[outer - 2], "ldr", "w0"),
        Some(nested[0]),
        "first argument is not reloaded from its caller slot:\n{assembly}"
    );
    assert_eq!(
        stack_slot(&main[outer - 1], "ldr", "w1"),
        Some(nested[1]),
        "second argument is not reloaded from its caller slot:\n{assembly}"
    );
}

#[test]
fn three_nested_arguments_keep_three_distinct_caller_slots() {
    let dir = workdir("nested-three");
    let assembly_path = dir.join("main.s");
    let output = run_candidate(NESTED_THREE_ARGUMENTS, &assembly_path);
    assert_interpret(&output, "09", false);

    let assembly = read_asm(&assembly_path);
    let main = function_body(&assembly, "main");
    let calls = calls_in(&main);
    assert_eq!(
        calls.iter().map(|(_, target)| *target).collect::<Vec<_>>(),
        vec!["inc", "inc", "inc", "sum3"],
        "nested call order:\n{assembly}"
    );

    let nested = [
        spilled_result(&main, calls[0].0),
        spilled_result(&main, calls[1].0),
        spilled_result(&main, calls[2].0),
    ];
    assert_distinct(&nested, "three nested call results");

    let outer = calls[3].0;
    assert!(outer >= 3, "outer call lacks argument setup:\n{assembly}");
    assert_eq!(
        stack_slot(&main[outer - 3], "ldr", "w0"),
        Some(nested[0]),
        "argument zero is not reloaded in source order:\n{assembly}"
    );
    assert_eq!(
        stack_slot(&main[outer - 2], "ldr", "w1"),
        Some(nested[1]),
        "argument one is not reloaded in source order:\n{assembly}"
    );
    assert_eq!(
        stack_slot(&main[outer - 1], "ldr", "w2"),
        Some(nested[2]),
        "argument two is not reloaded in source order:\n{assembly}"
    );
}

#[test]
fn caller_intermediate_survives_a_later_call() {
    let dir = workdir("live-intermediate");
    let assembly_path = dir.join("main.s");
    let output = run_candidate(INTERMEDIATE_ACROSS_CALL, &assembly_path);
    assert_interpret(&output, "2e", false);

    let assembly = read_asm(&assembly_path);
    let combine = function_body(&assembly, "combine");
    let mul = combine
        .iter()
        .position(|line| line.trim() == "mul w0, w0, w1")
        .unwrap_or_else(|| panic!("combine has no multiply:\n{assembly}"));
    let (live_store, live_slot) = first_store_after(&combine, mul);
    let call = combine
        .iter()
        .position(|line| call_target(line) == Some("twice"))
        .unwrap_or_else(|| panic!("combine does not call twice:\n{assembly}"));
    assert!(
        mul < live_store && live_store < call,
        "live value is not spilled before the call:\n{assembly}"
    );

    let result_slot = spilled_result(&combine, call);
    assert_distinct(
        &[live_slot, result_slot],
        "live caller value versus call result",
    );

    let sum = combine
        .iter()
        .position(|line| line.trim() == "add w0, w0, w1")
        .unwrap_or_else(|| panic!("combine has no final add:\n{assembly}"));
    assert!(
        call + 2 < sum,
        "final add does not follow the call:\n{assembly}"
    );
    assert_eq!(
        stack_slot(&combine[sum - 2], "ldr", "w0"),
        Some(live_slot),
        "the caller intermediate is not reloaded after the call:\n{assembly}"
    );
    assert_eq!(
        stack_slot(&combine[sum - 1], "ldr", "w1"),
        Some(result_slot),
        "the call result is not reloaded:\n{assembly}"
    );

    assert!(
        live_slot != 0 && live_slot != 4 && result_slot != 0 && result_slot != 4,
        "call traffic aliases a caller parameter slot:\n{assembly}"
    );
    assert_eq!(
        slot_accesses(&combine, "str", "w0", 0),
        1,
        "parameter `a` is written more than once:\n{assembly}"
    );
    assert_eq!(
        slot_accesses(&combine, "str", "w1", 4),
        1,
        "parameter `b` is written more than once:\n{assembly}"
    );

    let (sub, restore) = frame_bounds(&combine, "combine");
    let lr_save = combine
        .iter()
        .position(|line| line.trim_start().starts_with("str x30, [sp,"))
        .unwrap_or_else(|| panic!("combine does not save LR:\n{assembly}"));
    let lr_restore = combine
        .iter()
        .position(|line| line.trim_start().starts_with("ldr x30, [sp,"))
        .unwrap_or_else(|| panic!("combine does not restore LR:\n{assembly}"));
    assert!(
        sub < lr_save && lr_save < call && call < lr_restore && lr_restore < restore,
        "caller frame or LR is not live across the call:\n{assembly}"
    );
}

#[test]
fn repeated_calls_reload_the_mutated_local_from_its_caller_slot() {
    let dir = workdir("repeated-local");
    let assembly_path = dir.join("main.s");
    let output = run_candidate(REPEATED_LOCAL_UPDATES, &assembly_path);
    assert_interpret(&output, "05", false);

    let assembly = read_asm(&assembly_path);
    let main = function_body(&assembly, "main");
    let calls = calls_in(&main);
    assert_eq!(calls.len(), 3, "expected three calls:\n{assembly}");
    assert!(
        calls.iter().all(|(_, target)| *target == "inc"),
        "unexpected callee:\n{assembly}"
    );

    let mut x_slot: Option<u32> = None;
    let mut argument_slots = Vec::new();
    let mut result_slots = Vec::new();
    for &(call, _) in &calls {
        assert!(
            call >= 3 && call + 3 < main.len(),
            "unexpected call shape at line {call}:\n{assembly}"
        );
        let before = stack_slot(&main[call - 3], "ldr", "w0")
            .unwrap_or_else(|| panic!("missing local reload before call {call}:\n{assembly}"));
        let argument_store = stack_slot(&main[call - 2], "str", "w0")
            .unwrap_or_else(|| panic!("missing argument spill before call {call}:\n{assembly}"));
        let argument_load = stack_slot(&main[call - 1], "ldr", "w0")
            .unwrap_or_else(|| panic!("missing argument reload before call {call}:\n{assembly}"));
        assert_eq!(
            argument_store, argument_load,
            "argument is not loaded from its spill slot at call {call}:\n{assembly}"
        );
        match x_slot {
            Some(slot) => assert_eq!(
                slot, before,
                "call {call} reads a different local slot:\n{assembly}"
            ),
            None => x_slot = Some(before),
        }

        let result_store = stack_slot(&main[call + 1], "str", "w0")
            .unwrap_or_else(|| panic!("missing result spill after call {call}:\n{assembly}"));
        let result_load = stack_slot(&main[call + 2], "ldr", "w0")
            .unwrap_or_else(|| panic!("missing result reload after call {call}:\n{assembly}"));
        assert_eq!(
            result_store, result_load,
            "call result is not loaded from its spill slot at call {call}:\n{assembly}"
        );
        assert_eq!(
            stack_slot(&main[call + 3], "str", "w0"),
            Some(before),
            "call result is not stored back into the local at call {call}:\n{assembly}"
        );

        argument_slots.push(argument_store);
        result_slots.push(result_store);
    }

    let x = x_slot.expect("no local slot observed");
    assert_distinct(&argument_slots, "repeated call arguments");
    assert_distinct(&result_slots, "repeated call results");
    assert!(
        argument_slots.iter().all(|slot| *slot != x),
        "an argument clobbers the caller local slot:\n{assembly}"
    );
    assert!(
        result_slots.iter().all(|slot| *slot != x),
        "a call result clobbers the caller local slot:\n{assembly}"
    );
    assert_eq!(
        slot_accesses(&main, "str", "w0", x),
        4,
        "expected one local initialization and three local updates:\n{assembly}"
    );
    assert_eq!(
        slot_accesses(&main, "ldr", "w0", x),
        4,
        "expected three argument reads and one return read of the local:\n{assembly}"
    );

    let lr = main
        .iter()
        .position(|line| line.trim_start().starts_with("ldr x30, [sp,"))
        .unwrap_or_else(|| panic!("main does not restore LR:\n{assembly}"));
    assert!(lr >= 3, "return shape is too short:\n{assembly}");
    let returned = stack_slot(&main[lr - 1], "ldr", "w0")
        .unwrap_or_else(|| panic!("return does not load its value slot:\n{assembly}"));
    assert_eq!(
        stack_slot(&main[lr - 2], "str", "w0"),
        Some(returned),
        "return value is not spilled by the final local load:\n{assembly}"
    );
    assert_eq!(
        stack_slot(&main[lr - 3], "ldr", "w0"),
        Some(x),
        "return does not read the mutated caller local:\n{assembly}"
    );

    let (sub, restore) = frame_bounds(&main, "main");
    assert!(
        sub < calls[0].0 && calls[2].0 < restore,
        "caller frame is not live across all calls:\n{assembly}"
    );
}

#[test]
fn three_level_repeated_nesting_feeds_each_call_from_the_caller_frame() {
    let dir = workdir("repeated-nested");
    let assembly_path = dir.join("main.s");
    let output = run_candidate(REPEATED_NESTED, &assembly_path);
    assert_interpret(&output, "18", false);

    let assembly = read_asm(&assembly_path);
    let main = function_body(&assembly, "main");
    let calls = calls_in(&main);
    assert_eq!(
        calls.iter().map(|(_, target)| *target).collect::<Vec<_>>(),
        vec!["bump", "bump", "pair", "bump"],
        "repeated nested call order:\n{assembly}"
    );

    let inner = [
        spilled_result(&main, calls[0].0),
        spilled_result(&main, calls[1].0),
    ];
    assert_distinct(&inner, "inner repeated bump results");

    let pair_call = calls[2].0;
    assert!(pair_call >= 2, "pair call lacks arguments:\n{assembly}");
    assert_eq!(
        stack_slot(&main[pair_call - 2], "ldr", "w0"),
        Some(inner[0]),
        "pair does not reload the first inner bump result:\n{assembly}"
    );
    assert_eq!(
        stack_slot(&main[pair_call - 1], "ldr", "w1"),
        Some(inner[1]),
        "pair does not reload the second inner bump result:\n{assembly}"
    );

    let pair_result = spilled_result(&main, pair_call);
    let outer = calls[3].0;
    assert_eq!(
        stack_slot(&main[outer - 1], "ldr", "w0"),
        Some(pair_result),
        "outer bump does not reload the pair result:\n{assembly}"
    );

    let outer_result = spilled_result(&main, outer);
    let lr = main
        .iter()
        .position(|line| line.trim_start().starts_with("ldr x30, [sp,"))
        .unwrap_or_else(|| panic!("main does not restore LR:\n{assembly}"));
    assert_eq!(
        stack_slot(&main[lr - 1], "ldr", "w0"),
        Some(outer_result),
        "return does not read the outer bump result:\n{assembly}"
    );

    let (sub, restore) = frame_bounds(&main, "main");
    assert!(
        sub < calls[0].0 && outer < restore,
        "caller frame is not live across the nested calls:\n{assembly}"
    );
}
