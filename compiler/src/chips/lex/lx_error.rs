// ============================================================================
// chips/lx_error.rs — T04 LX18 lexer-error-recovery worker (DRAFT)
//
// Scope (T04_LEX_CHIPS.md): `InvalidToken → Diagnostic/Recovery`. On an
// illegal token the chip reports one typed diagnostic at the error span and
// guarantees finite advance: the lexer always consumes at least one byte and
// never re-reads the same byte. The offending spelling is preserved verbatim
// (no normalization, no truncation), and recovery never panics.
//
// DRAFT status: task kind (`LEX` local 25), chip ID (`ChipId(59)`), the
// kind-registry row, the stage-2 row, and the store-owner allowlist row are
// integrator-owned and NOT frozen here. Nothing in this file registers
// itself; the integrator wires `mod`, registry, routing, and allowlist at
// freeze time.
//
// Byte-window plumbing: raw source bytes live with Host I/O, not on the bus,
// so the error offset and byte window are Host-fed dispatch pins carried
// explicitly in [`LxErrorInput`]. `project_lx_error_input` takes them as
// arguments (the bus contributes kind/state validation only); `handle`
// therefore reports `Unsupported` until the dispatch pin is wired — the pure
// core (`recover_at`, `split_error_stream`) is fully tested without the bus.
//
// Finite-advance contract (checked by tests, relied on by the driver loop):
// `recover_at` returns `None` only at EOF (`offset >= window.len()`, where
// the driver stops); otherwise it returns a recovery with
// `1 <= advance <= remaining`. `split_error_stream` drives this to prove
// arbitrary-byte fuzz input terminates with total advance == input length
// and byte-exact spelling preservation (M1-NEG-04 shape: `@` → one
// unknown-character diagnostic, spelling `@`, advance exactly one byte).
//
// Ack-only: no `RecoveryPlan` result carrier is frozen yet, so a valid
// recovery is acknowledged, not published; the consumer recomputes the plan
// through `recover_at` until the carrier lands (same pattern as LX11's
// pending `CodeUnits`).
// ============================================================================

use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{ChipId, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

/// DRAFT LX18 chip ID (`ChipId(59)`, next free code after LX15).
/// Integrator-owned; frozen at registration time, not here.
pub const LX18_CHIP: ChipId = ChipId(59);

/// DRAFT LX18 local code (`LEX` owners continue at local 25).
/// Integrator-owned; frozen in `TaskKindRegistry` at freeze time, not here.
pub const LX18_LOCAL: u16 = 25;

/// DRAFT LX18 task kind (`LEX` local 25). A function, not a `const`, so no
/// frozen kind is fabricated before the integrator registry freeze.
pub fn lx18_task_kind() -> TaskKind {
    TaskKind::new(TaskGroup::LEX, LX18_LOCAL).expect("LEX local 25 is a valid kind code")
}

/// Typed lexer-error classification for one invalid token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LexErrorKind {
    /// A byte that starts no token (`@`, `$`, `` ` ``, stray control).
    /// Always advances exactly one byte (M1-NEG-04).
    UnknownCharacter,
    /// An opening quote with no closing quote on the scan (`'ab`, `"cd`).
    UnterminatedQuote,
    /// A number-headed run that no literal decoder accepts. The detailed
    /// radix/suffix/exponent verdict stays with LX05/LX06/LX09; this gate
    /// only bounds the error token so recovery stays finite.
    InvalidNumberTail,
}

impl LexErrorKind {
    /// Short stable label for snapshots and messages.
    pub const fn name(self) -> &'static str {
        match self {
            Self::UnknownCharacter => "unknown-character",
            Self::UnterminatedQuote => "unterminated-quote",
            Self::InvalidNumberTail => "invalid-number-tail",
        }
    }

    /// The typed `Invalid` diagnostic for this error (task-family code,
    /// matching the sibling LX chips; numeric codes stay a T01 `/6` item).
    pub fn diagnostic(self, spelling: &[u8]) -> DiagnosticDraft {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!(
                "invalid token ({}): {:?}",
                self.name(),
                String::from_utf8_lossy(spelling)
            ),
        )
    }
}

/// One finite recovery step at an error offset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorRecovery {
    /// Byte offset of the error token within the window.
    pub offset: usize,
    /// Bytes consumed: always `1 <= advance <= window.len() - offset`.
    pub advance: usize,
    /// Error classification.
    pub kind: LexErrorKind,
    /// Original spelling bytes, preserved verbatim.
    pub spelling: Vec<u8>,
}

