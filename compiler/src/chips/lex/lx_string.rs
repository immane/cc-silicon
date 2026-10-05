// ============================================================================
// chips/lex/lx_string.rs — T04 LX13 string-literal worker
// (Wave 3 slice 8, `/33`)
//
// Prefix (`none`/`u8`/`L`/`u`/`U`) + quoted body -> one `LiteralRecord`
// (`kind: String`) carrying the decoded code units, exactly one appended
// terminating zero, and the output-side token back-link (`token: Some`).
//
// Frozen registration (integrator-owned): `LX13_TASK_KIND` aliases
// `TaskKind::LEX_STRING_DECODE` (`LEX` local 23), `LX13_CHIP` is
// `crate::manifest::LX13_CHIP` (`ChipId(43)`), the kind-registry row lives
// in `TaskKindRegistry::lx_string_slice()`, the stage-2 row in
// `STAGE_ASSIGNMENT`, the routed layer is 2, the store-owner allowlist row
// authorizes (`LX13_CHIP`, `lex.literals`), and the acceptance test is
// `compiler/tests/c33_string.rs`.
//
// Length convention (frozen by this chip): `units` (and therefore the
// committed `value` byte length divided by the element width) INCLUDES the
// terminating zero. An embedded NUL (`"\0"`, `"\x00"`, ...) is an ordinary
// mid-string zero unit and is preserved; only the single final unit is the
// terminator. The empty literal `""` decodes to exactly one unit (`[0]`).
//
// Wide encoding (frozen AArch64 GNU/Linux target model): `L` units are
// `wchar_t` (size/encoding read from the frozen config, expected 4/utf32);
// `u` units are UTF-16 code units (2 bytes, non-BMP code points become a
// surrogate pair); `U` units are UTF-32 code units (4 bytes); `none`/`u8`
// units are single bytes. Narrow non-ASCII source bytes pass through as
// byte units; wide non-ASCII source bytes are decoded as UTF-8.
//
// Temp-wire + revert: decoding runs over a chip-local `Vec<u32>` scratch.
// The scratch is moved into the append proposal only on full success; any
// invalid escape, bad encoding, or range violation drops the scratch and
// yields a single `Fail` (no partial append, no state change).
//
// DEFECTs requiring integrator/T04/T01 co-freeze (recorded, not silently
// resolved here). D1 (dedicated kind), D2 (chip ID), and the test path are
// frozen by `/33` (see the header); D3–D6 below stay open:
//
// * D1 task kind: FROZEN as `TaskKind::LEX_STRING_DECODE` (`LEX` local 23,
//   `lex.string_decode`); the manifest and projector below claim it.
// * D2 chip ID: FROZEN as `LX13_CHIP = ChipId(43)` with the store-owner
//   allowlist row for (`lex.literals`, string-decode kind).
// * D3 token kind: `TokenKind` has no `String` member (M1-closed produced
//   subset), so the chip accepts any committed token kind whose PP token
//   is `StringLiteral` instead of requiring a string token kind.
// * D4 radix: the frozen radix domain is `{2, 8, 10, 16}` with no string
//   member; the chip commits `radix: 0` as the `/33`-frozen non-numeric
//   marker, pending co-freeze. `0` must never be read as a numeric base.
// * D5 candidate type: `Lx08CandidateType` is the M1-closed `{Int}` with no
//   string member; the chip commits `Int` as an explicit placeholder that
//   must NOT be consumed as a type. String candidate vocabulary/rules are
//   a T04/T08/T01 co-freeze item; until then no downstream stage may treat
//   this record as typed.
// * D6 prefix re-derivation: the PP scanner owns prefix recognition; this
//   chip re-derives the prefix from the committed spelling (chip-local
//   copy, greedy `u8` before `u`). A scanner/chip disagreement is a
//   protocol DEFECT, reported not masked.
// ============================================================================

