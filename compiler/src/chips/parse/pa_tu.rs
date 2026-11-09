// ============================================================================
// chips/pa_tu.rs — T05 PA01 TU-parse worker (Wave 2 slice 3, `/12`)
//
// Reads 13 committed C tokens in source order, validates the exact M1 shape
// (`int main(void){return 2+3;}` plus EOF) including every keyword,
// identifier, punctuator, and EOF spelling, appends the nine-node M1 tree in
// pre-order, and completes with the TU root reference. Any other token
// sequence is explicit `Unsupported`: the full parser stays deferred. Token
// ranges, the declarator name, and the literal leaves all reference
// committed records; same-batch node IDs are pre-order predictions the
// commit verifies loudly. The File-Enter edge stays deferred to the T06
// slice: this worker commits the TU root first (OPEN-01 direction) but emits
// no scope edge.
// ============================================================================

use crate::bus::{NodeKind, NodeRecord, TokenKind};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{LiteralId, NameId, RecordRef, TaskId, TokenId};
use crate::limits::{
    CALL_MAX_ARGUMENTS, CALL_MAX_AST_NODES, CALL_MAX_EXPRESSION_DEPTH, CALL_MAX_FUNCTIONS,
    CALL_MAX_PARAMETERS,
};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PA_TU_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};
use std::collections::BTreeMap;

/// One projected token: kind, interned name, and committed surface spelling.
#[derive(Clone, Debug)]
pub struct ProjectedToken {
    /// C-token kind.
    pub kind: TokenKind,
    /// Interned name, if any.
    pub name: Option<NameId>,
    /// Spelling bytes for named tokens (interned spelling).
    pub spelling: Vec<u8>,
    /// Committed PP spelling (surface spelling for punctuators and EOF).
    pub pp_spelling: Vec<u8>,
}

