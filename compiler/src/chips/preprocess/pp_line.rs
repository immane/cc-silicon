// ============================================================================
// chips/preprocess/pp_line.rs — T03 PP23 line-directive worker
//
// Expands one line directive (`LineTokens -> LogicalLocation`, T03:31,
// CHIP_PLAN.md PP23 row; `pp_directive.rs:574-578` defers a `line`
// directive as `Unsupported` to this PP23 worker).
//
// Physical vs logical: the physical location is where the directive line
// itself sits (the payload tokens' spans in their owning `SourceId`); the
// logical location is what the directive declares for the lines that
// follow it (a 1-based line number plus an optional file name) so that
// `__LINE__`, `__FILE__`, and diagnostics agree, including across an
// include return (the includer re-asserts its own logical location).
// This worker validates one directive line and reports its logical
// location; it never touches the physical bytes.
//
// Accepted forms over the line's pp-tokens in payload order:
//
// * `#line number "file"?` — the second token is Identifier `line`, the
//   third is the pp-number line number, the optional fourth is the file
//   string, and nothing may follow.
// * GNU line marker `# lineno "file" flags?` — the second token is the
//   pp-number line number, the optional third is the file string, and any
//   further tokens must be pp-numbers (flag values are accepted and
//   ignored).
//
// The `#` introducer also accepts the `%:` spelling, matching the PP05
// directive-line recognition in `pp_directive.rs`.
//
// Rules: the number spelling must be all ASCII digits with value
// `1..=2^31-1` (zero, out-of-range, and non-digit spellings such as a
// pp-number `1e5` are typed `Invalid` failures); the file spelling must be
// a `"..."` string literal decoded with only `\\` and `\"` escapes (any
// other escape, a missing quote, or a non-string token in file position is
// a typed failure); a missing file means retain-current-file
// (`file: None`, so the consumer keeps the physical source).
//
// Frozen registration (`/28` integrator-owned): `PP23_TASK_KIND` aliases
// `TaskKind::PREPROCESS_LINE_DIRECTIVE` (PREPROCESS local 32),
// `PP23_CHIP` is `crate::manifest::PP23_CHIP` (`ChipId(35)`), the
// kind-registry row lives in `TaskKindRegistry::pp_line_slice()`, the
// stage-1 row in `STAGE_ASSIGNMENT`, the routed layer is 1, and the
// acceptance test is `compiler/tests/c28_line.rs`. There is deliberately
// no bus write and hence no allowlist row: success completes `Ack` (the
// validated location is returned by `line_location` for the wiring layer)
// and every failure path is a typed `Fail`. Malformed input is never
// silently accepted.
// ============================================================================

use crate::bus::{PpTokenKind, PpTokenRecord, SpanRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{PpTokenId, RecordRef, SourceId, SpanId, TaskId};
use crate::manifest::{
    BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP23_CHIP,
};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

/// Frozen PP23 task kind (aliases `TaskKind::PREPROCESS_LINE_DIRECTIVE`,
/// frozen by the `/28` integrator; the local code is `PREPROCESS` 32,
/// first code after `/27`).
pub const PP23_TASK_KIND: TaskKind = TaskKind::PREPROCESS_LINE_DIRECTIVE;

/// Maximum logical line number: `2^31 - 1`.
pub const PP_LINE_MAX: u32 = 2_147_483_647;

/// Logical location proposed by PP23 (candidate shape).
///
/// The field shapes copy `SpanRecord { source, start, end, expansion }`
/// (see `pp_scan.rs`, which uses the bus span types) without importing a
/// store type: `source` is the physical source owning the directive line,
/// `line` is the declared 1-based logical line, and `file` is the decoded
/// logical file (`None` means retain-current-file: the consumer keeps the
/// physical source). No name-to-`SourceId` resolution exists at this layer,
/// so the decoded file travels as bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogicalLocation {
    /// Physical source owning the directive line.
    pub source: SourceId,
    /// Declared logical line number (`1..=2^31-1`).
    pub line: u32,
    /// Decoded logical file bytes (`None` retains the current file).
    pub file: Option<Vec<u8>>,
}

