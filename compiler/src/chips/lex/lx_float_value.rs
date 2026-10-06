// ============================================================================
// chips/lx_float_value.rs — T04 LX10 float-value worker
// (Wave 3 slice 7, `/32`)
//
// Committed pp-number spelling → correctly-rounded value check with
// integer arithmetic only (no host floating point). Frozen registration
// (integrator-owned): `LX10_TASK_KIND` aliases
// `TaskKind::LEX_FLOAT_VALUE` (`LEX` local 20, first code after LX09),
// `LX10_CHIP` is `crate::manifest::LX10_CHIP` (`ChipId(40)`), the
// kind-registry row lives in `TaskKindRegistry::lx_float_slice()`, the
// stage-2 row in `STAGE_ASSIGNMENT`, the routed layer is 2, and the
// acceptance test is `compiler/tests/c32_float.rs`. No allowlist row:
// the chip is Ack-only (zero writes).
//
// Scope: a valid spelling completes with `Ack` and appends nothing: the
// bits are certified by the pure `convert_float_parts` core (unit-pinned
// bit patterns), and no `FloatBits` result carrier is frozen yet — the
// same Ack-only position as the LX11 escape draft (D3 there). Range
// outcomes ride as value flags, never as value-level failures (C
// `strtod`-like semantics): overflow completes `Ack` (the pure core
// reports infinity + flags), it never `Fail`s. `float` (binary32) and
// `double` (binary64) are correctly rounded (round-to-nearest-even,
// subnormals, signed zero, overflow → infinity). `long double`
// (`l`/`L`, binary128 per the work order and the target model) is an
// explicit `Unsupported`; binary16/x87-extended are likewise unsupported.
// Malformed spellings fail `Invalid`; integer-shaped spellings (LX05–LX08
// territory) fail `Unsupported`, never a fabricated value.
//
// `FloatParts` reconciliation (see `docs/tasks/LX_FLOAT_SLICE.md`): no
// frozen `FloatParts` record exists, so `FloatParts` here stays a LOCAL
// shape (digit-bearing, for the conversion core), distinct from LX09's
// local syntax-only shape — each chip copies locally by the `compose_map`
// copy precedent and imports nothing from its sibling. The spelling is
// the frozen interchange: LX10 re-reads the committed PP spelling
// (DOC-10 resolution) through its own chip-local spelling→parts parser
// below, which mirrors the LX09 syntax rule case-for-case (same accept
// set, same `Invalid` messages); `c32_float` pins the agreement.
// ============================================================================

use crate::bus::PpTokenKind;
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{RecordRef, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, LX10_CHIP};
use crate::target::FloatFormat;
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

/// Frozen LX10 task kind (aliases `TaskKind::LEX_FLOAT_VALUE`,
/// frozen by the `/32` integrator; the local code is `LEX` 20, first
/// code after LX09).
pub const LX10_TASK_KIND: TaskKind = TaskKind::LEX_FLOAT_VALUE;

/// Lexical floating suffix carried for provenance (local LX09 mirror shape).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FloatSuffix {
    /// No suffix (`double` by C default; the caller selects the format).
    None,
    /// `f`/`F` (`float` by C default).
    Float,
    /// `l`/`L` (`long double` by C default).
    LongDouble,
}

/// Parsed floating literal (LOCAL MIRROR of the unfrozen LX09 output; D1).
///
/// Decimal: value = (-1)^negative * digits * 10^(exponent - frac_len).
/// Hex: value = (-1)^negative * digits * 2^(exponent - 4 * frac_len),
/// where `digits` is `int_digits ++ frac_digits` read in `radix`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FloatParts {
    /// Leading `-` (sign bit, including `-0.0`).
    pub negative: bool,
    /// `10` (decimal) or `16` (hexadecimal).
    pub radix: u8,
    /// Integer-part digit values (`0..radix`).
    pub int_digits: Vec<u8>,
    /// Fraction-part digit values (`0..radix`).
    pub frac_digits: Vec<u8>,
    /// Decimal `e` exponent or binary `p` exponent (already signed).
    pub exponent: i64,
    /// Lexical suffix (provenance only; conversion uses `format`).
    pub suffix: FloatSuffix,
}

/// Correctly-rounded conversion result.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FloatConvertOutcome {
    /// Raw IEEE bits, right-aligned (`u32`-wide for binary32, `u64` for
    /// binary64; upper bits zero).
    pub bits: u64,
    /// Finite magnitude exceeded the format range (result is infinity).
    pub overflow: bool,
    /// Result is zero/subnormal while the exact value is nonzero and
    /// inexact (tiny after rounding + accuracy loss).
    pub underflow: bool,
    /// Exact value is not representable (includes halfway and overflow).
    pub inexact: bool,
}

/// Binary format parameters: precision bits (incl. implicit bit), max
/// unbiased exponent, min normal unbiased exponent.
fn format_params(format: FloatFormat) -> Option<(u32, i128, i128)> {
    match format {
        FloatFormat::IeeeBinary32 => Some((24, 127, -126)),
        FloatFormat::IeeeBinary64 => Some((53, 1023, -1022)),
        FloatFormat::IeeeBinary16 | FloatFormat::IeeeBinary128 | FloatFormat::X87Extended => None,
    }
}

/// Narrow projection for the float-value computation.
#[derive(Clone, Debug)]
pub struct LxFloatValueInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Committed PP spelling bytes.
    pub spelling: Vec<u8>,
}

