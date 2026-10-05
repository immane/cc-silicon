// ============================================================================
// chips/preprocess/pp_undef.rs — T03 PP08 undef-tombstone writer
// (Wave 2 slice 14, `/23`)
//
// Reads one post-`#` `#undef` line's refs. The first token must be the
// identifier `undef` (anything else is a protocol fault); exactly one
// identifier operand must follow (anything else is a malformed typed
// failure). Lookup takes the greatest committed `MacroId` with equal
// spelling: absent or latest-tombstone completes `Ack` with no append (C
// ignores it); present-and-defined appends one tombstone
// (`{spelling, [], false, [], true}`) and completes `Record` of the
// predicted ID. No children, no joins.
// ============================================================================

use crate::bus::{MacroRecord, PpTokenKind, PpTokenRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{MacroId, PpTokenId, RecordRef, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP08_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// Narrow projection for the undef-tombstone computation.
#[derive(Clone, Debug)]
pub struct PpUndefInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Line token bodies in payload order.
    pub tokens: Vec<(PpTokenId, PpTokenRecord)>,
    /// Committed macro definitions in ascending-ID order (latest wins).
    pub macros: Vec<(MacroId, MacroRecord)>,
    /// `macros` arena count at dispatch (tombstone prediction base).
    pub macros_allocated: u32,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pp_undef_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpUndefInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("undef of unknown task {}", task.index())))?;
    if record.kind != TaskKind::PREPROCESS_MACRO_UNDEF {
        return Err(protocol_fault(format!(
            "undef task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.is_empty() {
        return Err(protocol_fault(format!(
            "undef task {} payload must carry pp-token refs",
            task.index()
        )));
    }
    let mut tokens = Vec::with_capacity(record.payload.refs.len());
    for reference in &record.payload.refs {
        match reference {
            RecordRef::PpToken(id) => match bus.arenas.pp_tokens.get(*id) {
                Ok(found) => tokens.push((*id, found.clone())),
                Err(_) => {
                    return Err(DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("undef reads missing pp-token {}", id.index()),
                    ));
                }
            },
            _ => {
                return Err(protocol_fault(format!(
                    "undef task {} payload must be pp-token refs",
                    task.index()
                )));
            }
        }
    }
    let mut macros: Vec<(MacroId, MacroRecord)> = bus
        .arenas
        .macros
        .iter()
        .map(|(id, body)| (id, body.clone()))
        .collect();
    macros.sort_by_key(|(id, _)| id.index());
    Ok(PpUndefInput {
        task,
        state: record.state.clone(),
        tokens,
        macros,
        macros_allocated: bus.arenas.macros.allocated(),
    })
}

/// The T03 undef-tombstone writer (PP08 slice scope).
pub struct PpUndefChip;

impl Worker for PpUndefChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP08_CHIP,
            chip_name: "PpUndefChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![TaskKind::PREPROCESS_MACRO_UNDEF],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Pp, "macros"),
            ],
            writes: vec![FieldPath::new(StoreId::Pp, "macros")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c23_macro.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pp_undef_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

/// True when the token is the identifier `undef`.
fn is_undef_token(token: &PpTokenRecord) -> bool {
    token.kind == PpTokenKind::Identifier && token.spelling == b"undef"
}

/// True when the token is any identifier.
fn is_identifier(token: &PpTokenRecord) -> bool {
    token.kind == PpTokenKind::Identifier
}

/// Latest committed record with spelling equal to `spelling` (`macros` is
/// in ascending-ID order, so the last match wins). `None` when absent.
fn latest_with_spelling(macros: &[(MacroId, MacroRecord)], spelling: &[u8]) -> Option<MacroRecord> {
    let mut latest: Option<MacroRecord> = None;
    for (_, record) in macros {
        if record.spelling == spelling {
            latest = Some(record.clone());
        }
    }
    latest
}

impl PpUndefChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &PpUndefInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("undef task {} is not running", input.task.index()),
                ),
            )];
        }
        let mut cursor = 0usize;
        if input.tokens.len() <= cursor {
            return vec![fail(
                input.task,
                protocol_fault(format!(
                    "undef task {} payload must carry pp-token refs",
                    input.task.index()
                )),
            )];
        }
        if !is_undef_token(&input.tokens[cursor].1) {
            return vec![fail(
                input.task,
                protocol_fault(format!(
                    "undef task {} first token must be identifier `undef`",
                    input.task.index()
                )),
            )];
        }
        cursor += 1;
        if input.tokens.len() != cursor + 1 {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "undef task {} must carry exactly one identifier operand",
                        input.task.index()
                    ),
                ),
            )];
        }
        if !is_identifier(&input.tokens[cursor].1) {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "undef task {} operand must be an identifier",
                        input.task.index()
                    ),
                ),
            )];
        }
        let spelling = input.tokens[cursor].1.spelling.clone();
        match latest_with_spelling(&input.macros, &spelling) {
            None => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Some(latest) => {
                if latest.undefined {
                    return vec![Proposal::Complete {
                        task: input.task,
                        value: ResultValue::Ack,
                    }];
                }
                let predicted = MacroId::from_index(input.macros_allocated);
                vec![
                    Proposal::AppendRecords {
                        task: input.task,
                        batch: AppendBatch {
                            records: vec![RecordDraft {
                                family: crate::ids::RecordFamily::Macro,
                                index: DraftRef(0),
                            }],
                            bodies: vec![G1DraftBody::Macro(MacroRecord {
                                spelling,
                                params: Vec::new(),
                                variadic: false,
                                replacement: Vec::new(),
                                undefined: true,
                            })],
                        },
                    },
                    Proposal::Complete {
                        task: input.task,
                        value: ResultValue::Record(RecordRef::Macro(predicted)),
                    },
                ]
            }
        }
    }
}