use crate::bus::{LiteralRecord, PpTokenKind};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{LiteralId, RecordRef, SpanId, TaskId, TokenId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, LX13_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType};
use crate::target::{ScalarKind, WideCharEncoding};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// Frozen LX13 task kind (aliases `TaskKind::LEX_STRING_DECODE`,
/// frozen by the `/33` integrator; the local code is `LEX` 23).
pub const LX13_TASK_KIND: TaskKind = TaskKind::LEX_STRING_DECODE;

/// String literal prefix, re-derived from the committed spelling (see D6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StringPrefix {
    /// `"..."`: narrow character units.
    None,
    /// `u8"..."`: UTF-8 code units (one byte each here).
    U8,
    /// `L"..."`: wide character units (`wchar_t`).
    Wide,
    /// `u"..."`: UTF-16 code units.
    Char16,
    /// `U"..."`: UTF-32 code units.
    Char32,
}

impl StringPrefix {
    /// Stable label used in diagnostics.
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::U8 => "u8",
            Self::Wide => "L",
            Self::Char16 => "u",
            Self::Char32 => "U",
        }
    }
}

/// Decoded element type of a string literal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StringElementType {
    /// Plain narrow characters (`none` prefix).
    Char,
    /// UTF-8 code units (`u8` prefix).
    Utf8,
    /// Wide characters (`L` prefix; width from the frozen target model).
    Wide,
    /// UTF-16 code units (`u` prefix).
    Char16,
    /// UTF-32 code units (`U` prefix).
    Char32,
}

impl StringElementType {
    /// Stable label used in diagnostics.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Char => "char",
            Self::Utf8 => "char8_t",
            Self::Wide => "wchar_t",
            Self::Char16 => "char16_t",
            Self::Char32 => "char32_t",
        }
    }
}

/// Chip-local decoded string: code-unit values plus element type.
///
/// `units` INCLUDES the single terminating zero (empty `""` -> `[0]`).
/// Embedded NULs are ordinary mid-string zero units.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StringRecord {
    /// Decoded code units, terminator included.
    pub units: Vec<u32>,
    /// Element type of the units.
    pub element_type: StringElementType,
}

/// String decode failure (always surfaces as a `Fail`, never a panic).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StringError {
    /// The spelling has no well-formed `prefix"body"` shape.
    Unterminated,
    /// A `\` escape is malformed, unknown, truncated, or out of range.
    BadEscape(&'static str),
    /// A `\u`/`\U` escape is malformed or names an invalid code point.
    BadUniversal(&'static str),
    /// A wide source byte sequence is not valid UTF-8.
    BadUtf8,
    /// The frozen `wchar_t` width is one this chip does not encode.
    BadWcharWidth(u8),
    /// A narrow `\u`/`\U` value does not fit in one byte (multibyte
    /// conversion is deferred; see the helper docs).
    NarrowUniversalOutOfRange,
}

impl StringError {
    /// Human-readable detail for the diagnostic message.
    pub const fn message(&self) -> &'static str {
        match self {
            Self::Unterminated => "unterminated string literal",
            Self::BadEscape(reason) => reason,
            Self::BadUniversal(reason) => reason,
            Self::BadUtf8 => "invalid UTF-8 in wide string literal",
            Self::BadWcharWidth(_) => "unsupported wchar_t width for L\"...\"",
            Self::NarrowUniversalOutOfRange => {
                "universal character name does not fit in a narrow unit"
            }
        }
    }
}

impl std::fmt::Display for StringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadWcharWidth(width) => {
                write!(f, "unsupported wchar_t width ({width}) for L\"...\"")
            }
            _ => write!(f, "{}", self.message()),
        }
    }
}

/// Split the literal prefix from the committed spelling (chip-local copy of
/// the scanner's recognition; greedy `u8` before `u`; see D6).
pub fn split_string_prefix(spelling: &[u8]) -> (StringPrefix, &[u8]) {
    if spelling.len() >= 3 && spelling[0] == b'u' && spelling[1] == b'8' {
        (StringPrefix::U8, &spelling[2..])
    } else if let Some((&first, rest)) = spelling.split_first() {
        match first {
            b'L' => (StringPrefix::Wide, rest),
            b'u' => (StringPrefix::Char16, rest),
            b'U' => (StringPrefix::Char32, rest),
            _ => (StringPrefix::None, spelling),
        }
    } else {
        (StringPrefix::None, spelling)
    }
}

