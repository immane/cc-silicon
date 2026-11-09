// ============================================================================
// chips/preprocess/pp_conditional.rs — T03 PP19 conditional-inclusion
// worker (Wave 2 slice 13, `/22`)
//
// Reads all committed pp-token refs, tracks the conditional stack over
// directive lines (recognized with the `/21` raw walk-back predicate),
// evaluates `#if`/`#elif` expressions with a chip-local PP-int evaluator
// (recursive descent plus `defined`, folded from the PP20–PP22 catalog
// rows — see `PP_CONDITIONAL_SLICE.md` §6 for the promotion criteria;
// `/23` amendment: `defined` and `#ifdef` read the committed macro table;
// B3-01: `#if`/`#elif` expand committed object-like identifiers before
// integer evaluation, with a blue-paint set and bounded depth),
// and completes `Records` of the active-line token refs. Conditional
// directive lines are fully consumed here (never kept); active
// non-conditional lines — directives like `#error` included — pass
// through verbatim for downstream PP05/PP06; inactive lines are dropped;
// the EOF ref is always kept. No writes, no children, no joins.
// ============================================================================

use crate::bus::{MacroRecord, PpTokenKind, PpTokenRecord, SpanRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{MacroId, PpTokenId, RecordRef, SourceId, SpanId, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP19_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

/// Narrow projection for the conditional-inclusion computation.
#[derive(Clone, Debug)]
pub struct PpConditionalInput {
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
    /// Committed macro definitions in ascending-ID order (latest wins).
    pub macros: Vec<(MacroId, MacroRecord)>,
    /// Committed pp-tokens referenced by macro replacement lists, in
    /// first-use order over the ascending-ID macro table.
    pub macro_tokens: Vec<(PpTokenId, PpTokenRecord)>,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pp_conditional_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpConditionalInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("conditional of unknown task {}", task.index())))?;
    if record.kind != TaskKind::PREPROCESS_CONDITIONAL {
        return Err(protocol_fault(format!(
            "conditional task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.is_empty() {
        return Err(protocol_fault(format!(
            "conditional task {} payload must carry pp-token refs",
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
                        format!("conditional reads missing pp-token {}", id.index()),
                    ));
                }
            },
            _ => {
                return Err(protocol_fault(format!(
                    "conditional task {} payload must be pp-token refs",
                    task.index()
                )));
            }
        }
    }
    let mut spans: Vec<(SpanId, SpanRecord)> = Vec::new();
    for (_, token) in &tokens {
        if spans.iter().all(|(known, _)| *known != token.span) {
            match bus.arenas.spans.get(token.span) {
                Ok(found) => spans.push((token.span, *found)),
                Err(_) => {
                    return Err(DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("conditional reads missing span {}", token.span.index()),
                    ));
                }
            }
        }
    }
    let mut sources: Vec<(SourceId, Vec<u8>)> = Vec::new();
    for (_, span) in &spans {
        if sources.iter().all(|(known, _)| *known != span.source) {
            match bus.arenas.sources.get(span.source) {
                Ok(found) => sources.push((span.source, found.bytes.clone())),
                Err(_) => {
                    return Err(DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("conditional reads missing source {}", span.source.index()),
                    ));
                }
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
    let mut macro_tokens: Vec<(PpTokenId, PpTokenRecord)> = Vec::new();
    for (_, record) in &macros {
        for id in &record.replacement {
            if macro_tokens.iter().any(|(known, _)| known == id) {
                continue;
            }
            match bus.arenas.pp_tokens.get(*id) {
                Ok(found) => macro_tokens.push((*id, found.clone())),
                Err(_) => {
                    return Err(DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!(
                            "conditional reads missing macro replacement pp-token {}",
                            id.index()
                        ),
                    ));
                }
            }
        }
    }
    Ok(PpConditionalInput {
        task,
        state: record.state.clone(),
        tokens,
        spans,
        sources,
        macros,
        macro_tokens,
    })
}

/// The T03 conditional-inclusion worker (PP19 slice scope).
pub struct PpConditionalChip;

impl Worker for PpConditionalChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP19_CHIP,
            chip_name: "PpConditionalChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![TaskKind::PREPROCESS_CONDITIONAL],
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
            ],
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec![
                "compiler/tests/c22_conditional.rs",
                "compiler/tests/c43_pp_if_macro.rs",
            ],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pp_conditional_input(bus, task) {
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

/// One open conditional level.
#[derive(Clone, Copy, Debug)]
struct CondLevel {
    /// Enclosing region activity when the level opened.
    parent_active: bool,
    /// A branch already taken at this level.
    taken: bool,
    /// `#else` already seen at this level.
    else_seen: bool,
}

impl PpConditionalChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &PpConditionalInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("conditional task {} is not running", input.task.index()),
                ),
            )];
        }
        let lines = match group_raw_lines(input) {
            Ok(lines) => lines,
            Err(diagnostic) => return vec![fail(input.task, diagnostic)],
        };
        let mut kept: Vec<RecordRef> = Vec::new();
        let mut stack: Vec<CondLevel> = Vec::new();
        let mut active = true;
        let mut eof: Option<RecordRef> = None;
        for line in &lines {
            for index in line.members.iter() {
                if input.tokens[*index].1.kind == PpTokenKind::Eof {
                    eof = Some(RecordRef::PpToken(input.tokens[*index].0));
                }
            }
            if !raw_line_is_directive(input, line) {
                if active {
                    push_line_refs(input, line, &mut kept);
                }
                continue;
            }
            if line.members.len() == 1 {
                continue;
            }
            let name = match directive_name(input, line) {
                Ok(name) => name,
                Err(diagnostic) => return vec![fail(input.task, diagnostic)],
            };
            if is_conditional_name(&name) {
                if let Err(diagnostic) =
                    conditional_step(input, &mut stack, &mut active, &name, line)
                {
                    return vec![fail(input.task, diagnostic)];
                }
                continue;
            }
            if active {
                push_line_refs(input, line, &mut kept);
            }
        }
        if !stack.is_empty() {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "unterminated conditional: `#endif` missing",
                ),
            )];
        }
        match eof {
            Some(reference) => {
                kept.push(reference);
                vec![Proposal::Complete {
                    task: input.task,
                    value: ResultValue::Records(kept),
                }]
            }
            None => {
                vec![fail(
                    input.task,
                    DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        "conditional reads a stream with no EOF token",
                    ),
                )]
            }
        }
    }
}

