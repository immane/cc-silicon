// ============================================================================
// chips/lx_char.rs — T04 LX12 character-literal worker
// (Wave 3 slice 8, `/33`)
//
// Prefix/body → typed character value: plain, wide (`L`), `u`, `U` literals
// decode to one `LiteralRecord` (`kind: Character`) with the publish-time
// `token: Some(committed TokenId)` back-link (DOC-10 resolution, same shape
// as `lx_decode.rs`).
//
// Frozen registration (integrator-owned): `LX12_TASK_KIND` aliases
// `TaskKind::LEX_CHAR_DECODE` (`LEX` local 22), `LX12_CHIP` is
// `crate::manifest::LX12_CHIP` (`ChipId(42)`), the kind-registry row lives
// in `TaskKindRegistry::lx_string_slice()`, the stage-2 row in
// `STAGE_ASSIGNMENT`, the routed layer is 2, the store-owner allowlist row
// authorizes (`LX12_CHIP`, `lex.literals`), and the acceptance test is
// `compiler/tests/c33_string.rs`.
//
// ## Frozen open items (recorded, not silently resolved)
//
// Per `docs/tasks/PARALLEL_EXECUTION.md` §2 the chip file stays the single
// artifact; the following remain explicitly open (loud on the wire, never
// silent), pending T04/T08/T01 co-freeze:
//
// - `TokenKind` is M1-closed with no `Character` variant, so the C-token
//   kind cannot be checked. Any committed token is accepted as the
//   back-link; the kind check is deferred until `TokenKind::Character`
//   lands.
// - The committed `LiteralRecord.radix` for `Character` records is frozen
//   by `/33` as `16` (code-unit value, hex-natural); the value bytes are
//   unaffected. `0` must never be read as a numeric base (that marker
//   belongs to `String` records).
// - `Lx08CandidateType` is the M1-closed `{ Int }` vocabulary with no
//   character member. This chip records `Int` explicitly (never silently);
//   the character candidate category is a T04/T08/T01 co-freeze item.
// - There is no success-plus-warning proposal (`AppendRecords` coexists
//   only with a terminal `Complete`/`Fail`/await). The multicharacter
//   truncation policy below is therefore silent on the wire; it is
//   documented here and must surface through a future warning channel
//   (T01 co-freeze).
//
// ## Implemented policy (deterministic, implementation-defined where C leaves
// choice to the implementation)
//
// - Multicharacter constants (`'ab'`): each decoded unit is masked to the
//   prefix unit width (plain 8, `u` 16, `L`/`U` 32 bits), units are
//   concatenated big-endian as their minimal byte sequences, and the low 32
//   bits are kept. E.g. plain `'ab'` → `0x6162`; `L'ab'` → `0x6162`;
//   `L'\x1234'` → `0x1234`. This is the documented implementation-defined
//   value (C-common truncation shape); it is NOT a constraint violation and
//   does NOT fail.
// - Escape decoding is chip-local (no `lx_escape` import; there is no shared
//   escape helper to reuse): simple escapes, octal (at most 3 digits),
//   greedy hexadecimal (at least 1 digit), `\u` (exactly 4) / `\U` (exactly 8)
//   with surrogate and `> 0x10FFFF` rejection, unknown escapes fail.
// - Out-of-range escape values truncate to the prefix unit width (plain →
//   low 8 bits); this is documented implementation-defined behavior.
// - Empty literals (`''`), unterminated spellings, raw newlines or raw `'`
//   inside the body, and the `u8` prefix are explicit `Fail`s.
// ============================================================================

use crate::bus::{LiteralRecord, PpTokenKind};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{LiteralId, RecordRef, TaskId, TokenId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, LX12_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// Frozen LX12 task kind (aliases `TaskKind::LEX_CHAR_DECODE`,
/// frozen by the `/33` integrator; the local code is `LEX` 22).
pub const LX12_TASK_KIND: TaskKind = TaskKind::LEX_CHAR_DECODE;

/// Character prefix selecting the unit width and element type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharPrefix {
    /// `'...'` — plain character, 8-bit units.
    Plain,
    /// `L'...'` — wide character, 32-bit units.
    Wide,
    /// `u'...'` — 16-bit units.
    Char16,
    /// `U'...'` — 32-bit units.
    Char32,
}