/// Narrow projection for the TU-parse computation.
#[derive(Clone, Debug)]
pub struct PaTuInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token IDs in source order.
    pub tokens: Vec<TokenId>,
    /// Present tokens by ID.
    pub bodies: BTreeMap<TokenId, ProjectedToken>,
    /// Committed literal by token ID (exactly one per integer token).
    pub literals_by_token: BTreeMap<TokenId, LiteralId>,
    /// `nodes` arena count at dispatch (pre-order prediction base).
    pub nodes_allocated: u32,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pa_tu_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaTuInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("parse of unknown task {}", task.index())))?;
    if record.kind != TaskKind::PARSE_TU {
        return Err(protocol_fault(format!(
            "parse task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    let mut tokens = Vec::new();
    let mut bodies = BTreeMap::new();
    for reference in &record.payload.refs {
        let RecordRef::Token(id) = reference else {
            return Err(protocol_fault(format!(
                "parse task {} payload must carry tokens only",
                task.index()
            )));
        };
        tokens.push(*id);
        if let Ok(body) = bus.arenas.tokens.get(*id) {
            let spelling = body
                .name
                .and_then(|name| bus.intern.get(name).ok().map(|bytes| bytes.to_vec()))
                .unwrap_or_default();
            let pp_spelling = bus
                .arenas
                .pp_tokens
                .get(body.pp_token)
                .map(|pp| pp.spelling.clone())
                .unwrap_or_default();
            bodies.insert(
                *id,
                ProjectedToken {
                    kind: body.kind,
                    name: body.name,
                    spelling,
                    pp_spelling,
                },
            );
        }
    }
    // Committed literals indexed by their token back-link (ID order).
    let mut literals_by_token = BTreeMap::new();
    for (id, literal) in bus.arenas.literals.iter() {
        if let Some(token) = literal.token {
            literals_by_token.insert(token, id);
        }
    }
    Ok(PaTuInput {
        task,
        state: record.state.clone(),
        tokens,
        bodies,
        literals_by_token,
        nodes_allocated: bus.arenas.nodes.allocated(),
    })
}

/// The T05 TU-parse worker (M1 fixed-shape slice scope).
pub struct PaTuChip;

impl Worker for PaTuChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PA_TU_CHIP,
            chip_name: "PaTuChip",
            group: TaskGroup::PARSE,
            task_kinds: vec![TaskKind::PARSE_TU],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Lex, "tokens"),
                FieldPath::new(StoreId::Lex, "literals"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Names, "entries"),
                FieldPath::new(StoreId::Parse, "nodes"),
            ],
            writes: vec![FieldPath::new(StoreId::Parse, "nodes")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec![
                "compiler/tests/c12_parse.rs",
                "compiler/tests/c40_pa_guard.rs",
                "compiler/tests/call_parse.rs",
            ],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pa_tu_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

/// Expected M1 token kinds in source order.
const M1_KINDS: &[TokenKind] = &[
    TokenKind::Keyword,
    TokenKind::Identifier,
    TokenKind::Punctuator,
    TokenKind::Keyword,
    TokenKind::Punctuator,
    TokenKind::Punctuator,
    TokenKind::Keyword,
    TokenKind::Integer,
    TokenKind::Punctuator,
    TokenKind::Integer,
    TokenKind::Punctuator,
    TokenKind::Punctuator,
    TokenKind::Eof,
];

/// Expected M1 punctuator spellings at their fixed token positions.
const M1_PUNCTUATORS: &[(usize, &[u8])] = &[
    (2, b"("),
    (4, b")"),
    (5, b"{"),
    (8, b"+"),
    (10, b";"),
    (11, b"}"),
];

/// `/52`/`/53` source-locals prefix token kinds:
/// `int main ( void ) { int x = <int> ; <statements> } EOF`.
const M1_LOCALS_PREFIX_KINDS: &[TokenKind] = &[
    TokenKind::Keyword,
    TokenKind::Identifier,
    TokenKind::Punctuator,
    TokenKind::Keyword,
    TokenKind::Punctuator,
    TokenKind::Punctuator,
    TokenKind::Keyword,
    TokenKind::Identifier,
    TokenKind::Punctuator,
    TokenKind::Integer,
    TokenKind::Punctuator,
];

/// `/52`/`/53` keyword/identifier spellings at fixed prefix positions.
const M1_LOCALS_PREFIX_WORDS: &[(&[u8], usize)] = &[
    (b"int".as_slice(), 0),
    (b"main".as_slice(), 1),
    (b"void".as_slice(), 3),
    (b"int".as_slice(), 6),
];

/// `/52` punctuator spellings at fixed prefix positions.
const M1_LOCALS_PREFIX_PUNCTUATORS: &[(usize, &[u8])] =
    &[(2, b"("), (4, b")"), (5, b"{"), (8, b"="), (10, b";")];

/// Whether the token stream is exactly the legacy canonical M1 shape.
fn is_legacy_canonical(input: &PaTuInput) -> bool {
    if input.tokens.len() != M1_KINDS.len() {
        return false;
    }
    for (position, id) in input.tokens.iter().enumerate() {
        let Some(projected) = input.bodies.get(id) else {
            return false;
        };
        if projected.kind != M1_KINDS[position] {
            return false;
        }
    }
    for (spelling, position) in [
        (b"int".as_slice(), 0usize),
        (b"main".as_slice(), 1),
        (b"void".as_slice(), 3),
        (b"return".as_slice(), 6),
    ] {
        if input.bodies[&input.tokens[position]].spelling != spelling {
            return false;
        }
    }
    for (position, spelling) in M1_PUNCTUATORS {
        if input.bodies[&input.tokens[*position]].pp_spelling != *spelling {
            return false;
        }
    }
    let eof = &input.bodies[&input.tokens[12]];
    eof.pp_spelling.is_empty() && eof.name.is_none()
}

/// Whether the stream is one of the legacy `/52`/`/53` single-local forms.
/// Calls and non-local identifiers belong to the general parser.
fn is_legacy_locals(input: &PaTuInput) -> bool {
    if input.tokens.len() < 16 {
        return false;
    }
    for (position, expected) in M1_LOCALS_PREFIX_KINDS.iter().enumerate() {
        let Some(projected) = input.bodies.get(&input.tokens[position]) else {
            return false;
        };
        if projected.kind != *expected {
            return false;
        }
    }
    for (spelling, position) in M1_LOCALS_PREFIX_WORDS {
        if input.bodies[&input.tokens[*position]].spelling != *spelling {
            return false;
        }
    }
    for (position, spelling) in M1_LOCALS_PREFIX_PUNCTUATORS {
        if input.bodies[&input.tokens[*position]].pp_spelling != *spelling {
            return false;
        }
    }
    let Some(local_name) = input.bodies[&input.tokens[7]].name else {
        return false;
    };
    for (position, id) in input.tokens.iter().enumerate().skip(11) {
        let Some(projected) = input.bodies.get(id) else {
            return false;
        };
        if projected.kind == TokenKind::Eof {
            return true;
        }
        if projected.kind != TokenKind::Identifier {
            continue;
        }
        if projected.name != Some(local_name) {
            return false;
        }
        if input
            .tokens
            .get(position + 1)
            .and_then(|next| input.bodies.get(next))
            .is_some_and(|next| next.kind == TokenKind::Punctuator && next.pp_spelling == b"(")
        {
            return false;
        }
    }
    true
}

/// One parsed local expression node. IDs are assigned after the parse.
struct ExprNode {
    kind: NodeKind,
    first: usize,
    last: usize,
    name: Option<NameId>,
    literal: Option<LiteralId>,
    children: Vec<ExprNode>,
}

/// One pre-order node row: kind, parent, children, token range, name, literal.
type NodeSpec = (
    NodeKind,
    Option<crate::ids::NodeId>,
    Vec<crate::ids::NodeId>,
    usize,
    usize,
    Option<NameId>,
    Option<LiteralId>,
);

impl PaTuChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &PaTuInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("parse task {} is not running", input.task.index()),
                ),
            )];
        }
        // The legacy paths keep their exact node order and spans. The
        // classifier is final: a legacy shape that fails is never retried by
        // the general parser.
        if is_legacy_canonical(input) {
            return self.compute_canonical(input);
        }
        if is_legacy_locals(input) {
            return self.compute_locals(input);
        }
        self.compute_general(input)
    }

    /// Canonical M1 shape (`int main(void){return 2+3;}` plus EOF).
    fn compute_canonical(&self, input: &PaTuInput) -> Vec<Proposal> {
        if input.tokens.len() != M1_KINDS.len() {
            return vec![fail(
                input.task,
                DiagnosticDraft::unsupported(
                    "non-M1 token sequence: exactly 13 M1 tokens required",
                ),
            )];
        }
        // Fixed-shape gate: kinds in order, keyword/identifier spellings exact.
        let words: &[(&[u8], usize)] = &[
            (b"int".as_slice(), 0),
            (b"main".as_slice(), 1),
            (b"void".as_slice(), 3),
            (b"return".as_slice(), 6),
        ];
        for (position, id) in input.tokens.iter().enumerate() {
            let Some(projected) = input.bodies.get(id) else {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("parse reads missing token {}", id.index()),
                    ),
                )];
            };
            if projected.kind != M1_KINDS[position] {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::unsupported("non-M1 token shape at fixed position"),
                )];
            }
        }
        for (spelling, position) in words {
            let id = input.tokens[*position];
            if input.bodies[&id].spelling != *spelling {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::unsupported("non-M1 keyword/identifier spelling"),
                )];
            }
        }
        // Fixed-shape gate: punctuator spellings and the EOF token shape.
        for (position, spelling) in M1_PUNCTUATORS {
            if input.bodies[&input.tokens[*position]].pp_spelling != *spelling {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::unsupported("non-M1 punctuator spelling"),
                )];
            }
        }
        let eof = &input.bodies[&input.tokens[12]];
        if !eof.pp_spelling.is_empty() || eof.name.is_some() {
            return vec![fail(
                input.task,
                DiagnosticDraft::unsupported("non-M1 EOF token shape"),
            )];
        }
        // Integer leaves need exactly one committed literal each.
        let mut leaves = Vec::new();
        for position in [7, 9] {
            let id = input.tokens[position];
            let Some(literal) = input.literals_by_token.get(&id) else {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("parse reads missing literal for token {}", id.index()),
                    ),
                )];
            };
            leaves.push(*literal);
        }
        let declarator_name = input.bodies[&input.tokens[1]].name;
        // Pre-order node allocation: TU(0) funcdef(1) spec(2) decl(3)
        // compound(4) return(5) binary(6) lit2(7) lit3(8).
        let base = input.nodes_allocated;
        let node = |index: u32| crate::ids::NodeId::from_index(base.saturating_add(index));
        let specs: Vec<NodeSpec> = vec![
            (
                NodeKind::TranslationUnit,
                None,
                vec![node(1)],
                0,
                12,
                None,
                None,
            ),
            (
                NodeKind::FunctionDefinition,
                Some(node(0)),
                vec![node(2), node(3), node(4)],
                0,
                11,
                None,
                None,
            ),
            (
                NodeKind::Specifiers,
                Some(node(1)),
                vec![],
                0,
                0,
                None,
                None,
            ),
            (
                NodeKind::Declarator,
                Some(node(1)),
                vec![],
                1,
                4,
                declarator_name,
                None,
            ),
            (
                NodeKind::Compound,
                Some(node(1)),
                vec![node(5)],
                5,
                11,
                None,
                None,
            ),
            (
                NodeKind::Return,
                Some(node(4)),
                vec![node(6)],
                6,
                10,
                None,
                None,
            ),
            (
                NodeKind::BinaryAdd,
                Some(node(5)),
                vec![node(7), node(8)],
                7,
                9,
                None,
                None,
            ),
            (
                NodeKind::IntLiteral,
                Some(node(6)),
                vec![],
                7,
                7,
                None,
                Some(leaves[0]),
            ),
            (
                NodeKind::IntLiteral,
                Some(node(6)),
                vec![],
                9,
                9,
                None,
                Some(leaves[1]),
            ),
        ];
        append_nodes(input, specs)
    }

    /// `/52`/`/53` source-locals shape. The fixed prefix is
    /// `int main(void){int x=<int>;`, followed by zero or more assignments
    /// `x=<expr>;`, then `return <expr>; } EOF`. Parsing happens before any
    /// append; failures leave the bus untouched.
    fn compute_locals(&self, input: &PaTuInput) -> Vec<Proposal> {
        if input.tokens.len() < 16 {
            return vec![fail(
                input.task,
                DiagnosticDraft::unsupported("locals token sequence is too short"),
            )];
        }
        for (position, expected) in M1_LOCALS_PREFIX_KINDS.iter().enumerate() {
            let id = input.tokens[position];
            let Some(projected) = input.bodies.get(&id) else {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("parse reads missing token {}", id.index()),
                    ),
                )];
            };
            if projected.kind != *expected {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::unsupported("non-locals token shape at fixed prefix"),
                )];
            }
        }
        for (spelling, position) in M1_LOCALS_PREFIX_WORDS {
            if input.bodies[&input.tokens[*position]].spelling != *spelling {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::unsupported("non-locals keyword/identifier spelling"),
                )];
            }
        }
        for (position, spelling) in M1_LOCALS_PREFIX_PUNCTUATORS {
            if input.bodies[&input.tokens[*position]].pp_spelling != *spelling {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::unsupported("non-locals punctuator spelling"),
                )];
            }
        }
        let Some(decl_name) = input.bodies[&input.tokens[7]].name else {
            return vec![fail(
                input.task,
                DiagnosticDraft::unsupported("locals declaration has no identifier name"),
            )];
        };
        let initializer_token = input.tokens[9];
        let Some(initializer) = input.literals_by_token.get(&initializer_token) else {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "parse reads missing literal for token {}",
                        initializer_token.index()
                    ),
                ),
            )];
        };
        let main_name = input.bodies[&input.tokens[1]].name;
        let mut parser = ExprParser {
            input,
            pos: 11,
            local_name: Some(decl_name),
        };
        let mut assignments = Vec::new();
        let (return_index, return_expression, semicolon, brace, eof) = loop {
            let is_return = {
                let body = match parser.body(parser.pos) {
                    Ok(body) => body,
                    Err(diagnostic) => return vec![fail(input.task, diagnostic)],
                };
                body.kind == TokenKind::Keyword && body.spelling == b"return"
            };
            if is_return {
                let return_index = parser.pos;
                parser.pos += 1;
                let expression = match parser.parse_expression() {
                    Ok(expression) => expression,
                    Err(diagnostic) => return vec![fail(input.task, diagnostic)],
                };
                let semicolon = match parser.expect_punct(b";") {
                    Ok(index) => index,
                    Err(diagnostic) => return vec![fail(input.task, diagnostic)],
                };
                let brace = match parser.expect_punct(b"}") {
                    Ok(index) => index,
                    Err(diagnostic) => return vec![fail(input.task, diagnostic)],
                };
                let eof = parser.pos;
                let eof_body = match parser.body(eof) {
                    Ok(body) => body,
                    Err(diagnostic) => return vec![fail(input.task, diagnostic)],
                };
                if eof_body.kind != TokenKind::Eof
                    || !eof_body.pp_spelling.is_empty()
                    || eof_body.name.is_some()
                {
                    return vec![fail(
                        input.task,
                        DiagnosticDraft::unsupported("missing trailing EOF"),
                    )];
                }
                parser.pos += 1;
                if parser.pos != input.tokens.len() {
                    return vec![fail(
                        input.task,
                        DiagnosticDraft::unsupported("tokens after the function body"),
                    )];
                }
                break (return_index, expression, semicolon, brace, eof);
            }
            let lhs_name = {
                let body = match parser.body(parser.pos) {
                    Ok(body) => body,
                    Err(diagnostic) => return vec![fail(input.task, diagnostic)],
                };
                if body.kind != TokenKind::Identifier {
                    return vec![fail(
                        input.task,
                        DiagnosticDraft::unsupported(
                            "locals statement is neither assignment nor return",
                        ),
                    )];
                }
                body.name
            };
            if lhs_name != Some(decl_name) {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::unsupported(
                        "locals assignment names an identifier other than the local",
                    ),
                )];
            }
            let lhs_index = parser.pos;
            parser.pos += 1;
            if let Err(diagnostic) = parser.expect_punct(b"=") {
                return vec![fail(input.task, diagnostic)];
            }
            let rhs = match parser.parse_expression() {
                Ok(expression) => expression,
                Err(diagnostic) => return vec![fail(input.task, diagnostic)],
            };
            if let Err(diagnostic) = parser.expect_punct(b";") {
                return vec![fail(input.task, diagnostic)];
            }
            assignments.push((lhs_index, rhs));
        };
        let base = input.nodes_allocated;
        let node = |index: u32| crate::ids::NodeId::from_index(base.saturating_add(index));
        let mut next = 9u32;
        let mut dynamic = std::collections::BTreeMap::new();
        let mut assignment_ids = Vec::new();
        for (lhs_index, rhs) in assignments {
            let assignment_offset = next;
            next += 1;
            let assignment = node(assignment_offset);
            let lhs = node(next);
            dynamic.insert(
                next,
                (
                    NodeKind::Identifier,
                    Some(assignment),
                    vec![],
                    lhs_index,
                    lhs_index,
                    Some(decl_name),
                    None,
                ),
            );
            next += 1;
            let rhs_last = rhs.last;
            let rhs_root = assign_expression(rhs, &mut next, assignment, base, &mut dynamic);
            dynamic.insert(
                assignment_offset,
                (
                    NodeKind::Assignment,
                    Some(node(4)),
                    vec![lhs, rhs_root],
                    lhs_index,
                    rhs_last,
                    None,
                    None,
                ),
            );
            assignment_ids.push(assignment);
        }
        let return_offset = next;
        next += 1;
        let ret = node(return_offset);
        let return_root = assign_expression(return_expression, &mut next, ret, base, &mut dynamic);
        dynamic.insert(
            return_offset,
            (
                NodeKind::Return,
                Some(node(4)),
                vec![return_root],
                return_index,
                semicolon,
                None,
                None,
            ),
        );
        let mut compound_children = vec![node(5)];
        compound_children.extend(assignment_ids);
        compound_children.push(ret);
        let mut specs: Vec<NodeSpec> = vec![
            (
                NodeKind::TranslationUnit,
                None,
                vec![node(1)],
                0,
                eof,
                None,
                None,
            ),
            (
                NodeKind::FunctionDefinition,
                Some(node(0)),
                vec![node(2), node(3), node(4)],
                0,
                brace,
                None,
                None,
            ),
            (
                NodeKind::Specifiers,
                Some(node(1)),
                vec![],
                0,
                0,
                None,
                None,
            ),
            (
                NodeKind::Declarator,
                Some(node(1)),
                vec![],
                1,
                4,
                main_name,
                None,
            ),
            (
                NodeKind::Compound,
                Some(node(1)),
                compound_children,
                5,
                brace,
                None,
                None,
            ),
            (
                NodeKind::Declaration,
                Some(node(4)),
                vec![node(6), node(7), node(8)],
                6,
                10,
                None,
                None,
            ),
            (
                NodeKind::Specifiers,
                Some(node(5)),
                vec![],
                6,
                6,
                None,
                None,
            ),
            (
                NodeKind::Declarator,
                Some(node(5)),
                vec![],
                7,
                7,
                Some(decl_name),
                None,
            ),
            (
                NodeKind::IntLiteral,
                Some(node(5)),
                vec![],
                9,
                9,
                None,
                Some(*initializer),
            ),
        ];
        for offset in 9..next {
            let Some(spec) = dynamic.remove(&offset) else {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        "missing parsed locals node",
                    ),
                )];
            };
            specs.push(spec);
        }
        if specs.len() > CALL_MAX_AST_NODES {
            return vec![fail(
                input.task,
                DiagnosticDraft::unsupported("translation unit exceeds the 4096-node AST budget"),
            )];
        }
        append_nodes(input, specs)
    }

    /// `/54`/`/55` multi-function parser. Every node is built locally first;
    /// the single append happens only after the whole TU validates.
    fn compute_general(&self, input: &PaTuInput) -> Vec<Proposal> {
        let mut parser = CallParser::new(input);
        let (root, node_count) = match parser.parse_translation_unit() {
            Ok(parsed) => parsed,
            Err(diagnostic) => return vec![fail(input.task, diagnostic)],
        };
        if node_count > CALL_MAX_AST_NODES {
            return vec![fail(
                input.task,
                DiagnosticDraft::unsupported("translation unit exceeds the 4096-node AST budget"),
            )];
        }
        append_general(input, root, node_count)
    }
}