/// Append one line's token refs, in order (the EOF token is excluded:
/// it is always appended once at the end, whatever line it sits on).
fn push_line_refs(input: &PpConditionalInput, line: &RawLine, kept: &mut Vec<RecordRef>) {
    for index in line.members.iter() {
        if input.tokens[*index].1.kind == PpTokenKind::Eof {
            continue;
        }
        kept.push(RecordRef::PpToken(input.tokens[*index].0));
    }
}

/// True for the six conditional directive names (consumed here, never kept).
fn is_conditional_name(name: &[u8]) -> bool {
    name == b"if"
        || name == b"ifdef"
        || name == b"ifndef"
        || name == b"elif"
        || name == b"else"
        || name == b"endif"
}

/// One conditional-directive step: update the stack and current activity.
/// Conditional lines are consumed (never kept). Expressions evaluate even
/// in inactive regions (fail-closed); malformed lines fail everywhere.
fn conditional_step(
    input: &PpConditionalInput,
    stack: &mut Vec<CondLevel>,
    active: &mut bool,
    name: &[u8],
    line: &RawLine,
) -> Result<(), DiagnosticDraft> {
    let err = |detail: &str| {
        DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Task, 4), detail.to_string())
    };
    if name == b"if" {
        let tokens = expr_tokens(input, line)?;
        let expanded = expand_condition_tokens(&tokens, &input.macros, &input.macro_tokens)?;
        let value = eval_pp_expr(&expanded, &input.macros)?;
        let take = *active && value != 0;
        stack.push(CondLevel {
            parent_active: *active,
            taken: take,
            else_seen: false,
        });
        *active = take;
        return Ok(());
    }
    if name == b"ifdef" || name == b"ifndef" {
        if line.members.len() != 3 {
            return Err(err("malformed conditional: expected exactly one name"));
        }
        let record = &input.tokens[line.members[2]].1;
        if record.kind != PpTokenKind::Identifier {
            return Err(err("malformed conditional: expected an identifier"));
        }
        // `/23` amendment of the `/22` frozen-`false` rule: the committed
        // table decides (latest record wins; tombstones count as absent).
        let defined = macro_defined(input, &record.spelling);
        let take = *active && (defined == (name == b"ifdef"));
        stack.push(CondLevel {
            parent_active: *active,
            taken: take,
            else_seen: false,
        });
        *active = take;
        return Ok(());
    }
    if name == b"elif" {
        if line.members.len() < 3 {
            return Err(err("malformed conditional: expected an expression"));
        }
        let level = match stack.last_mut() {
            Some(level) => level,
            None => return Err(err("`#elif` without `#if`")),
        };
        if level.else_seen {
            return Err(err("`#elif` after `#else`"));
        }
        let tokens = expr_tokens(input, line)?;
        let expanded = expand_condition_tokens(&tokens, &input.macros, &input.macro_tokens)?;
        let value = eval_pp_expr(&expanded, &input.macros)?;
        let take = !level.taken && level.parent_active && value != 0;
        level.taken = level.taken || take;
        *active = take;
        return Ok(());
    }
    if name == b"else" {
        if line.members.len() != 2 {
            return Err(err("malformed conditional: trailing tokens after `#else`"));
        }
        let level = match stack.last_mut() {
            Some(level) => level,
            None => return Err(err("`#else` without `#if`")),
        };
        if level.else_seen {
            return Err(err("multiple `#else`"));
        }
        level.else_seen = true;
        let take = level.parent_active && !level.taken;
        level.taken = true;
        *active = take;
        return Ok(());
    }
    if name == b"endif" {
        if line.members.len() != 2 {
            return Err(err("malformed conditional: trailing tokens after `#endif`"));
        }
        match stack.pop() {
            Some(_) => {}
            None => return Err(err("`#endif` without `#if`")),
        }
        *active = match stack.last() {
            Some(level) => level.parent_active && level.taken,
            None => true,
        };
        return Ok(());
    }
    Err(err(
        "internal error: non-conditional name reached the conditional step",
    ))
}

