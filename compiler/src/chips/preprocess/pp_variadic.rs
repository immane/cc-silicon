// ============================================================================
// chips/preprocess/pp_variadic.rs — T03 PP16 variadic-macro worker (FROZEN
// `/26`; see `docs/tasks/PP_VARIADIC_SLICE.md`)
//
// Covers the `MacroRecord::variadic` definitions that PP09/PP12 defer as
// explicit `Unsupported`. Two payload shapes share one chip:
//
// * Stream mode (PP09 extension): payload = all active pp-token refs (no
//   leading `Macro` ref). Scans the stream exactly like `pp_invoke.rs`
//   (directive lines pass through verbatim via the `/21` raw walk-back,
//   nested spans are never double-dispatched), but only variadic-definition
//   invocations fan out — one single-mode child per invocation behind a
//   single `AwaitChildren` — and stitch on resume. Invocations of
//   non-variadic definitions with the correct arity pass through verbatim
//   (PP09 owns them); anything else is a typed `Fail`, never silent.
// * Single mode (PP12 counterpart): payload = `[Macro(def)]` + invocation
//   refs. Substitutes one variadic invocation (`...` collection,
//   `__VA_ARGS__`, `__VA_OPT__`, `#`/`##` with prescan, blue-paint rescan
//   with a macro-count+2 breaker) and completes the expansion refs.
//
// `__VA_OPT__` policy (chip choice, C23 standard): `__VA_OPT__(content)`
// expands to `content` iff the variadic tail holds at least one
// preprocessing token, else to nothing. Parameters inside the content are
// substituted like the rest of the replacement list; `__VA_OPT__` nests
// recursively. An empty variadic tail is LEGAL (C23 6.10.3): `f("x")` on
// `#define f(fmt, ...)` binds an empty `__VA_ARGS__` (placemarker) and an
// empty `__VA_OPT__` — never a failure.
//
// Fixed/variadic parameter rule: `def.params` entries spelling
// `__VA_ARGS__` alias the variadic tail (covers `#define f(__VA_ARGS__)`
// and `#define f(a, __VA_ARGS__)`); every other param is fixed and the
// call must supply at least that many arguments. Extra arguments form the
// tail in order (possibly empty).
//
// GNU `, ## __VA_ARGS__` comma swallowing is NOT implemented: `#`/`##`
// follow the frozen PP12 rules verbatim, so a retained comma survives an
// empty tail. T03 qualifies swallowing by dialect and no dialect pin is
// projected here (same reason PP12 has no dialect reads); enabling it is
// an integrator-owned dialect-gated follow-up.
//
// Fail-closed edges: `# __VA_OPT__` and `##` directly adjacent to a
// `__VA_OPT__` node are constraint `Fail`s (the standard gives them no
// meaning here); a bare `__VA_OPT__` without `(` is a constraint `Fail`;
// an unbalanced `__VA_OPT__(` is malformed.
//
// Mismatch matrix (all typed `Fail`, all naming PP16):
// * single mode with a non-variadic def → `Unsupported` (PP12 owns
//   non-variadic definitions; mirrors PP12's variadic-`Unsupported`);
// * stream mode with extra or missing arguments on a non-variadic def
//   (variadic-shaped use on a non-variadic definition) → malformed
//   (`Task`, 4);
// * function-like variadic name without a span-adjacent `(` → plain
//   identifier, no dispatch (same adjacency rule as PP06/PP09);
// * unknown identifiers and tombstoned/undefined macros → plain tokens.
// Nested variadic invocations inside a NON-variadic invocation's argument
// list are left for PP09/PP12 (which fail them as explicit `Unsupported`
// until a PP09/PP16 rescan coordinator lands); the single-mode engine
// itself expands nested macros of either flavor internally.
//
// Frozen-join semantics (same as PP05/PP09): resume runs only when every
// awaited child is terminal; the first `Failed` child fails this task
// reusing that child's diagnostic (no aggregate minted); stitching runs
// only in the all-`Completed` case.
//
// No cross-chip imports: every predicate, the `/21` walk-back, the
// invocation scan/stitch, and the substitute/rescan engine are chip-local
// copies of the frozen PP09/PP12 rules (copy precedent: the `compose_map`
// copies in `pp_splice.rs`/`pp_comment.rs`). Pasted spellings are validated
// by a chip-local single-token classifier (identifier / pp-number /
// punctuator tables / quoted literals) instead of importing the scanner.
//
// Frozen registration (`/26` integrator-owned): `PP16_TASK_KIND` aliases
// `TaskKind::PREPROCESS_VARIADIC_MACRO` (PREPROCESS local 30),
// `PP16_CHIP` is `crate::manifest::PP16_CHIP` (`ChipId(33)`), the
// kind-registry row lives in `TaskKindRegistry::pp_variadic_slice()`, the
// stage-1 row in `STAGE_ASSIGNMENT`, the routed layer is 1, the
// `(PP16_CHIP, Pp, "tokens")` allowlist row authorizes single-mode
// appends, the acceptance test is `compiler/tests/c26_variadic.rs`, and
// stream dispatch runs before/around PP09 (PP09 owns non-variadic uses).
// ============================================================================

use crate::bus::{MacroRecord, PpTokenKind, PpTokenRecord, SpanRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft, DiagnosticRecord};
use crate::ids::{MacroId, PpTokenId, RecordFamily, RecordRef, SourceId, SpanId, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP16_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, ChildRef, DraftRef, Payload, Proposal, ResultValue, StoreId, TaskDraft, TaskGroup,
    TaskKind, TaskState,
};

/// Frozen PP16 task kind (single kind for both payload shapes; aliases
/// `TaskKind::PREPROCESS_VARIADIC_MACRO`, frozen by the `/26` integrator;
/// the local code is `PREPROCESS` 30, first code after `/25`).
pub const PP16_TASK_KIND: TaskKind = TaskKind::PREPROCESS_VARIADIC_MACRO;

/// Narrow projection for the variadic-macro computation.
#[derive(Clone, Debug)]
pub struct PpVariadicInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload shape: stream (all pp-tokens) or single (`Macro` + call).
    pub mode: PpVariadicMode,
    /// Stream-mode token bodies in payload order (single mode: empty).
    pub tokens: Vec<(PpTokenId, PpTokenRecord)>,
    /// Source byte strings referenced by the stream spans, first-use order.
    pub sources: Vec<(SourceId, Vec<u8>)>,
    /// Single-mode invoked definition ID (stream mode: `MacroId::NONE`).
    pub def_id: MacroId,
    /// Single-mode invoked definition body (stream mode: blank record).
    pub def: MacroRecord,
    /// Single-mode invocation token bodies in payload order.
    pub invocation: Vec<(PpTokenId, PpTokenRecord)>,
    /// Body pool for substitution and nested rescan: every pp-token
    /// referenced by the invoked definition's or any committed macro's
    /// replacement list, in ascending-ID order.
    pub pool: Vec<(PpTokenId, PpTokenRecord)>,
    /// Span records referenced by the stream/invocation/pool tokens, in
    /// first-use (stream) or ascending-ID (single) order.
    pub spans: Vec<(SpanId, SpanRecord)>,
    /// Committed macro definitions in ascending-ID order (latest wins).
    pub macros: Vec<(MacroId, MacroRecord)>,
    /// Already-enqueued children (tasks with `parent == Some(task)`, in
    /// `TaskId` order) with their states.
    pub children: Vec<(TaskId, TaskState)>,
    /// Committed child expansion refs, aligned 1:1 with `children`.
    pub expansions: Vec<(TaskId, Vec<RecordRef>)>,
    /// Committed diagnostics of the failed children, by child task.
    pub child_diagnostics: Vec<(TaskId, DiagnosticRecord)>,
    /// `pp_tokens` arena count at dispatch (synthesized-append base).
    pub pp_tokens_allocated: u32,
}

