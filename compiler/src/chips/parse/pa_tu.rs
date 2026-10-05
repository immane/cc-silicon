// ============================================================================
// chips/pa_tu.rs — T05 PA01 TU-parse worker (Wave 2 slice 3, `/12`)
//
// Reads 13 committed C tokens in source order, validates the exact M1 shape
// (`int main(void){return 2+3;}` plus EOF), appends the nine-node M1 tree in
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
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PA_TU_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};
use std::collections::BTreeMap;

/// One projected token: kind plus resolved spelling for named tokens.
#[derive(Clone, Debug)]
pub struct ProjectedToken {
    /// C-token kind.
    pub kind: TokenKind,
    /// Interned name, if any.
    pub name: Option<NameId>,
    /// Spelling bytes for named tokens.
    pub spelling: Vec<u8>,
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
            bodies.insert(
                *id,
                ProjectedToken {
                    kind: body.kind,
                    name: body.name,
                    spelling,
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
                FieldPath::new(StoreId::Names, "entries"),
                FieldPath::new(StoreId::Parse, "nodes"),
            ],
            writes: vec![FieldPath::new(StoreId::Parse, "nodes")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c12_parse.rs"],
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
                value: ResultValue::Record(RecordRef::Node(node(0))),
            },
        ]
    }
}
