// ============================================================================
// chips/preprocess/pp_enter.rs — T03 PP18 include-enter worker
// (Wave 2 slice 16, `/25`)
//
// Reads one include-enter task (payload: full-stream pp-token refs plus
// exactly one trailing `Source` ref naming the resolved header), groups the
// stream into raw lines with the `/21` walk-back predicate (owned chip-local
// copy of the PP05 grouping shape), and splices exactly one level of header
// tokens into the stream: every `#include` line whose header resolves (per
// the `/25` §2 path policy, owned chip-local copy) to the payload's trailing
// source is replaced by that source's scanned token refs (header EOF
// excluded — the outer stream keeps its single EOF). Non-`#include`
// directive lines and non-directive lines pass through verbatim; nested
// `#include` lines inside spliced header content pass through verbatim
// (single-pass by construction; control loops own nesting). Completes with
// `Records(stitched)` in stream order. No writes, no children, no joins.
// ============================================================================

use crate::bus::{PpTokenKind, PpTokenRecord, SpanRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{PpTokenId, RecordRef, SourceId, SpanId, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP18_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

/// Narrow projection for the include-enter computation.
#[derive(Clone, Debug)]
pub struct PpIncludeEnterInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Stream pp-token bodies in payload order (trailing `Source` excluded).
    pub tokens: Vec<(PpTokenId, PpTokenRecord)>,
    /// Span records referenced by the stream tokens, in first-use order.
    pub spans: Vec<(SpanId, SpanRecord)>,
    /// Source byte strings referenced by those spans, in first-use order.
    pub sources: Vec<(SourceId, Vec<u8>)>,
    /// Resolved header source (the payload's trailing `Source` ref).
    pub header: SourceId,
    /// Committed pp-tokens whose span source equals the header, ascending ID.
    pub header_tokens: Vec<(PpTokenId, PpTokenRecord)>,
    /// Committed source names (`(source, name bytes)`), ascending source ID.
    pub names: Vec<(SourceId, Vec<u8>)>,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pp_include_enter_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpIncludeEnterInput, DiagnosticDraft> {
    let record =
        bus.arenas.tasks.get(task).map_err(|_| {
            protocol_fault(format!("include-enter of unknown task {}", task.index()))
        })?;
    if record.kind != TaskKind::PREPROCESS_INCLUDE_ENTER {
        return Err(protocol_fault(format!(
            "include-enter task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() < 2 {
        return Err(protocol_fault(format!(
            "include-enter task {} payload must carry stream pp-token refs plus one trailing source",
            task.index()
        )));
    }
    let last = record.payload.refs.len() - 1;
    let header = match record.payload.refs[last] {
        RecordRef::Source(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "include-enter task {} payload must end in exactly one source ref",
                task.index()
            )));
        }
    };
    let mut token_ids = Vec::with_capacity(last);
    for reference in &record.payload.refs[..last] {
        match reference {
            RecordRef::PpToken(id) => token_ids.push(*id),
            _ => {
                return Err(protocol_fault(format!(
                    "include-enter task {} payload must be stream pp-token refs plus one trailing source",
                    task.index()
                )));
            }
        }
    }
    match bus.arenas.sources.get(header) {
        Ok(_) => {}
        Err(_) => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("include-enter reads missing source {}", header.index()),
            ));
        }
    }
    let mut tokens = Vec::with_capacity(token_ids.len());
    for id in token_ids {
        match bus.arenas.pp_tokens.get(id) {
            Ok(found) => tokens.push((id, found.clone())),
            Err(_) => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("include-enter reads missing pp-token {}", id.index()),
                ));
            }
        }
    }
    let mut spans: Vec<(SpanId, SpanRecord)> = Vec::new();
    for (_, token) in &tokens {
        if !span_is_projected(&spans, token.span) {
            match bus.arenas.spans.get(token.span) {
                Ok(found) => spans.push((token.span, *found)),
                Err(_) => {
                    return Err(DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("include-enter reads missing span {}", token.span.index()),
                    ));
                }
            }
        }
    }
    let mut sources: Vec<(SourceId, Vec<u8>)> = Vec::new();
    for (_, span) in &spans {
        if !source_is_projected(&sources, span.source) {
            match bus.arenas.sources.get(span.source) {
                Ok(found) => sources.push((span.source, found.bytes.clone())),
                Err(_) => {
                    return Err(DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("include-enter reads missing source {}", span.source.index()),
                    ));
                }
            }
        }
    }
    let mut names: Vec<(SourceId, Vec<u8>)> = Vec::new();
    for (id, source) in bus.arenas.sources.iter() {
        match bus.intern.get(source.name) {
            Ok(bytes) => names.push((id, bytes.to_vec())),
            Err(_) => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("include-enter reads missing name for source {}", id.index()),
                ));
            }
        }
    }
    let mut header_tokens: Vec<(PpTokenId, PpTokenRecord)> = Vec::new();
    for (id, body) in bus.arenas.pp_tokens.iter() {
        let span = match bus.arenas.spans.get(body.span) {
            Ok(found) => *found,
            Err(_) => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "include-enter reads pp-token {} with missing span {}",
                        id.index(),
                        body.span.index()
                    ),
                ));
            }
        };
        if span.source == header {
            header_tokens.push((id, body.clone()));
        }
    }
    Ok(PpIncludeEnterInput {
        task,
        state: record.state.clone(),
        tokens,
        spans,
        sources,
        header,
        header_tokens,
        names,
    })
}

