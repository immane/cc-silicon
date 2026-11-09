// ============================================================================
// chips/verify/vf_interpret.rs — T13 VF12 symbolic IR interpreter
// (Wave 2 slice 8, `/17`)
//
// Reads one committed `Function` and symbolically models the M1 covered
// subset (one entry block, `Constant` then `Return`) without executing
// target code and without target widths: the `Return` operand must be the
// value the `Constant` materializes from the committed `ConstRecord`.
// Completes `Record` of the modeled `ConstId`; any non-covered shape fails
// loudly (never a pass). This file contains no arithmetic and appends no
// records: the interpreter reuses the fold's committed constant verbatim.
// Multi-block functions, extra operations, and wider op coverage stay
// deferred (the frozen `IrOp` set has only the two M1 variants today).
// ============================================================================

use crate::bus::{
    BlockRecord, ConstRecord, ConstValue, FunctionRecord, InstructionRecord, IntegerConst, IrOp,
    ValueRecord,
};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{BlockId, ConstId, FunctionId, InstructionId, RecordRef, TaskId, TypeId, ValueId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, VF12_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};
use std::collections::BTreeMap;

/// Narrow projection for the symbolic-interpret computation.
#[derive(Clone, Debug)]
pub struct Vf12Input {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Interpreted function.
    pub function: FunctionId,
    /// Committed function body.
    pub function_body: FunctionRecord,
    /// Blocks owned by the function (M1: exactly one).
    pub blocks: Vec<(BlockId, BlockRecord)>,
    /// Instructions of those blocks, in ascending `InstructionId` order.
    pub instructions: Vec<(InstructionId, InstructionRecord)>,
    /// The returned value and its committed body.
    pub ret_value: (ValueId, ValueRecord),
    /// The modeled constant and its committed body.
    pub modeled: (ConstId, ConstRecord),
    /// Canonical `int` type.
    pub int: TypeId,
    /// Committed constants, in ascending ID order.
    pub consts: Vec<(ConstId, ConstRecord)>,
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
    let mut blocks: Vec<(BlockId, BlockRecord)> = bus
        .arenas
        .blocks
        .iter()
        .filter(|(_, body)| body.function == function)
        .map(|(id, body)| (id, body.clone()))
        .collect();
    blocks.sort_by_key(|(id, _)| id.index());
    let mut instructions: Vec<(InstructionId, InstructionRecord)> = bus
        .arenas
        .instructions
        .iter()
        .filter(|(_, body)| blocks.iter().any(|(bid, _)| *bid == body.block))
        .map(|(id, body)| (id, body.clone()))
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
    // Resolve the Return operand value and the Constant immediate here so
    // the narrow input carries committed bodies, not lookup keys. The
    // M1 shape has exactly one `Return` naming one value produced by one
    // `Constant`; anything else fails in `compute`, never here.
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
                },
            ),
            int,
            consts,
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

