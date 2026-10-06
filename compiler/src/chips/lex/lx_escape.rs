// ============================================================================
// chips/lx_escape.rs — T04 LX11 escape-decode worker
// (Wave 3 slice 8, `/33`)
//
// LiteralBody → CodeUnits: decode C escape sequences in literal body bytes
// into Unicode scalar code-unit values (`Vec<u32>`, content units only, no
// terminator — the string terminator is LX13's job, the empty body is LX12's
// `Fail`). Rule set (C11 6.4.4.4 + 6.4.3): simple escapes, octal (at most 3
// digits), greedy hexadecimal (`\x` consumes every following hex digit, at
// least one required), `\uXXXX` / `\UXXXXXXXX` with range validation. Any
// illegal escape is a typed `Fail` ("invalid ..."), never a panic and never
// a silent reinterpretation.
//
// Frozen registration (integrator-owned): `LX11_TASK_KIND` aliases
// `TaskKind::LEX_ESCAPE_DECODE` (`LEX` local 21, first code after `/32`),
// `LX11_CHIP` is `crate::manifest::LX11_CHIP` (`ChipId(41)`), the
// kind-registry row lives in `TaskKindRegistry::lx_string_slice()`, the
// stage-2 row in `STAGE_ASSIGNMENT`, the routed layer is 2, and the
// acceptance test is `compiler/tests/c33_string.rs`. No allowlist row:
// the chip is Ack-only (zero writes).
//
// ## Target character encoding (documented assumption)
//
// The execution character set is Unicode with a UTF-8 narrow encoding:
//
// - Simple/octal/hex escapes yield scalar values directly (`\n` → `0x0A`,
//   `\x41` → `0x41`, `\xff` → `0xFF`). A lone `0xFF`-style byte is preserved
//   as a scalar even though it is not well-formed UTF-8 on its own: C narrow
//   strings carry raw byte values, and width enforcement belongs to the
//   prefix-owning consumer (LX12/LX13), not here.
// - `\u`/`\U` values are Unicode scalar values placed directly (LX13 encodes
//   them per prefix: UTF-8 bytes, UTF-16 pairs, UTF-32 units; narrow units
//   keep values that fit in one byte).
// - Raw source bytes are assumed to be UTF-8 (same as the target): ASCII
//   bytes map to their scalar value, well-formed multibyte sequences decode
//   to one scalar each, malformed sequences fail. Source encodings other
//   than UTF-8 are outside this chip's scope.
// - Hex/octal values pass through untruncated (up to `u32::MAX` for greedy
//   hex); prefix unit-width masking/truncation is LX12/LX13 policy, not LX11.
// - Only UCNs face scalar-range rejection here (surrogates, `> U+10FFFF`,
//   and the C11 6.4.3 `< U+00A0` rule except `$`/`@`/`` ` ``), because C
//   constrains universal character names themselves. Hex/octal ranges are
//   consumer width rules.
//
// ## Temp-wire + revert
//
// Decoding accumulates into a chip-local `Vec<u32>` scratch (the temp wire).
// The scratch is returned only on full success; any invalid escape, bad
// encoding, or range violation drops the scratch and yields exactly one
// `Fail` proposal (no partial append, no state change). There is no
// guessing or backtracking: greedy `\x` and 3-digit octal apply
// deterministically, and every ill-formed input maps to one typed defect.
//
// ## Escape-copy reconciliation (frozen)
//
// LX12 (`lx_char.rs`) and LX13 (`lx_string.rs`) each carry a chip-local
// escape copy and explicitly import no shared helper: each chip keeps its
// own local copy by the `compose_map`/`FloatParts` copy precedent, and the
// frozen interchange is the spelling plus the `lx.escape-decode` rule, not
// a shared function. Convergence on one helper is deferred to a
// T04/T01 co-freeze (behavioral deltas are documented per chip and pinned
// by `c33_string`; see `docs/tasks/LX_STRING_SLICE.md` §3). Known deltas:
// LX11 UTF-8-decodes raw non-ASCII bytes (LX12 and LX13-narrow pass bytes
// through; LX13-wide decodes UTF-8); LX11 passes hex values through
// untruncated (LX12 truncates to the prefix width, LX13-narrow fails above
// `0xFF`); LX11 enforces the UCN `< U+00A0` trio rule (siblings check only
// surrogates/`> U+10FFFF`).
//
// ## Open items (recorded, not masked)
//
// - CodeUnits carrier: no frozen task-result or record carrier exists for
//   `CodeUnits`, so `compute` completes with `Ack` (decode validity) on
//   success instead of publishing the units. The units are available through
//   the pure `decode_escape_body` for direct same-tick reuse by LX12/LX13
//   (same pattern as `decimal_magnitude`/`is_keyword`); a task-boundary
//   carrier (child-task result or record family) is a T04/T01 co-freeze item.
// - GNU `\e` is rejected as an unknown escape; extension escapes are
//   LX15 scope, gated on a dialect mode this chip does not take.
// ============================================================================

