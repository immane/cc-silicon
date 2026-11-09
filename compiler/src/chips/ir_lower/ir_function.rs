// ============================================================================
// chips/ir_lower/ir_function.rs — T09 IR01/IR02/IR03/IR19 function-lowering
// worker (Wave 2 slice 6, `/15`; `/40` A3 closure; `/54` direct calls)
//
// Reads one committed `FunctionDefinition` node, the committed
// `FunctionDefinition` `SemRecord` (node-keyed link, exactly one), the
// checked `Return`/expression `SemRecord`s, and the committed folded
// `ConstRecord`s, then appends one `Function`, one `Block`, one `Value` per
// expression result, and the ordered `Instruction`s for the function. The
// signature is consumed from `SemRecord.ty` (`ir.function-consumes-committed-
// sem`); the declarator-linked `SymbolRecord.ty` is a structural link that is
// cross-checked only and is never used as a fallback signature source.
//
// `/54` extends the closed shape with implicit-`int` parameters and direct
// calls. Parameter values bind at function entry, so no `Parameter`
// instruction is emitted; every identifier expression carries the resolved
// object symbol and lowers to `LocalLoad`. `ConstRecord.origins` selects the
// literal-origin constant for each literal without value guessing. `Call`
// instructions name an earlier file-scope function symbol explicitly, with
// arguments in source order and the return value as the result.
// ============================================================================

use crate::bus::{
    ConstValue, EffectMask, InstructionRecord, IrOp, Linkage, NodeKind, ScopeKind, SemRecord,
    StorageDuration, SymbolKind, TypeKind, ValueCategory,
};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{
    BlockId, ConstId, FunctionId, LiteralId, NameId, NodeId, RecordRef, ScopeId, SemId, SymbolId,
    TaskId, TypeId, ValueId,
};
use crate::manifest::{
    BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, IR_FUNCTION_CHIP,
};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};
use std::collections::{BTreeMap, BTreeSet};