/// Payload shape accepted by the variadic chip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PpVariadicMode {
    /// All payload refs are pp-tokens: PP09-style variadic fan-out.
    Stream,
    /// First payload ref is a macro: one variadic substitution.
    Single,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pp_variadic_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpVariadicInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("variadic of unknown task {}", task.index())))?;
    if record.kind.raw() != PP16_TASK_KIND.raw() {
        return Err(protocol_fault(format!(
            "variadic task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.is_empty() {
        return Err(protocol_fault(format!(
            "variadic task {} payload must carry record refs",
            task.index()
        )));
    }
    let single = matches!(record.payload.refs[0], RecordRef::Macro(_));
    if single {
        project_single_input(bus, task)
    } else {
        project_stream_input(bus, task)
    }
}

/// Project a single-mode (`[Macro]` + invocation refs) input.
fn project_single_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpVariadicInput, DiagnosticDraft> {
    let record = match bus.arenas.tasks.get(task) {
        Ok(found) => found,
        Err(_) => {
            return Err(protocol_fault(format!(
                "variadic of unknown task {}",
                task.index()
            )));
        }
    };
    let def_id = match record.payload.refs[0] {
        RecordRef::Macro(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "variadic single task {} payload must lead with a macro ref",
                task.index()
            )));
        }
    };
    let mut invocation_ids = Vec::with_capacity(record.payload.refs.len() - 1);
    for reference in record.payload.refs.iter().skip(1) {
        match reference {
            RecordRef::PpToken(id) => invocation_ids.push(*id),
            _ => {
                return Err(protocol_fault(format!(
                    "variadic single task {} payload must be pp-token refs after the macro",
                    task.index()
                )));
            }
        }
    }
    let def = bus.arenas.macros.get(def_id).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("variadic reads missing macro {}", def_id.index()),
        )
    })?;
    let mut invocation = Vec::with_capacity(invocation_ids.len());
    for id in invocation_ids {
        match bus.arenas.pp_tokens.get(id) {
            Ok(found) => invocation.push((id, found.clone())),
            Err(_) => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("variadic reads missing pp-token {}", id.index()),
                ));
            }
        }
    }
    let mut macros: Vec<(MacroId, MacroRecord)> = bus
        .arenas
        .macros
        .iter()
        .map(|(id, body)| (id, body.clone()))
        .collect();
    macros.sort_by_key(|(id, _)| id.index());
    let mut pool_ids: Vec<PpTokenId> = Vec::new();
    for token in &def.replacement {
        pool_ids.push(*token);
    }
    for (_, body) in &macros {
        for token in &body.replacement {
            pool_ids.push(*token);
        }
    }
    pool_ids.sort_by_key(|id| id.index());
    pool_ids.dedup();
    let mut pool = Vec::with_capacity(pool_ids.len());
    for id in pool_ids {
        match bus.arenas.pp_tokens.get(id) {
            Ok(found) => pool.push((id, found.clone())),
            Err(_) => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("variadic reads missing pp-token {}", id.index()),
                ));
            }
        }
    }
    let mut span_ids: Vec<SpanId> = Vec::new();
    for (_, token) in &invocation {
        span_ids.push(token.span);
    }
    for (_, token) in &pool {
        span_ids.push(token.span);
    }
    span_ids.sort_by_key(|id| id.index());
    span_ids.dedup();
    let mut spans = Vec::with_capacity(span_ids.len());
    for id in span_ids {
        match bus.arenas.spans.get(id) {
            Ok(found) => spans.push((id, *found)),
            Err(_) => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("variadic reads missing span {}", id.index()),
                ));
            }
        }
    }
    let (children, expansions, child_diagnostics) = project_children(bus, task)?;
    Ok(PpVariadicInput {
        task,
        state: record.state.clone(),
        mode: PpVariadicMode::Single,
        tokens: Vec::new(),
        sources: Vec::new(),
        def_id,
        def: def.clone(),
        invocation,
        pool,
        spans,
        macros,
        children,
        expansions,
        child_diagnostics,
        pp_tokens_allocated: bus.arenas.pp_tokens.allocated(),
    })
}

/// Project a stream-mode (all pp-token refs) input.
fn project_stream_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpVariadicInput, DiagnosticDraft> {
    let record = match bus.arenas.tasks.get(task) {
        Ok(found) => found,
        Err(_) => {
            return Err(protocol_fault(format!(
                "variadic of unknown task {}",
                task.index()
            )));
        }
    };
    let mut tokens = Vec::with_capacity(record.payload.refs.len());
    for reference in &record.payload.refs {
        match reference {
            RecordRef::PpToken(id) => match bus.arenas.pp_tokens.get(*id) {
                Ok(found) => tokens.push((*id, found.clone())),
                Err(_) => {
                    return Err(DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("variadic reads missing pp-token {}", id.index()),
                    ));
                }
            },
            _ => {
                return Err(protocol_fault(format!(
                    "variadic stream task {} payload must be pp-token refs",
                    task.index()
                )));
            }
        }
    }
    let mut spans: Vec<(SpanId, SpanRecord)> = Vec::new();
    for (_, token) in &tokens {
        if span_is_projected(&spans, token.span) {
            continue;
        }
        match bus.arenas.spans.get(token.span) {
            Ok(found) => spans.push((token.span, *found)),
            Err(_) => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("variadic reads missing span {}", token.span.index()),
                ));
            }
        }
    }
    let mut sources: Vec<(SourceId, Vec<u8>)> = Vec::new();
    for (_, span) in &spans {
        if source_is_projected(&sources, span.source) {
            continue;
        }
        match bus.arenas.sources.get(span.source) {
            Ok(found) => sources.push((span.source, found.bytes.clone())),
            Err(_) => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("variadic reads missing source {}", span.source.index()),
                ));
            }
        }
    }
    let mut macros: Vec<(MacroId, MacroRecord)> = bus
        .arenas
        .macros
        .iter()
        .map(|(id, body)| (id, body.clone()))
        .collect();
    macros.sort_by_key(|(id, _)| id.index());
    let (children, expansions, child_diagnostics) = project_children(bus, task)?;
    Ok(PpVariadicInput {
        task,
        state: record.state.clone(),
        mode: PpVariadicMode::Stream,
        tokens,
        sources,
        def_id: MacroId::NONE,
        def: MacroRecord {
            spelling: Vec::new(),
            params: Vec::new(),
            function_like: false,
            variadic: false,
            replacement: Vec::new(),
            undefined: true,
        },
        invocation: Vec::new(),
        pool: Vec::new(),
        spans,
        macros,
        children,
        expansions,
        child_diagnostics,
        pp_tokens_allocated: bus.arenas.pp_tokens.allocated(),
    })
}

/// Projected awaited-child state for the frozen join: child states in
/// `TaskId` order, committed expansion refs 1:1, and committed failure
/// diagnostics by child.
type ChildProjection = (
    Vec<(TaskId, TaskState)>,
    Vec<(TaskId, Vec<RecordRef>)>,
    Vec<(TaskId, DiagnosticRecord)>,
);

/// Project already-enqueued children with their states, committed expansion
/// refs, and committed failure diagnostics.
fn project_children(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<ChildProjection, DiagnosticDraft> {
    let mut children: Vec<(TaskId, TaskState)> = Vec::new();
    for (id, child) in bus.arenas.tasks.iter() {
        if child.parent == Some(task) {
            children.push((id, child.state.clone()));
        }
    }
    children.sort_by_key(|(id, _)| id.index());
    let mut expansions: Vec<(TaskId, Vec<RecordRef>)> = Vec::new();
    let mut child_diagnostics: Vec<(TaskId, DiagnosticRecord)> = Vec::new();
    for (id, state) in &children {
        match state {
            TaskState::Completed(result) => {
                let stored = match bus.arenas.results.get(*result) {
                    Ok(found) => found,
                    Err(_) => {
                        return Err(protocol_fault(format!(
                            "variadic reads missing result {}",
                            result.index()
                        )));
                    }
                };
                match &stored.value {
                    ResultValue::Records(refs) => expansions.push((*id, refs.clone())),
                    _ => {
                        return Err(protocol_fault(format!(
                            "variadic child {} result must be record refs",
                            id.index()
                        )));
                    }
                }
            }
            TaskState::Failed(diagnostic) => {
                match bus.arenas.diagnostics.get(*diagnostic) {
                    Ok(found) => child_diagnostics.push((*id, found.clone())),
                    Err(_) => {
                        return Err(DiagnosticDraft::error(
                            DiagnosticCode::new(DiagGroup::Task, 4),
                            format!("variadic reads missing diagnostic {}", diagnostic.index()),
                        ));
                    }
                }
                expansions.push((*id, Vec::new()));
            }
            _ => expansions.push((*id, Vec::new())),
        }
    }
    Ok((children, expansions, child_diagnostics))
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

/// The T03 variadic-macro worker (PP16 proposed scope).
pub struct PpVariadicChip;

impl Worker for PpVariadicChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP16_CHIP,
            chip_name: "PpVariadicChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![PP16_TASK_KIND],
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
                FieldPath::new(StoreId::Diagnostics, "entries"),
            ],
            writes: vec![FieldPath::new(StoreId::Pp, "tokens")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c26_variadic.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pp_variadic_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl PpVariadicChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// Stream mode fans out (first dispatch) or stitches (resume under the
    /// frozen join); single mode substitutes one variadic invocation. Every
    /// path — including all failure paths — yields proposals; nothing
    /// panics and nothing completes silently.
    pub fn compute(&self, input: &PpVariadicInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("variadic task {} is not running", input.task.index()),
                ),
            )];
        }
        match input.mode {
            PpVariadicMode::Stream => {
                if input.children.is_empty() {
                    dispatch_variadic_stream(input)
                } else {
                    resume_variadic_stream(input)
                }
            }
            PpVariadicMode::Single => compute_variadic_single(input),
        }
    }
}

// ---------------------------------------------------------------------------
// Stream mode: PP09-extension fan-out + stitch for variadic definitions.
// ---------------------------------------------------------------------------

