// ============================================================================
// chips/preprocess/pp_define.rs — T03 PP06 macro-definition worker
// (Wave 2 slice 14, `/23`)
//
// Reads one post-`#` `#define` line's refs (fanned out by the caller). The
// first token must be the identifier `define` (anything else is a protocol
// fault); the next token must be the macro-name identifier (anything else
// is a malformed typed failure). A `(` punctuator token span-adjacent to
// the name (same source, `name.span.end == paren.span.start`) opens a
// function-like parameter list (`Identifier` list, optional trailing `...`;
// `__VA_ARGS__` anywhere in params or replacement sets `variadic`);
// otherwise the rest of the line is an object-like replacement list
// (possibly empty). Lookup takes the greatest committed `MacroId` with
// equal spelling: absent or latest-tombstone appends one fresh record
// (`undefined: false`) and completes `Record` of the predicted ID
// (tombstones supersede silently by ID order, no PP07 involved);
// present-and-defined enqueues one PP07 redefine child (payload = line
// refs + `Macro(old)`) and awaits it. Resume runs only under the frozen
// join (a `Failed` child fails this task in the router reusing that
// child's diagnostic, so no aggregate is ever minted here): every child
// must be terminal (else protocol fault) and the benign redefine completes
// `Ack` with no append. Reads Tasks `active.*`, Pp `tokens`+`macros`, and
// `Sources` `spans` (span records feed the frozen §3 adjacency rule only);
// writes Pp `macros`.
// ============================================================================

use crate::bus::{MacroRecord, PpTokenKind, PpTokenRecord, SpanRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{MacroId, PpTokenId, RecordRef, SpanId, TaskId};
use crate::manifest::{
    BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP06_CHIP, PP07_CHIP,
};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, ChildRef, DraftRef, Payload, Proposal, ResultValue, StoreId, TaskDraft, TaskGroup,
    TaskKind, TaskState,
};