/// The VF12 symbolic interpreter (M1 covered subset).
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
            ],
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c17_vf12.rs"],
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
    /// access, no arithmetic, no store writes).
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
        // Exactly one entry block, ordinal 0, owned by the function.
        if input.blocks.len() != 1 {
            return vec![shape_fail(input.task, "function has more than one block")];
        }
        let (block_id, block_body) = &input.blocks[0];
        if block_body.function != input.function
            || input.function_body.entry != *block_id
            || block_body.ordinal != 0
        {
            return vec![shape_fail(input.task, "block is not the single entry")];
        }
        // Non-canonical local programs are interpreted as a small
        // deterministic stack machine (`/52`).
        if input.instructions.len() != 2 {
            return self.compute_local_program(input, block_id);
        }
        // Canonical `Constant` then `Return`, in ascending ID order.
        let (const_id, constant) = &input.instructions[0];
        let (ret_id, ret) = &input.instructions[1];
        if const_id.index() >= ret_id.index() {
            return vec![shape_fail(input.task, "instructions out of order")];
        }
        if constant.op != IrOp::Constant || constant.block != *block_id {
            return vec![shape_fail(
                input.task,
                "first instruction is not the block constant",
            )];
        }
        if ret.op != IrOp::Return || ret.block != *block_id {
            return vec![shape_fail(
                input.task,
                "second instruction is not the block return",
            )];
        }
        if !constant.operands.is_empty() {
            return vec![shape_fail(input.task, "constant takes operands")];
        }
        if constant.immediate != Some(input.modeled.0) || constant.result != Some(input.ret_value.0)
        {
            return vec![shape_fail(input.task, "constant names an unmodeled value")];
        }
        if ret.operands != vec![input.ret_value.0] {
            return vec![shape_fail(input.task, "return names an unmodeled value")];
        }
        if ret.immediate.is_some() || ret.result.is_some() {
            return vec![shape_fail(input.task, "return carries a payload")];
        }
        if input.ret_value.1.ty != input.int {
            return vec![shape_fail(input.task, "returned value is not int")];
        }
        if input.modeled.0.index() == u32::MAX || input.ret_value.0.index() == u32::MAX {
            return vec![shape_fail(input.task, "dangling shape marker")];
        }
        // `/42` `ir.op-immediate-type`: the committed constant and the
        // returned value must carry the same type.
        if input.modeled.1.ty != input.ret_value.1.ty {
            return vec![shape_fail(
                input.task,
                "constant immediate type differs from value type",
            )];
        }
        if input.modeled.1.value.as_integer().is_none() {
            return vec![shape_fail(input.task, "modeled constant is not integer")];
        }
        vec![Proposal::Complete {
            task: input.task,
            value: ResultValue::Record(RecordRef::Const(input.modeled.0)),
        }]
    }

    /// `/52` general local-program interpreter over the covered IR ops:
    /// `Constant`, `LocalLoad`, `LocalStore`, `Add`/`Sub`/`Mul`/`Div`/`Rem`,
    /// `Neg`, and the final `Return`. Values and locals are canonical `int`.
    fn compute_local_program(&self, input: &Vf12Input, block_id: &BlockId) -> Vec<Proposal> {
        let mut values: BTreeMap<ValueId, i32> = BTreeMap::new();
        let mut locals: BTreeMap<crate::ids::SymbolId, i32> = BTreeMap::new();
        let mut returned = None;
        for (index, (instruction_id, instruction)) in input.instructions.iter().enumerate() {
            if instruction.block != *block_id {
                return vec![shape_fail(input.task, "local-program block mismatch")];
            }
            if returned.is_some() {
                return vec![shape_fail(
                    input.task,
                    "local-program instruction after Return",
                )];
            }
            match instruction.op {
                IrOp::Constant => {
                    if !instruction.operands.is_empty()
                        || instruction.local.is_some()
                        || !instruction.successors.is_empty()
                    {
                        return vec![shape_fail(input.task, "local-program constant shape")];
                    }
                    let Some(const_id) = instruction.immediate else {
                        return vec![shape_fail(input.task, "local-program constant has no id")];
                    };
                    let Some(result) = instruction.result else {
                        return vec![shape_fail(
                            input.task,
                            "local-program constant has no value",
                        )];
                    };
                    let Some((_, body)) = input.consts.iter().find(|(id, _)| *id == const_id)
                    else {
                        return vec![shape_fail(input.task, "local-program constant is missing")];
                    };
                    let Some(integer) = body.value.as_integer() else {
                        return vec![shape_fail(
                            input.task,
                            "local-program constant is not integer",
                        )];
                    };
                    let Some(value) = canonical_integer_i128(integer) else {
                        return vec![shape_fail(
                            input.task,
                            "local-program constant is out of range",
                        )];
                    };
                    let Ok(value) = i32::try_from(value) else {
                        return vec![shape_fail(
                            input.task,
                            "local-program constant is not canonical int",
                        )];
                    };
                    if values.insert(result, value).is_some() {
                        return vec![shape_fail(input.task, "local-program duplicate value id")];
                    }
                }
                IrOp::LocalLoad => {
                    if !instruction.operands.is_empty()
                        || instruction.immediate.is_some()
                        || !instruction.successors.is_empty()
                    {
                        return vec![shape_fail(input.task, "local-program load shape")];
                    }
                    let Some(local) = instruction.local else {
                        return vec![shape_fail(input.task, "local-program load has no local")];
                    };
                    let Some(result) = instruction.result else {
                        return vec![shape_fail(input.task, "local-program load has no value")];
                    };
                    let Some(value) = locals.get(&local).copied() else {
                        return vec![shape_fail(
                            input.task,
                            "local-program load reads an uninitialized local",
                        )];
                    };
                    if values.insert(result, value).is_some() {
                        return vec![shape_fail(input.task, "local-program duplicate value id")];
                    }
                }
                IrOp::LocalStore => {
                    if instruction.operands.len() != 1
                        || instruction.immediate.is_some()
                        || instruction.result.is_some()
                        || !instruction.successors.is_empty()
                    {
                        return vec![shape_fail(input.task, "local-program store shape")];
                    }
                    let Some(local) = instruction.local else {
                        return vec![shape_fail(input.task, "local-program store has no local")];
                    };
                    let Some(value) = values.get(&instruction.operands[0]).copied() else {
                        return vec![shape_fail(
                            input.task,
                            "local-program store reads a missing value",
                        )];
                    };
                    locals.insert(local, value);
                }
                IrOp::Add | IrOp::Sub | IrOp::Mul | IrOp::Div | IrOp::Rem => {
                    if instruction.operands.len() != 2
                        || instruction.immediate.is_some()
                        || instruction.local.is_some()
                        || !instruction.successors.is_empty()
                    {
                        return vec![shape_fail(input.task, "local-program binary shape")];
                    }
                    let Some(result) = instruction.result else {
                        return vec![shape_fail(input.task, "local-program binary has no value")];
                    };
                    let Some(lhs) = values.get(&instruction.operands[0]).copied() else {
                        return vec![shape_fail(
                            input.task,
                            "local-program binary reads a missing lhs",
                        )];
                    };
                    let Some(rhs) = values.get(&instruction.operands[1]).copied() else {
                        return vec![shape_fail(
                            input.task,
                            "local-program binary reads a missing rhs",
                        )];
                    };
                    let lhs = i128::from(lhs);
                    let rhs = i128::from(rhs);
                    let result_value = match instruction.op {
                        IrOp::Add => lhs.checked_add(rhs),
                        IrOp::Sub => lhs.checked_sub(rhs),
                        IrOp::Mul => lhs.checked_mul(rhs),
                        IrOp::Div => {
                            if rhs == 0 {
                                return vec![shape_fail(
                                    input.task,
                                    "local-program division by zero",
                                )];
                            }
                            lhs.checked_div(rhs)
                        }
                        IrOp::Rem => {
                            if rhs == 0 {
                                return vec![shape_fail(
                                    input.task,
                                    "local-program remainder by zero",
                                )];
                            }
                            lhs.checked_rem(rhs)
                        }
                        _ => unreachable!(),
                    };
                    let Some(result_value) = result_value else {
                        return vec![shape_fail(input.task, "local-program arithmetic overflow")];
                    };
                    let Ok(result_value) = i32::try_from(result_value) else {
                        return vec![shape_fail(
                            input.task,
                            "local-program arithmetic is not canonical int",
                        )];
                    };
                    if values.insert(result, result_value).is_some() {
                        return vec![shape_fail(input.task, "local-program duplicate value id")];
                    }
                }
                IrOp::Neg => {
                    if instruction.operands.len() != 1
                        || instruction.immediate.is_some()
                        || instruction.local.is_some()
                        || !instruction.successors.is_empty()
                    {
                        return vec![shape_fail(input.task, "local-program neg shape")];
                    }
                    let Some(result) = instruction.result else {
                        return vec![shape_fail(input.task, "local-program neg has no value")];
                    };
                    let Some(operand) = values.get(&instruction.operands[0]).copied() else {
                        return vec![shape_fail(
                            input.task,
                            "local-program neg reads a missing value",
                        )];
                    };
                    let value = i128::from(operand).checked_neg();
                    let Some(value) = value else {
                        return vec![shape_fail(input.task, "local-program neg overflow")];
                    };
                    let Ok(value) = i32::try_from(value) else {
                        return vec![shape_fail(
                            input.task,
                            "local-program neg is not canonical int",
                        )];
                    };
                    if values.insert(result, value).is_some() {
                        return vec![shape_fail(input.task, "local-program duplicate value id")];
                    }
                }
                IrOp::Return => {
                    if instruction.operands.len() != 1
                        || instruction.immediate.is_some()
                        || instruction.result.is_some()
                        || instruction.local.is_some()
                        || !instruction.successors.is_empty()
                    {
                        return vec![shape_fail(input.task, "local-program return shape")];
                    }
                    if index + 1 != input.instructions.len() {
                        return vec![shape_fail(
                            input.task,
                            "local-program Return is not the last instruction",
                        )];
                    }
                    let Some(value) = values.get(&instruction.operands[0]).copied() else {
                        return vec![shape_fail(
                            input.task,
                            "local-program return reads a missing value",
                        )];
                    };
                    returned = Some(value);
                }
                IrOp::Branch | IrOp::CondBranch => {
                    return vec![shape_fail(
                        input.task,
                        "local-program control flow is not covered",
                    )];
                }
            }
            let _ = instruction_id;
        }
        let Some(returned) = returned else {
            return vec![shape_fail(input.task, "local-program has no Return")];
        };
        vec![Proposal::Complete {
            task: input.task,
            value: ResultValue::Integer(encode_canonical_int(returned)),
        }]
    }
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