/// Build the narrow projection for one dispatched task.
///
/// Decode input is the committed T03 PP spelling/kind (DOC-10 resolution):
/// exactly one `RecordRef::PpToken` of kind `PpNumber`. The C-token relation,
/// if any, is output-side only; this chip never reads a C `TokenId`. The
/// target format is selected from the spelling's own suffix
/// (`f`/`F` → binary32, absent → binary64, `l`/`L` → deferred binary128);
/// the target model is not re-read per task.
pub fn project_lx_float_value_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<LxFloatValueInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("float-value of unknown task {}", task.index())))?;
    if record.kind != LX10_TASK_KIND {
        return Err(protocol_fault(format!(
            "float-value task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "float-value task {} payload must be exactly [PpToken], got {} refs",
            task.index(),
            record.payload.refs.len()
        )));
    }
    let pp_token = match record.payload.refs[0] {
        RecordRef::PpToken(id) => Some(id),
        _ => None,
    };
    let Some(pp_token) = pp_token else {
        return Err(protocol_fault(format!(
            "float-value task {} payload must be exactly [PpToken]",
            task.index()
        )));
    };
    let pp_body = bus.arenas.pp_tokens.get(pp_token).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("float value reads missing pp-token {}", pp_token.index()),
        )
    })?;
    if pp_body.kind != PpTokenKind::PpNumber {
        return Err(DiagnosticDraft::unsupported(
            "floating value reads a non-pp-number pp-token",
        ));
    }
    Ok(LxFloatValueInput {
        task,
        state: record.state.clone(),
        spelling: pp_body.spelling.clone(),
    })
}

/// The T04 float-value worker (LX10 slice scope).
pub struct LxFloatValueChip;

impl Worker for LxFloatValueChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: LX10_CHIP,
            chip_name: "LxFloatValueChip",
            group: TaskGroup::LEX,
            task_kinds: vec![LX10_TASK_KIND],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Pp, "tokens"),
            ],
            // No store writes: Ack-only (the FloatBits carrier is unfrozen;
            // bits are certified by the pure core, as with LX11's D3).
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c32_float.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_lx_float_value_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl LxFloatValueChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// A convertible spelling completes with `Ack` and appends nothing.
    /// Range outcomes are NOT failures (overflow → infinity is certified
    /// by the pure core's flags); malformed spellings fail `Invalid`,
    /// non-float spellings and deferred formats fail `Unsupported`.
    pub fn compute(&self, input: &LxFloatValueInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                protocol_fault(format!(
                    "float-value task {} is not running",
                    input.task.index()
                )),
            )];
        }
        let (parts, format) = match spelling_to_value_parts(&input.spelling) {
            Ok(parsed) => parsed,
            Err(error) => return vec![fail(input.task, error.diagnostic())],
        };
        match convert_float_parts(&parts, format) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }
}

/// Why a pp-number spelling carries no convertible float value.
///
/// Mirrors the LX09 syntax taxonomy case-for-case (same accept set, same
/// `Invalid` messages — a deliberate chip-local copy, not an import):
/// `NotFloat` covers empty, integer-shaped, and non-pp-number spellings
/// (LX05–LX08 territory); the exponent/suffix variants cover malformed
/// float shapes. `DeferredFormat` is LX10-only: the `l`/`L` suffix selects
/// binary128, whose correctly-rounded conversion is deferred.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatValueError {
    /// Not a floating-constant shape.
    NotFloat,
    /// A decimal `e`/`E` exponent without required digits, or trailing
    /// garbage after the suffix.
    BadExponent,
    /// A hexadecimal mantissa without the mandatory binary exponent, or
    /// trailing garbage after a hex suffix.
    MissingBinaryExponent,
    /// A suffix other than `f`/`F`/`l`/`L`.
    BadSuffix,
    /// The `l`/`L` suffix (binary128 `long double`, conversion deferred).
    DeferredFormat,
}

impl FloatValueError {
    /// The typed bus failure for this outcome: malformed float shapes are
    /// `Invalid`; non-float spellings and deferred formats are
    /// `Unsupported` (loud, never a fabricated value).
    pub fn diagnostic(self) -> DiagnosticDraft {
        match self {
            Self::NotFloat => {
                DiagnosticDraft::unsupported("spelling is not a floating-constant shape")
            }
            Self::BadExponent => DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "malformed decimal exponent in number spelling",
            ),
            Self::MissingBinaryExponent => DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "hexadecimal float spelling needs a binary exponent",
            ),
            Self::BadSuffix => DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "unsupported suffix in floating spelling",
            ),
            Self::DeferredFormat => DiagnosticDraft::unsupported(
                "LX10 conversion to ieee-binary128 is deferred (binary32/binary64 only)",
            ),
        }
    }
}

/// Chip-local spelling → value-parts parser with suffix-selected format.
///
/// A deliberate copy of the LX09 syntax rule (same accept/reject set),
/// extended to extract digit values and the conversion format. A pp-number
/// spelling never carries a sign, so `negative` is always false here.
/// Deterministic over bytes; no I/O.
pub fn spelling_to_value_parts(
    spelling: &[u8],
) -> Result<(FloatParts, FloatFormat), FloatValueError> {
    if spelling.len() >= 2 && spelling[0] == b'0' && (spelling[1] == b'x' || spelling[1] == b'X') {
        return hex_spelling_to_parts(spelling);
    }
    decimal_spelling_to_parts(spelling)
}

