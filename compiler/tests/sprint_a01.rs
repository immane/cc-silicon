// ============================================================================
// sprint_a01.rs — independent A01 ABI evidence: argument register locations
// for the covered 0–8 `int` parameter subset, with distinct positional values.
//
// The test builds synthetic committed IR (no C source, no GCC invocation) and
// runs the existing host-only emitter as the implementation under test. The
// expected locations are hand-derived from AAPCS64, not from the emitter:
// integer arguments arrive in w0..w7 in parameter order. The values at each
// position are distinct primes (7, 11, 13, 17, 19, 23, 29, 31), so a swap,
// drop, or duplicate cannot produce the expected caller register trace.
//
// Two independent observations are combined:
//   * caller: the value stored for positional argument i is loaded into w{i}
//     immediately before `bl`, tracked through the emitted constant slots;
//   * callee: the entry saves are exactly the incoming registers w0..w{n-1},
//     and parameter i reloads the slot that received w{i}.
//
// This is assembly-text evidence only. It does not execute the program, does
// not use a reference compiler on candidate C, and does not extend the claim
// beyond the covered int-only subset.
// ============================================================================

#[path = "../src/bin/candidate/target_m1.rs"]
mod target_m1;

use cc_silicon_compiler::bus::{
    BlockRecord, CompilerBus, ConstRecord, ConstValue, FunctionRecord, InstructionRecord, IntRank,
    IntegerConst, IrOp, Linkage, ScopeKind, ScopeRecord, StorageDuration, SymbolKind, SymbolRecord,
    TypeKind, TypeRecord, ValueRecord,
};
use cc_silicon_compiler::ids::{
    BlockId, ConstId, NameId, NodeId, ScopeId, SymbolId, TypeId, ValueId,
};
use std::collections::BTreeMap;

/// Hand-selected distinct positional values for arguments 0..7.
const POSITIONAL_VALUES: [i32; 8] = [7, 11, 13, 17, 19, 23, 29, 31];

struct Module {
    bus: CompilerBus,
    int: TypeId,
    file_scope: ScopeId,
    next_decl: u32,
}

#[derive(Clone)]
struct Function {
    symbol: SymbolId,
    block: BlockId,
    parameters: Vec<SymbolId>,
}

impl Module {
    fn new() -> Self {
        let mut bus = CompilerBus::default();
        let limits = bus.limits();
        let int = bus
            .arenas
            .types
            .alloc(
                TypeRecord {
                    kind: TypeKind::Int {
                        rank: IntRank::Int,
                        signed: true,
                    },
                },
                &limits,
            )
            .unwrap();
        let file_scope = bus
            .arenas
            .scopes
            .alloc(
                ScopeRecord {
                    kind: ScopeKind::File,
                    parent: None,
                    owner: None,
                },
                &limits,
            )
            .unwrap();
        Self {
            bus,
            int,
            file_scope,
            next_decl: 0,
        }
    }

    fn next_node(&mut self) -> NodeId {
        let node = NodeId::from_index(self.next_decl);
        self.next_decl += 1;
        node
    }

    fn intern(&mut self, name: &str) -> NameId {
        let limits = self.bus.limits();
        self.bus.intern.intern(name.as_bytes(), &limits).unwrap()
    }