/// Element type and unit width for a prefix at the frozen target model.
///
/// `wchar_bytes` is the configured `wchar_t` size (expected 4 on the
/// frozen AArch64 target); any other width is an explicit failure, never
/// a silent host-ABI substitution.
pub fn element_for_prefix(
    prefix: StringPrefix,
    wchar_bytes: u8,
) -> Result<(StringElementType, u8), StringError> {
    match prefix {
        StringPrefix::None => Ok((StringElementType::Char, 1)),
        StringPrefix::U8 => Ok((StringElementType::Utf8, 1)),
        StringPrefix::Char16 => Ok((StringElementType::Char16, 2)),
        StringPrefix::Char32 => Ok((StringElementType::Char32, 4)),
        StringPrefix::Wide => match wchar_bytes {
            2 => Ok((StringElementType::Wide, 2)),
            4 => Ok((StringElementType::Wide, 4)),
            other => Err(StringError::BadWcharWidth(other)),
        },
    }
}

/// Decode one UTF-8 sequence starting at `bytes[0]` (which is `>= 0x80`).
/// Returns the code point and its byte length; rejects overlong forms,
/// surrogates, values above `U+10FFFF`, and truncation.
fn decode_utf8_char(bytes: &[u8]) -> Result<(u32, usize), StringError> {
    let first = bytes[0];
    let (code_point, length) = if first & 0xE0 == 0xC0 {
        (u32::from(first & 0x1F), 2)
    } else if first & 0xF0 == 0xE0 {
        (u32::from(first & 0x0F), 3)
    } else if first & 0xF8 == 0xF0 {
        (u32::from(first & 0x07), 4)
    } else {
        return Err(StringError::BadUtf8);
    };
    if bytes.len() < length {
        return Err(StringError::BadUtf8);
    }
    let mut value = code_point;
    for &byte in bytes.iter().skip(1).take(length - 1) {
        if byte & 0xC0 != 0x80 {
            return Err(StringError::BadUtf8);
        }
        value = (value << 6) | u32::from(byte & 0x3F);
    }
    let min = match length {
        2 => 0x80,
        3 => 0x800,
        _ => 0x10000,
    };
    if value < min || (0xD800..0xE000).contains(&value) || value > 0x10FFFF {
        return Err(StringError::BadUtf8);
    }
    Ok((value, length))
}

/// Hex digit value, if the byte is an ASCII hex digit.
fn hex_value(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some(u32::from(byte - b'0')),
        b'a'..=b'f' => Some(u32::from(byte - b'a') + 10),
        b'A'..=b'F' => Some(u32::from(byte - b'A') + 10),
        _ => None,
    }
}

/// Push one code point for the element type, encoding non-BMP values as a
/// UTF-16 surrogate pair for 2-byte units. Surrogate halves from escapes
/// are invalid; narrow units hold at most `0xFF`.
fn push_code_point(
    scratch: &mut Vec<u32>,
    unit_bytes: u8,
    narrow: bool,
    value: u32,
) -> Result<(), StringError> {
    if narrow {
        if value > 0xFF {
            return Err(StringError::NarrowUniversalOutOfRange);
        }
        scratch.push(value);
        return Ok(());
    }
    if (0xD800..0xE000).contains(&value) {
        return Err(StringError::BadUniversal(
            "surrogate half is not a code point",
        ));
    }
    if value > 0x10FFFF {
        return Err(StringError::BadUniversal("code point above U+10FFFF"));
    }
    if unit_bytes == 2 && value > 0xFFFF {
        let shifted = value - 0x10000;
        scratch.push(0xD800 + (shifted >> 10));
        scratch.push(0xDC00 + (shifted & 0x3FF));
        return Ok(());
    }
    scratch.push(value);
    Ok(())
}