/// Narrow projection for the line-directive computation.
#[derive(Clone, Debug)]
pub struct PpLineInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// The directive line's token bodies in payload order.
    pub tokens: Vec<(PpTokenId, PpTokenRecord)>,
    /// Span records referenced by the payload tokens, in first-use order.
    pub spans: Vec<(SpanId, SpanRecord)>,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pp_line_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpLineInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("line of unknown task {}", task.index())))?;
    if record.kind != PP23_TASK_KIND {
        return Err(protocol_fault(format!(
            "line task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.is_empty() {
        return Err(protocol_fault(format!(
            "line task {} payload must carry at least one pp-token",
            task.index()
        )));
    }
    let mut tokens = Vec::with_capacity(record.payload.refs.len());
    for reference in &record.payload.refs {
        let id = match *reference {
            RecordRef::PpToken(id) => id,
            _ => {
                return Err(protocol_fault(format!(
                    "line task {} payload must be pp-token refs",
                    task.index()
                )));
            }
        };
        match bus.arenas.pp_tokens.get(id) {
            Ok(found) => tokens.push((id, found.clone())),
            Err(_) => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("line reads missing pp-token {}", id.index()),
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
                        format!("line reads missing span {}", token.span.index()),
                    ));
                }
            }
        }
    }
    Ok(PpLineInput {
        task,
        state: record.state.clone(),
        tokens,
        spans,
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

/// The T03 line-directive worker (PP23 candidate scope).
pub struct PpLineChip;

impl Worker for PpLineChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP23_CHIP,
            chip_name: "PpLineChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![PP23_TASK_KIND],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Sources, "spans"),
            ],
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c28_line.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pp_line_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl PpLineChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// Validates the directive line via [`line_location`]: success completes
    /// `Ack` with no bus writes (the wiring layer consumes the validated
    /// location), and every failure path is a typed `Fail`.
    pub fn compute(&self, input: &PpLineInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("line task {} is not running", input.task.index()),
                ),
            )];
        }
        match line_location(input) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }
}

/// Validate one directive line and report its logical location.
///
/// Pure over the narrow projection: parses the number/file forms, resolves
/// the physical source from the first token's span, and returns the
/// proposal-ready [`LogicalLocation`]. The wiring layer persists the
/// location; this function performs no bus write.
pub fn line_location(input: &PpLineInput) -> Result<LogicalLocation, DiagnosticDraft> {
    let (line, file) = parse_line_directive(&input.tokens)?;
    let first = match input.tokens.first() {
        Some(found) => found,
        None => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "line directive carries no tokens",
            ));
        }
    };
    let mut source: Option<SourceId> = None;
    for (known, record) in &input.spans {
        if *known == first.1.span {
            source = Some(record.source);
        }
    }
    let source = match source {
        Some(found) => found,
        None => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("line reads unprojected span {}", first.1.span.index()),
            ));
        }
    };
    Ok(LogicalLocation { source, line, file })
}

/// Parse one directive line's tokens into `(line, file)`.
///
/// Dispatches on the second token: Identifier `line` selects the
/// `#line number "file"?` form (nothing may follow the optional file);
/// a pp-number selects the GNU `# lineno "file" flags?` form (only
/// pp-number flags may follow the optional file, values ignored).
fn parse_line_directive(
    tokens: &[(PpTokenId, PpTokenRecord)],
) -> Result<(u32, Option<Vec<u8>>), DiagnosticDraft> {
    let first = match tokens.first() {
        Some(found) => found,
        None => {
            return Err(invalid("line directive carries no tokens"));
        }
    };
    if !is_hash_token(&first.1) {
        return Err(invalid("line directive must start with `#`"));
    }
    let second = match tokens.get(1) {
        Some(found) => found,
        None => {
            return Err(invalid(
                "line directive needs `line` or a line number after `#`",
            ));
        }
    };
    if is_line_name(&second.1) {
        return parse_hash_line_form(tokens);
    }
    if second.1.kind == PpTokenKind::PpNumber {
        return parse_gnu_marker_form(tokens);
    }
    Err(invalid(
        "line directive needs `line` or a line number after `#`",
    ))
}

/// Parse the `#line number "file"?` tail (tokens past `# line`).
fn parse_hash_line_form(
    tokens: &[(PpTokenId, PpTokenRecord)],
) -> Result<(u32, Option<Vec<u8>>), DiagnosticDraft> {
    let number = match tokens.get(2) {
        Some(found) => found,
        None => {
            return Err(invalid("`#line` needs a line number"));
        }
    };
    if number.1.kind != PpTokenKind::PpNumber {
        return Err(invalid("`#line` needs a pp-number line number"));
    }
    let line = parse_line_number(&number.1.spelling)?;
    match tokens.get(3) {
        None => Ok((line, None)),
        Some(file) => {
            if file.1.kind != PpTokenKind::StringLiteral {
                return Err(invalid("`#line` file name must be a string literal"));
            }
            if tokens.get(4).is_some() {
                return Err(invalid("unexpected tokens after `#line` file name"));
            }
            let decoded = decode_file_string(&file.1.spelling)?;
            Ok((line, Some(decoded)))
        }
    }
}