    fn declare(&mut self, name: &str, parameter_names: &[&str], linkage: Linkage) -> Function {
        let symbol_name = self.intern(name);
        let mut parameter_name_ids = Vec::new();
        for parameter_name in parameter_names {
            parameter_name_ids.push(self.intern(parameter_name));
        }

        let limits = self.bus.limits();
        let signature = self
            .bus
            .arenas
            .types
            .alloc(
                TypeRecord {
                    kind: TypeKind::Function {
                        result: self.int,
                        params: vec![self.int; parameter_names.len()],
                        prototype: true,
                        variadic: false,
                    },
                },
                &limits,
            )
            .unwrap();

        let scope_owner = self.next_node();
        let limits = self.bus.limits();
        let scope = self
            .bus
            .arenas
            .scopes
            .alloc(
                ScopeRecord {
                    kind: ScopeKind::Block,
                    parent: Some(self.file_scope),
                    owner: Some(scope_owner),
                },
                &limits,
            )
            .unwrap();

        let mut parameters = Vec::new();
        for parameter_name in parameter_name_ids {
            let decl = self.next_node();
            let limits = self.bus.limits();
            let parameter = self
                .bus
                .arenas
                .symbols
                .alloc(
                    SymbolRecord {
                        name: parameter_name,
                        scope,
                        kind: SymbolKind::Object,
                        ty: Some(self.int),
                        linkage: Linkage::None,
                        storage: StorageDuration::Automatic,
                        decl,
                    },
                    &limits,
                )
                .unwrap();
            parameters.push(parameter);
        }

        let decl = self.next_node();
        let limits = self.bus.limits();
        let symbol = self
            .bus
            .arenas
            .symbols
            .alloc(
                SymbolRecord {
                    name: symbol_name,
                    scope: self.file_scope,
                    kind: SymbolKind::Function,
                    ty: Some(signature),
                    linkage,
                    storage: StorageDuration::Static,
                    decl,
                },
                &limits,
            )
            .unwrap();

        let entry = BlockId::from_index(self.bus.arenas.blocks.allocated());
        let limits = self.bus.limits();
        let function = self
            .bus
            .arenas
            .functions
            .alloc(
                FunctionRecord {
                    symbol,
                    signature,
                    entry,
                    linkage,
                    parameters: parameters.clone(),
                },
                &limits,
            )
            .unwrap();
        let limits = self.bus.limits();
        self.bus
            .arenas
            .blocks
            .alloc(
                BlockRecord {
                    function,
                    ordinal: 0,
                },
                &limits,
            )
            .unwrap();

        Function {
            symbol,
            block: entry,
            parameters,
        }
    }

    fn body(&mut self, function: &Function) -> Body<'_> {
        Body {
            module: self,
            function: function.clone(),
        }
    }
}

struct Body<'a> {
    module: &'a mut Module,
    function: Function,
}

impl Body<'_> {
    fn value(&mut self) -> ValueId {
        let limits = self.module.bus.limits();
        self.module
            .bus
            .arenas
            .values
            .alloc(
                ValueRecord {
                    ty: self.module.int,
                },
                &limits,
            )
            .unwrap()
    }

    fn instruction(
        &mut self,
        op: IrOp,
        operands: Vec<ValueId>,
        immediate: Option<ConstId>,
        result: Option<ValueId>,
        local: Option<SymbolId>,
        callee: Option<SymbolId>,
    ) {
        let limits = self.module.bus.limits();
        self.module
            .bus
            .arenas
            .instructions
            .alloc(
                InstructionRecord {
                    op,
                    block: self.function.block,
                    operands,
                    immediate,
                    result,
                    local,
                    successors: vec![],
                    callee,
                },
                &limits,
            )
            .unwrap();
    }

    fn constant(&mut self, value: i32) -> ValueId {
        let limits = self.module.bus.limits();
        let constant = self
            .module
            .bus
            .arenas
            .consts
            .alloc(
                ConstRecord {
                    ty: self.module.int,
                    value: ConstValue::Integer(IntegerConst::new(
                        value.unsigned_abs().to_be_bytes().to_vec(),
                        value < 0,
                    )),
                    origins: vec![],
                },
                &limits,
            )
            .unwrap();
        let result = self.value();
        self.instruction(
            IrOp::Constant,
            vec![],
            Some(constant),
            Some(result),
            None,
            None,
        );
        result
    }

    fn load_parameter(&mut self, index: usize) -> ValueId {
        let local = self.function.parameters[index];
        let result = self.value();
        self.instruction(
            IrOp::LocalLoad,
            vec![],
            None,
            Some(result),
            Some(local),
            None,
        );
        result
    }

    fn call(&mut self, callee: SymbolId, arguments: Vec<ValueId>) -> ValueId {
        let result = self.value();
        self.instruction(
            IrOp::Call,
            arguments,
            None,
            Some(result),
            None,
            Some(callee),
        );
        result
    }

    fn ret(self, value: ValueId) {
        let mut body = self;
        body.instruction(IrOp::Return, vec![value], None, None, None, None);
    }
}

/// Build and emit one module: `fN` takes N parameters and main calls it with
/// the first N distinct positional values. The callee loads every parameter
/// and returns the last one, so all parameter slots are exercised.
fn emit_arity(parameter_count: usize) -> String {
    let mut module = Module::new();
    let parameter_names: Vec<String> = (0..parameter_count)
        .map(|index| format!("p{index}"))
        .collect();
    let parameter_name_refs: Vec<&str> = parameter_names.iter().map(String::as_str).collect();
    let callee = module.declare(
        &format!("f{parameter_count}"),
        &parameter_name_refs,
        Linkage::External,
    );

    {
        let mut body = module.body(&callee);
        let result = if parameter_count == 0 {
            body.constant(0)
        } else {
            let mut last = None;
            for index in 0..parameter_count {
                last = Some(body.load_parameter(index));
            }
            last.expect("at least one parameter")
        };
        body.ret(result);
    }

    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let mut arguments = Vec::new();
        for value in POSITIONAL_VALUES.iter().take(parameter_count) {
            arguments.push(body.constant(*value));
        }
        let result = body.call(callee.symbol, arguments);
        body.ret(result);
    }

    let assembly = target_m1::emit_m1_assembly(&module.bus)
        .unwrap_or_else(|error| panic!("arity {parameter_count}: {error}"));
    String::from_utf8(assembly).expect("emitted assembly is UTF-8")
}