/// Parse exactly `digits` hex digits at `body[position..]`; the caller
/// guarantees the bound check wording via this single helper so short
/// `\u`/`\U` forms fail instead of reading past the body.
fn parse_fixed_hex(body: &[u8], position: usize, digits: usize) -> Result<u32, StringError> {
    if body.len().saturating_sub(position) < digits {
        return Err(StringError::BadUniversal(
            "truncated universal character name",
        ));
    }
    let mut value: u32 = 0;
    for &byte in body.iter().skip(position).take(digits) {
        let Some(digit) = hex_value(byte) else {
            return Err(StringError::BadUniversal(
                "universal character name needs hex digits",
            ));
        };
        value = value * 16 + digit;
    }
    Ok(value)
}

/// Decode the inner body bytes (between the quotes) for the prefix into
/// code units, then append the single terminating zero. This is the
/// temp-wire scratch: callers drop it on `Err` (revert) and commit it on
/// `Ok`. Escape decoding (simple/octal/greedy-hex/`\u`/`\U`) is the LX11
/// rule set applied here on a chip-local copy.
pub fn decode_string_body(
    prefix: StringPrefix,
    body: &[u8],
    wchar_bytes: u8,
) -> Result<StringRecord, StringError> {
    let (element_type, unit_bytes) = element_for_prefix(prefix, wchar_bytes)?;
    let narrow = unit_bytes == 1;
    let mut scratch: Vec<u32> = Vec::new();
    let mut index = 0;
    while index < body.len() {
        let byte = body[index];
        if byte != b'\\' {
            if narrow || byte < 0x80 {
                scratch.push(u32::from(byte));
                index += 1;
            } else {
                let (code_point, length) = decode_utf8_char(&body[index..])?;
                push_code_point(&mut scratch, unit_bytes, false, code_point)?;
                index += length;
            }
            continue;
        }
        index += 1;
        let Some(&escape) = body.get(index) else {
            return Err(StringError::BadEscape("truncated escape at end of body"));
        };
        match escape {
            b'\'' => {
                scratch.push(0x27);
                index += 1;
            }
            b'"' => {
                scratch.push(0x22);
                index += 1;
            }
            b'?' => {
                scratch.push(0x3F);
                index += 1;
            }
            b'\\' => {
                scratch.push(0x5C);
                index += 1;
            }
            b'a' => {
                scratch.push(0x07);
                index += 1;
            }
            b'b' => {
                scratch.push(0x08);
                index += 1;
            }
            b'f' => {
                scratch.push(0x0C);
                index += 1;
            }
            b'n' => {
                scratch.push(0x0A);
                index += 1;
            }
            b'r' => {
                scratch.push(0x0D);
                index += 1;
            }
            b't' => {
                scratch.push(0x09);
                index += 1;
            }
            b'v' => {
                scratch.push(0x0B);
                index += 1;
            }
            b'x' => {
                // Greedy hex: consume every consecutive hex digit (the
                // classic C `"a\x42c"` pitfall is preserved, not repaired).
                let mut digits = 0usize;
                let mut value: u64 = 0;
                while let Some(&digit_byte) = body.get(index + 1 + digits) {
                    let Some(digit) = hex_value(digit_byte) else {
                        break;
                    };
                    value = value * 16 + u64::from(digit);
                    digits += 1;
                }
                if digits == 0 {
                    return Err(StringError::BadEscape("`\\x` needs at least one hex digit"));
                }
                index += 1 + digits;
                if narrow {
                    if value > 0xFF {
                        return Err(StringError::BadEscape(
                            "hex escape does not fit in a narrow unit",
                        ));
                    }
                    scratch.push(value as u32);
                } else {
                    let value_u32 = u32::try_from(value)
                        .map_err(|_| StringError::BadEscape("hex escape above U+10FFFF"))?;
                    push_code_point(&mut scratch, unit_bytes, false, value_u32).map_err(|_| {
                        StringError::BadEscape("hex escape is not a valid code point")
                    })?;
                }
            }
            b'u' => {
                let value = parse_fixed_hex(body, index + 1, 4)?;
                index += 1 + 4;
                push_code_point(&mut scratch, unit_bytes, narrow, value)?;
            }
            b'U' => {
                let value = parse_fixed_hex(body, index + 1, 8)?;
                index += 1 + 8;
                push_code_point(&mut scratch, unit_bytes, narrow, value)?;
            }
            b'0'..=b'7' => {
                // Octal: at most three digits total, first already seen.
                let mut value: u32 = u32::from(escape - b'0');
                let mut consumed = 1usize;
                while consumed < 3 {
                    match body.get(index + consumed) {
                        Some(b'0'..=b'7') => {
                            value = value * 8 + u32::from(body[index + consumed] - b'0');
                            consumed += 1;
                        }
                        _ => break,
                    }
                }
                index += consumed;
                if narrow {
                    if value > 0xFF {
                        return Err(StringError::BadEscape(
                            "octal escape does not fit in a narrow unit",
                        ));
                    }
                    scratch.push(value);
                } else {
                    push_code_point(&mut scratch, unit_bytes, false, value)?;
                }
            }
            _ => {
                return Err(StringError::BadEscape("unknown escape sequence"));
            }
        }
    }
    scratch.push(0);
    Ok(StringRecord {
        units: scratch,
        element_type,
    })
}