/// One variadic invocation: the committed definition plus the token index
/// range (`start..end`, end exclusive) it consumes.
struct VariadicInvocation {
    /// Latest committed variadic definition used.
    def: MacroId,
    /// First token index (the macro name).
    start: usize,
    /// One past the last consumed token index.
    end: usize,
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

/// First-dispatch path: scan, then passthrough-complete or fan out one
/// single-mode child per variadic invocation.
fn dispatch_variadic_stream(input: &PpVariadicInput) -> Vec<Proposal> {
    let verbatim = match mark_verbatim(input) {
        Ok(found) => found,
        Err(diagnostic) => return vec![fail(input.task, diagnostic)],
    };
    let invocations = match scan_variadic_invocations(input, &verbatim) {
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
            kind: PP16_TASK_KIND,
            owner: PP16_CHIP,
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

/// Resume path: under the frozen join this dispatch runs only after every
/// child is terminal. The first `Failed` child fails this task reusing
/// that child's diagnostic (no aggregate minted); stitching runs only in
/// the all-`Completed` case.
fn resume_variadic_stream(input: &PpVariadicInput) -> Vec<Proposal> {
    for (_, state) in &input.children {
        if !state.is_terminal() {
            return vec![fail(
                input.task,
                protocol_fault("variadic resume reads a non-terminal child"),
            )];
        }
    }
    for (id, state) in &input.children {
        if matches!(state, TaskState::Failed(_)) {
            return vec![fail(input.task, first_child_diagnostic(input, *id))];
        }
    }
    if input.children.len() != input.expansions.len() {
        return vec![fail(
            input.task,
            protocol_fault("variadic resume projects an expansion per child"),
        )];
    }
    let verbatim = match mark_verbatim(input) {
        Ok(found) => found,
        Err(diagnostic) => return vec![fail(input.task, diagnostic)],
    };
    let invocations = match scan_variadic_invocations(input, &verbatim) {
        Ok(found) => found,
        Err(diagnostic) => return vec![fail(input.task, diagnostic)],
    };
    if invocations.len() != input.children.len() {
        return vec![fail(
            input.task,
            protocol_fault("variadic resume replays one invocation per child"),
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
                protocol_fault("variadic resume replays an out-of-order invocation"),
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
                    protocol_fault("variadic resume indexes a missing child"),
                )];
            }
        };
        let expansion = match input.expansions.get(position) {
            Some(found) => found,
            None => {
                return vec![fail(
                    input.task,
                    protocol_fault("variadic resume indexes a missing expansion"),
                )];
            }
        };
        if owned.0 != expansion.0 {
            return vec![fail(
                input.task,
                protocol_fault("variadic resume misaligns a child expansion"),
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

/// Reuse the first failed child's committed diagnostic for the join
/// failure (defensive fallback is an internal `Fail`, never silent).
fn first_child_diagnostic(input: &PpVariadicInput, child: TaskId) -> DiagnosticDraft {
    for (known, record) in &input.child_diagnostics {
        if *known == child {
            return DiagnosticDraft {
                severity: record.severity,
                code: record.code,
                message: record.message.clone(),
                span: record.span,
                task: record.task,
            };
        }
    }
    DiagnosticDraft::error(
        DiagnosticCode::new(DiagGroup::Internal, 1),
        format!(
            "variadic resume reuses a missing diagnostic for child {}",
            child.index()
        ),
    )
}

/// Variadic invocation scan in stream order, skipping consumed spans once
/// used. Directive-line tokens pass through (never invocation starts,
/// never collected into argument lists). Variadic-definition uses fan out;
/// non-variadic definitions either pass through (matching arity — PP09
/// owns them) or fail fast as variadic-shaped misuse naming PP16.
/// Unknown identifiers and undefined macros stay plain tokens.
fn scan_variadic_invocations(
    input: &PpVariadicInput,
    verbatim: &[bool],
) -> Result<Vec<VariadicInvocation>, DiagnosticDraft> {
    let mut out: Vec<VariadicInvocation> = Vec::new();
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
            let end = collect_call_end(input, verbatim, next)?;
            if found.1.variadic {
                out.push(VariadicInvocation {
                    def: found.0,
                    start: cursor,
                    end,
                });
            } else {
                check_nonvariadic_arity(input, verbatim, &spelling, &found.1, next)?;
            }
            cursor = end;
        } else if found.1.variadic {
            out.push(VariadicInvocation {
                def: found.0,
                start: cursor,
                end: cursor + 1,
            });
            cursor += 1;
        } else {
            cursor += 1;
        }
    }
    Ok(out)
}

/// Arity gate for a non-variadic function-like definition met in the PP16
/// stream: the call must carry exactly `params.len()` arguments (with the
/// frozen zero-parameter `f()` normalization). Anything else is a
/// variadic-shaped use on a non-variadic definition and fails naming PP16.
fn check_nonvariadic_arity(
    input: &PpVariadicInput,
    verbatim: &[bool],
    spelling: &[u8],
    def: &MacroRecord,
    paren: usize,
) -> Result<(), DiagnosticDraft> {
    let got = count_call_args(input, verbatim, paren)?;
    if got != def.params.len() {
        let text = String::from_utf8_lossy(spelling);
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!(
                "PP16 rejects variadic-shaped use of non-variadic macro `{text}`: expected {} arguments, found {got}",
                def.params.len()
            ),
        ));
    }
    Ok(())
}

/// Count the top-level arguments of the call opening at `paren`
/// (`()` counts zero; otherwise top-level commas plus one).
fn count_call_args(
    input: &PpVariadicInput,
    verbatim: &[bool],
    paren: usize,
) -> Result<usize, DiagnosticDraft> {
    let end = collect_call_end(input, verbatim, paren)?;
    if paren + 1 == end - 1 {
        return Ok(0);
    }
    let mut count = 1usize;
    let mut depth = 0u32;
    let mut cursor = paren + 1;
    while cursor < end - 1 {
        let record = &input.tokens[cursor].1;
        if record.kind == PpTokenKind::Punctuator && record.spelling.as_slice() == b"(" {
            depth = depth.saturating_add(1);
        } else if record.kind == PpTokenKind::Punctuator && record.spelling.as_slice() == b")" {
            depth = depth.saturating_sub(1);
        } else if record.kind == PpTokenKind::Punctuator
            && record.spelling.as_slice() == b","
            && depth == 0
        {
            count = count.saturating_add(1);
        }
        cursor += 1;
    }
    Ok(count)
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
fn open_paren_at(input: &PpVariadicInput, verbatim: &[bool], index: usize) -> bool {
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
    input: &PpVariadicInput,
    name: SpanId,
    paren: SpanId,
) -> Result<bool, DiagnosticDraft> {
    let name_span = match find_span(input, name) {
        Some(found) => found,
        None => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("variadic reads unprojected span {}", name.index()),
            ));
        }
    };
    let paren_span = match find_span(input, paren) {
        Some(found) => found,
        None => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("variadic reads unprojected span {}", paren.index()),
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
    input: &PpVariadicInput,
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

/// Verbatim marking: one flag per stream token, set for tokens on
/// directive lines (which pass through untouched).
fn mark_verbatim(input: &PpVariadicInput) -> Result<Vec<bool>, DiagnosticDraft> {
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

/// Group stream tokens into raw lines: consecutive tokens sharing the same
/// `(source, line)` key form one line, in stream order.
fn group_raw_lines(input: &PpVariadicInput) -> Result<Vec<RawLine>, DiagnosticDraft> {
    let mut lines: Vec<RawLine> = Vec::new();
    for index in 0..input.tokens.len() {
        let span = match find_span(input, input.tokens[index].1.span) {
            Some(found) => found,
            None => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "variadic reads unprojected span {}",
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
                    format!("variadic reads unprojected source {}", span.source.index()),
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
                        "variadic groups an empty line set",
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
fn find_span(input: &PpVariadicInput, id: SpanId) -> Option<SpanRecord> {
    for (known, record) in &input.spans {
        if *known == id {
            return Some(*record);
        }
    }
    None
}

/// Look up projected source bytes by ID.
fn find_source(input: &PpVariadicInput, id: SourceId) -> Option<&[u8]> {
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

/// Directive-line test (`/21` predicate, owned chip-local copy per the
/// shared-helper precedent): the first token is `#`/`%:` AND the raw
/// walk-back from its offset passes. Missing span/source data falls back
/// to non-directive here, but that path is unreachable: `group_raw_lines`
/// above already fails loudly on the same lookups for every token.
fn raw_line_is_directive(input: &PpVariadicInput, line: &RawLine) -> bool {
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

// ---------------------------------------------------------------------------
// Single mode: one variadic substitution (PP12 counterpart).
// ---------------------------------------------------------------------------

/// One working token inside the substitution engine.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Tok {
    /// Preprocessing-token kind.
    kind: PpTokenKind,
    /// Raw spelling bytes.
    spelling: Vec<u8>,
    /// Source span: the committed span for verbatim copies, the invocation
    /// name span for synthesized tokens.
    span: SpanRecord,
    /// Committed ID when this is a verbatim copy (`Some` reuses the ID with
    /// no append); `None` for synthesized tokens appended at materialize.
    committed: Option<PpTokenId>,
    /// Blue-paint spellings blocking expansion of this token.
    paint: Vec<Vec<u8>>,
}

/// Shared engine context: committed definitions plus the body pool.
struct Engine<'a> {
    /// Committed macros in ascending-ID order.
    macros: &'a [(MacroId, MacroRecord)],
    /// Replacement token bodies.
    pool: &'a [(PpTokenId, PpTokenRecord)],
    /// Projected span records (adjacency checks).
    spans: &'a [(SpanId, SpanRecord)],
    /// Invocation name span for synthesized tokens.
    name_span: SpanRecord,
    /// Nesting circuit breaker: committed-macro-count + 2.
    max_depth: usize,
}

/// One detected invocation inside a token stream.
struct FoundInvocation {
    /// Start index of the invocation in the stream.
    pos: usize,
    /// First index past the invocation (name + balanced args).
    end: usize,
    /// Invoked definition body.
    def: MacroRecord,
    /// Raw argument token lists in order.
    args: Vec<Vec<Tok>>,
}

/// Variadic operands for one substitution: the comma-joined tail forms
/// plus the invocation name span for synthesized tokens.
struct VaSub<'a> {
    /// Raw tail joined with synthesized commas (possibly empty).
    joint_raw: &'a [Tok],
    /// Prescanned tail joined with synthesized commas (possibly empty).
    joint_cooked: &'a [Tok],
    /// Invocation name span for synthesized tokens.
    name_span: SpanRecord,
}

/// Single-mode computation: validate the invocation shape against the
/// definition flags, then expand and materialize.
fn compute_variadic_single(input: &PpVariadicInput) -> Vec<Proposal> {
    let name = match input.invocation.first() {
        Some((_, body)) => body,
        None => {
            return vec![fail(
                input.task,
                protocol_fault(format!(
                    "variadic single task {} payload must carry invocation pp-token refs",
                    input.task.index()
                )),
            )];
        }
    };
    if name.kind != PpTokenKind::Identifier || name.spelling != input.def.spelling {
        return vec![fail(
            input.task,
            protocol_fault(format!(
                "variadic single task {} invocation name does not match its definition",
                input.task.index()
            )),
        )];
    }
    if input.def.undefined {
        return vec![fail(
            input.task,
            protocol_fault(format!(
                "variadic single task {} invokes a tombstoned definition",
                input.task.index()
            )),
        )];
    }
    if !input.def.variadic {
        let text = String::from_utf8_lossy(&input.def.spelling);
        return vec![fail(
            input.task,
            DiagnosticDraft::unsupported(format!(
                "PP16 received non-variadic macro `{text}`: non-variadic definitions expand in PP12"
            )),
        )];
    }
    let name_span = match find_span(input, name.span) {
        Some(found) => found,
        None => {
            return vec![fail(
                input.task,
                internal(format!(
                    "variadic reads unprojected span {}",
                    name.span.index()
                )),
            )];
        }
    };
    let engine = Engine {
        macros: &input.macros,
        pool: &input.pool,
        spans: &input.spans,
        name_span,
        max_depth: input.macros.len().saturating_add(2),
    };
    let invocation_toks = match to_toks(&input.invocation, &input.spans) {
        Ok(tokens) => tokens,
        Err(diagnostic) => return vec![fail(input.task, diagnostic)],
    };
    let args_raw = match collect_top_args(&invocation_toks, input.def.function_like) {
        Ok(args) => args,
        Err(diagnostic) => return vec![fail(input.task, diagnostic)],
    };
    let expanded = match expand_invocation(&input.def, args_raw, &[], 1, &engine) {
        Ok(stream) => stream,
        Err(diagnostic) => return vec![fail(input.task, diagnostic)],
    };
    materialize_single(input, &expanded)
}

/// Lift committed bodies into working tokens (verbatim copies, unpainted).
fn to_toks(
    bodies: &[(PpTokenId, PpTokenRecord)],
    spans: &[(SpanId, SpanRecord)],
) -> Result<Vec<Tok>, DiagnosticDraft> {
    let mut out = Vec::with_capacity(bodies.len());
    for (id, body) in bodies {
        let span = match find_span_in(spans, body.span) {
            Some(found) => found,
            None => {
                return Err(internal(format!(
                    "variadic reads unprojected span {}",
                    body.span.index()
                )));
            }
        };
        out.push(Tok {
            kind: body.kind,
            spelling: body.spelling.clone(),
            span,
            committed: Some(*id),
            paint: Vec::new(),
        });
    }
    Ok(out)
}

/// Look up a projected span record by ID (engine lists).
fn find_span_in(spans: &[(SpanId, SpanRecord)], id: SpanId) -> Option<SpanRecord> {
    for (known, body) in spans {
        if *known == id {
            return Some(*body);
        }
    }
    None
}

/// True for the PP06 span-adjacency rule: same source with
/// `name.end == paren.start`.
fn spans_adjacent_tok(name: &SpanRecord, paren: &SpanRecord) -> bool {
    name.source == paren.source && name.end == paren.start
}

/// Collect the top-level invocation arguments against the definition flag:
/// object-like (`function_like == false`) takes exactly `[name]`, and
/// function-like takes `[name, (, ...]` through the matching `)`. A
/// flag/shape mismatch is a protocol fault (the dispatcher guarantees the
/// shape); an unbalanced list or trailing tokens are malformed `Fail`s.
fn collect_top_args(
    invocation: &[Tok],
    function_like: bool,
) -> Result<Vec<Vec<Tok>>, DiagnosticDraft> {
    if !function_like {
        if invocation.len() != 1 {
            return Err(protocol_fault(
                "variadic object invocation must carry exactly the macro name",
            ));
        }
        return Ok(Vec::new());
    }
    if invocation.len() < 2 {
        return Err(protocol_fault(
            "variadic function invocation must carry the macro name and an argument list",
        ));
    }
    let second = match invocation.get(1) {
        Some(found) => found,
        None => {
            return Err(malformed(
                "malformed invocation: expected `(` after the macro name",
            ));
        }
    };
    if !is_open_paren(second) {
        return Err(malformed(
            "malformed invocation: expected `(` after the macro name",
        ));
    }
    let close = match match_close(invocation, 1) {
        Some(found) => found,
        None => {
            return Err(malformed(
                "malformed invocation: unbalanced `(` in the argument list",
            ));
        }
    };
    if close + 1 != invocation.len() {
        return Err(malformed(
            "malformed invocation: trailing tokens past the argument list",
        ));
    }
    let mut inner: Vec<Tok> = Vec::new();
    for (index, tok) in invocation.iter().enumerate() {
        if index >= 2 && index < close {
            inner.push(tok.clone());
        }
    }
    Ok(collect_args(&inner))
}

/// Malformed-invocation typed failure (`Task`, 4).
fn malformed(detail: impl Into<String>) -> DiagnosticDraft {
    DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Task, 4), detail)
}

/// Constraint-violation typed failure (`Task`, 5): `#`/`##` misuse and
/// `__VA_OPT__` shape errors.
fn constraint(detail: impl Into<String>) -> DiagnosticDraft {
    DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Task, 5), detail)
}

/// Internal invariant failure (a projection or engine bug, never panicked).
fn internal(detail: impl Into<String>) -> DiagnosticDraft {
    DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Internal, 1), detail)
}