use crate::bus::PpTokenKind;
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{RecordRef, SpanId, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, LX11_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

/// Frozen LX11 task kind (aliases `TaskKind::LEX_ESCAPE_DECODE`,
/// frozen by the `/33` integrator; the local code is `LEX` 21, first
/// code after `/32`).
pub const LX11_TASK_KIND: TaskKind = TaskKind::LEX_ESCAPE_DECODE;

/// Decoded literal-body code units: Unicode scalar values in body order.
///
/// Content units only (no terminator; LX13 appends exactly one). Narrow and
/// wide encodings of these scalars are the prefix-owning consumer's job;
/// see the module header for the target-encoding contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeUnits(pub Vec<u32>);

impl CodeUnits {
    /// Unit values in body order.
    pub fn units(&self) -> &[u32] {
        &self.0
    }

    /// Consume into the unit vector.
    pub fn into_units(self) -> Vec<u32> {
        self.0
    }
}

/// LX11 escape-decode failure (always surfaces as a typed `Fail`, never a
/// panic and never a silent substitution).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EscapeError {
    /// The spelling has no well-formed `prefix"body"` / `prefix'body'` shape.
    Unterminated,
    /// The body ends with a lone `\`.
    TrailingBackslash,
    /// `\` followed by a byte that starts no escape (holds the byte).
    UnknownEscape(u8),
    /// `\x` with no following hex digit.
    EmptyHex,
    /// Greedy `\x` value exceeds 32 bits (no consumer can represent it).
    HexTooLarge,
    /// `\u`/`\U` cut short by the end of the body.
    TruncatedUniversal,
    /// `\u`/`\U` containing a non-hex digit.
    NonHexUniversal,
    /// UCN naming a surrogate half (`U+D800..U+DFFF`).
    SurrogateUniversal,
    /// UCN above `U+10FFFF`.
    OutOfRangeUniversal,
    /// UCN below `U+00A0` other than `$` (`U+0024`), `@` (`U+0040`),
    /// `` ` `` (`U+0060`); C11 6.4.3.
    ControlUniversal,
    /// A raw multibyte sequence is not well-formed UTF-8.
    BadUtf8,
    /// A raw newline or carriage return inside the body.
    RawLineBreak,
}

impl EscapeError {
    /// Human-readable detail for the diagnostic message.
    pub const fn message(&self) -> &'static str {
        match self {
            Self::Unterminated => "unterminated literal",
            Self::TrailingBackslash => "trailing backslash in literal body",
            Self::UnknownEscape(_) => "unknown escape sequence",
            Self::EmptyHex => "`\\x` needs at least one hex digit",
            Self::HexTooLarge => "hex escape value exceeds 32 bits",
            Self::TruncatedUniversal => "truncated universal character name",
            Self::NonHexUniversal => "universal character name needs hex digits",
            Self::SurrogateUniversal => "universal character name is a surrogate half",
            Self::OutOfRangeUniversal => "universal character name above U+10FFFF",
            Self::ControlUniversal => "universal character name below U+00A0 must be $, @, or `",
            Self::BadUtf8 => "invalid UTF-8 in literal body",
            Self::RawLineBreak => "unescaped line break in literal body",
        }
    }
}

