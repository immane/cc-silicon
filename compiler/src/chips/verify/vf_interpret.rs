// ============================================================================
// chips/verify/vf_interpret.rs — T13 VF12 symbolic IR interpreter
// (Wave 2 slice 8, `/17`; `/55` direct-call slice)
//
// Reads the committed IR arenas and models the covered straight-line
// subset without executing target code and without target widths:
//
//   * canonical no-call `Constant` + `Return` completes `Record` of the
//     modeled `ConstId` (the `/17` result);
//   * local programs complete `Integer` (the `/49`–`/53` result);
//   * direct-call programs complete the full signed `Integer` result.
//
// Every function owns exactly one entry block and a straight-line
// instruction sequence in ascending `InstructionId` order ending in one
// `Return`. Calls resolve through `FunctionRecord.symbol` to exactly one
// function; the call graph is checked for cycles before execution. Each
// invocation has its own value/local frame, and the interpreter is bounded by
// `CALL_MAX_DEPTH` live frames and `CALL_MAX_STEPS` executed instructions.
// Any non-covered shape or runtime fault is a loud typed failure; the chip
// never panics and never writes a record.
// ============================================================================

use crate::bus::{
    BlockRecord, ConstRecord, ConstValue, FunctionRecord, InstructionRecord, IntegerConst, IrOp,
    StorageDuration, SymbolKind, SymbolRecord, TypeKind, TypeRecord, ValueRecord,
};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{
    BlockId, ConstId, FunctionId, InstructionId, RecordRef, SymbolId, TaskId, TypeId, ValueId,
};
use crate::limits::{
    CALL_MAX_ARGUMENTS, CALL_MAX_DEPTH, CALL_MAX_FUNCTIONS, CALL_MAX_PARAMETERS, CALL_MAX_STEPS,
};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, VF12_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};
use std::collections::{BTreeMap, BTreeSet};

/// Narrow projection for the symbolic-interpret computation.
#[derive(Clone, Debug)]
pub struct Vf12Input {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Selected main function.
    pub function: FunctionId,
    /// Committed selected function body.
    pub function_body: FunctionRecord,
    /// Blocks owned by the selected function (one today).
    pub blocks: Vec<(BlockId, BlockRecord)>,
    /// Selected function instructions in ascending `InstructionId` order.
    pub instructions: Vec<(InstructionId, InstructionRecord)>,
    /// The returned value and its committed body (legacy `/17` staging).
    pub ret_value: (ValueId, ValueRecord),
    /// The modeled constant and its committed body (legacy `/17` staging).
    pub modeled: (ConstId, ConstRecord),
    /// Canonical `int` type.
    pub int: TypeId,
    /// Committed constants, in ascending ID order.
    pub consts: Vec<(ConstId, ConstRecord)>,
    /// Every committed function, in ascending ID order.
    pub functions: Vec<(FunctionId, FunctionRecord)>,
    /// Every committed block, in ascending ID order.
    pub all_blocks: Vec<(BlockId, BlockRecord)>,
    /// Every committed instruction, in ascending ID order.
    pub all_instructions: Vec<(InstructionId, InstructionRecord)>,
    /// Every committed value, in ascending ID order.
    pub values: Vec<(ValueId, ValueRecord)>,
    /// Every committed type, in ascending ID order.
    pub types: Vec<(TypeId, TypeRecord)>,
    /// Every committed symbol, in ascending ID order.
    pub symbols: Vec<(SymbolId, SymbolRecord)>,
}

