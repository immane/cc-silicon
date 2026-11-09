// ============================================================================
// candidate/target_m1.rs — host-only AArch64 assembly bootstrap
//
// Consumes a committed single-block integer IR function and emits GNU AArch64
// assembly. The first slice handled only Constant+Return; the current host
// slice adds automatic locals, local load/store, integer arithmetic, and one
// return. Calls, memory, branches, and general ABI lowering still fail
// closed. No contract freeze is implied by this bootstrap.
// ============================================================================

use cc_silicon_compiler::bus::{
    ConstValue, InstructionRecord, IntRank, IrOp, StorageDuration, SymbolRecord, TypeKind,
};
use cc_silicon_compiler::ids::{ConstId, SymbolId, TypeId, ValueId};
use std::collections::{BTreeMap, BTreeSet};

/// Emit GNU AArch64 assembly for the supported committed IR shape.
pub fn emit_m1_assembly(bus: &cc_silicon_compiler::bus::CompilerBus) -> Result<Vec<u8>, String> {
    let mut functions = bus.arenas.functions.iter();
    let Some((_, function)) = functions.next() else {
        return Err("target emit: missing IR function".to_string());
    };
    if functions.next().is_some() {
        return Err("target emit: more than one IR function".to_string());
    }
    let mut blocks = bus.arenas.blocks.iter();
    let Some((block_id, _block)) = blocks.next() else {
        return Err("target emit: missing IR block".to_string());
    };
    if blocks.next().is_some() || function.entry != block_id {
        return Err("target emit: expected one entry block".to_string());
    }
    let int = int_type(bus)?;
    let signature = bus
        .arenas
        .types
        .get(function.signature)
        .map_err(|_| "target emit: missing function signature type".to_string())?;
    let TypeKind::Function {
        result,
        params,
        prototype,
        variadic,
    } = &signature.kind
    else {
        return Err("target emit: signature is not a function type".to_string());
    };
    if !params.is_empty() || !*prototype || *variadic || *result != int {
        return Err("target emit: only int(void) is supported".to_string());
    }

    let instructions: Vec<(cc_silicon_compiler::ids::InstructionId, &InstructionRecord)> =
        bus.arenas.instructions.iter().collect();
    if instructions.is_empty() {
        return Err("target emit: missing Constant instruction".to_string());
    }
    if instructions.len() == 2 {
        let (first_id, first) = instructions[0];
        let (second_id, second) = instructions[1];
        if first.op == IrOp::Constant
            && second.op == IrOp::Return
            && first.local.is_none()
            && first.successors.is_empty()
            && second.local.is_none()
            && second.successors.is_empty()
        {
            return emit_constant_return(bus, int, first_id, first, second_id, second);
        }
    }
    emit_stack_machine(bus, int, &instructions)
}

fn int_type(bus: &cc_silicon_compiler::bus::CompilerBus) -> Result<TypeId, String> {
    let mut found = None;
    for (id, record) in bus.arenas.types.iter() {
        if record.kind
            == (TypeKind::Int {
                rank: IntRank::Int,
                signed: true,
            })
        {
            if found.is_some() {
                return Err("target emit: ambiguous canonical int type".to_string());
            }
            found = Some(id);
        }
    }
    found.ok_or_else(|| "target emit: missing canonical int type".to_string())
}

fn emit_constant_return(
    bus: &cc_silicon_compiler::bus::CompilerBus,
    int: TypeId,
    constant_id: cc_silicon_compiler::ids::InstructionId,
    constant: &InstructionRecord,
    return_id: cc_silicon_compiler::ids::InstructionId,
    ret: &InstructionRecord,
) -> Result<Vec<u8>, String> {
    check_constant(constant_id, constant)?;
    check_return(return_id, ret, constant)?;
    check_value_type(bus, int, constant.result.expect("checked"))?;
    let const_id = constant
        .immediate
        .ok_or_else(|| "target emit: Constant has no immediate".to_string())?;
    let value = integer_value(bus, const_id)?;
    let mut assembly = String::new();
    push_header(&mut assembly);
    emit_i32(&mut assembly, value);
    assembly.push_str("    ret\n");
    push_footer(&mut assembly);
    Ok(assembly.into_bytes())
}

