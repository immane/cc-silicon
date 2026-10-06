// ============================================================================
// chips/preprocess/pp_scan.rs — T03 PP04 token-scan worker
// (Wave 2 slice 11, `/20`)
//
// Reads one committed `CommentFree` artifact, scans PP tokens with maximal
// munch, and appends one span per token (raw offsets remapped through the
// input map before any draft is formed) plus the tokens in order, closing
// with exactly one zero-width EOF. `/20` scope: identifiers, pp-numbers
// (digit-led or leading-dot start, with `e`/`E`/`p`/`P` sign continuation),
// the full C11 punctuator table (maximal munch, longest match first),
// escape-aware string/character literals (one token each), and `#include`
// header names (angle or quoted, one token each, spelling includes the
// delimiters). Lone `\`, `@`, `$`, backtick, non-ASCII bytes, and
// universal-character names are explicit `Unsupported` (never
// mis-tokenized); a newline/EOF before a literal/header close is a typed
// (`Task`, 4) failure. The `/16` subset rules (six punctuators, literal
// `Unsupported`) are SUPERSEDED by this slice; `M1_PUNCTUATORS` stays
// exported as frozen history only.
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

/// Full C11 multi-character punctuator spellings, longest match first
/// (`/20`; chip-local — `M1_PUNCTUATORS` stays exported as frozen history).
const FULL_PUNCTUATORS_MULTI: &[&[u8]] = &[
    b"%:%:", b"...", b"<<=", b">>=", b"<<", b">>", b"<=", b">=", b"==", b"!=", b"&&", b"||", b"++",
    b"--", b"->", b"*=", b"/=", b"%=", b"+=", b"-=", b"&=", b"^=", b"|=", b"##", b"<:", b":>",
    b"<%", b"%>", b"%:",
];

/// Full C11 single-character punctuator spellings (`/20`; chip-local).
const FULL_PUNCTUATORS_SINGLE: &[u8] = b"[](){}.&*+-~!/%<>^|?:;=,#";

/// Header-name flavor located by the include-prefix lookahead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HeaderKind {
    Angle,
    Quoted,
}

/// True for identifier bytes (`[A-Za-z0-9_]`), used for the `include`
/// trailing-boundary check (`include_next` must not match).
fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
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

/// Skip `/*...*/` starting at `pos` (which must hold `/` then `*`);
/// returns the first index past `*/`, or `None` when unterminated.
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

/// Maximal-munch match over the full C11 punctuator table: returns the
/// matched length (longest first), or `None` when no punctuator starts here.
fn match_punctuator_len(input: &[u8], pos: usize) -> Option<usize> {
    let rest = &input[pos..];
    for cand in FULL_PUNCTUATORS_MULTI {
        if rest.starts_with(cand) {
            return Some(cand.len());
        }
    }
    for single in FULL_PUNCTUATORS_SINGLE {
        if rest[0] == *single {
            return Some(1);
        }
    }
    None
}

/// Scan one escape-aware quoted segment starting at the opening quote and
/// return the first index past the close: a backslash escapes the next byte
/// (any, including newline), the first unescaped matching quote closes, and
/// a newline/EOF before the close is a typed (`Task`, 4) failure.
fn scan_quoted_end(
    input: &[u8],
    start: usize,
    quote: u8,
    unterminated: &str,
) -> Result<usize, DiagnosticDraft> {
    let mut cursor = start + 1;
    while cursor < input.len() {
        let byte = input[cursor];
        if byte == b'\\' {
            cursor += 1;
            if cursor >= input.len() {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    unterminated,
                ));
            }
            cursor += 1;
            continue;
        }
        if byte == b'\n' {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                unterminated,
            ));
        }
        cursor += 1;
        if byte == quote {
            return Ok(cursor);
        }
    }
    Err(DiagnosticDraft::error(
        DiagnosticCode::new(DiagGroup::Task, 4),
        unterminated,
    ))
}

