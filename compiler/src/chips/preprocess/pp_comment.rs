// ============================================================================
// chips/preprocess/pp_comment.rs — T03 PP03 comment-replace worker
// (Wave 2 slice 11, `/20`)
//
// Reads one committed `Spliced` artifact, replaces comments with whitespace
// (one space per comment, every logical newline preserved), and appends one
// `CommentFree` artifact with the composed map. `/20` scope: comment
// openers inside string/character literals and `#include` header names are
// literal bytes that pass through byte-identical (backslash escapes the
// next byte inside `"..."`/`'...'` and quoted headers; angle headers take
// no escapes); an unterminated literal/header fails with a typed
// diagnostic, and an unterminated block comment fails with the M1-NEG-01
// carrier shape. The `/16` rule rejecting literal-bearing inputs as
// `Unsupported` is SUPERSEDED by this slice.
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
/// output-to-input boundary map (`/20` full-scan contract, shared predicate
/// with PP04 per `PP_FULL_SCAN_SLICE.md` §§2–3). `//` runs to the newline
/// (preserved, one space); `/*` runs to the first `*/` with every inner
/// newline preserved (one space); EOF inside `/*` is the typed
/// M1-NEG-01 unterminated-comment failure. Comment openers inside
/// `"..."`/`'...'` literals and `#include` header names are literal bytes
/// passing through byte-identical with per-byte identity map entries: a
/// backslash escapes the next byte (any) inside literals and quoted
/// headers, angle headers take no escapes/comments, and a newline/EOF
/// before the close is a typed (`Task`, 4) unterminated failure. The
/// include prefix is lexical and line-start only (`[ \t]*`, `#` or `%:`,
/// `[ \t]*`, `include` plus an identifier boundary, then spaces/tabs and
/// block comments skipped — `//` or a newline aborts it, leaving the
/// `//` a normal comment); `include_next` and non-line-start `#` never
/// open a header context.
pub fn replace_comments(input: &[u8]) -> Result<(Vec<u8>, Vec<u64>), DiagnosticDraft> {
    let mut out: Vec<u8> = Vec::with_capacity(input.len());
    let mut mid: Vec<u64> = Vec::with_capacity(input.len() + 1);
    mid.push(0);
    let mut cursor = 0usize;
    let mut pending_header: Option<(usize, HeaderKind)> = None;
    while cursor < input.len() {
        if let Some((header_at, header_kind)) = pending_header {
            if cursor == header_at {
                pending_header = None;
                consume_header(input, &mut cursor, &mut out, &mut mid, header_kind)?;
                continue;
            }
            if cursor > header_at {
                pending_header = None;
            }
        }
        let byte = input[cursor];
        if byte == b'"' || byte == b'\'' {
            consume_quoted(input, &mut cursor, &mut out, &mut mid, byte)?;
            continue;
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
        if is_hash_or_percent_colon_at(input, cursor)
            && is_line_start_blank(input, cursor)
            && pending_header.is_none()
        {
            pending_header = match_include_header(input, cursor);
        }
        out.push(byte);
        cursor += 1;
        mid.push(cursor as u64);
    }
    debug_assert_eq!(mid.len() as u64, out.len() as u64 + 1);
    Ok((out, mid))
}

/// Header-name flavor located by the include-prefix lookahead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HeaderKind {
    Angle,
    Quoted,
}

/// Emit one input byte with its per-byte identity map entry.
fn emit_byte(input: &[u8], cursor: &mut usize, out: &mut Vec<u8>, mid: &mut Vec<u64>) {
    out.push(input[*cursor]);
    *cursor += 1;
    mid.push(*cursor as u64);
}

/// True when the cursor holds `#`, or `%` followed by `:`.
fn is_hash_or_percent_colon_at(input: &[u8], pos: usize) -> bool {
    if input.get(pos) == Some(&b'#') {
        return true;
    }
    input.get(pos) == Some(&b'%') && input.get(pos + 1) == Some(&b':')
}

/// True when every byte since the previous newline (or input start) is a
/// space or tab, so a `#`/`%:` here may open an include prefix (§2).
fn is_line_start_blank(input: &[u8], pos: usize) -> bool {
    let mut back = pos;
    while back > 0 {
        back -= 1;
        if input[back] == b'\n' {
            return true;
        }
        if input[back] != b' ' && input[back] != b'\t' {
            return false;
        }
    }
    true
}

