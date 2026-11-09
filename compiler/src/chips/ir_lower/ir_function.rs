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
    BlockId, ConstId, FunctionId, LiteralId, NameId, NodeId, RecordRef, SemId, SymbolId, TaskId,
    TypeId, ValueId,
};
use crate::manifest::{
    BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, IR_FUNCTION_CHIP,
};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};
use std::collections::BTreeMap;

/// `/52` checked expression tree over one automatic local and integer
/// literals. `Identity` is a source unary plus with no runtime instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalExpr {
    /// Local load.
    Identifier {
        /// Committed `Identifier` node.
        node: NodeId,
    },
    /// Constant materialization from a committed fold.
    Literal {
        /// Committed `IntLiteral` node.
        node: NodeId,
        /// Folded constant for that literal.
        constant: ConstId,
    },
    /// Binary arithmetic over two subexpressions.
    Binary {
        /// Arithmetic instruction.
        op: IrOp,
        /// Left operand.
        lhs: Box<LocalExpr>,
        /// Right operand.
        rhs: Box<LocalExpr>,
    },
    /// Unary negation.
    Unary {
        /// Unary instruction (`Neg` today).
        op: IrOp,
        /// Operand.
        operand: Box<LocalExpr>,
    },
    /// Unary plus: lower the operand directly.
    Identity(Box<LocalExpr>),
}

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
    /// `/52` checked local expression tree; `None` for non-local shapes.
    pub local_expr: Option<LocalExpr>,
    /// `/53` assignment RHS expressions in source order.
    pub local_assignments: Vec<LocalExpr>,
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
        children
            if children.len() >= 2
                && children.last().is_some()
                && bus
                    .arenas
                    .nodes
                    .get(children[0])
                    .map(|body| body.kind == NodeKind::Declaration)
                    .unwrap_or(false) =>
        {
            let declaration_body = bus
                .arenas
                .nodes
                .get(children[0])
                .map_err(|_| missing("node", children[0].index()))?;
            if declaration_body.children.len() != 3 {
                return Err(DiagnosticDraft::unsupported(
                    "lower of a non-covered declaration",
                ));
            }
            let declarator = declaration_body.children[1];
            let initializer = declaration_body.children[2];
            let ret = *children.last().expect("checked above");
            (ret, Some((declarator, initializer)))
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
    if local_shape.is_none() && operand_body.kind != NodeKind::BinaryAdd {
        return Err(DiagnosticDraft::unsupported(
            "lower of a non-additive canonical return operand",
        ));
    }
    if local_shape.is_some()
        && !matches!(
            operand_body.kind,
            NodeKind::Identifier
                | NodeKind::IntLiteral
                | NodeKind::BinaryAdd
                | NodeKind::BinarySub
                | NodeKind::BinaryMul
                | NodeKind::BinaryDiv
                | NodeKind::BinaryRem
                | NodeKind::UnaryPlus
                | NodeKind::UnaryMinus
        )
    {
        return Err(DiagnosticDraft::unsupported(
            "lower of a non-covered local expression root",
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
    let mut local_expr = None;
    let mut local_assignments = Vec::new();
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
        let compound_body = bus
            .arenas
            .nodes
            .get(compound)
            .map_err(|_| missing("node", compound.index()))?;
        let assignment_roots: Vec<NodeId> = compound_body.children[1..]
            .split_last()
            .map(|(_, rest)| {
                rest.iter()
                    .filter_map(|statement| {
                        bus.arenas
                            .nodes
                            .get(*statement)
                            .ok()
                            .and_then(|body| (body.kind == NodeKind::Assignment).then_some(body))
                            .and_then(|body| body.children.get(1).copied())
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut literal_order = vec![literal];
        for rhs in &assignment_roots {
            collect_expression_literals(bus, *rhs, &mut literal_order)?;
        }
        collect_expression_literals(bus, operand, &mut literal_order)?;
        let consts: Vec<(ConstId, &crate::bus::ConstRecord)> = bus.arenas.consts.iter().collect();
        if consts.len() != literal_order.len() {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!(
                    "lower reads {} folded constants for {} literals",
                    consts.len(),
                    literal_order.len()
                ),
            ));
        }
        let mut literal_consts: BTreeMap<LiteralId, ConstId> = BTreeMap::new();
        for (literal, (constant, body)) in literal_order.iter().zip(consts) {
            let literal_body = bus
                .arenas
                .literals
                .get(*literal)
                .map_err(|_| missing("literal", literal.index()))?;
            let ConstValue::Integer(integer) = &body.value else {
                return Err(DiagnosticDraft::unsupported(
                    "lower of a non-integer folded literal",
                ));
            };
            if integer.magnitude != literal_body.value || integer.negative != literal_body.negative
            {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "lower reads a folded value that does not match literal {}",
                        literal.index()
                    ),
                ));
            }
            if body.ty != int || bus.arenas.types.get(body.ty).is_err() {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "lower reads literal {} with a non-int or dangling type",
                        literal.index()
                    ),
                ));
            }
            literal_consts.insert(*literal, constant);
        }
        let constant = *literal_consts.get(&literal).ok_or_else(|| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "lower reads no folded initializer",
            )
        })?;
        let constant_record_ty = int;
        for rhs in assignment_roots {
            local_assignments.push(build_local_expr(
                bus,
                rhs,
                local_name,
                int,
                &literal_consts,
            )?);
        }
        local_expr = Some(build_local_expr(
            bus,
            operand,
            local_name,
            int,
            &literal_consts,
        )?);
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
        local_expr,
        local_assignments,
        functions_allocated: bus.arenas.functions.allocated(),
        blocks_allocated: bus.arenas.blocks.allocated(),
        values_allocated: bus.arenas.values.allocated(),
        instructions_allocated: bus.arenas.instructions.allocated(),
    })
}

