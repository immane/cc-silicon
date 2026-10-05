// ============================================================================
// chips/preprocess/pp_directive.rs — T03 PP05 directive-dispatch worker
// (Wave 2 slice 12, `/21`)
//
// Reads all committed pp-token refs, groups them into raw lines via token
// spans → source offsets (first token per raw line), and classifies
// directive lines exactly per `PP_DIRECTIVE_SLICE.md` §2: a line is a
// directive line iff its first pp-token is `#` or `%:` AND the raw
// walk-back over the committed source bytes passes (spaces/tabs skipped, a
// `\n` or input start accepts, a same-line `/*...*/` is skipped and walking
// continues, anything else rejects — so `//`-killed and mid-line `#` stay
// dead). Zero directive lines complete with `Ack`. A malformed line (`#` +
// non-identifier) fails with a typed error. Any non-`error` directive name
// fails fast as explicit `Unsupported` naming the deferred owner (the §2
// taxonomy is the deliverable where bodies are deferred). Otherwise one
// PP26 child per `#error` line is enqueued (payload = line refs after the
// `#`) and awaited (SE_BIN enqueue/await-all pattern); on resume any child
// `Failed` fails with `"{n} error directive(s) rejected; first: {msg}"`
// read from the committed diagnostics. No writes.
// ============================================================================

use crate::bus::{PpTokenKind, PpTokenRecord, SpanRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{PpTokenId, RecordRef, SourceId, SpanId, TaskId};
use crate::manifest::{
    BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP05_CHIP, PP26_CHIP,
};
use crate::task::{
    ChildRef, Payload, Proposal, ResultValue, StoreId, TaskDraft, TaskGroup, TaskKind, TaskState,
};

/// Narrow projection for the directive-dispatch computation.
#[derive(Clone, Debug)]
pub struct PpDirectiveInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// All committed pp-tokens in ID order.
    pub tokens: Vec<(PpTokenId, PpTokenRecord)>,
    /// Span records referenced by the payload tokens, in first-use order.
    pub spans: Vec<(SpanId, SpanRecord)>,
    /// Source byte strings referenced by those spans, in first-use order.
    pub sources: Vec<(SourceId, Vec<u8>)>,
    /// Already-enqueued children (tasks with `parent == Some(task)`, in
    /// `TaskId` order) with their states; the message is the committed
    /// diagnostic message for `Failed` children.
    pub children: Vec<(TaskId, TaskState, Option<String>)>,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pp_directive_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpDirectiveInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("directive of unknown task {}", task.index())))?;
    if record.kind != TaskKind::PREPROCESS_DIRECTIVE {
        return Err(protocol_fault(format!(
            "directive task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.is_empty() {
        return Err(protocol_fault(format!(
            "directive task {} payload must carry pp-token refs",
            task.index()
        )));
    }
    let mut token_ids = Vec::with_capacity(record.payload.refs.len());
    for reference in &record.payload.refs {
        match reference {
            RecordRef::PpToken(id) => token_ids.push(*id),
            _ => {
                return Err(protocol_fault(format!(
                    "directive task {} payload must be pp-token refs",
                    task.index()
                )));
            }
        }
    }
    let state = record.state.clone();
    let mut tokens = Vec::with_capacity(token_ids.len());
    for id in token_ids {
        match bus.arenas.pp_tokens.get(id) {
            Ok(found) => tokens.push((id, found.clone())),
            Err(_) => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("directive reads missing pp-token {}", id.index()),
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
                        format!("directive reads missing span {}", token.span.index()),
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
                        format!("directive reads missing source {}", span.source.index()),
                    ));
                }
            }
        }
    }
    let mut children = Vec::new();
    for (id, child) in bus.arenas.tasks.iter() {
        if child.parent == Some(task) {
            let message = match child.state {
                TaskState::Failed(diagnostic) => bus
                    .arenas
                    .diagnostics
                    .get(diagnostic)
                    .ok()
                    .map(|record| record.message.clone()),
                _ => None,
            };
            children.push((id, child.state.clone(), message));
        }
    }
    Ok(PpDirectiveInput {
        task,
        state,
        tokens,
        spans,
        sources,
        children,
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

/// The T03 directive-dispatch worker (PP05 slice scope).
pub struct PpDirectiveChip;

impl Worker for PpDirectiveChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP05_CHIP,
            chip_name: "PpDirectiveChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![TaskKind::PREPROCESS_DIRECTIVE],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Sources, "bytes"),
                FieldPath::new(StoreId::Sources, "spans"),
                FieldPath::new(StoreId::Diagnostics, "entries"),
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
        let input = match project_pp_directive_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

/// One raw source line's worth of payload tokens, in stream order.
struct RawLine {
    /// Owning source.
    source: SourceId,
    /// Zero-based line index (count of `\n` before the line start).
    line: usize,
    /// Indices into the input token vector, in order.
    members: Vec<usize>,
}

impl PpDirectiveChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// First dispatch (no children yet): group the stream into raw lines,
    /// classify directive lines, and either Ack (none), Fail (malformed or
    /// non-`error` directive), or enqueue one PP26 child per `#error` line
    /// and await them all. Resume dispatch runs only when the frozen join
    /// readies this task (all children `Completed`, which PP26 never
    /// produces): the failed-children aggregate below is
    /// unreachable-defensive, because a `Failed` child makes the join
    /// itself fail this task reusing that child's diagnostic.
    pub fn compute(&self, input: &PpDirectiveInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("directive task {} is not running", input.task.index()),
                ),
            )];
        }
        if !input.children.is_empty() {
            return resume_pp_directive(input);
        }
        dispatch_pp_directive(input)
    }
}

