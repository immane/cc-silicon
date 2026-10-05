// ============================================================================
// chips/preprocess/pp_redefine.rs — T03 PP07 benign-redefinition verifier
// (Wave 2 slice 14, `/23`)
//
// Reads one post-`#` `#define` line's pp-token refs plus exactly one
// committed incumbent `Macro` ref, re-parses the line with a chip-local
// copy of the §3 `#define` grammar (first token must be Identifier
// `define`; function-like iff a `(` punctuator is span-adjacent to the
// name; params are an `Identifier` list with an optional trailing `...`;
// `__VA_ARGS__` anywhere sets `variadic`), and checks benign equivalence
// against the incumbent: same name spelling, same params spelling
// sequence, same variadic flag, same replacement SPELLING sequence
// (whitespace-insensitive by construction since pp-tokens carry no
// whitespace). Benign completes `Ack` (no append, ever); anything else
// fails naming the differing part (`name`, `params`, `variadic`, or
// `replacement`). Reads Tasks `active.*`, Pp `tokens`+`macros`, and Sources
// `spans` (span bodies are required for the §3 adjacency rule); no writes.
// ============================================================================

use crate::bus::{MacroRecord, PpTokenKind, PpTokenRecord, SpanRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{PpTokenId, RecordRef, SpanId, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP07_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

/// Narrow projection for the benign-redefinition check.
#[derive(Clone, Debug)]
pub struct PpRedefineInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// The post-`#` line's token bodies in payload order.
    pub line: Vec<(PpTokenId, PpTokenRecord)>,
    /// Span records referenced by the line tokens, in first-use order.
    pub spans: Vec<(SpanId, SpanRecord)>,
    /// Incumbent macro body.
    pub incumbent: MacroRecord,
    /// Incumbent replacement spellings in order (resolved from the
    /// committed pp-token bodies the incumbent's replacement IDs point
    /// at, since comparison is spelling-based).
    pub incumbent_replacement: Vec<Vec<u8>>,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pp_redefine_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpRedefineInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("redefine of unknown task {}", task.index())))?;
    if record.kind != TaskKind::PREPROCESS_MACRO_REDEFINE {
        return Err(protocol_fault(format!(
            "redefine task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() < 2 {
        return Err(protocol_fault(format!(
            "redefine task {} payload must carry line refs plus one macro ref",
            task.index()
        )));
    }
    let (line_refs, macro_ref) = match record.payload.refs.split_at(record.payload.refs.len() - 1) {
        (line, [single]) => (line, single),
        _ => {
            return Err(protocol_fault(format!(
                "redefine task {} payload must end in exactly one macro ref",
                task.index()
            )));
        }
    };
    if line_refs.is_empty() {
        return Err(protocol_fault(format!(
            "redefine task {} payload must carry at least one line ref",
            task.index()
        )));
    }
    let mut line = Vec::with_capacity(line_refs.len());
    for reference in line_refs {
        let id = match *reference {
            RecordRef::PpToken(id) => id,
            _ => {
                return Err(protocol_fault(format!(
                    "redefine task {} line refs must precede the single macro ref",
                    task.index()
                )));
            }
        };
        let body = bus.arenas.pp_tokens.get(id).map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("redefine reads missing pp-token {}", id.index()),
            )
        })?;
        line.push((id, body.clone()));
    }
    let incumbent_id = match *macro_ref {
        RecordRef::Macro(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "redefine task {} payload must end in exactly one macro ref",
                task.index()
            )));
        }
    };
    let incumbent = bus.arenas.macros.get(incumbent_id).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("redefine reads missing macro {}", incumbent_id.index()),
        )
    })?;
    let mut incumbent_replacement = Vec::with_capacity(incumbent.replacement.len());
    for id in &incumbent.replacement {
        let body = bus.arenas.pp_tokens.get(*id).map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("redefine reads missing pp-token {}", id.index()),
            )
        })?;
        incumbent_replacement.push(body.spelling.clone());
    }
    let mut spans: Vec<(SpanId, SpanRecord)> = Vec::new();
    for (_, token) in &line {
        if !span_is_projected(&spans, token.span) {
            match bus.arenas.spans.get(token.span) {
                Ok(found) => spans.push((token.span, *found)),
                Err(_) => {
                    return Err(DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("redefine reads missing span {}", token.span.index()),
                    ));
                }
            }
        }
    }
    Ok(PpRedefineInput {
        task,
        state: record.state.clone(),
        line,
        spans,
        incumbent: incumbent.clone(),
        incumbent_replacement,
    })
}