/// Decimal spelling → digit-bearing parts (mirrors LX09's decimal rule).
fn decimal_spelling_to_parts(
    spelling: &[u8],
) -> Result<(FloatParts, FloatFormat), FloatValueError> {
    if spelling.is_empty() {
        return Err(FloatValueError::NotFloat);
    }
    let mut cursor = 0usize;
    let mut int_digits: Vec<u8> = Vec::new();
    let mut frac_digits: Vec<u8> = Vec::new();
    let mut has_point = false;
    if spelling[cursor] == b'.' {
        cursor = 1;
        let start = cursor;
        cursor = consume_digit_values(spelling, cursor, &mut frac_digits);
        if cursor == start {
            return Err(FloatValueError::NotFloat);
        }
        has_point = true;
    } else {
        if !spelling[cursor].is_ascii_digit() {
            return Err(FloatValueError::NotFloat);
        }
        cursor = consume_digit_values(spelling, cursor, &mut int_digits);
        if cursor < spelling.len() && spelling[cursor] == b'.' {
            has_point = true;
            cursor = consume_digit_values(spelling, cursor + 1, &mut frac_digits);
        }
    }
    let mut exponent: i64 = 0;
    let mut has_exponent = false;
    if cursor < spelling.len() && (spelling[cursor] == b'e' || spelling[cursor] == b'E') {
        let (next, value) = parse_saturated_exponent(spelling, cursor + 1)?;
        cursor = next;
        exponent = value;
        has_exponent = true;
    }
    if !has_point && !has_exponent {
        // Neither point nor exponent: integer-shaped (LX05–LX08 territory).
        return Err(FloatValueError::NotFloat);
    }
    let (next, suffix) = parse_value_suffix(spelling, cursor)?;
    if next != spelling.len() {
        return Err(FloatValueError::BadExponent);
    }
    let format = suffix_format(suffix)?;
    Ok((
        FloatParts {
            negative: false,
            radix: 10,
            int_digits,
            frac_digits,
            exponent,
            suffix,
        },
        format,
    ))
}

/// Hexadecimal spelling → digit-bearing parts (`0x`/`0X` prefix already
/// seen; mirrors LX09's hex rule: the binary exponent is mandatory).
fn hex_spelling_to_parts(spelling: &[u8]) -> Result<(FloatParts, FloatFormat), FloatValueError> {
    let mut cursor = 2usize;
    let mut int_digits: Vec<u8> = Vec::new();
    let mut frac_digits: Vec<u8> = Vec::new();
    cursor = consume_hex_digit_values(spelling, cursor, &mut int_digits);
    let int_count = int_digits.len();
    if cursor < spelling.len() && spelling[cursor] == b'.' {
        cursor = consume_hex_digit_values(spelling, cursor + 1, &mut frac_digits);
    }
    if int_count == 0 && frac_digits.is_empty() {
        return Err(FloatValueError::NotFloat);
    }
    if cursor >= spelling.len() || (spelling[cursor] != b'p' && spelling[cursor] != b'P') {
        return Err(FloatValueError::MissingBinaryExponent);
    }
    let (next, exponent) = match parse_saturated_exponent(spelling, cursor + 1) {
        Ok(parsed) => parsed,
        Err(_) => {
            return Err(if cursor + 1 >= spelling.len() {
                FloatValueError::MissingBinaryExponent
            } else {
                FloatValueError::BadExponent
            });
        }
    };
    cursor = next;
    let (next, suffix) = parse_value_suffix(spelling, cursor)?;
    if next != spelling.len() {
        return Err(FloatValueError::MissingBinaryExponent);
    }
    let format = suffix_format(suffix)?;
    Ok((
        FloatParts {
            negative: false,
            radix: 16,
            int_digits,
            frac_digits,
            exponent,
            suffix,
        },
        format,
    ))
}

/// Parse an exponent tail at `cursor` (caller consumed `e`/`E`/`p`/`P`):
/// optional sign, then one or more decimal digits, saturated to `i64`
/// (saturation is range-safe: the ratio builders exit early past their
/// per-format magnitude bounds).
fn parse_saturated_exponent(
    spelling: &[u8],
    cursor: usize,
) -> Result<(usize, i64), FloatValueError> {
    let mut cursor = cursor;
    let mut negative = false;
    if cursor < spelling.len() && (spelling[cursor] == b'+' || spelling[cursor] == b'-') {
        negative = spelling[cursor] == b'-';
        cursor += 1;
    }
    let start = cursor;
    let mut value: i64 = 0;
    while cursor < spelling.len() && spelling[cursor].is_ascii_digit() {
        value = value
            .saturating_mul(10)
            .saturating_add(i64::from(spelling[cursor] - b'0'));
        cursor += 1;
    }
    if cursor == start {
        return Err(FloatValueError::BadExponent);
    }
    Ok((
        cursor,
        if negative {
            value.saturating_neg()
        } else {
            value
        },
    ))
}

/// Parse one optional `f`/`F`/`l`/`L` suffix at `cursor` (mirrors the
/// LX09 suffix rule exactly, including its trailing-garbage contract,
/// which the caller enforces). Any other letter (integer suffixes, GNU
/// imaginary) is rejected.
fn parse_value_suffix(
    spelling: &[u8],
    cursor: usize,
) -> Result<(usize, FloatSuffix), FloatValueError> {
    if cursor >= spelling.len() {
        return Ok((cursor, FloatSuffix::None));
    }
    let suffix = match spelling[cursor] {
        b'f' | b'F' => FloatSuffix::Float,
        b'l' | b'L' => FloatSuffix::LongDouble,
        _ => {
            if spelling[cursor].is_ascii_alphabetic() || spelling[cursor] == b'_' {
                return Err(FloatValueError::BadSuffix);
            }
            return Err(FloatValueError::BadExponent);
        }
    };
    Ok((cursor + 1, suffix))
}

/// Select the conversion format from a parsed suffix (`f`/`F` →
/// binary32, absent → binary64). The `l`/`L` suffix selects binary128,
/// whose correctly-rounded conversion is deferred (`Unsupported`).
fn suffix_format(suffix: FloatSuffix) -> Result<FloatFormat, FloatValueError> {
    match suffix {
        FloatSuffix::None => Ok(FloatFormat::IeeeBinary64),
        FloatSuffix::Float => Ok(FloatFormat::IeeeBinary32),
        FloatSuffix::LongDouble => Err(FloatValueError::DeferredFormat),
    }
}

/// Advance past ASCII decimal digits, collecting digit values.
fn consume_digit_values(spelling: &[u8], cursor: usize, out: &mut Vec<u8>) -> usize {
    let mut cursor = cursor;
    while cursor < spelling.len() && spelling[cursor].is_ascii_digit() {
        out.push(spelling[cursor] - b'0');
        cursor += 1;
    }
    cursor
}