fn emit_stack_machine(
    bus: &cc_silicon_compiler::bus::CompilerBus,
    int: TypeId,
    instructions: &[(cc_silicon_compiler::ids::InstructionId, &InstructionRecord)],
) -> Result<Vec<u8>, String> {
    let mut locals: BTreeSet<SymbolId> = BTreeSet::new();
    let mut values: BTreeSet<ValueId> = BTreeSet::new();
    let mut produced: BTreeSet<ValueId> = BTreeSet::new();
    let mut saw_return = false;
    for (index, (id, record)) in instructions.iter().enumerate() {
        if saw_return {
            return Err(format!(
                "target emit: instruction {} follows Return",
                id.index()
            ));
        }
        match record.op {
            IrOp::Constant => {
                check_constant(*id, record)?;
                let const_id = record
                    .immediate
                    .ok_or_else(|| "target emit: Constant has no immediate".to_string())?;
                let _ = integer_value(bus, const_id)?;
                let result = record.result.expect("checked");
                if !values.insert(result) {
                    return Err("target emit: duplicate value id".to_string());
                }
                produced.insert(result);
            }
            IrOp::LocalLoad => {
                let local = record
                    .local
                    .ok_or_else(|| "target emit: LocalLoad has no local".to_string())?;
                check_local(bus, int, local)?;
                if !record.operands.is_empty()
                    || record.immediate.is_some()
                    || !record.successors.is_empty()
                {
                    return Err("target emit: malformed LocalLoad".to_string());
                }
                locals.insert(local);
                let result = record
                    .result
                    .ok_or_else(|| "target emit: LocalLoad has no result".to_string())?;
                if !values.insert(result) {
                    return Err("target emit: duplicate value id".to_string());
                }
                produced.insert(result);
            }
            IrOp::LocalStore => {
                let local = record
                    .local
                    .ok_or_else(|| "target emit: LocalStore has no local".to_string())?;
                check_local(bus, int, local)?;
                if record.operands.len() != 1
                    || record.immediate.is_some()
                    || record.result.is_some()
                    || !record.successors.is_empty()
                {
                    return Err("target emit: malformed LocalStore".to_string());
                }
                check_produced(&produced, record.operands[0])?;
                locals.insert(local);
            }
            IrOp::Add | IrOp::Sub | IrOp::Mul | IrOp::Div | IrOp::Rem => {
                if record.operands.len() != 2
                    || record.local.is_some()
                    || record.immediate.is_some()
                    || !record.successors.is_empty()
                {
                    return Err("target emit: malformed binary arithmetic".to_string());
                }
                check_produced(&produced, record.operands[0])?;
                check_produced(&produced, record.operands[1])?;
                let result = record
                    .result
                    .ok_or_else(|| "target emit: arithmetic has no result".to_string())?;
                if !values.insert(result) {
                    return Err("target emit: duplicate value id".to_string());
                }
                produced.insert(result);
            }
            IrOp::Neg => {
                if record.operands.len() != 1
                    || record.local.is_some()
                    || record.immediate.is_some()
                    || !record.successors.is_empty()
                {
                    return Err("target emit: malformed Neg".to_string());
                }
                check_produced(&produced, record.operands[0])?;
                let result = record
                    .result
                    .ok_or_else(|| "target emit: Neg has no result".to_string())?;
                if !values.insert(result) {
                    return Err("target emit: duplicate value id".to_string());
                }
                produced.insert(result);
            }
            IrOp::Return => {
                if record.operands.len() != 1
                    || record.local.is_some()
                    || record.immediate.is_some()
                    || record.result.is_some()
                    || !record.successors.is_empty()
                {
                    return Err("target emit: malformed Return".to_string());
                }
                check_produced(&produced, record.operands[0])?;
                if index + 1 != instructions.len() {
                    return Err("target emit: Return is not the last instruction".to_string());
                }
                saw_return = true;
            }
            other => {
                return Err(format!(
                    "target emit: op {other:?} is outside the local/arithmetic slice"
                ));
            }
        }
    }
    if !saw_return {
        return Err("target emit: missing Return".to_string());
    }
    for value in &values {
        check_value_type(bus, int, *value)?;
    }
    let locals: Vec<SymbolId> = locals.into_iter().collect();
    let values: Vec<ValueId> = values.into_iter().collect();
    let mut local_slots: BTreeMap<SymbolId, u32> = BTreeMap::new();
    for (index, id) in locals.iter().enumerate() {
        local_slots.insert(*id, (index as u32) * 4);
    }
    let mut value_slots: BTreeMap<ValueId, u32> = BTreeMap::new();
    for (index, id) in values.iter().enumerate() {
        value_slots.insert(*id, (locals.len() as u32 + index as u32) * 4);
    }
    let slot_count = locals.len() + values.len();
    let frame = ((slot_count as u32) * 4 + 15) & !15;
    let frame = frame.max(16);

    let mut assembly = String::new();
    push_header(&mut assembly);
    assembly.push_str(&format!("    sub sp, sp, #{frame}\n"));
    for (_id, record) in instructions {
        match record.op {
            IrOp::Constant => {
                let const_id = record.immediate.expect("checked");
                let value = integer_value(bus, const_id)?;
                emit_i32(&mut assembly, value);
                let result = record.result.expect("checked");
                store_value(&mut assembly, value_slots[&result]);
            }
            IrOp::LocalLoad => {
                let local = record.local.expect("checked");
                assembly.push_str(&format!("    ldr w0, [sp, #{}]\n", local_slots[&local]));
                let result = record.result.expect("checked");
                store_value(&mut assembly, value_slots[&result]);
            }
            IrOp::LocalStore => {
                let local = record.local.expect("checked");
                load_value(&mut assembly, value_slots[&record.operands[0]], 0);
                assembly.push_str(&format!("    str w0, [sp, #{}]\n", local_slots[&local]));
            }
            IrOp::Add | IrOp::Sub | IrOp::Mul | IrOp::Div | IrOp::Rem => {
                load_value(&mut assembly, value_slots[&record.operands[0]], 0);
                load_value(&mut assembly, value_slots[&record.operands[1]], 1);
                match record.op {
                    IrOp::Add => assembly.push_str("    add w0, w0, w1\n"),
                    IrOp::Sub => assembly.push_str("    sub w0, w0, w1\n"),
                    IrOp::Mul => assembly.push_str("    mul w0, w0, w1\n"),
                    IrOp::Div => assembly.push_str("    sdiv w0, w0, w1\n"),
                    IrOp::Rem => {
                        assembly.push_str("    sdiv w2, w0, w1\n");
                        assembly.push_str("    msub w0, w2, w1, w0\n");
                    }
                    _ => unreachable!(),
                }
                let result = record.result.expect("checked");
                store_value(&mut assembly, value_slots[&result]);
            }
            IrOp::Neg => {
                load_value(&mut assembly, value_slots[&record.operands[0]], 0);
                assembly.push_str("    neg w0, w0\n");
                let result = record.result.expect("checked");
                store_value(&mut assembly, value_slots[&result]);
            }
            IrOp::Return => {
                load_value(&mut assembly, value_slots[&record.operands[0]], 0);
                assembly.push_str(&format!("    add sp, sp, #{frame}\n"));
                assembly.push_str("    ret\n");
            }
            _ => unreachable!(),
        }
    }
    push_footer(&mut assembly);
    Ok(assembly.into_bytes())
}