impl std::fmt::Display for EscapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownEscape(byte) => {
                write!(f, "unknown escape sequence ('\\{}')", (*byte) as char)
            }
            _ => write!(f, "{}", self.message()),
        }
    }
}

/// Numeric value of one ASCII hex digit, if the byte is one.
fn hex_value(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some(u32::from(byte - b'0')),
        b'a'..=b'f' => Some(u32::from(byte - b'a') + 10),
        b'A'..=b'F' => Some(u32::from(byte - b'A') + 10),
        _ => None,
    }
}

/// Decode one strict UTF-8 sequence starting at `bytes[0]` (which the caller
/// guarantees is `>= 0x80`). Returns the scalar value and its byte length;
/// rejects overlong forms, surrogate halves, values above `U+10FFFF`,
/// bad continuations, and truncation.
fn decode_utf8_char(bytes: &[u8]) -> Result<(u32, usize), EscapeError> {
    let first = bytes[0];
    let (mut value, length) = if first & 0xE0 == 0xC0 {
        (u32::from(first & 0x1F), 2)
    } else if first & 0xF0 == 0xE0 {
        (u32::from(first & 0x0F), 3)
    } else if first & 0xF8 == 0xF0 {
        (u32::from(first & 0x07), 4)
    } else {
        return Err(EscapeError::BadUtf8);
    };
    if bytes.len() < length {
        return Err(EscapeError::BadUtf8);
    }
    for &byte in bytes.iter().skip(1).take(length - 1) {
        if byte & 0xC0 != 0x80 {
            return Err(EscapeError::BadUtf8);
        }
        value = (value << 6) | u32::from(byte & 0x3F);
    }
    let min = match length {
        2 => 0x80,
        3 => 0x800,
        _ => 0x10_000,
    };
    if value < min || (0xD800..0xE000).contains(&value) || value > 0x10_FFFF {
        return Err(EscapeError::BadUtf8);
    }
    Ok((value, length))
}

/// Decode one escape starting at `body[pos] == b'\\'`.
///
/// Returns the scalar value and the position just past the escape. Octal
/// takes at most 3 digits, hex is greedy, UCNs take exactly 4 (`\u`) or 8
/// (`\U`) digits with scalar-range validation.
fn decode_one_escape(body: &[u8], pos: usize) -> Result<(u32, usize), EscapeError> {
    debug_assert_eq!(body[pos], b'\\');
    let Some(&head) = body.get(pos + 1) else {
        return Err(EscapeError::TrailingBackslash);
    };
    match head {
        b'\'' => Ok((0x27, pos + 2)),
        b'"' => Ok((0x22, pos + 2)),
        b'?' => Ok((0x3F, pos + 2)),
        b'\\' => Ok((0x5C, pos + 2)),
        b'a' => Ok((0x07, pos + 2)),
        b'b' => Ok((0x08, pos + 2)),
        b'f' => Ok((0x0C, pos + 2)),
        b'n' => Ok((0x0A, pos + 2)),
        b'r' => Ok((0x0D, pos + 2)),
        b't' => Ok((0x09, pos + 2)),
        b'v' => Ok((0x0B, pos + 2)),
        b'0'..=b'7' => {
            let mut value: u32 = 0;
            let mut end = pos + 1;
            while end < body.len() && end < pos + 4 && matches!(body[end], b'0'..=b'7') {
                value = value * 8 + u32::from(body[end] - b'0');
                end += 1;
            }
            Ok((value, end))
        }
        b'x' => {
            let mut value: u64 = 0;
            let mut end = pos + 2;
            while end < body.len() {
                let Some(digit) = hex_value(body[end]) else {
                    break;
                };
                value = value * 16 + u64::from(digit);
                end += 1;
            }
            if end == pos + 2 {
                return Err(EscapeError::EmptyHex);
            }
            if value > u64::from(u32::MAX) {
                return Err(EscapeError::HexTooLarge);
            }
            Ok((value as u32, end))
        }
        b'u' | b'U' => {
            let digits = if head == b'u' { 4 } else { 8 };
            if body.len().saturating_sub(pos + 2) < digits {
                return Err(EscapeError::TruncatedUniversal);
            }
            let mut value: u32 = 0;
            for offset in 0..digits {
                match hex_value(body[pos + 2 + offset]) {
                    Some(digit) => {
                        value = value * 16 + digit;
                    }
                    None => return Err(EscapeError::NonHexUniversal),
                }
            }
            if value > 0x10_FFFF {
                return Err(EscapeError::OutOfRangeUniversal);
            }
            if (0xD800..0xE000).contains(&value) {
                return Err(EscapeError::SurrogateUniversal);
            }
            if value < 0xA0 && value != 0x24 && value != 0x40 && value != 0x60 {
                return Err(EscapeError::ControlUniversal);
            }
            Ok((value, pos + 2 + digits))
        }
        _ => Err(EscapeError::UnknownEscape(head)),
    }
}