fn function_text<'a>(assembly: &'a str, name: &str) -> &'a str {
    let label = format!("\n{name}:\n");
    let start = assembly
        .find(&label)
        .map(|index| index + 1)
        .unwrap_or_else(|| panic!("missing function label `{name}` in:\n{assembly}"));
    let rest = &assembly[start..];
    let end = rest
        .find(&format!(".size {name}, .-{name}"))
        .unwrap_or_else(|| panic!("missing .size for `{name}` in:\n{assembly}"));
    &rest[..end]
}

fn instruction_lines(body: &str) -> Vec<&str> {
    body.lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !line.starts_with('.')
                && !line.starts_with('#')
                && !line.ends_with(':')
        })
        .collect()
}

fn split_mnemonic(line: &str) -> (&str, &str) {
    let mut parts = line.splitn(2, char::is_whitespace);
    let mnemonic = parts.next().unwrap_or("");
    let operands = parts.next().unwrap_or("").trim();
    (mnemonic, operands)
}

fn first_operand(operands: &str) -> &str {
    operands.split(',').next().unwrap_or("").trim()
}

fn w_register(text: &str) -> Option<u8> {
    text.strip_prefix('w')?.parse::<u8>().ok()
}

/// Parse the `[sp, #N]` operand of a load or store.
fn stack_slot(operands: &str) -> Option<u32> {
    let open = operands.find('[')?;
    let close = operands[open..].find(']')? + open;
    let inner = &operands[open + 1..close];
    let mut parts = inner.split(',');
    if parts.next()?.trim() != "sp" {
        return None;
    }
    parts.next()?.trim().strip_prefix('#')?.parse().ok()
}

/// Parse `w0, #imm[, lsl #shift]` as emitted by `movz`/`movk`.
fn parse_move(operands: &str) -> Option<(u8, u32, u32)> {
    let mut parts = operands.split(',').map(str::trim);
    let register = w_register(parts.next()?)?;
    let immediate = parts.next()?.strip_prefix('#')?.parse::<u32>().ok()?;
    let shift = match parts.next() {
        None => 0,
        Some(specification) => {
            let mut tokens = specification.split_whitespace();
            if tokens.next()? != "lsl" {
                return None;
            }
            tokens.next()?.strip_prefix('#')?.parse::<u32>().ok()?
        }
    };
    if immediate > 0xffff || shift > 48 || parts.next().is_some() {
        return None;
    }
    Some((register, immediate, shift))
}

/// Assert that, immediately before `bl callee`, positional argument i is in
/// w{i} with the hand-derived value `expected[i]`.
fn assert_call_registers(assembly: &str, caller: &str, callee: &str, expected: &[i32]) {
    let body = function_text(assembly, caller);
    let lines = instruction_lines(body);
    let calls: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| {
            let (mnemonic, operands) = split_mnemonic(line);
            mnemonic == "bl" && first_operand(operands) == callee
        })
        .map(|(index, _)| index)
        .collect();
    assert_eq!(
        calls.len(),
        1,
        "`{caller}` must call `{callee}` exactly once:\n{body}"
    );
    let call_index = calls[0];

    let mut slot_values: BTreeMap<u32, i32> = BTreeMap::new();
    let mut pending: Option<u32> = None;
    let mut loaded: BTreeMap<u8, Option<i32>> = BTreeMap::new();

    for line in &lines[..call_index] {
        let (mnemonic, operands) = split_mnemonic(line);
        match mnemonic {
            "movz" => {
                if let Some((0, immediate, shift)) = parse_move(operands) {
                    pending = Some((immediate & 0xffff) << shift);
                }
            }
            "movk" => {
                if let Some((0, immediate, shift)) = parse_move(operands) {
                    let bits = pending.unwrap_or_default();
                    let mask = 0xffffu32 << shift;
                    pending = Some((bits & !mask) | ((immediate & 0xffff) << shift));
                }
            }
            "str" => {
                if let (Some(0), Some(bits), Some(slot)) = (
                    w_register(first_operand(operands)),
                    pending,
                    stack_slot(operands),
                ) {
                    slot_values.insert(slot, bits as i32);
                }
                pending = None;
            }
            "ldr" => {
                if let (Some(register), Some(slot)) =
                    (w_register(first_operand(operands)), stack_slot(operands))
                {
                    let value = slot_values.get(&slot).copied();
                    assert!(
                        loaded.insert(register, value).is_none(),
                        "`{caller}` loads argument register w{register} twice before `bl {callee}`:\n{body}"
                    );
                }
            }
            _ => pending = None,
        }
    }

    assert_eq!(
        loaded.len(),
        expected.len(),
        "`{caller}` must set exactly {} argument registers before `bl {callee}`:\n{body}",
        expected.len()
    );
    for (index, value) in expected.iter().enumerate() {
        let register = u8::try_from(index).expect("argument index fits in u8");
        assert_eq!(
            loaded.get(&register).copied().flatten(),
            Some(*value),
            "argument {index} must travel in w{register} with value {value}; \
             traced register values: {loaded:?}\n{body}"
        );
    }
}