fn push_header(assembly: &mut String) {
    assembly.push_str("# cc-silicon AArch64 bootstrap\n");
    assembly.push_str(".text\n");
    assembly.push_str(".align 2\n");
    assembly.push_str(".globl main\n");
    assembly.push_str(".type main, %function\n");
    assembly.push_str("main:\n");
}

fn push_footer(assembly: &mut String) {
    assembly.push_str(".size main, .-main\n");
    assembly.push_str(".section .note.GNU-stack,\"\",%progbits\n");
}

fn load_value(assembly: &mut String, slot: u32, register: u8) {
    assembly.push_str(&format!("    ldr w{register}, [sp, #{slot}]\n"));
}

fn store_value(assembly: &mut String, slot: u32) {
    assembly.push_str(&format!("    str w0, [sp, #{slot}]\n"));
}

fn check_local(
    bus: &cc_silicon_compiler::bus::CompilerBus,
    int: TypeId,
    local: SymbolId,
) -> Result<(), String> {
    let symbol: &SymbolRecord = bus
        .arenas
        .symbols
        .get(local)
        .map_err(|_| format!("target emit: missing local symbol {}", local.index()))?;
    if symbol.kind != cc_silicon_compiler::bus::SymbolKind::Object
        || symbol.storage != StorageDuration::Automatic
        || symbol.ty != Some(int)
    {
        return Err(format!(
            "target emit: local symbol {} is not an automatic int object",
            local.index()
        ));
    }
    Ok(())
}