/// True when the token is the `(` punctuator.
fn is_open_paren(tok: &Tok) -> bool {
    tok.kind == PpTokenKind::Punctuator && tok.spelling.as_slice() == b"("
}

/// True when the token is the `)` punctuator.
fn is_close_paren(tok: &Tok) -> bool {
    tok.kind == PpTokenKind::Punctuator && tok.spelling.as_slice() == b")"
}

/// True when the token is the `,` punctuator.
fn is_comma(tok: &Tok) -> bool {
    tok.kind == PpTokenKind::Punctuator && tok.spelling.as_slice() == b","
}

/// True when the token is the `#` punctuator.
fn is_hash(tok: &Tok) -> bool {
    tok.kind == PpTokenKind::Punctuator && tok.spelling.as_slice() == b"#"
}

/// True when the token is the `##` punctuator.
fn is_paste(tok: &Tok) -> bool {
    tok.kind == PpTokenKind::Punctuator && tok.spelling.as_slice() == b"##"
}

/// True when the token is the identifier `__VA_ARGS__`.
fn is_va_args(tok: &Tok) -> bool {
    tok.kind == PpTokenKind::Identifier && tok.spelling.as_slice() == b"__VA_ARGS__"
}

/// True when the token is the identifier `__VA_OPT__`.
fn is_va_opt(tok: &Tok) -> bool {
    tok.kind == PpTokenKind::Identifier && tok.spelling.as_slice() == b"__VA_OPT__"
}