/// Narrow projection for the macro-definition computation.
#[derive(Clone, Debug)]
pub struct PpDefineInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Line token bodies in payload order.
    pub tokens: Vec<(PpTokenId, PpTokenRecord)>,
    /// Span records referenced by the line tokens, in first-use order
    /// (name/`(` adjacency check only).
    pub spans: Vec<(SpanId, SpanRecord)>,
    /// Committed macro definitions in ascending-ID order (latest wins).
    pub macros: Vec<(MacroId, MacroRecord)>,
    /// `macros` arena count at dispatch (fresh-append prediction base).
    pub macros_allocated: u32,
    /// Already-enqueued children (tasks with `parent == Some(task)`, in
    /// `TaskId` order) with their states.
    pub children: Vec<(TaskId, TaskState)>,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pp_define_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpDefineInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("define of unknown task {}", task.index())))?;
    if record.kind != TaskKind::PREPROCESS_MACRO_DEFINE {
        return Err(protocol_fault(format!(
            "define task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.is_empty() {
        return Err(protocol_fault(format!(
            "define task {} payload must carry pp-token refs",
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
                        format!("define reads missing pp-token {}", id.index()),
                    ));
                }
            },
            _ => {
                return Err(protocol_fault(format!(
                    "define task {} payload must be pp-token refs",
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
                    format!("define reads missing span {}", token.span.index()),
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
    let mut children: Vec<(TaskId, TaskState)> = Vec::new();
    for (id, child) in bus.arenas.tasks.iter() {
        if child.parent == Some(task) {
            children.push((id, child.state.clone()));
        }
    }
    children.sort_by_key(|(id, _)| id.index());
    Ok(PpDefineInput {
        task,
        state: record.state.clone(),
        tokens,
        spans,
        macros,
        macros_allocated: bus.arenas.macros.allocated(),
        children,
    })
}

/// The T03 macro-definition worker (PP06 slice scope).
pub struct PpDefineChip;

impl Worker for PpDefineChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP06_CHIP,
            chip_name: "PpDefineChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![TaskKind::PREPROCESS_MACRO_DEFINE],
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
            writes: vec![FieldPath::new(StoreId::Pp, "macros")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c23_macro.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pp_define_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

/// One parsed `#define` line (frozen §3 grammar).
struct ParsedDefine {
    /// Macro name spelling (raw bytes).
    name: Vec<u8>,
    /// Function-like (span-adjacent paren list present).
    function_like: bool,
    /// Parameter spellings in order (`[]` for object-like).
    params: Vec<Vec<u8>>,
    /// Variadic (`...`/`__VA_ARGS__` present).
    variadic: bool,
    /// Replacement list (line pp-token IDs in order).
    replacement: Vec<PpTokenId>,
}

impl PpDefineChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// First dispatch (no children yet): parse the line, then either append
    /// a fresh record (absent or latest-tombstone) or enqueue one PP07
    /// child and await it. Resume dispatch runs only when the frozen join
    /// readies this task: a `Failed` child makes the join itself fail this
    /// task reusing that child's diagnostic, so resume only asserts terminal
    /// children and completes `Ack` with no append (benign redefine is a
    /// no-op); no aggregate message is ever minted here.
    pub fn compute(&self, input: &PpDefineInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("define task {} is not running", input.task.index()),
                ),
            )];
        }
        if !input.children.is_empty() {
            return resume_pp_define(input);
        }
        dispatch_pp_define(input)
    }
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

/// Look up a projected span record by ID.
fn find_span(input: &PpDefineInput, id: SpanId) -> Option<SpanRecord> {
    for (known, record) in &input.spans {
        if *known == id {
            return Some(*record);
        }
    }
    None
}

/// Malformed-line typed failure (`Task`, 4).
fn malformed(task: TaskId, detail: impl Into<String>) -> DiagnosticDraft {
    let _ = task;
    DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Task, 4), detail)
}

/// True when the token is the identifier `define`.
fn is_define_name(record: &PpTokenRecord) -> bool {
    record.kind == PpTokenKind::Identifier && record.spelling == b"define"
}

/// True when the token is the identifier `__VA_ARGS__`.
fn is_va_args(record: &PpTokenRecord) -> bool {
    record.kind == PpTokenKind::Identifier && record.spelling.as_slice() == b"__VA_ARGS__"
}

/// True when the token is the `)` punctuator.
fn is_close_paren(record: &PpTokenRecord) -> bool {
    record.kind == PpTokenKind::Punctuator && record.spelling.as_slice() == b")"
}

/// True when the token is the `,` punctuator.
fn is_comma(record: &PpTokenRecord) -> bool {
    record.kind == PpTokenKind::Punctuator && record.spelling.as_slice() == b","
}

/// True when the token is the `...` punctuator.
fn is_ellipsis(record: &PpTokenRecord) -> bool {
    record.kind == PpTokenKind::Punctuator && record.spelling.as_slice() == b"..."
}

/// Latest committed record with spelling equal to `spelling` (`macros` is
/// in ascending-ID order, so the last match wins). `None` when absent.
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
    latest
}

/// First-dispatch path: parse the line, then append fresh or fan out to PP07.
fn dispatch_pp_define(input: &PpDefineInput) -> Vec<Proposal> {
    let parsed = match parse_define_line(input) {
        Ok(parsed) => parsed,
        Err(diagnostic) => return vec![fail(input.task, diagnostic)],
    };
    match latest_with_spelling(&input.macros, &parsed.name) {
        Some((old, latest)) if !latest.undefined => collision_enqueue(input, old),
        _ => fresh_append(input, &parsed),
    }
}

/// Resume path: under the frozen join this dispatch runs only after every
/// child is terminal AND readied, i.e. in practice only when the child
/// `Completed` (a `Failed` child makes the join itself fail this task
/// reusing that child's diagnostic). Any non-terminal child is a protocol
/// fault; otherwise the benign redefine completes `Ack` with no append.
fn resume_pp_define(input: &PpDefineInput) -> Vec<Proposal> {
    for (_, state) in &input.children {
        if !state.is_terminal() {
            return vec![fail(
                input.task,
                protocol_fault("define resume reads a non-terminal child"),
            )];
        }
    }
    vec![Proposal::Complete {
        task: input.task,
        value: ResultValue::Ack,
    }]
}