fn check_value_type(
    bus: &cc_silicon_compiler::bus::CompilerBus,
    int: TypeId,
    value: ValueId,
) -> Result<(), String> {
    let record = bus
        .arenas
        .values
        .get(value)
        .map_err(|_| format!("target emit: missing value {}", value.index()))?;
    if record.ty != int {
        return Err(format!(
            "target emit: value {} is not the canonical int",
            value.index()
        ));
    }
    Ok(())
}

fn check_produced(produced: &BTreeSet<ValueId>, value: ValueId) -> Result<(), String> {
    if produced.contains(&value) {
        Ok(())
    } else {
        Err(format!(
            "target emit: operand {} is not produced before use",
            value.index()
        ))
    }
}

fn check_constant(
    id: cc_silicon_compiler::ids::InstructionId,
    record: &InstructionRecord,
) -> Result<(), String> {
    if record.op != IrOp::Constant
        || !record.operands.is_empty()
        || record.immediate.is_none()
        || record.result.is_none()
        || record.local.is_some()
        || !record.successors.is_empty()
    {
        return Err(format!(
            "target emit: instruction {} is not the Constant shape",
            id.index()
        ));
    }
    Ok(())
}

fn check_return(
    id: cc_silicon_compiler::ids::InstructionId,
    record: &InstructionRecord,
    constant: &InstructionRecord,
) -> Result<(), String> {
    if record.op != IrOp::Return
        || record.operands != vec![constant.result.expect("checked")]
        || record.immediate.is_some()
        || record.result.is_some()
        || record.local.is_some()
        || !record.successors.is_empty()
    {
        return Err(format!(
            "target emit: instruction {} is not the M1 Return shape",
            id.index()
        ));
    }
    Ok(())
}

fn integer_value(bus: &cc_silicon_compiler::bus::CompilerBus, id: ConstId) -> Result<i32, String> {
    let record = bus
        .arenas
        .consts
        .get(id)
        .map_err(|_| format!("target emit: missing constant {}", id.index()))?;
    let ConstValue::Integer(integer) = &record.value else {
        return Err("target emit: only integer constants are supported".to_string());
    };
    let mut magnitude: u128 = 0;
    if integer.magnitude.len() > 16 {
        return Err("target emit: constant does not fit the M1 int range".to_string());
    }
    for byte in &integer.magnitude {
        magnitude = (magnitude << 8) | u128::from(*byte);
    }
    let signed = if integer.negative {
        -(magnitude as i128)
    } else {
        magnitude as i128
    };
    i32::try_from(signed).map_err(|_| format!("target emit: constant {signed} does not fit i32"))
}

fn emit_i32(assembly: &mut String, value: i32) {
    let bits = value as u32;
    let low = bits & 0xffff;
    let high = (bits >> 16) & 0xffff;
    if low == 0 && high == 0 {
        assembly.push_str("    movz w0, #0\n");
        return;
    }
    assembly.push_str(&format!("    movz w0, #{low}\n"));
    if high != 0 {
        assembly.push_str(&format!("    movk w0, #{high}, lsl #16\n"));
    }
}

#[cfg(test)]
mod tests {
    use super::emit_m1_assembly;
    use cc_silicon_compiler::bus::{
        BlockRecord, CompilerBus, ConstRecord, ConstValue, FunctionRecord, InstructionRecord,
        IntRank, IntegerConst, IrOp, Linkage, StorageDuration, SymbolKind, SymbolRecord, TypeKind,
        TypeRecord, ValueRecord,
    };
    use cc_silicon_compiler::ids::{BlockId, NameId, NodeId, ScopeId, SymbolId, TypeId, ValueId};