/// One parsed direct-call translation unit.
struct CallParser<'a> {
    input: &'a PaTuInput,
    pos: usize,
    node_count: usize,
}

impl<'a> CallParser<'a> {
    fn new(input: &'a PaTuInput) -> Self {
        Self {
            input,
            pos: 0,
            node_count: 0,
        }
    }

    fn body(&self, index: usize) -> Result<&'a ProjectedToken, DiagnosticDraft> {
        let id = self.input.tokens.get(index).ok_or_else(|| {
            DiagnosticDraft::unsupported("direct-call token sequence ends before a closed shape")
        })?;
        self.input.bodies.get(id).ok_or_else(|| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("parse reads missing token {}", id.index()),
            )
        })
    }

    fn bump(&mut self) -> Result<usize, DiagnosticDraft> {
        let index = self.pos;
        let _ = self.body(index)?;
        self.pos += 1;
        Ok(index)
    }

    fn keyword(&self, index: usize, word: &[u8]) -> bool {
        self.body(index)
            .is_ok_and(|body| body.kind == TokenKind::Keyword && body.spelling == word)
    }

    fn punct(&self, index: usize, spelling: &[u8]) -> bool {
        self.body(index)
            .is_ok_and(|body| body.kind == TokenKind::Punctuator && body.pp_spelling == spelling)
    }

    fn expect_keyword(&mut self, word: &[u8]) -> Result<usize, DiagnosticDraft> {
        if self.keyword(self.pos, word) {
            return self.bump();
        }
        Err(DiagnosticDraft::unsupported(
            "expected an `int`, `void`, or `return` keyword",
        ))
    }

    fn expect_punct(&mut self, spelling: &[u8]) -> Result<usize, DiagnosticDraft> {
        if self.punct(self.pos, spelling) {
            return self.bump();
        }
        Err(DiagnosticDraft::unsupported("missing expected punctuator"))
    }

    fn expect_identifier(&mut self) -> Result<(usize, NameId), DiagnosticDraft> {
        let index = self.pos;
        let body = self.body(index)?;
        if body.kind != TokenKind::Identifier {
            return Err(DiagnosticDraft::unsupported("expected an identifier"));
        }
        let Some(name) = body.name else {
            return Err(DiagnosticDraft::unsupported(
                "identifier has no interned name",
            ));
        };
        self.pos += 1;
        Ok((index, name))
    }

    fn make(
        &mut self,
        kind: NodeKind,
        first: usize,
        last: usize,
        name: Option<NameId>,
        literal: Option<LiteralId>,
        children: Vec<ExprNode>,
    ) -> ExprNode {
        self.node_count += 1;
        ExprNode {
            kind,
            first,
            last,
            name,
            literal,
            children,
        }
    }

    fn literal(&mut self) -> Result<(usize, LiteralId), DiagnosticDraft> {
        let index = self.pos;
        let body = self.body(index)?;
        if body.kind != TokenKind::Integer {
            return Err(DiagnosticDraft::unsupported("expected an integer literal"));
        }
        let token = self.input.tokens[index];
        let Some(literal) = self.input.literals_by_token.get(&token) else {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("parse reads missing literal for token {}", token.index()),
            ));
        };
        self.pos += 1;
        Ok((index, *literal))
    }

    fn parse_translation_unit(&mut self) -> Result<(ExprNode, usize), DiagnosticDraft> {
        let mut functions = Vec::new();
        loop {
            let body = self.body(self.pos)?;
            if body.kind == TokenKind::Eof {
                break;
            }
            if functions.len() == CALL_MAX_FUNCTIONS {
                return Err(DiagnosticDraft::unsupported(
                    "translation unit exceeds the 32-function budget",
                ));
            }
            functions.push(self.parse_function()?);
        }
        if functions.is_empty() {
            return Err(DiagnosticDraft::unsupported(
                "translation unit has no function definition",
            ));
        }
        let eof = self.pos;
        let body = self.body(eof)?;
        if !body.pp_spelling.is_empty() || body.name.is_some() {
            return Err(DiagnosticDraft::unsupported("malformed EOF token"));
        }
        self.pos += 1;
        if self.pos != self.input.tokens.len() {
            return Err(DiagnosticDraft::unsupported(
                "tokens after the trailing EOF",
            ));
        }
        let root = self.make(NodeKind::TranslationUnit, 0, eof, None, None, functions);
        Ok((root, self.node_count))
    }

    fn parse_function(&mut self) -> Result<ExprNode, DiagnosticDraft> {
        let first = self.expect_keyword(b"int")?;
        let specifiers = self.make(NodeKind::Specifiers, first, first, None, None, vec![]);
        let (name_index, name) = self.expect_identifier()?;
        let open = self.expect_punct(b"(")?;
        let mut parameters = Vec::new();
        if self.keyword(self.pos, b"void") {
            let _ = self.bump()?;
            let close = self.expect_punct(b")")?;
            let declarator = self.make(
                NodeKind::Declarator,
                name_index,
                close,
                Some(name),
                None,
                vec![],
            );
            return self.finish_function(first, specifiers, declarator);
        }
        if self.punct(self.pos, b")") {
            return Err(DiagnosticDraft::unsupported(
                "empty `()` parameter lists are not accepted",
            ));
        }
        loop {
            let parameter_first = self.expect_keyword(b"int")?;
            let (parameter_last, parameter_name) = self.expect_identifier()?;
            parameters.push(self.make(
                NodeKind::Parameter,
                parameter_first,
                parameter_last,
                Some(parameter_name),
                None,
                vec![],
            ));
            if parameters.len() > CALL_MAX_PARAMETERS {
                return Err(DiagnosticDraft::unsupported(
                    "function exceeds the eight-parameter budget",
                ));
            }
            if self.punct(self.pos, b",") {
                let _ = self.bump()?;
                continue;
            }
            break;
        }
        let close = self.expect_punct(b")")?;
        let parameter_list =
            self.make(NodeKind::ParameterList, open, close, None, None, parameters);
        let declarator = self.make(
            NodeKind::Declarator,
            name_index,
            close,
            Some(name),
            None,
            vec![parameter_list],
        );
        self.finish_function(first, specifiers, declarator)
    }

    fn finish_function(
        &mut self,
        first: usize,
        specifiers: ExprNode,
        declarator: ExprNode,
    ) -> Result<ExprNode, DiagnosticDraft> {
        let open = self.expect_punct(b"{")?;
        let mut statements = Vec::new();
        if self.keyword(self.pos, b"int") {
            statements.push(self.parse_local_declaration()?);
            if self.keyword(self.pos, b"int") {
                return Err(DiagnosticDraft::unsupported(
                    "only one local declaration is accepted",
                ));
            }
        }
        while self.body(self.pos)?.kind == TokenKind::Identifier {
            let (lhs_index, lhs_name) = self.expect_identifier()?;
            if !self.punct(self.pos, b"=") {
                return Err(DiagnosticDraft::unsupported(
                    "call statements and non-assignment identifier statements are rejected",
                ));
            }
            let _ = self.bump()?;
            let rhs = self.parse_expression(0)?;
            let semicolon = self.expect_punct(b";")?;
            let _ = semicolon;
            let lhs = self.make(
                NodeKind::Identifier,
                lhs_index,
                lhs_index,
                Some(lhs_name),
                None,
                vec![],
            );
            let last = rhs.last;
            statements.push(self.make(
                NodeKind::Assignment,
                lhs_index,
                last,
                None,
                None,
                vec![lhs, rhs],
            ));
        }
        let return_index = self.expect_keyword(b"return")?;
        let expression = self.parse_expression(0)?;
        let semicolon = self.expect_punct(b";")?;
        let returned = self.make(
            NodeKind::Return,
            return_index,
            semicolon,
            None,
            None,
            vec![expression],
        );
        statements.push(returned);
        let close = self.expect_punct(b"}")?;
        let compound = self.make(NodeKind::Compound, open, close, None, None, statements);
        Ok(self.make(
            NodeKind::FunctionDefinition,
            first,
            close,
            None,
            None,
            vec![specifiers, declarator, compound],
        ))
    }

    fn parse_local_declaration(&mut self) -> Result<ExprNode, DiagnosticDraft> {
        let first = self.expect_keyword(b"int")?;
        let specifiers = self.make(NodeKind::Specifiers, first, first, None, None, vec![]);
        let (name_index, name) = self.expect_identifier()?;
        let declarator = self.make(
            NodeKind::Declarator,
            name_index,
            name_index,
            Some(name),
            None,
            vec![],
        );
        let _ = self.expect_punct(b"=")?;
        let (literal_index, literal) = self.literal()?;
        let semicolon = self.expect_punct(b";")?;
        let initializer = self.make(
            NodeKind::IntLiteral,
            literal_index,
            literal_index,
            None,
            Some(literal),
            vec![],
        );
        Ok(self.make(
            NodeKind::Declaration,
            first,
            semicolon,
            None,
            None,
            vec![specifiers, declarator, initializer],
        ))
    }

    /// `depth` counts grouping, unary, and call-argument nesting; left-folded
    /// binary chains stay at their current level.
    fn parse_expression(&mut self, depth: usize) -> Result<ExprNode, DiagnosticDraft> {
        if depth > CALL_MAX_EXPRESSION_DEPTH {
            return Err(DiagnosticDraft::unsupported(
                "expression nesting exceeds the 32-level budget",
            ));
        }
        self.parse_additive(depth)
    }

    fn parse_additive(&mut self, depth: usize) -> Result<ExprNode, DiagnosticDraft> {
        let mut lhs = self.parse_multiplicative(depth)?;
        loop {
            let kind = if self.punct(self.pos, b"+") {
                NodeKind::BinaryAdd
            } else if self.punct(self.pos, b"-") {
                NodeKind::BinarySub
            } else {
                break;
            };
            let _ = self.bump()?;
            let rhs = self.parse_multiplicative(depth)?;
            let first = lhs.first;
            let last = rhs.last;
            lhs = self.make(kind, first, last, None, None, vec![lhs, rhs]);
        }
        Ok(lhs)
    }

    fn parse_multiplicative(&mut self, depth: usize) -> Result<ExprNode, DiagnosticDraft> {
        let mut lhs = self.parse_unary(depth)?;
        loop {
            let kind = if self.punct(self.pos, b"*") {
                NodeKind::BinaryMul
            } else if self.punct(self.pos, b"/") {
                NodeKind::BinaryDiv
            } else if self.punct(self.pos, b"%") {
                NodeKind::BinaryRem
            } else {
                break;
            };
            let _ = self.bump()?;
            let rhs = self.parse_unary(depth)?;
            let first = lhs.first;
            let last = rhs.last;
            lhs = self.make(kind, first, last, None, None, vec![lhs, rhs]);
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self, depth: usize) -> Result<ExprNode, DiagnosticDraft> {
        if depth > CALL_MAX_EXPRESSION_DEPTH {
            return Err(DiagnosticDraft::unsupported(
                "expression nesting exceeds the 32-level budget",
            ));
        }
        let kind = if self.punct(self.pos, b"+") {
            NodeKind::UnaryPlus
        } else if self.punct(self.pos, b"-") {
            NodeKind::UnaryMinus
        } else {
            return self.parse_primary(depth);
        };
        let first = self.bump()?;
        let operand = self.parse_unary(depth + 1)?;
        let last = operand.last;
        Ok(self.make(kind, first, last, None, None, vec![operand]))
    }

    fn parse_primary(&mut self, depth: usize) -> Result<ExprNode, DiagnosticDraft> {
        let index = self.pos;
        let body = self.body(index)?;
        match body.kind {
            TokenKind::Integer => {
                let (first, literal) = self.literal()?;
                Ok(self.make(
                    NodeKind::IntLiteral,
                    first,
                    first,
                    None,
                    Some(literal),
                    vec![],
                ))
            }
            TokenKind::Identifier => {
                let (first, name) = self.expect_identifier()?;
                let callee =
                    self.make(NodeKind::Identifier, first, first, Some(name), None, vec![]);
                if !self.punct(self.pos, b"(") {
                    return Ok(callee);
                }
                let _ = self.bump()?;
                let mut arguments = vec![callee];
                if self.punct(self.pos, b")") {
                    let close = self.bump()?;
                    let last = close;
                    return Ok(self.make(NodeKind::Call, first, last, None, None, arguments));
                }
                if self.keyword(self.pos, b"void") {
                    return Err(DiagnosticDraft::unsupported(
                        "`f(void)` call arguments are not accepted",
                    ));
                }
                loop {
                    arguments.push(self.parse_expression(depth + 1)?);
                    if arguments.len() > CALL_MAX_ARGUMENTS + 1 {
                        return Err(DiagnosticDraft::unsupported(
                            "call exceeds the eight-argument budget",
                        ));
                    }
                    if self.punct(self.pos, b",") {
                        let _ = self.bump()?;
                        continue;
                    }
                    break;
                }
                let close = self.expect_punct(b")")?;
                Ok(self.make(NodeKind::Call, first, close, None, None, arguments))
            }
            TokenKind::Punctuator if body.pp_spelling == b"(" => {
                let _ = self.bump()?;
                let inner = self.parse_expression(depth + 1)?;
                let _ = self.expect_punct(b")")?;
                Ok(inner)
            }
            _ => Err(DiagnosticDraft::unsupported(
                "expression primary is not covered by the direct-call slice",
            )),
        }
    }
}

