// ============================================================================
// chips/lx_float_syntax.rs — T04 LX09 float-syntax worker
// (Wave 3 slice 7, `/32`)
//
// Parses one pp-number spelling into `FloatParts` (decimal/hexponent,
// decimal point, float suffix). Frozen registration (integrator-owned):
// `LX09_TASK_KIND` aliases `TaskKind::LEX_FLOAT_SYNTAX` (`LEX` local 19,
// first code after `/11`), `LX09_CHIP` is `crate::manifest::LX09_CHIP`
// (`ChipId(39)`), the kind-registry row lives in
// `TaskKindRegistry::lx_float_slice()`, the stage-2 row in
// `STAGE_ASSIGNMENT`, the routed layer is 2, and the acceptance test is
// `compiler/tests/c32_float.rs`. No allowlist row: the chip is Ack-only
// (zero writes).
//
// Scope: syntax only. A valid spelling completes with `Ack` and emits NO
// `LiteralRecord` — pp-numbers are never treated as valid C constants here.
// The parts are certified through the pure `parse_float_parts` helper
// (same-file, deterministic, no I/O); they reach any future value
// carrier through the spelling, not through a shared struct (see the
// `FloatParts` reconciliation note in `docs/tasks/LX_FLOAT_SLICE.md`:
// each chip keeps its own local copy by the `compose_map` copy
// precedent). Every malformed spelling fails with a typed `Invalid`
// diagnostic; integer-only spellings (LX05–LX08 territory) and GNU
// suffixes (LX15 territory) fail rather than succeeding with fabricated
// parts.
// ============================================================================

use crate::bus::PpTokenKind;
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{RecordRef, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, LX09_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

/// Frozen LX09 task kind (aliases `TaskKind::LEX_FLOAT_SYNTAX`,
/// frozen by the `/32` integrator; the local code is `LEX` 19, first
/// code after `/11`).
pub const LX09_TASK_KIND: TaskKind = TaskKind::LEX_FLOAT_SYNTAX;

/// Float radix carried in `FloatParts`, matching the `LiteralRecord.radix`
/// domain (`10` decimal, `16` hexadecimal).
pub const FLOAT_RADIX_DECIMAL: u8 = 10;
/// Hexadecimal float radix.
pub const FLOAT_RADIX_HEX: u8 = 16;

/// A parsed binary/decimal exponent: optional sign plus decimal digits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FloatExponent {
    /// True for a `-` sign, false for `+`/absent.
    pub negative: bool,
    /// Exponent digits in source order (at least one).
    pub digits: Vec<u8>,
}

/// A C floating suffix: `f`/`F` (float), `l`/`L` (long double), or absent
/// (double). GNU imaginary (`i`/`j`) belongs to LX15 and is rejected here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatSuffix {
    /// No suffix (double).
    None,
    /// `f`/`F` suffix (float).
    F,
    /// `l`/`L` suffix (long double).
    L,
}

/// Lexical float parts: the LX09 output shape (`NumberSpelling → FloatParts`).
/// This is syntax only, never a constant value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FloatParts {
    /// `10` (decimal, `e`/`E` exponent) or `16` (hex, `p`/`P` exponent).
    pub radix: u8,
    /// Whether a decimal point is present (`1.5`, `1.`, `.5`, `0x1.fp+2`).
    pub has_point: bool,
    /// Decimal (`e`/`E`) or binary (`p`/`P`) exponent, when present.
    pub exponent: Option<FloatExponent>,
    /// Floating suffix (`f`/`F`/`l`/`L`), or `None`.
    pub suffix: FloatSuffix,
}

/// Why a pp-number spelling carries no valid float syntax.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FloatSyntaxError {
    /// Empty spelling, a non-float (integer-shaped) spelling, or a spelling
    /// outside the pp-number character set.
    NotFloat,
    /// A decimal `e`/`E` exponent without required digits (`1e`, `1e+`,
    /// `1e+foo`), or trailing garbage after the suffix.
    BadExponent,
    /// A hexadecimal mantissa without the mandatory binary exponent
    /// (`0x1`, `0x1.f`, `0x.`, `0x1p` with no digits).
    MissingBinaryExponent,
    /// A suffix other than `f`/`F`/`l`/`L` (`1.5ll`, `1.0i`, `0x1p+2u`).
    BadSuffix,
}

impl FloatSyntaxError {
    /// The typed `Invalid` diagnostic for this failure.
    pub fn diagnostic(&self) -> DiagnosticDraft {
        invalid(float_syntax_message(self))
    }
}

