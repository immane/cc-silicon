// ============================================================================
// chips/preprocess/pp_substitute.rs — T03 PP12 macro-substitution worker
// (Wave 2 slice 15, `/24`)
//
// Substitutes a single macro invocation (argument prescan, `#`/`##`, blue-
// paint rescan) and completes with the expansion refs. Verbatim replacement
// copies REUSE their committed `PpTokenId`s; synthesized tokens (prescanned,
// pasted, stringized) append new `PpToken` records whose span is the
// invocation name token's span (per-token provenance is deferred to PP27).
//
// PP10 (arg collect), PP11 (arg prescan), PP13 (stringify), PP14 (paste),
// and PP15 (rescan) ship as tested pure helpers in this file, folded per the
// slice doc §6: argument processing and rescan never cross a task boundary —
// they are subroutines of one substitution decision. Promotion requires ALL
// of: an independent consumer task for an intermediate (e.g. cross-tick
// rescan state, cached prescans), plus frozen carriers.
//
// Predicted IDs mirror the `PpScanChip` pattern: the append batch carries
// ONLY `PpToken` bodies, so the Nth synthesized token of the batch predicts
// `pp_tokens_allocated + (earlier PpToken bodies in the batch)`; verbatim
// copies reuse committed IDs and need no prediction.
//
// Frozen rules recorded here:
// - `MacroRecord::function_like` decides the invocation shape: object-like
//   takes exactly `[name]`, function-like takes the paren form; a flag/shape
//   mismatch is a protocol fault (the dispatcher guarantees the shape).
// - Rescan uses real span adjacency: a function-like `(` must satisfy the
//   PP06 rule (same source, `name.span.end == paren.span.start`), so `F (`
//   with a space never expands.
// - Pastes are validated by the frozen `crate::chips::scan`: the
//   concatenated bytes must scan to exactly one token covering every byte.
// ============================================================================

use crate::bus::{MacroRecord, PpTokenKind, PpTokenRecord, SpanRecord};
use crate::chips::{fail, protocol_fault, scan, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{MacroId, PpTokenId, RecordFamily, RecordRef, SpanId, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP12_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// Narrow projection for the macro-substitution computation.
#[derive(Clone, Debug)]
pub struct PpSubstituteInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Invoked definition ID (first payload ref).
    pub def_id: MacroId,
    /// Invoked definition body.
    pub def: MacroRecord,
    /// Invocation token bodies in payload order.
    pub invocation: Vec<(PpTokenId, PpTokenRecord)>,
    /// Body pool for substitution and nested prescan: every pp-token
    /// referenced by the invoked definition's or any committed macro's
    /// replacement list, in ascending-ID order. Required because the
    /// replacement lists name committed IDs whose bodies must be projected.
    pub pool: Vec<(PpTokenId, PpTokenRecord)>,
    /// Span records referenced by the invocation and pool tokens, in
    /// ascending-ID order (adjacency checks and working-token spans).
    pub spans: Vec<(SpanId, SpanRecord)>,
    /// Committed macro definitions in ascending-ID order (latest wins).
    pub macros: Vec<(MacroId, MacroRecord)>,
    /// `pp_tokens` arena count at dispatch (synthesized-append base).
    pub pp_tokens_allocated: u32,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pp_substitute_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpSubstituteInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("substitute of unknown task {}", task.index())))?;
    if record.kind != TaskKind::PREPROCESS_MACRO_SUBSTITUTE {
        return Err(protocol_fault(format!(
            "substitute task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() < 2 {
        return Err(protocol_fault(format!(
            "substitute task {} payload must carry one macro plus invocation pp-token refs",
            task.index()
        )));
    }
    let def_id = match record.payload.refs[0] {
        RecordRef::Macro(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "substitute task {} payload must lead with a macro ref",
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
                    "substitute task {} payload must be pp-token refs after the macro",
                    task.index()
                )));
            }
        }
    }
    let def = bus.arenas.macros.get(def_id).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("substitute reads missing macro {}", def_id.index()),
        )
    })?;
    let mut invocation = Vec::with_capacity(invocation_ids.len());
    for id in invocation_ids {
        match bus.arenas.pp_tokens.get(id) {
            Ok(found) => invocation.push((id, found.clone())),
            Err(_) => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("substitute reads missing pp-token {}", id.index()),
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
                    format!("substitute reads missing pp-token {}", id.index()),
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
                    format!("substitute reads missing span {}", id.index()),
                ));
            }
        }
    }
    Ok(PpSubstituteInput {
        task,
        state: record.state.clone(),
        def_id,
        def: def.clone(),
        invocation,
        pool,
        spans,
        macros,
        pp_tokens_allocated: bus.arenas.pp_tokens.allocated(),
    })
}

/// The T03 macro-substitution worker (PP12 slice scope).
pub struct PpSubstituteChip;

