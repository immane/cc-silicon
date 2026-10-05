// ============================================================================
// chips/preprocess/pp_emit.rs — T03 PP28 preprocessed-emit worker
// (candidate scope; T03:36, FinalPpTokens -> PpArtifact)
//
// Serializes the final pp-token stream (post-directive, post-expansion)
// into one map-mandatory `Preprocessed` artifact whose bytes re-lex to the
// same token stream. Directive lines are stripped, every token spelling is
// preserved byte-identical, and tokens are joined with whitespace so
// nothing glues (`+ +` never becomes `++`, adjacent strings stay two
// tokens). The output carries no `#line`/GNU markers; logical locations
// stay with PP23 (see the emission-policy note below).
//
// Narrow projection (task + state + pp-token refs + span records +
// referenced source bytes + primary source + prediction base + budget),
// pure `compute`, ZST, no closures, typed `Fail` only. One tick appends at
// most one artifact and completes `Record(Artifact)` or fails; there is no
// fan-out, so the frozen-join obligation is vacuous (same position as the
// PP25 worker in `pp_pragma.rs`). Deterministic: the bytes and the map are
// a pure function of the payload order plus the projected records.
// No I/O: the chip never reads files, samples the environment, or calls
// other chips.
//
// Emission policy (deliberate choices, not oversights):
// * Directive-strip: a `#`/`%:` punctuator with no expansion that opens a
//   physical line is a consumed directive introducer (PP05 rule), so it and
//   the rest of its physical line are dropped. A macro-generated `#`
//   (expansion present) is never an introducer and is emitted as an
//   ordinary token. `#pragma` directive lines strip the same way; the
//   `_Pragma` operator form is ordinary tokens and is preserved.
// * `#line`/GNU-marker policy, coordinated with PP23: PP28 emits NO line
//   markers. Marker accuracy needs the PP23 logical location, which does
//   not travel in the token payload; emitting markers from physical spans
//   would disagree with `__LINE__`/diagnostics after an include return.
//   PP23 owns the logical location at the wiring layer, and this artifact
//   stays marker-free so re-lexing cannot resurrect a stale directive.
// * Whitespace separation: tokens are joined with one space, or with one
//   newline when both neighbors keep primary/foreign source bytes showing
//   a line break between them. Separation is unconditional, so the gluing
//   hazards (`+ +` vs `++`, string adjacency, placemarker edges) cannot
//   occur by construction; newlines are layout-only and never semantic.
// * Spelling preservation: string/character literals and header names are
//   emitted byte-identical (escapes untouched); no unescaping, requoting,
//   or normalization is applied at this layer.
// * Multi-source boundary: the artifact carries the primary location map
//   only. The primary source is the first token's span source; bytes of
//   foreign-source tokens are emitted verbatim but their map entries
//   collapse zero-width onto the current primary boundary, and
//   expansion-reordered primary offsets clamp forward so the map stays
//   monotonic. Full cross-source provenance is deferred to the `/6`
//   provenance design (same primary-map-only position as rev 53).
// * Re-lex guarantee: the output is whitespace-separated spellings plus a
//   terminal newline, which the PP04 scanner reads back as the same token
//   kinds and spellings in the same order (directive lines excluded, since
//   consumed directives are not output tokens).
//
// Typed-failure (DEFECT) cases: unknown task, unexpected kind, and
// non-pp-token payload positions are protocol faults; missing token/span/
// source records, a non-`Running` task, an empty non-`Eof` spelling, an
// inverted span, and a span past its own source end are typed task errors;
// an over-budget payload is a typed config error; a map that still fails
// `check_map` (unreachable by construction) is a typed internal error.
// Malformed input is never silently accepted and never panics.
//
// Frozen registration (integrator-owned): `PP28_TASK_KIND` aliases
// `TaskKind::PREPROCESS_EMIT` (`PREPROCESS` local 35),
// `PP28_CHIP` is `crate::manifest::PP28_CHIP` (`ChipId(38)`), the
// kind-registry row lives in `TaskKindRegistry::pp_emit_slice()`,
// the stage-1 row in `STAGE_ASSIGNMENT`, the routed layer is 1, the
// `Artifacts fragments` allowlist row authorizes the single
// `Preprocessed` append, and the acceptance test is
// `compiler/tests/c31_emit.rs`.
// ============================================================================

