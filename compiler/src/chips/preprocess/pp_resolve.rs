// ============================================================================
// chips/preprocess/pp_resolve.rs — T03 PP17 include-name resolver
// (Wave 2 slice 16, `/25`)
//
// Reads exactly one `HeaderName` pp-token ref (anything else is a protocol
// fault). Strips the `<...>`/`"..."` delimiters (anything else, or an empty
// name, is a malformed typed failure) and resolves the spelling against
// every committed source in ascending-ID order: an exact full-name byte
// match wins immediately; otherwise basename-suffix matches (bytes after
// the last `/`) decide — zero matches fail `not-loaded`, more than one
// fails `ambiguous`, exactly one completes `Record(Source)`. Quote vs angle
// behave identically (config search-path divergence is recorded future
// work). Never appends, never enqueues.
// ============================================================================

use crate::bus::{PpTokenKind, PpTokenRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{PpTokenId, RecordRef, SourceId, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP17_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

/// Narrow projection for the include-resolve computation.
#[derive(Clone, Debug)]
pub struct PpIncludeResolveInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Header token body (must be `HeaderName`).
    pub header: (PpTokenId, PpTokenRecord),
    /// Candidate sources in ascending-ID order: (source id, interned name
    /// bytes) for every committed source.
    pub candidates: Vec<(SourceId, Vec<u8>)>,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pp_include_resolve_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpIncludeResolveInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("resolve of unknown task {}", task.index())))?;
    if record.kind != TaskKind::PREPROCESS_INCLUDE_RESOLVE {
        return Err(protocol_fault(format!(
            "resolve task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "resolve task {} payload must carry exactly one header-name token",
            task.index()
        )));
    }
    let header_id = match record.payload.refs[0] {
        RecordRef::PpToken(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "resolve task {} payload must be a pp-token ref",
                task.index()
            )));
        }
    };
    let header_body = bus.arenas.pp_tokens.get(header_id).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("resolve reads missing pp-token {}", header_id.index()),
        )
    })?;
    if header_body.kind != PpTokenKind::HeaderName {
        return Err(protocol_fault(format!(
            "resolve task {} payload must be a header-name token",
            task.index()
        )));
    }
    let mut candidates: Vec<(SourceId, Vec<u8>)> = Vec::new();
    for (id, body) in bus.arenas.sources.iter() {
        match bus.intern.get(body.name) {
            Ok(name) => candidates.push((id, name.to_vec())),
            Err(_) => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("resolve reads missing name for source {}", id.index()),
                ));
            }
        }
    }
    candidates.sort_by_key(|(id, _)| id.index());
    Ok(PpIncludeResolveInput {
        task,
        state: record.state.clone(),
        header: (header_id, header_body.clone()),
        candidates,
    })
}

/// The T03 include-name resolver (PP17 slice scope).
pub struct PpIncludeResolveChip;

impl Worker for PpIncludeResolveChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP17_CHIP,
            chip_name: "PpIncludeResolveChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![TaskKind::PREPROCESS_INCLUDE_RESOLVE],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Sources, "bytes"),
                FieldPath::new(StoreId::Names, "entries"),
            ],
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c25_include.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pp_include_resolve_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

/// Strip one `<...>` or `"..."` delimiter pair. `None` when the spelling
/// carries any other shape. Quote vs angle behave identically here.
fn strip_header_delimiters(spelling: &[u8]) -> Option<Vec<u8>> {
    if spelling.len() < 2 {
        return None;
    }
    let first = spelling[0];
    let last = spelling[spelling.len() - 1];
    let paired = (first == b'<' && last == b'>') || (first == b'"' && last == b'"');
    if !paired {
        return None;
    }
    let mut inner = Vec::with_capacity(spelling.len() - 2);
    for byte in spelling.iter().take(spelling.len() - 1).skip(1) {
        inner.push(*byte);
    }
    Some(inner)
}

/// Basename of an interned source name: bytes after the last `/`
/// (the whole name when it holds no `/`).
fn basename(name: &[u8]) -> &[u8] {
    let mut start = 0usize;
    for (index, byte) in name.iter().enumerate() {
        if *byte == b'/' {
            start = index + 1;
        }
    }
    &name[start..]
}

/// Lossy display form of a header spelling for diagnostics.
fn header_display(name: &[u8]) -> String {
    String::from_utf8_lossy(name).into_owned()
}

impl PpIncludeResolveChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &PpIncludeResolveInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("resolve task {} is not running", input.task.index()),
                ),
            )];
        }
        let spelling = &input.header.1.spelling;
        let Some(name) = strip_header_delimiters(spelling) else {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "resolve task {} has malformed header spelling",
                        input.task.index()
                    ),
                ),
            )];
        };
        if name.is_empty() {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("resolve task {} has empty header name", input.task.index()),
                ),
            )];
        }
        for (id, candidate) in &input.candidates {
            if *candidate == name {
                return vec![Proposal::Complete {
                    task: input.task,
                    value: ResultValue::Record(RecordRef::Source(*id)),
                }];
            }
        }
        let mut suffix_hits: Vec<SourceId> = Vec::new();
        for (id, candidate) in &input.candidates {
            if basename(candidate) == name.as_slice() {
                suffix_hits.push(*id);
            }
        }
        if suffix_hits.is_empty() {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "header `{}` is not loaded ({} sources searched)",
                        header_display(&name),
                        input.candidates.len()
                    ),
                ),
            )];
        }
        if suffix_hits.len() != 1 {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "ambiguous header `{}`: {} matches",
                        header_display(&name),
                        suffix_hits.len()
                    ),
                ),
            )];
        }
        vec![Proposal::Complete {
            task: input.task,
            value: ResultValue::Record(RecordRef::Source(suffix_hits[0])),
        }]
    }
}