/// True when `id` already has a projected span record.
fn span_is_projected(spans: &[(SpanId, SpanRecord)], id: SpanId) -> bool {
    for (known, _) in spans {
        if *known == id {
            return true;
        }
    }
    false
}

/// True when `id` already has projected source bytes.
fn source_is_projected(sources: &[(SourceId, Vec<u8>)], id: SourceId) -> bool {
    for (known, _) in sources {
        if *known == id {
            return true;
        }
    }
    false
}

/// The T03 include-enter worker (PP18 slice scope).
pub struct PpIncludeEnterChip;

impl Worker for PpIncludeEnterChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP18_CHIP,
            chip_name: "PpIncludeEnterChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![TaskKind::PREPROCESS_INCLUDE_ENTER],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Sources, "bytes"),
                FieldPath::new(StoreId::Sources, "spans"),
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
        let input = match project_pp_include_enter_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

/// One raw source line's worth of stream tokens, in stream order.
struct RawLine {
    /// Owning source.
    source: SourceId,
    /// Zero-based line index (count of `\n` before the line start).
    line: usize,
    /// Indices into the input token vector, in order.
    members: Vec<usize>,
}

impl PpIncludeEnterChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// Groups the stream into raw lines, replaces every `#include` line
    /// whose header resolves to the payload's trailing source with that
    /// source's scanned token refs, passes every other line through
    /// verbatim, and completes with the stitched stream plus the single
    /// stream EOF. Every failure path is a `Fail` proposal, never a panic.
    pub fn compute(&self, input: &PpIncludeEnterInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("include-enter task {} is not running", input.task.index()),
                ),
            )];
        }
        if input.header_tokens.is_empty() {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "header tokens not scanned; scan the header source first",
                ),
            )];
        }
        let lines = match group_raw_lines(input) {
            Ok(lines) => lines,
            Err(diagnostic) => return vec![fail(input.task, diagnostic)],
        };
        let mut stitched: Vec<RecordRef> = Vec::new();
        let mut eof: Option<RecordRef> = None;
        for line in &lines {
            for index in line.members.iter() {
                if input.tokens[*index].1.kind == PpTokenKind::Eof {
                    eof = Some(RecordRef::PpToken(input.tokens[*index].0));
                }
            }
            if !raw_line_is_directive(input, line) {
                push_line_refs(input, line, &mut stitched);
                continue;
            }
            if line.members.len() == 1 {
                push_line_refs(input, line, &mut stitched);
                continue;
            }
            let name = match directive_name(input, line) {
                Ok(name) => name,
                Err(diagnostic) => return vec![fail(input.task, diagnostic)],
            };
            if name.as_slice() == b"include_next" {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::unsupported(
                        "unsupported directive `include_next`: deferred past this slice (T12 owns `include_next`)",
                    ),
                )];
            }
            if name.as_slice() != b"include" {
                push_line_refs(input, line, &mut stitched);
                continue;
            }
            let spelling = match include_header_spelling(input, line) {
                Ok(spelling) => spelling,
                Err(diagnostic) => return vec![fail(input.task, diagnostic)],
            };
            let resolved = match resolve_header(&input.names, &spelling) {
                Ok(resolved) => resolved,
                Err(diagnostic) => return vec![fail(input.task, diagnostic)],
            };
            if resolved != input.header {
                return vec![fail(
                    input.task,
                    protocol_fault(format!(
                        "include-enter task {} resolved header source {} but the payload carries {}",
                        input.task.index(),
                        resolved.index(),
                        input.header.index()
                    )),
                )];
            }
            push_header_refs(input, &mut stitched);
        }
        match eof {
            Some(reference) => {
                stitched.push(reference);
                vec![Proposal::Complete {
                    task: input.task,
                    value: ResultValue::Records(stitched),
                }]
            }
            None => vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "include-enter reads a stream with no EOF token",
                ),
            )],
        }
    }
}

