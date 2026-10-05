// ============================================================================
// chips/ir_lower/ir_function.rs — T09 IR01/IR02/IR03/IR19 function-lowering
// worker (Wave 2 slice 6, `/15`)
//
// Reads one committed `FunctionDefinition` node, checks the committed
// `main` symbol, its signature, the committed `SemRecord`s, and the single
// committed folded `ConstRecord`, then appends one `Function`, one `Block`,
// one `Value`, and two `Instruction`s (`Constant` + `Return`) in order and
// completes with the function reference. The `Constant` carries the
// fold's `ConstId` verbatim and never recomputes the value (no refold:
// this file contains no arithmetic). The terminator (`Return`) is the
// greatest `InstructionId`. FunctionEnd/IR28 stays deferred.
// ============================================================================

use crate::bus::{InstructionRecord, IrOp, Linkage, NodeKind, ValueCategory};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{BlockId, ConstId, FunctionId, RecordRef, SymbolId, TaskId, TypeId, ValueId};
use crate::manifest::{
    BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, IR_FUNCTION_CHIP,
};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// Narrow projection for the function-lowering computation.
#[derive(Clone, Debug)]
pub struct IrFunctionInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Checked `main` symbol.
    pub symbol: SymbolId,
    /// Its linkage (reused for the IR function).
    pub linkage: Linkage,
    /// ABI-neutral signature.
    pub signature: TypeId,
    /// Canonical `int` type (value + expected-type checks).
    pub int: TypeId,
    /// The single folded constant.
    pub constant: ConstId,
    /// Arena counts at dispatch (positional prediction bases).
    pub functions_allocated: u32,
    /// Block prediction base.
    pub blocks_allocated: u32,
    /// Value prediction base.
    pub values_allocated: u32,
    /// Instruction prediction base.
    pub instructions_allocated: u32,
}

/// Build the narrow projection for one dispatched task.
pub fn project_ir_function_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<IrFunctionInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("lower of unknown task {}", task.index())))?;
    if record.kind != TaskKind::IR_FUNCTION {
        return Err(protocol_fault(format!(
            "lower task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "lower task {} payload must carry exactly one function node",
            task.index()
        )));
    }
    let funcdef = match record.payload.refs[0] {
        RecordRef::Node(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "lower task {} payload must be a node",
                task.index()
            )));
        }
    };
    let missing = |what: &str, index: u32| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("lower reads missing {what} {index}"),
        )
    };
    let funcdef_body = bus
        .arenas
        .nodes
        .get(funcdef)
        .map_err(|_| missing("node", funcdef.index()))?;
    if funcdef_body.kind != NodeKind::FunctionDefinition {
        return Err(DiagnosticDraft::unsupported(
            "lower of a non-function-definition node",
        ));
    }
    if funcdef_body.children.len() != 3 {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("lower reads malformed function node {}", funcdef.index()),
        ));
    }
    let declarator = funcdef_body.children[1];
    let declarator_body = bus
        .arenas
        .nodes
        .get(declarator)
        .map_err(|_| missing("node", declarator.index()))?;
    if declarator_body.kind != NodeKind::Declarator {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("lower reads malformed declarator {}", declarator.index()),
        ));
    }
    // The checked `main` symbol: exactly one symbol declared at this node.
    let mut symbol = None;
    for (id, sym) in bus.arenas.symbols.iter() {
        if sym.decl == declarator {
            if symbol.is_some() {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "duplicate symbol for declarator",
                ));
            }
            symbol = Some((id, sym.clone()));
        }
    }
    let Some((symbol_id, symbol_body)) = symbol else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "lower reads undeclared function",
        ));
    };
    let Some(signature) = symbol_body.ty else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "lower reads untyped symbol",
        ));
    };
    // Canonical `int`: exactly one committed.
    let mut int = None;
    for (id, record) in bus.arenas.types.iter() {
        if record.kind
            == (crate::bus::TypeKind::Int {
                rank: crate::bus::IntRank::Int,
                signed: true,
            })
        {
            if int.is_some() {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "duplicate canonical int type",
                ));
            }
            int = Some(id);
        }
    }
    let Some(int) = int else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "canonical int type is not committed",
        ));
    };
    // The checked `Return` + `BinaryAdd` facts: presence only here (T07 owns
    // the values); the operand type check below pins `int`.
    let compound = funcdef_body.children[2];
    let compound_body = bus
        .arenas
        .nodes
        .get(compound)
        .map_err(|_| missing("node", compound.index()))?;
    if compound_body.kind != NodeKind::Compound || compound_body.children.len() != 1 {
        return Err(DiagnosticDraft::unsupported(
            "lower of a multi-statement body",
        ));
    }
    let ret = compound_body.children[0];
    let ret_body = bus
        .arenas
        .nodes
        .get(ret)
        .map_err(|_| missing("node", ret.index()))?;
    if ret_body.kind != NodeKind::Return || ret_body.children.len() != 1 {
        return Err(DiagnosticDraft::unsupported(
            "lower of a non-return statement",
        ));
    }
    let binary = ret_body.children[0];
    let binary_body = bus
        .arenas
        .nodes
        .get(binary)
        .map_err(|_| missing("node", binary.index()))?;
    if binary_body.kind != NodeKind::BinaryAdd {
        return Err(DiagnosticDraft::unsupported(
            "lower of a non-additive return operand",
        ));
    }
    for node in [ret, binary] {
        let Some((_, sem)) = bus.arenas.sem.iter().find(|(_, sem)| sem.node == node) else {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("lower reads unchecked node {}", node.index()),
            ));
        };
        if sem.ty != int || sem.category != ValueCategory::NonLvalue {
            return Err(DiagnosticDraft::unsupported(
                "lower of a non-int checked operand",
            ));
        }
        if sem.effects != crate::bus::EffectMask(0) {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("lower reads effectful node {}", node.index()),
            ));
        }
    }
    // The single folded constant: exactly one committed `ConstRecord`
    // (M1 flow folds once). Zero is missing input; more than one is an
    // ambiguous handoff — both fail loudly, never a guessed constant.
    let mut constant = None;
    for (id, _) in bus.arenas.consts.iter() {
        if constant.is_some() {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "ambiguous folded constants for lowering",
            ));
        }
        constant = Some(id);
    }
    let Some(constant) = constant else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "lower reads missing folded constant",
        ));
    };
    Ok(IrFunctionInput {
        task,
        state: record.state.clone(),
        symbol: symbol_id,
        linkage: symbol_body.linkage,
        signature,
        int,
        constant,
        functions_allocated: bus.arenas.functions.allocated(),
        blocks_allocated: bus.arenas.blocks.allocated(),
        values_allocated: bus.arenas.values.allocated(),
        instructions_allocated: bus.arenas.instructions.allocated(),
    })
}