/// Assert that the callee entry saves exactly w0..w{n-1} and that parameter i
/// is reloaded from the slot that received incoming register w{i}.
fn assert_callee_parameters(assembly: &str, callee: &str, parameter_count: usize) {
    let body = function_text(assembly, callee);
    let lines = instruction_lines(body);

    // Prologue shape: optional `sub sp`, optional LR save, then one store per
    // incoming argument register. Body instructions start after those stores.
    let mut index = 0;
    let starts_with_sub = lines.get(index).is_some_and(|line| {
        let (mnemonic, operands) = split_mnemonic(line);
        mnemonic == "sub" && operands.starts_with("sp, sp,")
    });
    if starts_with_sub {
        index += 1;
    }
    let saves_lr = lines.get(index).is_some_and(|line| {
        let (mnemonic, operands) = split_mnemonic(line);
        mnemonic == "str" && first_operand(operands) == "x30"
    });
    if saves_lr {
        index += 1;
    }

    let mut saved: BTreeMap<u8, u32> = BTreeMap::new();
    while let Some(line) = lines.get(index) {
        let (mnemonic, operands) = split_mnemonic(line);
        if mnemonic != "str" {
            break;
        }
        let Some(register) = w_register(first_operand(operands)) else {
            break;
        };
        let Some(slot) = stack_slot(operands) else {
            break;
        };
        assert!(
            saved.insert(register, slot).is_none(),
            "`{callee}` saves incoming w{register} twice:\n{body}"
        );
        index += 1;
    }

    assert_eq!(
        saved.len(),
        parameter_count,
        "`{callee}` must save exactly {parameter_count} incoming argument registers:\n{body}"
    );
    for position in 0..parameter_count {
        let register = u8::try_from(position).expect("parameter index fits in u8");
        assert!(
            saved.contains_key(&register),
            "`{callee}` must take parameter {position} from w{register}; saved {saved:?}:\n{body}"
        );
    }

    if parameter_count == 0 {
        return;
    }

    let mut parameter_loads = Vec::new();
    for line in &lines[index..] {
        let (mnemonic, operands) = split_mnemonic(line);
        if let ("ldr", Some(0), Some(slot)) = (
            mnemonic,
            w_register(first_operand(operands)),
            stack_slot(operands),
        ) {
            parameter_loads.push(slot);
        }
    }
    assert!(
        parameter_loads.len() >= parameter_count,
        "`{callee}` does not reload all parameters:\n{body}"
    );
    for (position, load) in parameter_loads.iter().take(parameter_count).enumerate() {
        let register = u8::try_from(position).expect("parameter index fits in u8");
        assert_eq!(
            *load, saved[&register],
            "parameter {position} must be read from the slot holding incoming w{register}:\n{body}"
        );
    }
}

#[test]
fn zero_through_eight_arguments_use_distinct_positional_registers() {
    for parameter_count in 0..=8usize {
        let assembly = emit_arity(parameter_count);
        let callee = format!("f{parameter_count}");
        let label = format!("{callee}:");
        assert!(
            assembly.lines().any(|line| line.trim() == label.as_str()),
            "arity {parameter_count}: missing `{label}` label:\n{assembly}"
        );
        assert_callee_parameters(&assembly, &callee, parameter_count);
        assert_call_registers(
            &assembly,
            "main",
            &callee,
            &POSITIONAL_VALUES[..parameter_count],
        );
    }
}
