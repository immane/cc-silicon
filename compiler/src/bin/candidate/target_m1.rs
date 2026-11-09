// ============================================================================
// candidate/target_m1.rs — host-only AArch64 assembly bootstrap
//
// Consumes a committed straight-line integer IR module and emits GNU AArch64
// assembly. The first slice handled only Constant+Return; the current host
// slice adds automatic locals, local load/store, integer arithmetic, and
// same-module direct calls. Recursion, variadics, external/indirect calls,
// memory, control flow, and general ABI lowering still fail closed. No
// contract freeze is implied by this bootstrap.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, ConstValue, FunctionRecord, InstructionRecord, IntRank, IrOp, Linkage, ScopeKind,
    StorageDuration, SymbolKind, SymbolRecord, TypeKind,
};
use cc_silicon_compiler::ids::{
    BlockId, ConstId, FunctionId, InstructionId, ScopeId, SymbolId, TypeId, ValueId,
};
use std::collections::{BTreeMap, BTreeSet};

/// Maximum arguments/parameters for the covered int-only call prototype.
const MAX_ARGUMENTS: usize = 8;
/// Largest 16-byte-aligned AArch64 stack frame accepted by this emitter.
const MAX_FRAME_BYTES: u32 = 4080;

/// Emit GNU AArch64 assembly for the supported committed IR module.
pub fn emit_m1_assembly(bus: &CompilerBus) -> Result<Vec<u8>, String> {
    let int = int_type(bus)?;
    let functions = collect_functions(bus)?;
    let blocks = validate_blocks(bus, &functions)?;
    let names = validate_function_names(bus, &functions)?;
    validate_signatures(bus, int, &functions, &names)?;
    validate_parameters(bus, int, &functions, &names)?;
    let instructions = validate_instructions(bus, int, &functions, &blocks)?;
    let call_targets = validate_calls(bus, &functions, &instructions)?;
    let layouts = build_layouts(bus, int, &functions, &instructions)?;
    emit_module(
        bus,
        int,
        &functions,
        &names,
        &instructions,
        &call_targets,
        &layouts,
    )
}

#[derive(Clone, Copy)]
struct FunctionEntry<'a> {
    id: FunctionId,
    record: &'a FunctionRecord,
    symbol: &'a SymbolRecord,
}

#[derive(Clone, Copy)]
struct FunctionName<'a> {
    text: &'a str,
    is_main: bool,
}

struct FunctionLayout {
    symbol_slots: BTreeMap<SymbolId, u32>,
    value_slots: BTreeMap<ValueId, u32>,
    frame: u32,
    nonleaf: bool,
    fast_constant: Option<i32>,
}

type FunctionInstructions<'a> = BTreeMap<FunctionId, Vec<(InstructionId, &'a InstructionRecord)>>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Visit {
    Gray,
    Black,
}

fn int_type(bus: &CompilerBus) -> Result<TypeId, String> {
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

fn collect_functions<'a>(bus: &'a CompilerBus) -> Result<Vec<FunctionEntry<'a>>, String> {
    let mut functions = Vec::new();
    for (id, record) in bus.arenas.functions.iter() {
        let symbol = bus.arenas.symbols.get(record.symbol).map_err(|_| {
            format!(
                "target emit: missing function symbol {}",
                record.symbol.index()
            )
        })?;
        functions.push(FunctionEntry { id, record, symbol });
    }
    if functions.is_empty() {
        return Err("target emit: missing main".to_string());
    }
    Ok(functions)
}

fn validate_blocks(
    bus: &CompilerBus,
    functions: &[FunctionEntry<'_>],
) -> Result<BTreeMap<BlockId, FunctionId>, String> {
    let known: BTreeSet<FunctionId> = functions.iter().map(|function| function.id).collect();
    let mut block_owner = BTreeMap::new();
    for (block_id, block) in bus.arenas.blocks.iter() {
        if !known.contains(&block.function) {
            return Err(format!(
                "target emit: block {} belongs to a missing function",
                block_id.index()
            ));
        }
        block_owner.insert(block_id, block.function);
    }

    let mut entries = BTreeMap::new();
    for function in functions {
        let entry = block_owner
            .iter()
            .find(|(_, owner)| **owner == function.id)
            .map(|(block, _)| *block)
            .ok_or_else(|| {
                format!(
                    "target emit: function {} is missing its entry block",
                    function.id.index()
                )
            })?;
        if entry != function.record.entry {
            return Err(format!(
                "target emit: function {} entry block mismatch",
                function.id.index()
            ));
        }
        if entries.insert(entry, function.id).is_some() {
            return Err("target emit: expected one entry block".to_string());
        }
    }
    if block_owner.len() != functions.len() {
        return Err("target emit: expected one entry block".to_string());
    }
    Ok(entries)
}

fn validate_function_names<'a>(
    bus: &'a CompilerBus,
    functions: &[FunctionEntry<'a>],
) -> Result<BTreeMap<FunctionId, FunctionName<'a>>, String> {
    let mut names = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut main_count = 0usize;
    for function in functions {
        let symbol = function.symbol;
        if symbol.kind != SymbolKind::Function {
            return Err(format!(
                "target emit: function symbol {} is not a function",
                symbol.name.index()
            ));
        }
        let scope = bus.arenas.scopes.get(symbol.scope).map_err(|_| {
            format!(
                "target emit: function symbol {} is not file scope",
                symbol.name.index()
            )
        })?;
        if scope.kind != ScopeKind::File {
            return Err(format!(
                "target emit: function symbol {} is not file scope",
                symbol.name.index()
            ));
        }
        let bytes = bus
            .intern
            .get(symbol.name)
            .map_err(|_| "target emit: unsafe name (missing interned name)".to_string())?;
        let text = validate_identifier(bytes)?;
        if !seen.insert(bytes.to_vec()) {
            return Err(format!("target emit: duplicate function symbol {text}"));
        }
        if text == "main" {
            main_count += 1;
        }
        names.insert(
            function.id,
            FunctionName {
                text,
                is_main: text == "main",
            },
        );
    }
    if main_count == 0 {
        return Err("target emit: missing main".to_string());
    }
    if main_count > 1 {
        return Err("target emit: duplicate function symbol main".to_string());
    }
    Ok(names)
}