/// Checked expression tree over the closed `/49`–`/54` source subset.
/// `Identity` is a source unary plus with no runtime instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalExpr {
    /// Object load.
    Identifier {
        /// Committed `Identifier` node.
        node: NodeId,
        /// Object (local or parameter) named by the identifier.
        symbol: SymbolId,
    },
    /// Constant materialization from a committed literal-origin fold.
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
    /// Direct call with a resolved file-scope function symbol.
    Call {
        /// Committed `Call` node.
        node: NodeId,
        /// Resolved callee function symbol.
        callee: SymbolId,
        /// Argument expressions in source order.
        args: Vec<LocalExpr>,
    },
    /// General per-function lowering envelope. This carries the ordered
    /// parameter objects and the per-identifier object bindings without
    /// changing the frozen public records.
    Program {
        /// Parameter objects in declaration order.
        parameters: Vec<SymbolId>,
        /// Identifier node to object-symbol bindings.
        identifiers: BTreeMap<NodeId, SymbolId>,
        /// Automatic local, if the body declares one.
        local: Option<SymbolId>,
        /// Materialized initializer constant for `local`.
        initializer: Option<ConstId>,
        /// Assignment right-hand sides in source order.
        assignments: Vec<LocalExpr>,
        /// Returned expression.
        value: Box<LocalExpr>,
    },
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
    /// Committed type carried by the selected `ConstRecord` (`/42`).
    pub constant_record_ty: TypeId,
    /// The single folded constant (canonical path) or the initializer
    /// constant (general path); unused by a parameter-only program.
    pub constant: ConstId,
    /// Automatic local for the `/49` locals shape; `None` otherwise.
    pub local: Option<SymbolId>,
    /// General-path envelope (`LocalExpr::Program`) or legacy expression.
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
/// declarator/parameter shape, canonical `int`, committed function-definition
/// fact, fact signature shape, structural symbol cross-check, checked operand
/// facts, then the selected constants. A fresh bus with no `SemRecord`s
/// therefore fails on the missing function-definition fact before the missing
/// `Return` fact.
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
    // (b) `FunctionDefinition` keeps exactly three children
    // `[Specifiers, Declarator, Compound]`.
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
    // `/54`: a parameterized function declarator appends exactly one
    // `ParameterList` with 1–8 `Parameter` leaves. `(void)` and local
    // declarators keep no children.
    let parameter_nodes: Vec<NodeId> = match declarator_body.children.as_slice() {
        [] => Vec::new(),
        [list] => {
            let list_body = bus
                .arenas
                .nodes
                .get(*list)
                .map_err(|_| missing("node", list.index()))?;
            if list_body.kind != NodeKind::ParameterList {
                return Err(DiagnosticDraft::unsupported(
                    "lower of a declarator without a parameter list",
                ));
            }
            if list_body.children.is_empty() {
                return Err(DiagnosticDraft::unsupported(
                    "lower of an empty parameter list; use `(void)` for zero parameters",
                ));
            }
            if list_body.children.len() > 8 {
                return Err(DiagnosticDraft::unsupported(
                    "lower of more than eight parameters",
                ));
            }
            for parameter in &list_body.children {
                let parameter_body = bus
                    .arenas
                    .nodes
                    .get(*parameter)
                    .map_err(|_| missing("node", parameter.index()))?;
                if parameter_body.kind != NodeKind::Parameter
                    || parameter_body.name.is_none()
                    || !parameter_body.children.is_empty()
                {
                    return Err(DiagnosticDraft::unsupported(
                        "lower of a malformed parameter node",
                    ));
                }
            }
            list_body.children.clone()
        }
        _ => {
            return Err(DiagnosticDraft::unsupported(
                "lower of a malformed function declarator",
            ));
        }
    };
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
    // `/49`–`/54`: a single-return body, optionally after one local
    // declaration and zero or more assignments. Anything else stays
    // unsupported.
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
            for statement in &children[1..children.len() - 1] {
                let statement_body = bus
                    .arenas
                    .nodes
                    .get(*statement)
                    .map_err(|_| missing("node", statement.index()))?;
                if statement_body.kind != NodeKind::Assignment || statement_body.children.len() != 2 {
                    return Err(DiagnosticDraft::unsupported(
                        "lower of a non-assignment statement",
                    ));
                }
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
    let has_call = expression_contains_call(bus, operand)?;
    // The canonical single-fold path is preserved only when there are no
    // parameters, no local declaration, and no call in the returned
    // expression. Everything else uses the general per-function program.
    let canonical = parameter_nodes.is_empty() && local_shape.is_none() && !has_call;
    if canonical && operand_body.kind != NodeKind::BinaryAdd {
        return Err(DiagnosticDraft::unsupported(
            "lower of a non-additive canonical return operand",
        ));
    }
    if !canonical
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
                | NodeKind::Call
        )
    {
        return Err(DiagnosticDraft::unsupported(
            "lower of a non-covered expression root",
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
    // must name a covered `int(int, ...)` prototype with the same parameter
    // count as the AST list.
    let signature = function_fact.ty;
    let signature_body = bus.arenas.types.get(signature).map_err(|_| {
        DiagnosticDraft::unsupported("lower of an uncommitted function signature")
    })?;
    match &signature_body.kind {
        TypeKind::Function {
            result,
            params,
            prototype,
            variadic,
        } if *result == int
            && *prototype
            && !*variadic
            && params.len() == parameter_nodes.len()
            && params.len() <= 8
            && params.iter().all(|param| *param == int) => {}
        _ => {
            return Err(DiagnosticDraft::unsupported(
                "lower of a non-M1 function signature: `int` result and `int` parameters required",
            ));
        }
    }
    // (f) The structural symbol link: exactly one symbol declared at the
    // declarator, cross-checked against the fact signature. A divergent
    // `symbol.ty` is never used as the signature source.
    let (symbol_id, symbol_body) = symbol_for_decl(bus, declarator)?;
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
    // operand type check pins `int`. Their committed types seed the result
    // and immediate-type rules.
    let return_sem = unique_sem(bus, ret)?;
    let operand_sem = unique_sem(bus, operand)?;
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
    let block_scope = function_block_scope(bus, compound)?;
    if canonical {
        // The canonical single folded constant: exactly one committed
        // `ConstRecord` (M1 flow folds once). When other functions have
        // already contributed constants, select the one whose origins match
        // the folded literal pair; an ambiguous match still fails loudly.
        let (constant, constant_record_ty) = select_canonical_constant(bus, operand)?;
        return Ok(IrFunctionInput {
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
            local: None,
            local_expr: None,
            local_assignments: vec![],
            functions_allocated: bus.arenas.functions.allocated(),
            blocks_allocated: bus.arenas.blocks.allocated(),
            values_allocated: bus.arenas.values.allocated(),
            instructions_allocated: bus.arenas.instructions.allocated(),
        });
    }
    let parameters = parameter_symbols(bus, &parameter_nodes, int, block_scope)?;
    let mut known_objects: BTreeMap<NameId, SymbolId> = BTreeMap::new();
    for (parameter, symbol) in parameter_nodes.iter().zip(&parameters) {
        let body = bus
            .arenas
            .nodes
            .get(*parameter)
            .map_err(|_| missing("node", parameter.index()))?;
        let Some(name) = body.name else {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("lower reads nameless parameter {}", parameter.index()),
            ));
        };
        if known_objects.insert(name, *symbol).is_some() {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "lower reads duplicate parameter name",
            ));
        }
    }
    let local_symbol;
    let initializer;
    let assignment_roots;
    let mut literal_order = Vec::new();
    if let Some((local_declarator, initializer_node)) = local_shape {
        let declarator_body = bus
            .arenas
            .nodes
            .get(local_declarator)
            .map_err(|_| missing("node", local_declarator.index()))?;
        if declarator_body.kind != NodeKind::Declarator || !declarator_body.children.is_empty() {
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
        let (local_symbol_id, local_body) = symbol_for_decl(bus, local_declarator)?;
        if local_body.name != local_name
            || local_body.kind != SymbolKind::Object
            || local_body.storage != StorageDuration::Automatic
            || local_body.linkage != Linkage::None
            || local_body.ty != Some(int)
        {
            return Err(DiagnosticDraft::unsupported(
                "lower of a non-automatic-int local",
            ));
        }
        if let Some(scope) = block_scope {
            if local_body.scope != scope {
                return Err(DiagnosticDraft::unsupported(
                    "lower of a local outside the function block",
                ));
            }
        }
        if known_objects.insert(local_name, local_symbol_id).is_some() {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "lower reads duplicate object name",
            ));
        }
        let initializer_body = bus
            .arenas
            .nodes
            .get(initializer_node)
            .map_err(|_| missing("node", initializer_node.index()))?;
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
        literal_order.push(literal);
        assignment_roots = compound_body.children[1..]
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
                    .collect::<Vec<NodeId>>()
            })
            .unwrap_or_default();
        // `/53` assignments target the single local; parameter assignment is
        // outside this slice and is rejected rather than silently retargeted.
        for statement in &compound_body.children[1..compound_body.children.len() - 1] {
            let statement_body = bus
                .arenas
                .nodes
                .get(*statement)
                .map_err(|_| missing("node", statement.index()))?;
            let lhs = statement_body.children[0];
            let lhs_body = bus
                .arenas
                .nodes
                .get(lhs)
                .map_err(|_| missing("node", lhs.index()))?;
            if lhs_body.kind != NodeKind::Identifier || lhs_body.name != Some(local_name) {
                return Err(DiagnosticDraft::unsupported(
                    "lower of an assignment target that is not the local",
                ));
            }
        }
        local_symbol = Some(local_symbol_id);
        initializer = Some(literal);
    } else {
        local_symbol = None;
        initializer = None;
        assignment_roots = Vec::new();
    }
    for rhs in &assignment_roots {
        collect_expression_literals(bus, *rhs, &mut literal_order)?;
    }
    collect_expression_literals(bus, operand, &mut literal_order)?;
    let literal_consts = literal_constants(bus, &literal_order, int)?;
    let initializer_constant = initializer.map(|literal| {
        literal_consts
            .get(&literal)
            .copied()
            .expect("validated literal constants cover the initializer")
    });
    let mut builder = LocalExprBuilder {
        bus,
        int,
        literal_consts: &literal_consts,
        known: &known_objects,
        identifiers: BTreeMap::new(),
        enclosing: funcdef,
        current_symbol: symbol_id,
    };
    let mut assignments = Vec::with_capacity(assignment_roots.len());
    for rhs in &assignment_roots {
        assignments.push(builder.build(*rhs)?);
    }
    let value = builder.build(operand)?;
    let identifiers = builder.identifiers;
    let constant = initializer_constant
        .or_else(|| {
            literal_order
                .iter()
                .find_map(|literal| literal_consts.get(literal).copied())
        })
        .unwrap_or_else(|| ConstId::from_index(0));
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
        constant_record_ty: int,
        constant,
        local: local_symbol,
        local_expr: Some(LocalExpr::Program {
            parameters,
            identifiers,
            local: local_symbol,
            initializer: initializer_constant,
            assignments: assignments.clone(),
            value: Box::new(value),
        }),
        local_assignments: assignments,
        functions_allocated: bus.arenas.functions.allocated(),
        blocks_allocated: bus.arenas.blocks.allocated(),
        values_allocated: bus.arenas.values.allocated(),
        instructions_allocated: bus.arenas.instructions.allocated(),
    })
}