/// Split a full literal spelling (`"..."`, `'...'`, with any prefix) into
/// the raw body bytes between the quotes.
///
/// The prefix (if any) is ignored: prefix semantics belong to LX12/LX13.
/// The opening quote is the first `'` or `"` byte; the closing quote must be
/// the same byte as the spelling's last byte.
pub fn split_literal_body(spelling: &[u8]) -> Result<&[u8], EscapeError> {
    let mut open: Option<(usize, u8)> = None;
    for (index, &byte) in spelling.iter().enumerate() {
        if byte == b'\'' || byte == b'"' {
            open = Some((index, byte));
            break;
        }
    }
    let Some((start, quote)) = open else {
        return Err(EscapeError::Unterminated);
    };
    if spelling.len() < start + 2 || spelling[spelling.len() - 1] != quote {
        return Err(EscapeError::Unterminated);
    }
    Ok(&spelling[start + 1..spelling.len() - 1])
}

/// Decode literal body bytes into code units (the LX11 rule set).
///
/// This is the temp-wire scratch owner: callers keep the returned units on
/// `Ok` and drop everything on `Err` (revert). An empty body decodes to no
/// units (the empty-string terminator and the empty-character `Fail` are
/// LX13/LX12 policy, not LX11). No terminator is appended.
pub fn decode_escape_body(body: &[u8]) -> Result<CodeUnits, EscapeError> {
    let mut units: Vec<u32> = Vec::new();
    let mut pos = 0;
    while pos < body.len() {
        let byte = body[pos];
        if byte == b'\\' {
            let (unit, next) = decode_one_escape(body, pos)?;
            units.push(unit);
            pos = next;
        } else if byte == b'\n' || byte == b'\r' {
            return Err(EscapeError::RawLineBreak);
        } else if byte < 0x80 {
            units.push(u32::from(byte));
            pos += 1;
        } else {
            let (unit, length) = decode_utf8_char(&body[pos..])?;
            units.push(unit);
            pos += length;
        }
    }
    Ok(CodeUnits(units))
}

/// Narrow projection for the escape-decode computation.
#[derive(Clone, Debug)]
pub struct LxEscapeInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Committed PP spelling bytes (`prefix"body"` / `prefix'body'`, escapes
    /// preserved); the body is split out by `compute`.
    pub spelling: Vec<u8>,
    /// Committed PP span, attached to `Fail` diagnostics.
    pub span: SpanId,
}