use crate::bus::{ArtifactKind, ArtifactRecord, PpTokenKind, PpTokenRecord, SpanRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{ArtifactId, PpTokenId, RecordFamily, RecordRef, SourceId, SpanId, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP28_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// Frozen PP28 task kind (aliases `TaskKind::PREPROCESS_EMIT`,
/// frozen by the `/31` integrator; the local code is `PREPROCESS` 35,
/// first code after `/30`).
pub const PP28_TASK_KIND: TaskKind = TaskKind::PREPROCESS_EMIT;

/// Narrow projection for the preprocessed-emit computation.
#[derive(Clone, Debug)]
pub struct PpEmitInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token bodies in payload order.
    pub tokens: Vec<(PpTokenId, PpTokenRecord)>,
    /// Span records referenced by the payload tokens, in first-use order.
    pub spans: Vec<(SpanId, SpanRecord)>,
    /// Source bytes for every source referenced by the projected spans, in
    /// first-use order over `spans` (bounds checks, line-start detection,
    /// and separator newlines are per-source, never host-read).
    pub sources: Vec<(SourceId, Vec<u8>)>,
    /// Primary source: the first payload token's span source, owning the
    /// emitted location map.
    pub primary: SourceId,
    /// `artifacts` arena count at dispatch (single-append prediction base).
    pub artifacts_allocated: u32,
    /// Configured source/artifact byte budget (`limits.max_source_bytes`).
    pub max_source_bytes: u64,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pp_emit_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpEmitInput, DiagnosticDraft> {
    let record = match bus.arenas.tasks.get(task) {
        Ok(record) => record,
        Err(_) => {
            return Err(protocol_fault(format!(
                "emit of unknown task {}",
                task.index()
            )));
        }
    };
    if record.kind != PP28_TASK_KIND {
        return Err(protocol_fault(format!(
            "emit task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.is_empty() {
        return Err(protocol_fault(format!(
            "emit task {} payload must carry at least one pp-token",
            task.index()
        )));
    }
    let mut tokens = Vec::with_capacity(record.payload.refs.len());
    let mut index = 0usize;
    while index < record.payload.refs.len() {
        let id = match record.payload.refs[index] {
            RecordRef::PpToken(id) => id,
            _ => {
                return Err(protocol_fault(format!(
                    "emit task {} payload position {index} must be a pp-token",
                    task.index()
                )));
            }
        };
        match bus.arenas.pp_tokens.get(id) {
            Ok(found) => tokens.push((id, found.clone())),
            Err(_) => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("emit reads missing pp-token {}", id.index()),
                ));
            }
        }
        index += 1;
    }
    let mut spans: Vec<(SpanId, SpanRecord)> = Vec::new();
    let mut cursor = 0usize;
    while cursor < tokens.len() {
        let want = tokens[cursor].1.span;
        if !span_is_projected(&spans, want) {
            match bus.arenas.spans.get(want) {
                Ok(found) => spans.push((want, *found)),
                Err(_) => {
                    return Err(DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("emit reads missing span {}", want.index()),
                    ));
                }
            }
        }
        cursor += 1;
    }
    let first_span = match tokens.first() {
        Some(found) => found.1.span,
        None => {
            return Err(protocol_fault(format!(
                "emit task {} payload produced no tokens",
                task.index()
            )));
        }
    };
    let mut primary: Option<SourceId> = None;
    let mut scan = 0usize;
    while scan < spans.len() {
        if spans[scan].0 == first_span {
            primary = Some(spans[scan].1.source);
        }
        scan += 1;
    }
    let primary = match primary {
        Some(found) => found,
        None => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("emit reads unprojected span {}", first_span.index()),
            ));
        }
    };
    let mut sources: Vec<(SourceId, Vec<u8>)> = Vec::new();
    let mut position = 0usize;
    while position < spans.len() {
        let want = spans[position].1.source;
        if !source_is_projected(&sources, want) {
            match bus.arenas.sources.get(want) {
                Ok(found) => sources.push((want, found.bytes.clone())),
                Err(_) => {
                    return Err(DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("emit reads missing source {}", want.index()),
                    ));
                }
            }
        }
        position += 1;
    }
    Ok(PpEmitInput {
        task,
        state: record.state.clone(),
        tokens,
        spans,
        sources,
        primary,
        artifacts_allocated: bus.arenas.artifacts.allocated(),
        max_source_bytes: bus.limits().max_source_bytes,
    })
}