fn symbol_for_decl(
    bus: &crate::bus::CompilerBus,
    decl: NodeId,
) -> Result<(SymbolId, crate::bus::SymbolRecord), DiagnosticDraft> {
    let mut found = None;
    for (id, symbol) in bus.arenas.symbols.iter() {
        if symbol.decl != decl {
            continue;
        }
        if found.is_some() {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("lower reads duplicate symbol for declarator {}", decl.index()),
            ));
        }
        found = Some((id, symbol.clone()));
    }
    found.ok_or_else(|| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("lower reads undeclared declarator {}", decl.index()),
        )
    })
}

fn unique_sem(
    bus: &crate::bus::CompilerBus,
    node: NodeId,
) -> Result<SemRecord, DiagnosticDraft> {
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
}

/// Find the block scope owned by the function body, falling back to a single
/// committed block scope for the single-function fixtures.
fn function_block_scope(
    bus: &crate::bus::CompilerBus,
    compound: NodeId,
) -> Result<Option<ScopeId>, DiagnosticDraft> {
    let mut blocks = Vec::new();
    let mut owned = Vec::new();
    for (id, scope) in bus.arenas.scopes.iter() {
        if scope.kind != ScopeKind::Block {
            continue;
        }
        blocks.push(id);
        if scope.owner == Some(compound) {
            owned.push(id);
        }
    }
    if owned.len() > 1 {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!(
                "lower reads duplicate block scope for function node {}",
                compound.index()
            ),
        ));
    }
    if let Some(scope) = owned.first() {
        return Ok(Some(*scope));
    }
    match blocks.as_slice() {
        [] => Ok(None),
        [scope] => Ok(Some(*scope)),
        _ => Ok(None),
    }
}