    struct Fixture {
        bus: CompilerBus,
        int: TypeId,
        block: BlockId,
        local: SymbolId,
    }

    impl Fixture {
        fn new() -> Self {
            let mut bus = CompilerBus::default();
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
                    &bus.limits(),
                )
                .unwrap();
            let function_type = bus
                .arenas
                .types
                .alloc(
                    TypeRecord {
                        kind: TypeKind::Function {
                            result: int,
                            params: vec![],
                            prototype: true,
                            variadic: false,
                        },
                    },
                    &bus.limits(),
                )
                .unwrap();
            let main_symbol = bus
                .arenas
                .symbols
                .alloc(
                    SymbolRecord {
                        name: NameId::from_index(0),
                        scope: ScopeId::from_index(0),
                        kind: SymbolKind::Function,
                        ty: Some(function_type),
                        linkage: Linkage::External,
                        storage: StorageDuration::Static,
                        decl: NodeId::from_index(0),
                    },
                    &bus.limits(),
                )
                .unwrap();
            let local = bus
                .arenas
                .symbols
                .alloc(
                    SymbolRecord {
                        name: NameId::from_index(1),
                        scope: ScopeId::from_index(0),
                        kind: SymbolKind::Object,
                        ty: Some(int),
                        linkage: Linkage::None,
                        storage: StorageDuration::Automatic,
                        decl: NodeId::from_index(1),
                    },
                    &bus.limits(),
                )
                .unwrap();
            let function = bus
                .arenas
                .functions
                .alloc(
                    FunctionRecord {
                        symbol: main_symbol,
                        signature: function_type,
                        entry: BlockId::from_index(0),
                        linkage: Linkage::External,
                    },
                    &bus.limits(),
                )
                .unwrap();
            let block = bus
                .arenas
                .blocks
                .alloc(
                    BlockRecord {
                        function,
                        ordinal: 0,
                    },
                    &bus.limits(),
                )
                .unwrap();
            Self {
                bus,
                int,
                block,
                local,
            }
        }

        fn value(&mut self) -> ValueId {
            self.bus
                .arenas
                .values
                .alloc(ValueRecord { ty: self.int }, &self.bus.limits())
                .unwrap()
        }

        fn instruction(
            &mut self,
            op: IrOp,
            operands: Vec<ValueId>,
            immediate: Option<cc_silicon_compiler::ids::ConstId>,
            result: Option<ValueId>,
            local: Option<SymbolId>,
            successors: Vec<BlockId>,
        ) {
            self.bus
                .arenas
                .instructions
                .alloc(
                    InstructionRecord {
                        op,
                        block: self.block,
                        operands,
                        immediate,
                        result,
                        local,
                        successors,
                    },
                    &self.bus.limits(),
                )
                .unwrap();
        }

        fn constant(&mut self, value: i32) -> ValueId {
            let magnitude = value.unsigned_abs().to_be_bytes().to_vec();
            let constant = self
                .bus
                .arenas
                .consts
                .alloc(
                    ConstRecord {
                        ty: self.int,
                        value: ConstValue::Integer(IntegerConst {
                            magnitude,
                            negative: value < 0,
                        }),
                    },
                    &self.bus.limits(),
                )
                .unwrap();
            let result = self.value();
            self.instruction(
                IrOp::Constant,
                vec![],
                Some(constant),
                Some(result),
                None,
                vec![],
            );
            result
        }

        fn store_local(&mut self, value: ValueId) {
            let local = self.local;
            self.instruction(
                IrOp::LocalStore,
                vec![value],
                None,
                None,
                Some(local),
                vec![],
            );
        }

        fn load_local(&mut self) -> ValueId {
            let result = self.value();
            let local = self.local;
            self.instruction(
                IrOp::LocalLoad,
                vec![],
                None,
                Some(result),
                Some(local),
                vec![],
            );
            result
        }