impl ErrorRecovery {
    /// End offset (exclusive) of the error token.
    pub fn end(&self) -> usize {
        self.offset + self.advance
    }
}

/// Whether a byte can start a pp-number (`1…`, `.5…`).
fn is_number_head(window: &[u8], offset: usize) -> bool {
    match window.get(offset) {
        Some(byte) if byte.is_ascii_digit() => true,
        Some(b'.') => matches!(window.get(offset + 1), Some(next) if next.is_ascii_digit()),
        _ => false,
    }
}

/// Whether a byte continues a pp-number body scan.
fn is_number_body(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'.'
}

/// Compute one finite recovery step at `offset`.
///
/// Returns `None` only at EOF (`offset >= window.len()`), where the driver
/// stops. Otherwise returns a recovery with `advance >= 1` and the exact
/// source bytes preserved. Pure and total: no loops without progress, no
/// panics on any input (empty, binary, unterminated quotes, trailing
/// backslash).
pub fn recover_at(window: &[u8], offset: usize) -> Option<ErrorRecovery> {
    if offset >= window.len() {
        return None;
    }
    let head = window[offset];
    if head == b'\'' || head == b'"' {
        return Some(recover_quote(window, offset, head));
    }
    if is_number_head(window, offset) {
        return Some(recover_number_tail(window, offset));
    }
    // Unknown byte: exactly one byte (M1-NEG-04), spelling preserved.
    Some(ErrorRecovery {
        offset,
        advance: 1,
        kind: LexErrorKind::UnknownCharacter,
        spelling: window[offset..offset + 1].to_vec(),
    })
}

/// Quote-headed recovery: consume to the matching close quote, or to a raw
/// newline / end of window when unterminated. Advance is at least 2
/// (open quote plus one byte of body-or-close) unless the quote is the last
/// byte, in which case it is exactly 1 — always finite, never zero.
fn recover_quote(window: &[u8], offset: usize, quote: u8) -> ErrorRecovery {
    let mut pos = offset + 1;
    while pos < window.len() {
        let byte = window[pos];
        if byte == b'\\' {
            // Skip the escaped byte so an escaped quote does not end the
            // scan; a trailing lone backslash ends the scan (bounded).
            pos += 2;
            continue;
        }
        if byte == quote || byte == b'\n' {
            pos += 1;
            break;
        }
        pos += 1;
    }
    let end = pos.min(window.len());
    let end = end.max(offset + 1);
    ErrorRecovery {
        offset,
        advance: end - offset,
        kind: LexErrorKind::UnterminatedQuote,
        spelling: window[offset..end].to_vec(),
    }
}

/// Number-headed recovery: consume the maximal number-body run. The run is
/// invalid by construction (recovery only runs where decoders refused it);
/// spelling is preserved verbatim and advance equals the run length (>= 1,
//  the head byte matched).
fn recover_number_tail(window: &[u8], offset: usize) -> ErrorRecovery {
    let mut pos = offset;
    while pos < window.len() && is_number_body(window[pos]) {
        pos += 1;
    }
    debug_assert!(
        pos > offset,
        "number head guarantees at least one body byte"
    );
    ErrorRecovery {
        offset,
        advance: pos - offset,
        kind: LexErrorKind::InvalidNumberTail,
        spelling: window[offset..pos].to_vec(),
    }
}

/// Drive recovery over a whole error window (fuzz/no-hang harness).
///
/// Repeatedly applies [`recover_at`] from `0` until EOF. Terminates for all
/// inputs because every step advances by at least one byte, so the step
/// count is at most `window.len()`. The returned spellings concatenate to
/// the input byte-exactly (checked by tests).
pub fn split_error_stream(window: &[u8]) -> Vec<ErrorRecovery> {
    let mut out = Vec::new();
    let mut offset = 0;
    // Bound is structural (each step advances >= 1), not a fuel hack: the
    // `debug_assert` below would fire on any finite-advance violation.
    while let Some(recovery) = recover_at(window, offset) {
        debug_assert!(recovery.advance >= 1);
        debug_assert!(offset + recovery.advance <= window.len());
        offset += recovery.advance;
        out.push(recovery);
    }
    debug_assert_eq!(offset, window.len());
    out
}

/// Narrow projection for the error-recovery computation.
///
/// The byte window and error offset are Host-fed dispatch pins (raw source
/// bytes live with Host I/O, not on the bus) taken as explicit arguments;
/// the bus contributes task kind/state validation only.
#[derive(Clone, Debug)]
pub struct LxErrorInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Host-fed source byte window containing the error offset.
    pub window: Vec<u8>,
    /// Byte offset of the invalid token within `window`.
    pub offset: usize,
}