fn parameter_symbols(
    bus: &crate::bus::CompilerBus,
    parameters: &[NodeId],
    int: TypeId,
    block_scope: Option<ScopeId>,
) -> Result<Vec<SymbolId>, DiagnosticDraft> {
    if parameters.is_empty() {
        return Ok(Vec::new());
    }
    let Some(block_scope) = block_scope else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "lower reads no block scope for parameters",
        ));
    };
    let mut symbols = Vec::with_capacity(parameters.len());
    let mut names: BTreeMap<NameId, NodeId> = BTreeMap::new();
    for parameter in parameters {
        let body = bus.arenas.nodes.get(*parameter).map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("lower reads missing parameter node {}", parameter.index()),
            )
        })?;
        if body.kind != NodeKind::Parameter || !body.children.is_empty() {
            return Err(DiagnosticDraft::unsupported(
                "lower of a malformed parameter node",
            ));
        }
        let Some(name) = body.name else {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("lower reads nameless parameter {}", parameter.index()),
            ));
        };
        if names.insert(name, *parameter).is_some() {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "lower reads duplicate parameter name",
            ));
        }
        let (symbol, body) = symbol_for_decl(bus, *parameter)?;
        if body.name != name
            || body.kind != SymbolKind::Object
            || body.storage != StorageDuration::Automatic
            || body.linkage != Linkage::None
            || body.ty != Some(int)
            || body.scope != block_scope
        {
            return Err(DiagnosticDraft::unsupported(
                "lower of a parameter outside the automatic-int function block",
            ));
        }
        symbols.push(symbol);
    }
    Ok(symbols)
}