/// Parameter index when the token is exactly the `params[p]` identifier.
fn param_index(params: &[Vec<u8>], tok: &Tok) -> Option<usize> {
    if tok.kind != PpTokenKind::Identifier {
        return None;
    }
    for (index, param) in params.iter().enumerate() {
        if param.as_slice() == tok.spelling.as_slice() {
            return Some(index);
        }
    }
    None
}

/// Find the `)` matching the `(` at `open` (parens depth scan). `None` when
/// unbalanced.
fn match_close(stream: &[Tok], open: usize) -> Option<usize> {
    let mut depth = 0u32;
    let mut index = open;
    while index < stream.len() {
        let tok = stream.get(index)?;
        if is_open_paren(tok) {
            depth = depth.saturating_add(1);
        }
        if is_close_paren(tok) {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(index);
            }
        }
        index += 1;
    }
    None
}

/// PP10: split a balanced inner argument list on top-level commas. An empty
/// inner list yields one empty argument; the zero-argument call shape is
/// normalized by the caller.
fn collect_args(inner: &[Tok]) -> Vec<Vec<Tok>> {
    let mut args: Vec<Vec<Tok>> = Vec::new();
    args.push(Vec::new());
    let mut depth = 0u32;
    for tok in inner {
        if is_comma(tok) && depth == 0 {
            args.push(Vec::new());
            continue;
        }
        if is_open_paren(tok) {
            depth = depth.saturating_add(1);
        }
        if is_close_paren(tok) {
            depth = depth.saturating_sub(1);
        }
        let last = args.len().saturating_sub(1);
        if last < args.len() {
            args[last].push(tok.clone());
        }
    }
    args
}

/// Latest committed record with spelling equal to `spelling` (`macros` is
/// in ascending-ID order, so the last match wins). `None` when absent.
fn latest_def(macros: &[(MacroId, MacroRecord)], spelling: &[u8]) -> Option<MacroRecord> {
    let mut latest: Option<MacroRecord> = None;
    for (_, body) in macros {
        if body.spelling.as_slice() == spelling {
            latest = Some(body.clone());
        }
    }
    latest
}

/// Look up a pooled replacement body by ID.
fn find_in_pool(pool: &[(PpTokenId, PpTokenRecord)], id: PpTokenId) -> Option<&PpTokenRecord> {
    for (known, body) in pool {
        if *known == id {
            return Some(body);
        }
    }
    None
}

/// True when `spelling` is blue-painted.
fn is_painted(paint: &[Vec<u8>], spelling: &[u8]) -> bool {
    for painted in paint {
        if painted.as_slice() == spelling {
            return true;
        }
    }
    false
}

/// Push `spelling` onto a paint set unless already present.
fn paint_insert(paint: &mut Vec<Vec<u8>>, spelling: &[u8]) {
    if !is_painted(paint, spelling) {
        paint.push(spelling.to_vec());
    }
}

/// Paint every token of a substitution result with the expanded macro.
fn paint_all(stream: &mut [Tok], spelling: &[u8]) {
    for tok in stream {
        paint_insert(&mut tok.paint, spelling);
    }
}

/// Union of two paint sets, order-stable and deduplicated.
fn union_paint(first: &[Vec<u8>], second: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = first.to_vec();
    for spelling in second {
        paint_insert(&mut out, spelling);
    }
    out
}

/// Resolve a definition replacement list to working tokens via the pool.
fn replacement_toks(def: &MacroRecord, engine: &Engine<'_>) -> Result<Vec<Tok>, DiagnosticDraft> {
    let mut out = Vec::with_capacity(def.replacement.len());
    for id in &def.replacement {
        match find_in_pool(engine.pool, *id) {
            Some(body) => {
                let span = match find_span_in(engine.spans, body.span) {
                    Some(found) => found,
                    None => {
                        return Err(internal(format!(
                            "variadic reads unprojected span {}",
                            body.span.index()
                        )));
                    }
                };
                out.push(Tok {
                    kind: body.kind,
                    spelling: body.spelling.clone(),
                    span,
                    committed: Some(*id),
                    paint: Vec::new(),
                });
            }
            None => {
                return Err(internal(format!(
                    "variadic reads unpooled replacement token {}",
                    id.index()
                )));
            }
        }
    }
    Ok(out)
}

/// Per-parameter raw flags: true when the parameter appears as an operand
/// of `#`/`##` in the replacement (those arguments are passed raw). Runs
/// over the `__VA_OPT__`-flattened list so content operands count.
fn raw_param_set(replacement: &[Tok], params: &[Vec<u8>]) -> Vec<bool> {
    let mut raw = vec![false; params.len()];
    let mut index = 0usize;
    while index < replacement.len() {
        let tok = match replacement.get(index) {
            Some(found) => found,
            None => break,
        };
        if is_hash(tok) {
            if let Some(next) = replacement.get(index + 1) {
                if let Some(p) = param_index(params, next) {
                    if p < raw.len() {
                        raw[p] = true;
                    }
                }
            }
        }
        if is_paste(tok) {
            if index > 0 {
                if let Some(prev) = replacement.get(index - 1) {
                    if let Some(p) = param_index(params, prev) {
                        if p < raw.len() {
                            raw[p] = true;
                        }
                    }
                }
            }
            if let Some(next) = replacement.get(index + 1) {
                if let Some(p) = param_index(params, next) {
                    if p < raw.len() {
                        raw[p] = true;
                    }
                }
            }
        }
        index += 1;
    }
    raw
}

/// PP11: fully expand one argument with the same substitute+rescan engine
/// under the current paint set.
fn prescan_arg(
    arg: &[Tok],
    paint: &[Vec<u8>],
    depth: usize,
    engine: &Engine<'_>,
) -> Result<Vec<Tok>, DiagnosticDraft> {
    expand_stream(arg, paint, depth, engine)
}

/// PP13: stringize raw argument tokens into one `StringLiteral`: spellings
/// joined with single spaces, `\` and `"` escaped, quotes wrapped.
fn stringize(raw: &[Tok], name_span: SpanRecord) -> Tok {
    let mut spelling: Vec<u8> = Vec::new();
    spelling.push(b'"');
    let mut first = true;
    for tok in raw {
        if !first {
            spelling.push(b' ');
        }
        first = false;
        for byte in tok.spelling.iter() {
            if *byte == b'\\' || *byte == b'"' {
                spelling.push(b'\\');
            }
            spelling.push(*byte);
        }
    }
    spelling.push(b'"');
    Tok {
        kind: PpTokenKind::StringLiteral,
        spelling,
        span: name_span,
        committed: None,
        paint: Vec::new(),
    }
}

/// Two-or-more character punctuators, longest first (chip-local copy of the
/// frozen scan table; exact equality decides single-token pastes).
const VA_PASTE_MULTI: &[&[u8]] = &[
    b"%:%:", b"...", b"<<=", b">>=", b"<<", b">>", b"<=", b">=", b"==", b"!=", b"&&", b"||", b"++",
    b"--", b"->", b"*=", b"/=", b"%=", b"+=", b"-=", b"&=", b"^=", b"|=", b"##", b"<:", b":>",
    b"<%", b"%>", b"%:",
];

/// Single-character punctuators (chip-local copy of the frozen scan table).
const VA_PASTE_SINGLE: &[u8] = b"[](){}.&*+-~!/%<>^|?:;=,#";

/// True for `[A-Za-z_]` identifier starts.
fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

/// True for `[A-Za-z0-9_]` identifier continuations.
fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// True for the C11 pp-number shape: a leading digit (or `.` plus a digit)
/// followed by pp-number characters (`+`/`-` only after `e`/`E`/`p`/`P`).
fn is_pp_number_shape(spelling: &[u8]) -> bool {
    if spelling.is_empty() {
        return false;
    }
    let mut cursor: usize;
    if spelling[0] == b'.' {
        match spelling.get(1) {
            Some(second) if second.is_ascii_digit() => cursor = 2,
            _ => return false,
        }
    } else if spelling[0].is_ascii_digit() {
        cursor = 1;
    } else {
        return false;
    }
    while cursor < spelling.len() {
        let byte = spelling[cursor];
        if byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'.' {
            cursor += 1;
            continue;
        }
        if (byte == b'+' || byte == b'-')
            && cursor > 0
            && matches!(spelling[cursor - 1], b'e' | b'E' | b'p' | b'P')
        {
            cursor += 1;
            continue;
        }
        return false;
    }
    true
}

/// True for a single quoted literal shape (`quote` is `"` or `'`):
/// opening and closing quotes with escapes honored and no raw newline or
/// unescaped inner quote.
fn is_quoted_shape(spelling: &[u8], quote: u8) -> bool {
    if spelling.len() < 2 {
        return false;
    }
    if spelling[0] != quote {
        return false;
    }
    if spelling[spelling.len() - 1] != quote {
        return false;
    }
    let last = spelling.len() - 1;
    let mut cursor = 1usize;
    while cursor < last {
        let byte = spelling[cursor];
        if byte == b'\n' {
            return false;
        }
        if byte == b'\\' {
            cursor += 1;
            if cursor >= last {
                return false;
            }
            if spelling[cursor] == b'\n' {
                return false;
            }
            cursor += 1;
            continue;
        }
        if byte == quote {
            return false;
        }
        cursor += 1;
    }
    true
}

