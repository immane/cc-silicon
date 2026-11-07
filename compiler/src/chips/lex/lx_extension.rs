// ============================================================================
// chips/lx_extension.rs — T04 LX15 literal-extension worker (DRAFT)
//
// Scope (T04_LEX_CHIPS.md): `ExtensionLiteral/mode → Literal`; GNU imaginary
// (`1i`, `1.0i`, `0x1p+2i`) and unknown standard-new suffixes gated per
// corpus switch. Mode allows/rejects; an unknown suffix is `Unsupported` and
// an identifier is NEVER misrecognized as an extension literal.
//
// DRAFT status: task kind (`LEX` local 24), chip ID (`ChipId(58)`), the
// kind-registry row, the stage-2 row, and the store-owner allowlist row are
// integrator-owned and NOT frozen here. Nothing in this file registers
// itself; the integrator wires `mod`, registry, routing, and allowlist at
// freeze time. The GNU gate reads the frozen bus config
// (`bus.config().dialect().is_gnu()`); no new config pin is invented.
//
// Mode-gate mapping (the `-fno-builtin`-shaped opt-out): when the dialect is
// strict ISO (GNU off), extension spellings are rejected as `Unsupported`
// rather than accepted — the same shape as `-fno-builtin` refusing builtin
// recognition, applied here to literal suffixes. Unknown suffixes are
// `Unsupported` in BOTH modes: never a fabricated literal, never silently
// treated as an identifier.
//
// Ack-only: no imaginary-literal value carrier is frozen yet, so a
// mode-allowed GNU imaginary spelling is acknowledged (valid), not published;
// the consumer recomputes the shape through `classify_extension_suffix`
// until the carrier lands (same pattern as LX11's pending `CodeUnits`).
// ============================================================================

use crate::bus::PpTokenKind;
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{ChipId, RecordRef, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath};
use crate::target::Dialect;
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

/// DRAFT LX15 chip ID (`ChipId(58)`, first free code after `SE_FUNC_CHIP`).
/// Integrator-owned; frozen at registration time, not here.
pub const LX15_CHIP: ChipId = ChipId(58);

/// DRAFT LX15 local code (`LEX` owners start new codes at local 24).
/// Integrator-owned; frozen in `TaskKindRegistry` at freeze time, not here.
pub const LX15_LOCAL: u16 = 24;

/// DRAFT LX15 task kind (`LEX` local 24). A function, not a `const`, so no
/// frozen kind is fabricated before the integrator registry freeze.
pub fn lx15_task_kind() -> TaskKind {
    TaskKind::new(TaskGroup::LEX, LX15_LOCAL).expect("LEX local 24 is a valid kind code")
}

/// GNU imaginary suffix letters (`1i`, `1.0I`, `0x1p+2j`, `3J`).
const IMAGINARY_SUFFIXES: &[u8] = b"iIjJ";

/// Whether a byte can start a pp-number (`1…`, `.5…`); anything else with a
/// trailing-letter tail is an identifier, never an extension candidate.
fn is_number_head(spelling: &[u8]) -> bool {
    match spelling {
        [first, ..] if first.is_ascii_digit() => true,
        [b'.', second, ..] if second.is_ascii_digit() => true,
        _ => false,
    }
}

/// Whether a byte may appear inside a pp-number body (C11 6.4.4.1 charset
/// plus `_`; exponent signs are covered because `+`/`-` never extend the
/// trailing-alpha tail scan below — they terminate it).
fn is_pp_number_body(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'.'
}

/// Typed verdict of the extension-suffix gate over one spelling.
///
/// The identifier rule is structural: a spelling whose head is not numeric
/// is `NotExtension` no matter what its tail looks like (`fi`, `j`, `job`
/// stay identifiers). Numeric spellings with a tail the gate does not
/// recognize are `UnknownSuffix`, never silently accepted and never
/// reclassified as identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionVerdict {
    /// No extension suffix present (identifiers, plain numbers, punctuation).
    /// Not this chip's business; a routed LX15 task carrying one is a
    /// loud misroute, never a silent pass.
    NotExtension,
    /// A numeric base with a GNU imaginary suffix (`i`/`I`/`j`/`J`).
    GnuImaginary,
    /// A numeric spelling with an unrecognized trailing-letter tail
    /// (`1q`, `1.5ll`, `1e+foo` from this gate's view; detailed float /
    /// suffix ownership stays with LX05/LX06/LX09).
    UnknownSuffix,
}