fn expression_contains_call(
    bus: &crate::bus::CompilerBus,
    node: NodeId,
) -> Result<bool, DiagnosticDraft> {
    let body = bus.arenas.nodes.get(node).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("lower reads missing expression node {}", node.index()),
        )
    })?;
    if body.kind == NodeKind::Call {
        return Ok(true);
    }
    for child in &body.children {
        if expression_contains_call(bus, *child)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn checked_int_fact(
    bus: &crate::bus::CompilerBus,
    node: NodeId,
    int: TypeId,
) -> Result<(), DiagnosticDraft> {
    let sem = unique_sem(bus, node)?;
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

/// Select the canonical folded constant. In a single-function fixture the
/// arena holds exactly one const. Once earlier functions have contributed
/// constants, the literal/literal `BinaryAdd` fold is identified by its
/// `[lhs, rhs]` origins; a duplicate or absent match is an ambiguous handoff.
fn select_canonical_constant(
    bus: &crate::bus::CompilerBus,
    operand: NodeId,
) -> Result<(ConstId, TypeId), DiagnosticDraft> {
    let consts: Vec<(ConstId, &crate::bus::ConstRecord)> = bus.arenas.consts.iter().collect();
    if consts.is_empty() {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "lower reads missing folded constant",
        ));
    }
    let selected = if consts.len() == 1 {
        consts[0]
    } else {
        let operand_body = bus.arenas.nodes.get(operand).map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("lower reads missing expression node {}", operand.index()),
            )
        })?;
        let pair = if operand_body.kind == NodeKind::BinaryAdd && operand_body.children.len() == 2 {
            let lhs = bus
                .arenas
                .nodes
                .get(operand_body.children[0])
                .ok()
                .and_then(|body| body.literal);
            let rhs = bus
                .arenas
                .nodes
                .get(operand_body.children[1])
                .ok()
                .and_then(|body| body.literal);
            match (lhs, rhs) {
                (Some(lhs), Some(rhs)) => Some([lhs, rhs]),
                _ => None,
            }
        } else {
            None
        };
        let Some(pair) = pair else {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "ambiguous folded constants for lowering",
            ));
        };
        let mut matches = consts
            .iter()
            .filter(|(_, body)| body.origins.as_slice() == pair.as_slice());
        let Some(first) = matches.next() else {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "ambiguous folded constants for lowering",
            ));
        };
        if matches.next().is_some() {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "ambiguous folded constants for lowering",
            ));
        }
        *first
    };
    if bus.arenas.types.get(selected.1.ty).is_err() {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!(
                "lower reads constant with dangling type {}",
                selected.1.ty.index()
            ),
        ));
    }
    Ok((selected.0, selected.1.ty))
}

/// Build the literal-to-constant map from `ConstRecord.origins`. Only a
/// single-literal origin is a literal constant; binary folds (`[lhs, rhs]`)
/// are deliberately not candidates. Missing, ambiguous, and dangling foreign
/// origins fail loudly instead of guessing by value.
fn literal_constants(
    bus: &crate::bus::CompilerBus,
    literals: &[LiteralId],
    int: TypeId,
) -> Result<BTreeMap<LiteralId, ConstId>, DiagnosticDraft> {
    let used: BTreeSet<LiteralId> = literals.iter().copied().collect();
    let mut selected: BTreeMap<LiteralId, ConstId> = BTreeMap::new();
    for (id, body) in bus.arenas.consts.iter() {
        if body.origins.len() != 1 {
            continue;
        }
        let literal = body.origins[0];
        if bus.arenas.literals.get(literal).is_err() {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!(
                    "lower reads folded constant {} with foreign literal origin {}",
                    id.index(),
                    literal.index()
                ),
            ));
        }
        if !used.contains(&literal) {
            continue;
        }
        if selected.insert(literal, id).is_some() {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!(
                    "lower reads ambiguous folded constants for literal {}",
                    literal.index()
                ),
            ));
        }
    }
    for literal in literals {
        let Some(constant) = selected.get(literal).copied() else {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!(
                    "lower reads no literal-origin folded constant for literal {}",
                    literal.index()
                ),
            ));
        };
        let literal_body = bus.arenas.literals.get(*literal).map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("lower reads missing literal {}", literal.index()),
            )
        })?;
        let body = bus.arenas.consts.get(constant).map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("lower reads missing folded constant {}", constant.index()),
            )
        })?;
        let ConstValue::Integer(integer) = &body.value else {
            return Err(DiagnosticDraft::unsupported(
                "lower of a non-integer folded literal",
            ));
        };
        if integer.magnitude != literal_body.value || integer.negative != literal_body.negative {
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
    }
    Ok(selected)
}