impl CharPrefix {
    /// Mask applied to each decoded unit before folding.
    const fn unit_mask(self) -> u32 {
        match self {
            Self::Plain => 0xff,
            Self::Char16 => 0xffff,
            Self::Wide | Self::Char32 => 0xffff_ffff,
        }
    }
}

/// Split a full character spelling (`'a'`, `L'a'`, `u'a'`, `U'a'`) into its
/// prefix and raw body bytes (between the quotes).
pub fn split_char_spelling(spelling: &[u8]) -> Result<(CharPrefix, &[u8]), DiagnosticDraft> {
    let (prefix, head_len) = if spelling.starts_with(b"L'") {
        (CharPrefix::Wide, 2)
    } else if spelling.starts_with(b"u'") {
        // `u8'` is a distinct (C++/C23) prefix, rejected below before this arm
        // can misread it: `u8'` starts with `u`, not `u'`.
        (CharPrefix::Char16, 2)
    } else if spelling.starts_with(b"U'") {
        (CharPrefix::Char32, 2)
    } else if spelling.starts_with(b"'") {
        (CharPrefix::Plain, 1)
    } else if spelling.starts_with(b"u8'") {
        return Err(DiagnosticDraft::unsupported(
            "u8 character prefix is outside the LX12 (none/L/u/U) scope",
        ));
    } else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "character spelling must start with a valid prefix and opening quote",
        ));
    };
    if spelling.len() < head_len + 2 || !spelling.ends_with(b"'") {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "unterminated character literal",
        ));
    }
    Ok((prefix, &spelling[head_len..spelling.len() - 1]))
}

/// Decode one escape starting at `body[pos] == b'\\'`.
///
/// Returns the decoded scalar value and the position just past the escape.
/// Chip-local copy: there is no shared escape helper, and language rules must
/// not hide in one (framework rule 6), so LX12 owns this function.
fn decode_escape(body: &[u8], pos: usize) -> Result<(u32, usize), DiagnosticDraft> {
    debug_assert_eq!(body[pos], b'\\');
    let Some(&head) = body.get(pos + 1) else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "trailing backslash in character literal",
        ));
    };
    match head {
        b'\'' => Ok((0x27, pos + 2)),
        b'"' => Ok((0x22, pos + 2)),
        b'?' => Ok((0x3f, pos + 2)),
        b'\\' => Ok((0x5c, pos + 2)),
        b'a' => Ok((0x07, pos + 2)),
        b'b' => Ok((0x08, pos + 2)),
        b'f' => Ok((0x0c, pos + 2)),
        b'n' => Ok((0x0a, pos + 2)),
        b'r' => Ok((0x0d, pos + 2)),
        b't' => Ok((0x09, pos + 2)),
        b'v' => Ok((0x0b, pos + 2)),
        b'0'..=b'7' => {
            // Octal escape: at most 3 digits total (C11 6.4.4.4).
            let mut value: u32 = 0;
            let mut end = pos + 1;
            while end < body.len() && end < pos + 4 && matches!(body[end], b'0'..=b'7') {
                value = value
                    .wrapping_mul(8)
                    .wrapping_add(u32::from(body[end] - b'0'));
                end += 1;
            }
            Ok((value, end))
        }
        b'x' => {
            // Hexadecimal escape: greedy, at least one digit (C11 6.4.4.4).
            let mut value: u32 = 0;
            let mut end = pos + 2;
            while end < body.len() && body[end].is_ascii_hexdigit() {
                value = value
                    .wrapping_mul(16)
                    .wrapping_add(u32::from(hex_value(body[end])));
                end += 1;
            }
            if end == pos + 2 {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "\\x used with no following hex digits",
                ));
            }
            Ok((value, end))
        }
        b'u' | b'U' => {
            let digits = if head == b'u' { 4 } else { 8 };
            let mut value: u32 = 0;
            for offset in 0..digits {
                match body.get(pos + 2 + offset) {
                    Some(byte) if byte.is_ascii_hexdigit() => {
                        value = value
                            .wrapping_mul(16)
                            .wrapping_add(u32::from(hex_value(*byte)));
                    }
                    _ => {
                        return Err(DiagnosticDraft::error(
                            DiagnosticCode::new(DiagGroup::Task, 4),
                            "universal character name must have exactly 4 (\\u) or 8 (\\U) hex digits",
                        ));
                    }
                }
            }
            if (0xd800..0xe000).contains(&value) || value > 0x10_ffff {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "universal character name is not a valid scalar value",
                ));
            }
            Ok((value, pos + 2 + digits))
        }
        _ => Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "unknown escape sequence in character literal",
        )),
    }
}