/// Expression token refs of an `#if`/`#elif` line (after `#` and the name).
fn expr_tokens(
    input: &PpConditionalInput,
    line: &RawLine,
) -> Result<Vec<PpTokenRecord>, DiagnosticDraft> {
    if line.members.len() < 3 {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "malformed conditional: expected an expression",
        ));
    }
    let mut out = Vec::new();
    for index in line.members.iter().skip(2) {
        out.push(input.tokens[*index].1.clone());
    }
    Ok(out)
}

/// Group payload tokens into raw lines: consecutive tokens sharing the same
/// `(source, line)` key form one line, in stream order.
fn group_raw_lines(input: &PpConditionalInput) -> Result<Vec<RawLine>, DiagnosticDraft> {
    let err =
        |detail: String| DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Task, 4), detail);
    let mut lines: Vec<RawLine> = Vec::new();
    for index in 0..input.tokens.len() {
        let span = match find_span(input, input.tokens[index].1.span) {
            Some(found) => found,
            None => {
                return Err(err(format!(
                    "conditional reads unprojected span {}",
                    input.tokens[index].1.span.index()
                )));
            }
        };
        let bytes = match find_source(input, span.source) {
            Some(found) => found,
            None => {
                return Err(err(format!(
                    "conditional reads unprojected source {}",
                    span.source.index()
                )));
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
                    return Err(err("conditional groups an empty line set".to_string()));
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
fn find_span(input: &PpConditionalInput, id: SpanId) -> Option<SpanRecord> {
    for (known, record) in &input.spans {
        if *known == id {
            return Some(*record);
        }
    }
    None
}

/// Look up projected source bytes by ID.
fn find_source(input: &PpConditionalInput, id: SourceId) -> Option<&[u8]> {
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

/// Directive-line test (`/21` predicate): the first token is `#`/`%:` AND
/// the raw walk-back from its offset passes.
fn raw_line_is_directive(input: &PpConditionalInput, line: &RawLine) -> bool {
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

/// Directive name: the next token when it is an `Identifier`, else a
/// malformed-line typed error.
fn directive_name(input: &PpConditionalInput, line: &RawLine) -> Result<Vec<u8>, DiagnosticDraft> {
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

/// True when the committed macro table defines this spelling: the latest
/// record with equal spelling exists and is not a tombstone.
fn macro_defined(input: &PpConditionalInput, spelling: &[u8]) -> bool {
    match latest_macro(&input.macros, spelling) {
        Some(record) => !record.undefined,
        None => false,
    }
}

/// Free-function twin for the expression parser (which cannot borrow the
/// whole input): same latest-wins rule over committed macro bodies.
fn defined_in(macros: &[(MacroId, MacroRecord)], spelling: &[u8]) -> i128 {
    match latest_macro(macros, spelling) {
        Some(record) if !record.undefined => 1,
        _ => 0,
    }
}

/// Expansion budget per `#if`/`#elif` expression. The blue-paint set already
/// terminates recursion; this bounds exponential replacement fan-out so a
/// pathological committed table fails deterministically instead of spinning.
const CONDITIONAL_EXPANSION_BUDGET: usize = 1 << 16;

/// Latest committed definition for a spelling (`macros` is ascending-ID, so
/// the last match wins). Tombstones are returned and treated as undefined by
/// callers.
fn latest_macro<'a>(
    macros: &'a [(MacroId, MacroRecord)],
    spelling: &[u8],
) -> Option<&'a MacroRecord> {
    let mut latest: Option<&'a MacroRecord> = None;
    for (_, record) in macros {
        if record.spelling.as_slice() == spelling {
            latest = Some(record);
        }
    }
    latest
}

/// Look up one committed replacement-list token in the projected pool.
fn find_macro_token(pool: &[(PpTokenId, PpTokenRecord)], id: PpTokenId) -> Option<&PpTokenRecord> {
    pool.iter()
        .find(|(known, _)| *known == id)
        .map(|(_, record)| record)
}

/// True when `spelling` is already blue-painted in the current expansion
/// chain (C11 6.10.3.4 rescan/hide-set for the object-like case).
fn is_painted(paint: &[Vec<u8>], spelling: &[u8]) -> bool {
    paint.iter().any(|painted| painted.as_slice() == spelling)
}

/// Add `spelling` to the paint set unless already present.
fn paint_insert(paint: &mut Vec<Vec<u8>>, spelling: &[u8]) {
    if !is_painted(paint, spelling) {
        paint.push(spelling.to_vec());
    }
}

/// C11 6.10.1 replacement pass for one `#if`/`#elif` controlling expression.
/// `defined` operands are copied verbatim; other object-like macro names are
/// replaced and rescanned under a blue-paint hide set. Function-like
/// invocations are outside this slice and fail loudly rather than silently
/// evaluating the macro name as zero.
fn expand_condition_tokens(
    tokens: &[PpTokenRecord],
    macros: &[(MacroId, MacroRecord)],
    pool: &[(PpTokenId, PpTokenRecord)],
) -> Result<Vec<PpTokenRecord>, DiagnosticDraft> {
    let mut expander = ConditionalExpander {
        macros,
        pool,
        max_depth: macros.len().saturating_add(2),
        budget: CONDITIONAL_EXPANSION_BUDGET,
    };
    let mut out = Vec::new();
    expander.expand_stream(tokens, &[], 0, &mut out)?;
    Ok(out)
}

/// Object-like expansion state for one controlling expression.
struct ConditionalExpander<'a> {
    macros: &'a [(MacroId, MacroRecord)],
    pool: &'a [(PpTokenId, PpTokenRecord)],
    /// Nesting bound: committed-macro-count + 2 (mirrors PP12's breaker).
    max_depth: usize,
    /// Remaining macro expansions across the whole expression.
    budget: usize,
}

impl ConditionalExpander<'_> {
    fn err(&self, detail: &str) -> DiagnosticDraft {
        DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Task, 4), detail.to_string())
    }

    fn expand_stream(
        &mut self,
        tokens: &[PpTokenRecord],
        paint: &[Vec<u8>],
        depth: usize,
        out: &mut Vec<PpTokenRecord>,
    ) -> Result<(), DiagnosticDraft> {
        let mut index = 0usize;
        while index < tokens.len() {
            let token = &tokens[index];
            if token.kind == PpTokenKind::Identifier && token.spelling.as_slice() == b"defined" {
                copy_defined_operand(tokens, &mut index, out);
                continue;
            }
            if token.kind == PpTokenKind::Identifier {
                if let Some(def) = latest_macro(self.macros, &token.spelling).cloned() {
                    if !def.undefined && !is_painted(paint, &def.spelling) {
                        if def.function_like {
                            if next_is_open_paren(tokens, index + 1) {
                                return Err(DiagnosticDraft::unsupported(
                                    "function-like macro invocation in `#if`/`#elif` is deferred past this slice",
                                ));
                            }
                        } else {
                            if depth >= self.max_depth {
                                return Err(self.err(
                                    "conditional macro expansion exceeds the committed-macro-count bound",
                                ));
                            }
                            if self.budget == 0 {
                                return Err(
                                    self.err("conditional macro expansion budget exhausted")
                                );
                            }
                            self.budget -= 1;
                            let replacement = replacement_records(&def, self.pool)?;
                            let mut nested = paint.to_vec();
                            paint_insert(&mut nested, &def.spelling);
                            self.expand_stream(&replacement, &nested, depth + 1, out)?;
                            index += 1;
                            continue;
                        }
                    }
                }
            }
            out.push(token.clone());
            index += 1;
        }
        Ok(())
    }
}

