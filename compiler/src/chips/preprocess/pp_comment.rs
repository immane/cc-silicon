// ============================================================================
// chips/preprocess/pp_comment.rs — T03 PP03 comment-replace worker
// (Wave 2 slice 7, `/16`)
//
// Reads one committed `Spliced` artifact, replaces comments with whitespace
// (one space per comment, every logical newline preserved), and appends one
// `CommentFree` artifact with the composed map. M1 scope: comments are
// recognized in code state only; inputs containing string/character
// literals are explicit `Unsupported` (the full literal/header-name
// protection machine stays deferred); an unterminated block comment fails
// with a typed diagnostic (the M1-NEG-01 carrier shape).
// ============================================================================

use crate::bus::{ArtifactKind, ArtifactRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{ArtifactId, RecordRef, TaskId};
use crate::manifest::{
    BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP_COMMENT_CHIP,
};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// Compose an output-to-input boundary map through the input artifact's
/// input-to-raw map: `out[i] = map_in[mid[i]]`. (Owned copy per chip file;
/// keeps chips independent. Bounds hold by construction.)
pub fn compose_map(map_in: &[u64], mid: &[u64]) -> Vec<u64> {
    debug_assert!(mid
        .iter()
        .all(|boundary| (*boundary as usize) < map_in.len()));
    mid.iter()
        .map(|boundary| map_in[*boundary as usize])
        .collect()
}

/// Narrow projection for the comment-replace computation.
#[derive(Clone, Debug)]
pub struct PpCommentInput {
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
pub fn project_pp_comment_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpCommentInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("comment of unknown task {}", task.index())))?;
    if record.kind != TaskKind::PREPROCESS_COMMENT {
        return Err(protocol_fault(format!(
            "comment task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "comment task {} payload must carry exactly one artifact",
            task.index()
        )));
    }
    let artifact = match record.payload.refs[0] {
        RecordRef::Artifact(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "comment task {} payload must be an artifact",
                task.index()
            )));
        }
    };
    let input = bus.arenas.artifacts.get(artifact).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("comment reads missing artifact {}", artifact.index()),
        )
    })?;
    Ok(PpCommentInput {
        task,
        state: record.state.clone(),
        input: input.clone(),
        artifacts_allocated: bus.arenas.artifacts.allocated(),
    })
}

/// The T03 comment-replace worker (PP03 M1 slice scope).
pub struct PpCommentChip;

impl Worker for PpCommentChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP_COMMENT_CHIP,
            chip_name: "PpCommentChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![TaskKind::PREPROCESS_COMMENT],
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
        let input = match project_pp_comment_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl PpCommentChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &PpCommentInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("comment task {} is not running", input.task.index()),
                ),
            )];
        }
        if input.input.kind != ArtifactKind::Spliced {
            return vec![fail(
                input.task,
                DiagnosticDraft::unsupported("comment of a non-spliced artifact kind"),
            )];
        }
        let Some(source) = input.input.source else {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "comment reads a sourceless artifact",
                ),
            )];
        };
        let (bytes, mid) = match replace_comments(&input.input.bytes) {
            Ok(output) => output,
            Err(diagnostic) => return vec![fail(input.task, diagnostic)],
        };
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
                        kind: ArtifactKind::CommentFree,
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

/// Replace comments with whitespace; return the output bytes plus the
/// output-to-input boundary map. `//` runs to the newline (preserved);
/// `/*` runs to the first `*/` with every inner newline preserved. A `"`
/// or `'` anywhere is explicit `Unsupported` (literal/header-name
/// protection deferred); EOF inside `/*` is a typed unterminated-comment
/// failure. Each comment becomes exactly one space spanning the comment
/// (zero-width only when empty, which cannot occur for a real opener).
pub fn replace_comments(input: &[u8]) -> Result<(Vec<u8>, Vec<u64>), DiagnosticDraft> {
    let mut out: Vec<u8> = Vec::with_capacity(input.len());
    let mut mid: Vec<u64> = Vec::with_capacity(input.len() + 1);
    mid.push(0);
    let mut cursor = 0usize;
    while cursor < input.len() {
        let byte = input[cursor];
        if byte == b'"' || byte == b'\'' {
            return Err(DiagnosticDraft::unsupported(
                "string/character literal: full scan-state protection is deferred",
            ));
        }
        if byte == b'/' && input.get(cursor + 1) == Some(&b'/') {
            cursor += 2;
            while cursor < input.len() && input[cursor] != b'\n' {
                cursor += 1;
            }
            out.push(b' ');
            mid.push(cursor as u64);
            continue;
        }
        if byte == b'/' && input.get(cursor + 1) == Some(&b'*') {
            cursor += 2;
            loop {
                if cursor >= input.len() {
                    return Err(DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        "unterminated block comment",
                    ));
                }
                if input[cursor] == b'*' && input.get(cursor + 1) == Some(&b'/') {
                    cursor += 2;
                    break;
                }
                if input[cursor] == b'\n' {
                    out.push(b'\n');
                    mid.push((cursor + 1) as u64);
                }
                cursor += 1;
            }
            out.push(b' ');
            mid.push(cursor as u64);
            continue;
        }
        out.push(byte);
        cursor += 1;
        mid.push(cursor as u64);
    }
    debug_assert_eq!(mid.len() as u64, out.len() as u64 + 1);
    Ok((out, mid))
}