/// True when `id` already has a projected span record.
fn span_is_projected(spans: &[(SpanId, SpanRecord)], id: SpanId) -> bool {
    let mut index = 0usize;
    while index < spans.len() {
        if spans[index].0 == id {
            return true;
        }
        index += 1;
    }
    false
}

/// True when `id` already has projected source bytes.
fn source_is_projected(sources: &[(SourceId, Vec<u8>)], id: SourceId) -> bool {
    let mut index = 0usize;
    while index < sources.len() {
        if sources[index].0 == id {
            return true;
        }
        index += 1;
    }
    false
}

/// The T03 preprocessed-emit worker (PP28 candidate scope).
pub struct PpEmitChip;

impl Worker for PpEmitChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP28_CHIP,
            chip_name: "PpEmitChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![PP28_TASK_KIND],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Sources, "spans"),
                FieldPath::new(StoreId::Sources, "bytes"),
                FieldPath::new(StoreId::Artifacts, "fragments"),
                FieldPath::new(StoreId::Config, "limits"),
            ],
            writes: vec![FieldPath::new(StoreId::Artifacts, "fragments")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c31_emit.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pp_emit_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl PpEmitChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &PpEmitInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("emit task {} is not running", input.task.index()),
                ),
            )];
        }
        let emitted =
            match emit_preprocessed(&input.tokens, &input.spans, &input.sources, input.primary) {
                Ok(output) => output,
                Err(diagnostic) => return vec![fail(input.task, diagnostic)],
            };
        if emitted.bytes.len() as u64 > input.max_source_bytes {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Config, 1),
                    format!(
                        "preprocessed artifact needs {} bytes, budget is {}",
                        emitted.bytes.len(),
                        input.max_source_bytes
                    ),
                ),
            )];
        }
        let primary_len = match primary_source_len(&input.sources, input.primary) {
            Some(len) => len,
            None => {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        "emit reads unprojected primary source",
                    ),
                )];
            }
        };
        let record = ArtifactRecord {
            kind: ArtifactKind::Preprocessed,
            source: Some(input.primary),
            bytes: emitted.bytes,
            raw_offsets: emitted.raw_offsets,
        };
        if record.check_map(primary_len).is_err() {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Internal, 1),
                    "emit produced an invalid location map",
                ),
            )];
        }
        let predicted = ArtifactId::from_index(input.artifacts_allocated);
        vec![
            Proposal::AppendRecords {
                task: input.task,
                batch: AppendBatch {
                    records: vec![RecordDraft {
                        family: RecordFamily::Artifact,
                        index: DraftRef(0),
                    }],
                    bodies: vec![G1DraftBody::Artifact(record)],
                },
            },
            Proposal::Complete {
                task: input.task,
                value: ResultValue::Record(RecordRef::Artifact(predicted)),
            },
        ]
    }
}

/// Emitted preprocessed bytes plus the output-boundary to primary-source
/// boundary map (`raw_offsets.len() == bytes.len()+1`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmittedPreprocessed {
    /// Re-lexable preprocessed bytes (whitespace-separated spellings plus
    /// a terminal newline).
    pub bytes: Vec<u8>,
    /// Output-boundary to primary-source-boundary map (first `0`,
    /// monotonic nondecreasing, last `<=` primary source length).
    pub raw_offsets: Vec<u64>,
}