/// Encode units as little-endian bytes of `unit_bytes` width (1, 2, or 4).
/// The terminator is included because it is part of `units`.
pub fn encode_units_le(units: &[u32], unit_bytes: u8) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(units.len() * usize::from(unit_bytes));
    for &unit in units {
        for position in 0..unit_bytes {
            bytes.push(((unit >> (8 * position)) & 0xFF) as u8);
        }
    }
    bytes
}

/// Narrow projection for the string-literal computation.
#[derive(Clone, Debug)]
pub struct LxStringInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Committed C token under decode (output-side back-link target).
    pub token: TokenId,
    /// Committed PP spelling bytes (`prefix"body"`, escapes preserved).
    pub spelling: Vec<u8>,
    /// Committed token span for diagnostics.
    pub span: SpanId,
    /// `literals` arena count at dispatch (single-append prediction base).
    pub literals_allocated: u32,
    /// Frozen `wchar_t` size in bytes (expected 4; never the host width).
    pub wchar_bytes: u8,
    /// Whether the frozen wide encoding is UTF-32 (expected true).
    pub wchar_utf32: bool,
}

/// Build the narrow projection for one dispatched task.
///
/// Decode input is the committed T03 PP spelling/kind (DOC-10 resolution);
/// the C-token relation is the output-side publish-time back-link. A
/// non-`StringLiteral` PP token is explicit `Unsupported`, never a
/// fabricated string.
pub fn project_lx_string_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<LxStringInput, DiagnosticDraft> {
    let record =
        bus.arenas.tasks.get(task).map_err(|_| {
            protocol_fault(format!("string decode of unknown task {}", task.index()))
        })?;
    if record.kind != LX13_TASK_KIND {
        return Err(protocol_fault(format!(
            "string decode task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 2 {
        return Err(protocol_fault(format!(
            "string decode task {} payload must be exactly [Token, PpToken], got {} refs",
            task.index(),
            record.payload.refs.len()
        )));
    }
    let (Some(token), Some(pp_token)) = (
        match record.payload.refs[0] {
            RecordRef::Token(id) => Some(id),
            _ => None,
        },
        match record.payload.refs[1] {
            RecordRef::PpToken(id) => Some(id),
            _ => None,
        },
    ) else {
        return Err(protocol_fault(format!(
            "string decode task {} payload must be exactly [Token, PpToken]",
            task.index()
        )));
    };
    let token_body = bus.arenas.tokens.get(token).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("string decode reads missing token {}", token.index()),
        )
    })?;
    let pp_body = bus.arenas.pp_tokens.get(pp_token).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("string decode reads missing pp-token {}", pp_token.index()),
        )
    })?;
    if pp_body.kind != PpTokenKind::StringLiteral {
        return Err(DiagnosticDraft::unsupported(
            "string decode reads a non-string-literal pp-token",
        ));
    }
    let target = bus.config().target();
    let wchar_bytes = target.scalar(ScalarKind::WCharT).size;
    let wchar_utf32 = matches!(target.wchar_encoding(), WideCharEncoding::Utf32);
    Ok(LxStringInput {
        task,
        state: record.state.clone(),
        token,
        spelling: pp_body.spelling.clone(),
        span: token_body.span,
        literals_allocated: bus.arenas.literals.allocated(),
        wchar_bytes,
        wchar_utf32,
    })
}

