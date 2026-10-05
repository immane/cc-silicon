// ============================================================================
// chips/pp_normalize.rs — T03 PP01 source-normalize worker (Wave 2 slice 1)
//
// Reads one committed source, normalizes newlines, appends exactly one
// single-source `Normalized` artifact with its raw-boundary map, and
// completes with its predicted reference. M1 exercised scope only:
// identity LF input, missing-final-LF insertion, and CRLF collapse. A lone
// CR is explicit `Unsupported`, never a silent reinterpretation.
//
// Isolation: the semantic computation sees only `PpInput` (task header,
// the referenced source bytes, the artifact-arena count, and the byte
// budget). It never receives the full bus; the `Worker` adapter below owns
// the narrow projection.
// ============================================================================

use crate::chips::{fail, protocol_fault, Worker};
use crate::bus::{ArtifactKind, ArtifactRecord};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{ArtifactId, RecordRef, SourceId, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP01_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Payload, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};
use std::collections::BTreeMap;

/// Narrow projection for the normalize computation.
#[derive(Clone, Debug)]
pub struct PpInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Task kind (must be `PREPROCESS_NORMALIZE`).
    pub kind: TaskKind,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// The single declared source reference.
    pub source: SourceId,
    /// Committed source bytes.
    pub source_bytes: Vec<u8>,
    /// `artifacts` arena count at dispatch (single-append prediction base).
    pub artifacts_allocated: u32,
    /// Configured source/artifact byte budget (`limits.max_source_bytes`).
    pub max_source_bytes: u64,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pp_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("normalize of unknown task {}", task.index())))?;
    if record.kind != TaskKind::PREPROCESS_NORMALIZE {
        return Err(protocol_fault(format!(
            "normalize task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "normalize task {} payload must carry exactly one source, got {}",
            task.index(),
            record.payload.refs.len()
        )));
    }
    let source = match record.payload.refs[0] {
        RecordRef::Source(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "normalize task {} payload position 0 must be a source",
                task.index()
            )));
        }
    };
    let source_bytes = bus
        .arenas
        .sources
        .get(source)
        .map(|record| record.bytes.clone())
        .map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("normalize reads missing source {}", source.index()),
            )
        })?;
    Ok(PpInput {
        task,
        kind: record.kind,
        state: record.state.clone(),
        source,
        source_bytes,
        artifacts_allocated: bus.arenas.artifacts.allocated(),
        max_source_bytes: bus.limits().max_source_bytes,
    })
}

/// The T03 source-normalize worker: the first Wave 2 chip.
pub struct PpNormalizeChip;

impl Worker for PpNormalizeChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP01_CHIP,
            chip_name: "PpNormalizeChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![TaskKind::PREPROCESS_NORMALIZE],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Sources, "bytes"),
                FieldPath::new(StoreId::Artifacts, "fragments"),
                FieldPath::new(StoreId::Config, "limits"),
            ],
            writes: vec![FieldPath::new(StoreId::Artifacts, "fragments")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c10_pp01.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pp_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl PpNormalizeChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &PpInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("normalize task {} is not running", input.task.index()),
                ),
            )];
        }
        let (bytes, raw_offsets) = match normalize(&input.source_bytes) {
            Ok(output) => output,
            Err(diagnostic) => return vec![fail(input.task, diagnostic)],
        };
        if bytes.len() as u64 > input.max_source_bytes {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Config, 1),
                    format!(
                        "normalized artifact needs {} bytes, budget is {}",
                        bytes.len(),
                        input.max_source_bytes
                    ),
                ),
            )];
        }
        let predicted = ArtifactId::from_index(input.artifacts_allocated);
        vec![
            Proposal::AppendRecords {
                task: input.task,
                batch: AppendBatch {
                    records: vec![RecordDraft {
                        family: crate::ids::RecordFamily::Artifact,
                        index: DraftRef(0),
                    }],
                    bodies: vec![G1DraftBody::Artifact(ArtifactRecord {
                        kind: ArtifactKind::Normalized,
                        source: Some(input.source),
                        bytes,
                        raw_offsets,
                    })],
                },
            },
            Proposal::Complete {
                task: input.task,
                value: ResultValue::Record(RecordRef::Artifact(predicted)),
            },
        ]
    }
}

/// Normalize raw source bytes to the M1 `Normalized` form.
///
/// Returns the output bytes plus the output-boundary to raw-boundary map
/// (`raw_offsets.len() == bytes.len()+1`). CRLF collapses to one LF whose
/// start maps to the raw CR and whose end maps after the raw LF; a missing
/// terminal LF is inserted as a zero-width map at raw EOF. A lone CR is
/// explicit `Unsupported` (beyond the M1 exercised scope).
pub fn normalize(raw: &[u8]) -> Result<(Vec<u8>, Vec<u64>), DiagnosticDraft> {
    let mut out: Vec<u8> = Vec::with_capacity(raw.len() + 1);
    let mut map: Vec<u64> = Vec::with_capacity(raw.len() + 2);
    map.push(0);
    let mut cursor = 0usize;
    while cursor < raw.len() {
        if raw[cursor] == b'\r' {
            match raw.get(cursor + 1) {
                Some(b'\n') => {
                    out.push(b'\n');
                    map.push((cursor + 2) as u64);
                    cursor += 2;
                }
                _ => {
                    return Err(DiagnosticDraft::unsupported(
                        "lone CR is outside the M1 exercised normalization scope",
                    ));
                }
            }
        } else {
            out.push(raw[cursor]);
            map.push((cursor + 1) as u64);
            cursor += 1;
        }
    }
    if out.last() != Some(&b'\n') {
        out.push(b'\n');
        map.push(raw.len() as u64);
    }
    debug_assert_eq!(map.len() as u64, out.len() as u64 + 1);
    debug_assert_eq!(map.first(), Some(&0));
    Ok((out, map))
}

/// Reference literal bodies by ID (test helper surface parity with fold).
#[allow(dead_code)]
pub fn referenced_sources(payload: &Payload) -> BTreeMap<SourceId, ()> {
    let mut sources = BTreeMap::new();
    for reference in &payload.refs {
        if let RecordRef::Source(id) = reference {
            sources.insert(*id, ());
        }
    }
    sources
}