/// Resume path: under the frozen `/9` join this dispatch runs only after
/// every child is terminal AND readied, i.e. in practice only when all
/// children `Completed` (a `Failed` child makes the join itself fail this
/// task reusing that child's diagnostic, so the failed branch below is
/// unreachable-defensive and mints no record on the real failure path).
fn resume_pp_directive(input: &PpDirectiveInput) -> Vec<Proposal> {
    for (_, state, _) in &input.children {
        if !state.is_terminal() {
            return vec![fail(
                input.task,
                protocol_fault("directive resume reads a non-terminal child"),
            )];
        }
    }
    let mut failed = 0u32;
    let mut first: Option<String> = None;
    for (_, state, message) in &input.children {
        if matches!(state, TaskState::Failed(_)) {
            failed += 1;
            if first.is_none() {
                first = Some(match message {
                    Some(text) => text.clone(),
                    None => "missing diagnostic".to_string(),
                });
            }
        }
    }
    if failed > 0 {
        let head = match first {
            Some(text) => text,
            None => "missing diagnostic".to_string(),
        };
        return vec![fail(
            input.task,
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("{failed} error directive(s) rejected; first: {head}"),
            ),
        )];
    }
    vec![Proposal::Complete {
        task: input.task,
        value: ResultValue::Ack,
    }]
}

/// First-dispatch path: group, classify, then Ack, Fail, or fan out.
fn dispatch_pp_directive(input: &PpDirectiveInput) -> Vec<Proposal> {
    let lines = match group_raw_lines(input) {
        Ok(lines) => lines,
        Err(diagnostic) => return vec![fail(input.task, diagnostic)],
    };
    let mut errors: Vec<usize> = Vec::new();
    for (position, line) in lines.iter().enumerate() {
        if !raw_line_is_directive(input, line) {
            continue;
        }
        if line.members.len() == 1 {
            continue;
        }
        let name = match directive_name(input, line) {
            Ok(name) => name,
            Err(diagnostic) => return vec![fail(input.task, diagnostic)],
        };
        if name.as_slice() == b"error" {
            errors.push(position);
            continue;
        }
        return vec![fail(input.task, unsupported_for_name(&name))];
    }
    if errors.is_empty() {
        return vec![Proposal::Complete {
            task: input.task,
            value: ResultValue::Ack,
        }];
    }
    let mut proposals = Vec::new();
    for position in &errors {
        let found = &lines[*position];
        let mut refs = Vec::new();
        for index in 1..found.members.len() {
            refs.push(RecordRef::PpToken(input.tokens[found.members[index]].0));
        }
        proposals.push(Proposal::Enqueue(TaskDraft {
            kind: TaskKind::PREPROCESS_DIAGNOSTIC,
            owner: PP26_CHIP,
            parent: Some(input.task),
            payload: Payload::from_refs(refs),
            continuation: None,
        }));
    }
    let mut awaited = Vec::new();
    for index in 0..errors.len() {
        awaited.push(ChildRef::OwnBatch(index as u32));
    }
    proposals.push(Proposal::AwaitChildren {
        task: input.task,
        children: awaited,
    });
    proposals
}