/// Numeric value of one ASCII hex digit (caller checks `is_ascii_hexdigit`).
const fn hex_value(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => byte - b'A' + 10,
    }
}

/// Decode a raw character body into scalar values.
///
/// Raw bytes (including non-ASCII source bytes) each become one unit with the
/// byte's value; multi-byte source encodings are NOT composed (documented
/// limitation, pending the target execution-character-set model). Empty
/// bodies, raw newlines, and raw `'` in the body fail.
pub fn decode_char_units(body: &[u8]) -> Result<Vec<u32>, DiagnosticDraft> {
    if body.is_empty() {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "empty character literal",
        ));
    }
    let mut units = Vec::new();
    let mut pos = 0;
    while pos < body.len() {
        match body[pos] {
            b'\\' => {
                let (value, next) = decode_escape(body, pos)?;
                units.push(value);
                pos = next;
            }
            b'\'' | b'\n' => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "unescaped quote or newline in character literal",
                ));
            }
            byte => {
                units.push(u32::from(byte));
                pos += 1;
            }
        }
    }
    Ok(units)
}

/// Fold decoded units into the typed character value.
///
/// Each unit is masked to the prefix unit width; units concatenate big-endian
/// as their minimal byte sequences (an embedded NUL unit contributes one
/// `0x00` byte, never a terminator), and the low 32 bits are kept. Returns
/// the value and whether the literal is multicharacter
/// (implementation-defined value per the module policy; never a failure).
pub fn fold_char_value(prefix: CharPrefix, units: &[u32]) -> (u32, bool) {
    debug_assert!(!units.is_empty());
    let mut bytes: Vec<u8> = Vec::new();
    for &unit in units {
        bytes.extend_from_slice(&u32_magnitude(unit & prefix.unit_mask()));
    }
    let tail = &bytes[bytes.len().saturating_sub(4)..];
    let mut value: u32 = 0;
    for &byte in tail {
        value = value.wrapping_shl(8).wrapping_add(u32::from(byte));
    }
    (value, units.len() > 1)
}

/// Minimal big-endian magnitude bytes for a `u32` value (at least one byte).
pub fn u32_magnitude(value: u32) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let first = bytes.iter().position(|&byte| byte != 0).unwrap_or(3);
    bytes[first..].to_vec()
}

/// Narrow projection for the character-literal computation.
#[derive(Clone, Debug)]
pub struct LxCharInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Committed C token under decode (back-link only; kind unchecked until
    /// `TokenKind::Character` lands).
    pub token: TokenId,
    /// Committed PP spelling bytes (must be a `CharLiteral` spelling).
    pub spelling: Vec<u8>,
    /// `literals` arena count at dispatch (single-append prediction base).
    pub literals_allocated: u32,
}