/// Copy `defined` plus its operand form verbatim so macro replacement cannot
/// rewrite the name the operator inspects.
fn copy_defined_operand(tokens: &[PpTokenRecord], index: &mut usize, out: &mut Vec<PpTokenRecord>) {
    let defined = match tokens.get(*index) {
        Some(token) => token.clone(),
        None => return,
    };
    out.push(defined);
    *index += 1;
    let next = match tokens.get(*index) {
        Some(token) => token.clone(),
        None => return,
    };
    if next.kind == PpTokenKind::Punctuator && next.spelling.as_slice() == b"(" {
        out.push(next);
        *index += 1;
        if let Some(operand) = tokens.get(*index) {
            out.push(operand.clone());
            *index += 1;
        }
        if let Some(close) = tokens.get(*index) {
            if close.kind == PpTokenKind::Punctuator && close.spelling.as_slice() == b")" {
                out.push(close.clone());
                *index += 1;
            }
        }
    } else {
        out.push(next);
        *index += 1;
    }
}

/// True when the next token is an invocation-opening `(`. Whitespace is not
/// significant for a function-like call (unlike the `#define` shape).
fn next_is_open_paren(tokens: &[PpTokenRecord], index: usize) -> bool {
    match tokens.get(index) {
        Some(token) => token.kind == PpTokenKind::Punctuator && token.spelling.as_slice() == b"(",
        None => false,
    }
}