/// The T03 benign-redefinition verifier (PP07): benign equivalence
/// completes `Ack` with no append; every other outcome is a `Fail`.
pub struct PpRedefineChip;

impl Worker for PpRedefineChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP07_CHIP,
            chip_name: "PpRedefineChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![TaskKind::PREPROCESS_MACRO_REDEFINE],
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
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c23_macro.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pp_redefine_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

/// One re-parsed `#define` line (chip-local copy of the §3 grammar).
struct ParsedDefine {
    /// Macro name spelling.
    name: Vec<u8>,
    /// Function-like (span-adjacent paren list present).
    function_like: bool,
    /// Parameter spellings in order (`[]` for object-like).
    params: Vec<Vec<u8>>,
    /// Variadic (`...`/`__VA_ARGS__` present).
    variadic: bool,
    /// Replacement spellings in order (possibly empty).
    replacement: Vec<Vec<u8>>,
}

/// True when the record is an `Identifier` with the given spelling.
fn is_identifier_spelling(record: &PpTokenRecord, spelling: &[u8]) -> bool {
    record.kind == PpTokenKind::Identifier && record.spelling.as_slice() == spelling
}

/// True when the record is a `Punctuator` with the given spelling.
fn is_punct_spelling(record: &PpTokenRecord, spelling: &[u8]) -> bool {
    record.kind == PpTokenKind::Punctuator && record.spelling.as_slice() == spelling
}

/// True when the span ID already has a projected span record.
fn span_is_projected(spans: &[(SpanId, SpanRecord)], id: SpanId) -> bool {
    for (known, _) in spans {
        if *known == id {
            return true;
        }
    }
    false
}

/// Look up a projected span record by span ID.
fn find_span(spans: &[(SpanId, SpanRecord)], id: SpanId) -> Option<SpanRecord> {
    for (known, body) in spans {
        if *known == id {
            return Some(*body);
        }
    }
    None
}

/// Malformed-line failure (typed `Task` error, never a protocol fault).
fn malformed(message: impl Into<String>) -> DiagnosticDraft {
    DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Task, 4), message)
}

/// True when the `(` token is span-adjacent to the macro name (same
/// source, no gap): the C whitespace-sensitivity rule for function-like
/// definitions. Missing span bodies here mean the projection is
/// internally inconsistent.
fn paren_is_adjacent(
    name: &PpTokenRecord,
    open: &PpTokenRecord,
    spans: &[(SpanId, SpanRecord)],
) -> Result<bool, DiagnosticDraft> {
    let name_span = match find_span(spans, name.span) {
        Some(found) => found,
        None => {
            return Err(protocol_fault("redefine projection is missing a line span"));
        }
    };
    let open_span = match find_span(spans, open.span) {
        Some(found) => found,
        None => {
            return Err(protocol_fault("redefine projection is missing a line span"));
        }
    };
    Ok(name_span.source == open_span.source && open_span.start == name_span.end)
}