/// Pure extension-suffix classification over one spelling (no bus access).
pub fn classify_extension_suffix(spelling: &[u8]) -> ExtensionVerdict {
    if spelling.is_empty() {
        return ExtensionVerdict::UnknownSuffix;
    }
    let last = spelling[spelling.len() - 1];
    if IMAGINARY_SUFFIXES.contains(&last) {
        let base = &spelling[..spelling.len() - 1];
        if !base.is_empty() && is_number_head(base) {
            return ExtensionVerdict::GnuImaginary;
        }
        // `i`, `fi`, `__j`: letter-headed, hence identifiers by construction.
        return ExtensionVerdict::NotExtension;
    }
    if !is_number_head(spelling) {
        return ExtensionVerdict::NotExtension;
    }
    // Numeric head: a trailing letter run this gate does not recognize is an
    // unknown suffix (`123` has no tail → NotExtension; `1q` → Unknown).
    // Hex-aware: after `0x`/`0X`, `[0-9a-fA-F]` are digits, never a suffix
    // (`0x1F` → NotExtension; only `[g-zG-Z_]` can start a hex suffix tail).
    let hex = spelling.starts_with(b"0x") || spelling.starts_with(b"0X");
    let mut tail_start = spelling.len();
    while tail_start > 0 {
        let byte = spelling[tail_start - 1];
        let is_tail = if hex {
            matches!(byte, b'g'..=b'z' | b'G'..=b'Z' | b'_')
        } else {
            byte.is_ascii_alphabetic() || byte == b'_'
        };
        if is_tail {
            tail_start -= 1;
        } else {
            break;
        }
    }
    if tail_start < spelling.len() {
        // Guard: the tail must follow a pp-number body, not e.g. `+`/`-`
        // (`1e+foo` still counts: the detailed verdict belongs to LX09, this
        // gate only refuses to accept it).
        let head = &spelling[..tail_start];
        if !head.is_empty()
            && head
                .iter()
                .all(|byte| is_pp_number_body(*byte) || *byte == b'+' || *byte == b'-')
        {
            return ExtensionVerdict::UnknownSuffix;
        }
    }
    ExtensionVerdict::NotExtension
}

/// Narrow projection for the extension-gate computation.
#[derive(Clone, Debug)]
pub struct LxExtensionInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Committed PP spelling bytes (must be a `PpNumber` spelling).
    pub spelling: Vec<u8>,
    /// Frozen GNU gate: true in a GNU dialect, false in strict ISO.
    pub gnu: bool,
}

/// Build the narrow projection for one dispatched task.
///
/// Decode input is the committed T03 PP spelling/kind (DOC-10 resolution):
/// exactly one `RecordRef::PpToken` of kind `PpNumber`. The mode gate reads
/// the frozen bus config dialect; no new pin is introduced.
pub fn project_lx_extension_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<LxExtensionInput, DiagnosticDraft> {
    let record =
        bus.arenas.tasks.get(task).map_err(|_| {
            protocol_fault(format!("extension gate of unknown task {}", task.index()))
        })?;
    if record.kind != lx15_task_kind() {
        return Err(protocol_fault(format!(
            "extension task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "extension task {} payload must be exactly [PpToken], got {} refs",
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
            "extension task {} payload must be exactly [PpToken]",
            task.index()
        )));
    };
    let pp_body = bus.arenas.pp_tokens.get(pp_token).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("extension gate reads missing pp-token {}", pp_token.index()),
        )
    })?;
    if pp_body.kind != PpTokenKind::PpNumber {
        return Err(DiagnosticDraft::unsupported(
            "extension gate reads a non-pp-number pp-token",
        ));
    }
    Ok(LxExtensionInput {
        task,
        state: record.state.clone(),
        spelling: pp_body.spelling.clone(),
        gnu: bus.config().dialect().is_gnu(),
    })
}

/// The T04 LX15 literal-extension worker (draft).
pub struct LxExtensionChip;

impl Worker for LxExtensionChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: LX15_CHIP,
            chip_name: "LxExtensionChip",
            group: TaskGroup::LEX,
            task_kinds: vec![lx15_task_kind()],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Config, "dialect"),
            ],
            // Ack-only: no imaginary-literal carrier is frozen yet.
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
        let input = match project_lx_extension_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl LxExtensionChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// Success is exactly one `Complete(Ack)`; every other path is exactly
    /// one `Fail`. Identifiers are never claimed: a routed non-extension
    /// spelling fails loudly as a misroute instead of being accepted.
    pub fn compute(&self, input: &LxExtensionInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("extension task {} is not running", input.task.index()),
                ),
            )];
        }
        match classify_extension_suffix(&input.spelling) {
            ExtensionVerdict::NotExtension => vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "extension gate routed a non-extension spelling; identifiers are never extension literals",
                ),
            )],
            ExtensionVerdict::GnuImaginary if input.gnu => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            ExtensionVerdict::GnuImaginary => vec![fail(
                input.task,
                DiagnosticDraft::unsupported(
                    "GNU imaginary literal requires a GNU dialect (strict ISO rejects the extension)",
                ),
            )],
            ExtensionVerdict::UnknownSuffix => vec![fail(
                input.task,
                DiagnosticDraft::unsupported(
                    "unknown literal suffix is outside the frozen literal scope",
                ),
            )],
        }
    }
}

