// ============================================================================
// chips/preprocess/pp_pragma.rs — T03 PP25 pragma-dispatch worker
//
// Expands one pragma construct (`PragmaTokens -> PragmaRecord`, T03:33,
// CHIP_PLAN.md PP25 row; `pp_directive.rs` defers a `pragma` directive as
// `Unsupported` to this PP25 worker; `#pragma once` deferred out of the
// include slice).
//
// Narrow projection (task + state + pp-token refs), pure `compute`, ZST,
// no closures, reads-only, typed `Fail` only. One tick completes `Ack` or
// fails; there is no fan-out, so the frozen-join obligation is vacuous.
// Deterministic: the outcome is a pure function of the payload token order.
// No I/O: the chip never reads files, samples the environment, or calls
// other chips.
//
// Dispatch contract: the payload carries exactly one pragma construct in
// source order — either post-`#` refs whose first token is Identifier
// `pragma` (the remainder are the directive params, possibly empty), or
// the four operator tokens Identifier `_Pragma`, Punctuator `(`, StringLiteral,
// Punctuator `)`. Any other first token is a wrong-dispatch protocol
// fault; an empty payload is a protocol fault.
//
// Behavior:
// * `_Pragma("...")` string decoding per `decode_pragma_string` (quotes
//   stripped, simple one-character escapes mapped, optional `u8`/`u`/`U`/
//   `L` prefix tolerated and preserved as raw bytes without transcoding).
// * `#pragma` params and decoded operator text classify to `PragmaClass`:
//   `once` records the once-flag for the header guard (downstream
//   include-resolve PP17/PP18 may consult it once `PragmaRecord`
//   persistence lands — see below), `pack` with a `push`/`pop`
//   operator records the alignment-stack op, and every other well-formed
//   (supported-set or unknown) pragma records as opaque.
// * Unknown-pragma policy (deliberate choice, not an oversight): per
//   ISO C11 6.10.6p1 any pragma not recognized by the implementation is
//   ignored, so a well-formed unknown pragma classifies `Opaque` and the
//   task completes `Ack` — benign ignore, NOT `Fail`. Only a malformed
//   `_Pragma` string (missing quotes, raw newline/quote, dangling
//   backslash, or a non-simple escape this slice cannot decode) fails as
//   Invalid (`DiagGroup::Task`, code 4), because the operand cannot be
//   given any meaning, ignored or otherwise.
//
// Deferred with loud boundaries (never silent guessing):
// * No `PragmaRecord` family/store exists yet, so this chip is
//   reads-only: the classification (the record content) is computed
//   deterministically and exposed through `classify_directive_params` /
//   `classify_operator_text` for tests, but nothing is persisted and the
//   completing `Ack` carries no data. Downstream once-consultation waits
//   on the record freeze. Nothing is fabricated.
// * String decoding covers simple escapes only; hexadecimal, octal,
//   and universal-character-name escapes fail Invalid rather than
//   decode silently wrong.
// * `pack` arguments beyond the `push`/`pop` operator (e.g. an
//   alignment value) are not interpreted; the stack op is recorded and
//   the remainder is benignly ignored with the record store.
//
// Frozen registration (`/29` integrator-owned): `PP25_TASK_KIND` aliases
// `TaskKind::PREPROCESS_PRAGMA_DIRECTIVE` (PREPROCESS local 33),
// `PP25_CHIP` is `crate::manifest::PP25_CHIP` (`ChipId(36)`), the
// kind-registry row lives in `TaskKindRegistry::pp_pragma_slice()`, the
// stage-1 row in `STAGE_ASSIGNMENT`, the routed layer is 1, and the
// acceptance test is `compiler/tests/c29_pragma.rs`. There is deliberately
// no bus write and hence no allowlist row: success completes `Ack` (the
// classification is returned by `classify_*` for the wiring layer)
// and every failure path is a typed `Fail`. Malformed input is never
// silently accepted.
// ============================================================================