/// Advance past ASCII hexadecimal digits, collecting digit values.
fn consume_hex_digit_values(spelling: &[u8], cursor: usize, out: &mut Vec<u8>) -> usize {
    let mut cursor = cursor;
    while cursor < spelling.len() && spelling[cursor].is_ascii_hexdigit() {
        out.push(hex_value(spelling[cursor]));
        cursor += 1;
    }
    cursor
}

/// Value of one ASCII hexadecimal digit.
fn hex_value(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => byte - b'A' + 10,
    }
}

/// Exact scaled magnitude: the ratio plus the two early range exits.
///
/// `Ratio(num, den)` carries value `num/den` (both positive, `num` nonzero);
/// `Inf`/`Zero` are the per-format early exits proven by one-sided bounds in
/// `decimal_ratio`/`hex_ratio` (exact, never estimated).
enum Scaled {
    Ratio(Big, Big),
    Inf,
    Zero,
}

/// LX10 semantic core: correctly-rounded `FloatParts` → `FloatBits`.
///
/// Round-to-nearest-even on the exact value using integer arithmetic only.
/// Decimal inputs become an exact ratio `N/D`; hex inputs an exact scaled
/// ratio; the quotient is resolved by word-bounded binary search with an
/// exact remainder comparison, so halfway cases round to even
/// deterministically.
pub fn convert_float_parts(
    parts: &FloatParts,
    format: FloatFormat,
) -> Result<FloatConvertOutcome, DiagnosticDraft> {
    let Some((precision, emax, emin)) = format_params(format) else {
        return Err(DiagnosticDraft::unsupported(format!(
            "LX10 conversion to {} is deferred (binary32/binary64 only)",
            format.name()
        )));
    };
    validate_parts(parts)?;
    let mut digits: Vec<u8> = Vec::with_capacity(
        parts
            .int_digits
            .len()
            .saturating_add(parts.frac_digits.len()),
    );
    digits.extend_from_slice(&parts.int_digits);
    digits.extend_from_slice(&parts.frac_digits);
    let digit_count = digits.len() as i128;
    let frac_len = parts.frac_digits.len() as i128;

    let mut mantissa = Big::from_digits(&digits, parts.radix);
    if mantissa.is_zero() {
        // Signed zero: exponent and digits are irrelevant; exact.
        return Ok(FloatConvertOutcome {
            bits: sign_bit(parts.negative, precision),
            overflow: false,
            underflow: false,
            inexact: false,
        });
    }

    let scaled = if parts.radix == 10 {
        decimal_ratio(
            &mut mantissa,
            parts.exponent as i128 - frac_len,
            digit_count,
            emax,
        )
    } else {
        hex_ratio(
            &mut mantissa,
            parts.exponent as i128 - 4 * frac_len,
            digit_count,
            emax,
        )
    };
    match scaled {
        Scaled::Inf => Ok(FloatConvertOutcome {
            bits: sign_bit(parts.negative, precision) | inf_bits(precision),
            overflow: true,
            underflow: false,
            inexact: true,
        }),
        Scaled::Zero => Ok(FloatConvertOutcome {
            bits: sign_bit(parts.negative, precision),
            overflow: false,
            underflow: true,
            inexact: true,
        }),
        Scaled::Ratio(num, denom) => {
            // Unbiased exponent e with 2^e <= num/denom < 2^(e+1).
            let exp2 = floor_log2_ratio(&num, &denom);
            if exp2 > emax {
                return Ok(FloatConvertOutcome {
                    bits: sign_bit(parts.negative, precision) | inf_bits(precision),
                    overflow: true,
                    underflow: false,
                    inexact: true,
                });
            }
            if exp2 < emin {
                return Ok(round_subnormal(
                    &num,
                    &denom,
                    parts.negative,
                    precision,
                    emin,
                ));
            }
            Ok(round_normal(
                &num,
                &denom,
                parts.negative,
                precision,
                emax,
                exp2,
            ))
        }
    }
}

/// Reject shapes LX09 must never emit (defense in depth; D1 mirror).
fn validate_parts(parts: &FloatParts) -> Result<(), DiagnosticDraft> {
    if parts.radix != 10 && parts.radix != 16 {
        return Err(DiagnosticDraft::unsupported(format!(
            "LX10 radix {} is outside {{10, 16}}",
            parts.radix
        )));
    }
    if parts.int_digits.is_empty() && parts.frac_digits.is_empty() {
        return Err(protocol_fault("LX10 float parts carry no digits"));
    }
    for digit in parts.int_digits.iter().chain(parts.frac_digits.iter()) {
        if *digit >= parts.radix {
            return Err(protocol_fault(format!(
                "LX10 digit {} out of range for radix {}",
                digit, parts.radix
            )));
        }
    }
    Ok(())
}

/// Decimal exact ratio with per-format early range exits.
///
/// `exp = exponent - frac_len`. Bounds (`M >= 1`, `M < 10^digits`):
/// f64 overflows past `exp > 311` and is zero below
/// `exp + digits < -331` (half the least subnormal is ~2.5e-324);
/// f32 past `exp > 41` / below `exp + digits < -61`.
fn decimal_ratio(mantissa: &mut Big, exp: i128, digit_count: i128, emax: i128) -> Scaled {
    let (over_exp, under_mag) = if emax > 200 { (311, -331) } else { (41, -61) };
    if exp > over_exp {
        return Scaled::Inf;
    }
    if exp >= 0 {
        mantissa.mul_pow10(exp as u64);
        Scaled::Ratio(mantissa.clone(), Big::from_u64(1))
    } else {
        if exp + digit_count < under_mag {
            return Scaled::Zero;
        }
        Scaled::Ratio(mantissa.clone(), Big::pow10((-exp) as u64))
    }
}