/// Chip-local paste validator: the concatenated spelling must form exactly
/// one preprocessing token (identifier, pp-number, punctuator, or quoted
/// literal), else `None`. Replaces the frozen-scanner import the PP12
/// engine uses; header names and `Eof` can never arise here.
fn classify_single_token(spelling: &[u8]) -> Option<PpTokenKind> {
    if spelling.is_empty() {
        return None;
    }
    if is_ident_start(spelling[0]) {
        let mut ok = true;
        for byte in spelling.iter().skip(1) {
            if !is_ident_continue(*byte) {
                ok = false;
                break;
            }
        }
        if ok {
            return Some(PpTokenKind::Identifier);
        }
        return None;
    }
    if spelling[0] == b'"' {
        if is_quoted_shape(spelling, b'"') {
            return Some(PpTokenKind::StringLiteral);
        }
        return None;
    }
    if spelling[0] == b'\'' {
        if is_quoted_shape(spelling, b'\'') {
            return Some(PpTokenKind::CharLiteral);
        }
        return None;
    }
    if is_pp_number_shape(spelling) {
        return Some(PpTokenKind::PpNumber);
    }
    for cand in VA_PASTE_MULTI {
        if spelling == *cand {
            return Some(PpTokenKind::Punctuator);
        }
    }
    if spelling.len() == 1 {
        for single in VA_PASTE_SINGLE {
            if spelling[0] == *single {
                return Some(PpTokenKind::Punctuator);
            }
        }
    }
    None
}

/// PP14: combine one `##` paste. An empty side drops to the other side;
/// both empty vanish; two single tokens paste and the concatenated bytes
/// must classify (chip-local validator) as exactly one token, else a
/// constraint `Fail`.
fn paste_seq(
    left: Vec<Tok>,
    right: Vec<Tok>,
    name_span: SpanRecord,
) -> Result<Vec<Tok>, DiagnosticDraft> {
    if left.is_empty() {
        return Ok(right);
    }
    if right.is_empty() {
        return Ok(left);
    }
    if left.len() != 1 || right.len() != 1 {
        return Err(constraint(
            "`##` cannot paste a multi-token operand sequence",
        ));
    }
    let left_tok = match left.first() {
        Some(found) => found,
        None => return Err(internal("paste reads an empty left operand")),
    };
    let right_tok = match right.first() {
        Some(found) => found,
        None => return Err(internal("paste reads an empty right operand")),
    };
    let mut spelling = left_tok.spelling.clone();
    spelling.extend_from_slice(&right_tok.spelling);
    let kind = match classify_single_token(&spelling) {
        Some(found) => found,
        None => return Err(constraint("`##` paste does not form a single pp-token")),
    };
    Ok(vec![Tok {
        kind,
        spelling,
        span: name_span,
        committed: None,
        paint: union_paint(&left_tok.paint, &right_tok.paint),
    }])
}

/// Expand every `__VA_OPT__(content)` node in the replacement (C23 rule):
/// the content (recursively expanded) survives iff the variadic tail holds
/// at least one preprocessing token, else it vanishes. `#` immediately
/// before, or `##` immediately around, a `__VA_OPT__` node is a constraint
/// `Fail`; a missing or unbalanced `(` is malformed.
fn flatten_va_opt(replacement: &[Tok], tail_empty: bool) -> Result<Vec<Tok>, DiagnosticDraft> {
    let mut out: Vec<Tok> = Vec::new();
    let mut index = 0usize;
    while index < replacement.len() {
        let current = match replacement.get(index) {
            Some(found) => found,
            None => return Err(internal("va_opt reads past the replacement list")),
        };
        if is_va_opt(current) {
            if index > 0 {
                if let Some(prev) = replacement.get(index - 1) {
                    if is_hash(prev) {
                        return Err(constraint("`#` cannot stringize `__VA_OPT__`"));
                    }
                }
            }
            let open = match replacement.get(index + 1) {
                Some(found) => found,
                None => {
                    return Err(constraint("`__VA_OPT__` must be followed by `(`"));
                }
            };
            if !is_open_paren(open) {
                return Err(constraint("`__VA_OPT__` must be followed by `(`"));
            }
            let close = match match_close(replacement, index + 1) {
                Some(found) => found,
                None => {
                    return Err(malformed(
                        "malformed `__VA_OPT__`: unbalanced `(` in its content",
                    ));
                }
            };
            if let Some(after) = replacement.get(close + 1) {
                if is_paste(after) {
                    return Err(constraint("`##` cannot paste `__VA_OPT__` directly"));
                }
            }
            if !tail_empty {
                let mut content: Vec<Tok> = Vec::new();
                for (cursor, tok) in replacement.iter().enumerate() {
                    if cursor > index + 1 && cursor < close {
                        content.push(tok.clone());
                    }
                }
                let nested = flatten_va_opt(&content, tail_empty)?;
                out.extend(nested);
            }
            index = close + 1;
            continue;
        }
        if is_paste(current) {
            if let Some(next) = replacement.get(index + 1) {
                if is_va_opt(next) {
                    return Err(constraint("`##` cannot paste `__VA_OPT__` directly"));
                }
            }
        }
        out.push(current.clone());
        index += 1;
    }
    Ok(out)
}

/// True when the variadic tail is used raw anywhere in the flattened
/// replacement: `# __VA_ARGS__` or `__VA_ARGS__` adjacent to `##`
/// (those tail arguments bypass prescan, mirroring PP11).
fn va_uses_raw(flat: &[Tok]) -> bool {
    let mut index = 0usize;
    while index < flat.len() {
        let current = match flat.get(index) {
            Some(found) => found,
            None => break,
        };
        if is_hash(current) {
            if let Some(next) = flat.get(index + 1) {
                if is_va_args(next) {
                    return true;
                }
            }
        }
        if is_paste(current) {
            if index > 0 {
                if let Some(prev) = flat.get(index - 1) {
                    if is_va_args(prev) {
                        return true;
                    }
                }
            }
            if let Some(next) = flat.get(index + 1) {
                if is_va_args(next) {
                    return true;
                }
            }
        }
        index += 1;
    }
    false
}

/// Join argument token lists with synthesized comma separators (the commas
/// carry the invocation name span and no committed ID).
fn join_with_commas(args: &[Vec<Tok>], name_span: SpanRecord) -> Vec<Tok> {
    let mut out: Vec<Tok> = Vec::new();
    let mut first = true;
    for arg in args {
        if !first {
            out.push(Tok {
                kind: PpTokenKind::Punctuator,
                spelling: b",".to_vec(),
                span: name_span,
                committed: None,
                paint: Vec::new(),
            });
        }
        first = false;
        out.extend(arg.iter().cloned());
    }
    out
}

/// Substitute one non-variadic replacement list (frozen PP12 rules,
/// verbatim): parameter occurrences take the prescanned argument (empty
/// arguments contribute NOTHING — placemarker), `# param` stringizes the
/// raw argument, `A ## B` pastes, and a surviving `__VA_ARGS__` is a
/// constraint `Fail` (a non-variadic definition can never bind it).
fn substitute_once(
    params: &[Vec<u8>],
    replacement: &[Tok],
    prescanned: &[Vec<Tok>],
    raw_args: &[Vec<Tok>],
    name_span: SpanRecord,
) -> Result<Vec<Tok>, DiagnosticDraft> {
    let mut out: Vec<Tok> = Vec::new();
    let mut left_is_empty_raw = false;
    let mut index = 0usize;
    while index < replacement.len() {
        let current = match replacement.get(index) {
            Some(found) => found,
            None => return Err(internal("substitute reads past the replacement list")),
        };
        if is_paste(current) {
            let left: Vec<Tok> = if left_is_empty_raw {
                Vec::new()
            } else {
                match out.pop() {
                    Some(found) => vec![found],
                    None => return Err(constraint("`##` has no left operand")),
                }
            };
            left_is_empty_raw = false;
            let next = match replacement.get(index + 1) {
                Some(found) => found,
                None => return Err(constraint("`##` has no right operand")),
            };
            if is_hash(next) || is_paste(next) {
                return Err(constraint("`##` operand must be a pp-token or parameter"));
            }
            if is_va_args(next) || is_va_opt(next) {
                return Err(constraint(
                    "`__VA_ARGS__` requires a variadic definition (PP16)",
                ));
            }
            let right: Vec<Tok> = match param_index(params, next) {
                Some(p) => match raw_args.get(p) {
                    Some(found) => found.clone(),
                    None => {
                        return Err(internal("substitute reads a missing raw argument"));
                    }
                },
                None => vec![next.clone()],
            };
            let pasted = paste_seq(left, right, name_span)?;
            out.extend(pasted);
            index += 2;
            continue;
        }
        if is_hash(current) {
            left_is_empty_raw = false;
            let operand = match replacement.get(index + 1) {
                Some(found) => found,
                None => return Err(constraint("`#` has no operand")),
            };
            let p = match param_index(params, operand) {
                Some(found) => found,
                None => return Err(constraint("`#` operand must be a macro parameter")),
            };
            let raw = match raw_args.get(p) {
                Some(found) => found,
                None => return Err(internal("substitute reads a missing raw argument")),
            };
            out.push(stringize(raw, name_span));
            index += 2;
            continue;
        }
        match param_index(params, current) {
            Some(p) => {
                let following_paste = match replacement.get(index + 1) {
                    Some(next) => is_paste(next),
                    None => false,
                };
                if following_paste {
                    let raw = match raw_args.get(p) {
                        Some(found) => found,
                        None => return Err(internal("substitute reads a missing raw argument")),
                    };
                    left_is_empty_raw = raw.is_empty();
                    out.extend(raw.clone());
                } else {
                    left_is_empty_raw = false;
                    let expanded = match prescanned.get(p) {
                        Some(found) => found,
                        None => {
                            return Err(internal("substitute reads a missing prescanned argument"));
                        }
                    };
                    out.extend(expanded.clone());
                }
                index += 1;
            }
            None => {
                left_is_empty_raw = false;
                if is_va_args(current) || is_va_opt(current) {
                    return Err(constraint(
                        "`__VA_ARGS__` requires a variadic definition (PP16)",
                    ));
                }
                out.push(current.clone());
                index += 1;
            }
        }
    }
    Ok(out)
}