/// Build the narrow projection for one dispatched task.
pub fn project_vf12_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<Vf12Input, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("vf12 of unknown task {}", task.index())))?;
    if record.kind != TaskKind::VERIFICATION_IR_INTERPRET {
        return Err(protocol_fault(format!(
            "vf12 task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "vf12 task {} payload must carry exactly one function",
            task.index()
        )));
    }
    let function = match record.payload.refs[0] {
        RecordRef::Function(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "vf12 task {} payload must be a function",
                task.index()
            )));
        }
    };
    let missing = |what: &str, index: u32| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("vf12 reads missing {what} {index}"),
        )
    };
    let function_body = bus
        .arenas
        .functions
        .get(function)
        .map_err(|_| missing("function", function.index()))?
        .clone();
    let mut functions: Vec<(FunctionId, FunctionRecord)> = bus
        .arenas
        .functions
        .iter()
        .map(|(id, body)| (id, body.clone()))
        .collect();
    functions.sort_by_key(|(id, _)| id.index());
    let mut all_blocks: Vec<(BlockId, BlockRecord)> = bus
        .arenas
        .blocks
        .iter()
        .map(|(id, body)| (id, body.clone()))
        .collect();
    all_blocks.sort_by_key(|(id, _)| id.index());
    let mut all_instructions: Vec<(InstructionId, InstructionRecord)> = bus
        .arenas
        .instructions
        .iter()
        .map(|(id, body)| (id, body.clone()))
        .collect();
    all_instructions.sort_by_key(|(id, _)| id.index());
    let mut blocks: Vec<(BlockId, BlockRecord)> = all_blocks
        .iter()
        .filter(|(_, body)| body.function == function)
        .cloned()
        .collect();
    blocks.sort_by_key(|(id, _)| id.index());
    let selected_blocks: BTreeSet<BlockId> = blocks.iter().map(|(id, _)| *id).collect();
    let mut instructions: Vec<(InstructionId, InstructionRecord)> = all_instructions
        .iter()
        .filter(|(_, body)| selected_blocks.contains(&body.block))
        .cloned()
        .collect();
    instructions.sort_by_key(|(id, _)| id.index());
    // Canonical `int` must be committed (the only M1 value type).
    let mut int = None;
    for (id, record) in bus.arenas.types.iter() {
        if record.kind
            == (crate::bus::TypeKind::Int {
                rank: crate::bus::IntRank::Int,
                signed: true,
            })
        {
            int = Some(id);
            break;
        }
    }
    let Some(int) = int else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "vf12 reads missing canonical int type",
        ));
    };
    // The modeled constant and returned value are resolved in `compute`
    // from the instruction shapes; the projector only stages the arenas
    // the shapes point at. Load every value and const the shapes may name
    // so `compute` never touches the bus.
    let mut values: Vec<(ValueId, ValueRecord)> = bus
        .arenas
        .values
        .iter()
        .map(|(id, body)| (id, body.clone()))
        .collect();
    values.sort_by_key(|(id, _)| id.index());
    let mut consts: Vec<(ConstId, ConstRecord)> = bus
        .arenas
        .consts
        .iter()
        .map(|(id, body)| (id, body.clone()))
        .collect();
    consts.sort_by_key(|(id, _)| id.index());
    let mut types: Vec<(TypeId, TypeRecord)> = bus
        .arenas
        .types
        .iter()
        .map(|(id, body)| (id, body.clone()))
        .collect();
    types.sort_by_key(|(id, _)| id.index());
    let mut symbols: Vec<(SymbolId, SymbolRecord)> = bus
        .arenas
        .symbols
        .iter()
        .map(|(id, body)| (id, body.clone()))
        .collect();
    symbols.sort_by_key(|(id, _)| id.index());
    // Resolve the Return operand value and the Constant immediate here so
    // the narrow input carries committed bodies, not lookup keys. The
    // canonical shape has exactly one `Return` naming one value produced by
    // one `Constant`; anything else fails in `compute`, never here.
    let ret_operand = instructions
        .iter()
        .find(|(_, body)| body.op == IrOp::Return)
        .and_then(|(_, body)| body.operands.first().copied());
    let modeled_id = instructions
        .iter()
        .find(|(_, body)| body.op == IrOp::Constant)
        .and_then(|(_, body)| body.immediate);
    let (Some(ret_value_id), Some(modeled_id)) = (ret_operand, modeled_id) else {
        // Shape detail stays in `compute`: stage an empty marker and let
        // the shape rule fail loudly. Use the task's own IDs as sentinels
        // that no shape rule can accept (see `compute`).
        return Ok(Vf12Input {
            task,
            state: record.state.clone(),
            function,
            function_body,
            blocks,
            instructions,
            ret_value: (
                ValueId::from_index(u32::MAX),
                ValueRecord {
                    ty: TypeId::from_index(u32::MAX),
                },
            ),
            modeled: (
                ConstId::from_index(u32::MAX),
                ConstRecord {
                    ty: TypeId::from_index(u32::MAX),
                    value: ConstValue::Integer(IntegerConst {
                        magnitude: Vec::new(),
                        negative: false,
                    }),
                    origins: Vec::new(),
                },
            ),
            int,
            consts,
            functions,
            all_blocks,
            all_instructions,
            values,
            types,
            symbols,
        });
    };
    let Some(ret_value) = values.iter().find(|(id, _)| *id == ret_value_id).cloned() else {
        return Err(missing("value", ret_value_id.index()));
    };
    let Some(modeled) = consts.iter().find(|(id, _)| *id == modeled_id).cloned() else {
        return Err(missing("const", modeled_id.index()));
    };
    Ok(Vf12Input {
        task,
        state: record.state.clone(),
        function,
        function_body,
        blocks,
        instructions,
        ret_value,
        modeled,
        int,
        consts,
        functions,
        all_blocks,
        all_instructions,
        values,
        types,
        symbols,
    })
}