/// Build the narrow projection for one dispatched task.
///
/// `window`/`offset` arrive from the Host dispatch envelope; until that pin
/// is frozen, `Worker::handle` reports the pending plumbing (it cannot reach
/// this constructor without the Host bytes).
pub fn project_lx_error_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
    window: &[u8],
    offset: usize,
) -> Result<LxErrorInput, DiagnosticDraft> {
    let record =
        bus.arenas.tasks.get(task).map_err(|_| {
            protocol_fault(format!("error recovery of unknown task {}", task.index()))
        })?;
    if record.kind != lx18_task_kind() {
        return Err(protocol_fault(format!(
            "error task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    Ok(LxErrorInput {
        task,
        state: record.state.clone(),
        window: window.to_vec(),
        offset,
    })
}

/// The T04 LX18 lexer-error-recovery worker (draft).
pub struct LxErrorChip;

impl Worker for LxErrorChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: LX18_CHIP,
            chip_name: "LxErrorChip",
            group: TaskGroup::LEX,
            task_kinds: vec![lx18_task_kind()],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
            ],
            // Ack-only: no `RecoveryPlan` carrier is frozen yet.
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec![],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        // The Host byte-window dispatch pin is integrator-owned and unwired:
        // without the error bytes no recovery plan may be fabricated, so the
        // pending plumbing fails loudly instead of guessing. Pure-core
        // coverage lives in `compute` tests below.
        let _ = bus;
        vec![fail(
            task,
            DiagnosticDraft::unsupported(
                "LX18 byte-window dispatch pin is pending integrator wiring; recovery plans are computed via recover_at",
            ),
        )]
    }
}