struct LocalExprBuilder<'a> {
    bus: &'a crate::bus::CompilerBus,
    int: TypeId,
    literal_consts: &'a BTreeMap<LiteralId, ConstId>,
    known: &'a BTreeMap<NameId, SymbolId>,
    identifiers: BTreeMap<NodeId, SymbolId>,
    enclosing: NodeId,
    current_symbol: SymbolId,
}

impl LocalExprBuilder<'_> {
    fn build(&mut self, node: NodeId) -> Result<LocalExpr, DiagnosticDraft> {
        let body = self.bus.arenas.nodes.get(node).map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("lower reads missing expression node {}", node.index()),
            )
        })?;
        checked_int_fact(self.bus, node, self.int)?;
        match body.kind {
            NodeKind::IntLiteral => {
                let Some(literal) = body.literal else {
                    return Err(DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("lower reads literal-less node {}", node.index()),
                    ));
                };
                let Some(constant) = self.literal_consts.get(&literal).copied() else {
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
                let symbol = resolve_known_identifier(self.bus, node, self.known, self.int)?;
                match self.identifiers.insert(node, symbol) {
                    None => {}
                    Some(existing) if existing == symbol => {}
                    Some(_) => {
                        return Err(DiagnosticDraft::error(
                            DiagnosticCode::new(DiagGroup::Task, 4),
                            format!(
                                "lower reads conflicting identifier bindings for node {}",
                                node.index()
                            ),
                        ));
                    }
                }
                Ok(LocalExpr::Identifier { node, symbol })
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
                    lhs: Box::new(self.build(body.children[0])?),
                    rhs: Box::new(self.build(body.children[1])?),
                })
            }
            NodeKind::UnaryPlus => {
                if body.children.len() != 1 {
                    return Err(DiagnosticDraft::unsupported(
                        "lower of a malformed unary plus",
                    ));
                }
                Ok(LocalExpr::Identity(Box::new(self.build(body.children[0])?)))
            }
            NodeKind::UnaryMinus => {
                if body.children.len() != 1 {
                    return Err(DiagnosticDraft::unsupported(
                        "lower of a malformed unary minus",
                    ));
                }
                Ok(LocalExpr::Unary {
                    op: IrOp::Neg,
                    operand: Box::new(self.build(body.children[0])?),
                })
            }
            NodeKind::Call => {
                if body.children.is_empty() || body.children.len() > 9 {
                    return Err(DiagnosticDraft::unsupported(
                        "lower of a call outside the one-to-eight argument shape",
                    ));
                }
                let callee = resolve_callee(
                    self.bus,
                    self.enclosing,
                    self.current_symbol,
                    node,
                    body.children[0],
                    self.int,
                    body.children.len() - 1,
                )?;
                let mut args = Vec::with_capacity(body.children.len() - 1);
                for argument in &body.children[1..] {
                    args.push(self.build(*argument)?);
                }
                Ok(LocalExpr::Call { node, callee, args })
            }
            _ => Err(DiagnosticDraft::unsupported(
                "lower of a non-covered expression node",
            )),
        }
    }
}