/// The T09 function-lowering worker (IR01/IR02/IR03/IR19 slice scope).
pub struct IrFunctionChip;

impl Worker for IrFunctionChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: IR_FUNCTION_CHIP,
            chip_name: "IrFunctionChip",
            group: TaskGroup::IR_LOWER,
            task_kinds: vec![TaskKind::IR_FUNCTION],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Parse, "nodes"),
                FieldPath::new(StoreId::Symbols, "symbols"),
                FieldPath::new(StoreId::Types, "records"),
                FieldPath::new(StoreId::Sem, "records"),
                FieldPath::new(StoreId::Constants, "records"),
                FieldPath::new(StoreId::Ir, "functions"),
                FieldPath::new(StoreId::Ir, "blocks"),
                FieldPath::new(StoreId::Ir, "values"),
                FieldPath::new(StoreId::Ir, "instructions"),
            ],
            writes: vec![
                FieldPath::new(StoreId::Ir, "functions"),
                FieldPath::new(StoreId::Ir, "blocks"),
                FieldPath::new(StoreId::Ir, "values"),
                FieldPath::new(StoreId::Ir, "instructions"),
            ],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c15_ir.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_ir_function_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl IrFunctionChip {
    /// Pure semantic computation over the narrow projection (no bus access
    /// and no arithmetic: the `Constant` carries the fold's `ConstId`
    /// verbatim).
    pub fn compute(&self, input: &IrFunctionInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("lower task {} is not running", input.task.index()),
                ),
            )];
        }
        let function = FunctionId::from_index(input.functions_allocated);
        let block = BlockId::from_index(input.blocks_allocated);
        let value = ValueId::from_index(input.values_allocated);
        let linkage = input.linkage;
        vec![
            Proposal::AppendRecords {
                task: input.task,
                batch: AppendBatch {
                    records: vec![
                        RecordDraft {
                            family: crate::ids::RecordFamily::Function,
                            index: DraftRef(0),
                        },
                        RecordDraft {
                            family: crate::ids::RecordFamily::Block,
                            index: DraftRef(1),
                        },
                        RecordDraft {
                            family: crate::ids::RecordFamily::Value,
                            index: DraftRef(2),
                        },
                        RecordDraft {
                            family: crate::ids::RecordFamily::Instruction,
                            index: DraftRef(3),
                        },
                        RecordDraft {
                            family: crate::ids::RecordFamily::Instruction,
                            index: DraftRef(4),
                        },
                    ],
                    bodies: vec![
                        G1DraftBody::Function(crate::bus::FunctionRecord {
                            symbol: input.symbol,
                            signature: input.signature,
                            entry: block,
                            linkage,
                        }),
                        G1DraftBody::Block(crate::bus::BlockRecord {
                            function,
                            ordinal: 0,
                        }),
                        G1DraftBody::Value(crate::bus::ValueRecord { ty: input.int }),
                        G1DraftBody::Instruction(InstructionRecord {
                            op: IrOp::Constant,
                            block,
                            operands: vec![],
                            immediate: Some(input.constant),
                            result: Some(value),
                        }),
                        G1DraftBody::Instruction(InstructionRecord {
                            op: IrOp::Return,
                            block,
                            operands: vec![value],
                            immediate: None,
                            result: None,
                        }),
                    ],
                },
            },
            Proposal::Complete {
                task: input.task,
                value: ResultValue::Record(RecordRef::Function(function)),
            },
        ]
    }
}