/// Group payload tokens into raw lines: consecutive tokens sharing the same
/// `(source, line)` key form one line, in stream order.
fn group_raw_lines(input: &PpDirectiveInput) -> Result<Vec<RawLine>, DiagnosticDraft> {
    let mut lines: Vec<RawLine> = Vec::new();
    for index in 0..input.tokens.len() {
        let span = match find_span(input, input.tokens[index].1.span) {
            Some(found) => found,
            None => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "directive reads unprojected span {}",
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
                    format!("directive reads unprojected source {}", span.source.index()),
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
                        "directive groups an empty line set",
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
fn find_span(input: &PpDirectiveInput, id: SpanId) -> Option<SpanRecord> {
    for (known, record) in &input.spans {
        if *known == id {
            return Some(*record);
        }
    }
    None
}

/// Look up projected source bytes by ID.
fn find_source(input: &PpDirectiveInput, id: SourceId) -> Option<&[u8]> {
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

/// Directive-line test (§2 frozen): the first token is `#`/`%:` AND the raw
/// walk-back from its offset passes. Missing span/source data falls back to
/// non-directive here, but that path is unreachable: `group_raw_lines`
/// above already fails loudly on the same lookups for every token.
fn raw_line_is_directive(input: &PpDirectiveInput, line: &RawLine) -> bool {
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

/// Raw walk-back (§2 frozen): from the `#` offset skip `[ \t]`; input start
/// or `\n` accepts; a same-line `/*...*/` is skipped and walking continues;
/// anything else (including `//`, mid-line bytes, unterminated or
/// multi-line comments) rejects.
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

/// Directive name (§2 frozen): the next token when it is an `Identifier`,
/// else a malformed-line typed error.
fn directive_name(input: &PpDirectiveInput, line: &RawLine) -> Result<Vec<u8>, DiagnosticDraft> {
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

/// Fail-closed taxonomy (§2 frozen): `error` never reaches here (it fans
/// out); `warning` and every other name is explicit `Unsupported` naming the
/// deferred owner.
fn unsupported_for_name(name: &[u8]) -> DiagnosticDraft {
    let text = String::from_utf8_lossy(name);
    if name == b"warning" {
        return DiagnosticDraft::unsupported(format!(
            "unsupported directive `{text}`: non-fatal diagnostics need a protocol wire (maximal deferral; no PP26-style failure product exists)"
        ));
    }
    if name == b"define" || name == b"undef" {
        return DiagnosticDraft::unsupported(format!(
            "unsupported directive `{text}`: macros are deferred to the macro chips (PP06 definition, PP09-PP16 expansion)"
        ));
    }
    if name == b"include" || name == b"include_next" {
        return DiagnosticDraft::unsupported(format!(
            "unsupported directive `{text}`: header inclusion is deferred to PP17/PP18 (include-resolve/include-enter-exit chips)"
        ));
    }
    if name == b"if"
        || name == b"ifdef"
        || name == b"ifndef"
        || name == b"elif"
        || name == b"else"
        || name == b"endif"
    {
        return DiagnosticDraft::unsupported(format!(
            "unsupported directive `{text}`: conditionals are deferred to PP19-PP22 (conditional-directive chips)"
        ));
    }
    if name == b"pragma" {
        return DiagnosticDraft::unsupported(format!(
            "unsupported directive `{text}`: deferred to PP25 (pragma-dispatch chip)"
        ));
    }
    if name == b"line" {
        return DiagnosticDraft::unsupported(format!(
            "unsupported directive `{text}`: deferred to PP23 (line-directive chip)"
        ));
    }
    DiagnosticDraft::unsupported(format!(
        "unsupported directive `{text}`: deferred past this slice (no owner frozen; fail-closed)"
    ))
}