/// Serialize the final pp-token stream to re-lexable preprocessed bytes.
///
/// Drops `Eof` tokens and consumed directive lines (an unexpanded `#`/`%:`
/// opening a physical line, plus the rest of that line), emits every other
/// token spelling byte-identical separated by one space (one newline when
/// both neighbors show a line break in their shared source bytes), and
/// closes with a terminal newline. Each output byte maps to a primary
/// boundary: token bytes map into their span (clamped forward so
/// expansion-reordered spans stay monotonic), while separators and
/// foreign-source token bytes collapse zero-width onto the running
/// boundary. The map therefore satisfies `check_map` by construction.
pub fn emit_preprocessed(
    tokens: &[(PpTokenId, PpTokenRecord)],
    spans: &[(SpanId, SpanRecord)],
    sources: &[(SourceId, Vec<u8>)],
    primary: SourceId,
) -> Result<EmittedPreprocessed, DiagnosticDraft> {
    let primary_len = match primary_source_len(sources, primary) {
        Some(len) => len,
        None => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "emit reads unprojected primary source",
            ));
        }
    };
    let mut out: Vec<u8> = Vec::new();
    let mut map: Vec<u64> = Vec::new();
    map.push(0);
    let mut prev: u64 = 0;
    let mut first = true;
    let mut prev_source: Option<SourceId> = None;
    let mut prev_end: u64 = 0;
    let mut cursor = 0usize;
    while cursor < tokens.len() {
        let span = match find_span(spans, tokens[cursor].1.span) {
            Some(found) => found,
            None => {
                return Err(invalid(format!(
                    "emit reads unprojected span {}",
                    tokens[cursor].1.span.index()
                )));
            }
        };
        if tokens[cursor].1.kind == PpTokenKind::Eof {
            cursor += 1;
            continue;
        }
        if tokens[cursor].1.spelling.is_empty() {
            return Err(invalid("emit reads a pp-token with empty spelling"));
        }
        if is_hash_token(&tokens[cursor].1)
            && span.expansion.is_none()
            && source_is_projected(sources, span.source)
            && is_line_start(source_bytes_of(sources, span.source), span.start)
        {
            let hash_start = span.start;
            let hash_source = span.source;
            cursor += 1;
            while cursor < tokens.len() {
                if tokens[cursor].1.kind == PpTokenKind::Eof {
                    break;
                }
                let next = match find_span(spans, tokens[cursor].1.span) {
                    Some(found) => found,
                    None => {
                        return Err(invalid(format!(
                            "emit reads unprojected span {}",
                            tokens[cursor].1.span.index()
                        )));
                    }
                };
                if next.source != hash_source {
                    break;
                }
                if has_line_break(
                    source_bytes_of(sources, hash_source),
                    hash_start,
                    next.start,
                ) {
                    break;
                }
                cursor += 1;
            }
            continue;
        }
        let own_len = match source_len_of(sources, span.source) {
            Some(len) => len,
            None => {
                return Err(invalid(format!(
                    "emit reads a span of unprojected source {}",
                    span.source.index()
                )));
            }
        };
        if span.start > span.end {
            return Err(invalid("emit reads an inverted span"));
        }
        if span.end > own_len {
            return Err(invalid("emit reads a span past its source end"));
        }
        if first {
            first = false;
        } else {
            let mut newline = false;
            if let Some(previous) = prev_source {
                if previous == span.source
                    && has_line_break(source_bytes_of(sources, span.source), prev_end, span.start)
                {
                    newline = true;
                }
            }
            if newline {
                out.push(b'\n');
            } else {
                out.push(b' ');
            }
            map.push(prev);
        }
        let spelling = &tokens[cursor].1.spelling;
        let mut index = 0usize;
        while index < spelling.len() {
            let mut off = prev;
            if span.source == primary {
                let mut raw = span.start.saturating_add(index as u64);
                if raw > primary_len {
                    raw = primary_len;
                }
                if raw > prev {
                    off = raw;
                }
            }
            out.push(spelling[index]);
            map.push(off);
            prev = off;
            index += 1;
        }
        prev_source = Some(span.source);
        prev_end = span.end;
        cursor += 1;
    }
    out.push(b'\n');
    if first {
        map.push(0);
    } else {
        map.push(prev);
    }
    debug_assert_eq!(map.len() as u64, out.len() as u64 + 1);
    debug_assert_eq!(map.first(), Some(&0));
    Ok(EmittedPreprocessed {
        bytes: out,
        raw_offsets: map,
    })
}