impl Worker for PpSubstituteChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP12_CHIP,
            chip_name: "PpSubstituteChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![TaskKind::PREPROCESS_MACRO_SUBSTITUTE],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Pp, "macros"),
                FieldPath::new(StoreId::Sources, "spans"),
            ],
            writes: vec![FieldPath::new(StoreId::Pp, "tokens")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c24_expand.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pp_substitute_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

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

impl PpSubstituteChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &PpSubstituteInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("substitute task {} is not running", input.task.index()),
                ),
            )];
        }
        let name = match input.invocation.first() {
            Some((_, body)) => body,
            None => {
                return vec![fail(
                    input.task,
                    protocol_fault(format!(
                        "substitute task {} payload must carry invocation pp-token refs",
                        input.task.index()
                    )),
                )];
            }
        };
        if name.kind != PpTokenKind::Identifier || name.spelling != input.def.spelling {
            return vec![fail(
                input.task,
                protocol_fault(format!(
                    "substitute task {} invocation name does not match its definition",
                    input.task.index()
                )),
            )];
        }
        if input.def.undefined {
            return vec![fail(
                input.task,
                protocol_fault(format!(
                    "substitute task {} invokes a tombstoned definition",
                    input.task.index()
                )),
            )];
        }
        let name_span = match find_span(&input.spans, name.span) {
            Some(found) => found,
            None => {
                return vec![fail(
                    input.task,
                    internal(format!(
                        "substitute reads unprojected span {}",
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
        materialize(input, &expanded)
    }
}

/// Lift committed bodies into working tokens (verbatim copies, unpainted).
fn to_toks(
    bodies: &[(PpTokenId, PpTokenRecord)],
    spans: &[(SpanId, SpanRecord)],
) -> Result<Vec<Tok>, DiagnosticDraft> {
    let mut out = Vec::with_capacity(bodies.len());
    for (id, body) in bodies {
        let span = match find_span(spans, body.span) {
            Some(found) => found,
            None => {
                return Err(internal(format!(
                    "substitute reads unprojected span {}",
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

/// Look up a projected span record by ID.
fn find_span(spans: &[(SpanId, SpanRecord)], id: SpanId) -> Option<SpanRecord> {
    for (known, body) in spans {
        if *known == id {
            return Some(*body);
        }
    }
    None
}

/// True for the PP06 span-adjacency rule: same source with
/// `name.end == paren.start`.
fn spans_adjacent(name: &SpanRecord, paren: &SpanRecord) -> bool {
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
                "substitute object invocation must carry exactly the macro name",
            ));
        }
        return Ok(Vec::new());
    }
    if invocation.len() < 2 {
        return Err(protocol_fault(
            "substitute function invocation must carry the macro name and an argument list",
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
/// `__VA_ARGS__` outside a variadic definition.
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
/// inner list yields one empty argument; the zero-parameter `f()` case is
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
                let span = match find_span(engine.spans, body.span) {
                    Some(found) => found,
                    None => {
                        return Err(internal(format!(
                            "substitute reads unprojected span {}",
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
                    "substitute reads unpooled replacement token {}",
                    id.index()
                )));
            }
        }
    }
    Ok(out)
}

/// Per-parameter raw flags: true when the parameter appears as an operand
/// of `#`/`##` in the replacement (those arguments are passed raw).
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

/// PP14: combine one `##` paste. An empty side drops to the other side;
/// both empty vanish; two single tokens paste and the concatenated bytes
/// must scan (frozen scanner) to exactly one token covering every byte,
/// else a constraint `Fail` (the scan diagnostic propagates directly).
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
    let scanned = scan(&spelling)?;
    if scanned.len() != 1 {
        return Err(constraint("`##` paste does not form a single pp-token"));
    }
    let single = match scanned.first() {
        Some(found) => found,
        None => return Err(constraint("`##` paste does not form a single pp-token")),
    };
    if single.start != 0 || single.end != spelling.len() {
        return Err(constraint("`##` paste does not form a single pp-token"));
    }
    Ok(vec![Tok {
        kind: single.kind,
        spelling,
        span: name_span,
        committed: None,
        paint: union_paint(&left_tok.paint, &right_tok.paint),
    }])
}

/// Substitute one replacement list: parameter occurrences take the
/// prescanned argument (empty arguments contribute NOTHING — placemarker),
/// `# param` stringizes the raw argument, `A ## B` pastes, and
/// `__VA_ARGS__` outside a variadic definition is a constraint `Fail`.
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
            if is_va_args(next) {
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
                if is_va_args(current) {
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
        if !spans_adjacent(&tok.span, &open.span) {
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

/// Expand one invocation: paint the expanded macro, prescan every argument
/// except `#`/`##` operands (raw), substitute once, then rescan the result.
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
            "substitute rescan circuit breaker: nesting exceeds committed-macro-count + 2",
        ));
    }
    if def.variadic {
        return Err(DiagnosticDraft::unsupported(
            "variadic macro expansion is deferred to PP16",
        ));
    }
    let mut args = args_raw;
    if args.len() == 1 && args[0].is_empty() && def.params.is_empty() {
        args = Vec::new();
    }
    if args.len() != def.params.len() {
        return Err(malformed(format!(
            "malformed invocation: expected {} arguments, found {}",
            def.params.len(),
            args.len()
        )));
    }
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
fn materialize(input: &PpSubstituteInput, expanded: &[Tok]) -> Vec<Proposal> {
    let name_span = match input.invocation.first() {
        Some((_, body)) => body.span,
        None => {
            return vec![fail(
                input.task,
                protocol_fault("substitute materializes an empty invocation"),
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