/// Build the loud failure for a non-covered IR shape (never a pass).
fn shape_fail(task: TaskId, detail: &str) -> Proposal {
    fail(
        task,
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("vf12: non-covered IR shape ({detail})"),
        ),
    )
}

/// Validated, deterministically ordered straight-line program.
struct Prepared {
    functions: BTreeMap<FunctionId, FunctionRecord>,
    instructions: BTreeMap<FunctionId, Vec<(InstructionId, InstructionRecord)>>,
    consts: BTreeMap<ConstId, ConstRecord>,
    call_targets: BTreeMap<SymbolId, FunctionId>,
}

impl Prepared {
    /// Execute one invocation with a fresh frame.
    fn execute(
        &self,
        function: FunctionId,
        arguments: &[i32],
        depth: usize,
        steps: &mut u64,
    ) -> Result<i32, String> {
        if depth > CALL_MAX_DEPTH {
            return Err(format!(
                "call depth {} exceeds live-frame budget {}",
                depth, CALL_MAX_DEPTH
            ));
        }
        let body = self
            .functions
            .get(&function)
            .ok_or_else(|| "execution names a missing function".to_string())?;
        if arguments.len() != body.parameters.len() {
            return Err("call argument count differs from the signature".to_string());
        }
        let mut values: BTreeMap<ValueId, i32> = BTreeMap::new();
        let mut locals: BTreeMap<SymbolId, i32> = BTreeMap::new();
        for (parameter, argument) in body.parameters.iter().zip(arguments.iter()) {
            if locals.insert(*parameter, *argument).is_some() {
                return Err("duplicate parameter binding".to_string());
            }
        }
        let instructions = self
            .instructions
            .get(&function)
            .ok_or_else(|| "execution names a function without instructions".to_string())?;
        for (instruction_id, instruction) in instructions {
            *steps = steps
                .checked_add(1)
                .ok_or_else(|| "step counter overflow".to_string())?;
            if *steps > CALL_MAX_STEPS {
                return Err(format!(
                    "executed step {} exceeds instruction budget {}",
                    *steps, CALL_MAX_STEPS
                ));
            }
            match instruction.op {
                IrOp::Constant => {
                    let const_id = instruction
                        .immediate
                        .ok_or_else(|| "Constant has no immediate".to_string())?;
                    let result = instruction
                        .result
                        .ok_or_else(|| "Constant has no result".to_string())?;
                    let value = self
                        .consts
                        .get(&const_id)
                        .ok_or_else(|| "Constant names a missing const".to_string())?;
                    let value = const_i32(value)
                        .ok_or_else(|| "Constant is not a canonical int".to_string())?;
                    if values.insert(result, value).is_some() {
                        return Err("duplicate value id".to_string());
                    }
                }
                IrOp::LocalLoad => {
                    let local = instruction
                        .local
                        .ok_or_else(|| "LocalLoad has no local".to_string())?;
                    let result = instruction
                        .result
                        .ok_or_else(|| "LocalLoad has no result".to_string())?;
                    let value = locals
                        .get(&local)
                        .copied()
                        .ok_or_else(|| "LocalLoad reads an uninitialized local".to_string())?;
                    if values.insert(result, value).is_some() {
                        return Err("duplicate value id".to_string());
                    }
                }
                IrOp::LocalStore => {
                    let local = instruction
                        .local
                        .ok_or_else(|| "LocalStore has no local".to_string())?;
                    let operand = instruction
                        .operands
                        .first()
                        .copied()
                        .ok_or_else(|| "LocalStore has no operand".to_string())?;
                    let value = values
                        .get(&operand)
                        .copied()
                        .ok_or_else(|| "LocalStore reads a missing value".to_string())?;
                    locals.insert(local, value);
                }
                IrOp::Add | IrOp::Sub | IrOp::Mul | IrOp::Div | IrOp::Rem => {
                    let result = instruction
                        .result
                        .ok_or_else(|| "binary instruction has no result".to_string())?;
                    let lhs = instruction
                        .operands
                        .first()
                        .and_then(|id| values.get(id))
                        .copied()
                        .ok_or_else(|| "binary instruction reads a missing lhs".to_string())?;
                    let rhs = instruction
                        .operands
                        .get(1)
                        .and_then(|id| values.get(id))
                        .copied()
                        .ok_or_else(|| "binary instruction reads a missing rhs".to_string())?;
                    let value = match instruction.op {
                        IrOp::Add => lhs.checked_add(rhs),
                        IrOp::Sub => lhs.checked_sub(rhs),
                        IrOp::Mul => lhs.checked_mul(rhs),
                        IrOp::Div => {
                            if rhs == 0 {
                                return Err("division by zero".to_string());
                            }
                            lhs.checked_div(rhs)
                        }
                        IrOp::Rem => {
                            if rhs == 0 {
                                return Err("remainder by zero".to_string());
                            }
                            lhs.checked_rem(rhs)
                        }
                        _ => None,
                    }
                    .ok_or_else(|| "checked integer arithmetic overflow".to_string())?;
                    if values.insert(result, value).is_some() {
                        return Err("duplicate value id".to_string());
                    }
                }
                IrOp::Neg => {
                    let result = instruction
                        .result
                        .ok_or_else(|| "Neg has no result".to_string())?;
                    let operand = instruction
                        .operands
                        .first()
                        .and_then(|id| values.get(id))
                        .copied()
                        .ok_or_else(|| "Neg reads a missing value".to_string())?;
                    let value = operand
                        .checked_neg()
                        .ok_or_else(|| "Neg overflow".to_string())?;
                    if values.insert(result, value).is_some() {
                        return Err("duplicate value id".to_string());
                    }
                }
                IrOp::Call => {
                    let result = instruction
                        .result
                        .ok_or_else(|| "Call has no result".to_string())?;
                    let callee = instruction
                        .callee
                        .ok_or_else(|| "Call has no callee".to_string())?;
                    let target = self
                        .call_targets
                        .get(&callee)
                        .copied()
                        .ok_or_else(|| "Call names a missing function".to_string())?;
                    let mut arguments = Vec::with_capacity(instruction.operands.len());
                    for operand in &instruction.operands {
                        let value = values.get(operand).copied().ok_or_else(|| {
                            "Call reads an argument before its definition".to_string()
                        })?;
                        arguments.push(value);
                    }
                    let value = self.execute(target, &arguments, depth + 1, steps)?;
                    if values.insert(result, value).is_some() {
                        return Err("duplicate value id".to_string());
                    }
                }
                IrOp::Return => {
                    let operand = instruction
                        .operands
                        .first()
                        .copied()
                        .ok_or_else(|| "Return has no operand".to_string())?;
                    let value = values
                        .get(&operand)
                        .copied()
                        .ok_or_else(|| "Return reads a missing value".to_string())?;
                    return Ok(value);
                }
                IrOp::Branch | IrOp::CondBranch => {
                    return Err(format!(
                        "control flow is not covered at instruction {}",
                        instruction_id.index()
                    ));
                }
            }
        }
        Err("function has no Return".to_string())
    }
}

