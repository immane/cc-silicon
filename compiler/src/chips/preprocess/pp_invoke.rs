// ============================================================================
// chips/preprocess/pp_invoke.rs — T03 PP09 macro-invocation fan-out worker
// (Wave 2 slice 15, `/24`)
//
// Reads all active pp-token refs, scans the stream in order for top-level
// macro invocations, and fans out one PP12 substitute child per invocation
// (payload = `[Macro(def)]` + invocation refs) behind a single
// `AwaitChildren` over the whole batch; on resume stitches the expanded
// stream in original order (untouched refs reused verbatim, each
// invocation replaced by its child's committed `Records` refs) and
// completes `Records`. Directive lines (first token `#`/`%:` + the `/21`
// raw walk-back, chip-local copy per the shared-helper precedent) pass
// through verbatim — never expanded, never dispatched, never stitched
// away. Zero invocations complete `Records` of all input refs with no
// appends and no children. This chip appends NOTHING itself (children do);
// it only enqueues, awaits, and stitches. No writes, no spans invented.
//
// Frozen-join conventions (same as PP05): resume-compute runs only when
// all children are terminal AND readied, i.e. in practice only when all
// children `Completed` — a `Failed` child makes the join itself fail this
// task reusing that child's diagnostic, so the failed branch below is
// unreachable-defensive (explicit `Fail`, never silent, never an
// aggregate minted on the real path).
//
// Invocation rules (`PP_EXPAND_SLICE.md` §3): an `Identifier` spelling
// with a latest committed non-tombstone definition is an object
// invocation when the definition is object-like, or a function
// invocation when the definition is function-like AND the next token is
// a span-adjacent `(` (same source, `name.span.end == paren.span.start`).
// Function-likeness is read off the committed definition's
// `function_like` flag: an object-like definition (`false`) expands on a
// bare name, while a function-like definition (`true`, including
// zero-parameter `#define F()`) invokes only on a span-adjacent `F()` —
// a bare `F` stays a plain identifier. A function-like name WITHOUT an
// adjacent `(` is a plain identifier, never an invocation. Balanced
// argument collection is depth-counted with top-level comma splits:
// `()` (immediately empty) is ZERO args, `(,)` is two empties, `f(a,)`
// is two args with an empty second. A variadic definition USE (an actual
// invocation of a `variadic` def) fails fast as explicit `Unsupported`
// naming PP16. Unknown identifiers stay plain tokens. Consumed ranges
// are skipped ahead past, so nested spans never double-dispatch.
// ============================================================================