/// Append one line's token refs, in order (the EOF token is excluded: it is
/// always appended once at the end, whatever line it sits on).
fn push_line_refs(input: &PpIncludeEnterInput, line: &RawLine, kept: &mut Vec<RecordRef>) {
    for index in line.members.iter() {
        if input.tokens[*index].1.kind == PpTokenKind::Eof {
            continue;
        }
        kept.push(RecordRef::PpToken(input.tokens[*index].0));
    }
}

/// Append the resolved header's scanned token refs, in ascending-ID order
/// (the header's own EOF is excluded: the outer stream keeps its single
/// EOF; nested `#include` lines pass through verbatim, single-pass).
fn push_header_refs(input: &PpIncludeEnterInput, kept: &mut Vec<RecordRef>) {
    for (id, body) in &input.header_tokens {
        if body.kind == PpTokenKind::Eof {
            continue;
        }
        kept.push(RecordRef::PpToken(*id));
    }
}

/// Header spelling of one `#include` line: the line's non-EOF members must
/// be exactly `#`, `include`, and one `HeaderName` token. A missing or
/// trailing token is malformed; a non-literal token in the header position
/// (macro-generated names) is explicit `Unsupported` per the `/25` §2 rule.
fn include_header_spelling(
    input: &PpIncludeEnterInput,
    line: &RawLine,
) -> Result<Vec<u8>, DiagnosticDraft> {
    let err = |detail: &str| {
        DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Task, 4), detail.to_string())
    };
    let mut positions: Vec<usize> = Vec::new();
    for index in line.members.iter() {
        if input.tokens[*index].1.kind == PpTokenKind::Eof {
            continue;
        }
        positions.push(*index);
    }
    if positions.len() < 3 {
        return Err(err(
            "malformed `#include` line: expected a header name after `include`",
        ));
    }
    if positions.len() > 3 {
        return Err(err(
            "malformed `#include` line: trailing tokens after the header name",
        ));
    }
    let body = &input.tokens[positions[2]].1;
    if body.kind != PpTokenKind::HeaderName {
        return Err(DiagnosticDraft::unsupported(
            "macro-generated header name after `#include`: expansion interplay is deferred past this slice (PP09/PP17 own expansion)",
        ));
    }
    strip_header_delimiters(&body.spelling)
}