/// Resolve one definition's replacement list through the projected pool.
fn replacement_records(
    def: &MacroRecord,
    pool: &[(PpTokenId, PpTokenRecord)],
) -> Result<Vec<PpTokenRecord>, DiagnosticDraft> {
    let mut out = Vec::with_capacity(def.replacement.len());
    for id in &def.replacement {
        match find_macro_token(pool, *id) {
            Some(record) => out.push(record.clone()),
            None => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "conditional reads unprojected macro replacement token {}",
                        id.index()
                    ),
                ));
            }
        }
    }
    Ok(out)
}

/// Evaluate one `#if`/`#elif` expression (pp-token records, stream order)
/// to an exact `i128`. The caller has already performed C11 macro
/// replacement. Both sides of `&&`/`||`/`?:` always evaluate (fail-closed);
/// division/modulo by zero, bad shifts, and overflow fail.
fn eval_pp_expr(
    tokens: &[PpTokenRecord],
    macros: &[(MacroId, MacroRecord)],
) -> Result<i128, DiagnosticDraft> {
    let mut parser = ExprParser {
        tokens,
        macros,
        pos: 0,
    };
    let value = parser.parse_conditional()?;
    if parser.pos != tokens.len() {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "malformed expression: trailing tokens",
        ));
    }
    Ok(value)
}