use crate::bus::{MacroRecord, PpTokenKind, PpTokenRecord, SpanRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{MacroId, PpTokenId, RecordRef, SourceId, SpanId, TaskId};
use crate::manifest::{
    BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP09_CHIP, PP12_CHIP,
};
use crate::task::{
    ChildRef, Payload, Proposal, ResultValue, StoreId, TaskDraft, TaskGroup, TaskKind, TaskState,
};

/// Narrow projection for the macro-invocation computation.
#[derive(Clone, Debug)]
pub struct PpInvokeInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// All committed pp-tokens in payload (stream) order.
    pub tokens: Vec<(PpTokenId, PpTokenRecord)>,
    /// Span records referenced by the payload tokens, in first-use order.
    pub spans: Vec<(SpanId, SpanRecord)>,
    /// Source byte strings referenced by those spans, in first-use order.
    pub sources: Vec<(SourceId, Vec<u8>)>,
    /// Committed macro definitions in ascending-ID order (latest wins).
    pub macros: Vec<(MacroId, MacroRecord)>,
    /// Already-enqueued children (tasks with `parent == Some(task)`, in
    /// `TaskId` order) with their states.
    pub children: Vec<(TaskId, TaskState)>,
    /// Committed child expansion refs, aligned 1:1 with `children`
    /// (`Completed` children resolve their `Records` result; any other
    /// state projects an empty placeholder — resume asserts terminality
    /// before touching these).
    pub expansions: Vec<(TaskId, Vec<RecordRef>)>,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pp_invoke_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpInvokeInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("invoke of unknown task {}", task.index())))?;
    if record.kind != TaskKind::PREPROCESS_MACRO_INVOKE {
        return Err(protocol_fault(format!(
            "invoke task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.is_empty() {
        return Err(protocol_fault(format!(
            "invoke task {} payload must carry pp-token refs",
            task.index()
        )));
    }
    let mut tokens = Vec::with_capacity(record.payload.refs.len());
    for reference in &record.payload.refs {
        match reference {
            RecordRef::PpToken(id) => match bus.arenas.pp_tokens.get(*id) {
                Ok(found) => tokens.push((*id, found.clone())),
                Err(_) => {
                    return Err(DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("invoke reads missing pp-token {}", id.index()),
                    ));
                }
            },
            _ => {
                return Err(protocol_fault(format!(
                    "invoke task {} payload must be pp-token refs",
                    task.index()
                )));
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
                        format!("invoke reads missing span {}", token.span.index()),
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
                        format!("invoke reads missing source {}", span.source.index()),
                    ));
                }
            }
        }
    }
    let mut macros: Vec<(MacroId, MacroRecord)> = Vec::new();
    for (id, body) in bus.arenas.macros.iter() {
        macros.push((id, body.clone()));
    }
    macros.sort_by(cmp_macro_id);
    let mut children: Vec<(TaskId, TaskState)> = Vec::new();
    for (id, child) in bus.arenas.tasks.iter() {
        if child.parent == Some(task) {
            children.push((id, child.state.clone()));
        }
    }
    children.sort_by(cmp_child_id);
    let mut expansions: Vec<(TaskId, Vec<RecordRef>)> = Vec::new();
    for (id, state) in &children {
        match state {
            TaskState::Completed(result) => {
                let stored = match bus.arenas.results.get(*result) {
                    Ok(found) => found,
                    Err(_) => {
                        return Err(protocol_fault(format!(
                            "invoke reads missing result {}",
                            result.index()
                        )));
                    }
                };
                match &stored.value {
                    ResultValue::Records(refs) => expansions.push((*id, refs.clone())),
                    _ => {
                        return Err(protocol_fault(format!(
                            "invoke child {} result must be record refs",
                            id.index()
                        )));
                    }
                }
            }
            _ => expansions.push((*id, Vec::new())),
        }
    }
    Ok(PpInvokeInput {
        task,
        state: record.state.clone(),
        tokens,
        spans,
        sources,
        macros,
        children,
        expansions,
    })
}

/// Ascending-`MacroId` order (arena iteration already yields it; the sort
/// pins the projector contract explicitly).
fn cmp_macro_id(a: &(MacroId, MacroRecord), b: &(MacroId, MacroRecord)) -> std::cmp::Ordering {
    a.0.index().cmp(&b.0.index())
}

/// Ascending-`TaskId` order for committed children.
fn cmp_child_id(a: &(TaskId, TaskState), b: &(TaskId, TaskState)) -> std::cmp::Ordering {
    a.0.index().cmp(&b.0.index())
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

/// The T03 macro-invocation worker (PP09 slice scope).
pub struct PpInvokeChip;

impl Worker for PpInvokeChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP09_CHIP,
            chip_name: "PpInvokeChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![TaskKind::PREPROCESS_MACRO_INVOKE],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Pp, "macros"),
                FieldPath::new(StoreId::Sources, "bytes"),
                FieldPath::new(StoreId::Sources, "spans"),
                FieldPath::new(StoreId::Tasks, "results"),
                FieldPath::new(StoreId::Tasks, "completed"),
            ],
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c24_expand.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pp_invoke_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

/// One top-level invocation: the committed definition plus the token
/// index range (`start..end`, end exclusive) it consumes.
struct Invocation {
    /// Latest committed definition used.
    def: MacroId,
    /// First token index (the macro name).
    start: usize,
    /// One past the last consumed token index.
    end: usize,
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

impl PpInvokeChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// First dispatch (no children yet): scan the stream for top-level
    /// invocations and either complete `Records` of all input refs (zero
    /// invocations — passthrough, no appends, no children) or enqueue one
    /// PP12 child per invocation and await them all. Resume dispatch runs
    /// only when the frozen join readies this task (all children
    /// `Completed`, which is when stitching runs): the failed-children
    /// branch below is unreachable-defensive, because a `Failed` child
    /// makes the join itself fail this task reusing that child's
    /// diagnostic.
    pub fn compute(&self, input: &PpInvokeInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("invoke task {} is not running", input.task.index()),
                ),
            )];
        }
        if !input.children.is_empty() {
            return resume_pp_invoke(input);
        }
        dispatch_pp_invoke(input)
    }
}

