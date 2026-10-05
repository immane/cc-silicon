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

use crate::bus::{BlockRecord, ConstRecord, FunctionRecord, InstructionRecord, IrOp, ValueRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{BlockId, ConstId, FunctionId, InstructionId, RecordRef, TaskId, TypeId, ValueId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, VF12_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

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
                    value: Vec::new(),
                    negative: false,
                },
            ),
            int,
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
        // Exactly `Constant` then `Return`, in ascending ID order.
        if input.instructions.len() != 2 {
            return vec![shape_fail(
                input.task,
                "block has more than two instructions",
            )];
        }
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
        vec![Proposal::Complete {
            task: input.task,
            value: ResultValue::Record(RecordRef::Const(input.modeled.0)),
        }]
    }
}
