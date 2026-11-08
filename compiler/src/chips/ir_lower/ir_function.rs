// ============================================================================
// chips/ir_lower/ir_function.rs — T09 IR01/IR02/IR03/IR19 function-lowering
// worker (Wave 2 slice 6, `/15`; `/40` A3 closure)
//
// Reads one committed `FunctionDefinition` node, the committed
// `FunctionDefinition` `SemRecord` (node-keyed link, exactly one), the
// `Return`/`BinaryAdd` `SemRecord`s, and the single committed folded
// `ConstRecord`, then appends one `Function`, one `Block`, one `Value`, and
// two `Instruction`s (`Constant` + `Return`) in order and completes with
// the function reference. The signature is consumed from `SemRecord.ty`
// (`ir.function-consumes-committed-sem`); the declarator-linked
// `SymbolRecord.ty` is a structural link that is cross-checked only and is
// never used as a fallback signature source.
//
// The `Constant` carries the fold's `ConstId` verbatim and never recomputes
// the value (no refold: this file contains no arithmetic). Its result
// `ValueRecord.ty` comes from the committed `BinaryAdd` `SemRecord`, and the
// `Return` `SemRecord.ty` must agree (interim
// `ir.constant-result-type-from-sem`; `ConstRecord.ty` and
// `ir.op-immediate-type` remain deferred to B2-02). The terminator
// (`Return`) is the greatest `InstructionId`. FunctionEnd/IR28 stays
// deferred.
// ============================================================================

use crate::bus::{
    EffectMask, InstructionRecord, IrOp, Linkage, NodeKind, SemRecord, TypeKind, ValueCategory,
};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{
    BlockId, ConstId, FunctionId, NodeId, RecordRef, SemId, SymbolId, TaskId, TypeId, ValueId,
};
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
    /// ABI-neutral signature consumed from the committed
    /// `FunctionDefinition` `SemRecord`.
    pub signature: TypeId,
    /// The committed `FunctionDefinition` fact that supplied `signature`.
    pub function_sem: SemId,
    /// Canonical `int` type (signature shape + checked-fact checks).
    pub int: TypeId,
    /// Committed type of the folded expression (`BinaryAdd` `SemRecord`).
    pub constant_ty: TypeId,
    /// Committed `Return` operand type (`Return` `SemRecord`).
    pub return_ty: TypeId,
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
///
/// The check order is normative (`/40` §3.4.3): task/payload, function shape,
/// canonical `int`, committed function-definition fact, fact signature shape,
/// structural symbol cross-check, checked operand facts, then the folded
/// constant. A fresh bus with no `SemRecord`s therefore fails on the missing
/// function-definition fact before the missing `Return` fact.
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
    // (b) The M1 `int main(void){return 2+3;}` skeleton: exactly one
    // `FunctionDefinition` with declarator and single-return/additive body.
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
    // (c) Canonical `int`: exactly one committed.
    let mut int = None;
    for (id, record) in bus.arenas.types.iter() {
        if record.kind
            == (TypeKind::Int {
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
    // (d) The committed `FunctionDefinition` fact (node-keyed link). Zero is
    // missing input and never falls back to `symbol.ty`; duplicates are a
    // DEFECT; a mistyped fact is rejected even when VF06 is not on the chain.
    let mut function_fact = None;
    for (id, sem) in bus.arenas.sem.iter() {
        if sem.node != funcdef {
            continue;
        }
        if function_fact.is_some() {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!(
                    "lower reads duplicate function-definition fact for node {}",
                    funcdef.index()
                ),
            ));
        }
        function_fact = Some((id, sem.clone()));
    }
    let Some((function_sem, function_fact)) = function_fact else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "lower reads unchecked function definition",
        ));
    };
    if function_fact.category != ValueCategory::NonLvalue || function_fact.effects != EffectMask(0)
    {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!(
                "lower reads mistyped function-definition fact {}",
                function_sem.index()
            ),
        ));
    }
    // (e) `signature` is the committed fact's type (not the symbol's); it
    // must name the committed M1 `int(void)` function shape.
    let signature = function_fact.ty;
    let signature_body =
        bus.arenas.types.get(signature).map_err(|_| {
            DiagnosticDraft::unsupported("lower of an uncommitted function signature")
        })?;
    match &signature_body.kind {
        TypeKind::Function {
            result,
            params,
            prototype,
            variadic,
        } if *result == int && params.is_empty() && *prototype && !*variadic => {}
        _ => {
            return Err(DiagnosticDraft::unsupported(
                "lower of a non-M1 function signature: exactly `int(void)` required",
            ));
        }
    }
    // (f) The structural symbol link: exactly one symbol declared at the
    // declarator, cross-checked against the fact signature. A divergent
    // `symbol.ty` is never used as the signature source.
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
    match symbol_body.ty {
        Some(ty) if ty == signature => {}
        Some(_) => {
            return Err(DiagnosticDraft::unsupported(
                "lower of a definition inconsistent with its prototype",
            ));
        }
        None => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "lower reads untyped symbol",
            ));
        }
    }
    // (g) The checked `Return` + `BinaryAdd` facts: exactly one each; the
    // operand type check pins `int`. Their committed types seed the interim
    // constant/return type rule.
    let unique_sem = |node: NodeId| -> Result<SemRecord, DiagnosticDraft> {
        let mut found = None;
        for (_, sem) in bus.arenas.sem.iter() {
            if sem.node != node {
                continue;
            }
            if found.is_some() {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "lower reads duplicate checked fact for node {}",
                        node.index()
                    ),
                ));
            }
            found = Some(sem.clone());
        }
        found.ok_or_else(|| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("lower reads unchecked node {}", node.index()),
            )
        })
    };
    let return_sem = unique_sem(ret)?;
    let binary_sem = unique_sem(binary)?;
    for sem in [&return_sem, &binary_sem] {
        if sem.ty != int || sem.category != ValueCategory::NonLvalue {
            return Err(DiagnosticDraft::unsupported(
                "lower of a non-int checked operand",
            ));
        }
        if sem.effects != EffectMask(0) {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("lower reads effectful node {}", sem.node.index()),
            ));
        }
    }
    // (h) The single folded constant: exactly one committed `ConstRecord`
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
        function_sem,
        int,
        constant_ty: binary_sem.ty,
        return_ty: return_sem.ty,
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
        // Interim `ir.constant-result-type-from-sem`: the `Constant` result
        // type is the committed `BinaryAdd` fact type, and the `Return`
        // operand fact must carry the same type. `ConstRecord.ty` is not
        // consulted; the immediate-type rule arrives with B2-02.
        if input.return_ty != input.constant_ty {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "lower reads constant type {} with return type {}",
                        input.constant_ty.index(),
                        input.return_ty.index()
                    ),
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
                        G1DraftBody::Value(crate::bus::ValueRecord {
                            ty: input.constant_ty,
                        }),
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