/// Resume path: under the frozen join this dispatch runs only after every
/// child is terminal AND readied, i.e. in practice only when all children
/// `Completed` (a `Failed` child makes the join itself fail this task
/// reusing that child's diagnostic, so the failed branch below is
/// unreachable-defensive and mints no record on the real failure path).
/// Re-scans the ORIGINAL stream for invocation ranges, then stitches:
/// untouched refs reused verbatim, each invocation replaced by its
/// child's committed `Records` refs, completing `Records`.
fn resume_pp_invoke(input: &PpInvokeInput) -> Vec<Proposal> {
    for (_, state) in &input.children {
        if !state.is_terminal() {
            return vec![fail(
                input.task,
                protocol_fault("invoke resume reads a non-terminal child"),
            )];
        }
    }
    for (_, state) in &input.children {
        if matches!(state, TaskState::Failed(_)) {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "invoke resume reads a failed child (unreachable under the frozen join)",
                ),
            )];
        }
    }
    if input.children.len() != input.expansions.len() {
        return vec![fail(
            input.task,
            protocol_fault("invoke resume projects an expansion per child"),
        )];
    }
    let verbatim = match mark_verbatim(input) {
        Ok(found) => found,
        Err(diagnostic) => return vec![fail(input.task, diagnostic)],
    };
    let invocations = match scan_invocations(input, &verbatim) {
        Ok(found) => found,
        Err(diagnostic) => return vec![fail(input.task, diagnostic)],
    };
    if invocations.len() != input.children.len() {
        return vec![fail(
            input.task,
            protocol_fault("invoke resume replays one invocation per child"),
        )];
    }
    let mut refs: Vec<RecordRef> = Vec::new();
    let mut cursor = 0usize;
    for (position, invocation) in invocations.iter().enumerate() {
        if invocation.start < cursor
            || invocation.end > input.tokens.len()
            || invocation.start >= invocation.end
        {
            return vec![fail(
                input.task,
                protocol_fault("invoke resume replays an out-of-order invocation"),
            )];
        }
        for index in cursor..invocation.start {
            refs.push(RecordRef::PpToken(input.tokens[index].0));
        }
        let owned = match input.children.get(position) {
            Some(found) => found,
            None => {
                return vec![fail(
                    input.task,
                    protocol_fault("invoke resume indexes a missing child"),
                )];
            }
        };
        let expansion = match input.expansions.get(position) {
            Some(found) => found,
            None => {
                return vec![fail(
                    input.task,
                    protocol_fault("invoke resume indexes a missing expansion"),
                )];
            }
        };
        if owned.0 != expansion.0 {
            return vec![fail(
                input.task,
                protocol_fault("invoke resume misaligns a child expansion"),
            )];
        }
        for reference in &expansion.1 {
            refs.push(*reference);
        }
        cursor = invocation.end;
    }
    for index in cursor..input.tokens.len() {
        refs.push(RecordRef::PpToken(input.tokens[index].0));
    }
    vec![Proposal::Complete {
        task: input.task,
        value: ResultValue::Records(refs),
    }]
}

/// First-dispatch path: scan, then passthrough-complete or fan out.
fn dispatch_pp_invoke(input: &PpInvokeInput) -> Vec<Proposal> {
    let verbatim = match mark_verbatim(input) {
        Ok(found) => found,
        Err(diagnostic) => return vec![fail(input.task, diagnostic)],
    };
    let invocations = match scan_invocations(input, &verbatim) {
        Ok(found) => found,
        Err(diagnostic) => return vec![fail(input.task, diagnostic)],
    };
    if invocations.is_empty() {
        let mut refs = Vec::with_capacity(input.tokens.len());
        for (id, _) in &input.tokens {
            refs.push(RecordRef::PpToken(*id));
        }
        return vec![Proposal::Complete {
            task: input.task,
            value: ResultValue::Records(refs),
        }];
    }
    let mut proposals = Vec::new();
    for invocation in &invocations {
        let mut refs: Vec<RecordRef> = Vec::with_capacity(invocation.end - invocation.start + 1);
        refs.push(RecordRef::Macro(invocation.def));
        for index in invocation.start..invocation.end {
            refs.push(RecordRef::PpToken(input.tokens[index].0));
        }
        proposals.push(Proposal::Enqueue(TaskDraft {
            kind: TaskKind::PREPROCESS_MACRO_SUBSTITUTE,
            owner: PP12_CHIP,
            parent: Some(input.task),
            payload: Payload::from_refs(refs),
            continuation: None,
        }));
    }
    let mut awaited = Vec::new();
    let mut key = 0u32;
    while key < invocations.len() as u32 {
        awaited.push(ChildRef::OwnBatch(key));
        key += 1;
    }
    proposals.push(Proposal::AwaitChildren {
        task: input.task,
        children: awaited,
    });
    proposals
}