/// Narrow projection for the float-syntax computation.
#[derive(Clone, Debug)]
pub struct LxFloatSyntaxInput {
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
/// if any, is output-side only; this chip never reads a C `TokenId`.
pub fn project_lx_float_syntax_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<LxFloatSyntaxInput, DiagnosticDraft> {
    let record =
        bus.arenas.tasks.get(task).map_err(|_| {
            protocol_fault(format!("float syntax of unknown task {}", task.index()))
        })?;
    let want = LX09_TASK_KIND;
    if record.kind != want {
        return Err(protocol_fault(format!(
            "float-syntax task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "float-syntax task {} payload must be exactly [PpToken], got {} refs",
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
            "float-syntax task {} payload must be exactly [PpToken]",
            task.index()
        )));
    };
    let pp_body = bus.arenas.pp_tokens.get(pp_token).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("float syntax reads missing pp-token {}", pp_token.index()),
        )
    })?;
    if pp_body.kind != PpTokenKind::PpNumber {
        return Err(invalid("float syntax reads a non-pp-number pp-token"));
    }
    Ok(LxFloatSyntaxInput {
        task,
        state: record.state.clone(),
        spelling: pp_body.spelling.clone(),
    })
}

/// The T04 float-syntax worker (LX09 candidate scope).
pub struct LxFloatSyntaxChip;

impl Worker for LxFloatSyntaxChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: LX09_CHIP,
            chip_name: "LxFloatSyntaxChip",
            group: TaskGroup::LEX,
            task_kinds: vec![LX09_TASK_KIND],
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
            tests: vec!["compiler/tests/c32_float.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_lx_float_syntax_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl LxFloatSyntaxChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// A valid spelling completes with `Ack` and appends nothing: parts are
    /// syntax, not constants. Every malformed spelling fails `Invalid`.
    pub fn compute(&self, input: &LxFloatSyntaxInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("float-syntax task {} is not running", input.task.index()),
                ),
            )];
        }
        match parse_float_parts(&input.spelling) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(error) => vec![fail(input.task, error.diagnostic())],
        }
    }
}

/// Parse a pp-number spelling into float parts.
///
/// Accepts decimal `1e+10`, `1.5`, `1.`, `.5` (exactly the spellings the
/// frozen pp-number shape admits) and hexadecimal `0x1.fp+2`, `0x1p-1`
/// (binary exponent mandatory). Rejects `1e`, `1e+foo`, hex without `p`,
/// integer-only spellings, and non-float suffixes with a typed error.
/// Deterministic over bytes; no I/O.
pub fn parse_float_parts(spelling: &[u8]) -> Result<FloatParts, FloatSyntaxError> {
    if spelling.len() >= 2 && spelling[0] == b'0' && (spelling[1] == b'x' || spelling[1] == b'X') {
        return parse_hex_float_parts(spelling);
    }
    parse_decimal_float_parts(spelling)
}

/// Decimal `NumberSpelling → FloatParts`.
fn parse_decimal_float_parts(spelling: &[u8]) -> Result<FloatParts, FloatSyntaxError> {
    if spelling.is_empty() {
        return Err(FloatSyntaxError::NotFloat);
    }
    let mut cursor = 0usize;
    if spelling[cursor] == b'.' {
        cursor = 1;
        if !is_ascii_digit_at(spelling, cursor) {
            return Err(FloatSyntaxError::NotFloat);
        }
        cursor = consume_digits(spelling, cursor);
        finish_decimal(spelling, cursor, true)
    } else {
        if !spelling[cursor].is_ascii_digit() {
            return Err(FloatSyntaxError::NotFloat);
        }
        cursor = consume_digits(spelling, cursor);
        let mut has_point = false;
        if cursor < spelling.len() && spelling[cursor] == b'.' {
            has_point = true;
            cursor = consume_digits(spelling, cursor + 1);
        }
        finish_decimal(spelling, cursor, has_point)
    }
}

/// Shared decimal tail: optional `e`/`E` exponent, optional suffix, end.
/// A decimal spelling with neither point nor exponent is integer-shaped
/// (LX05–LX08 territory) and rejected here.
fn finish_decimal(
    spelling: &[u8],
    cursor: usize,
    has_point: bool,
) -> Result<FloatParts, FloatSyntaxError> {
    let mut cursor = cursor;
    let mut exponent = None;
    if cursor < spelling.len() && (spelling[cursor] == b'e' || spelling[cursor] == b'E') {
        let (next, parsed) = parse_exponent_digits(spelling, cursor + 1)?;
        cursor = next;
        exponent = Some(parsed);
    }
    if !has_point && exponent.is_none() {
        return Err(FloatSyntaxError::NotFloat);
    }
    let (cursor, suffix) = parse_float_suffix(spelling, cursor)?;
    if cursor != spelling.len() {
        return Err(FloatSyntaxError::BadExponent);
    }
    Ok(FloatParts {
        radix: FLOAT_RADIX_DECIMAL,
        has_point,
        exponent,
        suffix,
    })
}