fn resolve_known_identifier(
    bus: &crate::bus::CompilerBus,
    node: NodeId,
    known: &BTreeMap<NameId, SymbolId>,
    int: TypeId,
) -> Result<SymbolId, DiagnosticDraft> {
    let body = bus.arenas.nodes.get(node).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("lower reads missing identifier node {}", node.index()),
        )
    })?;
    if body.kind != NodeKind::Identifier {
        return Err(DiagnosticDraft::unsupported(
            "lower of a non-identifier expression node",
        ));
    }
    let Some(name) = body.name else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("lower reads nameless identifier {}", node.index()),
        ));
    };
    let Some(symbol) = known.get(&name).copied() else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Semantic, 2),
            format!("lower reads undeclared identifier {}", name.index()),
        ));
    };
    let symbol_body = bus.arenas.symbols.get(symbol).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("lower reads missing symbol {}", symbol.index()),
        )
    })?;
    if symbol_body.kind != SymbolKind::Object
        || symbol_body.storage != StorageDuration::Automatic
        || symbol_body.ty != Some(int)
    {
        return Err(DiagnosticDraft::unsupported(
            "lower of an identifier that does not name an automatic int object",
        ));
    }
    Ok(symbol)
}

fn resolve_callee(
    bus: &crate::bus::CompilerBus,
    enclosing: NodeId,
    current_symbol: SymbolId,
    call: NodeId,
    callee: NodeId,
    int: TypeId,
    argument_count: usize,
) -> Result<SymbolId, DiagnosticDraft> {
    let callee_body = bus.arenas.nodes.get(callee).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("lower reads missing callee node {}", callee.index()),
        )
    })?;
    if callee_body.kind != NodeKind::Identifier {
        return Err(DiagnosticDraft::unsupported(
            "lower of a call whose callee is not an identifier",
        ));
    }
    let Some(name) = callee_body.name else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("lower reads nameless callee in call {}", call.index()),
        ));
    };
    let mut candidates = Vec::new();
    for (id, symbol) in bus.arenas.symbols.iter() {
        if symbol.name != name || symbol.kind != SymbolKind::Function {
            continue;
        }
        let file_scope = bus
            .arenas
            .scopes
            .get(symbol.scope)
            .map(|scope| scope.kind == ScopeKind::File)
            .unwrap_or(false);
        if file_scope {
            candidates.push((id, symbol.clone()));
        }
    }
    if candidates.is_empty() {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Semantic, 2),
            format!("lower reads unknown callee {}", name.index()),
        ));
    }
    if candidates.len() > 1 {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("lower reads ambiguous callee {}", name.index()),
        ));
    }
    let (symbol, body) = candidates.pop().expect("one candidate");
    let mut definitions = Vec::new();
    for (id, node) in bus.arenas.nodes.iter() {
        if node.kind == NodeKind::FunctionDefinition
            && node.children.get(1) == Some(&body.decl)
        {
            definitions.push(id);
        }
    }
    if definitions.is_empty() {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Semantic, 2),
            format!("lower reads undefined callee {}", name.index()),
        ));
    }
    if definitions.len() > 1 {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("lower reads duplicate callee definition {}", name.index()),
        ));
    }
    let definition = definitions[0];
    if symbol == current_symbol || definition == enclosing {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Semantic, 2),
            format!("lower reads self-recursive callee {}", name.index()),
        ));
    }
    if definition.index() >= enclosing.index() {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Semantic, 2),
            format!("lower reads forward callee {}", name.index()),
        ));
    }
    let Some(signature) = body.ty else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("lower reads untyped callee {}", name.index()),
        ));
    };
    let signature_body = bus.arenas.types.get(signature).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("lower reads missing callee signature {}", signature.index()),
        )
    })?;
    let arity = match &signature_body.kind {
        TypeKind::Function {
            result,
            params,
            prototype,
            variadic,
        } if *result == int
            && *prototype
            && !*variadic
            && params.len() <= 8
            && params.iter().all(|param| *param == int) =>
        {
            params.len()
        }
        _ => {
            return Err(DiagnosticDraft::unsupported(
                "lower of a callee signature outside the int-parameter subset",
            ));
        }
    };
    if arity != argument_count {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Semantic, 2),
            format!(
                "lower reads arity mismatch for callee {}: {} arguments for {} parameters",
                name.index(),
                argument_count,
                arity
            ),
        ));
    }
    Ok(symbol)
}