/// Find a projected span record by ID.
fn find_span(spans: &[(SpanId, SpanRecord)], id: SpanId) -> Option<SpanRecord> {
    let mut index = 0usize;
    while index < spans.len() {
        if spans[index].0 == id {
            return Some(spans[index].1);
        }
        index += 1;
    }
    None
}

/// Borrow projected bytes for a source known to be projected.
///
/// Callers check [`source_is_projected`] (directive strip) or resolve
/// lengths through [`source_len_of`] (bounds check) first, so the fallback
/// below is unreachable in the projected configurations; it exists only to
/// keep the accessor total.
fn source_bytes_of(sources: &[(SourceId, Vec<u8>)], id: SourceId) -> &[u8] {
    let mut index = 0usize;
    while index < sources.len() {
        if sources[index].0 == id {
            return sources[index].1.as_slice();
        }
        index += 1;
    }
    &[]
}

/// Length of a projected source, if present.
fn source_len_of(sources: &[(SourceId, Vec<u8>)], id: SourceId) -> Option<u64> {
    let mut index = 0usize;
    while index < sources.len() {
        if sources[index].0 == id {
            return Some(sources[index].1.len() as u64);
        }
        index += 1;
    }
    None
}

/// Length of the primary source, if projected.
fn primary_source_len(sources: &[(SourceId, Vec<u8>)], primary: SourceId) -> Option<u64> {
    source_len_of(sources, primary)
}

/// Clamp a raw offset to a valid index into `source`.
fn clamp_index(source: &[u8], off: u64) -> usize {
    if off > source.len() as u64 {
        source.len()
    } else {
        off as usize
    }
}

/// True when `off` opens a physical line: every byte back to the previous
/// newline (or the input start) is a space or tab.
fn is_line_start(source: &[u8], off: u64) -> bool {
    let mut pos = clamp_index(source, off);
    while pos > 0 {
        let byte = source[pos - 1];
        if byte == b'\n' {
            return true;
        }
        if byte != b' ' && byte != b'\t' {
            return false;
        }
        pos -= 1;
    }
    true
}

/// True when the half-open range between `a` and `b` (either order)
/// contains a newline.
fn has_line_break(source: &[u8], a: u64, b: u64) -> bool {
    let mut low = clamp_index(source, source_min(a, b));
    let high = clamp_index(source, source_max(a, b));
    while low < high {
        if source[low] == b'\n' {
            return true;
        }
        low += 1;
    }
    false
}

/// Minimum of two raw offsets.
fn source_min(a: u64, b: u64) -> u64 {
    if a < b {
        a
    } else {
        b
    }
}

/// Maximum of two raw offsets.
fn source_max(a: u64, b: u64) -> u64 {
    if a > b {
        a
    } else {
        b
    }
}

/// True when the record is a directive introducer (`#` or `%:`).
fn is_hash_token(record: &PpTokenRecord) -> bool {
    if record.kind != PpTokenKind::Punctuator {
        return false;
    }
    if record.spelling.as_slice() == b"#" {
        return true;
    }
    record.spelling.as_slice() == b"%:"
}

/// Typed task error for malformed emit input.
fn invalid(message: impl Into<String>) -> DiagnosticDraft {
    DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Task, 4), message)
}
