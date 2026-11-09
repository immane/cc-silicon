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
// `Return` `SemRecord.ty` must agree (`ir.constant-result-type-from-sem`).
// `/42` additionally requires the committed `ConstRecord.ty` to equal the
// `Constant` result type (`ir.op-immediate-type`). The terminator (`Return`)
// is the greatest `InstructionId`. FunctionEnd/IR28 stays deferred.
// ============================================================================

use crate::bus::{
    ConstValue, EffectMask, InstructionRecord, IrOp, Linkage, NodeKind, SemRecord, StorageDuration,
    SymbolKind, TypeKind, ValueCategory,
};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{
    BlockId, ConstId, FunctionId, LiteralId, NodeId, RecordRef, SemId, SymbolId, TaskId, TypeId,
    ValueId,
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
    /// Committed type carried by the folded `ConstRecord` (`/42`).
    pub constant_record_ty: TypeId,
    /// The single folded constant.
    pub constant: ConstId,
    /// Automatic local for the `/49` locals shape; `None` for the canonical
    /// constant-return shape.
    pub local: Option<SymbolId>,
    /// `/50` right-hand folded constant for `local + literal`.
    pub local_rhs: Option<ConstId>,
    /// `/50` supported local expression operator (`Add` only today).
    pub local_op: Option<IrOp>,
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
    if compound_body.kind != NodeKind::Compound {
        return Err(DiagnosticDraft::unsupported(
            "lower of a non-compound function body",
        ));
    }
    // `/49`: canonical single-return body or one local declaration followed
    // by one return. Anything else stays unsupported.
    let (ret, local_shape) = match compound_body.children.as_slice() {
        [ret] => (*ret, None),
        [declaration, ret] => {
            let declaration_body = bus
                .arenas
                .nodes
                .get(*declaration)
                .map_err(|_| missing("node", declaration.index()))?;
            if declaration_body.kind != NodeKind::Declaration
                || declaration_body.children.len() != 3
            {
                return Err(DiagnosticDraft::unsupported(
                    "lower of a non-covered declaration",
                ));
            }
            let declarator = declaration_body.children[1];
            let initializer = declaration_body.children[2];
            (*ret, Some((declarator, initializer)))
        }
        _ => {
            return Err(DiagnosticDraft::unsupported(
                "lower of a body outside the closed M1 shapes",
            ));
        }
    };
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
    let operand = ret_body.children[0];
    let operand_body = bus
        .arenas
        .nodes
        .get(operand)
        .map_err(|_| missing("node", operand.index()))?;
    let mut return_identifier = None;
    let mut local_rhs_node = None;
    let mut local_ir_op = None;
    match (local_shape, operand_body.kind) {
        (None, NodeKind::BinaryAdd) => {}
        (Some(_), NodeKind::Identifier) => {
            return_identifier = Some(operand);
        }
        (
            Some(_),
            NodeKind::BinaryAdd
            | NodeKind::BinarySub
            | NodeKind::BinaryMul
            | NodeKind::BinaryDiv
            | NodeKind::BinaryRem,
        ) => {
            local_ir_op = Some(match operand_body.kind {
                NodeKind::BinaryAdd => IrOp::Add,
                NodeKind::BinarySub => IrOp::Sub,
                NodeKind::BinaryMul => IrOp::Mul,
                NodeKind::BinaryDiv => IrOp::Div,
                NodeKind::BinaryRem => IrOp::Rem,
                _ => unreachable!(),
            });
            if operand_body.children.len() != 2 {
                return Err(DiagnosticDraft::unsupported(
                    "lower of a malformed local binary return",
                ));
            }
            for child in &operand_body.children {
                let child_body = bus
                    .arenas
                    .nodes
                    .get(*child)
                    .map_err(|_| missing("node", child.index()))?;
                match child_body.kind {
                    NodeKind::Identifier if return_identifier.is_none() => {
                        return_identifier = Some(*child);
                    }
                    NodeKind::IntLiteral if local_rhs_node.is_none() => {
                        local_rhs_node = Some(*child);
                    }
                    _ => {
                        return Err(DiagnosticDraft::unsupported(
                            "lower of a non-covered local binary operand",
                        ));
                    }
                }
            }
            if return_identifier.is_none() || local_rhs_node.is_none() {
                return Err(DiagnosticDraft::unsupported(
                    "lower of a local binary return without an identifier and literal",
                ));
            }
        }
        _ => {
            return Err(DiagnosticDraft::unsupported(
                "lower of a non-covered return operand",
            ));
        }
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
    // (g) The checked `Return` and operand facts: exactly one each; the
    // operand type check pins `int`. Their committed types seed the `/40`
    // result-type rule and the `/42` immediate-type rule.
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
    let operand_sem = unique_sem(operand)?;
    for sem in [&return_sem, &operand_sem] {
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
    let mut local_symbol = None;
    let mut local_rhs = None;
    let mut local_op = None;
    let (constant, constant_record_ty) = if let Some((local_declarator, initializer)) = local_shape
    {
        // The declaration's declarator names the automatic object.
        let declarator_body = bus
            .arenas
            .nodes
            .get(local_declarator)
            .map_err(|_| missing("node", local_declarator.index()))?;
        if declarator_body.kind != NodeKind::Declarator {
            return Err(DiagnosticDraft::unsupported(
                "lower of a non-declarator initializer",
            ));
        }
        let Some(local_name) = declarator_body.name else {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "lower reads nameless local declarator",
            ));
        };
        let mut symbols = bus
            .arenas
            .symbols
            .iter()
            .filter(|(_, symbol)| symbol.decl == local_declarator);
        let Some((symbol_id, symbol_body)) = symbols.next() else {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "lower reads undeclared local",
            ));
        };
        if symbols.next().is_some() {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "lower reads duplicate local symbol",
            ));
        }
        if symbol_body.name != local_name
            || symbol_body.kind != SymbolKind::Object
            || symbol_body.storage != StorageDuration::Automatic
            || symbol_body.linkage != Linkage::None
            || symbol_body.ty != Some(int)
        {
            return Err(DiagnosticDraft::unsupported(
                "lower of a non-automatic-int local",
            ));
        }
        let return_identifier = return_identifier.ok_or_else(|| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "lower reads no identifier for the local return",
            )
        })?;
        let return_body = bus
            .arenas
            .nodes
            .get(return_identifier)
            .map_err(|_| missing("node", return_identifier.index()))?;
        if return_body.kind != NodeKind::Identifier || return_body.name != Some(local_name) {
            return Err(DiagnosticDraft::unsupported(
                "lower of a return identifier that does not name the local",
            ));
        }
        let matching_const = |literal: LiteralId, what: &str| {
            let literal_body = bus
                .arenas
                .literals
                .get(literal)
                .map_err(|_| missing("literal", literal.index()))?;
            let mut matches = Vec::new();
            for (id, body) in bus.arenas.consts.iter() {
                let ConstValue::Integer(integer) = &body.value else {
                    continue;
                };
                if integer.magnitude == literal_body.value
                    && integer.negative == literal_body.negative
                {
                    matches.push((id, body.ty));
                }
            }
            if matches.len() != 1 {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("lower reads missing or ambiguous folded {what}"),
                ));
            }
            let (constant, ty) = matches[0];
            if ty != int || bus.arenas.types.get(ty).is_err() {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("lower reads {what} with a non-int or dangling type"),
                ));
            }
            Ok(constant)
        };
        // The initializer is one committed integer literal. Match its folded
        // constant by value; the match must be unique.
        let initializer_body = bus
            .arenas
            .nodes
            .get(initializer)
            .map_err(|_| missing("node", initializer.index()))?;
        if initializer_body.kind != NodeKind::IntLiteral {
            return Err(DiagnosticDraft::unsupported(
                "lower of a non-literal local initializer",
            ));
        }
        let Some(literal) = initializer_body.literal else {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "lower reads literal-less local initializer",
            ));
        };
        let constant = matching_const(literal, "initializer")?;
        let constant_record_ty = int;
        if let Some(rhs_node) = local_rhs_node {
            let rhs_body = bus
                .arenas
                .nodes
                .get(rhs_node)
                .map_err(|_| missing("node", rhs_node.index()))?;
            if rhs_body.kind != NodeKind::IntLiteral {
                return Err(DiagnosticDraft::unsupported(
                    "lower of a non-literal local binary operand",
                ));
            }
            let Some(rhs_literal) = rhs_body.literal else {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "lower reads literal-less local binary operand",
                ));
            };
            local_rhs = Some(matching_const(rhs_literal, "right-hand operand")?);
            local_op = local_ir_op;
        }
        local_symbol = Some(symbol_id);
        (constant, constant_record_ty)
    } else {
        // The canonical single folded constant: exactly one committed
        // `ConstRecord` (M1 flow folds once). Zero is missing input; more
        // than one is an ambiguous handoff — both fail loudly, never a
        // guessed constant.
        let mut constant = None;
        for (id, body) in bus.arenas.consts.iter() {
            if constant.is_some() {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "ambiguous folded constants for lowering",
                ));
            }
            constant = Some((id, body.ty));
        }
        let Some((constant, constant_record_ty)) = constant else {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "lower reads missing folded constant",
            ));
        };
        if bus.arenas.types.get(constant_record_ty).is_err() {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!(
                    "lower reads constant with dangling type {}",
                    constant_record_ty.index()
                ),
            ));
        }
        (constant, constant_record_ty)
    };
    Ok(IrFunctionInput {
        task,
        state: record.state.clone(),
        symbol: symbol_id,
        linkage: symbol_body.linkage,
        signature,
        function_sem,
        int,
        constant_ty: operand_sem.ty,
        return_ty: return_sem.ty,
        constant_record_ty,
        constant,
        local: local_symbol,
        local_rhs,
        local_op,
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
        // `ir.constant-result-type-from-sem`: the `Constant` result type is
        // the committed `BinaryAdd` fact type, and the `Return` operand fact
        // must carry the same type.
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
        // `/42` `ir.op-immediate-type`: the folded constant's committed type
        // must equal the `Constant` result value's type.
        if input.constant_record_ty != input.constant_ty {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "lower reads immediate type {} with result type {}",
                        input.constant_record_ty.index(),
                        input.constant_ty.index()
                    ),
                ),
            )];
        }
        if input.local.is_some() {
            return self.compute_local_return(input);
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
                            local: None,
                            successors: vec![],
                        }),
                        G1DraftBody::Instruction(InstructionRecord {
                            op: IrOp::Return,
                            block,
                            operands: vec![value],
                            immediate: None,
                            result: None,
                            local: None,
                            successors: vec![],
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

    /// `/49` source-locals lowering: `Constant`, `LocalStore`, `LocalLoad`,
    /// `Return`. The initializer constant is consumed verbatim; no arithmetic
    /// or folding is performed here.
    fn compute_local_return(&self, input: &IrFunctionInput) -> Vec<Proposal> {
        let Some(local) = input.local else {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "lower reads no local for the locals shape",
                ),
            )];
        };
        if input.local_rhs.is_some() || input.local_op.is_some() {
            return match (input.local_rhs, input.local_op) {
                (
                    Some(rhs),
                    Some(op @ (IrOp::Add | IrOp::Sub | IrOp::Mul | IrOp::Div | IrOp::Rem)),
                ) => self.compute_local_binary(input, local, rhs, op),
                _ => vec![fail(
                    input.task,
                    DiagnosticDraft::unsupported(
                        "lower of an unsupported local expression operator",
                    ),
                )],
            };
        }
        let function = FunctionId::from_index(input.functions_allocated);
        let block = BlockId::from_index(input.blocks_allocated);
        let initial = ValueId::from_index(input.values_allocated);
        let loaded = ValueId::from_index(input.values_allocated + 1);
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
                            family: crate::ids::RecordFamily::Value,
                            index: DraftRef(3),
                        },
                        RecordDraft {
                            family: crate::ids::RecordFamily::Instruction,
                            index: DraftRef(4),
                        },
                        RecordDraft {
                            family: crate::ids::RecordFamily::Instruction,
                            index: DraftRef(5),
                        },
                        RecordDraft {
                            family: crate::ids::RecordFamily::Instruction,
                            index: DraftRef(6),
                        },
                        RecordDraft {
                            family: crate::ids::RecordFamily::Instruction,
                            index: DraftRef(7),
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
                        G1DraftBody::Value(crate::bus::ValueRecord {
                            ty: input.constant_ty,
                        }),
                        G1DraftBody::Instruction(InstructionRecord {
                            op: IrOp::Constant,
                            block,
                            operands: vec![],
                            immediate: Some(input.constant),
                            result: Some(initial),
                            local: None,
                            successors: vec![],
                        }),
                        G1DraftBody::Instruction(InstructionRecord {
                            op: IrOp::LocalStore,
                            block,
                            operands: vec![initial],
                            immediate: None,
                            result: None,
                            local: Some(local),
                            successors: vec![],
                        }),
                        G1DraftBody::Instruction(InstructionRecord {
                            op: IrOp::LocalLoad,
                            block,
                            operands: vec![],
                            immediate: None,
                            result: Some(loaded),
                            local: Some(local),
                            successors: vec![],
                        }),
                        G1DraftBody::Instruction(InstructionRecord {
                            op: IrOp::Return,
                            block,
                            operands: vec![loaded],
                            immediate: None,
                            result: None,
                            local: None,
                            successors: vec![],
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

    /// `/50`/`/51` `local <op> literal` lowering:
    /// `Constant init`, `LocalStore`, `LocalLoad`, `Constant rhs`, `<op>`,
    /// `Return`.
    fn compute_local_binary(
        &self,
        input: &IrFunctionInput,
        local: SymbolId,
        rhs_const: ConstId,
        op: IrOp,
    ) -> Vec<Proposal> {
        let function = FunctionId::from_index(input.functions_allocated);
        let block = BlockId::from_index(input.blocks_allocated);
        let initial = ValueId::from_index(input.values_allocated);
        let loaded = ValueId::from_index(input.values_allocated + 1);
        let rhs = ValueId::from_index(input.values_allocated + 2);
        let sum = ValueId::from_index(input.values_allocated + 3);
        let linkage = input.linkage;
        let mut records = Vec::new();
        for index in 0..12 {
            let family = match index {
                0 => crate::ids::RecordFamily::Function,
                1 => crate::ids::RecordFamily::Block,
                2..=5 => crate::ids::RecordFamily::Value,
                _ => crate::ids::RecordFamily::Instruction,
            };
            records.push(RecordDraft {
                family,
                index: DraftRef(index),
            });
        }
        let bodies = vec![
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
            G1DraftBody::Value(crate::bus::ValueRecord {
                ty: input.constant_ty,
            }),
            G1DraftBody::Value(crate::bus::ValueRecord {
                ty: input.constant_ty,
            }),
            G1DraftBody::Value(crate::bus::ValueRecord {
                ty: input.constant_ty,
            }),
            G1DraftBody::Instruction(InstructionRecord {
                op: IrOp::Constant,
                block,
                operands: vec![],
                immediate: Some(input.constant),
                result: Some(initial),
                local: None,
                successors: vec![],
            }),
            G1DraftBody::Instruction(InstructionRecord {
                op: IrOp::LocalStore,
                block,
                operands: vec![initial],
                immediate: None,
                result: None,
                local: Some(local),
                successors: vec![],
            }),
            G1DraftBody::Instruction(InstructionRecord {
                op: IrOp::LocalLoad,
                block,
                operands: vec![],
                immediate: None,
                result: Some(loaded),
                local: Some(local),
                successors: vec![],
            }),
            G1DraftBody::Instruction(InstructionRecord {
                op: IrOp::Constant,
                block,
                operands: vec![],
                immediate: Some(rhs_const),
                result: Some(rhs),
                local: None,
                successors: vec![],
            }),
            G1DraftBody::Instruction(InstructionRecord {
                op,
                block,
                operands: vec![loaded, rhs],
                immediate: None,
                result: Some(sum),
                local: None,
                successors: vec![],
            }),
            G1DraftBody::Instruction(InstructionRecord {
                op: IrOp::Return,
                block,
                operands: vec![sum],
                immediate: None,
                result: None,
                local: None,
                successors: vec![],
            }),
        ];
        vec![
            Proposal::AppendRecords {
                task: input.task,
                batch: AppendBatch { records, bodies },
            },
            Proposal::Complete {
                task: input.task,
                value: ResultValue::Record(RecordRef::Function(function)),
            },
        ]
    }
}