        fn binary(&mut self, op: IrOp, lhs: ValueId, rhs: ValueId) -> ValueId {
            let result = self.value();
            self.instruction(op, vec![lhs, rhs], None, Some(result), None, vec![]);
            result
        }

        fn negate(&mut self, value: ValueId) -> ValueId {
            let result = self.value();
            self.instruction(IrOp::Neg, vec![value], None, Some(result), None, vec![]);
            result
        }

        fn ret(&mut self, value: ValueId) {
            self.instruction(IrOp::Return, vec![value], None, None, None, vec![]);
        }

        fn asm(&self) -> String {
            String::from_utf8(emit_m1_assembly(&self.bus).unwrap()).unwrap()
        }
    }

    #[test]
    fn local_and_arithmetic_ir_lowers_to_stack_slots() {
        let mut fixture = Fixture::new();
        let two = fixture.constant(2);
        fixture.store_local(two);
        let loaded = fixture.load_local();
        let three = fixture.constant(3);
        let sum = fixture.binary(IrOp::Add, loaded, three);
        fixture.ret(sum);

        let asm = fixture.asm();
        assert!(asm.contains("sub sp, sp, #32"), "{asm}");
        assert!(asm.contains("str w0, [sp, #0]"), "{asm}");
        assert!(asm.contains("ldr w0, [sp, #0]"), "{asm}");
        assert!(asm.contains("add w0, w0, w1"), "{asm}");
        assert!(asm.contains("add sp, sp, #32"), "{asm}");
        assert!(asm.contains("    ret\n"), "{asm}");
        run_native(&asm, 5);
    }

    #[test]
    fn every_integer_operator_lowers() {
        let mut fixture = Fixture::new();
        let ten = fixture.constant(10);
        fixture.store_local(ten);
        let x = fixture.load_local();
        let three = fixture.constant(3);
        let quotient = fixture.binary(IrOp::Div, x, three);
        let remainder = fixture.binary(IrOp::Rem, x, three);
        let two = fixture.constant(2);
        let scaled = fixture.binary(IrOp::Mul, remainder, two);
        let sum = fixture.binary(IrOp::Add, quotient, scaled);
        let one = fixture.constant(1);
        let difference = fixture.binary(IrOp::Sub, sum, one);
        let negated = fixture.negate(difference);
        fixture.ret(negated);

        let asm = fixture.asm();
        assert!(asm.contains("sdiv w0, w0, w1"), "{asm}");
        assert!(asm.contains("msub w0, w2, w1, w0"), "{asm}");
        assert!(asm.contains("mul w0, w0, w1"), "{asm}");
        assert!(asm.contains("add w0, w0, w1"), "{asm}");
        assert!(asm.contains("sub w0, w0, w1"), "{asm}");
        assert!(asm.contains("neg w0, w0"), "{asm}");
        run_native(&asm, 252);
    }

    #[test]
    fn unsupported_control_flow_fails_closed() {
        let mut fixture = Fixture::new();
        fixture.instruction(IrOp::Branch, vec![], None, None, None, vec![]);
        let error = emit_m1_assembly(&fixture.bus).unwrap_err();
        assert!(
            error.contains("outside the local/arithmetic slice"),
            "{error}"
        );
    }

    #[cfg(target_os = "linux")]
    fn run_native(assembly: &str, expected_exit: u8) {
        use std::process::Command;

        let machine = Command::new("gcc")
            .arg("-dumpmachine")
            .output()
            .expect("gcc available");
        if !String::from_utf8_lossy(&machine.stdout).contains("aarch64") {
            return;
        }
        let dir = std::env::temp_dir().join(format!(
            "m1-target-unit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock moves forward")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let source = dir.join("main.s");
        let executable = dir.join("main");
        std::fs::write(&source, assembly).expect("assembly file");
        let build = Command::new("gcc")
            .arg("-o")
            .arg(&executable)
            .arg(&source)
            .output()
            .expect("gcc runs");
        assert!(
            build.status.success(),
            "gcc failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let run = Command::new(&executable).output().expect("program runs");
        assert_eq!(run.status.code(), Some(i32::from(expected_exit)));
    }

    #[cfg(not(target_os = "linux"))]
    fn run_native(_assembly: &str, _expected_exit: u8) {}
}
