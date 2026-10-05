// ============================================================================
// chips/preprocess/pp_splice.rs — T03 PP02 line-splice worker
// (Wave 2 slice 7, `/16`)
//
// Reads one committed `Normalized` (or idempotent `Spliced`) artifact,
// deletes every backslash-newline pair, and appends one `Spliced` artifact
// whose map composes through the input map to the raw source. Inputs
// without a splice pair take the identity fast path (same bytes, copied
// map). A lone trailing backslash is an ordinary byte, never a splice.
// ============================================================================

use crate::bus::{ArtifactKind, ArtifactRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{ArtifactId, RecordRef, TaskId};
use crate::manifest::{
    BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP_SPLICE_CHIP,
};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// Compose an output-to-input boundary map through the input artifact's
/// input-to-raw map: `out[i] = map_in[mid[i]]`. (Owned copy per chip file;
/// keeps chips independent. Bounds hold by construction: every `mid`
/// entry is an input boundary position.)
pub fn compose_map(map_in: &[u64], mid: &[u64]) -> Vec<u64> {
    debug_assert!(mid
        .iter()
        .all(|boundary| (*boundary as usize) < map_in.len()));
    mid.iter()
        .map(|boundary| map_in[*boundary as usize])
        .collect()
}

/// Narrow projection for the splice computation.
#[derive(Clone, Debug)]
pub struct PpSpliceInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Input artifact body.
    pub input: ArtifactRecord,
    /// `artifacts` arena count at dispatch (prediction base).
    pub artifacts_allocated: u32,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pp_splice_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpSpliceInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("splice of unknown task {}", task.index())))?;
    if record.kind != TaskKind::PREPROCESS_SPLICE {
        return Err(protocol_fault(format!(
            "splice task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "splice task {} payload must carry exactly one artifact",
            task.index()
        )));
    }
    let artifact = match record.payload.refs[0] {
        RecordRef::Artifact(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "splice task {} payload must be an artifact",
                task.index()
            )));
        }
    };
    let input = bus.arenas.artifacts.get(artifact).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("splice reads missing artifact {}", artifact.index()),
        )
    })?;
    Ok(PpSpliceInput {
        task,
        state: record.state.clone(),
        input: input.clone(),
        artifacts_allocated: bus.arenas.artifacts.allocated(),
    })
}

/// The T03 line-splice worker (PP02 slice scope).
pub struct PpSpliceChip;

impl Worker for PpSpliceChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP_SPLICE_CHIP,
            chip_name: "PpSpliceChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![TaskKind::PREPROCESS_SPLICE],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Artifacts, "fragments"),
            ],
            writes: vec![FieldPath::new(StoreId::Artifacts, "fragments")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c16_pp.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pp_splice_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl PpSpliceChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &PpSpliceInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("splice task {} is not running", input.task.index()),
                ),
            )];
        }
        if !matches!(
            input.input.kind,
            ArtifactKind::Normalized | ArtifactKind::Spliced
        ) {
            return vec![fail(
                input.task,
                DiagnosticDraft::unsupported("splice of a non-spliceable artifact kind"),
            )];
        }
        let Some(source) = input.input.source else {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "splice reads a sourceless artifact",
                ),
            )];
        };
        let (bytes, mid) = splice(&input.input.bytes);
        let raw_offsets = compose_map(&input.input.raw_offsets, &mid);
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
                        kind: ArtifactKind::Spliced,
                        source: Some(source),
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

/// Delete every backslash-newline pair; return the output bytes plus the
/// output-to-input boundary map (`mid.len() == out.len()+1`).
pub fn splice(input: &[u8]) -> (Vec<u8>, Vec<u64>) {
    let mut out: Vec<u8> = Vec::with_capacity(input.len());
    let mut mid: Vec<u64> = Vec::with_capacity(input.len() + 1);
    mid.push(0);
    let mut cursor = 0usize;
    while cursor < input.len() {
        if input[cursor] == b'\\' && input.get(cursor + 1) == Some(&b'\n') {
            cursor += 2;
        } else {
            out.push(input[cursor]);
            cursor += 1;
            mid.push(cursor as u64);
        }
    }
    debug_assert_eq!(mid.len() as u64, out.len() as u64 + 1);
    (out, mid)
}