/// Top-level invocation scan in stream order, skipping nested spans once
/// consumed. Directive-line tokens pass through (never invocation starts,
/// never collected into argument lists). A variadic definition USE fails
/// fast as explicit `Unsupported` naming PP16; unbalanced or
/// directive-crossing argument lists are typed failures.
fn scan_invocations(
    input: &PpInvokeInput,
    verbatim: &[bool],
) -> Result<Vec<Invocation>, DiagnosticDraft> {
    let mut out: Vec<Invocation> = Vec::new();
    let mut cursor = 0usize;
    while cursor < input.tokens.len() {
        if is_verbatim(verbatim, cursor) {
            cursor += 1;
            continue;
        }
        if input.tokens[cursor].1.kind != PpTokenKind::Identifier {
            cursor += 1;
            continue;
        }
        let spelling = input.tokens[cursor].1.spelling.clone();
        let found = match latest_with_spelling(&input.macros, &spelling) {
            Some(found) => found,
            None => {
                cursor += 1;
                continue;
            }
        };
        if found.1.function_like {
            let next = cursor + 1;
            if !open_paren_at(input, verbatim, next) {
                cursor += 1;
                continue;
            }
            let adjacent = spans_adjacent(
                input,
                input.tokens[cursor].1.span,
                input.tokens[next].1.span,
            )?;
            if !adjacent {
                cursor += 1;
                continue;
            }
            if found.1.variadic {
                return Err(variadic_unsupported(&spelling));
            }
            let end = collect_call_end(input, verbatim, next)?;
            out.push(Invocation {
                def: found.0,
                start: cursor,
                end,
            });
            cursor = end;
        } else {
            if found.1.variadic {
                return Err(variadic_unsupported(&spelling));
            }
            out.push(Invocation {
                def: found.0,
                start: cursor,
                end: cursor + 1,
            });
            cursor += 1;
        }
    }
    Ok(out)
}

/// Latest committed record with spelling equal to `spelling` (`macros` is
/// in ascending-ID order, so the last match wins); a latest tombstone —
/// or absence — means undefined (`None`).
fn latest_with_spelling(
    macros: &[(MacroId, MacroRecord)],
    spelling: &[u8],
) -> Option<(MacroId, MacroRecord)> {
    let mut latest: Option<(MacroId, MacroRecord)> = None;
    for (id, record) in macros {
        if record.spelling.as_slice() == spelling {
            latest = Some((*id, record.clone()));
        }
    }
    match latest {
        Some((id, record)) if !record.undefined => Some((id, record)),
        _ => None,
    }
}

/// True when `index` names a verbatim (directive-line) token.
fn is_verbatim(verbatim: &[bool], index: usize) -> bool {
    matches!(verbatim.get(index), Some(true))
}

/// True when `index` holds a non-verbatim `(` punctuator candidate.
fn open_paren_at(input: &PpInvokeInput, verbatim: &[bool], index: usize) -> bool {
    if is_verbatim(verbatim, index) {
        return false;
    }
    match input.tokens.get(index) {
        Some((_, record)) => {
            record.kind == PpTokenKind::Punctuator && record.spelling.as_slice() == b"("
        }
        None => false,
    }
}

