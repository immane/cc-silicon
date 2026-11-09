// ============================================================================
// chips/verify/vf_syntax.rs — T13 VF05 token-AST invariant verifier
// (Wave 2 slice 9, `/18`; `/55` direct-call shapes)
//
// Reads one committed TU node plus the committed token and literal arenas
// and checks the syntax contract: parent/children reciprocity, token ranges
// contained in the parent range with ordered non-overlapping siblings,
// per-kind child counts, required fields (`Declarator`/`Identifier`/
// `Parameter` name, `IntLiteral` literal with its origin token), referenced
// tokens committed, and a unique trailing EOF. The `/55` table adds multiple
// function definitions, function parameter lists, parameters, and direct
// calls while retaining the canonical and `/49`–`/53` shapes. Function
// definition, parameter-list, call-argument, and TU-node budgets are closed
// implementation bounds. Completes `Ack`; any gap fails loudly.
// ============================================================================

use crate::bus::{LiteralRecord, NodeKind, NodeRecord, TokenKind, TokenRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{LiteralId, NodeId, RecordRef, TaskId, TokenId};
use crate::limits::{
    CALL_MAX_ARGUMENTS, CALL_MAX_AST_NODES, CALL_MAX_FUNCTIONS, CALL_MAX_PARAMETERS,
};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, VF05_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};
use std::collections::BTreeSet;

/// Narrow projection for the syntax-invariant check.
#[derive(Clone, Debug)]
pub struct Vf05Input {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// TU root.
    pub tu: NodeId,
    /// Reachable nodes in pre-order (committed bodies).
    pub tree: Vec<(NodeId, NodeRecord)>,
    /// Committed tokens in ascending-ID order.
    pub tokens: Vec<(TokenId, TokenRecord)>,
    /// Committed literals in ascending-ID order.
    pub literals: Vec<(LiteralId, LiteralRecord)>,
}

/// Build the narrow projection for one dispatched task.
pub fn project_vf05_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<Vf05Input, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("vf05 of unknown task {}", task.index())))?;
    if record.kind != TaskKind::VERIFICATION_TOKEN_AST_INVARIANT {
        return Err(protocol_fault(format!(
            "vf05 task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "vf05 task {} payload must carry exactly one TU node",
            task.index()
        )));
    }
    let tu = match record.payload.refs[0] {
        RecordRef::Node(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "vf05 task {} payload must be a node",
                task.index()
            )));
        }
    };
    let tu_body = bus.arenas.nodes.get(tu).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("vf05 reads missing TU node {}", tu.index()),
        )
    })?;
    if tu_body.kind != NodeKind::TranslationUnit {
        return Err(DiagnosticDraft::unsupported("vf05 of a non-TU root"));
    }
    if tu_body.parent.is_some() {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("vf05 TU node {} has a parent", tu.index()),
        ));
    }
    // Pre-order walk over committed children (deterministic). Dangling
    // child links fail here; shape rules fail in `compute`.
    let mut tree = Vec::new();
    let mut stack = vec![tu];
    let mut seen = BTreeSet::new();
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("vf05 reads a cyclic or shared AST node {}", id.index()),
            ));
        }
        let body = bus.arenas.nodes.get(id).map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("vf05 reads missing node {}", id.index()),
            )
        })?;
        for child in body.children.iter().rev() {
            stack.push(*child);
        }
        tree.push((id, body.clone()));
    }
    let mut tokens: Vec<(TokenId, TokenRecord)> = bus
        .arenas
        .tokens
        .iter()
        .map(|(id, body)| (id, body.clone()))
        .collect();
    tokens.sort_by_key(|(id, _)| id.index());
    let mut literals: Vec<(LiteralId, LiteralRecord)> = bus
        .arenas
        .literals
        .iter()
        .map(|(id, body)| (id, body.clone()))
        .collect();
    literals.sort_by_key(|(id, _)| id.index());
    Ok(Vf05Input {
        task,
        state: record.state.clone(),
        tu,
        tree,
        tokens,
        literals,
    })
}