/// Hex exact ratio with per-format early range exits.
///
/// `shift = exponent - 4 * frac_len` (binary). Bounds: f64 overflows past
/// bit-magnitude 1100 and is zero below -1150 (least subnormal is 2^-1074);
/// f32 past 150 / below -170.
fn hex_ratio(mantissa: &mut Big, shift: i128, digit_count: i128, emax: i128) -> Scaled {
    let (over_shift, under_mag) = if emax > 200 {
        (1100, -1150)
    } else {
        (150, -170)
    };
    if shift + 4 * digit_count < under_mag {
        return Scaled::Zero;
    }
    if shift >= 0 {
        if shift > over_shift {
            return Scaled::Inf;
        }
        mantissa.shl_bits(shift as u64);
        Scaled::Ratio(mantissa.clone(), Big::from_u64(1))
    } else {
        Scaled::Ratio(mantissa.clone(), Big::one_shl((-shift) as u64))
    }
}

/// Round the `exp2 < emin` case on the subnormal grid.
///
/// Grid step is `2^(emin-P+1)`; `q = round(N * 2^(P-1-emin) / D)`.
/// `q == 0` → signed zero; `q == 2^(P-1)` → smallest normal.
fn round_subnormal(
    num: &Big,
    denom: &Big,
    negative: bool,
    precision: u32,
    emin: i128,
) -> FloatConvertOutcome {
    let up = (precision as i128 - 1 - emin) as u64;
    let scaled = num.shl_to_new(up);
    let (quot, rem) = round_to_nearest_even(&scaled, denom, precision);
    let inexact = !rem.is_zero();
    if quot == 0 {
        return FloatConvertOutcome {
            bits: sign_bit(negative, precision),
            overflow: false,
            underflow: true,
            inexact: true,
        };
    }
    if quot == 1u64 << (precision - 1) {
        // Rounded up to the smallest normal: exponent field 1, mantissa 0.
        return FloatConvertOutcome {
            bits: sign_bit(negative, precision) | (1u64 << (precision - 1)),
            overflow: false,
            underflow: true,
            inexact,
        };
    }
    FloatConvertOutcome {
        bits: sign_bit(negative, precision) | quot,
        overflow: false,
        // Exact subnormals (hex-sourced) are not flagged (D3).
        underflow: inexact,
        inexact,
    }
}

/// Round the `emin <= exp2 <= emax` case on the normal grid.
///
/// Unit is `2^(exp2-P+1)`; `q == 2^P` renormalizes (possibly to infinity).
fn round_normal(
    num: &Big,
    denom: &Big,
    negative: bool,
    precision: u32,
    emax: i128,
    exp2: i128,
) -> FloatConvertOutcome {
    let unit = exp2 - precision as i128 + 1;
    let (scaled, den): (Big, Big) = if unit >= 0 {
        (num.clone(), denom.shl_to_new(unit as u64))
    } else {
        (num.shl_to_new((-unit) as u64), denom.clone())
    };
    let (mut quot, rem) = round_to_nearest_even(&scaled, &den, precision);
    let inexact = !rem.is_zero();
    let mut exp = exp2;
    if quot == 1u64 << precision {
        quot = 1u64 << (precision - 1);
        exp += 1;
        if exp > emax {
            return FloatConvertOutcome {
                bits: sign_bit(negative, precision) | inf_bits(precision),
                overflow: true,
                underflow: false,
                inexact: true,
            };
        }
    }
    let field = (exp + emax) as u64;
    FloatConvertOutcome {
        bits: sign_bit(negative, precision)
            | (field << (precision - 1))
            | (quot - (1u64 << (precision - 1))),
        overflow: false,
        underflow: false,
        inexact,
    }
}

/// `q0 = floor(num/den)` known below `2^precision`, rounded to nearest even.
///
/// Returns `(q, rem)` with `rem = num - q0 * den` (pre-round remainder, so
/// `rem.is_zero()` is the exactness test even on halfway round-ups).
fn round_to_nearest_even(num: &Big, den: &Big, precision: u32) -> (u64, Big) {
    let floor = floor_quotient_u64(num, den, precision);
    let back = den.mul_u64_to_new(floor);
    let rem = num.sub_new(&back);
    if rem.is_zero() {
        return (floor, rem);
    }
    let doubled = rem.shl_to_new(1);
    match doubled.cmp(den) {
        std::cmp::Ordering::Greater => (floor + 1, rem),
        std::cmp::Ordering::Equal => (floor + (floor & 1), rem),
        std::cmp::Ordering::Less => (floor, rem),
    }
}