/// Substitute one variadic replacement list (already `__VA_OPT__`
/// flattened): fixed parameters follow the frozen PP12 rules verbatim,
/// while `__VA_ARGS__` splices the joined tail (prescanned, or raw next
/// to `#`/`##`; empty tail contributes NOTHING) and `# __VA_ARGS__`
/// stringizes the joined raw tail.
fn substitute_variadic(
    params: &[Vec<u8>],
    raw_by_orig: &[Vec<Tok>],
    cooked_by_orig: &[Vec<Tok>],
    flat: &[Tok],
    va: &VaSub<'_>,
) -> Result<Vec<Tok>, DiagnosticDraft> {
    let mut out: Vec<Tok> = Vec::new();
    let mut left_is_empty_raw = false;
    let mut va_pending: Option<usize> = None;
    let mut index = 0usize;
    while index < flat.len() {
        let current = match flat.get(index) {
            Some(found) => found,
            None => return Err(internal("substitute reads past the replacement list")),
        };
        if is_paste(current) {
            let left: Vec<Tok> = match va_pending {
                Some(0) => Vec::new(),
                Some(1) => match out.pop() {
                    Some(found) => vec![found],
                    None => return Err(constraint("`##` has no left operand")),
                },
                Some(_) => {
                    return Err(constraint(
                        "`##` cannot paste a multi-token `__VA_ARGS__` sequence",
                    ));
                }
                None => {
                    if left_is_empty_raw {
                        Vec::new()
                    } else {
                        match out.pop() {
                            Some(found) => vec![found],
                            None => return Err(constraint("`##` has no left operand")),
                        }
                    }
                }
            };
            va_pending = None;
            left_is_empty_raw = false;
            let next = match flat.get(index + 1) {
                Some(found) => found,
                None => return Err(constraint("`##` has no right operand")),
            };
            if is_hash(next) || is_paste(next) {
                return Err(constraint("`##` operand must be a pp-token or parameter"));
            }
            if is_va_opt(next) {
                return Err(internal("substitute reads a surviving `__VA_OPT__`"));
            }
            let right: Vec<Tok> = if is_va_args(next) {
                va.joint_raw.to_vec()
            } else {
                match param_index(params, next) {
                    Some(p) => match raw_by_orig.get(p) {
                        Some(found) => found.clone(),
                        None => {
                            return Err(internal("substitute reads a missing raw argument"));
                        }
                    },
                    None => vec![next.clone()],
                }
            };
            let pasted = paste_seq(left, right, va.name_span)?;
            out.extend(pasted);
            index += 2;
            continue;
        }
        if is_hash(current) {
            va_pending = None;
            left_is_empty_raw = false;
            let operand = match flat.get(index + 1) {
                Some(found) => found,
                None => return Err(constraint("`#` has no operand")),
            };
            if is_va_opt(operand) {
                return Err(internal("substitute reads a surviving `__VA_OPT__`"));
            }
            if is_va_args(operand) {
                out.push(stringize(va.joint_raw, va.name_span));
                index += 2;
                continue;
            }
            let p = match param_index(params, operand) {
                Some(found) => found,
                None => return Err(constraint("`#` operand must be a macro parameter")),
            };
            let raw = match raw_by_orig.get(p) {
                Some(found) => found,
                None => return Err(internal("substitute reads a missing raw argument")),
            };
            out.push(stringize(raw, va.name_span));
            index += 2;
            continue;
        }
        if is_va_args(current) {
            left_is_empty_raw = false;
            va_pending = Some(va.joint_cooked.len());
            out.extend(va.joint_cooked.iter().cloned());
            index += 1;
            continue;
        }
        match param_index(params, current) {
            Some(p) => {
                va_pending = None;
                let following_paste = match flat.get(index + 1) {
                    Some(next) => is_paste(next),
                    None => false,
                };
                if following_paste {
                    let raw = match raw_by_orig.get(p) {
                        Some(found) => found,
                        None => return Err(internal("substitute reads a missing raw argument")),
                    };
                    left_is_empty_raw = raw.is_empty();
                    out.extend(raw.clone());
                } else {
                    left_is_empty_raw = false;
                    let expanded = match cooked_by_orig.get(p) {
                        Some(found) => found,
                        None => {
                            return Err(internal("substitute reads a missing prescanned argument"));
                        }
                    };
                    out.extend(expanded.clone());
                }
                index += 1;
            }
            None => {
                va_pending = None;
                left_is_empty_raw = false;
                if is_va_opt(current) {
                    return Err(internal("substitute reads a surviving `__VA_OPT__`"));
                }
                out.push(current.clone());
                index += 1;
            }
        }
    }
    Ok(out)
}

/// Detect every top-level invocation in a stream, left to right, skipping
/// nested spans once consumed: an unpainted `Identifier` with a defined
/// non-tombstone definition is an object invocation when the definition is
/// not function-like, else a function invocation when the next token is a
/// span-adjacent `(` (same source, `name.end == paren.start`) with a
/// balanced close (a non-adjacent or unbalanced `(` is left alone).
fn find_invocations(
    stream: &[Tok],
    paint: &[Vec<u8>],
    engine: &Engine<'_>,
) -> Result<Vec<FoundInvocation>, DiagnosticDraft> {
    let mut found: Vec<FoundInvocation> = Vec::new();
    let mut index = 0usize;
    while index < stream.len() {
        let tok = match stream.get(index) {
            Some(found_tok) => found_tok,
            None => return Err(internal("invocation scan reads past the stream")),
        };
        if tok.kind != PpTokenKind::Identifier {
            index += 1;
            continue;
        }
        let def = match latest_def(engine.macros, &tok.spelling) {
            Some(body) => body,
            None => {
                index += 1;
                continue;
            }
        };
        if def.undefined || is_painted(paint, &def.spelling) {
            index += 1;
            continue;
        }
        if tok.spelling.as_slice() == b"__VA_ARGS__" && !def.variadic {
            index += 1;
            continue;
        }
        if !def.function_like {
            found.push(FoundInvocation {
                pos: index,
                end: index + 1,
                def,
                args: Vec::new(),
            });
            index += 1;
            continue;
        }
        let open = match stream.get(index + 1) {
            Some(found_tok) => found_tok,
            None => {
                index += 1;
                continue;
            }
        };
        if !is_open_paren(open) {
            index += 1;
            continue;
        }
        if !spans_adjacent_tok(&tok.span, &open.span) {
            index += 1;
            continue;
        }
        let close = match match_close(stream, index + 1) {
            Some(found_close) => found_close,
            None => {
                index += 1;
                continue;
            }
        };
        let mut inner: Vec<Tok> = Vec::new();
        for (cursor, inner_tok) in stream.iter().enumerate() {
            if cursor > index + 1 && cursor < close {
                inner.push(inner_tok.clone());
            }
        }
        found.push(FoundInvocation {
            pos: index,
            end: close + 1,
            def,
            args: collect_args(&inner),
        });
        index = close + 1;
    }
    Ok(found)
}

/// Original parameter indices that bind fixed arguments: every
/// `def.params` entry except `__VA_ARGS__` spellings (which alias the
/// variadic tail instead).
fn fixed_origins(params: &[Vec<u8>]) -> Vec<usize> {
    let mut out = Vec::with_capacity(params.len());
    for (index, param) in params.iter().enumerate() {
        if param.as_slice() != b"__VA_ARGS__" {
            out.push(index);
        }
    }
    out
}