/// Recursive-descent PP-int parser over pp-token records (frozen §3).
struct ExprParser<'a> {
    tokens: &'a [PpTokenRecord],
    macros: &'a [(MacroId, MacroRecord)],
    pos: usize,
}

impl<'a> ExprParser<'a> {
    fn err(&self, detail: &str) -> DiagnosticDraft {
        DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Task, 4), detail.to_string())
    }

    fn peek(&self) -> Option<&'a PpTokenRecord> {
        self.tokens.get(self.pos)
    }

    /// Consume a punctuator with one of the given spellings.
    fn eat(&mut self, spellings: &[&[u8]]) -> bool {
        match self.peek() {
            Some(record)
                if record.kind == PpTokenKind::Punctuator
                    && spellings.contains(&record.spelling.as_slice()) =>
            {
                self.pos += 1;
                true
            }
            _ => false,
        }
    }

    fn parse_conditional(&mut self) -> Result<i128, DiagnosticDraft> {
        let cond = self.parse_or()?;
        if !self.eat(&[b"?"]) {
            return Ok(cond);
        }
        let taken = self.parse_conditional()?;
        if !self.eat(&[b":"]) {
            return Err(self.err("malformed expression: expected `:`"));
        }
        let skipped = self.parse_conditional()?;
        if cond != 0 {
            Ok(taken)
        } else {
            Ok(skipped)
        }
    }

    fn parse_or(&mut self) -> Result<i128, DiagnosticDraft> {
        let mut value = self.parse_and()?;
        while self.eat(&[b"||"]) {
            let rhs = self.parse_and()?;
            value = if value != 0 || rhs != 0 { 1 } else { 0 };
        }
        Ok(value)
    }

    fn parse_and(&mut self) -> Result<i128, DiagnosticDraft> {
        let mut value = self.parse_bitor()?;
        while self.eat(&[b"&&"]) {
            let rhs = self.parse_bitor()?;
            value = if value != 0 && rhs != 0 { 1 } else { 0 };
        }
        Ok(value)
    }

    fn parse_bitor(&mut self) -> Result<i128, DiagnosticDraft> {
        let mut value = self.parse_bitxor()?;
        while self.eat(&[b"|"]) {
            let rhs = self.parse_bitxor()?;
            value |= rhs;
        }
        Ok(value)
    }

    fn parse_bitxor(&mut self) -> Result<i128, DiagnosticDraft> {
        let mut value = self.parse_bitand()?;
        while self.eat(&[b"^"]) {
            let rhs = self.parse_bitand()?;
            value ^= rhs;
        }
        Ok(value)
    }

    fn parse_bitand(&mut self) -> Result<i128, DiagnosticDraft> {
        let mut value = self.parse_eq()?;
        while self.eat(&[b"&"]) {
            let rhs = self.parse_eq()?;
            value &= rhs;
        }
        Ok(value)
    }

    fn parse_eq(&mut self) -> Result<i128, DiagnosticDraft> {
        let mut value = self.parse_rel()?;
        loop {
            if self.eat(&[b"=="]) {
                let rhs = self.parse_rel()?;
                value = if value == rhs { 1 } else { 0 };
            } else if self.eat(&[b"!="]) {
                let rhs = self.parse_rel()?;
                value = if value != rhs { 1 } else { 0 };
            } else {
                return Ok(value);
            }
        }
    }

    fn parse_rel(&mut self) -> Result<i128, DiagnosticDraft> {
        let mut value = self.parse_shift()?;
        loop {
            if self.eat(&[b"<="]) {
                let rhs = self.parse_shift()?;
                value = if value <= rhs { 1 } else { 0 };
            } else if self.eat(&[b">="]) {
                let rhs = self.parse_shift()?;
                value = if value >= rhs { 1 } else { 0 };
            } else if self.eat(&[b"<"]) {
                let rhs = self.parse_shift()?;
                value = if value < rhs { 1 } else { 0 };
            } else if self.eat(&[b">"]) {
                let rhs = self.parse_shift()?;
                value = if value > rhs { 1 } else { 0 };
            } else {
                return Ok(value);
            }
        }
    }

    fn parse_shift(&mut self) -> Result<i128, DiagnosticDraft> {
        let mut value = self.parse_add()?;
        loop {
            if self.eat(&[b"<<"]) {
                let rhs = self.parse_add()?;
                value = shift_left(value, rhs).ok_or_else(|| self.err("bad shift"))?;
            } else if self.eat(&[b">>"]) {
                let rhs = self.parse_add()?;
                value = shift_right(value, rhs).ok_or_else(|| self.err("bad shift"))?;
            } else {
                return Ok(value);
            }
        }
    }

    fn parse_add(&mut self) -> Result<i128, DiagnosticDraft> {
        let mut value = self.parse_mul()?;
        loop {
            if self.eat(&[b"+"]) {
                let rhs = self.parse_mul()?;
                value = value
                    .checked_add(rhs)
                    .ok_or_else(|| self.err("integer overflow in conditional"))?;
            } else if self.eat(&[b"-"]) {
                let rhs = self.parse_mul()?;
                value = value
                    .checked_sub(rhs)
                    .ok_or_else(|| self.err("integer overflow in conditional"))?;
            } else {
                return Ok(value);
            }
        }
    }

    fn parse_mul(&mut self) -> Result<i128, DiagnosticDraft> {
        let mut value = self.parse_unary()?;
        loop {
            if self.eat(&[b"*"]) {
                let rhs = self.parse_unary()?;
                value = value
                    .checked_mul(rhs)
                    .ok_or_else(|| self.err("integer overflow in conditional"))?;
            } else if self.eat(&[b"/"]) {
                let rhs = self.parse_unary()?;
                if rhs == 0 {
                    return Err(self.err("division by zero in conditional"));
                }
                value = value
                    .checked_div(rhs)
                    .ok_or_else(|| self.err("integer overflow in conditional"))?;
            } else if self.eat(&[b"%"]) {
                let rhs = self.parse_unary()?;
                if rhs == 0 {
                    return Err(self.err("division by zero in conditional"));
                }
                value = value
                    .checked_rem(rhs)
                    .ok_or_else(|| self.err("integer overflow in conditional"))?;
            } else {
                return Ok(value);
            }
        }
    }

    fn parse_unary(&mut self) -> Result<i128, DiagnosticDraft> {
        if self.eat(&[b"+"]) {
            return self.parse_unary();
        }
        if self.eat(&[b"-"]) {
            let value = self.parse_unary()?;
            return value
                .checked_neg()
                .ok_or_else(|| self.err("integer overflow in conditional"));
        }
        if self.eat(&[b"~"]) {
            let value = self.parse_unary()?;
            return Ok(!value);
        }
        if self.eat(&[b"!"]) {
            let value = self.parse_unary()?;
            return Ok(if value == 0 { 1 } else { 0 });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<i128, DiagnosticDraft> {
        let record = match self.peek() {
            Some(record) => record.clone(),
            None => return Err(self.err("malformed expression: unexpected end")),
        };
        match record.kind {
            PpTokenKind::PpNumber => {
                self.pos += 1;
                match parse_int_literal(&record.spelling) {
                    Some(value) => Ok(value),
                    None => Err(self.err("malformed integer in conditional")),
                }
            }
            PpTokenKind::Identifier if record.spelling.as_slice() == b"defined" => {
                self.pos += 1;
                self.parse_defined()
            }
            PpTokenKind::Identifier => {
                self.pos += 1;
                Ok(0)
            }
            PpTokenKind::Punctuator if record.spelling.as_slice() == b"(" => {
                self.pos += 1;
                let value = self.parse_conditional()?;
                if !self.eat(&[b")"]) {
                    return Err(self.err("malformed expression: expected `)`"));
                }
                Ok(value)
            }
            PpTokenKind::CharLiteral => Err(DiagnosticDraft::unsupported(
                "character constants in conditionals are deferred past this slice",
            )),
            _ => Err(self.err("malformed expression: unexpected token")),
        }
    }

    /// `defined identifier` or `defined ( identifier )` against the
    /// committed table (latest record wins; tombstones count as absent);
    /// malformed uses fail loudly.
    fn parse_defined(&mut self) -> Result<i128, DiagnosticDraft> {
        // Copy the spelling out first: `peek` borrows `self`, and the
        // table read must not alias that borrow.
        if self.eat(&[b"("]) {
            let spelling = match self.peek() {
                Some(record)
                    if record.kind == PpTokenKind::Identifier
                        && record.spelling.as_slice() != b"defined" =>
                {
                    record.spelling.clone()
                }
                _ => return Err(self.err("malformed `defined`: expected an identifier")),
            };
            self.pos += 1;
            if !self.eat(&[b")"]) {
                return Err(self.err("malformed `defined`: expected `)`"));
            }
            return Ok(defined_in(self.macros, &spelling));
        }
        let spelling = match self.peek() {
            Some(record)
                if record.kind == PpTokenKind::Identifier
                    && record.spelling.as_slice() != b"defined" =>
            {
                record.spelling.clone()
            }
            _ => return Err(self.err("malformed `defined`: expected an identifier")),
        };
        self.pos += 1;
        Ok(defined_in(self.macros, &spelling))
    }
}

/// Exact `i128` integer-literal parse (decimal, `0x` hex, `0` octal, with
/// optional C integer suffixes ignored for value). Returns `None` on any
/// malformed spelling or overflow — never wraps, never guesses.
fn parse_int_literal(spelling: &[u8]) -> Option<i128> {
    let mut end = spelling.len();
    let mut suffix_letters = 0usize;
    while end > 0 {
        let byte = spelling[end - 1];
        if byte == b'u' || byte == b'U' || byte == b'l' || byte == b'L' {
            end -= 1;
            suffix_letters += 1;
        } else {
            break;
        }
    }
    if suffix_letters > 3 || end == 0 {
        return None;
    }
    let digits = &spelling[..end];
    let (body, radix) =
        if digits.len() > 2 && digits[0] == b'0' && (digits[1] == b'x' || digits[1] == b'X') {
            (&digits[2..], 16u32)
        } else if digits.len() > 1 && digits[0] == b'0' {
            (&digits[1..], 8u32)
        } else {
            (digits, 10u32)
        };
    if body.is_empty() {
        return None;
    }
    let mut value: i128 = 0;
    for byte in body {
        let digit = (*byte as char).to_digit(radix)?;
        value = value
            .checked_mul(radix as i128)?
            .checked_add(digit as i128)?;
    }
    Some(value)
}

/// Exact left shift: the amount must satisfy `0 <= rhs < 128` and the
/// result must fit, else `None` (fail-closed, never wraps).
fn shift_left(value: i128, rhs: i128) -> Option<i128> {
    if !(0..128).contains(&rhs) {
        return None;
    }
    value.checked_shl(rhs as u32)
}

/// Exact arithmetic right shift: the amount must satisfy `0 <= rhs < 128`,
/// else `None`.
fn shift_right(value: i128, rhs: i128) -> Option<i128> {
    if !(0..128).contains(&rhs) {
        return None;
    }
    value.checked_shr(rhs as u32)
}