fn append_general(input: &PaTuInput, root: ExprNode, node_count: usize) -> Vec<Proposal> {
    let mut specs: Vec<Option<NodeSpec>> = (0..node_count).map(|_| None).collect();
    let mut next = 0u32;
    let _ = assign_general(root, None, &mut next, input.nodes_allocated, &mut specs);
    let specs = specs.into_iter().flatten().collect();
    append_nodes(input, specs)
}

fn assign_general(
    node: ExprNode,
    parent: Option<crate::ids::NodeId>,
    next: &mut u32,
    base: u32,
    specs: &mut [Option<NodeSpec>],
) -> crate::ids::NodeId {
    let offset = *next;
    *next += 1;
    let id = crate::ids::NodeId::from_index(base.saturating_add(offset));
    let mut children = Vec::new();
    for child in node.children {
        children.push(assign_general(child, Some(id), next, base, specs));
    }
    specs[offset as usize] = Some((
        node.kind,
        parent,
        children,
        node.first,
        node.last,
        node.name,
        node.literal,
    ));
    id
}

struct ExprParser<'a> {
    input: &'a PaTuInput,
    pos: usize,
    local_name: Option<NameId>,
}

impl<'a> ExprParser<'a> {
    fn body(&self, index: usize) -> Result<&'a ProjectedToken, DiagnosticDraft> {
        let id =
            self.input.tokens.get(index).ok_or_else(|| {
                DiagnosticDraft::unsupported("expression ends before a closed shape")
            })?;
        self.input.bodies.get(id).ok_or_else(|| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("parse reads missing token {}", id.index()),
            )
        })
    }

    fn punct(&self, index: usize) -> Option<&[u8]> {
        let body = self.body(index).ok()?;
        (body.kind == TokenKind::Punctuator).then_some(body.pp_spelling.as_slice())
    }

    fn bump(&mut self) -> Result<usize, DiagnosticDraft> {
        let index = self.pos;
        let _ = self.body(index)?;
        self.pos += 1;
        Ok(index)
    }

    fn expect_punct(&mut self, spelling: &[u8]) -> Result<usize, DiagnosticDraft> {
        if self.punct(self.pos) == Some(spelling) {
            return self.bump();
        }
        Err(DiagnosticDraft::unsupported("missing expected punctuator"))
    }

    fn parse_expression(&mut self) -> Result<ExprNode, DiagnosticDraft> {
        self.parse_binary(0)
    }

    fn parse_binary(&mut self, min_bp: u8) -> Result<ExprNode, DiagnosticDraft> {
        let mut lhs = self.parse_unary()?;
        while let Some((kind, bp)) = self.peek_binary() {
            if bp < min_bp {
                break;
            }
            let _ = self.bump()?;
            let rhs = self.parse_binary(bp + 1)?;
            let first = lhs.first;
            let last = rhs.last;
            lhs = ExprNode {
                kind,
                first,
                last,
                name: None,
                literal: None,
                children: vec![lhs, rhs],
            };
        }
        Ok(lhs)
    }

    fn peek_binary(&self) -> Option<(NodeKind, u8)> {
        let spelling = self.punct(self.pos)?;
        match spelling {
            b"*" => Some((NodeKind::BinaryMul, 20)),
            b"/" => Some((NodeKind::BinaryDiv, 20)),
            b"%" => Some((NodeKind::BinaryRem, 20)),
            b"+" => Some((NodeKind::BinaryAdd, 10)),
            b"-" => Some((NodeKind::BinarySub, 10)),
            _ => None,
        }
    }

    fn parse_unary(&mut self) -> Result<ExprNode, DiagnosticDraft> {
        let kind = match self.punct(self.pos) {
            Some(b"+") => NodeKind::UnaryPlus,
            Some(b"-") => NodeKind::UnaryMinus,
            _ => return self.parse_primary(),
        };
        let first = self.bump()?;
        let operand = self.parse_unary()?;
        let last = operand.last;
        Ok(ExprNode {
            kind,
            first,
            last,
            name: None,
            literal: None,
            children: vec![operand],
        })
    }

    fn parse_primary(&mut self) -> Result<ExprNode, DiagnosticDraft> {
        let index = self.pos;
        let body = self.body(index)?;
        match body.kind {
            TokenKind::Integer => {
                let token = self.input.tokens[index];
                let Some(literal) = self.input.literals_by_token.get(&token) else {
                    return Err(DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("parse reads missing literal for token {}", token.index()),
                    ));
                };
                self.pos += 1;
                Ok(ExprNode {
                    kind: NodeKind::IntLiteral,
                    first: index,
                    last: index,
                    name: None,
                    literal: Some(*literal),
                    children: vec![],
                })
            }
            TokenKind::Identifier => {
                let Some(name) = body.name else {
                    return Err(DiagnosticDraft::unsupported(
                        "local expression identifier has no name",
                    ));
                };
                if self.local_name != Some(name) {
                    return Err(DiagnosticDraft::unsupported(
                        "local expression uses an identifier other than the local",
                    ));
                }
                self.pos += 1;
                Ok(ExprNode {
                    kind: NodeKind::Identifier,
                    first: index,
                    last: index,
                    name: Some(name),
                    literal: None,
                    children: vec![],
                })
            }
            TokenKind::Punctuator if body.pp_spelling == b"(" => {
                self.pos += 1;
                let inner = self.parse_binary(0)?;
                let _ = self.expect_punct(b")")?;
                Ok(inner)
            }
            _ => Err(DiagnosticDraft::unsupported(
                "local expression primary is not covered",
            )),
        }
    }
}