fn lower_local_expression(
    expression: &LocalExpr,
    local: SymbolId,
    block: BlockId,
    value_base: u32,
    next_value: &mut u32,
    instructions: &mut Vec<InstructionRecord>,
) -> ValueId {
    match expression {
        LocalExpr::Literal { constant, .. } => {
            let result = ValueId::from_index(value_base + *next_value);
            *next_value += 1;
            instructions.push(InstructionRecord {
                op: IrOp::Constant,
                block,
                operands: vec![],
                immediate: Some(*constant),
                result: Some(result),
                local: None,
                successors: vec![],
                callee: None,
            });
            result
        }
        LocalExpr::Identifier { .. } => {
            let result = ValueId::from_index(value_base + *next_value);
            *next_value += 1;
            instructions.push(InstructionRecord {
                op: IrOp::LocalLoad,
                block,
                operands: vec![],
                immediate: None,
                result: Some(result),
                local: Some(local),
                successors: vec![],
                callee: None,
            });
            result
        }
        LocalExpr::Binary { op, lhs, rhs } => {
            let lhs =
                lower_local_expression(lhs, local, block, value_base, next_value, instructions);
            let rhs =
                lower_local_expression(rhs, local, block, value_base, next_value, instructions);
            let result = ValueId::from_index(value_base + *next_value);
            *next_value += 1;
            instructions.push(InstructionRecord {
                op: *op,
                block,
                operands: vec![lhs, rhs],
                immediate: None,
                result: Some(result),
                local: None,
                successors: vec![],
                callee: None,
            });
            result
        }
        LocalExpr::Unary { op, operand } => {
            let operand =
                lower_local_expression(operand, local, block, value_base, next_value, instructions);
            let result = ValueId::from_index(value_base + *next_value);
            *next_value += 1;
            instructions.push(InstructionRecord {
                op: *op,
                block,
                operands: vec![operand],
                immediate: None,
                result: Some(result),
                local: None,
                successors: vec![],
                callee: None,
            });
            result
        }
        LocalExpr::Identity(operand) => {
            lower_local_expression(operand, local, block, value_base, next_value, instructions)
        }
    }
}