/// Whether one constant record carries a canonical signed `i32`.
fn const_i32(record: &ConstRecord) -> Option<i32> {
    let integer = match &record.value {
        ConstValue::Integer(integer) => integer,
        ConstValue::Float(_) | ConstValue::Address(_) => return None,
    };
    let value = canonical_integer_i128(integer)?;
    i32::try_from(value).ok()
}

/// The VF12 symbolic interpreter.
pub struct Vf12Chip;

impl Worker for Vf12Chip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: VF12_CHIP,
            chip_name: "Vf12Chip",
            group: TaskGroup::VERIFICATION,
            task_kinds: vec![TaskKind::VERIFICATION_IR_INTERPRET],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Ir, "functions"),
                FieldPath::new(StoreId::Ir, "blocks"),
                FieldPath::new(StoreId::Ir, "values"),
                FieldPath::new(StoreId::Ir, "instructions"),
                FieldPath::new(StoreId::Constants, "records"),
                FieldPath::new(StoreId::Types, "records"),
                FieldPath::new(StoreId::Symbols, "symbols"),
            ],
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec![
                "compiler/tests/c17_vf12.rs",
                "compiler/tests/call_verify.rs",
            ],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_vf12_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl Vf12Chip {
    /// Pure symbolic interpretation over the narrow projection (no bus
    /// access, no arithmetic on target state, no store writes).
    pub fn compute(&self, input: &Vf12Input) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("vf12 task {} is not running", input.task.index()),
                ),
            )];
        }
        let prepared = match prepare(input) {
            Ok(prepared) => prepared,
            Err(detail) => return vec![shape_fail(input.task, &detail)],
        };
        let Some(selected) = prepared.instructions.get(&input.function) else {
            return vec![shape_fail(
                input.task,
                "selected function has no instructions",
            )];
        };
        // Preserve the `/17` canonical no-call result: one `Constant` feeding
        // the single `Return` completes the modeled `ConstId`.
        let no_calls = prepared
            .instructions
            .values()
            .flatten()
            .all(|(_, instruction)| instruction.op != IrOp::Call);
        if no_calls && selected.len() == 2 {
            let (_, constant) = &selected[0];
            let (_, ret) = &selected[1];
            if constant.op == IrOp::Constant
                && ret.op == IrOp::Return
                && constant.result.is_some()
                && constant.result == ret.operands.first().copied()
            {
                let Some(const_id) = constant.immediate else {
                    return vec![shape_fail(
                        input.task,
                        "canonical constant has no immediate",
                    )];
                };
                if (!input.modeled.0.is_none() && input.modeled.0 != const_id)
                    || (!input.ret_value.0.is_none() && constant.result != Some(input.ret_value.0))
                {
                    return vec![shape_fail(input.task, "dangling shape marker")];
                }
                if !input.modeled.0.is_none()
                    && !input.ret_value.0.is_none()
                    && input.modeled.1.ty != input.ret_value.1.ty
                {
                    return vec![shape_fail(
                        input.task,
                        "constant immediate type differs from value type",
                    )];
                }
                return vec![Proposal::Complete {
                    task: input.task,
                    value: ResultValue::Record(RecordRef::Const(const_id)),
                }];
            }
        }
        let mut steps = 0u64;
        match prepared.execute(input.function, &[], 1, &mut steps) {
            Ok(value) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Integer(encode_canonical_int(value)),
            }],
            Err(detail) => vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("vf12: direct-call interpretation failed ({detail})"),
                ),
            )],
        }
    }
}