use crate::bus::{PpTokenKind, PpTokenRecord};
use crate::chips::{protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{PpTokenId, RecordRef, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP25_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

/// Frozen PP25 task kind (aliases `TaskKind::PREPROCESS_PRAGMA_DIRECTIVE`,
/// frozen by the `/29` integrator; the local code is `PREPROCESS` 33,
/// first code after `/28`).
pub const PP25_TASK_KIND: TaskKind = TaskKind::PREPROCESS_PRAGMA_DIRECTIVE;

/// Narrow projection for the pragma-dispatch computation.
#[derive(Clone, Debug)]
pub struct PpPragmaInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// The pragma construct's token bodies in payload order.
    pub tokens: Vec<(PpTokenId, PpTokenRecord)>,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pp_pragma_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpPragmaInput, DiagnosticDraft> {
    let record = match bus.arenas.tasks.get(task) {
        Ok(record) => record,
        Err(_) => {
            return Err(protocol_fault(format!(
                "pragma of unknown task {}",
                task.index()
            )));
        }
    };
    if record.kind != PP25_TASK_KIND {
        return Err(protocol_fault(format!(
            "pragma task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.is_empty() {
        return Err(protocol_fault(format!(
            "pragma task {} payload must carry at least one pp-token",
            task.index()
        )));
    }
    let mut tokens = Vec::with_capacity(record.payload.refs.len());
    for owned in &record.payload.refs {
        let id = match *owned {
            RecordRef::PpToken(id) => id,
            _ => {
                return Err(protocol_fault(format!(
                    "pragma task {} payload must be pp-token refs",
                    task.index()
                )));
            }
        };
        let body = match bus.arenas.pp_tokens.get(id) {
            Ok(body) => body,
            Err(_) => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("pragma reads missing pp-token {}", id.index()),
                ));
            }
        };
        tokens.push((id, body.clone()));
    }
    Ok(PpPragmaInput {
        task,
        state: record.state.clone(),
        tokens,
    })
}

/// The T03 pragma-dispatch worker (PP25 candidate slice).
pub struct PpPragmaChip;

impl Worker for PpPragmaChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP25_CHIP,
            chip_name: "PpPragmaChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![PP25_TASK_KIND],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Pp, "tokens"),
            ],
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c29_pragma.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pp_pragma_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![crate::chips::fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

/// The deterministic classification of one pragma construct: the record
/// content that a future `PragmaRecord` would persist. `Once`
/// carries the header-guard once-flag; `PackPush`/`PackPop` carry the
/// alignment-stack op; `Opaque` covers every other well-formed pragma,
/// including the supported set with no special handling in this slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PragmaClass {
    /// `#pragma once` / `_Pragma("once")`: header-guard once-flag.
    Once,
    /// `#pragma pack(push, ...)` / `_Pragma("pack(push, ...)")`.
    PackPush,
    /// `#pragma pack(pop, ...)` / `_Pragma("pack(pop, ...)")`.
    PackPop,
    /// Any other well-formed pragma (supported-set or unknown): recorded
    /// opaquely and benignly ignored per C11 6.10.6p1.
    Opaque,
}

/// Structured `_Pragma` string-decode failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PragmaStringError {
    /// Missing quotes, raw quote/newline inside, or a dangling backslash.
    Malformed,
    /// A hexadecimal, octal, or universal escape this slice cannot decode.
    UnsupportedEscape,
}

impl PragmaStringError {
    /// Map to a structured Invalid diagnostic (`Task` group, code 4).
    pub fn to_diagnostic(self) -> DiagnosticDraft {
        match self {
            Self::Malformed => DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "malformed _Pragma string: expected a quoted string literal",
            ),
            Self::UnsupportedEscape => DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "unsupported escape in _Pragma string: only simple one-character escapes decode in this slice",
            ),
        }
    }
}

/// True when the record is an identifier with the given spelling.
fn is_identifier_spelling(record: &PpTokenRecord, name: &[u8]) -> bool {
    record.kind == PpTokenKind::Identifier && record.spelling.as_slice() == name
}

/// True when the record is a punctuator with the given spelling.
fn is_punct_spelling(record: &PpTokenRecord, text: &[u8]) -> bool {
    record.kind == PpTokenKind::Punctuator && record.spelling.as_slice() == text
}

/// True for the ASCII whitespace bytes that may pad pragma text.
fn is_pragma_space(byte: u8) -> bool {
    byte == b' ' || byte == b'\t' || byte == b'\n' || byte == 0x0B || byte == 0x0C || byte == b'\r'
}