/// Strip the `<...>` or `"..."` delimiters from a header-name spelling,
/// yielding the lookup spelling. Missing delimiters or an empty inner
/// spelling is malformed per the `/25` §2 rule.
fn strip_header_delimiters(spelling: &[u8]) -> Result<Vec<u8>, DiagnosticDraft> {
    let err = |detail: &str| {
        DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Task, 4), detail.to_string())
    };
    if spelling.len() >= 2 {
        let first = spelling[0];
        let last = spelling[spelling.len() - 1];
        if (first == b'<' && last == b'>') || (first == b'"' && last == b'"') {
            let mut inner = Vec::new();
            for byte in &spelling[1..spelling.len() - 1] {
                inner.push(*byte);
            }
            if inner.is_empty() {
                return Err(err("malformed `#include`: empty header name"));
            }
            return Ok(inner);
        }
    }
    Err(err("malformed `#include`: header name without delimiters"))
}

/// Resolve a header spelling over committed source names (`/25` §2 policy,
/// chip-local copy): exact full-name match first, else basename suffix
/// match (bytes after the last `/` in each name). First match wins;
/// multiple basename matches fail ambiguous (never guesses); no match
/// fails not-loaded.
fn resolve_header(
    names: &[(SourceId, Vec<u8>)],
    spelling: &[u8],
) -> Result<SourceId, DiagnosticDraft> {
    for (id, name) in names {
        if name.as_slice() == spelling {
            return Ok(*id);
        }
    }
    let mut matches = 0u32;
    let mut first: Option<SourceId> = None;
    for (id, name) in names {
        if basename(name.as_slice()) == spelling {
            matches += 1;
            if first.is_none() {
                first = Some(*id);
            }
        }
    }
    let display = String::from_utf8_lossy(spelling);
    if matches == 1 {
        match first {
            Some(id) => Ok(id),
            None => Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "include-enter resolved an empty header match set",
            )),
        }
    } else if matches > 1 {
        Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("ambiguous header `{display}`: {matches} matches"),
        ))
    } else {
        Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!(
                "header `{display}` is not loaded ({} sources searched)",
                names.len()
            ),
        ))
    }
}

/// Basename of a source name: bytes after the last `/`, or the whole name
/// when it holds no `/`.
fn basename(name: &[u8]) -> &[u8] {
    let mut start = 0usize;
    let mut cursor = 0usize;
    while cursor < name.len() {
        if name[cursor] == b'/' {
            start = cursor + 1;
        }
        cursor += 1;
    }
    &name[start..]
}

/// Group stream tokens into raw lines: consecutive tokens sharing the same
/// `(source, line)` key form one line, in stream order.
fn group_raw_lines(input: &PpIncludeEnterInput) -> Result<Vec<RawLine>, DiagnosticDraft> {
    let mut lines: Vec<RawLine> = Vec::new();
    for index in 0..input.tokens.len() {
        let span = match find_span(input, input.tokens[index].1.span) {
            Some(found) => found,
            None => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "include-enter reads unprojected span {}",
                        input.tokens[index].1.span.index()
                    ),
                ));
            }
        };
        let bytes = match find_source(input, span.source) {
            Some(found) => found,
            None => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "include-enter reads unprojected source {}",
                        span.source.index()
                    ),
                ));
            }
        };
        let line = line_index(bytes, span.start);
        let extends = match lines.last() {
            Some(open) => open.source == span.source && open.line == line,
            None => false,
        };
        if extends {
            match lines.last_mut() {
                Some(open) => open.members.push(index),
                None => {
                    return Err(DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        "include-enter groups an empty line set",
                    ));
                }
            }
        } else {
            lines.push(RawLine {
                source: span.source,
                line,
                members: vec![index],
            });
        }
    }
    Ok(lines)
}

/// Look up a projected span record by ID.
fn find_span(input: &PpIncludeEnterInput, id: SpanId) -> Option<SpanRecord> {
    for (known, record) in &input.spans {
        if *known == id {
            return Some(*record);
        }
    }
    None
}

/// Look up projected source bytes by ID.
fn find_source(input: &PpIncludeEnterInput, id: SourceId) -> Option<&[u8]> {
    for (known, bytes) in &input.sources {
        if *known == id {
            return Some(bytes.as_slice());
        }
    }
    None
}

