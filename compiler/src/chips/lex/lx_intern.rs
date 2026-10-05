// ============================================================================
// chips/lx_intern.rs — T04 LX02 name-intern worker (Wave 2 slice 2, `/11`)
//
// Reads committed PP tokens, appends one `Name` body per first-seen
// identifier spelling in payload order, and completes with `Ack`. PP has no
// keyword distinction, so keyword spellings (`int`, `void`, `return`) intern
// exactly like identifiers (resolves the NI-02 1-vs-4 count: the M1 fixture
// interns four spellings). The commit interns lookup-first, so re-runs and
// duplicates are idempotent and deterministic.
// ============================================================================

use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{PpTokenId, RecordRef, TaskId};
use crate::manifest::{
    BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, LX_INTERN_CHIP,
};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};
use std::collections::{BTreeMap, BTreeSet};

/// Narrow projection for the intern computation.
#[derive(Clone, Debug)]
pub struct LxInternInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload PP-token refs in source order.
    pub refs: Vec<RecordRef>,
    /// Present PP-token `(kind-is-identifier, spelling)` by ID.
    pub pp_tokens: BTreeMap<PpTokenId, (bool, Vec<u8>)>,
}

/// Build the narrow projection for one dispatched task.
pub fn project_lx_intern_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<LxInternInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("intern of unknown task {}", task.index())))?;
    if record.kind != TaskKind::LEX_INTERN {
        return Err(protocol_fault(format!(
            "intern task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    let mut pp_tokens = BTreeMap::new();
    for reference in &record.payload.refs {
        if let RecordRef::PpToken(id) = reference {
            if let Ok(body) = bus.arenas.pp_tokens.get(*id) {
                pp_tokens.insert(
                    *id,
                    (
                        body.kind == crate::bus::PpTokenKind::Identifier,
                        body.spelling.clone(),
                    ),
                );
            }
        }
    }
    Ok(LxInternInput {
        task,
        state: record.state.clone(),
        refs: record.payload.refs.clone(),
        pp_tokens,
    })
}

/// The T04 name-intern worker.
pub struct LxInternChip;

impl Worker for LxInternChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: LX_INTERN_CHIP,
            chip_name: "LxInternChip",
            group: TaskGroup::LEX,
            task_kinds: vec![TaskKind::LEX_INTERN],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Pp, "tokens"),
            ],
            writes: vec![FieldPath::new(StoreId::Names, "entries")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c11_lex.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_lx_intern_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl LxInternChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &LxInternInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("intern task {} is not running", input.task.index()),
                ),
            )];
        }
        let mut seen: BTreeSet<Vec<u8>> = BTreeSet::new();
        let mut bodies: Vec<G1DraftBody> = Vec::new();
        let mut records = Vec::new();
        for reference in &input.refs {
            let RecordRef::PpToken(id) = reference else {
                continue;
            };
            let Some((is_identifier, spelling)) = input.pp_tokens.get(id) else {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("intern reads missing pp-token {}", id.index()),
                    ),
                )];
            };
            if !is_identifier || !seen.insert(spelling.clone()) {
                continue;
            }
            records.push(RecordDraft {
                family: crate::ids::RecordFamily::Name,
                index: DraftRef(records.len() as u32),
            });
            bodies.push(G1DraftBody::Name {
                spelling: spelling.clone(),
            });
        }
        if bodies.is_empty() {
            return vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }];
        }
        vec![
            Proposal::AppendRecords {
                task: input.task,
                batch: AppendBatch { records, bodies },
            },
            Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            },
        ]
    }
}