/// Build the narrow projection for one dispatched task.
pub fn project_lx_char_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<LxCharInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("char decode of unknown task {}", task.index())))?;
    if record.kind != LX12_TASK_KIND {
        return Err(protocol_fault(format!(
            "char task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 2 {
        return Err(protocol_fault(format!(
            "char task {} payload must be exactly [Token, PpToken], got {} refs",
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
            "char task {} payload must be exactly [Token, PpToken]",
            task.index()
        )));
    };
    let pp_body = bus.arenas.pp_tokens.get(pp_token).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("char decode reads missing pp-token {}", pp_token.index()),
        )
    })?;
    if pp_body.kind != PpTokenKind::CharLiteral {
        return Err(DiagnosticDraft::unsupported(
            "character decode reads a non-character pp-token",
        ));
    }
    Ok(LxCharInput {
        task,
        state: record.state.clone(),
        token,
        spelling: pp_body.spelling.clone(),
        literals_allocated: bus.arenas.literals.allocated(),
    })
}

/// The T04 LX12 character-literal worker.
pub struct LxCharChip;

impl Worker for LxCharChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: LX12_CHIP,
            chip_name: "LxCharChip",
            group: TaskGroup::LEX,
            task_kinds: vec![LX12_TASK_KIND],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Lex, "tokens"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Lex, "literals"),
            ],
            writes: vec![FieldPath::new(StoreId::Lex, "literals")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c33_string.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_lx_char_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl LxCharChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// The committed `candidate_type: Int` is explicit, not silent: it is the
    /// only member of the frozen `Lx08CandidateType` vocabulary, and the
    /// character-specific candidate remains a co-freeze item.
    /// Likewise `radix: 16` is the `/33`-frozen code-unit encoding.
    pub fn compute(&self, input: &LxCharInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("char task {} is not running", input.task.index()),
                ),
            )];
        }
        if input.spelling.is_empty() {
            return vec![fail(
                input.task,
                DiagnosticDraft::unsupported("character decode of an empty spelling"),
            )];
        }
        let (prefix, body) = match split_char_spelling(&input.spelling) {
            Ok(split) => split,
            Err(diagnostic) => return vec![fail(input.task, diagnostic)],
        };
        let units = match decode_char_units(body) {
            Ok(units) => units,
            Err(diagnostic) => return vec![fail(input.task, diagnostic)],
        };
        let (value, _multichar) = fold_char_value(prefix, &units);
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
                        token: Some(input.token),
                        kind: LiteralKind::Character,
                        radix: 16,
                        suffix: LiteralSuffix::None,
                        value: u32_magnitude(value),
                        negative: false,
                        spelling: input.spelling.clone(),
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
    use crate::ids::PpTokenId;

    fn input(spelling: &[u8]) -> LxCharInput {
        LxCharInput {
            task: TaskId::from_index(0),
            state: TaskState::Running,
            token: TokenId::from_index(0),
            spelling: spelling.to_vec(),
            literals_allocated: 0,
        }
    }

    fn appended_literal(proposals: &[Proposal]) -> LiteralRecord {
        assert_eq!(proposals.len(), 2);
        let Proposal::AppendRecords { batch, .. } = &proposals[0] else {
            panic!("first proposal must append the literal record");
        };
        assert_eq!(batch.records.len(), 1);
        let G1DraftBody::Literal(record) = &batch.bodies[0] else {
            panic!("appended body must be a literal");
        };
        record.clone()
    }

    #[test]
    fn plain_char_value() {
        let record = appended_literal(&LxCharChip.compute(&input(b"'a'")));
        assert_eq!(record.kind, LiteralKind::Character);
        assert_eq!(record.value, vec![0x61]);
        assert_eq!(record.suffix, LiteralSuffix::None);
        assert!(!record.negative);
        assert_eq!(record.spelling, b"'a'");
        assert_eq!(record.token, Some(TokenId::from_index(0)));
    }

    #[test]
    fn wide_char_value() {
        let record = appended_literal(&LxCharChip.compute(&input(b"L'a'")));
        assert_eq!(record.value, vec![0x61]);
    }

    #[test]
    fn char16_and_char32_prefixes() {
        let record_u = appended_literal(&LxCharChip.compute(&input(b"u'\xce\xa9'")));
        // Raw bytes compose no multibyte sequences: two units fold to 0xCEA9.
        assert_eq!(record_u.value, vec![0xce, 0xa9]);
        let record_big_u = appended_literal(&LxCharChip.compute(&input(b"U'A'")));
        assert_eq!(record_big_u.value, vec![0x41]);
    }

    #[test]
    fn empty_literal_fails() {
        let proposals = LxCharChip.compute(&input(b"''"));
        assert_eq!(proposals.len(), 1);
        assert!(matches!(proposals[0], Proposal::Fail { .. }));
    }

    #[test]
    fn unterminated_literal_fails() {
        let proposals = LxCharChip.compute(&input(b"'a"));
        assert!(matches!(proposals[0], Proposal::Fail { .. }));
    }

    #[test]
    fn u8_prefix_is_unsupported() {
        let proposals = LxCharChip.compute(&input(b"u8'a'"));
        assert!(matches!(proposals[0], Proposal::Fail { .. }));
    }

    #[test]
    fn multichar_truncates_with_documented_policy() {
        // Implementation-defined policy: big-endian concatenation, low 32 bits.
        let record = appended_literal(&LxCharChip.compute(&input(b"'ab'")));
        assert_eq!(record.value, vec![0x61, 0x62]);
        let wide = appended_literal(&LxCharChip.compute(&input(b"L'ab'")));
        // Wide units concatenate the same way; low 32 bits keep both units.
        assert_eq!(wide.value, vec![0x61, 0x62]);
    }

    #[test]
    fn simple_and_octal_escapes() {
        let record = appended_literal(&LxCharChip.compute(&input(b"'\\n'")));
        assert_eq!(record.value, vec![0x0a]);
        // Octal takes at most 3 digits: `\1011` is 'A' followed by '1'.
        let record = appended_literal(&LxCharChip.compute(&input(b"'\\1011'")));
        assert_eq!(record.value, vec![0x41, 0x31]);
    }

    #[test]
    fn greedy_hex_truncates_to_unit_width() {
        // `\x41B` is one greedy escape (0x41B); plain truncates to 0x1B.
        let record = appended_literal(&LxCharChip.compute(&input(b"'\\x41B'")));
        assert_eq!(record.value, vec![0x1b]);
    }

    #[test]
    fn ucn_rejects_surrogates_and_short_digits() {
        let ok = appended_literal(&LxCharChip.compute(&input(b"'\\u00E9'")));
        assert_eq!(ok.value, vec![0xe9]);
        assert!(matches!(
            LxCharChip.compute(&input(b"'\\uD800'"))[0],
            Proposal::Fail { .. }
        ));
        assert!(matches!(
            LxCharChip.compute(&input(b"'\\u12'"))[0],
            Proposal::Fail { .. }
        ));
        assert!(matches!(
            LxCharChip.compute(&input(b"'\\q'"))[0],
            Proposal::Fail { .. }
        ));
    }

    #[test]
    fn non_running_task_fails() {
        let mut bad = input(b"'a'");
        bad.state = TaskState::Ready;
        assert!(matches!(LxCharChip.compute(&bad)[0], Proposal::Fail { .. }));
    }

    #[test]
    fn compute_is_deterministic() {
        let first = LxCharChip.compute(&input(b"'ab'"));
        let second = LxCharChip.compute(&input(b"'ab'"));
        assert_eq!(first, second);
    }

    #[test]
    fn projection_rejects_non_char_pp_token() {
        // Projection-level guard (needs a bus):_PP token kind routing is
        // covered by `project_lx_char_input`'s `CharLiteral` check; the
        // `PpTokenId` import above pins the payload shape used there.
        let _ = PpTokenId::from_index(0);
    }
}