/// Zero-based line index of a raw offset: the count of `\n` bytes strictly
/// before it (offsets past the end clamp to the source end, never panic).
fn line_index(bytes: &[u8], offset: u64) -> usize {
    let mut line = 0usize;
    let mut cursor = 0usize;
    let end = (offset as usize).min(bytes.len());
    while cursor < end {
        if bytes[cursor] == b'\n' {
            line += 1;
        }
        cursor += 1;
    }
    line
}

/// True when the record is a directive introducer (`#` or `%:`).
fn is_hash_token(record: &PpTokenRecord) -> bool {
    record.kind == PpTokenKind::Punctuator
        && (record.spelling.as_slice() == b"#" || record.spelling.as_slice() == b"%:")
}

/// Directive-line test (`/21` predicate, chip-local copy): the first token
/// is `#`/`%:` AND the raw walk-back from its offset passes. Missing
/// span/source data falls back to non-directive here, but that path is
/// unreachable: `group_raw_lines` above already fails loudly on the same
/// lookups for every token.
fn raw_line_is_directive(input: &PpIncludeEnterInput, line: &RawLine) -> bool {
    let first = match line.members.first() {
        Some(index) => *index,
        None => return false,
    };
    if !is_hash_token(&input.tokens[first].1) {
        return false;
    }
    let span = match find_span(input, input.tokens[first].1.span) {
        Some(found) => found,
        None => return false,
    };
    let bytes = match find_source(input, span.source) {
        Some(found) => found,
        None => return false,
    };
    directive_hash_passes(bytes, span.start)
}

/// Raw walk-back (`/21` frozen, chip-local copy): from the `#` offset skip
/// `[ \t]`; input start or `\n` accepts; a same-line `/*...*/` is skipped
/// and walking continues; anything else (including `//`, mid-line bytes,
/// unterminated or multi-line comments) rejects.
fn directive_hash_passes(bytes: &[u8], hash_start: u64) -> bool {
    let mut cursor = (hash_start as usize).min(bytes.len());
    loop {
        while cursor > 0 && (bytes[cursor - 1] == b' ' || bytes[cursor - 1] == b'\t') {
            cursor -= 1;
        }
        if cursor == 0 {
            return true;
        }
        if cursor >= 2 && bytes[cursor - 2] == b'*' && bytes[cursor - 1] == b'/' {
            match walk_back_block_comment(bytes, cursor) {
                Some(open) => {
                    cursor = open;
                    continue;
                }
                None => return false,
            }
        }
        if bytes[cursor - 1] == b'\n' {
            return true;
        }
        return false;
    }
}

/// Walk back over one closing `*/` (`close_end` = first index past it) to
/// the matching same-line `/*`; returns the opener index, or `None` when no
/// opener exists or a newline sits inside (L1 conservative reject).
fn walk_back_block_comment(bytes: &[u8], close_end: usize) -> Option<usize> {
    let mut scan = close_end.saturating_sub(2);
    loop {
        if bytes.get(scan) == Some(&b'/') && bytes.get(scan + 1) == Some(&b'*') {
            for byte in &bytes[scan..close_end] {
                if *byte == b'\n' {
                    return None;
                }
            }
            return Some(scan);
        }
        if scan == 0 {
            return None;
        }
        scan -= 1;
    }
}

/// Directive name (`/21` frozen, chip-local copy): the next token when it is
/// an `Identifier`, else a malformed-line typed error.
fn directive_name(input: &PpIncludeEnterInput, line: &RawLine) -> Result<Vec<u8>, DiagnosticDraft> {
    if line.members.len() < 2 {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "malformed directive line: expected an identifier after `#`",
        ));
    }
    let record = &input.tokens[line.members[1]].1;
    if record.kind != PpTokenKind::Identifier {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "malformed directive line: expected an identifier after `#`",
        ));
    }
    Ok(record.spelling.clone())
}