/// Validate every committed function, block, instruction, value, constant,
/// symbol, signature, and the call graph before any execution begins.
fn prepare(input: &Vf12Input) -> Result<Prepared, String> {
    if input.int.index() == u32::MAX {
        return Err("canonical int type is missing".to_string());
    }
    // Instruction IDs are arena order: reject a narrow projection that
    // reorders or duplicates them instead of silently sorting.
    for instructions in [&input.instructions, &input.all_instructions] {
        for pair in instructions.windows(2) {
            if pair[0].0.index() >= pair[1].0.index() {
                return Err("instructions are not in ascending InstructionId order".to_string());
            }
        }
    }
    let mut types: BTreeMap<TypeId, TypeRecord> = BTreeMap::new();
    for (id, record) in &input.types {
        if types.insert(*id, record.clone()).is_some() {
            return Err("duplicate type id".to_string());
        }
    }
    match types.get(&input.int).map(|record| &record.kind) {
        Some(TypeKind::Int {
            rank: crate::bus::IntRank::Int,
            signed: true,
        }) => {}
        _ => return Err("canonical int type is not committed".to_string()),
    }
    let mut functions: BTreeMap<FunctionId, FunctionRecord> = BTreeMap::new();
    for (id, record) in &input.functions {
        if functions.insert(*id, record.clone()).is_some() {
            return Err("duplicate function id".to_string());
        }
    }
    if !functions.contains_key(&input.function) {
        return Err("selected main function is not committed".to_string());
    }
    if functions.len() > CALL_MAX_FUNCTIONS {
        return Err("function count is outside the budget".to_string());
    }
    // The selected function body is the narrow-input authority; other
    // functions come from the committed vector.
    functions.insert(input.function, input.function_body.clone());
    let mut symbols: BTreeMap<SymbolId, SymbolRecord> = BTreeMap::new();
    for (id, record) in &input.symbols {
        if symbols.insert(*id, record.clone()).is_some() {
            return Err("duplicate symbol id".to_string());
        }
    }
    let mut call_symbols: BTreeMap<SymbolId, Vec<FunctionId>> = BTreeMap::new();
    let mut parameter_symbols: BTreeSet<SymbolId> = BTreeSet::new();
    for (function_id, function) in &functions {
        if function.parameters.len() > CALL_MAX_PARAMETERS {
            return Err("function parameter count is outside the budget".to_string());
        }
        let signature = types
            .get(&function.signature)
            .ok_or_else(|| "function signature type is missing".to_string())?;
        let TypeKind::Function {
            result,
            params,
            prototype,
            variadic,
        } = &signature.kind
        else {
            return Err("function signature is not a function type".to_string());
        };
        if *result != input.int
            || !*prototype
            || *variadic
            || params.len() > CALL_MAX_PARAMETERS
            || params.iter().any(|param| *param != input.int)
            || params.len() != function.parameters.len()
        {
            return Err("function signature does not match its parameters".to_string());
        }
        call_symbols
            .entry(function.symbol)
            .or_default()
            .push(*function_id);
        for parameter in &function.parameters {
            if !parameter_symbols.insert(*parameter) {
                return Err("duplicate parameter symbol".to_string());
            }
            let symbol = symbols
                .get(parameter)
                .ok_or_else(|| "parameter symbol is not committed".to_string())?;
            if symbol.kind != SymbolKind::Object
                || symbol.storage != StorageDuration::Automatic
                || symbol.ty != Some(input.int)
            {
                return Err("parameter is not an automatic int object".to_string());
            }
        }
    }
    let mut all_blocks: BTreeMap<BlockId, BlockRecord> = BTreeMap::new();
    for (id, record) in &input.all_blocks {
        if all_blocks.insert(*id, record.clone()).is_some() {
            return Err("duplicate block id".to_string());
        }
        if !functions.contains_key(&record.function) {
            return Err("block names a missing function".to_string());
        }
    }
    let mut all_instructions: BTreeMap<InstructionId, InstructionRecord> = BTreeMap::new();
    for (id, record) in &input.all_instructions {
        if all_instructions.insert(*id, record.clone()).is_some() {
            return Err("duplicate instruction id".to_string());
        }
        if !all_blocks.contains_key(&record.block) {
            return Err("instruction names a missing block".to_string());
        }
    }
    let mut selected_blocks_by_id: BTreeMap<BlockId, BlockRecord> = BTreeMap::new();
    for (id, record) in &input.blocks {
        if selected_blocks_by_id.insert(*id, record.clone()).is_some() {
            return Err("duplicate selected block id".to_string());
        }
    }
    let committed_selected: BTreeSet<BlockId> = all_blocks
        .iter()
        .filter(|(_, record)| record.function == input.function)
        .map(|(id, _)| *id)
        .collect();
    let mirror_selected: BTreeSet<BlockId> = selected_blocks_by_id.keys().copied().collect();
    if committed_selected != mirror_selected {
        return Err("selected block projection differs from the committed blocks".to_string());
    }
    let mut selected_instructions_by_id: BTreeMap<InstructionId, InstructionRecord> =
        BTreeMap::new();
    for (id, record) in &input.instructions {
        if selected_instructions_by_id
            .insert(*id, record.clone())
            .is_some()
        {
            return Err("duplicate selected instruction id".to_string());
        }
    }
    let selected_block_ids = mirror_selected;
    let mut instructions: BTreeMap<FunctionId, Vec<(InstructionId, InstructionRecord)>> =
        BTreeMap::new();
    for function_id in functions.keys() {
        let mut block_ids: Vec<BlockId> = if *function_id == input.function {
            selected_block_ids.iter().copied().collect()
        } else {
            all_blocks
                .iter()
                .filter(|(_, record)| record.function == *function_id)
                .map(|(id, _)| *id)
                .collect()
        };
        block_ids.sort_by_key(|id| id.index());
        if block_ids.len() != 1 {
            return Err("function does not own exactly one entry block".to_string());
        }
        let block_id = block_ids[0];
        let block = all_blocks
            .get(&block_id)
            .ok_or_else(|| "function entry block is missing".to_string())?;
        if block.function != *function_id || block.ordinal != 0 {
            return Err("function entry block ownership or ordinal is wrong".to_string());
        }
        let function = functions
            .get(function_id)
            .ok_or_else(|| "function is missing".to_string())?;
        if function.entry != block_id {
            return Err("function entry does not name its block".to_string());
        }
        let mut function_instructions: Vec<(InstructionId, InstructionRecord)> =
            if *function_id == input.function {
                selected_instructions_by_id
                    .iter()
                    .map(|(id, record)| (*id, record.clone()))
                    .collect()
            } else {
                all_instructions
                    .iter()
                    .filter(|(_, record)| record.block == block_id)
                    .map(|(id, record)| (*id, record.clone()))
                    .collect()
            };
        function_instructions.sort_by_key(|(id, _)| id.index());
        for pair in function_instructions.windows(2) {
            if pair[0].0.index() >= pair[1].0.index() {
                return Err("instructions are not in ascending InstructionId order".to_string());
            }
        }
        for (instruction_id, instruction) in &function_instructions {
            if instruction.block != block_id {
                return Err("instruction is not owned by its function block".to_string());
            }
            validate_instruction(instruction)
                .map_err(|detail| format!("instruction {}: {detail}", instruction_id.index()))?;
        }
        let returns = function_instructions
            .iter()
            .filter(|(_, instruction)| instruction.op == IrOp::Return)
            .count();
        if returns != 1 {
            return Err("function must have exactly one Return".to_string());
        }
        if function_instructions
            .last()
            .is_none_or(|(_, instruction)| instruction.op != IrOp::Return)
        {
            return Err("Return is not the last instruction".to_string());
        }
        instructions.insert(*function_id, function_instructions);
    }
    // Every committed value has exactly one producer, and every operand is
    // produced by an earlier instruction in the same function.
    let mut values: BTreeMap<ValueId, ValueRecord> = BTreeMap::new();
    for (id, record) in &input.values {
        if values.insert(*id, record.clone()).is_some() {
            return Err("duplicate value id".to_string());
        }
        if record.ty != input.int {
            return Err("value is not canonical int".to_string());
        }
    }
    let mut producers: BTreeMap<ValueId, (FunctionId, InstructionId)> = BTreeMap::new();
    for (function_id, function_instructions) in &instructions {
        for (instruction_id, instruction) in function_instructions {
            let Some(result) = instruction.result else {
                continue;
            };
            if !values.contains_key(&result) {
                return Err("instruction result has no committed value".to_string());
            }
            if producers
                .insert(result, (*function_id, *instruction_id))
                .is_some()
            {
                return Err("duplicate value producer".to_string());
            }
        }
    }
    if values.len() != producers.len() {
        return Err("committed value has no producer".to_string());
    }
    for (function_id, function_instructions) in &instructions {
        for (instruction_id, instruction) in function_instructions {
            for operand in &instruction.operands {
                let Some((producer_function, producer_instruction)) = producers.get(operand) else {
                    return Err("operand names a missing value".to_string());
                };
                if producer_function != function_id {
                    return Err("operand crosses a function boundary".to_string());
                }
                if producer_instruction.index() >= instruction_id.index() {
                    return Err("operand is used before its definition".to_string());
                }
            }
        }
    }
    let mut consts: BTreeMap<ConstId, ConstRecord> = BTreeMap::new();
    for (id, record) in &input.consts {
        if consts.insert(*id, record.clone()).is_some() {
            return Err("duplicate const id".to_string());
        }
        if record.ty != input.int {
            return Err("constant is not canonical int".to_string());
        }
        if const_i32(record).is_none() {
            return Err("constant is not a canonical int value".to_string());
        }
    }
    let mut call_targets: BTreeMap<SymbolId, FunctionId> = BTreeMap::new();
    let mut adjacency: BTreeMap<FunctionId, BTreeSet<FunctionId>> = BTreeMap::new();
    for function_id in functions.keys() {
        adjacency.entry(*function_id).or_default();
    }
    for (function_id, function_instructions) in &instructions {
        for (_, instruction) in function_instructions {
            if instruction.op != IrOp::Call {
                continue;
            }
            let callee = instruction
                .callee
                .ok_or_else(|| "Call has no callee".to_string())?;
            let candidates = call_symbols
                .get(&callee)
                .ok_or_else(|| "Call target is not a defined function".to_string())?;
            let target = match candidates.as_slice() {
                [only] => *only,
                _ => return Err("Call target resolves to duplicate functions".to_string()),
            };
            let target_body = functions
                .get(&target)
                .ok_or_else(|| "Call target is missing".to_string())?;
            if instruction.operands.len() != target_body.parameters.len() {
                return Err("Call argument count differs from the signature".to_string());
            }
            call_targets.entry(callee).or_insert(target);
            adjacency.entry(*function_id).or_default().insert(target);
        }
    }
    if has_cycle(&adjacency) {
        return Err("call graph contains a cycle".to_string());
    }
    Ok(Prepared {
        functions,
        instructions,
        consts,
        call_targets,
    })
}