impl LxErrorChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// A valid error offset completes `Ack` (the plan is recomputed by the
    /// consumer via [`recover_at`] until the carrier lands); an offset
    /// already at EOF or a non-running task is exactly one `Fail`. The
    /// spelling is preserved verbatim in the `{offset, advance}` plan shape
    /// the consumer re-derives — nothing is normalized or dropped.
    pub fn compute(&self, input: &LxErrorInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("error task {} is not running", input.task.index()),
                ),
            )];
        }
        match recover_at(&input.window, input.offset) {
            Some(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            None => vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "error offset {} is at end of a {}-byte window: nothing to recover",
                        input.offset,
                        input.window.len()
                    ),
                ),
            )],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(window: &[u8], offset: usize) -> LxErrorInput {
        LxErrorInput {
            task: TaskId::from_index(0),
            state: TaskState::Running,
            window: window.to_vec(),
            offset,
        }
    }

    fn is_ack(proposals: &[Proposal]) -> bool {
        matches!(
            &proposals[..],
            [Proposal::Complete {
                value: ResultValue::Ack,
                ..
            }]
        )
    }

    #[test]
    fn m1_neg_04_unknown_character_advances_one_byte() {
        // `int main(void){return 2@3;}` shape: `@` is one unknown-character
        // diagnostic, spelling preserved, advance exactly one byte.
        let recovery = recover_at(b"2@3", 1).expect("offset 1 is an error byte");
        assert_eq!(recovery.kind, LexErrorKind::UnknownCharacter);
        assert_eq!(recovery.spelling, b"@");
        assert_eq!(recovery.advance, 1);
        assert_eq!(recovery.end(), 2);
        let diagnostic = recovery.kind.diagnostic(&recovery.spelling);
        assert_eq!(diagnostic.code.group, DiagGroup::Task);
    }

    #[test]
    fn every_single_byte_recovers_with_advance_one() {
        // The unknown-byte arm covers the full byte range except the two
        // quote heads and number heads (handled by their own arms below).
        for byte in 0u8..=255 {
            if byte == b'\'' || byte == b'"' || byte.is_ascii_digit() {
                continue;
            }
            // `.` alone is not a number head; letters are not either.
            let recovery = recover_at(&[byte], 0).expect("any lone byte recovers");
            assert_eq!(recovery.advance, 1, "byte {byte:#04X} must advance one");
            assert_eq!(recovery.spelling, vec![byte], "spelling must be preserved");
        }
    }

    #[test]
    fn unterminated_quote_consumes_to_newline_or_end() {
        let recovery = recover_at(b"'ab\ncd", 0).expect("quote recovers");
        assert_eq!(recovery.kind, LexErrorKind::UnterminatedQuote);
        assert_eq!(recovery.spelling, b"'ab\n");
        assert_eq!(recovery.advance, 4);
        let to_end = recover_at(b"'ab", 0).expect("quote at EOF recovers");
        assert_eq!(to_end.spelling, b"'ab");
        assert_eq!(to_end.advance, 3);
        // Escaped quotes do not end the scan.
        let escaped = recover_at(b"'a\\'b", 0).expect("escaped quote recovers");
        assert_eq!(escaped.spelling, b"'a\\'b");
        // A lone trailing quote still advances (never zero).
        let lone = recover_at(b"x'", 1).expect("lone quote recovers");
        assert_eq!(lone.advance, 1);
    }

    #[test]
    fn number_tail_consumes_the_maximal_run() {
        let recovery = recover_at(b"12qq rest", 0).expect("bad number recovers");
        assert_eq!(recovery.kind, LexErrorKind::InvalidNumberTail);
        assert_eq!(recovery.spelling, b"12qq");
        assert_eq!(recovery.advance, 4);
    }

    #[test]
    fn eof_offset_returns_none_and_fails_loudly() {
        assert_eq!(recover_at(b"abc", 3), None);
        assert_eq!(recover_at(b"", 0), None);
        let proposals = LxErrorChip.compute(&input(b"abc", 3));
        assert_eq!(proposals.len(), 1);
        assert!(matches!(proposals[0], Proposal::Fail { .. }));
    }

    #[test]
    fn valid_offsets_ack() {
        assert!(is_ack(&LxErrorChip.compute(&input(b"a@b", 1))));
        assert!(is_ack(&LxErrorChip.compute(&input(b"'ab", 0))));
        assert!(is_ack(&LxErrorChip.compute(&input(b"12qq", 0))));
    }

    #[test]
    fn non_running_task_fails() {
        let mut bad = input(b"@", 0);
        bad.state = TaskState::Ready;
        assert!(matches!(
            LxErrorChip.compute(&bad)[0],
            Proposal::Fail { .. }
        ));
    }

    #[test]
    fn fuzz_no_hang_and_byte_exact_preservation() {
        // Arbitrary-bytes sweep: split always terminates, total advance is
        // exact, and concatenated spellings reproduce the input byte-for-byte.
        let mut cases: Vec<Vec<u8>> = vec![
            vec![],
            b"@".to_vec(),
            b"'unterminated".to_vec(),
            b"\"also unterminated".to_vec(),
            b"\x00\x01\x02\xff\xfe".to_vec(),
            b"2@3".to_vec(),
            b"''".to_vec(),
            b"'\\".to_vec(),
            b"12qq 34..zz '@' \"q\"".to_vec(),
        ];
        // Exhaustive 2-byte combinations over an adversarial alphabet.
        let alphabet: &[u8] = b"@'\"\\012.ab\n\xff\x00e+-_jzZ";
        for a in alphabet {
            for b in alphabet {
                cases.push(vec![*a, *b]);
            }
        }
        // Deterministic xorshift64* stream for a longer pseudorandom case.
        let mut state: u64 = 0x243F_6A88_85A3_08D3;
        let mut random = vec![0u8; 4096];
        for byte in &mut random {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            *byte = (state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 56) as u8;
        }
        cases.push(random);
        for window in &cases {
            let mut steps = 0usize;
            let mut joined: Vec<u8> = Vec::new();
            let mut offset = 0;
            while let Some(recovery) = recover_at(window, offset) {
                assert!(recovery.advance >= 1, "finite advance violated");
                assert!(offset + recovery.advance <= window.len());
                offset += recovery.advance;
                joined.extend_from_slice(&recovery.spelling);
                steps += 1;
                assert!(steps <= window.len() + 1, "no-hang bound violated");
            }
            assert_eq!(offset, window.len(), "recovery must cover the window");
            assert_eq!(joined, *window, "spelling must be preserved exactly");
            // The batch driver agrees with the manual loop.
            let stream = split_error_stream(window);
            assert_eq!(stream.len(), steps);
        }
    }

    #[test]
    fn compute_is_deterministic() {
        let first = LxErrorChip.compute(&input(b"a@b", 1));
        let second = LxErrorChip.compute(&input(b"a@b", 1));
        assert_eq!(first, second);
    }

    #[test]
    fn draft_ids_follow_documented_conventions() {
        assert_eq!(LX18_CHIP, ChipId(59));
        assert_eq!(LX18_LOCAL, 25);
        assert_eq!(lx18_task_kind().group(), TaskGroup::LEX);
        assert_eq!(lx18_task_kind().local(), 25);
    }
}