fn checked_int_fact(
    bus: &crate::bus::CompilerBus,
    node: NodeId,
    int: TypeId,
) -> Result<(), DiagnosticDraft> {
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
    let Some(sem) = found else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("lower reads unchecked node {}", node.index()),
        ));
    };
    if sem.ty != int || sem.category != ValueCategory::NonLvalue {
        return Err(DiagnosticDraft::unsupported(
            "lower of a non-int checked expression node",
        ));
    }
    if sem.effects != EffectMask(0) {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("lower reads effectful node {}", node.index()),
        ));
    }
    Ok(())
}

fn collect_expression_literals(
    bus: &crate::bus::CompilerBus,
    node: NodeId,
    out: &mut Vec<LiteralId>,
) -> Result<(), DiagnosticDraft> {
    let body = bus.arenas.nodes.get(node).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("lower reads missing expression node {}", node.index()),
        )
    })?;
    if body.kind == NodeKind::IntLiteral {
        let Some(literal) = body.literal else {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("lower reads literal-less node {}", node.index()),
            ));
        };
        out.push(literal);
        return Ok(());
    }
    for child in &body.children {
        collect_expression_literals(bus, *child, out)?;
    }
    Ok(())
}

fn build_local_expr(
    bus: &crate::bus::CompilerBus,
    node: NodeId,
    local_name: NameId,
    int: TypeId,
    literal_consts: &BTreeMap<LiteralId, ConstId>,
) -> Result<LocalExpr, DiagnosticDraft> {
    let body = bus.arenas.nodes.get(node).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("lower reads missing expression node {}", node.index()),
        )
    })?;
    checked_int_fact(bus, node, int)?;
    match body.kind {
        NodeKind::IntLiteral => {
            let Some(literal) = body.literal else {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("lower reads literal-less node {}", node.index()),
                ));
            };
            let Some(constant) = literal_consts.get(&literal).copied() else {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "lower reads no folded constant for literal {}",
                        literal.index()
                    ),
                ));
            };
            Ok(LocalExpr::Literal { node, constant })
        }
        NodeKind::Identifier => {
            if body.name != Some(local_name) {
                return Err(DiagnosticDraft::unsupported(
                    "lower of an identifier that does not name the local",
                ));
            }
            Ok(LocalExpr::Identifier { node })
        }
        NodeKind::BinaryAdd
        | NodeKind::BinarySub
        | NodeKind::BinaryMul
        | NodeKind::BinaryDiv
        | NodeKind::BinaryRem => {
            if body.children.len() != 2 {
                return Err(DiagnosticDraft::unsupported(
                    "lower of a malformed binary expression",
                ));
            }
            let op = match body.kind {
                NodeKind::BinaryAdd => IrOp::Add,
                NodeKind::BinarySub => IrOp::Sub,
                NodeKind::BinaryMul => IrOp::Mul,
                NodeKind::BinaryDiv => IrOp::Div,
                NodeKind::BinaryRem => IrOp::Rem,
                _ => unreachable!(),
            };
            Ok(LocalExpr::Binary {
                op,
                lhs: Box::new(build_local_expr(
                    bus,
                    body.children[0],
                    local_name,
                    int,
                    literal_consts,
                )?),
                rhs: Box::new(build_local_expr(
                    bus,
                    body.children[1],
                    local_name,
                    int,
                    literal_consts,
                )?),
            })
        }
        NodeKind::UnaryPlus => {
            if body.children.len() != 1 {
                return Err(DiagnosticDraft::unsupported(
                    "lower of a malformed unary plus",
                ));
            }
            Ok(LocalExpr::Identity(Box::new(build_local_expr(
                bus,
                body.children[0],
                local_name,
                int,
                literal_consts,
            )?)))
        }
        NodeKind::UnaryMinus => {
            if body.children.len() != 1 {
                return Err(DiagnosticDraft::unsupported(
                    "lower of a malformed unary minus",
                ));
            }
            Ok(LocalExpr::Unary {
                op: IrOp::Neg,
                operand: Box::new(build_local_expr(
                    bus,
                    body.children[0],
                    local_name,
                    int,
                    literal_consts,
                )?),
            })
        }
        _ => Err(DiagnosticDraft::unsupported(
            "lower of a non-covered local expression node",
        )),
    }
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
            return self.compute_local_expression(input);
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
                            parameters: vec![],
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
                            callee: None,
                        }),
                        G1DraftBody::Instruction(InstructionRecord {
                            op: IrOp::Return,
                            block,
                            operands: vec![value],
                            immediate: None,
                            result: None,
                            local: None,
                            successors: vec![],
                            callee: None,
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

    /// `/52` source-local expression lowering: materialize the initializer,
    /// store it, lower the checked expression tree in post-order, then return
    /// the root value.
    fn compute_local_expression(&self, input: &IrFunctionInput) -> Vec<Proposal> {
        let (Some(local), Some(expression)) = (input.local, input.local_expr.as_ref()) else {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "lower reads no local expression",
                ),
            )];
        };
        let function = FunctionId::from_index(input.functions_allocated);
        let block = BlockId::from_index(input.blocks_allocated);
        let linkage = input.linkage;
        let mut next_value = 0u32;
        let mut instructions = Vec::new();
        let initial = ValueId::from_index(input.values_allocated);
        next_value += 1;
        instructions.push(InstructionRecord {
            op: IrOp::Constant,
            block,
            operands: vec![],
            immediate: Some(input.constant),
            result: Some(initial),
            local: None,
            successors: vec![],
            callee: None,
        });
        instructions.push(InstructionRecord {
            op: IrOp::LocalStore,
            block,
            operands: vec![initial],
            immediate: None,
            result: None,
            local: Some(local),
            successors: vec![],
            callee: None,
        });
        for assignment in &input.local_assignments {
            let value = lower_local_expression(
                assignment,
                local,
                block,
                input.values_allocated,
                &mut next_value,
                &mut instructions,
            );
            instructions.push(InstructionRecord {
                op: IrOp::LocalStore,
                block,
                operands: vec![value],
                immediate: None,
                result: None,
                local: Some(local),
                successors: vec![],
                callee: None,
            });
        }
        let returned = lower_local_expression(
            expression,
            local,
            block,
            input.values_allocated,
            &mut next_value,
            &mut instructions,
        );
        instructions.push(InstructionRecord {
            op: IrOp::Return,
            block,
            operands: vec![returned],
            immediate: None,
            result: None,
            local: None,
            successors: vec![],
            callee: None,
        });
        let value_count = next_value as usize;
        let mut records = Vec::with_capacity(2 + value_count + instructions.len());
        for index in 0..2 + value_count + instructions.len() {
            let family = if index == 0 {
                crate::ids::RecordFamily::Function
            } else if index == 1 {
                crate::ids::RecordFamily::Block
            } else if index <= 1 + value_count {
                crate::ids::RecordFamily::Value
            } else {
                crate::ids::RecordFamily::Instruction
            };
            records.push(RecordDraft {
                family,
                index: DraftRef(index as u32),
            });
        }
        let mut bodies = Vec::with_capacity(2 + value_count + instructions.len());
        bodies.push(G1DraftBody::Function(crate::bus::FunctionRecord {
            symbol: input.symbol,
            signature: input.signature,
            entry: block,
            linkage,
            parameters: vec![],
        }));
        bodies.push(G1DraftBody::Block(crate::bus::BlockRecord {
            function,
            ordinal: 0,
        }));
        for _ in 0..value_count {
            bodies.push(G1DraftBody::Value(crate::bus::ValueRecord {
                ty: input.constant_ty,
            }));
        }
        for instruction in instructions {
            bodies.push(G1DraftBody::Instruction(instruction));
        }
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
