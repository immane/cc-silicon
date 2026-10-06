// ============================================================================
// chips/preprocess/pp_diagnostic.rs — T03 PP26 `#error` reporter
// (Wave 2 slice 12, `/21`)
//
// Reads one dispatched directive line's post-`#` pp-token refs and always
// fails with the line's message (the diagnostic IS the product). The first
// ref must be Identifier `error` (wrong dispatch is a protocol fault); the
// message is the remaining token spellings joined with single spaces
// (empty remainder reports `"error directive"`). This never-complete worker
// is by design. Reads Tasks `active.*` and Pp `tokens`; no writes.
// ============================================================================

use crate::bus::{PpTokenKind, PpTokenRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{PpTokenId, RecordRef, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP26_CHIP};
use crate::task::{Proposal, StoreId, TaskGroup, TaskKind, TaskState};

/// Narrow projection for the `#error` diagnostic computation.
#[derive(Clone, Debug)]
pub struct PpDiagnosticInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// The line's token bodies in payload order.
    pub tokens: Vec<(PpTokenId, PpTokenRecord)>,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pp_diagnostic_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpDiagnosticInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("diagnostic of unknown task {}", task.index())))?;
    if record.kind != TaskKind::PREPROCESS_DIAGNOSTIC {
        return Err(protocol_fault(format!(
            "diagnostic task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.is_empty() {
        return Err(protocol_fault(format!(
            "diagnostic task {} payload must carry at least one pp-token",
            task.index()
        )));
    }
    let mut tokens = Vec::with_capacity(record.payload.refs.len());
    for owned in &record.payload.refs {
        let id = match *owned {
            RecordRef::PpToken(id) => id,
            _ => {
                return Err(protocol_fault(format!(
                    "diagnostic task {} payload must be pp-token refs",
                    task.index()
                )));
            }
        };
        let body = bus.arenas.pp_tokens.get(id).map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("diagnostic reads missing pp-token {}", id.index()),
            )
        })?;
        tokens.push((id, body.clone()));
    }
    Ok(PpDiagnosticInput {
        task,
        state: record.state.clone(),
        tokens,
    })
}

/// The T03 `#error` reporter (PP26): always fails with the line's message.
pub struct PpDiagnosticChip;

impl Worker for PpDiagnosticChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP26_CHIP,
            chip_name: "PpDiagnosticChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![TaskKind::PREPROCESS_DIAGNOSTIC],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Pp, "tokens"),
            ],
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c21_directive.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pp_diagnostic_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

/// True when the record is the Identifier spelling `error`.
fn is_error_name(record: &PpTokenRecord) -> bool {
    record.kind == PpTokenKind::Identifier && record.spelling == b"error".to_vec()
}

/// Lossy UTF-8 decode of one token spelling.
fn decode_spelling(spelling: &[u8]) -> String {
    String::from_utf8_lossy(spelling).into_owned()
}

/// Join the post-`error` token spellings with single spaces; an empty
/// remainder (or an all-empty join) reports the default message.
fn error_message(tokens: &[(PpTokenId, PpTokenRecord)]) -> String {
    let mut message = String::new();
    let mut started = false;
    for (_, record) in tokens.iter().skip(1) {
        if started {
            message.push(' ');
        }
        started = true;
        message.push_str(&decode_spelling(&record.spelling));
    }
    if message.is_empty() {
        message.push_str("error directive");
    }
    message
}

impl PpDiagnosticChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &PpDiagnosticInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("diagnostic task {} is not running", input.task.index()),
                ),
            )];
        }
        let first = match input.tokens.first() {
            Some(found) => found,
            None => {
                return vec![fail(
                    input.task,
                    protocol_fault(format!(
                        "diagnostic task {} payload must carry at least one pp-token",
                        input.task.index()
                    )),
                )];
            }
        };
        if !is_error_name(&first.1) {
            return vec![fail(
                input.task,
                protocol_fault(format!(
                    "diagnostic task {} dispatched a non-error directive line",
                    input.task.index()
                )),
            )];
        }
        vec![fail(
            input.task,
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                error_message(&input.tokens),
            ),
        )]
    }
}
