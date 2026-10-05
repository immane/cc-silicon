// ============================================================================
// chips/lx_classify.rs — T04 LX01/LX03/LX04 token-classify worker
// (Wave 2 slice 2, `/11`)
//
// Reads committed PP tokens in payload order, appends one C `Token` per
// token in the same order, and completes with `Records` of the predicted
// token refs. Identifier spellings resolve through the committed intern
// table (the LX intern task runs first); a missing name fails loudly.
// Non-M1 PP spellings (non-decimal pp-numbers, string/character literals)
// are explicit `Unsupported`. Token spans reuse the committed PP spans;
// T04 writes no spans.
// ============================================================================

use super::{fail, protocol_fault, Worker};
use crate::bus::{PpTokenKind, TokenKind};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{NameId, PpTokenId, RecordRef, SpanId, TaskId, TokenId};
use crate::manifest::{
    BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, LX_CLASSIFY_CHIP,
};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};
use std::collections::BTreeMap;

/// The frozen C11 keyword table (membership-tested, order-free).
pub const C11_KEYWORDS: &[&[u8]] = &[
    b"auto",
    b"break",
    b"case",
    b"char",
    b"const",
    b"continue",
    b"default",
    b"do",
    b"double",
    b"else",
    b"enum",
    b"extern",
    b"float",
    b"for",
    b"goto",
    b"if",
    b"inline",
    b"int",
    b"long",
    b"register",
    b"restrict",
    b"return",
    b"short",
    b"signed",
    b"sizeof",
    b"static",
    b"struct",
    b"switch",
    b"typedef",
    b"union",
    b"unsigned",
    b"void",
    b"volatile",
    b"while",
    b"_Alignas",
    b"_Alignof",
    b"_Atomic",
    b"_Bool",
    b"_Complex",
    b"_Generic",
    b"_Imaginary",
    b"_Noreturn",
    b"_Static_assert",
    b"_Thread_local",
];

/// Whether a spelling is a C11 keyword.
pub fn is_keyword(spelling: &[u8]) -> bool {
    C11_KEYWORDS.contains(&spelling)
}

/// One projected PP token: kind, span, spelling, and the resolved name for
/// identifier spellings (`None` when the spelling is not yet interned, which
/// fails loudly in `compute`).
#[derive(Clone, Debug)]
pub struct ProjectedPpToken {
    /// PP-token kind.
    pub kind: PpTokenKind,
    /// Committed PP span, reused by the C token.
    pub span: SpanId,
    /// Raw spelling bytes.
    pub spelling: Vec<u8>,
    /// Committed interned name for identifier spellings.
    pub name: Option<NameId>,
}

/// Narrow projection for the classify computation.
#[derive(Clone, Debug)]
pub struct LxClassifyInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload PP-token refs in source order.
    pub refs: Vec<RecordRef>,
    /// Present PP tokens by ID.
    pub pp_tokens: BTreeMap<PpTokenId, ProjectedPpToken>,
    /// `tokens` arena count at dispatch (append prediction base).
    pub tokens_allocated: u32,
}

/// Build the narrow projection for one dispatched task.
pub fn project_lx_classify_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<LxClassifyInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("classify of unknown task {}", task.index())))?;
    if record.kind != TaskKind::LEX_CLASSIFY {
        return Err(protocol_fault(format!(
            "classify task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    let mut pp_tokens = BTreeMap::new();
    for reference in &record.payload.refs {
        if let RecordRef::PpToken(id) = reference {
            if let Ok(body) = bus.arenas.pp_tokens.get(*id) {
                let name = if body.kind == PpTokenKind::Identifier {
                    bus.intern.lookup(&body.spelling)
                } else {
                    None
                };
                pp_tokens.insert(
                    *id,
                    ProjectedPpToken {
                        kind: body.kind,
                        span: body.span,
                        spelling: body.spelling.clone(),
                        name,
                    },
                );
            }
        }
    }
    Ok(LxClassifyInput {
        task,
        state: record.state.clone(),
        refs: record.payload.refs.clone(),
        pp_tokens,
        tokens_allocated: bus.arenas.tokens.allocated(),
    })
}

/// The T04 token-classify worker (LX01/LX03/LX04 slice scope).
pub struct LxClassifyChip;

impl Worker for LxClassifyChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: LX_CLASSIFY_CHIP,
            chip_name: "LxClassifyChip",
            group: TaskGroup::LEX,
            task_kinds: vec![TaskKind::LEX_CLASSIFY],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Names, "entries"),
                FieldPath::new(StoreId::Lex, "tokens"),
            ],
            writes: vec![FieldPath::new(StoreId::Lex, "tokens")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c11_lex.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_lx_classify_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl LxClassifyChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &LxClassifyInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("classify task {} is not running", input.task.index()),
                ),
            )];
        }
        let mut records = Vec::new();
        let mut bodies = Vec::new();
        let mut refs = Vec::new();
        for reference in &input.refs {
            let RecordRef::PpToken(id) = reference else {
                return vec![fail(
                    input.task,
                    protocol_fault(format!(
                        "classify task {} payload must carry pp-tokens only",
                        input.task.index()
                    )),
                )];
            };
            let Some(projected) = input.pp_tokens.get(id) else {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("classify reads missing pp-token {}", id.index()),
                    ),
                )];
            };
            let kind = match projected.kind {
                PpTokenKind::Identifier => {
                    if is_keyword(&projected.spelling) {
                        TokenKind::Keyword
                    } else {
                        TokenKind::Identifier
                    }
                }
                PpTokenKind::PpNumber => {
                    if projected.spelling.iter().all(|byte| byte.is_ascii_digit()) {
                        TokenKind::Integer
                    } else {
                        return vec![fail(
                            input.task,
                            DiagnosticDraft::unsupported(
                                "non-decimal pp-number is outside the M1 exercised subset",
                            ),
                        )];
                    }
                }
                PpTokenKind::Punctuator => TokenKind::Punctuator,
                PpTokenKind::Eof => TokenKind::Eof,
            };
            let name = match projected.kind {
                PpTokenKind::Identifier => match projected.name {
                    Some(name) => Some(name),
                    None => {
                        return vec![fail(
                            input.task,
                            DiagnosticDraft::error(
                                DiagnosticCode::new(DiagGroup::Task, 4),
                                format!(
                                    "classify reads missing interned name for pp-token {}",
                                    id.index()
                                ),
                            ),
                        )];
                    }
                },
                _ => None,
            };
            let position = records.len() as u32;
            records.push(RecordDraft {
                family: crate::ids::RecordFamily::Token,
                index: DraftRef(position),
            });
            bodies.push(G1DraftBody::Token(crate::bus::TokenRecord {
                kind,
                span: projected.span,
                name,
                pp_token: *id,
            }));
            refs.push(RecordRef::Token(TokenId::from_index(
                input.tokens_allocated.saturating_add(position),
            )));
        }
        if bodies.is_empty() {
            return vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Records(refs),
            }];
        }
        vec![
            Proposal::AppendRecords {
                task: input.task,
                batch: AppendBatch { records, bodies },
            },
            Proposal::Complete {
                task: input.task,
                value: ResultValue::Records(refs),
            },
        ]
    }
}