/// Scan one angle-header segment starting at `<` and return the first index
/// past `>`: no escape or comment processing, and a newline/EOF before the
/// close is a typed (`Task`, 4) failure.
fn scan_angle_end(input: &[u8], start: usize) -> Result<usize, DiagnosticDraft> {
    let mut cursor = start + 1;
    while cursor < input.len() {
        let byte = input[cursor];
        if byte == b'\n' {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "unterminated header name",
            ));
        }
        cursor += 1;
        if byte == b'>' {
            return Ok(cursor);
        }
    }
    Err(DiagnosticDraft::error(
        DiagnosticCode::new(DiagGroup::Task, 4),
        "unterminated header name",
    ))
}

/// Scan comment-free bytes with maximal munch (`/20` full-scan contract,
/// shared predicate with PP03 per `PP_FULL_SCAN_SLICE.md` §§2+4).
/// Identifiers are unchanged; pp-numbers start on `[0-9]` or `.`+`[0-9]`
/// with the frozen continue rule; punctuators use the full C11 table
/// longest-match-first; string/char/header segments each become exactly one
/// token (header contexts come from the line-start include-prefix
/// lookahead); lone `\`, `@`, `$`, backtick, non-ASCII bytes, and anything
/// else outside the pp-token set are explicit `Unsupported`.
pub fn scan(input: &[u8]) -> Result<Vec<ScannedToken>, DiagnosticDraft> {
    let mut tokens = Vec::new();
    let mut cursor = 0usize;
    let mut pending_header: Option<(usize, HeaderKind)> = None;
    while cursor < input.len() {
        if let Some((header_at, _)) = pending_header {
            if cursor > header_at {
                pending_header = None;
            }
        }
        let byte = input[cursor];
        if byte.is_ascii_whitespace() {
            cursor += 1;
            continue;
        }
        if is_hash_or_percent_colon_at(input, cursor)
            && is_line_start_blank(input, cursor)
            && pending_header.is_none()
        {
            if let Some(found) = match_include_header(input, cursor) {
                pending_header = Some(found);
            }
        }
        if byte == b'"' {
            if matches!(pending_header, Some((at, HeaderKind::Quoted)) if at == cursor) {
                pending_header = None;
                let end = scan_quoted_end(input, cursor, b'"', "unterminated header name")?;
                tokens.push(ScannedToken {
                    kind: PpTokenKind::HeaderName,
                    start: cursor,
                    end,
                });
                cursor = end;
                continue;
            }
            let end = scan_quoted_end(input, cursor, b'"', "unterminated string literal")?;
            tokens.push(ScannedToken {
                kind: PpTokenKind::StringLiteral,
                start: cursor,
                end,
            });
            cursor = end;
            continue;
        }
        if byte == b'\'' {
            let end = scan_quoted_end(input, cursor, b'\'', "unterminated character literal")?;
            tokens.push(ScannedToken {
                kind: PpTokenKind::CharLiteral,
                start: cursor,
                end,
            });
            cursor = end;
            continue;
        }
        if byte == b'<' && matches!(pending_header, Some((at, HeaderKind::Angle)) if at == cursor) {
            pending_header = None;
            let end = scan_angle_end(input, cursor)?;
            tokens.push(ScannedToken {
                kind: PpTokenKind::HeaderName,
                start: cursor,
                end,
            });
            cursor = end;
            continue;
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
        if byte.is_ascii_digit()
            || (byte == b'.' && cursor + 1 < input.len() && input[cursor + 1].is_ascii_digit())
        {
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
        if let Some(len) = match_punctuator_len(input, cursor) {
            tokens.push(ScannedToken {
                kind: PpTokenKind::Punctuator,
                start: cursor,
                end: cursor + len,
            });
            cursor += len;
            continue;
        }
        if byte == b'\\' {
            return Err(DiagnosticDraft::unsupported(
                "lone backslash: universal-character names are deferred past this slice",
            ));
        }
        if byte == b'@' || byte == b'$' || byte == b'`' {
            return Err(DiagnosticDraft::unsupported(
                "character outside the C11 pp-token set",
            ));
        }
        if !byte.is_ascii() {
            return Err(DiagnosticDraft::unsupported(
                "non-ASCII byte: deferred past this slice",
            ));
        }
        return Err(DiagnosticDraft::unsupported(
            "byte outside the C11 pp-token set",
        ));
    }
    Ok(tokens)
}