/// Find a committed token body by ID (deterministic linear scan).
fn find_token(tokens: &[(TokenId, TokenRecord)], id: TokenId) -> Option<&(TokenId, TokenRecord)> {
    tokens.iter().find(|(tid, _)| *tid == id)
}

/// Build the loud failure for a syntax-contract gap (never a pass).
fn syntax_fail(task: TaskId, detail: &str) -> Proposal {
    fail(
        task,
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("vf05: broken token-AST contract ({detail})"),
        ),
    )
}

/// Look up one listed node kind.
fn listed_kind(tree: &[(NodeId, NodeRecord)], id: NodeId) -> Option<NodeKind> {
    tree.iter()
        .find(|(nid, _)| *nid == id)
        .map(|(_, body)| body.kind)
}

/// Kinds allowed as a standalone expression child in the `/55` subset.
fn is_expression_kind(kind: NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::IntLiteral
            | NodeKind::Identifier
            | NodeKind::Call
            | NodeKind::BinaryAdd
            | NodeKind::BinarySub
            | NodeKind::BinaryMul
            | NodeKind::BinaryDiv
            | NodeKind::BinaryRem
            | NodeKind::UnaryPlus
            | NodeKind::UnaryMinus
    )
}

/// Check the closed `/55` child-shape table for one node.
fn check_shape(
    tree: &[(NodeId, NodeRecord)],
    body: &NodeRecord,
    expected_parent: Option<NodeId>,
) -> Result<(), &'static str> {
    let children = &body.children;
    let child_kinds = || -> Option<Vec<NodeKind>> {
        children
            .iter()
            .map(|child| listed_kind(tree, *child))
            .collect()
    };
    match body.kind {
        NodeKind::TranslationUnit => {
            if !(1..=CALL_MAX_FUNCTIONS).contains(&children.len()) {
                return Err("translation unit function count is outside the budget");
            }
            if child_kinds() != Some(vec![NodeKind::FunctionDefinition; children.len()]) {
                return Err("translation unit children are not function definitions");
            }
        }
        NodeKind::FunctionDefinition => {
            if child_kinds()
                != Some(vec![
                    NodeKind::Specifiers,
                    NodeKind::Declarator,
                    NodeKind::Compound,
                ])
            {
                return Err("function definition shape is not covered");
            }
            let declarator = &tree
                .iter()
                .find(|(nid, _)| Some(*nid) == children.get(1).copied())
                .ok_or("function declarator is not listed")?
                .1;
            if declarator.children.len() > 1 {
                return Err("function declarator has extra children");
            }
            if let Some(parameter_list) = declarator.children.first() {
                if listed_kind(tree, *parameter_list) != Some(NodeKind::ParameterList) {
                    return Err("function declarator child is not a parameter list");
                }
            }
        }
        NodeKind::Specifiers => {
            if !children.is_empty() {
                return Err("specifiers carry children");
            }
        }
        NodeKind::Declarator => {
            if children.len() > 1 {
                return Err("declarator has extra children");
            }
            if let Some(parameter_list) = children.first() {
                if listed_kind(tree, *parameter_list) != Some(NodeKind::ParameterList) {
                    return Err("declarator child is not a parameter list");
                }
                let parent_kind = expected_parent.and_then(|parent| listed_kind(tree, parent));
                if parent_kind != Some(NodeKind::FunctionDefinition) {
                    return Err("parameter list is not on a function declarator");
                }
            }
        }
        NodeKind::Compound => {
            let Some(kinds) = child_kinds() else {
                return Err("compound child is not listed");
            };
            let ok = match kinds.as_slice() {
                [NodeKind::Return] => true,
                [NodeKind::Declaration, rest @ ..] => {
                    !rest.is_empty()
                        && rest.last() == Some(&NodeKind::Return)
                        && rest[..rest.len() - 1]
                            .iter()
                            .all(|kind| *kind == NodeKind::Assignment)
                }
                _ => false,
            };
            if !ok {
                return Err("compound statement shape is not covered");
            }
        }
        NodeKind::Return => {
            if children.len() != 1
                || child_kinds().is_none_or(|kinds| !is_expression_kind(kinds[0]))
            {
                return Err("return does not carry one expression");
            }
        }
        NodeKind::BinaryAdd
        | NodeKind::BinarySub
        | NodeKind::BinaryMul
        | NodeKind::BinaryDiv
        | NodeKind::BinaryRem => {
            let Some(kinds) = child_kinds() else {
                return Err("binary child is not listed");
            };
            if kinds.len() != 2 || !kinds.iter().all(|kind| is_expression_kind(*kind)) {
                return Err("binary children are not two expressions");
            }
        }
        NodeKind::UnaryPlus | NodeKind::UnaryMinus => {
            if children.len() != 1
                || child_kinds().is_none_or(|kinds| !is_expression_kind(kinds[0]))
            {
                return Err("unary does not carry one expression");
            }
        }
        NodeKind::Assignment => {
            let Some(kinds) = child_kinds() else {
                return Err("assignment child is not listed");
            };
            if kinds.first() != Some(&NodeKind::Identifier)
                || !kinds.get(1).is_some_and(|kind| is_expression_kind(*kind))
            {
                return Err("assignment child shape is not covered");
            }
        }
        NodeKind::IntLiteral => {
            if !children.is_empty() {
                return Err("literal leaf carries children");
            }
        }
        NodeKind::Declaration => {
            if child_kinds()
                != Some(vec![
                    NodeKind::Specifiers,
                    NodeKind::Declarator,
                    NodeKind::IntLiteral,
                ])
            {
                return Err("declaration child shape is not covered");
            }
        }
        NodeKind::Identifier => {
            if !children.is_empty() {
                return Err("identifier leaf carries children");
            }
        }
        NodeKind::ParameterList => {
            if children.is_empty() || children.len() > CALL_MAX_PARAMETERS {
                return Err("parameter list count is outside the budget");
            }
            if child_kinds() != Some(vec![NodeKind::Parameter; children.len()]) {
                return Err("parameter list children are not parameters");
            }
        }
        NodeKind::Parameter => {
            if !children.is_empty() {
                return Err("parameter leaf carries children");
            }
        }
        NodeKind::Call => {
            if children.len() > CALL_MAX_ARGUMENTS + 1 {
                return Err("call argument count is outside the budget");
            }
            let Some(kinds) = child_kinds() else {
                return Err("call child is not listed");
            };
            if kinds.first() != Some(&NodeKind::Identifier)
                || !kinds[1..].iter().all(|kind| is_expression_kind(*kind))
            {
                return Err("call child shape is not covered");
            }
        }
        NodeKind::ExpressionStatement => {
            return Err("V1 AST kind is not covered by the VF05 shape table");
        }
    }
    Ok(())
}