/// Floor of `num/den`, known to be `< 2^precision`, via exact-multiply
/// bisection (`lo` ok, `hi` strict upper bound; deterministic, no estimates).
fn floor_quotient_u64(num: &Big, den: &Big, precision: u32) -> u64 {
    let mut lo: u64 = 0;
    let mut hi: u64 = 1u64 << precision;
    while lo + 1 < hi {
        let mid = lo + (hi - lo) / 2;
        if den.mul_u64_to_new(mid).cmp(num) != std::cmp::Ordering::Greater {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

/// `floor(log2(num/den))` for positive `num`/`den` (single-adjust exact: the
/// bit-length difference is within one of the true quotient logarithm).
fn floor_log2_ratio(num: &Big, den: &Big) -> i128 {
    let diff = num.bit_len() as i128 - den.bit_len() as i128;
    if diff >= 0 {
        if num.cmp(&den.shl_to_new(diff as u64)) != std::cmp::Ordering::Less {
            diff
        } else {
            diff - 1
        }
    } else if num.shl_to_new((-diff) as u64).cmp(den) != std::cmp::Ordering::Less {
        diff
    } else {
        diff - 1
    }
}

/// Sign field positioned for a `precision`-bit significand layout.
fn sign_bit(negative: bool, precision: u32) -> u64 {
    (u64::from(negative)) << (precision - 1 + exp_width(precision))
}

/// Infinity magnitude (all-ones exponent, zero mantissa) for the layout.
fn inf_bits(precision: u32) -> u64 {
    ((1u64 << exp_width(precision)) - 1) << (precision - 1)
}

/// Exponent field width implied by the supported precisions.
fn exp_width(precision: u32) -> u32 {
    match precision {
        24 => 8,
        53 => 11,
        _ => 0,
    }
}

/// Minimal unsigned big integer (little-endian `u32` limbs). Only the ops
/// the rounding core needs: construction, shifts, small/word multiply,
/// subtraction (minuend ≥ subtrahend), comparison, bit length.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Big {
    limbs: Vec<u32>,
}

impl Big {
    fn zero() -> Self {
        Self { limbs: Vec::new() }
    }

    fn from_u64(mut value: u64) -> Self {
        let mut limbs = Vec::new();
        while value > 0 {
            limbs.push((value & 0xffff_ffff) as u32);
            value >>= 32;
        }
        Self { limbs }
    }

    fn one_shl(bits: u64) -> Self {
        Self::from_u64(1).shl_to_new(bits)
    }

    fn is_zero(&self) -> bool {
        self.limbs.iter().all(|limb| *limb == 0)
    }

    fn normalize(&mut self) {
        while self.limbs.last() == Some(&0) {
            self.limbs.pop();
        }
    }

    /// Big-endian digit values in `radix` (10 or 16) to magnitude.
    fn from_digits(digits: &[u8], radix: u8) -> Self {
        let mut value = Self::zero();
        for digit in digits {
            value.mul_small(u32::from(radix));
            value.add_small(u32::from(*digit));
        }
        value
    }

    fn add_small(&mut self, add: u32) {
        let mut carry = u64::from(add);
        let mut index = 0;
        while carry > 0 {
            if index == self.limbs.len() {
                self.limbs.push(0);
            }
            let total = u64::from(self.limbs[index]) + (carry & 0xffff_ffff);
            carry = (total >> 32) + (carry >> 32);
            self.limbs[index] = (total & 0xffff_ffff) as u32;
            index += 1;
        }
    }

    fn mul_small(&mut self, factor: u32) {
        if factor == 0 || self.is_zero() {
            self.limbs.clear();
            return;
        }
        let mut carry: u64 = 0;
        for limb in self.limbs.iter_mut() {
            let total = u64::from(*limb) * u64::from(factor) + carry;
            *limb = (total & 0xffff_ffff) as u32;
            carry = total >> 32;
        }
        while carry > 0 {
            self.limbs.push((carry & 0xffff_ffff) as u32);
            carry >>= 32;
        }
    }

    fn mul_pow10(&mut self, exp: u64) {
        let mut remaining = exp;
        while remaining >= 9 {
            self.mul_small(1_000_000_000);
            remaining -= 9;
        }
        let mut step: u32 = 1;
        for _ in 0..remaining {
            step *= 10;
        }
        if step > 1 {
            self.mul_small(step);
        }
    }

    fn pow10(exp: u64) -> Self {
        let mut value = Self::from_u64(1);
        value.mul_pow10(exp);
        value
    }

    fn shl_to_new(&self, bits: u64) -> Self {
        if self.is_zero() {
            return Self::zero();
        }
        let words = (bits / 32) as usize;
        let rem = (bits % 32) as u32;
        let mut limbs = vec![0; words + self.limbs.len() + usize::from(rem > 0)];
        let mut carry: u64 = 0;
        for (index, limb) in self.limbs.iter().enumerate() {
            let total = (u64::from(*limb) << rem) | carry;
            limbs[words + index] = (total & 0xffff_ffff) as u32;
            carry = total >> 32;
        }
        if carry > 0 {
            limbs[words + self.limbs.len()] = carry as u32;
        }
        let mut value = Self { limbs };
        value.normalize();
        value
    }

    fn shl_bits(&mut self, bits: u64) {
        let shifted = self.shl_to_new(bits);
        self.limbs = shifted.limbs;
    }

    /// `self * factor` (factor fits `u64`; the quotient search stays
    /// word-wide because every rounded quotient fits `precision` bits).
    fn mul_u64_to_new(&self, factor: u64) -> Self {
        if factor == 0 || self.is_zero() {
            return Self::zero();
        }
        let lo = (factor & 0xffff_ffff) as u32;
        let hi = (factor >> 32) as u32;
        let mut limbs = vec![0; self.limbs.len() + 2];
        let mut carry: u64 = 0;
        for (index, limb) in self.limbs.iter().enumerate() {
            let total = u64::from(*limb) * u64::from(lo) + carry;
            limbs[index] = (total & 0xffff_ffff) as u32;
            carry = total >> 32;
        }
        limbs[self.limbs.len()] = (carry & 0xffff_ffff) as u32;
        limbs[self.limbs.len() + 1] = (carry >> 32) as u32;
        if hi > 0 {
            let mut carry: u64 = 0;
            for (index, limb) in self.limbs.iter().enumerate() {
                let total = u64::from(*limb) * u64::from(hi) + u64::from(limbs[index + 1]) + carry;
                limbs[index + 1] = (total & 0xffff_ffff) as u32;
                carry = total >> 32;
            }
            let last = self.limbs.len() + 1;
            let total = u64::from(limbs[last]) + carry;
            limbs[last] = (total & 0xffff_ffff) as u32;
            let mut rest = total >> 32;
            while rest > 0 {
                limbs.push((rest & 0xffff_ffff) as u32);
                rest >>= 32;
            }
        }
        let mut value = Self { limbs };
        value.normalize();
        value
    }

    /// `self - other`; caller guarantees `self >= other`.
    fn sub_new(&self, other: &Self) -> Self {
        let mut limbs = vec![0; self.limbs.len()];
        let mut borrow: i64 = 0;
        for index in 0..self.limbs.len() {
            let mine = i64::from(self.limbs[index]);
            let theirs = if index < other.limbs.len() {
                i64::from(other.limbs[index])
            } else {
                0
            };
            let diff = mine - theirs - borrow;
            if diff < 0 {
                limbs[index] = (diff + (1i64 << 32)) as u32;
                borrow = 1;
            } else {
                limbs[index] = diff as u32;
                borrow = 0;
            }
        }
        debug_assert_eq!(borrow, 0, "Big::sub_new requires self >= other");
        let mut value = Self { limbs };
        value.normalize();
        value
    }

    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let (mut a, mut b) = (self.limbs.len(), other.limbs.len());
        while a > 0 && self.limbs[a - 1] == 0 {
            a -= 1;
        }
        while b > 0 && other.limbs[b - 1] == 0 {
            b -= 1;
        }
        if a != b {
            return a.cmp(&b);
        }
        for index in (0..a).rev() {
            if self.limbs[index] != other.limbs[index] {
                return self.limbs[index].cmp(&other.limbs[index]);
            }
        }
        std::cmp::Ordering::Equal
    }

    fn bit_len(&self) -> u64 {
        let mut len = self.limbs.len();
        while len > 0 && self.limbs[len - 1] == 0 {
            len -= 1;
        }
        if len == 0 {
            return 0;
        }
        let top = self.limbs[len - 1];
        (32 * (len - 1)) as u64 + (32 - top.leading_zeros() as u64)
    }
}

/// Build decimal `FloatParts` for tests (digits must be `0..=9`).
#[cfg(test)]
fn decimal_parts(
    negative: bool,
    int_digits: &[u8],
    frac_digits: &[u8],
    exponent: i64,
) -> FloatParts {
    FloatParts {
        negative,
        radix: 10,
        int_digits: int_digits.to_vec(),
        frac_digits: frac_digits.to_vec(),
        exponent,
        suffix: FloatSuffix::None,
    }
}

/// Build hex `FloatParts` for tests (digit values `0..=15`).
#[cfg(test)]
fn hex_parts(negative: bool, int_digits: &[u8], frac_digits: &[u8], exponent: i64) -> FloatParts {
    FloatParts {
        negative,
        radix: 16,
        int_digits: int_digits.to_vec(),
        frac_digits: frac_digits.to_vec(),
        exponent,
        suffix: FloatSuffix::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::ResultValue;

    fn convert(parts: &FloatParts, format: FloatFormat) -> FloatConvertOutcome {
        convert_float_parts(parts, format).expect("conversion must succeed")
    }

    #[test]
    fn zero_and_negative_zero_double() {
        let zero = convert(
            &decimal_parts(false, &[0], &[], 0),
            FloatFormat::IeeeBinary64,
        );
        assert_eq!(
            zero,
            FloatConvertOutcome {
                bits: 0,
                overflow: false,
                underflow: false,
                inexact: false,
            }
        );
        let neg = convert(
            &decimal_parts(true, &[0], &[0, 0], 5),
            FloatFormat::IeeeBinary64,
        );
        assert_eq!(neg.bits, 0x8000_0000_0000_0000);
        assert!(!neg.inexact);
    }

    #[test]
    fn one_exact_both_formats() {
        let d = convert(
            &decimal_parts(false, &[1], &[], 0),
            FloatFormat::IeeeBinary64,
        );
        assert_eq!(d.bits, 0x3FF0_0000_0000_0000);
        assert!(!d.inexact);
        let f = convert(
            &decimal_parts(false, &[1], &[], 0),
            FloatFormat::IeeeBinary32,
        );
        assert_eq!(f.bits, 0x3F80_0000);
        assert!(!f.inexact);
    }

    #[test]
    fn three_exact_from_hex() {
        // 0x1.8p1 = 3.0.
        let d = convert(&hex_parts(false, &[1], &[8], 1), FloatFormat::IeeeBinary64);
        assert_eq!(d.bits, 0x4008_0000_0000_0000);
        assert!(!d.inexact);
    }

    #[test]
    fn seven_point_75_exact_from_hex() {
        // 0x1.fp+2 = 7.75.
        let d = convert(&hex_parts(false, &[1], &[15], 2), FloatFormat::IeeeBinary64);
        assert_eq!(d.bits, 0x401F_0000_0000_0000);
        assert!(!d.inexact);
    }

    #[test]
    fn halfway_rounds_to_even_float() {
        // 16777217 = 2^24 + 1: halfway between 2^24 and 2^24+2 → even 2^24.
        let even: Vec<u8> = vec![1, 6, 7, 7, 7, 2, 1, 7];
        let out = convert(
            &decimal_parts(false, &even, &[], 0),
            FloatFormat::IeeeBinary32,
        );
        assert_eq!(out.bits, 0x4B80_0000);
        assert!(out.inexact);
        // 16777219: halfway between 16777218 (odd) and 16777220 (even).
        let odd: Vec<u8> = vec![1, 6, 7, 7, 7, 2, 1, 9];
        let out = convert(
            &decimal_parts(false, &odd, &[], 0),
            FloatFormat::IeeeBinary32,
        );
        assert_eq!(out.bits, 0x4B80_0002);
        assert!(out.inexact);
    }

    #[test]
    fn halfway_rounds_to_even_double() {
        // 2^53 + 1 → even 2^53.
        let digits: Vec<u8> = vec![9, 0, 0, 7, 1, 9, 9, 2, 5, 4, 7, 4, 0, 9, 9, 3];
        let out = convert(
            &decimal_parts(false, &digits, &[], 0),
            FloatFormat::IeeeBinary64,
        );
        assert_eq!(out.bits, 0x4340_0000_0000_0000);
        assert!(out.inexact);
    }

    #[test]
    fn tenth_is_inexact_with_known_bits() {
        let f = convert(
            &decimal_parts(false, &[], &[1], 0),
            FloatFormat::IeeeBinary32,
        );
        assert_eq!(f.bits, 0x3DCC_CCCD);
        assert!(f.inexact);
        let d = convert(
            &decimal_parts(false, &[], &[1], 0),
            FloatFormat::IeeeBinary64,
        );
        assert_eq!(d.bits, 0x3FB9_9999_9999_999A);
        assert!(d.inexact);
    }

    #[test]
    fn decimal_exponent_scales_exactly() {
        // 1.5e3 = 1500.
        let d = convert(
            &decimal_parts(false, &[1], &[5], 3),
            FloatFormat::IeeeBinary64,
        );
        assert_eq!(d.bits, 0x4097_7000_0000_0000);
        assert!(!d.inexact);
    }

    #[test]
    fn overflow_yields_infinity_with_flag() {
        let d = convert(
            &decimal_parts(false, &[1], &[], 400),
            FloatFormat::IeeeBinary64,
        );
        assert_eq!(d.bits, 0x7FF0_0000_0000_0000);
        assert!(d.overflow && d.inexact && !d.underflow);
        let f = convert(
            &decimal_parts(true, &[9], &[], 99),
            FloatFormat::IeeeBinary32,
        );
        assert_eq!(f.bits, 0xFF80_0000);
        assert!(f.overflow);
    }

    #[test]
    fn round_up_crossing_emax_overflows() {
        // 2^1024 - 2^970: halfway between max-finite and 2^1024, mantissa
        // even upward → renormalizes across emax to infinity.
        let mut int = vec![7u8];
        int.extend(std::iter::repeat_n(15u8, 12));
        int.push(14);
        int.extend(std::iter::repeat_n(0u8, 13));
        assert_eq!(int.len(), 27);
        let out = convert(&hex_parts(false, &int, &[], 917), FloatFormat::IeeeBinary64);
        assert_eq!(out.bits, 0x7FF0_0000_0000_0000);
        assert!(out.overflow);
    }

    #[test]
    fn smallest_subnormal_exact_from_hex() {
        // 0x0.0000000000001p-1022 = 2^-1074 (13 hex fraction digits).
        let frac: Vec<u8> = {
            let mut frac = vec![0u8; 12];
            frac.push(1);
            frac
        };
        let out = convert(
            &hex_parts(false, &[0], &frac, -1022),
            FloatFormat::IeeeBinary64,
        );
        assert_eq!(out.bits, 1);
        assert!(!out.inexact && !out.underflow);
    }

    #[test]
    fn halfway_below_least_subnormal_rounds_to_zero() {
        // 0x0.8p-1074 = half the least subnormal → even (zero), inexact.
        let out = convert(
            &hex_parts(false, &[0], &[8], -1074),
            FloatFormat::IeeeBinary64,
        );
        assert_eq!(out.bits, 0);
        assert!(out.inexact && out.underflow && !out.overflow);
    }

    #[test]
    fn tiny_decimal_underflows() {
        let out = convert(
            &decimal_parts(false, &[], &[1], -400),
            FloatFormat::IeeeBinary64,
        );
        assert_eq!(out.bits, 0);
        assert!(out.underflow && out.inexact);
    }

    #[test]
    fn deferred_formats_are_unsupported() {
        let parts = decimal_parts(false, &[1], &[], 0);
        for format in [
            FloatFormat::IeeeBinary16,
            FloatFormat::IeeeBinary128,
            FloatFormat::X87Extended,
        ] {
            assert!(convert_float_parts(&parts, format).is_err());
        }
    }

    #[test]
    fn malformed_parts_are_rejected() {
        let bad_radix = FloatParts {
            radix: 8,
            ..decimal_parts(false, &[1], &[], 0)
        };
        assert!(convert_float_parts(&bad_radix, FloatFormat::IeeeBinary64).is_err());
        let bad_digit = decimal_parts(false, &[1, 10], &[], 0);
        assert!(convert_float_parts(&bad_digit, FloatFormat::IeeeBinary64).is_err());
        let no_digits = decimal_parts(false, &[], &[], 0);
        assert!(convert_float_parts(&no_digits, FloatFormat::IeeeBinary64).is_err());
    }

    #[test]
    fn worker_compute_acks_valid_and_fails_loud() {
        let chip = LxFloatValueChip;
        let ack = |spelling: &[u8]| LxFloatValueInput {
            task: TaskId::from_index(0),
            state: TaskState::Running,
            spelling: spelling.to_vec(),
        };
        // Convertible spellings complete `Ack` (range included: overflow
        // is certified, never a value-level failure).
        for spelling in [b"1.5".as_slice(), b"0x1.fp+2", b"1e400", b"2.5f"] {
            let proposals = chip.compute(&ack(spelling));
            assert_eq!(
                proposals,
                vec![Proposal::Complete {
                    task: TaskId::from_index(0),
                    value: ResultValue::Ack,
                }]
            );
        }
        // Malformed spellings fail `Invalid`; integer-shaped spellings and
        // the deferred `l` suffix fail `Unsupported`.
        for spelling in [b"1e".as_slice(), b"0x1", b"42", b"1.5l"] {
            let proposals = chip.compute(&ack(spelling));
            assert_eq!(proposals.len(), 1);
            assert!(matches!(proposals[0], Proposal::Fail { .. }));
        }
        let _ = ResultValue::Ack;
    }

    #[test]
    fn spelling_parser_selects_suffix_format() {
        let (_, f32) = spelling_to_value_parts(b"2.5f").expect("f suffix parses");
        assert_eq!(f32, FloatFormat::IeeeBinary32);
        let (_, f64) = spelling_to_value_parts(b"2.5").expect("no suffix parses");
        assert_eq!(f64, FloatFormat::IeeeBinary64);
        assert_eq!(
            spelling_to_value_parts(b"2.5l"),
            Err(FloatValueError::DeferredFormat)
        );
        assert_eq!(
            spelling_to_value_parts(b"42"),
            Err(FloatValueError::NotFloat)
        );
        assert_eq!(
            spelling_to_value_parts(b"1e"),
            Err(FloatValueError::BadExponent)
        );
    }
}
