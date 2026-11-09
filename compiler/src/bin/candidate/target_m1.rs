// ============================================================================
// candidate/target_m1.rs — host-only M1 AArch64 assembly prototype
//
// Consumes the committed M1 IR shape (one function, one block, Constant +
// Return) and emits GNU AArch64 assembly for `int main(void)`. This is a
// bootstrap target-emission path, not the T11 chip family and not a general
// code generator: any other IR shape fails closed.
// ============================================================================

use cc_silicon_compiler::bus::{ConstValue, InstructionRecord, IrOp, TypeKind};
use cc_silicon_compiler::ids::{ConstId, InstructionId};

/// Emit GNU AArch64 assembly for the committed M1 function.
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
    if !params.is_empty() || !*prototype || *variadic {
        return Err("target emit: only int(void) is supported".to_string());
    }
    let result_ty = bus
        .arenas
        .types
        .get(*result)
        .map_err(|_| "target emit: missing result type".to_string())?;
    if result_ty.kind
        != (TypeKind::Int {
            rank: cc_silicon_compiler::bus::IntRank::Int,
            signed: true,
        })
    {
        return Err("target emit: only signed int result is supported".to_string());
    }

    let mut instructions = bus.arenas.instructions.iter();
    let Some((constant_id, constant)) = instructions.next() else {
        return Err("target emit: missing Constant instruction".to_string());
    };
    let Some((return_id, ret)) = instructions.next() else {
        return Err("target emit: missing Return instruction".to_string());
    };
    if instructions.next().is_some() {
        return Err("target emit: expected exactly two instructions".to_string());
    }
    check_constant(constant_id, constant)?;
    check_return(return_id, ret, constant)?;
    let const_id = constant
        .immediate
        .ok_or_else(|| "target emit: Constant has no immediate".to_string())?;
    let value = integer_value(bus, const_id)?;

    let mut assembly = String::new();
    assembly.push_str("# cc-silicon M1 AArch64 prototype\n");
    assembly.push_str(".text\n");
    assembly.push_str(".align 2\n");
    assembly.push_str(".globl main\n");
    assembly.push_str(".type main, %function\n");
    assembly.push_str("main:\n");
    emit_i32(&mut assembly, value);
    assembly.push_str("    ret\n");
    assembly.push_str(".size main, .-main\n");
    assembly.push_str(".section .note.GNU-stack,\"\",%progbits\n");
    Ok(assembly.into_bytes())
}

fn check_constant(id: InstructionId, record: &InstructionRecord) -> Result<(), String> {
    if record.op != IrOp::Constant
        || !record.operands.is_empty()
        || record.immediate.is_none()
        || record.result.is_none()
        || record.local.is_some()
        || !record.successors.is_empty()
    {
        return Err(format!(
            "target emit: instruction {} is not the M1 Constant shape",
            id.index()
        ));
    }
    Ok(())
}

fn check_return(
    id: InstructionId,
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