/// True for the identifier-constituent bytes used for word-boundary checks.
fn is_word_byte(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

/// Map one simple escape character to its decoded byte; anything else
/// (hexadecimal, octal, universal) is this slice's explicit boundary.
fn simple_escape_byte(escaped: u8) -> Option<u8> {
    match escaped {
        b'\'' => Some(b'\''),
        b'"' => Some(b'"'),
        b'?' => Some(b'?'),
        b'\\' => Some(b'\\'),
        b'a' => Some(0x07),
        b'b' => Some(0x08),
        b'f' => Some(0x0C),
        b'n' => Some(0x0A),
        b'r' => Some(0x0D),
        b't' => Some(0x09),
        b'v' => Some(0x0B),
        _ => None,
    }
}

/// Decode one `_Pragma` string-literal spelling (quotes included) to its
/// raw text: strip the quotes (plus an optional `u8`/`u`/`U`/`L` prefix,
/// preserved as raw bytes without transcoding) and map simple escapes.
/// A raw quote or newline inside, a dangling backslash, or a non-simple
/// escape fails instead of decoding silently wrong.
pub fn decode_pragma_string(spelling: &[u8]) -> Result<Vec<u8>, PragmaStringError> {
    let mut cursor: usize;
    if spelling.len() >= 3 && spelling[0] == b'u' && spelling[1] == b'8' && spelling[2] == b'"' {
        cursor = 3;
    } else if spelling.len() >= 2
        && (spelling[0] == b'u' || spelling[0] == b'U' || spelling[0] == b'L')
        && spelling[1] == b'"'
    {
        cursor = 2;
    } else if !spelling.is_empty() && spelling[0] == b'"' {
        cursor = 1;
    } else {
        return Err(PragmaStringError::Malformed);
    }
    if spelling.is_empty() || spelling[spelling.len() - 1] != b'"' {
        return Err(PragmaStringError::Malformed);
    }
    let mut decoded = Vec::new();
    while cursor < spelling.len() - 1 {
        let byte = spelling[cursor];
        if byte == b'\\' {
            if cursor + 1 >= spelling.len() - 1 {
                return Err(PragmaStringError::Malformed);
            }
            let escaped = spelling[cursor + 1];
            match simple_escape_byte(escaped) {
                Some(value) => decoded.push(value),
                None => return Err(PragmaStringError::UnsupportedEscape),
            }
            cursor += 2;
            continue;
        }
        if byte == b'"' || byte == b'\n' || byte == b'\r' {
            return Err(PragmaStringError::Malformed);
        }
        decoded.push(byte);
        cursor += 1;
    }
    Ok(decoded)
}

/// True when `haystack` starts with `word` on an identifier boundary.
fn starts_with_word(haystack: &[u8], word: &[u8]) -> bool {
    if haystack.len() < word.len() {
        return false;
    }
    if &haystack[0..word.len()] != word {
        return false;
    }
    if haystack.len() == word.len() {
        return true;
    }
    !is_word_byte(haystack[word.len()])
}

/// Classify `pack`-leading operator text: after `pack`, skip whitespace,
/// require `(`, skip whitespace, then match the `push`/`pop` operator
/// word. Anything else is an unrecognized form and stays opaque
/// (arguments beyond the operator are not interpreted here).
fn classify_pack_text(core: &[u8]) -> PragmaClass {
    if core.len() < 4 || &core[0..4] != b"pack" {
        return PragmaClass::Opaque;
    }
    if core.len() > 4 && !is_pragma_space(core[4]) && core[4] != b'(' {
        return PragmaClass::Opaque;
    }
    let mut cursor = 4;
    while cursor < core.len() && is_pragma_space(core[cursor]) {
        cursor += 1;
    }
    if cursor >= core.len() || core[cursor] != b'(' {
        return PragmaClass::Opaque;
    }
    cursor += 1;
    while cursor < core.len() && is_pragma_space(core[cursor]) {
        cursor += 1;
    }
    if starts_with_word(&core[cursor..], b"push") {
        return PragmaClass::PackPush;
    }
    if starts_with_word(&core[cursor..], b"pop") {
        return PragmaClass::PackPop;
    }
    PragmaClass::Opaque
}

/// Classify decoded `_Pragma("...")` text at the byte level: surrounding
/// ASCII whitespace is trimmed, `once` alone is the header-guard flag,
/// `pack`-leading text goes through the pack matcher, and everything
/// else is opaque (benign ignore per C11 6.10.6p1).
pub fn classify_operator_text(text: &[u8]) -> PragmaClass {
    let mut start = 0;
    while start < text.len() && is_pragma_space(text[start]) {
        start += 1;
    }
    let mut end = text.len();
    while end > start && is_pragma_space(text[end - 1]) {
        end -= 1;
    }
    if end <= start {
        return PragmaClass::Opaque;
    }
    let core = &text[start..end];
    if core == b"once" {
        return PragmaClass::Once;
    }
    classify_pack_text(core)
}

/// Classify `#pragma` directive params (the post-`pragma` tokens): exactly
/// `[once]` is the header-guard flag; `[pack, (, push|pop, ...]` is the
/// alignment-stack op; everything else — including an empty param list
/// and any supported-set pragma with no special handling — is opaque
/// (benign ignore per C11 6.10.6p1).
pub fn classify_directive_params(params: &[(PpTokenId, PpTokenRecord)]) -> PragmaClass {
    if params.is_empty() {
        return PragmaClass::Opaque;
    }
    if params.len() == 1 && is_identifier_spelling(&params[0].1, b"once") {
        return PragmaClass::Once;
    }
    if params.len() >= 3
        && is_identifier_spelling(&params[0].1, b"pack")
        && is_punct_spelling(&params[1].1, b"(")
    {
        if is_identifier_spelling(&params[2].1, b"push") {
            return PragmaClass::PackPush;
        }
        if is_identifier_spelling(&params[2].1, b"pop") {
            return PragmaClass::PackPop;
        }
    }
    PragmaClass::Opaque
}

/// The shared malformed-operand diagnostic for `_Pragma` shapes that are
/// not exactly `_Pragma ( StringLiteral )`.
fn malformed_pragma_operand() -> DiagnosticDraft {
    DiagnosticDraft::error(
        DiagnosticCode::new(DiagGroup::Task, 4),
        "malformed _Pragma operand: expected `(\"...\")`",
    )
}

/// The shared not-running diagnostic for a mis-dispatched pragma task.
fn pragma_not_running(task: TaskId) -> DiagnosticDraft {
    DiagnosticDraft::error(
        DiagnosticCode::new(DiagGroup::Task, 4),
        format!("pragma task {} is not running", task.index()),
    )
}

/// Decode the `_Pragma ( StringLiteral )` operator token shape to its
/// classification. Any arity or token-kind deviation is Invalid (the
/// operand cannot be given a meaning); string-decode failures reuse the
/// typed `PragmaStringError` mapping.
fn decode_pragma_operator(input: &PpPragmaInput) -> Result<PragmaClass, DiagnosticDraft> {
    if input.tokens.len() != 4 {
        return Err(malformed_pragma_operand());
    }
    if !is_punct_spelling(&input.tokens[1].1, b"(") {
        return Err(malformed_pragma_operand());
    }
    if input.tokens[2].1.kind != PpTokenKind::StringLiteral {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "malformed _Pragma operand: expected a string literal",
        ));
    }
    if !is_punct_spelling(&input.tokens[3].1, b")") {
        return Err(malformed_pragma_operand());
    }
    let text = match decode_pragma_string(input.tokens[2].1.spelling.as_slice()) {
        Ok(text) => text,
        Err(error) => return Err(error.to_diagnostic()),
    };
    Ok(classify_operator_text(text.as_slice()))
}