/// Extension-mode gate over a frozen dialect (pure; mirrors the `gnu` pin
/// carried in [`LxExtensionInput`] so tests need no bus).
pub fn extension_allowed(dialect: Dialect) -> bool {
    dialect.is_gnu()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(spelling: &[u8], gnu: bool) -> LxExtensionInput {
        LxExtensionInput {
            task: TaskId::from_index(0),
            state: TaskState::Running,
            spelling: spelling.to_vec(),
            gnu,
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

    fn is_fail(proposals: &[Proposal]) -> bool {
        matches!(&proposals[..], [Proposal::Fail { .. }])
    }

    #[test]
    fn gnu_imaginary_accepted_only_in_gnu_mode() {
        for spelling in [b"1i".as_slice(), b"1.0I", b"0x1p+2j", b"3J", b".5i"] {
            assert_eq!(
                classify_extension_suffix(spelling),
                ExtensionVerdict::GnuImaginary,
                "spelling {spelling:?} must be a GNU imaginary shape"
            );
            assert!(is_ack(&LxExtensionChip.compute(&input(spelling, true))));
            let fail = &LxExtensionChip.compute(&input(spelling, false));
            assert!(is_fail(fail));
            let Proposal::Fail { diagnostic, .. } = &fail[0] else {
                unreachable!()
            };
            assert_eq!(diagnostic.code.group, DiagGroup::Unsupported);
        }
    }

    #[test]
    fn identifiers_are_never_extension_literals() {
        // Trailing `i`/`j` on a letter-headed spelling stays an identifier.
        for spelling in [b"i".as_slice(), b"j", b"fi", b"job", b"main", b"_Imaginary"] {
            assert_eq!(
                classify_extension_suffix(spelling),
                ExtensionVerdict::NotExtension,
                "spelling {spelling:?} must never classify as an extension"
            );
            // Routed anyway, the chip fails loudly instead of claiming it.
            assert!(is_fail(&LxExtensionChip.compute(&input(spelling, true))));
        }
    }

    #[test]
    fn plain_numbers_carry_no_extension() {
        // No tail, or a tail made only of hex digits after `0x`: LX05–LX09
        // scope, never an extension suffix.
        for spelling in [b"123".as_slice(), b"0x1F", b"0XABCDEF", b"1e+3", b".5"] {
            assert_eq!(
                classify_extension_suffix(spelling),
                ExtensionVerdict::NotExtension
            );
        }
    }

    #[test]
    fn unknown_suffix_is_unsupported_in_both_modes() {
        // `1F` is a standard float suffix (LX09's verdict to give); this gate
        // only refuses to accept it — never a literal, never an identifier.
        for spelling in [
            b"1q".as_slice(),
            b"1F",
            b"1.5ll",
            b"0x1p+2u",
            b"12ww",
            b"0x",
        ] {
            assert_eq!(
                classify_extension_suffix(spelling),
                ExtensionVerdict::UnknownSuffix,
                "spelling {spelling:?} must be an unknown extension suffix"
            );
            for gnu in [false, true] {
                let fail = &LxExtensionChip.compute(&input(spelling, gnu));
                assert!(is_fail(fail));
                let Proposal::Fail { diagnostic, .. } = &fail[0] else {
                    unreachable!()
                };
                assert_eq!(diagnostic.code.group, DiagGroup::Unsupported);
            }
        }
    }

    #[test]
    fn empty_spelling_is_unsupported_never_accepted() {
        assert_eq!(
            classify_extension_suffix(b""),
            ExtensionVerdict::UnknownSuffix
        );
    }

    #[test]
    fn non_running_task_fails() {
        let mut bad = input(b"1i", true);
        bad.state = TaskState::Ready;
        assert!(is_fail(&LxExtensionChip.compute(&bad)));
    }

    #[test]
    fn compute_is_deterministic() {
        let first = LxExtensionChip.compute(&input(b"1i", true));
        let second = LxExtensionChip.compute(&input(b"1i", true));
        assert_eq!(first, second);
    }

    #[test]
    fn draft_ids_follow_documented_conventions() {
        assert_eq!(LX15_CHIP, ChipId(58));
        assert_eq!(LX15_LOCAL, 24);
        assert_eq!(lx15_task_kind().group(), TaskGroup::LEX);
        assert_eq!(lx15_task_kind().local(), 24);
        assert!(extension_allowed(Dialect::Gnu11));
        assert!(!extension_allowed(Dialect::C11));
    }
}