fn assign_expression(
    node: ExprNode,
    next: &mut u32,
    parent: crate::ids::NodeId,
    base: u32,
    out: &mut std::collections::BTreeMap<u32, NodeSpec>,
) -> crate::ids::NodeId {
    let offset = *next;
    *next += 1;
    let id = crate::ids::NodeId::from_index(base.saturating_add(offset));
    let mut children = Vec::new();
    for child in node.children {
        children.push(assign_expression(child, next, id, base, out));
    }
    out.insert(
        offset,
        (
            node.kind,
            Some(parent),
            children,
            node.first,
            node.last,
            node.name,
            node.literal,
        ),
    );
    id
}

fn append_nodes(input: &PaTuInput, specs: Vec<NodeSpec>) -> Vec<Proposal> {
    let mut records = Vec::new();
    let mut bodies = Vec::new();
    for (position, (kind, parent, children, first, last, name, literal)) in
        specs.into_iter().enumerate()
    {
        records.push(RecordDraft {
            family: crate::ids::RecordFamily::Node,
            index: DraftRef(position as u32),
        });
        bodies.push(G1DraftBody::Node(NodeRecord {
            kind,
            parent,
            children,
            first_token: input.tokens[first],
            last_token: input.tokens[last],
            name,
            literal,
        }));
    }
    vec![
        Proposal::AppendRecords {
            task: input.task,
            batch: AppendBatch { records, bodies },
        },
        Proposal::Complete {
            task: input.task,
            value: ResultValue::Record(RecordRef::Node(crate::ids::NodeId::from_index(
                input.nodes_allocated,
            ))),
        },
    ]
}