/// Hexadecimal `NumberSpelling → FloatParts` (`0x`/`0X` prefix already seen).
/// The binary `p`/`P` exponent is mandatory, even when a point is present.
fn parse_hex_float_parts(spelling: &[u8]) -> Result<FloatParts, FloatSyntaxError> {
    let mut cursor = 2usize;
    let int_start = cursor;
    cursor = consume_hex_digits(spelling, cursor);
    let int_digits = cursor - int_start;
    let mut has_point = false;
    let mut frac_digits = 0usize;
    if cursor < spelling.len() && spelling[cursor] == b'.' {
        has_point = true;
        let frac_start = cursor + 1;
        cursor = consume_hex_digits(spelling, frac_start);
        frac_digits = cursor - frac_start;
    }
    if int_digits == 0 && frac_digits == 0 {
        return Err(FloatSyntaxError::NotFloat);
    }
    if cursor >= spelling.len() || (spelling[cursor] != b'p' && spelling[cursor] != b'P') {
        return Err(FloatSyntaxError::MissingBinaryExponent);
    }
    let (next, exponent) = parse_exponent_digits(spelling, cursor + 1).map_err(|_| {
        if cursor + 1 >= spelling.len() {
            FloatSyntaxError::MissingBinaryExponent
        } else {
            FloatSyntaxError::BadExponent
        }
    })?;
    cursor = next;
    let (cursor, suffix) = parse_float_suffix(spelling, cursor)?;
    if cursor != spelling.len() {
        return Err(FloatSyntaxError::MissingBinaryExponent);
    }
    Ok(FloatParts {
        radix: FLOAT_RADIX_HEX,
        has_point,
        exponent: Some(exponent),
        suffix,
    })
}

/// Parse an exponent tail at `cursor` (caller consumed `e`/`E`/`p`/`P`):
/// optional sign, then one or more decimal digits.
fn parse_exponent_digits(
    spelling: &[u8],
    cursor: usize,
) -> Result<(usize, FloatExponent), FloatSyntaxError> {
    let mut cursor = cursor;
    let mut negative = false;
    if cursor < spelling.len() && (spelling[cursor] == b'+' || spelling[cursor] == b'-') {
        negative = spelling[cursor] == b'-';
        cursor += 1;
    }
    let start = cursor;
    cursor = consume_digits(spelling, cursor);
    if cursor == start {
        return Err(FloatSyntaxError::BadExponent);
    }
    Ok((
        cursor,
        FloatExponent {
            negative,
            digits: spelling[start..cursor].to_vec(),
        },
    ))
}

/// Parse one optional `f`/`F`/`l`/`L` suffix at `cursor`. Any other letter
/// (integer suffixes, GNU imaginary) is rejected for LX09.
fn parse_float_suffix(
    spelling: &[u8],
    cursor: usize,
) -> Result<(usize, FloatSuffix), FloatSyntaxError> {
    if cursor >= spelling.len() {
        return Ok((cursor, FloatSuffix::None));
    }
    let suffix = match spelling[cursor] {
        b'f' | b'F' => FloatSuffix::F,
        b'l' | b'L' => FloatSuffix::L,
        _ => {
            if spelling[cursor].is_ascii_alphabetic() || spelling[cursor] == b'_' {
                return Err(FloatSyntaxError::BadSuffix);
            }
            return Err(FloatSyntaxError::BadExponent);
        }
    };
    Ok((cursor + 1, suffix))
}

/// Advance past ASCII decimal digits.
fn consume_digits(spelling: &[u8], cursor: usize) -> usize {
    let mut cursor = cursor;
    while cursor < spelling.len() && spelling[cursor].is_ascii_digit() {
        cursor += 1;
    }
    cursor
}

/// Advance past ASCII hexadecimal digits.
fn consume_hex_digits(spelling: &[u8], cursor: usize) -> usize {
    let mut cursor = cursor;
    while cursor < spelling.len() && spelling[cursor].is_ascii_hexdigit() {
        cursor += 1;
    }
    cursor
}

/// Whether `spelling` carries an ASCII digit at `cursor`.
fn is_ascii_digit_at(spelling: &[u8], cursor: usize) -> bool {
    cursor < spelling.len() && spelling[cursor].is_ascii_digit()
}

/// Stable message for a float-syntax failure (no addresses, no counts).
fn float_syntax_message(error: &FloatSyntaxError) -> &'static str {
    match error {
        FloatSyntaxError::NotFloat => "spelling is not a floating-constant shape",
        FloatSyntaxError::BadExponent => "malformed decimal exponent in number spelling",
        FloatSyntaxError::MissingBinaryExponent => {
            "hexadecimal float spelling needs a binary exponent"
        }
        FloatSyntaxError::BadSuffix => "unsupported suffix in floating spelling",
    }
}

/// Typed `Invalid` failure for malformed float spellings.
fn invalid(message: impl Into<String>) -> DiagnosticDraft {
    DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Task, 4), message)
}