/// Parse the GNU `# lineno "file" flags?` tail (tokens past `#`).
fn parse_gnu_marker_form(
    tokens: &[(PpTokenId, PpTokenRecord)],
) -> Result<(u32, Option<Vec<u8>>), DiagnosticDraft> {
    let line = parse_line_number(&tokens[1].1.spelling)?;
    let mut cursor = 2usize;
    let file: Option<Vec<u8>>;
    match tokens.get(cursor) {
        None => return Ok((line, None)),
        Some(candidate) => {
            if candidate.1.kind != PpTokenKind::StringLiteral {
                return Err(invalid("line marker file name must be a string literal"));
            }
            file = Some(decode_file_string(&candidate.1.spelling)?);
            cursor += 1;
        }
    }
    while cursor < tokens.len() {
        let flag = &tokens[cursor].1;
        if flag.kind != PpTokenKind::PpNumber {
            return Err(invalid("unexpected token after line marker file name"));
        }
        cursor += 1;
    }
    Ok((line, file))
}

/// Parse a pp-number spelling as a decimal line number in `1..=2^31-1.
///
/// The spelling must be all ASCII digits; the checked accumulation rejects
/// overflow before it can happen, so no host-width behavior leaks in.
fn parse_line_number(spelling: &[u8]) -> Result<u32, DiagnosticDraft> {
    if spelling.is_empty() {
        return Err(invalid("line directive needs a decimal line number"));
    }
    let mut value: u64 = 0;
    for index in 0..spelling.len() {
        let byte = spelling[index];
        if !byte.is_ascii_digit() {
            return Err(invalid(format!(
                "line number `{}` is not a decimal integer",
                String::from_utf8_lossy(spelling)
            )));
        }
        value = value * 10 + (byte - b'0') as u64;
        if value > PP_LINE_MAX as u64 {
            return Err(invalid("line number out of range (must be 1..=2147483647)"));
        }
    }
    if value == 0 {
        return Err(invalid("line number out of range (must be 1..=2147483647)"));
    }
    Ok(value as u32)
}

/// Decode a `"..."` string-literal spelling with only `\\` and `\"`
/// escapes. Any other escape, a trailing backslash, or missing quotes is a
/// typed failure.
fn decode_file_string(spelling: &[u8]) -> Result<Vec<u8>, DiagnosticDraft> {
    if spelling.len() < 2 || spelling[0] != b'"' || spelling[spelling.len() - 1] != b'"' {
        return Err(invalid(
            "line directive file name must be a `\"...\"` string",
        ));
    }
    let mut decoded = Vec::new();
    let mut index = 1usize;
    let end = spelling.len() - 1;
    while index < end {
        let byte = spelling[index];
        if byte != b'\\' {
            decoded.push(byte);
            index += 1;
            continue;
        }
        if index + 1 >= end {
            return Err(invalid("trailing backslash in line directive file name"));
        }
        let escaped = spelling[index + 1];
        if escaped == b'\\' {
            decoded.push(b'\\');
        } else if escaped == b'"' {
            decoded.push(b'"');
        } else {
            return Err(invalid(
                "unsupported escape in line directive file name (only `\\\\` and `\\\"`)",
            ));
        }
        index += 2;
    }
    Ok(decoded)
}

/// True when the record is a directive introducer (`#` or `%:`).
fn is_hash_token(record: &PpTokenRecord) -> bool {
    record.kind == PpTokenKind::Punctuator
        && (record.spelling.as_slice() == b"#" || record.spelling.as_slice() == b"%:")
}

/// True when the record is the Identifier spelling `line`.
fn is_line_name(record: &PpTokenRecord) -> bool {
    record.kind == PpTokenKind::Identifier && record.spelling.as_slice() == b"line"
}

/// Typed `Invalid` failure for malformed line directives.
fn invalid(message: impl Into<String>) -> DiagnosticDraft {
    DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Task, 4), message)
}