/// True for identifier bytes (`[A-Za-z0-9_]`), used for the `include`
/// trailing-boundary check (`include_next` must not match).
fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Skip `/*...*/` starting at `pos` (which must hold `/` then `*`);
/// returns the first index past `*/`, or `None` when unterminated (the
/// main loop re-reports that span as the typed block-comment failure).
fn skip_block_comment(input: &[u8], pos: usize) -> Option<usize> {
    let mut scan = pos + 2;
    while scan < input.len() {
        if input[scan] == b'*' && input.get(scan + 1) == Some(&b'/') {
            return Some(scan + 2);
        }
        scan += 1;
    }
    None
}

/// Lexical include-prefix lookahead (§2): from a line-start `#`/`%:` match
/// `include` with an identifier boundary, skip spaces/tabs and block
/// comments (`//` or a newline aborts: no header context), then report the
/// header start and flavor for `<`/`"` (anything else: no header context).
fn match_include_header(input: &[u8], pos: usize) -> Option<(usize, HeaderKind)> {
    let mut scan = pos;
    if input.get(scan) == Some(&b'#') {
        scan += 1;
    } else if input.get(scan) == Some(&b'%') && input.get(scan + 1) == Some(&b':') {
        scan += 2;
    } else {
        return None;
    }
    while input.get(scan) == Some(&b' ') || input.get(scan) == Some(&b'\t') {
        scan += 1;
    }
    let word: &[u8; 7] = b"include";
    let mut taken = 0usize;
    while taken < word.len() {
        if input.get(scan + taken) != Some(&word[taken]) {
            return None;
        }
        taken += 1;
    }
    scan += word.len();
    if matches!(input.get(scan), Some(next) if is_ident_byte(*next)) {
        return None;
    }
    loop {
        let byte = *input.get(scan)?;
        if byte == b' ' || byte == b'\t' {
            scan += 1;
            continue;
        }
        if byte == b'\n' {
            return None;
        }
        if byte == b'/' {
            if input.get(scan + 1) == Some(&b'*') {
                scan = skip_block_comment(input, scan)?;
                continue;
            }
            return None;
        }
        if byte == b'<' {
            return Some((scan, HeaderKind::Angle));
        }
        if byte == b'"' {
            return Some((scan, HeaderKind::Quoted));
        }
        return None;
    }
}

/// Consume one `"..."`/`'...'` literal starting at the opening quote,
/// emitting every byte verbatim: a backslash escapes the next byte (any),
/// the first unescaped matching quote closes, and a newline/EOF before
/// the close is a typed (`Task`, 4) failure.
fn consume_quoted(
    input: &[u8],
    cursor: &mut usize,
    out: &mut Vec<u8>,
    mid: &mut Vec<u64>,
    quote: u8,
) -> Result<(), DiagnosticDraft> {
    let unterminated = if quote == b'\'' {
        "unterminated character literal"
    } else {
        "unterminated string literal"
    };
    emit_byte(input, cursor, out, mid);
    loop {
        let byte = match input.get(*cursor) {
            None => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    unterminated,
                ));
            }
            Some(found) => *found,
        };
        if byte == b'\n' {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                unterminated,
            ));
        }
        if byte == b'\\' {
            emit_byte(input, cursor, out, mid);
            if input.get(*cursor).is_none() {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    unterminated,
                ));
            }
            emit_byte(input, cursor, out, mid);
            continue;
        }
        emit_byte(input, cursor, out, mid);
        if byte == quote {
            return Ok(());
        }
    }
}

/// Consume one header name starting at its opener, emitting every byte
/// verbatim: `<...>` runs to the first `>` with no escape or comment
/// processing, `"..."` reuses the escape-aware quote rule, and a
/// newline/EOF before the close is a typed (`Task`, 4) failure.
fn consume_header(
    input: &[u8],
    cursor: &mut usize,
    out: &mut Vec<u8>,
    mid: &mut Vec<u64>,
    kind: HeaderKind,
) -> Result<(), DiagnosticDraft> {
    if kind == HeaderKind::Quoted {
        return consume_quoted(input, cursor, out, mid, b'"');
    }
    emit_byte(input, cursor, out, mid);
    while *cursor < input.len() {
        let byte = input[*cursor];
        if byte == b'\n' {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "unterminated header name",
            ));
        }
        emit_byte(input, cursor, out, mid);
        if byte == b'>' {
            return Ok(());
        }
    }
    Err(DiagnosticDraft::error(
        DiagnosticCode::new(DiagGroup::Task, 4),
        "unterminated header name",
    ))
}