/// The VF05 token-AST invariant verifier (M1 and `/55` call scope).
pub struct Vf05Chip;

impl Worker for Vf05Chip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: VF05_CHIP,
            chip_name: "Vf05Chip",
            group: TaskGroup::VERIFICATION,
            task_kinds: vec![TaskKind::VERIFICATION_TOKEN_AST_INVARIANT],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Parse, "nodes"),
                FieldPath::new(StoreId::Lex, "tokens"),
                FieldPath::new(StoreId::Lex, "literals"),
            ],
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c18_vf05.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_vf05_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl Vf05Chip {
    /// Pure syntax-contract check over the narrow projection (no bus
    /// access, no store writes).
    pub fn compute(&self, input: &Vf05Input) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("vf05 task {} is not running", input.task.index()),
                ),
            )];
        }
        // `/55` total-node budget: the boundary is accepted, the next value
        // fails before any shape walk or write.
        if input.tree.len() > CALL_MAX_AST_NODES {
            return vec![syntax_fail(
                input.task,
                "translation unit exceeds the node budget",
            )];
        }
        // Unique trailing EOF over the committed token arena.
        let mut eof = None;
        for (id, body) in &input.tokens {
            if body.kind == TokenKind::Eof {
                if eof.is_some() {
                    return vec![syntax_fail(input.task, "more than one EOF token")];
                }
                eof = Some(*id);
            }
        }
        let Some(eof) = eof else {
            return vec![syntax_fail(input.task, "missing EOF token")];
        };
        for (id, _) in &input.tokens {
            if id.index() > eof.index() {
                return vec![syntax_fail(input.task, "token past EOF")];
            }
        }
        // Re-walk from the TU root so parent links, ranges, and sibling
        // order are all re-verified, never trusted from the walk order.
        // Stack entries carry the expected parent plus its token range.
        let mut stack = vec![(input.tu, None, 0u32, eof.index())];
        while let Some((id, expected_parent, parent_first, parent_last)) = stack.pop() {
            let Some((_, body)) = input.tree.iter().find(|(nid, _)| *nid == id) else {
                return vec![syntax_fail(input.task, "walk reaches an unlisted node")];
            };
            if body.parent != expected_parent {
                return vec![syntax_fail(input.task, "parent link is not reciprocal")];
            };
            if let Err(detail) = check_shape(&input.tree, body, expected_parent) {
                return vec![syntax_fail(input.task, detail)];
            }
            let needs_name = matches!(
                body.kind,
                NodeKind::Declarator | NodeKind::Identifier | NodeKind::Parameter
            );
            let needs_literal = body.kind == NodeKind::IntLiteral;
            if body.name.is_some() != needs_name {
                return vec![syntax_fail(input.task, "bad declarator name presence")];
            }
            if body.literal.is_some() != needs_literal {
                return vec![syntax_fail(input.task, "bad literal presence")];
            }
            if find_token(&input.tokens, body.first_token).is_none() {
                return vec![syntax_fail(input.task, "first token is not committed")];
            }
            if find_token(&input.tokens, body.last_token).is_none() {
                return vec![syntax_fail(input.task, "last token is not committed")];
            }
            if body.first_token.index() > body.last_token.index() {
                return vec![syntax_fail(input.task, "empty token range")];
            }
            if body.first_token.index() < parent_first || body.last_token.index() > parent_last {
                return vec![syntax_fail(input.task, "range escapes the parent range")];
            }
            if body.last_token.index() > eof.index() {
                return vec![syntax_fail(input.task, "range escapes EOF")];
            }
            // Sibling ranges are ordered and non-overlapping.
            let mut previous_last = None;
            for child in &body.children {
                let Some((_, child_body)) = input.tree.iter().find(|(nid, _)| *nid == *child)
                else {
                    return vec![syntax_fail(input.task, "child is not a listed node")];
                };
                if let Some(previous) = previous_last {
                    if child_body.first_token.index() <= previous {
                        return vec![syntax_fail(input.task, "sibling ranges overlap")];
                    }
                }
                previous_last = Some(child_body.last_token.index());
            }
            // IntLiteral leaves are single-token with a committed origin
            // literal whose origin token is the leaf token.
            if body.kind == NodeKind::IntLiteral {
                if body.first_token != body.last_token {
                    return vec![syntax_fail(input.task, "literal leaf spans tokens")];
                }
                let literal_id = body.literal.expect("literal presence checked above");
                let Some((_, literal)) = input.literals.iter().find(|(lid, _)| *lid == literal_id)
                else {
                    return vec![syntax_fail(input.task, "literal is not committed")];
                };
                match literal.token {
                    Some(origin) if origin == body.first_token => {}
                    _ => {
                        return vec![syntax_fail(
                            input.task,
                            "literal origin is not the leaf token",
                        )];
                    }
                }
            }
            for child in body.children.iter().rev() {
                stack.push((
                    *child,
                    Some(id),
                    body.first_token.index(),
                    body.last_token.index(),
                ));
            }
        }
        vec![Proposal::Complete {
            task: input.task,
            value: ResultValue::Ack,
        }]
    }
}