/// Span adjacency (the PP06 whitespace-sensitivity rule): same source
/// with `name.end == paren.start`. Unprojected spans are a typed
/// failure (unreachable: the projector covers every payload span).
fn spans_adjacent(
    input: &PpInvokeInput,
    name: SpanId,
    paren: SpanId,
) -> Result<bool, DiagnosticDraft> {
    let name_span = match find_span(input, name) {
        Some(found) => found,
        None => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("invoke reads unprojected span {}", name.index()),
            ));
        }
    };
    let paren_span = match find_span(input, paren) {
        Some(found) => found,
        None => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("invoke reads unprojected span {}", paren.index()),
            ));
        }
    };
    Ok(name_span.source == paren_span.source && name_span.end == paren_span.start)
}

/// Balanced argument collection from the `(` at `paren`: depth-counted
/// parens, returning one past the matching close. Verbatim
/// (directive-line) tokens inside the list and a missing close are typed
/// failures — fail-closed, never a silent partial invocation.
fn collect_call_end(
    input: &PpInvokeInput,
    verbatim: &[bool],
    paren: usize,
) -> Result<usize, DiagnosticDraft> {
    let mut depth = 0u32;
    let mut cursor = paren;
    while cursor < input.tokens.len() {
        if is_verbatim(verbatim, cursor) {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "macro invocation crosses a directive line",
            ));
        }
        let record = &input.tokens[cursor].1;
        if record.kind == PpTokenKind::Punctuator && record.spelling.as_slice() == b"(" {
            depth += 1;
        } else if record.kind == PpTokenKind::Punctuator && record.spelling.as_slice() == b")" {
            if depth == 0 {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "unbalanced parentheses in macro invocation",
                ));
            }
            depth -= 1;
            if depth == 0 {
                return Ok(cursor + 1);
            }
        }
        cursor += 1;
    }
    Err(DiagnosticDraft::error(
        DiagnosticCode::new(DiagGroup::Task, 4),
        "unbalanced parentheses in macro invocation",
    ))
}

/// Explicit `Unsupported` for a variadic-macro USE (PP16 owns variadics).
fn variadic_unsupported(spelling: &[u8]) -> DiagnosticDraft {
    let text = String::from_utf8_lossy(spelling);
    DiagnosticDraft::unsupported(format!(
        "use of variadic macro `{text}` is unsupported: variadic expansion is deferred to PP16"
    ))
}

/// Verbatim marking: one flag per payload token, set for tokens on
/// directive lines (which pass through untouched).
fn mark_verbatim(input: &PpInvokeInput) -> Result<Vec<bool>, DiagnosticDraft> {
    let lines = group_raw_lines(input)?;
    let mut verbatim = vec![false; input.tokens.len()];
    for line in &lines {
        if !raw_line_is_directive(input, line) {
            continue;
        }
        for member in &line.members {
            if *member < verbatim.len() {
                verbatim[*member] = true;
            }
        }
    }
    Ok(verbatim)
}

/// Group payload tokens into raw lines: consecutive tokens sharing the same
/// `(source, line)` key form one line, in stream order.
fn group_raw_lines(input: &PpInvokeInput) -> Result<Vec<RawLine>, DiagnosticDraft> {
    let mut lines: Vec<RawLine> = Vec::new();
    for index in 0..input.tokens.len() {
        let span = match find_span(input, input.tokens[index].1.span) {
            Some(found) => found,
            None => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "invoke reads unprojected span {}",
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
                    format!("invoke reads unprojected source {}", span.source.index()),
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
                        "invoke groups an empty line set",
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
fn find_span(input: &PpInvokeInput, id: SpanId) -> Option<SpanRecord> {
    for (known, record) in &input.spans {
        if *known == id {
            return Some(*record);
        }
    }
    None
}

/// Look up projected source bytes by ID.
fn find_source(input: &PpInvokeInput, id: SourceId) -> Option<&[u8]> {
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

/// Directive-line test (`/21` predicate, owned copy per the
/// shared-helper precedent): the first token is `#`/`%:` AND the raw
/// walk-back from its offset passes. Missing span/source data falls back
/// to non-directive here, but that path is unreachable: `group_raw_lines`
/// above already fails loudly on the same lookups for every token.
fn raw_line_is_directive(input: &PpInvokeInput, line: &RawLine) -> bool {
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

/// Raw walk-back (`/21` frozen): from the `#` offset skip `[ \t]`; input
/// start or `\n` accepts; a same-line `/*...*/` is skipped and walking
/// continues; anything else (including `//`, mid-line bytes,
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