/// Build the single `Fail` proposal for an invalid string literal body.
fn string_fail(task: TaskId, span: SpanId, error: StringError) -> Vec<Proposal> {
    vec![fail(
        task,
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("invalid string literal: {error}"),
        )
        .with_span(span),
    )]
}

/// The T04 string-literal worker (LX13).
pub struct LxStringChip;

impl Worker for LxStringChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: LX13_CHIP,
            chip_name: "LxStringChip",
            group: TaskGroup::LEX,
            task_kinds: vec![LX13_TASK_KIND],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Config, "target"),
                FieldPath::new(StoreId::Lex, "tokens"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Lex, "literals"),
            ],
            writes: vec![FieldPath::new(StoreId::Lex, "literals")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            // Acceptance fixture lands in `compiler/tests/c33_string.rs`.
            tests: vec!["compiler/tests/c33_string.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_lx_string_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl LxStringChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &LxStringInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("string decode task {} is not running", input.task.index()),
                ),
            )];
        }
        if !input.wchar_utf32 {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Config, 1),
                    "string decode needs the frozen utf32 wchar_t encoding",
                )
                .with_span(input.span),
            )];
        }
        let (prefix, quoted) = split_string_prefix(&input.spelling);
        if quoted.len() < 2 || !quoted.starts_with(b"\"") || !quoted.ends_with(b"\"") {
            return string_fail(input.task, input.span, StringError::Unterminated);
        }
        let body = &quoted[1..quoted.len() - 1];
        let decoded = match decode_string_body(prefix, body, input.wchar_bytes) {
            Ok(decoded) => decoded,
            Err(error) => return string_fail(input.task, input.span, error),
        };
        let (_, unit_bytes) = match element_for_prefix(prefix, input.wchar_bytes) {
            Ok(mapping) => mapping,
            Err(error) => return string_fail(input.task, input.span, error),
        };
        let value = encode_units_le(&decoded.units, unit_bytes);
        let predicted = LiteralId::from_index(input.literals_allocated);
        vec![
            Proposal::AppendRecords {
                task: input.task,
                batch: AppendBatch {
                    records: vec![RecordDraft {
                        family: crate::ids::RecordFamily::Literal,
                        index: DraftRef(0),
                    }],
                    bodies: vec![G1DraftBody::Literal(LiteralRecord {
                        // Output-side publish-time back-link (DOC-10): the
                        // decode input was the committed PP spelling, and the
                        // C-token relation resolves here at commit time.
                        token: Some(input.token),
                        kind: LiteralKind::String,
                        // D4: `0` is the explicit non-numeric marker, never a
                        // base; the numeric domain `{2, 8, 10, 16}` has no
                        // string member until co-freeze.
                        radix: 0,
                        suffix: LiteralSuffix::None,
                        // Little-endian unit bytes, terminator included;
                        // `units.len() == value.len() / unit_bytes`.
                        value,
                        negative: false,
                        spelling: input.spelling.clone(),
                        // D5: explicit placeholder, never a type. Downstream
                        // stages must not consume it until the string
                        // candidate vocabulary is co-frozen.
                        candidate_type: Lx08CandidateType::Int,
                    })],
                },
            },
            Proposal::Complete {
                task: input.task,
                value: ResultValue::Record(RecordRef::Literal(predicted)),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(prefix: StringPrefix, body: &[u8], wchar_bytes: u8) -> StringRecord {
        decode_string_body(prefix, body, wchar_bytes).expect("valid test body")
    }

    #[test]
    fn empty_string_is_a_single_nul() {
        let record = decode(StringPrefix::None, b"", 4);
        assert_eq!(record.units, vec![0]);
        assert_eq!(record.element_type, StringElementType::Char);
    }

    #[test]
    fn embedded_nul_is_preserved_with_length() {
        // `"a\0b"`: units are [a, 0, b, terminator], length 4 total.
        let record = decode(StringPrefix::None, b"a\\0b", 4);
        assert_eq!(record.units, vec![0x61, 0x00, 0x62, 0x00]);
        assert_eq!(record.units.len(), 4);
    }

    #[test]
    fn units_length_includes_terminator() {
        let record = decode(StringPrefix::None, b"ab", 4);
        assert_eq!(record.units, vec![0x61, 0x62, 0x00]);
        let value = encode_units_le(&record.units, 1);
        assert_eq!(value, vec![0x61, 0x62, 0x00]);
    }

    #[test]
    fn simple_and_octal_escapes() {
        let record = decode(StringPrefix::None, b"\\n\\t\\101", 4);
        assert_eq!(record.units, vec![0x0A, 0x09, 0x41, 0x00]);
    }

    #[test]
    fn greedy_hex_consumes_all_digits() {
        // `\x42` then literal `c` would need a split; greedy takes 0x42c.
        let record = decode(StringPrefix::None, b"\\x41", 4);
        assert_eq!(record.units, vec![0x41, 0x00]);
        assert!(decode_string_body(StringPrefix::None, b"\\x", 4).is_err());
    }

    #[test]
    fn hex_out_of_narrow_range_fails() {
        assert!(decode_string_body(StringPrefix::None, b"\\x100", 4).is_err());
    }

    #[test]
    fn universal_names_decode_and_validate() {
        let record = decode(StringPrefix::Char32, b"\\u00E9\\U0001F600", 4);
        assert_eq!(record.units, vec![0xE9, 0x1F600, 0x00]);
        assert!(decode_string_body(StringPrefix::Char32, b"\\uD800", 4).is_err());
        assert!(decode_string_body(StringPrefix::Char32, b"\\U00111000", 4).is_err());
        assert!(decode_string_body(StringPrefix::Char32, b"\\u00E", 4).is_err());
    }

    #[test]
    fn char16_non_bmp_becomes_surrogate_pair() {
        let record = decode(StringPrefix::Char16, b"\\U0001F600", 4);
        assert_eq!(record.units, vec![0xD83D, 0xDE00, 0x00]);
    }

    #[test]
    fn unknown_escape_and_truncation_fail() {
        assert!(decode_string_body(StringPrefix::None, b"\\q", 4).is_err());
        assert!(decode_string_body(StringPrefix::None, b"abc\\", 4).is_err());
    }

    #[test]
    fn prefix_split_prefers_u8_over_u() {
        let (prefix, rest) = split_string_prefix(b"u8\"ab\"");
        assert_eq!(prefix, StringPrefix::U8);
        assert_eq!(rest, b"\"ab\"");
        let (prefix, _) = split_string_prefix(b"u\"ab\"");
        assert_eq!(prefix, StringPrefix::Char16);
        let (prefix, _) = split_string_prefix(b"\"ab\"");
        assert_eq!(prefix, StringPrefix::None);
    }

    #[test]
    fn wide_units_use_configured_width() {
        let narrow16 = decode(StringPrefix::Wide, b"A", 2);
        assert_eq!(narrow16.units, vec![0x41, 0x00]);
        assert_eq!(
            encode_units_le(&narrow16.units, 2),
            vec![0x41, 0x00, 0x00, 0x00]
        );
        assert!(decode_string_body(StringPrefix::Wide, b"A", 3).is_err());
    }
}