/// Fresh path: append one defined record and complete with its predicted ID
/// (quota-1 single append, mirroring the `PpScanChip` prediction pattern).
fn fresh_append(input: &PpDefineInput, parsed: &ParsedDefine) -> Vec<Proposal> {
    let predicted = MacroId::from_index(input.macros_allocated);
    vec![
        Proposal::AppendRecords {
            task: input.task,
            batch: AppendBatch {
                records: vec![RecordDraft {
                    family: crate::ids::RecordFamily::Macro,
                    index: DraftRef(0),
                }],
                bodies: vec![G1DraftBody::Macro(MacroRecord {
                    spelling: parsed.name.clone(),
                    params: parsed.params.clone(),
                    function_like: parsed.function_like,
                    variadic: parsed.variadic,
                    replacement: parsed.replacement.clone(),
                    undefined: false,
                })],
            },
        },
        Proposal::Complete {
            task: input.task,
            value: ResultValue::Record(RecordRef::Macro(predicted)),
        },
    ]
}

/// Collision path: enqueue one PP07 child over the line refs plus the
/// incumbent record, and await it (single own-batch child).
fn collision_enqueue(input: &PpDefineInput, old: MacroId) -> Vec<Proposal> {
    let mut refs = Vec::with_capacity(input.tokens.len() + 1);
    for (id, _) in &input.tokens {
        refs.push(RecordRef::PpToken(*id));
    }
    refs.push(RecordRef::Macro(old));
    vec![
        Proposal::Enqueue(TaskDraft {
            kind: TaskKind::PREPROCESS_MACRO_REDEFINE,
            owner: PP07_CHIP,
            parent: Some(input.task),
            payload: Payload::from_refs(refs),
            continuation: None,
        }),
        Proposal::AwaitChildren {
            task: input.task,
            children: vec![ChildRef::OwnBatch(0)],
        },
    ]
}

/// Parse one post-`#` `#define` line per the frozen §3 grammar: first token
/// Identifier `define` (else protocol fault), then an Identifier name
/// (else malformed `Fail`), then either a span-adjacent `(` opening a
/// function-like parameter list or an object-like replacement rest.
fn parse_define_line(input: &PpDefineInput) -> Result<ParsedDefine, DiagnosticDraft> {
    let first = match input.tokens.first() {
        Some(found) => found,
        None => {
            return Err(protocol_fault(format!(
                "define task {} payload must carry pp-token refs",
                input.task.index()
            )));
        }
    };
    if !is_define_name(&first.1) {
        return Err(protocol_fault(format!(
            "define task {} first token must be identifier `define`",
            input.task.index()
        )));
    }
    let name_token = match input.tokens.get(1) {
        Some(found) => found,
        None => {
            return Err(malformed(
                input.task,
                "malformed `#define`: expected a macro name after `define`",
            ));
        }
    };
    if name_token.1.kind != PpTokenKind::Identifier {
        return Err(malformed(
            input.task,
            "malformed `#define`: expected a macro name after `define`",
        ));
    }
    let name = name_token.1.spelling.clone();
    if paren_adjacent(input)? {
        parse_function_like(input, name)
    } else {
        parse_object_like(input, name)
    }
}

/// True when a `(` punctuator token immediately follows the name token
/// (index 1): span-adjacent means same source with
/// `name.span.end == paren.span.start` (whitespace-sensitivity per C).
/// A missing or non-paren third token is object-like (`false`), never an
/// error; an unprojected span is a typed failure.
fn paren_adjacent(input: &PpDefineInput) -> Result<bool, DiagnosticDraft> {
    let paren = match input.tokens.get(2) {
        Some(found) => found,
        None => return Ok(false),
    };
    if paren.1.kind != PpTokenKind::Punctuator || paren.1.spelling.as_slice() != b"(" {
        return Ok(false);
    }
    let name_record = match input.tokens.get(1) {
        Some(found) => found,
        None => return Ok(false),
    };
    let name_span = match find_span(input, name_record.1.span) {
        Some(found) => found,
        None => {
            return Err(malformed(
                input.task,
                format!(
                    "define reads unprojected span {}",
                    name_record.1.span.index()
                ),
            ));
        }
    };
    let paren_span = match find_span(input, paren.1.span) {
        Some(found) => found,
        None => {
            return Err(malformed(
                input.task,
                format!("define reads unprojected span {}", paren.1.span.index()),
            ));
        }
    };
    Ok(name_span.source == paren_span.source && name_span.end == paren_span.start)
}