/// Classify one projected pragma construct: post-`#` params after
/// Identifier `pragma`, or the `_Pragma ( StringLiteral )` operator
/// shape. Any other first token is a wrong-dispatch protocol fault.
pub fn classify_pragma_input(input: &PpPragmaInput) -> Result<PragmaClass, DiagnosticDraft> {
    let first = match input.tokens.first() {
        Some(found) => found,
        None => {
            return Err(protocol_fault(format!(
                "pragma task {} payload must carry at least one pp-token",
                input.task.index()
            )));
        }
    };
    if is_identifier_spelling(&first.1, b"pragma") {
        return Ok(classify_directive_params(&input.tokens[1..]));
    }
    if is_identifier_spelling(&first.1, b"_Pragma") {
        return decode_pragma_operator(input);
    }
    Err(protocol_fault(format!(
        "pragma task {} dispatched a non-pragma token line",
        input.task.index()
    )))
}

impl PpPragmaChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// Every path yields exactly one terminal proposal: `Ack` for any
    /// well-formed pragma (the classification is the record content;
    /// persistence waits on the `PragmaRecord` freeze), or a
    /// typed `Fail` for mis-dispatch and malformed `_Pragma` strings.
    /// There is no fan-out, so no frozen join applies.
    pub fn compute(&self, input: &PpPragmaInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![crate::chips::fail(
                input.task,
                pragma_not_running(input.task),
            )];
        }
        let class = match classify_pragma_input(input) {
            Ok(class) => class,
            Err(diagnostic) => return vec![crate::chips::fail(input.task, diagnostic)],
        };
        let _ = class;
        vec![Proposal::Complete {
            task: input.task,
            value: ResultValue::Ack,
        }]
    }
}