/// Validate one instruction's closed `/55` shape.
fn validate_instruction(instruction: &InstructionRecord) -> Result<(), String> {
    if !instruction.successors.is_empty() {
        return Err("successors are not covered".to_string());
    }
    match instruction.op {
        IrOp::Constant => {
            if !instruction.operands.is_empty()
                || instruction.immediate.is_none()
                || instruction.result.is_none()
                || instruction.local.is_some()
                || instruction.callee.is_some()
            {
                return Err("Constant shape is malformed".to_string());
            }
        }
        IrOp::LocalLoad => {
            if !instruction.operands.is_empty()
                || instruction.immediate.is_some()
                || instruction.result.is_none()
                || instruction.local.is_none()
                || instruction.callee.is_some()
            {
                return Err("LocalLoad shape is malformed".to_string());
            }
        }
        IrOp::LocalStore => {
            if instruction.operands.len() != 1
                || instruction.immediate.is_some()
                || instruction.result.is_some()
                || instruction.local.is_none()
                || instruction.callee.is_some()
            {
                return Err("LocalStore shape is malformed".to_string());
            }
        }
        IrOp::Add | IrOp::Sub | IrOp::Mul | IrOp::Div | IrOp::Rem => {
            if instruction.operands.len() != 2
                || instruction.immediate.is_some()
                || instruction.result.is_none()
                || instruction.local.is_some()
                || instruction.callee.is_some()
            {
                return Err("binary instruction shape is malformed".to_string());
            }
        }
        IrOp::Neg => {
            if instruction.operands.len() != 1
                || instruction.immediate.is_some()
                || instruction.result.is_none()
                || instruction.local.is_some()
                || instruction.callee.is_some()
            {
                return Err("Neg shape is malformed".to_string());
            }
        }
        IrOp::Call => {
            if instruction.operands.len() > CALL_MAX_ARGUMENTS
                || instruction.immediate.is_some()
                || instruction.result.is_none()
                || instruction.local.is_some()
                || instruction.callee.is_none()
            {
                return Err("Call shape is malformed".to_string());
            }
        }
        IrOp::Return => {
            if instruction.operands.len() != 1
                || instruction.immediate.is_some()
                || instruction.result.is_some()
                || instruction.local.is_some()
                || instruction.callee.is_some()
            {
                return Err("Return shape is malformed".to_string());
            }
        }
        IrOp::Branch | IrOp::CondBranch => {
            return Err("control flow is not covered".to_string());
        }
    }
    Ok(())
}