/// Object-like parse: every token after the name is replacement (possibly
/// empty); `__VA_ARGS__` anywhere in it sets `variadic`.
fn parse_object_like(
    input: &PpDefineInput,
    name: Vec<u8>,
) -> Result<ParsedDefine, DiagnosticDraft> {
    let mut replacement = Vec::new();
    let mut variadic = false;
    for (id, record) in input.tokens.iter().skip(2) {
        if is_va_args(record) {
            variadic = true;
        }
        replacement.push(*id);
    }
    Ok(ParsedDefine {
        name,
        function_like: false,
        params: Vec::new(),
        variadic,
        replacement,
    })
}

/// Function-like parse over the tokens after the `(` (index 3): an
/// `Identifier` list with `,` separators, an optional trailing `...`, and
/// a closing `)`; `__VA_ARGS__` in params or replacement sets `variadic`.
/// A missing `)`, a bad parameter, or a non-trailing `...` is a malformed
/// `Fail`. The replacement is every token after the closing `)`.
fn parse_function_like(
    input: &PpDefineInput,
    name: Vec<u8>,
) -> Result<ParsedDefine, DiagnosticDraft> {
    let mut params: Vec<Vec<u8>> = Vec::new();
    let mut variadic = false;
    let mut cursor = 3usize;
    let head = match input.tokens.get(cursor) {
        Some(found) => found,
        None => {
            return Err(malformed(
                input.task,
                "malformed `#define`: expected `)` to close the parameter list",
            ));
        }
    };
    if is_close_paren(&head.1) {
        cursor += 1;
    } else {
        loop {
            let token = match input.tokens.get(cursor) {
                Some(found) => found,
                None => {
                    return Err(malformed(
                        input.task,
                        "malformed `#define`: expected `)` to close the parameter list",
                    ));
                }
            };
            if is_ellipsis(&token.1) {
                variadic = true;
                cursor += 1;
                let close = match input.tokens.get(cursor) {
                    Some(found) => found,
                    None => {
                        return Err(malformed(
                            input.task,
                            "malformed `#define`: expected `)` to close the parameter list",
                        ));
                    }
                };
                if !is_close_paren(&close.1) {
                    return Err(malformed(
                        input.task,
                        "malformed `#define`: `...` must be last in the parameter list",
                    ));
                }
                cursor += 1;
                break;
            }
            if token.1.kind != PpTokenKind::Identifier {
                return Err(malformed(
                    input.task,
                    "malformed `#define`: expected a parameter name",
                ));
            }
            if is_va_args(&token.1) {
                variadic = true;
            }
            params.push(token.1.spelling.clone());
            cursor += 1;
            let separator = match input.tokens.get(cursor) {
                Some(found) => found,
                None => {
                    return Err(malformed(
                        input.task,
                        "malformed `#define`: expected `)` to close the parameter list",
                    ));
                }
            };
            if is_comma(&separator.1) {
                cursor += 1;
                continue;
            }
            if is_close_paren(&separator.1) {
                cursor += 1;
                break;
            }
            return Err(malformed(
                input.task,
                "malformed `#define`: expected `,` or `)` in the parameter list",
            ));
        }
    }
    let mut replacement = Vec::new();
    for (id, record) in input.tokens.iter().skip(cursor) {
        if is_va_args(record) {
            variadic = true;
        }
        replacement.push(*id);
    }
    Ok(ParsedDefine {
        name,
        function_like: true,
        params,
        variadic,
        replacement,
    })
}