/// Expand one invocation of any flavor: paint the expanded macro, prescan
/// every argument except `#`/`##` operands (raw), substitute once
/// (variadic tail handling for variadic definitions, exact-arity frozen
/// rules otherwise), then rescan the result.
fn expand_invocation(
    def: &MacroRecord,
    args_raw: Vec<Vec<Tok>>,
    paint: &[Vec<u8>],
    depth: usize,
    engine: &Engine<'_>,
) -> Result<Vec<Tok>, DiagnosticDraft> {
    if depth > engine.max_depth {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "variadic rescan circuit breaker: nesting exceeds committed-macro-count + 2",
        ));
    }
    if def.variadic {
        return expand_variadic(def, args_raw, paint, depth, engine);
    }
    let mut args = args_raw;
    if args.len() == 1 && args[0].is_empty() && def.params.is_empty() {
        args = Vec::new();
    }
    if args.len() != def.params.len() {
        let text = String::from_utf8_lossy(&def.spelling);
        return Err(malformed(format!(
            "malformed invocation of non-variadic macro `{text}`: expected {} arguments, found {}",
            def.params.len(),
            args.len()
        )));
    }
    expand_nonvariadic(def, args, paint, depth, engine)
}

/// Expand one non-variadic invocation with the frozen PP12 rules verbatim
/// (exact arity is checked by the caller).
fn expand_nonvariadic(
    def: &MacroRecord,
    args: Vec<Vec<Tok>>,
    paint: &[Vec<u8>],
    depth: usize,
    engine: &Engine<'_>,
) -> Result<Vec<Tok>, DiagnosticDraft> {
    let replacement = replacement_toks(def, engine)?;
    let raw_mask = raw_param_set(&replacement, &def.params);
    let mut new_paint: Vec<Vec<u8>> = paint.to_vec();
    paint_insert(&mut new_paint, &def.spelling);
    let mut prescanned: Vec<Vec<Tok>> = Vec::with_capacity(args.len());
    let mut arg_index = 0usize;
    while arg_index < args.len() {
        let arg = match args.get(arg_index) {
            Some(found) => found,
            None => return Err(internal("prescan reads a missing argument")),
        };
        let raw = match raw_mask.get(arg_index) {
            Some(flag) => *flag,
            None => false,
        };
        if raw {
            prescanned.push(arg.clone());
        } else {
            prescanned.push(prescan_arg(arg, &new_paint, depth, engine)?);
        }
        arg_index += 1;
    }
    let mut substituted = substitute_once(
        &def.params,
        &replacement,
        &prescanned,
        &args,
        engine.name_span,
    )?;
    paint_all(&mut substituted, &def.spelling);
    expand_stream(&substituted, &new_paint, depth, engine)
}

/// Expand one variadic invocation: bind at least the fixed arguments (an
/// empty tail is legal), prescan (raw next to `#`/`##`), flatten
/// `__VA_OPT__` by tail emptiness, substitute, paint, and rescan.
fn expand_variadic(
    def: &MacroRecord,
    args_raw: Vec<Vec<Tok>>,
    paint: &[Vec<u8>],
    depth: usize,
    engine: &Engine<'_>,
) -> Result<Vec<Tok>, DiagnosticDraft> {
    let fixed = fixed_origins(&def.params);
    let mut args = args_raw;
    if args.len() == 1 && args[0].is_empty() && fixed.is_empty() {
        args = Vec::new();
    }
    if args.len() < fixed.len() {
        let text = String::from_utf8_lossy(&def.spelling);
        return Err(malformed(format!(
            "malformed invocation of variadic macro `{text}`: expected at least {} arguments, found {}",
            fixed.len(),
            args.len()
        )));
    }
    let mut raw_by_orig: Vec<Vec<Tok>> = Vec::new();
    let mut cooked_by_orig: Vec<Vec<Tok>> = Vec::new();
    for _ in 0..def.params.len() {
        raw_by_orig.push(Vec::new());
        cooked_by_orig.push(Vec::new());
    }
    let replacement = replacement_toks(def, engine)?;
    let mut tail_empty = true;
    for arg in args.iter().skip(fixed.len()) {
        if !arg.is_empty() {
            tail_empty = false;
            break;
        }
    }
    let flat = flatten_va_opt(&replacement, tail_empty)?;
    let raw_mask = raw_param_set(&flat, &def.params);
    let mut new_paint: Vec<Vec<u8>> = paint.to_vec();
    paint_insert(&mut new_paint, &def.spelling);
    let mut cursor = 0usize;
    while cursor < fixed.len() {
        let orig = match fixed.get(cursor) {
            Some(found) => *found,
            None => return Err(internal("variadic binds a missing fixed parameter")),
        };
        let arg = match args.get(cursor) {
            Some(found) => found,
            None => return Err(internal("prescan reads a missing argument")),
        };
        let raw = match raw_mask.get(orig) {
            Some(flag) => *flag,
            None => false,
        };
        let cooked = if raw {
            arg.clone()
        } else {
            prescan_arg(arg, &new_paint, depth, engine)?
        };
        if let Some(slot) = raw_by_orig.get_mut(orig) {
            *slot = arg.clone();
        }
        if let Some(slot) = cooked_by_orig.get_mut(orig) {
            *slot = cooked;
        }
        cursor += 1;
    }
    let tail_raw_all: Vec<Vec<Tok>> = args.iter().skip(fixed.len()).cloned().collect();
    let tail_needs_raw = va_uses_raw(&flat);
    let mut tail_cooked_all: Vec<Vec<Tok>> = Vec::with_capacity(tail_raw_all.len());
    for arg in &tail_raw_all {
        if tail_needs_raw {
            tail_cooked_all.push(arg.clone());
        } else {
            tail_cooked_all.push(prescan_arg(arg, &new_paint, depth, engine)?);
        }
    }
    let joint_raw = join_with_commas(&tail_raw_all, engine.name_span);
    let joint_cooked = join_with_commas(&tail_cooked_all, engine.name_span);
    let va = VaSub {
        joint_raw: &joint_raw,
        joint_cooked: &joint_cooked,
        name_span: engine.name_span,
    };
    let mut substituted =
        substitute_variadic(&def.params, &raw_by_orig, &cooked_by_orig, &flat, &va)?;
    paint_all(&mut substituted, &def.spelling);
    expand_stream(&substituted, &new_paint, depth, engine)
}

/// PP15: rescan loop over a token stream. Each round expands the leftmost
/// remaining unpainted invocation (fully, under its own paint) and splices
/// the result; the spliced region is final because paint sticks to tokens,
/// so every round consumes at least one stream token and the loop ends.
fn expand_stream(
    stream: &[Tok],
    paint: &[Vec<u8>],
    depth: usize,
    engine: &Engine<'_>,
) -> Result<Vec<Tok>, DiagnosticDraft> {
    let mut out: Vec<Tok> = Vec::new();
    let mut rest: Vec<Tok> = stream.to_vec();
    loop {
        let round = find_invocations(&rest, paint, engine)?;
        let inv = match round.first() {
            Some(found) => found,
            None => {
                out.extend(rest);
                break;
            }
        };
        if inv.pos > rest.len() || inv.end > rest.len() || inv.pos >= inv.end {
            return Err(internal("rescan splices an out-of-range invocation"));
        }
        out.extend_from_slice(&rest[..inv.pos]);
        let expanded = expand_invocation(&inv.def, inv.args.clone(), paint, depth + 1, engine)?;
        out.extend(expanded);
        rest = rest[inv.end..].to_vec();
    }
    Ok(out)
}

/// Materialize the expanded stream: verbatim copies reuse committed IDs
/// (no append); synthesized tokens append new `PpToken` records with the
/// invocation name span, with 1:1 predicted IDs.
fn materialize_single(input: &PpVariadicInput, expanded: &[Tok]) -> Vec<Proposal> {
    let name_span = match input.invocation.first() {
        Some((_, body)) => body.span,
        None => {
            return vec![fail(
                input.task,
                protocol_fault("variadic materializes an empty invocation"),
            )]
        }
    };
    let mut records: Vec<RecordDraft> = Vec::new();
    let mut bodies: Vec<G1DraftBody> = Vec::new();
    let mut refs: Vec<RecordRef> = Vec::with_capacity(expanded.len());
    let mut position = 0u32;
    for tok in expanded {
        match tok.committed {
            Some(id) => refs.push(RecordRef::PpToken(id)),
            None => {
                records.push(RecordDraft {
                    family: RecordFamily::PpToken,
                    index: DraftRef(position),
                });
                bodies.push(G1DraftBody::PpToken(PpTokenRecord {
                    kind: tok.kind,
                    span: name_span,
                    spelling: tok.spelling.clone(),
                }));
                refs.push(RecordRef::PpToken(PpTokenId::from_index(
                    input.pp_tokens_allocated.saturating_add(position),
                )));
                position += 1;
            }
        }
    }
    if records.is_empty() {
        return vec![Proposal::Complete {
            task: input.task,
            value: ResultValue::Records(refs),
        }];
    }
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