/// Deterministic cycle detection by Kahn's algorithm.
fn has_cycle(adjacency: &BTreeMap<FunctionId, BTreeSet<FunctionId>>) -> bool {
    let mut indegree: BTreeMap<FunctionId, usize> =
        adjacency.keys().map(|id| (*id, 0usize)).collect();
    for targets in adjacency.values() {
        for target in targets {
            if let Some(count) = indegree.get_mut(target) {
                *count += 1;
            }
        }
    }
    let mut ready: BTreeSet<FunctionId> = indegree
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(id, _)| *id)
        .collect();
    let mut visited = 0usize;
    while let Some(id) = ready.iter().next().copied() {
        ready.remove(&id);
        visited += 1;
        if let Some(targets) = adjacency.get(&id) {
            for target in targets {
                if let Some(count) = indegree.get_mut(target) {
                    *count -= 1;
                    if *count == 0 {
                        ready.insert(*target);
                    }
                }
            }
        }
    }
    visited != adjacency.len()
}

fn canonical_integer_i128(integer: &IntegerConst) -> Option<i128> {
    if integer.magnitude.len() > 16 {
        return None;
    }
    let mut magnitude: u128 = 0;
    for byte in &integer.magnitude {
        magnitude = (magnitude << 8) | u128::from(*byte);
    }
    if integer.negative {
        if magnitude > (i128::MAX as u128) + 1 {
            return None;
        }
        if magnitude == (i128::MAX as u128) + 1 {
            Some(i128::MIN)
        } else {
            Some(-(magnitude as i128))
        }
    } else {
        i128::try_from(magnitude).ok()
    }
}

fn encode_canonical_int(value: i32) -> IntegerConst {
    IntegerConst::new(value.unsigned_abs().to_be_bytes().to_vec(), value < 0)
}
