// ============================================================================
// chips/preprocess/pp_scan.rs — T03 PP04 token-scan worker
// (Wave 2 slice 7, `/16`)
//
// Reads one committed `CommentFree` artifact, scans PP tokens with maximal
// munch, and appends one span per token (raw offsets remapped through the
// input map before any draft is formed) plus the tokens in order, closing
// with exactly one zero-width EOF. M1 scope: identifiers, decimal-led
// pp-numbers (with `e`/`E`/`p`/`P` sign continuation), the six M1
// punctuators, and EOF. String/character literals, header names, and every
// other punctuator are explicit `Unsupported` (never mis-tokenized).
// ============================================================================

use crate::bus::{ArtifactKind, PpTokenKind, PpTokenRecord, SpanRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{PpTokenId, RecordRef, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP_SCAN_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// M1 punctuator spellings (single character each).
pub const M1_PUNCTUATORS: &[u8] = b"(){}+;";

/// Narrow projection for the token-scan computation.
#[derive(Clone, Debug)]
pub struct PpScanInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Input artifact body.
    pub input: crate::bus::ArtifactRecord,
    /// `spans` arena count at dispatch (prediction base).
    pub spans_allocated: u32,
    /// `pp_tokens` arena count at dispatch (prediction base).
    pub pp_tokens_allocated: u32,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pp_scan_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpScanInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("scan of unknown task {}", task.index())))?;
    if record.kind != TaskKind::PREPROCESS_SCAN {
        return Err(protocol_fault(format!(
            "scan task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "scan task {} payload must carry exactly one artifact",
            task.index()
        )));
    }
    let artifact = match record.payload.refs[0] {
        RecordRef::Artifact(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "scan task {} payload must be an artifact",
                task.index()
            )));
        }
    };
    let input = bus.arenas.artifacts.get(artifact).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("scan reads missing artifact {}", artifact.index()),
        )
    })?;
    Ok(PpScanInput {
        task,
        state: record.state.clone(),
        input: input.clone(),
        spans_allocated: bus.arenas.spans.allocated(),
        pp_tokens_allocated: bus.arenas.pp_tokens.allocated(),
    })
}

/// The T03 token-scan worker (PP04 M1 slice scope).
pub struct PpScanChip;

impl Worker for PpScanChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP_SCAN_CHIP,
            chip_name: "PpScanChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![TaskKind::PREPROCESS_SCAN],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Artifacts, "fragments"),
                FieldPath::new(StoreId::Pp, "tokens"),
            ],
            writes: vec![
                FieldPath::new(StoreId::Sources, "spans"),
                FieldPath::new(StoreId::Pp, "tokens"),
            ],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c16_pp.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pp_scan_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

/// One scanned token: output range plus kind and spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScannedToken {
    /// Token kind.
    pub kind: PpTokenKind,
    /// Start offset in the scanned bytes.
    pub start: usize,
    /// End offset (exclusive).
    pub end: usize,
}