/// Build the narrow projection for one dispatched task.
///
/// Decode input is the committed T03 PP spelling/kind (DOC-10 resolution).
/// Only `CharLiteral`/`StringLiteral` PP tokens are accepted; anything else
/// is explicit `Unsupported`, never a fabricated decode. The `Token` payload
/// ref is tag-checked only (the family envelope is `[Token, PpToken]`); the
/// token arena is not read because no back-link data is needed until the
/// `CodeUnits` carrier lands.
pub fn project_lx_escape_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<LxEscapeInput, DiagnosticDraft> {
    let record =
        bus.arenas.tasks.get(task).map_err(|_| {
            protocol_fault(format!("escape decode of unknown task {}", task.index()))
        })?;
    if record.kind != LX11_TASK_KIND {
        return Err(protocol_fault(format!(
            "escape decode task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 2 {
        return Err(protocol_fault(format!(
            "escape decode task {} payload must be exactly [Token, PpToken], got {} refs",
            task.index(),
            record.payload.refs.len()
        )));
    }
    let (is_token, Some(pp_token)) = (
        matches!(record.payload.refs[0], RecordRef::Token(_)),
        match record.payload.refs[1] {
            RecordRef::PpToken(id) => Some(id),
            _ => None,
        },
    ) else {
        return Err(protocol_fault(format!(
            "escape decode task {} payload must be exactly [Token, PpToken]",
            task.index()
        )));
    };
    if !is_token {
        return Err(protocol_fault(format!(
            "escape decode task {} payload must be exactly [Token, PpToken]",
            task.index()
        )));
    }
    let pp_body = bus.arenas.pp_tokens.get(pp_token).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("escape decode reads missing pp-token {}", pp_token.index()),
        )
    })?;
    if pp_body.kind != PpTokenKind::CharLiteral && pp_body.kind != PpTokenKind::StringLiteral {
        return Err(DiagnosticDraft::unsupported(
            "escape decode reads a non-literal pp-token",
        ));
    }
    Ok(LxEscapeInput {
        task,
        state: record.state.clone(),
        spelling: pp_body.spelling.clone(),
        span: pp_body.span,
    })
}

/// Build the single `Fail` proposal for an invalid literal body.
fn invalid_escape_fail(task: TaskId, span: SpanId, error: EscapeError) -> Proposal {
    fail(
        task,
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("invalid escape in literal body: {error}"),
        )
        .with_span(span),
    )
}

/// The T04 LX11 escape-decode worker.
pub struct LxEscapeChip;