fn validate_signatures(
    bus: &CompilerBus,
    int: TypeId,
    functions: &[FunctionEntry<'_>],
    names: &BTreeMap<FunctionId, FunctionName<'_>>,
) -> Result<(), String> {
    for function in functions {
        let name = names.get(&function.id).ok_or_else(|| {
            format!(
                "target emit: missing name for function {}",
                function.id.index()
            )
        })?;
        let signature = bus
            .arenas
            .types
            .get(function.record.signature)
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
        if params.len() != function.record.parameters.len() {
            if name.is_main {
                return Err("target emit: main must be int(void)".to_string());
            }
            return Err(format!(
                "target emit: parameter count mismatch for function {}",
                function.id.index()
            ));
        }
        if name.is_main {
            if *result != int
                || !params.is_empty()
                || !*prototype
                || *variadic
                || function.record.linkage != Linkage::External
                || function.symbol.linkage != Linkage::External
            {
                return Err("target emit: main must be int(void)".to_string());
            }
            continue;
        }
        if function.symbol.linkage != function.record.linkage {
            return Err(format!(
                "target emit: function {} linkage mismatch",
                name.text
            ));
        }
        if *result != int {
            return Err(format!(
                "target emit: function {} result is not int",
                name.text
            ));
        }
        if !*prototype || *variadic {
            return Err(format!(
                "target emit: function {} is not a canonical prototype",
                name.text
            ));
        }
        if params.len() > MAX_ARGUMENTS {
            return Err(format!(
                "target emit: more than eight arguments for function {}",
                name.text
            ));
        }
        for param in params {
            if *param != int {
                return Err(format!(
                    "target emit: function {} has a non-int parameter",
                    name.text
                ));
            }
        }
    }
    Ok(())
}

fn validate_parameters(
    bus: &CompilerBus,
    int: TypeId,
    functions: &[FunctionEntry<'_>],
    names: &BTreeMap<FunctionId, FunctionName<'_>>,
) -> Result<(), String> {
    let mut used_scopes: BTreeMap<ScopeId, FunctionId> = BTreeMap::new();
    for function in functions {
        let name = names.get(&function.id).ok_or_else(|| {
            format!(
                "target emit: missing name for function {}",
                function.id.index()
            )
        })?;
        let mut symbols = BTreeSet::new();
        let mut parameter_names = BTreeSet::new();
        let mut block_scope = None;
        for parameter in &function.record.parameters {
            if !symbols.insert(*parameter) {
                return Err(format!(
                    "target emit: duplicate parameter symbol {} in function {}",
                    parameter.index(),
                    name.text
                ));
            }
            let symbol = bus.arenas.symbols.get(*parameter).map_err(|_| {
                format!("target emit: missing parameter symbol {}", parameter.index())
            })?;
            if symbol.kind != SymbolKind::Object
                || symbol.storage != StorageDuration::Automatic
                || symbol.ty != Some(int)
            {
                return Err(format!(
                    "target emit: parameter symbol {} is not an automatic int object",
                    parameter.index()
                ));
            }
            let bytes = bus
                .intern
                .get(symbol.name)
                .map_err(|_| "target emit: unsafe name (missing interned name)".to_string())?;
            validate_identifier(bytes)?;
            if !parameter_names.insert(bytes.to_vec()) {
                return Err(format!(
                    "target emit: duplicate parameter name in function {}",
                    name.text
                ));
            }
            let scope = bus.arenas.scopes.get(symbol.scope).map_err(|_| {
                format!(
                    "target emit: parameter symbol {} is not in the function block scope",
                    parameter.index()
                )
            })?;
            if scope.kind != ScopeKind::Block || scope.parent != Some(function.symbol.scope) {
                return Err(format!(
                    "target emit: parameter symbol {} is not in the function block scope",
                    parameter.index()
                ));
            }
            match block_scope {
                None => block_scope = Some(symbol.scope),
                Some(existing) if existing != symbol.scope => {
                    return Err(format!(
                        "target emit: parameter symbol {} is not in the function block scope",
                        parameter.index()
                    ));
                }
                Some(_) => {}
            }
        }
        if let Some(scope) = block_scope {
            if let Some(previous) = used_scopes.insert(scope, function.id) {
                if previous != function.id {
                    return Err(format!(
                        "target emit: parameter symbol scope is shared by functions {} and {}",
                        previous.index(),
                        function.id.index()
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_instructions<'a>(
    bus: &'a CompilerBus,
    int: TypeId,
    functions: &[FunctionEntry<'a>],
    blocks: &BTreeMap<BlockId, FunctionId>,
) -> Result<FunctionInstructions<'a>, String> {
    let mut per_function: FunctionInstructions<'a> = BTreeMap::new();
    for function in functions {
        per_function.insert(function.id, Vec::new());
    }

    let mut producers: BTreeMap<ValueId, (FunctionId, InstructionId)> = BTreeMap::new();
    for (id, record) in bus.arenas.instructions.iter() {
        let function = blocks.get(&record.block).copied().ok_or_else(|| {
            format!(
                "target emit: instruction {} belongs to a missing block",
                id.index()
            )
        })?;
        validate_instruction_shape(bus, int, id, record)?;
        if let Some(result) = record.result {
            if producers.insert(result, (function, id)).is_some() {
                return Err(format!("target emit: duplicate value id {}", result.index()));
            }
        }
        per_function
            .get_mut(&function)
            .ok_or_else(|| {
                format!(
                    "target emit: instruction {} belongs to a missing function",
                    id.index()
                )
            })?
            .push((id, record));
    }

    for function in functions {
        let instructions = per_function.get(&function.id).ok_or_else(|| {
            format!(
                "target emit: missing instructions for function {}",
                function.id.index()
            )
        })?;
        if instructions.is_empty() {
            return Err("target emit: missing Return".to_string());
        }
        let mut saw_return = false;
        for (index, (id, record)) in instructions.iter().enumerate() {
            if saw_return {
                return Err(format!(
                    "target emit: instruction {} follows Return",
                    id.index()
                ));
            }
            if record.op == IrOp::Return {
                if index + 1 != instructions.len() {
                    return Err("target emit: Return is not the last instruction".to_string());
                }
                saw_return = true;
            }
        }
        if !saw_return {
            return Err("target emit: missing Return".to_string());
        }

        let mut produced = BTreeSet::new();
        for (_, record) in instructions {
            for operand in &record.operands {
                match producers.get(operand) {
                    None => {
                        return Err(format!(
                            "target emit: operand {} is not produced before use",
                            operand.index()
                        ));
                    }
                    Some((owner, _)) if *owner != function.id => {
                        return Err(format!(
                            "target emit: cross-function value {} in function {}",
                            operand.index(),
                            function.id.index()
                        ));
                    }
                    Some(_) => {}
                }
                if !produced.contains(operand) {
                    return Err(format!(
                        "target emit: operand {} is not produced before use",
                        operand.index()
                    ));
                }
            }
            if let Some(result) = record.result {
                produced.insert(result);
            }
        }
    }
    Ok(per_function)
}

fn validate_instruction_shape(
    bus: &CompilerBus,
    int: TypeId,
    id: InstructionId,
    record: &InstructionRecord,
) -> Result<(), String> {
    match record.op {
        IrOp::Constant => {
            if !record.operands.is_empty()
                || record.immediate.is_none()
                || record.result.is_none()
                || record.local.is_some()
                || !record.successors.is_empty()
                || record.callee.is_some()
            {
                return Err(format!(
                    "target emit: instruction {} is not the Constant shape",
                    id.index()
                ));
            }
            let constant = record.immediate.ok_or_else(|| {
                format!(
                    "target emit: instruction {} is not the Constant shape",
                    id.index()
                )
            })?;
            let _ = integer_value(bus, int, constant)?;
            check_value_type(
                bus,
                int,
                record.result.ok_or_else(|| {
                    format!(
                        "target emit: instruction {} is not the Constant shape",
                        id.index()
                    )
                })?,
            )?;
            Ok(())
        }
        IrOp::LocalLoad => {
            let local = record.local.ok_or_else(|| {
                format!(
                    "target emit: instruction {} is not the LocalLoad shape",
                    id.index()
                )
            })?;
            check_local(bus, int, local)?;
            if !record.operands.is_empty()
                || record.immediate.is_some()
                || record.result.is_none()
                || !record.successors.is_empty()
                || record.callee.is_some()
            {
                return Err(format!(
                    "target emit: instruction {} is not the LocalLoad shape",
                    id.index()
                ));
            }
            check_value_type(
                bus,
                int,
                record.result.ok_or_else(|| {
                    format!(
                        "target emit: instruction {} is not the LocalLoad shape",
                        id.index()
                    )
                })?,
            )?;
            Ok(())
        }
        IrOp::LocalStore => {
            let local = record.local.ok_or_else(|| {
                format!(
                    "target emit: instruction {} is not the LocalStore shape",
                    id.index()
                )
            })?;
            check_local(bus, int, local)?;
            if record.operands.len() != 1
                || record.immediate.is_some()
                || record.result.is_some()
                || !record.successors.is_empty()
                || record.callee.is_some()
            {
                return Err(format!(
                    "target emit: instruction {} is not the LocalStore shape",
                    id.index()
                ));
            }
            Ok(())
        }
        IrOp::Add | IrOp::Sub | IrOp::Mul | IrOp::Div | IrOp::Rem => {
            if record.operands.len() != 2
                || record.local.is_some()
                || record.immediate.is_some()
                || record.result.is_none()
                || !record.successors.is_empty()
                || record.callee.is_some()
            {
                return Err(format!(
                    "target emit: instruction {} is not the binary arithmetic shape",
                    id.index()
                ));
            }
            check_value_type(
                bus,
                int,
                record.result.ok_or_else(|| {
                    format!(
                        "target emit: instruction {} is not the binary arithmetic shape",
                        id.index()
                    )
                })?,
            )?;
            Ok(())
        }
        IrOp::Neg => {
            if record.operands.len() != 1
                || record.local.is_some()
                || record.immediate.is_some()
                || record.result.is_none()
                || !record.successors.is_empty()
                || record.callee.is_some()
            {
                return Err(format!(
                    "target emit: instruction {} is not the Neg shape",
                    id.index()
                ));
            }
            check_value_type(
                bus,
                int,
                record.result.ok_or_else(|| {
                    format!("target emit: instruction {} is not the Neg shape", id.index())
                })?,
            )?;
            Ok(())
        }
        IrOp::Call => {
            if record.operands.len() > MAX_ARGUMENTS {
                return Err("target emit: more than eight arguments".to_string());
            }
            if record.local.is_some()
                || record.immediate.is_some()
                || record.result.is_none()
                || !record.successors.is_empty()
            {
                return Err(format!(
                    "target emit: instruction {} is not the Call shape",
                    id.index()
                ));
            }
            if record.callee.is_none() {
                return Err(format!(
                    "target emit: unresolved callee in instruction {}",
                    id.index()
                ));
            }
            check_value_type(
                bus,
                int,
                record.result.ok_or_else(|| {
                    format!(
                        "target emit: instruction {} is not the Call shape",
                        id.index()
                    )
                })?,
            )?;
            Ok(())
        }
        IrOp::Return => {
            if record.operands.len() != 1
                || record.local.is_some()
                || record.immediate.is_some()
                || record.result.is_some()
                || !record.successors.is_empty()
                || record.callee.is_some()
            {
                return Err(format!(
                    "target emit: instruction {} is not the Return shape",
                    id.index()
                ));
            }
            Ok(())
        }
        IrOp::Branch | IrOp::CondBranch => Err(format!(
            "target emit: op {:?} is outside the local/arithmetic slice",
            record.op
        )),
    }
}

fn validate_calls(
    bus: &CompilerBus,
    functions: &[FunctionEntry<'_>],
    instructions: &FunctionInstructions<'_>,
) -> Result<BTreeMap<InstructionId, FunctionId>, String> {
    let mut by_symbol = BTreeMap::new();
    for function in functions {
        by_symbol.insert(function.record.symbol, function.id);
    }

    let mut edges: BTreeMap<FunctionId, Vec<FunctionId>> = BTreeMap::new();
    let mut call_targets = BTreeMap::new();
    for (function_id, list) in instructions {
        for (instruction_id, record) in list {
            if record.op != IrOp::Call {
                continue;
            }
            let callee_symbol = record.callee.ok_or_else(|| {
                format!(
                    "target emit: unresolved callee in instruction {}",
                    instruction_id.index()
                )
            })?;
            let symbol = bus
                .arenas
                .symbols
                .get(callee_symbol)
                .map_err(|_| format!("target emit: unresolved callee symbol {}", callee_symbol.index()))?;
            if symbol.kind != SymbolKind::Function {
                return Err(format!(
                    "target emit: unresolved callee symbol {}",
                    callee_symbol.index()
                ));
            }
            let scope = bus
                .arenas
                .scopes
                .get(symbol.scope)
                .map_err(|_| format!("target emit: unresolved callee symbol {}", callee_symbol.index()))?;
            if scope.kind != ScopeKind::File {
                return Err(format!(
                    "target emit: unresolved callee symbol {}",
                    callee_symbol.index()
                ));
            }
            let callee_id = by_symbol.get(&callee_symbol).copied().ok_or_else(|| {
                format!(
                    "target emit: unresolved callee symbol {}",
                    callee_symbol.index()
                )
            })?;
            let callee = functions
                .iter()
                .find(|function| function.id == callee_id)
                .ok_or_else(|| {
                    format!(
                        "target emit: unresolved callee symbol {}",
                        callee_symbol.index()
                    )
                })?;
            if record.operands.len() != callee.record.parameters.len() {
                return Err(format!(
                    "target emit: call arity mismatch in instruction {}",
                    instruction_id.index()
                ));
            }
            if callee_id == *function_id {
                return Err("target emit: recursive or cyclic call graph".to_string());
            }
            call_targets.insert(*instruction_id, callee_id);
            edges.entry(*function_id).or_default().push(callee_id);
        }
    }

    let mut color: BTreeMap<FunctionId, Visit> = BTreeMap::new();
    for function in functions {
        if !color.contains_key(&function.id) {
            visit(function.id, &edges, &mut color)?;
        }
    }
    Ok(call_targets)
}

fn visit(
    id: FunctionId,
    edges: &BTreeMap<FunctionId, Vec<FunctionId>>,
    color: &mut BTreeMap<FunctionId, Visit>,
) -> Result<(), String> {
    color.insert(id, Visit::Gray);
    if let Some(nexts) = edges.get(&id) {
        for next in nexts {
            match color.get(next).copied() {
                Some(Visit::Gray) => {
                    return Err("target emit: recursive or cyclic call graph".to_string());
                }
                Some(Visit::Black) => {}
                None => visit(*next, edges, color)?,
            }
        }
    }
    color.insert(id, Visit::Black);
    Ok(())
}

fn build_layouts(
    bus: &CompilerBus,
    int: TypeId,
    functions: &[FunctionEntry<'_>],
    instructions: &FunctionInstructions<'_>,
) -> Result<BTreeMap<FunctionId, FunctionLayout>, String> {
    let mut layouts = BTreeMap::new();
    for function in functions {
        let list = instructions.get(&function.id).ok_or_else(|| {
            format!(
                "target emit: missing instructions for function {}",
                function.id.index()
            )
        })?;
        let parameters: BTreeSet<SymbolId> =
            function.record.parameters.iter().copied().collect();
        let mut locals = BTreeSet::new();
        let mut values = BTreeSet::new();
        for (_, record) in list {
            if matches!(record.op, IrOp::LocalLoad | IrOp::LocalStore) {
                if let Some(local) = record.local {
                    if !parameters.contains(&local) {
                        locals.insert(local);
                    }
                }
            }
            if let Some(result) = record.result {
                values.insert(result);
            }
        }

        let mut symbol_slots = BTreeMap::new();
        let mut next = 0u32;
        for parameter in &function.record.parameters {
            symbol_slots.insert(*parameter, next);
            next = next.checked_add(4).ok_or_else(|| {
                "target emit: stack frame size overflow".to_string()
            })?;
        }
        for local in locals {
            symbol_slots.insert(local, next);
            next = next.checked_add(4).ok_or_else(|| {
                "target emit: stack frame size overflow".to_string()
            })?;
        }
        let mut value_slots = BTreeMap::new();
        for value in values {
            value_slots.insert(value, next);
            next = next.checked_add(4).ok_or_else(|| {
                "target emit: stack frame size overflow".to_string()
            })?;
        }

        let slot_count = (next / 4) as usize;
        let nonleaf = list.iter().any(|(_, record)| record.op == IrOp::Call);
        let frame = frame_size(slot_count, nonleaf)?;
        let fast_constant = if function.record.parameters.is_empty()
            && list.len() == 2
            && list[0].1.op == IrOp::Constant
            && list[1].1.op == IrOp::Return
        {
            let constant = list[0].1.immediate.ok_or_else(|| {
                format!(
                    "target emit: instruction {} is not the Constant shape",
                    list[0].0.index()
                )
            })?;
            Some(integer_value(bus, int, constant)?)
        } else {
            None
        };
        layouts.insert(
            function.id,
            FunctionLayout {
                symbol_slots,
                value_slots,
                frame,
                nonleaf,
                fast_constant,
            },
        );
    }
    Ok(layouts)
}

fn emit_module(
    bus: &CompilerBus,
    int: TypeId,
    functions: &[FunctionEntry<'_>],
    names: &BTreeMap<FunctionId, FunctionName<'_>>,
    instructions: &FunctionInstructions<'_>,
    call_targets: &BTreeMap<InstructionId, FunctionId>,
    layouts: &BTreeMap<FunctionId, FunctionLayout>,
) -> Result<Vec<u8>, String> {
    let mut assembly = String::new();
    assembly.push_str("# cc-silicon AArch64 bootstrap\n");
    assembly.push_str(".text\n");
    assembly.push_str(".align 2\n");
    for function in functions {
        let name = names.get(&function.id).ok_or_else(|| {
            format!(
                "target emit: missing name for function {}",
                function.id.index()
            )
        })?;
        match function.record.linkage {
            Linkage::External => {
                assembly.push_str(&format!(".globl {}\n", name.text));
            }
            Linkage::Internal => {}
            Linkage::None => {
                return Err(format!(
                    "target emit: function {} has no linkage",
                    function.id.index()
                ));
            }
        }
        assembly.push_str(&format!(".type {}, %function\n", name.text));
        assembly.push_str(&format!("{}:\n", name.text));
        let layout = layouts.get(&function.id).ok_or_else(|| {
            format!(
                "target emit: missing layout for function {}",
                function.id.index()
            )
        })?;
        let list = instructions.get(&function.id).ok_or_else(|| {
            format!(
                "target emit: missing instructions for function {}",
                function.id.index()
            )
        })?;
        emit_function(
            bus,
            int,
            function,
            layout,
            list,
            names,
            call_targets,
            &mut assembly,
        )?;
        assembly.push_str(&format!(".size {}, .-{}\n", name.text, name.text));
    }
    assembly.push_str(".section .note.GNU-stack,\"\",%progbits\n");
    Ok(assembly.into_bytes())
}

#[allow(clippy::too_many_arguments)]
fn emit_function(
    bus: &CompilerBus,
    int: TypeId,
    function: &FunctionEntry<'_>,
    layout: &FunctionLayout,
    instructions: &[(InstructionId, &InstructionRecord)],
    names: &BTreeMap<FunctionId, FunctionName<'_>>,
    call_targets: &BTreeMap<InstructionId, FunctionId>,
    assembly: &mut String,
) -> Result<(), String> {
    if let Some(value) = layout.fast_constant {
        emit_i32(assembly, value);
        assembly.push_str("    ret\n");
        return Ok(());
    }

    if layout.frame > 0 {
        assembly.push_str(&format!("    sub sp, sp, #{}\n", layout.frame));
    }
    if layout.nonleaf {
        let lr = layout
            .frame
            .checked_sub(8)
            .ok_or_else(|| "target emit: non-leaf frame is too small".to_string())?;
        assembly.push_str(&format!("    str x30, [sp, #{lr}]\n"));
    }
    for (index, parameter) in function.record.parameters.iter().enumerate() {
        let slot = layout.symbol_slots.get(parameter).ok_or_else(|| {
            format!(
                "target emit: missing slot for parameter symbol {}",
                parameter.index()
            )
        })?;
        assembly.push_str(&format!("    str w{index}, [sp, #{slot}]\n"));
    }

    for (instruction_id, record) in instructions {
        emit_instruction(
            bus,
            int,
            *instruction_id,
            record,
            layout,
            names,
            call_targets,
            assembly,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn emit_instruction(
    bus: &CompilerBus,
    int: TypeId,
    instruction_id: InstructionId,
    record: &InstructionRecord,
    layout: &FunctionLayout,
    names: &BTreeMap<FunctionId, FunctionName<'_>>,
    call_targets: &BTreeMap<InstructionId, FunctionId>,
    assembly: &mut String,
) -> Result<(), String> {
    match record.op {
        IrOp::Constant => {
            let constant = record.immediate.ok_or_else(|| {
                format!(
                    "target emit: instruction {} is not the Constant shape",
                    instruction_id.index()
                )
            })?;
            let value = integer_value(bus, int, constant)?;
            emit_i32(assembly, value);
            let result = record.result.ok_or_else(|| {
                format!(
                    "target emit: instruction {} is not the Constant shape",
                    instruction_id.index()
                )
            })?;
            let slot = layout.value_slots.get(&result).ok_or_else(|| {
                format!("target emit: missing slot for value {}", result.index())
            })?;
            store_value(assembly, *slot);
            Ok(())
        }
        IrOp::LocalLoad => {
            let local = record.local.ok_or_else(|| {
                format!(
                    "target emit: instruction {} is not the LocalLoad shape",
                    instruction_id.index()
                )
            })?;
            let slot = layout.symbol_slots.get(&local).ok_or_else(|| {
                format!("target emit: missing slot for symbol {}", local.index())
            })?;
            load_value(assembly, *slot, 0);
            let result = record.result.ok_or_else(|| {
                format!(
                    "target emit: instruction {} is not the LocalLoad shape",
                    instruction_id.index()
                )
            })?;
            let result_slot = layout.value_slots.get(&result).ok_or_else(|| {
                format!("target emit: missing slot for value {}", result.index())
            })?;
            store_value(assembly, *result_slot);
            Ok(())
        }
        IrOp::LocalStore => {
            let local = record.local.ok_or_else(|| {
                format!(
                    "target emit: instruction {} is not the LocalStore shape",
                    instruction_id.index()
                )
            })?;
            let operand = record.operands.first().ok_or_else(|| {
                format!(
                    "target emit: instruction {} is not the LocalStore shape",
                    instruction_id.index()
                )
            })?;
            let operand_slot = layout.value_slots.get(operand).ok_or_else(|| {
                format!("target emit: missing slot for value {}", operand.index())
            })?;
            load_value(assembly, *operand_slot, 0);
            let local_slot = layout.symbol_slots.get(&local).ok_or_else(|| {
                format!("target emit: missing slot for symbol {}", local.index())
            })?;
            assembly.push_str(&format!("    str w0, [sp, #{local_slot}]\n"));
            Ok(())
        }
        IrOp::Add | IrOp::Sub | IrOp::Mul | IrOp::Div | IrOp::Rem => {
            let lhs = record.operands.first().ok_or_else(|| {
                format!(
                    "target emit: instruction {} is not the binary arithmetic shape",
                    instruction_id.index()
                )
            })?;
            let rhs = record.operands.get(1).ok_or_else(|| {
                format!(
                    "target emit: instruction {} is not the binary arithmetic shape",
                    instruction_id.index()
                )
            })?;
            let lhs_slot = layout.value_slots.get(lhs).ok_or_else(|| {
                format!("target emit: missing slot for value {}", lhs.index())
            })?;
            let rhs_slot = layout.value_slots.get(rhs).ok_or_else(|| {
                format!("target emit: missing slot for value {}", rhs.index())
            })?;
            load_value(assembly, *lhs_slot, 0);
            load_value(assembly, *rhs_slot, 1);
            match record.op {
                IrOp::Add => assembly.push_str("    add w0, w0, w1\n"),
                IrOp::Sub => assembly.push_str("    sub w0, w0, w1\n"),
                IrOp::Mul => assembly.push_str("    mul w0, w0, w1\n"),
                IrOp::Div => assembly.push_str("    sdiv w0, w0, w1\n"),
                IrOp::Rem => {
                    assembly.push_str("    sdiv w2, w0, w1\n");
                    assembly.push_str("    msub w0, w2, w1, w0\n");
                }
                other => {
                    return Err(format!(
                        "target emit: op {other:?} is outside the local/arithmetic slice"
                    ));
                }
            }
            let result = record.result.ok_or_else(|| {
                format!(
                    "target emit: instruction {} is not the binary arithmetic shape",
                    instruction_id.index()
                )
            })?;
            let slot = layout.value_slots.get(&result).ok_or_else(|| {
                format!("target emit: missing slot for value {}", result.index())
            })?;
            store_value(assembly, *slot);
            Ok(())
        }
        IrOp::Neg => {
            let operand = record.operands.first().ok_or_else(|| {
                format!(
                    "target emit: instruction {} is not the Neg shape",
                    instruction_id.index()
                )
            })?;
            let operand_slot = layout.value_slots.get(operand).ok_or_else(|| {
                format!("target emit: missing slot for value {}", operand.index())
            })?;
            load_value(assembly, *operand_slot, 0);
            assembly.push_str("    neg w0, w0\n");
            let result = record.result.ok_or_else(|| {
                format!(
                    "target emit: instruction {} is not the Neg shape",
                    instruction_id.index()
                )
            })?;
            let slot = layout.value_slots.get(&result).ok_or_else(|| {
                format!("target emit: missing slot for value {}", result.index())
            })?;
            store_value(assembly, *slot);
            Ok(())
        }
        IrOp::Call => {
            for (index, argument) in record.operands.iter().enumerate() {
                let slot = layout.value_slots.get(argument).ok_or_else(|| {
                    format!("target emit: missing slot for value {}", argument.index())
                })?;
                load_value(assembly, *slot, index as u8);
            }
            let target = call_targets.get(&instruction_id).ok_or_else(|| {
                format!(
                    "target emit: unresolved callee in instruction {}",
                    instruction_id.index()
                )
            })?;
            let name = names.get(target).ok_or_else(|| {
                format!(
                    "target emit: missing name for callee function {}",
                    target.index()
                )
            })?;
            assembly.push_str(&format!("    bl {}\n", name.text));
            let result = record.result.ok_or_else(|| {
                format!(
                    "target emit: instruction {} is not the Call shape",
                    instruction_id.index()
                )
            })?;
            let slot = layout.value_slots.get(&result).ok_or_else(|| {
                format!("target emit: missing slot for value {}", result.index())
            })?;
            store_value(assembly, *slot);
            Ok(())
        }
        IrOp::Return => {
            let result = record.operands.first().ok_or_else(|| {
                format!(
                    "target emit: instruction {} is not the Return shape",
                    instruction_id.index()
                )
            })?;
            let slot = layout.value_slots.get(result).ok_or_else(|| {
                format!("target emit: missing slot for value {}", result.index())
            })?;
            load_value(assembly, *slot, 0);
            if layout.nonleaf {
                let lr = layout
                    .frame
                    .checked_sub(8)
                    .ok_or_else(|| "target emit: non-leaf frame is too small".to_string())?;
                assembly.push_str(&format!("    ldr x30, [sp, #{lr}]\n"));
            }
            if layout.frame > 0 {
                assembly.push_str(&format!("    add sp, sp, #{}\n", layout.frame));
            }
            assembly.push_str("    ret\n");
            Ok(())
        }
        IrOp::Branch | IrOp::CondBranch => Err(format!(
            "target emit: op {:?} is outside the local/arithmetic slice",
            record.op
        )),
    }
}

fn load_value(assembly: &mut String, slot: u32, register: u8) {
    assembly.push_str(&format!("    ldr w{register}, [sp, #{slot}]\n"));
}

fn store_value(assembly: &mut String, slot: u32) {
    assembly.push_str(&format!("    str w0, [sp, #{slot}]\n"));
}

/// Frame size for `slot_count` 4-byte slots plus the non-leaf LR slot,
/// aligned to the AArch64 stack requirement and bounded to the largest
/// 16-byte-aligned `sub sp, sp, #imm` value.
fn frame_size(slot_count: usize, nonleaf: bool) -> Result<u32, String> {
    let bytes = (slot_count as u64)
        .checked_mul(4)
        .ok_or_else(|| "target emit: stack frame size overflow".to_string())?;
    let bytes = if nonleaf {
        bytes
            .checked_add(8)
            .ok_or_else(|| "target emit: stack frame size overflow".to_string())?
    } else {
        bytes
    };
    let frame = bytes
        .checked_add(15)
        .ok_or_else(|| "target emit: stack frame size overflow".to_string())?
        & !15u64;
    let frame = if nonleaf { frame.max(16) } else { frame };
    if frame > u64::from(MAX_FRAME_BYTES) {
        return Err(format!(
            "target emit: frame exceeds the AArch64 immediate range ({frame} bytes)"
        ));
    }
    u32::try_from(frame).map_err(|_| "target emit: stack frame size overflow".to_string())
}

fn check_local(bus: &CompilerBus, int: TypeId, local: SymbolId) -> Result<(), String> {
    let symbol: &SymbolRecord = bus
        .arenas
        .symbols
        .get(local)
        .map_err(|_| format!("target emit: missing local symbol {}", local.index()))?;
    if symbol.kind != SymbolKind::Object
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

fn check_value_type(bus: &CompilerBus, int: TypeId, value: ValueId) -> Result<(), String> {
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

fn validate_identifier(bytes: &[u8]) -> Result<&str, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "target emit: unsafe name (invalid UTF-8)".to_string())?;
    let mut chars = text.bytes();
    let valid_start = chars
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_');
    if !valid_start
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(format!("target emit: unsafe name {text:?}"));
    }
    if is_reserved_register(text) {
        return Err(format!("target emit: unsafe name {text:?}"));
    }
    Ok(text)
}

fn is_reserved_register(name: &str) -> bool {
    if let Some(rest) = name.strip_prefix('x').or_else(|| name.strip_prefix('w')) {
        if rest.parse::<u32>().is_ok_and(|index| index <= 30) {
            return true;
        }
    }
    for prefix in ['b', 'h', 's', 'd', 'q', 'v', 'z'] {
        if let Some(rest) = name.strip_prefix(prefix) {
            if rest.parse::<u32>().is_ok_and(|index| index <= 31) {
                return true;
            }
        }
    }
    matches!(
        name,
        "sp" | "wsp"
            | "xzr"
            | "wzr"
            | "lr"
            | "fp"
            | "ip0"
            | "ip1"
            | "pc"
            | "pstate"
            | "nzcv"
            | "eq"
            | "ne"
            | "cs"
            | "hs"
            | "cc"
            | "lo"
            | "mi"
            | "pl"
            | "vs"
            | "vc"
            | "hi"
            | "ls"
            | "ge"
            | "lt"
            | "gt"
            | "le"
            | "al"
            | "nv"
    )
}

fn integer_value(bus: &CompilerBus, int: TypeId, id: ConstId) -> Result<i32, String> {
    let record = bus
        .arenas
        .consts
        .get(id)
        .map_err(|_| format!("target emit: missing constant {}", id.index()))?;
    if record.ty != int {
        return Err(format!(
            "target emit: constant {} is not the canonical int",
            id.index()
        ));
    }
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
    use super::{emit_m1_assembly, frame_size};
    use cc_silicon_compiler::bus::{
        BlockRecord, CompilerBus, ConstRecord, ConstValue, FunctionRecord, InstructionRecord,
        IntRank, IntegerConst, IrOp, Linkage, ScopeKind, ScopeRecord, StorageDuration, SymbolKind,
        SymbolRecord, TypeKind, TypeRecord, ValueRecord,
    };
    use cc_silicon_compiler::ids::{BlockId, NodeId, SymbolId, TypeId, ValueId};

    struct Fixture {
        bus: CompilerBus,
        int: TypeId,
        block: BlockId,
        local: SymbolId,
    }

    impl Fixture {
        fn new() -> Self {
            let mut bus = CompilerBus::default();
            let limits = bus.limits();
            let main_name = bus.intern.intern(b"main", &limits).unwrap();
            let local_name = bus.intern.intern(b"x", &limits).unwrap();
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
            let file_scope = bus
                .arenas
                .scopes
                .alloc(
                    ScopeRecord {
                        kind: ScopeKind::File,
                        parent: None,
                        owner: None,
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
                        name: main_name,
                        scope: file_scope,
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
                        name: local_name,
                        scope: file_scope,
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
                        parameters: vec![],
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
                        callee: None,
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
                        origins: vec![],
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

    #[test]
    fn constant_return_preserves_signed_bits() {
        let mut fixture = Fixture::new();
        let minus_one = fixture.constant(-1);
        fixture.ret(minus_one);
        let asm = fixture.asm();
        assert!(asm.contains("movz w0, #65535"), "{asm}");
        assert!(asm.contains("movk w0, #65535, lsl #16"), "{asm}");
        assert!(!asm.contains("sub sp, sp"), "{asm}");
        run_native(&asm, 255);
    }

    #[test]
    fn frame_sizes_align_and_fail_closed() {
        assert_eq!(frame_size(0, false).unwrap(), 0);
        assert_eq!(frame_size(0, true).unwrap(), 16);
        assert_eq!(frame_size(1, false).unwrap(), 16);
        assert_eq!(frame_size(5, false).unwrap(), 32);
        assert_eq!(frame_size(1020, false).unwrap(), 4080);
        assert!(frame_size(1021, false).is_err());
        assert!(frame_size(1024, false).is_err());
    }

    #[cfg(target_os = "linux")]
    fn run_native(assembly: &str, expected_exit: u8) {
        use std::process::Command;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let machine = Command::new("gcc")
            .arg("-dumpmachine")
            .output()
            .expect("gcc available");
        if !String::from_utf8_lossy(&machine.stdout).contains("aarch64") {
            return;
        }
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "m1-target-unit-{}-{}-{}",
            std::process::id(),
            serial,
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