fn lower_local_expression(
    expression: &LocalExpr,
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
        LocalExpr::Identifier { symbol, .. } => {
            let result = ValueId::from_index(value_base + *next_value);
            *next_value += 1;
            instructions.push(InstructionRecord {
                op: IrOp::LocalLoad,
                block,
                operands: vec![],
                immediate: None,
                result: Some(result),
                local: Some(*symbol),
                successors: vec![],
                callee: None,
            });
            result
        }
        LocalExpr::Binary { op, lhs, rhs } => {
            let lhs = lower_local_expression(lhs, block, value_base, next_value, instructions);
            let rhs = lower_local_expression(rhs, block, value_base, next_value, instructions);
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
                lower_local_expression(operand, block, value_base, next_value, instructions);
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
            lower_local_expression(operand, block, value_base, next_value, instructions)
        }
        LocalExpr::Call { callee, args, .. } => {
            let mut operands = Vec::with_capacity(args.len());
            for argument in args {
                operands.push(lower_local_expression(
                    argument,
                    block,
                    value_base,
                    next_value,
                    instructions,
                ));
            }
            let result = ValueId::from_index(value_base + *next_value);
            *next_value += 1;
            instructions.push(InstructionRecord {
                op: IrOp::Call,
                block,
                operands,
                immediate: None,
                result: Some(result),
                local: None,
                successors: vec![],
                callee: Some(*callee),
            });
            result
        }
        LocalExpr::Program { .. } => {
            unreachable!("program envelopes are not nested expressions")
        }
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
                FieldPath::new(StoreId::Symbols, "scopes"),
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
            tests: vec!["compiler/tests/c15_ir.rs", "compiler/tests/call_ir.rs"],
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
    /// and no arithmetic: every `Constant` carries a committed `ConstId`
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
        // the committed operand fact type, and the `Return` operand fact must
        // carry the same type.
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
        // `/42` `ir.op-immediate-type`: the selected constant's committed
        // type must equal the `Constant` result value's type.
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
        if matches!(input.local_expr, Some(LocalExpr::Program { .. })) {
            return self.compute_program(input);
        }
        if input.local.is_some() {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "lower reads a local without a program envelope",
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

    /// General `/54` lowering: optional `Constant` + `LocalStore` for the
    /// initializer, assignments in source order, then the return expression in
    /// post-order. Parameter values are bound implicitly and only loaded via
    /// their object symbols.
    fn compute_program(&self, input: &IrFunctionInput) -> Vec<Proposal> {
        let Some(LocalExpr::Program {
            parameters,
            local,
            initializer,
            assignments,
            value,
            ..
        }) = input.local_expr.as_ref()
        else {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "lower reads no program expression",
                ),
            )];
        };
        if local.is_none() && initializer.is_some() {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "lower reads an initializer without a local",
                ),
            )];
        }
        if local.is_none() && !assignments.is_empty() {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "lower reads an assignment without a local",
                ),
            )];
        }
        let function = FunctionId::from_index(input.functions_allocated);
        let block = BlockId::from_index(input.blocks_allocated);
        let linkage = input.linkage;
        let mut next_value = 0u32;
        let mut instructions = Vec::new();
        if let (Some(local), Some(initializer)) = (local, initializer) {
            let initial = ValueId::from_index(input.values_allocated);
            next_value += 1;
            instructions.push(InstructionRecord {
                op: IrOp::Constant,
                block,
                operands: vec![],
                immediate: Some(*initializer),
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
                local: Some(*local),
                successors: vec![],
                callee: None,
            });
        }
        for assignment in assignments {
            let assigned = lower_local_expression(
                assignment,
                block,
                input.values_allocated,
                &mut next_value,
                &mut instructions,
            );
            let local = local.expect("checked above");
            instructions.push(InstructionRecord {
                op: IrOp::LocalStore,
                block,
                operands: vec![assigned],
                immediate: None,
                result: None,
                local: Some(local),
                successors: vec![],
                callee: None,
            });
        }
        let returned = lower_local_expression(
            value,
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
            parameters: parameters.clone(),
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