impl PpScanChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &PpScanInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("scan task {} is not running", input.task.index()),
                ),
            )];
        }
        if input.input.kind != ArtifactKind::CommentFree {
            return vec![fail(
                input.task,
                DiagnosticDraft::unsupported("scan of a non-comment-free artifact kind"),
            )];
        }
        let Some(source) = input.input.source else {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "scan reads a sourceless artifact",
                ),
            )];
        };
        let tokens = match scan(&input.input.bytes) {
            Ok(tokens) => tokens,
            Err(diagnostic) => return vec![fail(input.task, diagnostic)],
        };
        // Remap every output range through the input map BEFORE forming a
        // draft, so stored spans are always raw source offsets. Spans append
        // first (positions `0..n`, EOF span at `n`), then tokens (positions
        // `n+1..`); token `i` references span `spans_allocated + i`. The map
        // is commit-validated (`len == bytes+1`), so output ranges index it
        // exactly.
        let map = &input.input.raw_offsets;
        let mut records = Vec::new();
        let mut bodies = Vec::new();
        let mut refs = Vec::new();
        let mut position = 0u32;
        for token in &tokens {
            records.push(RecordDraft {
                family: crate::ids::RecordFamily::Span,
                index: DraftRef(position),
            });
            bodies.push(G1DraftBody::Span(SpanRecord {
                source,
                start: map[token.start],
                end: map[token.end],
                expansion: None,
            }));
            position += 1;
        }
        // EOF span: zero-width at the raw end.
        let raw_end = map[input.input.bytes.len()];
        records.push(RecordDraft {
            family: crate::ids::RecordFamily::Span,
            index: DraftRef(position),
        });
        bodies.push(G1DraftBody::Span(SpanRecord {
            source,
            start: raw_end,
            end: raw_end,
            expansion: None,
        }));
        position += 1;
        for (index, token) in tokens.iter().enumerate() {
            records.push(RecordDraft {
                family: crate::ids::RecordFamily::PpToken,
                index: DraftRef(position),
            });
            bodies.push(G1DraftBody::PpToken(PpTokenRecord {
                kind: token.kind,
                span: crate::ids::SpanId::from_index(
                    input.spans_allocated.saturating_add(index as u32),
                ),
                spelling: input.input.bytes[token.start..token.end].to_vec(),
            }));
            refs.push(RecordRef::PpToken(PpTokenId::from_index(
                input.pp_tokens_allocated.saturating_add(refs.len() as u32),
            )));
            position += 1;
        }
        // EOF token with empty spelling.
        records.push(RecordDraft {
            family: crate::ids::RecordFamily::PpToken,
            index: DraftRef(position),
        });
        bodies.push(G1DraftBody::PpToken(PpTokenRecord {
            kind: PpTokenKind::Eof,
            span: crate::ids::SpanId::from_index(
                input.spans_allocated.saturating_add(tokens.len() as u32),
            ),
            spelling: vec![],
        }));
        refs.push(RecordRef::PpToken(PpTokenId::from_index(
            input.pp_tokens_allocated.saturating_add(refs.len() as u32),
        )));
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

/// Scan comment-free bytes with maximal munch (M1 subset).
pub fn scan(input: &[u8]) -> Result<Vec<ScannedToken>, DiagnosticDraft> {
    let mut tokens = Vec::new();
    let mut cursor = 0usize;
    while cursor < input.len() {
        let byte = input[cursor];
        if byte.is_ascii_whitespace() {
            cursor += 1;
            continue;
        }
        if byte == b'"' || byte == b'\'' {
            return Err(DiagnosticDraft::unsupported(
                "string/character literal: deferred past M1",
            ));
        }
        if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = cursor;
            cursor += 1;
            while cursor < input.len()
                && (input[cursor].is_ascii_alphanumeric() || input[cursor] == b'_')
            {
                cursor += 1;
            }
            tokens.push(ScannedToken {
                kind: PpTokenKind::Identifier,
                start,
                end: cursor,
            });
            continue;
        }
        if byte.is_ascii_digit() {
            let start = cursor;
            cursor += 1;
            while cursor < input.len() {
                let next = input[cursor];
                let continues = next.is_ascii_alphanumeric()
                    || next == b'_'
                    || next == b'.'
                    || ((next == b'+' || next == b'-')
                        && matches!(input[cursor - 1], b'e' | b'E' | b'p' | b'P'));
                if continues {
                    cursor += 1;
                } else {
                    break;
                }
            }
            tokens.push(ScannedToken {
                kind: PpTokenKind::PpNumber,
                start,
                end: cursor,
            });
            continue;
        }
        if M1_PUNCTUATORS.contains(&byte) {
            tokens.push(ScannedToken {
                kind: PpTokenKind::Punctuator,
                start: cursor,
                end: cursor + 1,
            });
            cursor += 1;
            continue;
        }
        return Err(DiagnosticDraft::unsupported(
            "punctuator outside the M1 exercised subset",
        ));
    }
    Ok(tokens)
}