impl Worker for LxEscapeChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: LX11_CHIP,
            chip_name: "LxEscapeChip",
            group: TaskGroup::LEX,
            task_kinds: vec![LX11_TASK_KIND],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Pp, "tokens"),
            ],
            // No appends: without the `CodeUnits` carrier there is nothing LX11 may
            // publish, so a successful decode only acknowledges validity.
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c33_string.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_lx_escape_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl LxEscapeChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// Failure paths yield exactly one `Fail` (the temp-wire scratch is
    /// reverted, never partially published). On success the fully validated
    /// units are acknowledged with `Ack`; publishing them awaits the
    /// `CodeUnits` carrier, so they are recomputed by the consumer through
    /// `decode_escape_body` until then.
    pub fn compute(&self, input: &LxEscapeInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("escape decode task {} is not running", input.task.index()),
                ),
            )];
        }
        let body = match split_literal_body(&input.spelling) {
            Ok(body) => body,
            Err(error) => return vec![invalid_escape_fail(input.task, input.span, error)],
        };
        // `CodeUnits` carrier pending: validated units acknowledged, not
        // published (no carrier). The `Ok` arm drops the scratch here:
        // nothing escapes this scope except the Ack below.
        if let Err(error) = decode_escape_body(body) {
            return vec![invalid_escape_fail(input.task, input.span, error)];
        }
        vec![Proposal::Complete {
            task: input.task,
            value: ResultValue::Ack,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::{CompilerBus, PpTokenRecord, SpanRecord, TokenKind, TokenRecord};
    use crate::ids::{PpTokenId, SourceId, TokenId};
    use crate::limits::Limits;
    use crate::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
    use crate::task::{Payload, TaskDraft};

    fn units(body: &[u8]) -> Vec<u32> {
        decode_escape_body(body)
            .expect("valid test body")
            .into_units()
    }

    fn escape_error(body: &[u8]) -> EscapeError {
        decode_escape_body(body).expect_err("body must be invalid")
    }

    #[test]
    fn simple_escapes_map_to_ascii_controls() {
        assert_eq!(
            units(b"\\a\\b\\f\\n\\r\\t\\v\\\\\\'\\\"\\?"),
            vec![0x07, 0x08, 0x0C, 0x0A, 0x0D, 0x09, 0x0B, 0x5C, 0x27, 0x22, 0x3F]
        );
    }

    #[test]
    fn octal_takes_at_most_three_digits() {
        assert_eq!(units(b"\\0"), vec![0x00]);
        assert_eq!(units(b"\\7"), vec![0x07]);
        // `\1011` is 'A' followed by a literal '1', not one 4-digit escape.
        assert_eq!(units(b"\\1011"), vec![0x41, 0x31]);
        // Full 3-digit range passes through for the consumer width check.
        assert_eq!(units(b"\\777"), vec![0x1FF]);
    }

    #[test]
    fn hex_is_greedy_and_needs_a_digit() {
        // Greedy: `\x41B` is ONE escape (0x41B); LX11 does not truncate to a
        // prefix width (consumer policy; see the reconciliation note above).
        assert_eq!(units(b"\\x41B"), vec![0x41B]);
        assert_eq!(units(b"\\x41"), vec![0x41]);
        assert_eq!(escape_error(b"\\x"), EscapeError::EmptyHex);
    }

    #[test]
    fn huge_hex_fails_only_past_u32() {
        assert_eq!(units(b"\\xFFFFFFFF"), vec![0xFFFF_FFFF]);
        assert_eq!(escape_error(b"\\x1FFFFFFFF"), EscapeError::HexTooLarge);
    }

    #[test]
    fn universal_names_decode_and_validate_ranges() {
        assert_eq!(units(b"\\u00E9"), vec![0xE9]);
        assert_eq!(units(b"\\U0001F600"), vec![0x1F600]);
        assert_eq!(units(b"\\U0010FFFF"), vec![0x10FFFF]);
        // The C11 6.4.3 trio stays legal below U+00A0.
        assert_eq!(units(b"\\u0024\\u0040\\u0060"), vec![0x24, 0x40, 0x60]);
        assert_eq!(escape_error(b"\\uD800"), EscapeError::SurrogateUniversal);
        assert_eq!(escape_error(b"\\uDFFF"), EscapeError::SurrogateUniversal);
        assert_eq!(
            escape_error(b"\\U00110000"),
            EscapeError::OutOfRangeUniversal
        );
        assert_eq!(escape_error(b"\\u000A"), EscapeError::ControlUniversal);
        assert_eq!(escape_error(b"\\u0000"), EscapeError::ControlUniversal);
        assert_eq!(escape_error(b"\\u12"), EscapeError::TruncatedUniversal);
        assert_eq!(escape_error(b"\\u12GH"), EscapeError::NonHexUniversal);
        assert_eq!(escape_error(b"\\U0001F60"), EscapeError::TruncatedUniversal);
    }

    #[test]
    fn unknown_and_truncated_escapes_fail() {
        assert!(matches!(
            escape_error(b"\\q"),
            EscapeError::UnknownEscape(b'q')
        ));
        // GNU `\e` is LX15 scope here, not a silent extension.
        assert!(matches!(
            escape_error(b"\\e"),
            EscapeError::UnknownEscape(b'e')
        ));
        assert_eq!(escape_error(b"abc\\"), EscapeError::TrailingBackslash);
        assert_eq!(escape_error(b"\\8"), EscapeError::UnknownEscape(b'8'));
        assert_eq!(escape_error(b"\\9"), EscapeError::UnknownEscape(b'9'));
    }

    #[test]
    fn raw_line_breaks_fail() {
        assert_eq!(escape_error(b"a\nb"), EscapeError::RawLineBreak);
        assert_eq!(escape_error(b"a\rb"), EscapeError::RawLineBreak);
    }

    #[test]
    fn empty_body_decodes_to_no_units() {
        assert_eq!(units(b""), Vec::<u32>::new());
    }

    #[test]
    fn raw_bytes_assume_utf8_source_and_target() {
        // U+00E9 in UTF-8 source bytes decodes to the scalar (documented
        // target contract; differs from the byte-passthrough sibling copies).
        assert_eq!(units(&[0xC3, 0xA9]), vec![0xE9]);
        assert_eq!(units(b"a"), vec![0x61]);
        assert_eq!(units(&[0x00]), vec![0x00]);
        // Malformed UTF-8 never becomes replacement characters or bytes.
        assert_eq!(escape_error(&[0x80]), EscapeError::BadUtf8);
        assert_eq!(escape_error(&[0xC0, 0xAF]), EscapeError::BadUtf8);
        assert_eq!(escape_error(&[0xE2, 0x82]), EscapeError::BadUtf8);
        assert_eq!(escape_error(&[0xED, 0xA0, 0x80]), EscapeError::BadUtf8);
    }

    #[test]
    fn body_split_handles_prefixes_and_quotes() {
        assert_eq!(split_literal_body(b"\"ab\""), Ok(b"ab".as_slice()));
        assert_eq!(split_literal_body(b"'a'"), Ok(b"a".as_slice()));
        assert_eq!(split_literal_body(b"L\"ab\""), Ok(b"ab".as_slice()));
        assert_eq!(split_literal_body(b"u8'x'"), Ok(b"x".as_slice()));
        assert_eq!(split_literal_body(b"\"a'b\""), Ok(b"a'b".as_slice()));
        assert_eq!(split_literal_body(b"\"\""), Ok(b"".as_slice()));
        assert_eq!(split_literal_body(b"\"ab"), Err(EscapeError::Unterminated));
        assert_eq!(split_literal_body(b"ab"), Err(EscapeError::Unterminated));
        assert_eq!(split_literal_body(b"\""), Err(EscapeError::Unterminated));
    }

    fn running_input(spelling: &[u8], span: SpanId) -> LxEscapeInput {
        LxEscapeInput {
            task: TaskId::from_index(0),
            state: TaskState::Running,
            spelling: spelling.to_vec(),
            span,
        }
    }

    #[test]
    fn compute_acks_valid_body() {
        let span = SpanId::from_index(0);
        let proposals = LxEscapeChip.compute(&running_input(b"\"a\\nB\\x41\"", span));
        assert_eq!(proposals.len(), 1);
        assert!(matches!(
            proposals[0],
            Proposal::Complete {
                value: ResultValue::Ack,
                ..
            }
        ));
    }

    #[test]
    fn compute_reverts_to_single_fail_on_invalid() {
        let span = SpanId::from_index(0);
        // Revert: exactly one Fail, never a partial AppendRecords beside it.
        for spelling in [
            b"\"\\q\"".as_slice(),
            b"\"abc\\".as_slice(),
            b"\"a\nb\"".as_slice(),
        ] {
            let proposals = LxEscapeChip.compute(&running_input(spelling, span));
            assert_eq!(proposals.len(), 1, "spelling: {spelling:?}");
            let Proposal::Fail { diagnostic, .. } = &proposals[0] else {
                panic!("invalid body must fail: {spelling:?}");
            };
            assert!(diagnostic
                .message
                .starts_with("invalid escape in literal body"));
            assert_eq!(diagnostic.span, Some(span));
        }
    }

    #[test]
    fn compute_rejects_unterminated_spelling() {
        let span = SpanId::from_index(0);
        let proposals = LxEscapeChip.compute(&running_input(b"\"ab", span));
        assert_eq!(proposals.len(), 1);
        assert!(matches!(proposals[0], Proposal::Fail { .. }));
    }

    #[test]
    fn non_running_task_fails() {
        let mut bad = running_input(b"\"a\"", SpanId::from_index(0));
        bad.state = TaskState::Ready;
        assert!(matches!(
            LxEscapeChip.compute(&bad)[0],
            Proposal::Fail { .. }
        ));
    }

    #[test]
    fn compute_is_deterministic() {
        let first = LxEscapeChip.compute(&running_input(b"'\\u00E9\\x41'", SpanId::from_index(0)));
        let second = LxEscapeChip.compute(&running_input(b"'\\u00E9\\x41'", SpanId::from_index(0)));
        assert_eq!(first, second);
    }

    fn seed_bus(spelling: &[u8], kind: PpTokenKind) -> (CompilerBus, TaskId) {
        let mut bus = CompilerBus::new(CompilerConfig::new(
            TargetSpec::aarch64_unknown_linux_gnu_unverified(),
            Dialect::C11,
            OptLevel::O0,
            vec![],
            Limits::fixture(),
        ));
        let limits = bus.limits();
        let name = bus.intern_name(b"lx_escape.c").unwrap();
        let source: SourceId = bus.alloc_source(name, spelling.to_vec()).unwrap();
        let span = bus
            .arenas
            .spans
            .alloc(
                SpanRecord {
                    source,
                    start: 0,
                    end: spelling.len() as u64,
                    expansion: None,
                },
                &limits,
            )
            .unwrap();
        let pp_token: PpTokenId = bus
            .arenas
            .pp_tokens
            .alloc(
                PpTokenRecord {
                    kind,
                    span,
                    spelling: spelling.to_vec(),
                },
                &limits,
            )
            .unwrap();
        let token: TokenId = bus
            .arenas
            .tokens
            .alloc(
                TokenRecord {
                    kind: TokenKind::Punctuator,
                    span,
                    name: None,
                    pp_token,
                },
                &limits,
            )
            .unwrap();
        let task = bus
            .bootstrap_task(TaskDraft {
                kind: LX11_TASK_KIND,
                owner: LX11_CHIP,
                parent: None,
                payload: Payload::from_refs(vec![
                    RecordRef::Token(token),
                    RecordRef::PpToken(pp_token),
                ]),
                continuation: None,
            })
            .unwrap();
        (bus, task)
    }

    #[test]
    fn projector_accepts_char_and_string_spellings() {
        let (bus, task) = seed_bus(b"\"a\\n\"", PpTokenKind::StringLiteral);
        let input = project_lx_escape_input(&bus, task).expect("string projects");
        assert_eq!(input.spelling, b"\"a\\n\"");
        let (bus, task) = seed_bus(b"'\\x41'", PpTokenKind::CharLiteral);
        let input = project_lx_escape_input(&bus, task).expect("char projects");
        assert_eq!(input.spelling, b"'\\x41'");
    }

    #[test]
    fn projector_rejects_non_literal_pp_token() {
        let (bus, task) = seed_bus(b"894", PpTokenKind::PpNumber);
        assert!(project_lx_escape_input(&bus, task).is_err());
    }

    #[test]
    fn projector_rejects_unknown_task_and_wrong_kind() {
        let (bus, _) = seed_bus(b"\"a\"", PpTokenKind::StringLiteral);
        let missing = project_lx_escape_input(&bus, TaskId::from_index(999));
        assert!(missing.is_err());
        let mut bus = bus;
        let wrong = bus
            .bootstrap_task(TaskDraft {
                kind: TaskKind::LEX_CLASSIFY,
                owner: LX11_CHIP,
                parent: None,
                payload: Payload::empty(),
                continuation: None,
            })
            .unwrap();
        assert!(project_lx_escape_input(&bus, wrong).is_err());
    }

    #[test]
    fn manifest_is_deterministic_and_points_at_this_file() {
        let manifest = LxEscapeChip.manifest();
        assert_eq!(manifest.id, LX11_CHIP);
        assert!(manifest.deterministic);
        assert_eq!(manifest.phase, crate::manifest::ChipPhase::Propagation);
        assert!(manifest.tests.contains(&"compiler/tests/c33_string.rs"));
    }
}