/// Re-parse one post-`#` `#define` line with the §3 grammar. The first
/// token must be Identifier `define` (wrong dispatch is a protocol
/// fault); every structural violation after that is a malformed `Fail`.
fn parse_define_line(
    line: &[(PpTokenId, PpTokenRecord)],
    spans: &[(SpanId, SpanRecord)],
) -> Result<ParsedDefine, DiagnosticDraft> {
    let first = match line.first() {
        Some(found) => found,
        None => {
            return Err(protocol_fault("redefine task line is empty"));
        }
    };
    if !is_identifier_spelling(&first.1, b"define") {
        return Err(protocol_fault(
            "redefine task line must start with `define`",
        ));
    }
    let name_token = match line.get(1) {
        Some(found) => found,
        None => {
            return Err(malformed(
                "redefine reads malformed define line: missing macro name",
            ));
        }
    };
    if name_token.1.kind != PpTokenKind::Identifier {
        return Err(malformed(
            "redefine reads malformed define line: bad macro name",
        ));
    }
    let name = name_token.1.spelling.clone();
    let rest = match line.get(2..) {
        Some(found) => found,
        None => {
            return Err(protocol_fault("redefine task line is truncated"));
        }
    };
    let mut params: Vec<Vec<u8>> = Vec::new();
    let mut variadic = false;
    let mut replacement: Vec<Vec<u8>> = Vec::new();
    let mut function_like = false;
    if let Some(open) = rest.first() {
        if is_punct_spelling(&open.1, b"(") && paren_is_adjacent(&name_token.1, &open.1, spans)? {
            function_like = true;
        }
    }
    if function_like {
        let mut cursor = 1usize;
        match rest.get(cursor) {
            None => {
                return Err(malformed(
                    "redefine reads malformed define line: missing `)`",
                ));
            }
            Some(found) if is_punct_spelling(&found.1, b")") => {
                cursor += 1;
            }
            Some(_) => loop {
                let token = match rest.get(cursor) {
                    Some(found) => found,
                    None => {
                        return Err(malformed(
                            "redefine reads malformed define line: missing `)`",
                        ));
                    }
                };
                if is_punct_spelling(&token.1, b"...") {
                    variadic = true;
                    cursor += 1;
                    match rest.get(cursor) {
                        Some(close) if is_punct_spelling(&close.1, b")") => {
                            cursor += 1;
                        }
                        _ => {
                            return Err(malformed(
                                "redefine reads malformed define line: `...` must be trailing",
                            ));
                        }
                    }
                    break;
                } else if token.1.kind == PpTokenKind::Identifier {
                    if token.1.spelling.as_slice() == b"__VA_ARGS__" {
                        variadic = true;
                    }
                    params.push(token.1.spelling.clone());
                    cursor += 1;
                    match rest.get(cursor) {
                        None => {
                            return Err(malformed(
                                "redefine reads malformed define line: missing `)`",
                            ));
                        }
                        Some(sep) if is_punct_spelling(&sep.1, b",") => {
                            cursor += 1;
                            match rest.get(cursor) {
                                None => {
                                    return Err(malformed(
                                        "redefine reads malformed define line: missing `)`",
                                    ));
                                }
                                Some(next) if is_punct_spelling(&next.1, b")") => {
                                    return Err(malformed(
                                        "redefine reads malformed define line: trailing comma",
                                    ));
                                }
                                Some(_) => {}
                            }
                        }
                        Some(close) if is_punct_spelling(&close.1, b")") => {
                            cursor += 1;
                            break;
                        }
                        Some(_) => {
                            return Err(malformed(
                                "redefine reads malformed define line: expected `,` or `)`",
                            ));
                        }
                    }
                } else {
                    return Err(malformed(
                        "redefine reads malformed define line: bad parameter",
                    ));
                }
            },
        }
        for (_, record) in rest.iter().skip(cursor) {
            if record.kind == PpTokenKind::Identifier
                && record.spelling.as_slice() == b"__VA_ARGS__"
            {
                variadic = true;
            }
            replacement.push(record.spelling.clone());
        }
    } else {
        for (_, record) in rest.iter() {
            if record.kind == PpTokenKind::Identifier
                && record.spelling.as_slice() == b"__VA_ARGS__"
            {
                variadic = true;
            }
            replacement.push(record.spelling.clone());
        }
    }
    Ok(ParsedDefine {
        name,
        function_like,
        params,
        variadic,
        replacement,
    })
}

impl PpRedefineChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &PpRedefineInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("redefine task {} is not running", input.task.index()),
                ),
            )];
        }
        if input.incumbent.undefined {
            return vec![fail(
                input.task,
                protocol_fault(format!(
                    "redefine task {} dispatched against an undefined incumbent",
                    input.task.index()
                )),
            )];
        }
        let parsed = match parse_define_line(&input.line, &input.spans) {
            Ok(parsed) => parsed,
            Err(diagnostic) => return vec![fail(input.task, diagnostic)],
        };
        if parsed.name != input.incumbent.spelling {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "redefine mismatch: name differs",
                ),
            )];
        }
        if parsed.params != input.incumbent.params {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "redefine mismatch: params differ",
                ),
            )];
        }
        if parsed.function_like != input.incumbent.function_like {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "redefine mismatch: function-like differs",
                ),
            )];
        }
        if parsed.variadic != input.incumbent.variadic {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "redefine mismatch: variadic differs",
                ),
            )];
        }
        if parsed.replacement != input.incumbent_replacement {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "redefine mismatch: replacement differs",
                ),
            )];
        }
        vec![Proposal::Complete {
            task: input.task,
            value: ResultValue::Ack,
        }]
    }
}
