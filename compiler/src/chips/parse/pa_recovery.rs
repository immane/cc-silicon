// ============================================================================
// chips/parse/pa_recovery.rs — T05 PA38 recovery worker (M1 slice).
//
// Frozen registration (integrator-owned): `PA38_TASK_KIND` aliases
//   `TaskKind::PARSE_RECOVERY` (`PARSE` local 26, `parse.recovery`),
//   `PA38_CHIP` is `crate::manifest::PA38_CHIP` (`ChipId(51)`), the
//   kind-registry row lives in `TaskKindRegistry::pa_recovery_slice()`,
//   the stage row is `(PARSE_RECOVERY, 2)`, the routed layer is 2,
//   and there is no store-owner allowlist row (Ack-only).
//
// Task header (per docs/tasks/TASK_TEMPLATE.md §1):
//
// ID / Name / Group: PA38 ParseRecoveryChip / PARSE (frozen `/36`:
//   `PARSE` local 26, `PA38_CHIP = ChipId(51)`; see the frozen
//   registration below).
// Contract version/hash: T05 frozen; T01 `/36` current. This file claims the
//   `parse.recovery` row only (PA38 in docs/tasks/T05_PARSE_CHIPS.md).
// Owned files: ONLY this file
//   (`compiler/src/chips/parse/pa_recovery.rs`). Never edits `mod.rs`,
//   bus/task/ids/manifest schemas, or registrations (see
//   docs/tasks/PARALLEL_EXECUTION.md §2).
// Task kind / payload fields / result tag: frozen `parse.recovery`
//   (`PARSE` local 26, [`pa38_task_kind`]); payload = committed
//   `RecordRef::Token` refs in source order forming the unconsumed suffix
//   starting at the fault cursor (`tokens[0]` is the fault site; no EOF
//   elision — EOF terminates the scan when present); result = `Ack` (no
//   next-cursor carrier: OB-30 open, caller holds the cursor, exactly like
//   the sibling PA16/PA20/PA22/PA28/PA32 Ack-only slices).
// Category / backend_class / phase / deterministic: Emulable / CpuReference
//   (non-TARGET group rule) / Propagation / true.
// Allowed dialects/targets: M1 Part A only (explicit delimiter sync over
//   the committed token window); every other recovery form is explicit
//   `Unsupported` (never a pass).
// Reads: `tasks.active.{id,kind,payload,state,owner}` (foundation),
//   `lex.tokens` (`/11`), `pp.tokens` (`/11`, punctuator ground truth),
//   `names.entries` (`/11`). Pure query: no bus mutation; workers emit
//   `Proposal`s only.
// Writes: NONE (Ack-only). No `AppendRecords`, no `StorePatch`, no
//   `Enqueue`, no `AwaitChildren`. The [`RecoveredCursor`] rows below copy
//   the resumed-cursor shape chip-locally for documentation/future split
//   only; they are NOT committed.
// Dependencies: T04 committed tokens + interned spellings + committed PP
//   spellings (producer facts only; never calls another chip).
// Preconditions: dispatched task is `Running` with the PA38 candidate
//   kind; payload carries `Token` refs only; referenced tokens are
//   committed; `tokens[0]` is the fault site.
// Transition states: Running → Complete(Ack) on a grounded sync (the
//   [`RecoveredCursor`] is documented for the wiring layer, not committed);
//   Running → Fail (`Unsupported` group) when the window holds no sync
//   token or the fault sits at EOF; Running → Fail (`Task` group code 4)
//   on not-running/missing-record protocol faults.
// Algorithm obligations: forward scan from the fault site with explicit
//   `(paren, bracket, brace)` depth counters; sync set is exactly `;`
//   (consumed), `)` / `}` / `{`-stop at zero depth (not consumed), and
//   `EOF` (see [`recover_cursor`]). Every `Ok` path satisfies the finite
//   advance guarantee `0 < index <= tokens.len()`.
// Invariants: deterministic; no I/O; no cross-chip calls; no global state;
//   ZST worker; `handle()` = project-then-compute; all paths yield
//   proposals; no `panic!`/`unwrap!`/`expect` on any path (missing records
//   are loud `Task/4` faults).
// Error codes / recovery: `Unsupported/1` for unrecoverable windows
//   (explicit rejection, stays in the denominator); `Task/4` for protocol
//   faults; `Protocol/1` for unknown-task/wrong-kind scaffolding.
// Required fixtures: `compiler/tests/c36_recovery.rs` (recovery slice
//   acceptance); chip-local `#[cfg(test)]` unit tests below pin the pure
//   core only.
// Integration acceptance: FROZEN `/36` (kind registration in
//   `TaskKindRegistry::pa_recovery_slice()`, stage-2 row in
//   `STAGE_ASSIGNMENT`, routed layer 2, no allowlist row —
//   Ack-only; cursor carrier, parent linkage, child-task await cleanup,
//   scope balance open).
// Known unsupported: multi-fault windows (one task recovers one fault),
//   heuristic/keyword sync (sync is delimiter-grounded only), recovery
//   across the caller-provided window edge (caller-bounded; see DEFECT-4).
//
// Frozen registration (integrator-owned): [`pa38_task_kind`] aliases
//   `TaskKind::PARSE_RECOVERY` (`PARSE` local 26, `parse.recovery`) —
//   the second free local after `PARSE_UNARY` (local 24), taken after
//   `PARSE_DECL_FINISH` (local 25); [`PA38_CHIP`] is `ChipId(51)`, the
//   next free ID after `PA14_CHIP` (`ChipId(50)`). Rows live in
//   `TaskKindRegistry::pa_recovery_slice()`, `STAGE_ASSIGNMENT`, and the
//   routed layer 2; this file holds no store-owner allowlist row
//   (Ack-only, zero writes).
//
// M1 boundary (see T05 item G): recovery certifies syntax resumption only;
//   no AST node is appended here (the TU slice owns `parse.nodes`), no
//   scope is entered/exited here (T06 owns it), and no literal value is
//   interpreted (T07/T08-owned).
//
// DEFECTs on ambiguity (reported, not silently resolved):
//   DEFECT-4 (windowing): the caller bounds the suffix window (normally
//     the current declaration/statement window, never the whole TU by
//     default). The chip never scans past the provided suffix; the
//     `{`-stop rule additionally refuses to enter a following function
//     body, but skipping a whole following *header* inside an over-wide
//     window is the caller's bounding fault, reported here and deferred
//     to the wiring layer.
//   DEFECT-5 (scope/await): dropping the failed frame's continuation and
//     balancing block scopes (`symbol_type.scope_exit`) is wiring-layer /
//     T06-integration work. This chip emits no `AwaitChildren`/`Enqueue`
//     so it creates no dangling child; it documents the recovered index
//     and completes `Ack`.
//   DEFECT-6 (ambiguity): delimiter identity uses committed PP spelling
//     bytes, never kind-only matching (a `Punctuator` whose spelling is
//     missing/unknown never syncs); spellings are preserved verbatim —
//     nothing is interned, synthesized, or normalized here.
// ============================================================================

use crate::bus::TokenKind;
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{ChipId, NameId, RecordRef, TaskId, TokenId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PA38_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};
use std::collections::BTreeMap;

/// Frozen task kind served by the PA38 worker.
///
/// Aliases `TaskKind::PARSE_RECOVERY` (`PARSE` local 26,
/// `parse.recovery`).
pub const PA38_TASK_KIND: TaskKind = TaskKind::PARSE_RECOVERY;

/// Candidate chip ID for PA38 parse recovery.
///
/// `ChipId(51)` — the next free ID after `PA14_CHIP` (`ChipId(50)`).
/// Frozen `/36` as `PA38_CHIP` (see `manifest.rs`); kept as the
/// documented next-free value.
pub const PA38_CANDIDATE_CHIP: ChipId = ChipId(51);

/// Candidate `PARSE` local code for `parse.recovery`.
///
/// Local 26 — the second free local after `PARSE_UNARY` (local 24),
/// taken after `PARSE_DECL_FINISH` (local 25). Frozen `/36`; see
/// [`pa38_task_kind`].
pub const PA38_LOCAL: u16 = 26;

/// Task kind for PA38 parse recovery (`parse.recovery`).
///
/// Returns the frozen [`PA38_TASK_KIND`] canonical. Registration in
/// `TaskKindRegistry::pa_recovery_slice()` is integrator-owned and lives
/// in `task.rs`, not here.
pub fn pa38_task_kind() -> TaskKind {
    PA38_TASK_KIND
}

/// How the scan resynchronized: the grounded delimiter kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncKind {
    /// `;` at zero depth (consumed: resume after it).
    Semicolon,
    /// `)` with no open paren (not consumed: caller owns the paren).
    CloseParen,
    /// `}` with no open brace (not consumed: caller owns the brace).
    CloseBrace,
    /// `{` at zero depth (not consumed: refuses to enter the next body;
    /// the anti-swallow stop, see DEFECT-4).
    OpenBrace,
    /// Committed `Eof` token (not consumed: resume at end of input).
    Eof,
}

/// Chip-local resumed-cursor shape for one recovery.
///
/// Positions are indices into the task's own token suffix (`tokens[0]` is
/// the fault site); links are plain indices rather than committed IDs.
/// Shapes are built by the pure path and documented — they are NOT
/// appended to any arena by this Ack-only slice (the caller holds the
/// cursor; OB-30 open).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecoveredCursor {
    /// Resumption index into the suffix (`0 < index <= tokens.len()`).
    ///
    /// `index == tokens.len()` means "resume at EOF" (only for
    /// [`SyncKind::Eof`]); otherwise `tokens[index]` is the next
    /// unconsumed token.
    pub index: usize,
    /// Grounded delimiter that terminated the scan.
    pub sync: SyncKind,
    /// Whether the sync token itself is consumed (`true` only for `;`
    /// and for a zero-position closer/open-brace forced advance).
    pub consumed_sync: bool,
}

/// Why a token suffix admits no recovery step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryError {
    /// A well-formed (or malformed) window outside the M1 recovery scope.
    Unsupported(&'static str),
}

impl RecoveryError {
    /// Map to the structured failure diagnostic for this rejection.
    pub fn diagnostic(&self) -> DiagnosticDraft {
        match self {
            Self::Unsupported(message) => DiagnosticDraft::unsupported(*message),
        }
    }
}

/// One projected token: kind plus resolved spellings.
///
/// Identifier/keyword spellings resolve through the intern table
/// (`spelling`); punctuator/number spellings resolve through the committed
/// PP token (`pp_spelling`), because committed C punctuator records carry
/// no interned name. Delimiter identity always uses
/// [`ProjectedRecoveryToken::effective_spelling`]; a missing backing
/// record leaves both sides empty, which matches no delimiter and is
/// skipped rather than guessed.
#[derive(Clone, Debug)]
pub struct ProjectedRecoveryToken {
    /// C-token kind.
    pub kind: TokenKind,
    /// Interned name, if any.
    pub name: Option<NameId>,
    /// Interned spelling bytes for named tokens.
    pub spelling: Vec<u8>,
    /// Committed PP spelling bytes (delimiter ground truth).
    pub pp_spelling: Vec<u8>,
}

impl ProjectedRecoveryToken {
    /// Effective spelling bytes: intern bytes for named tokens, committed
    /// PP bytes otherwise. The projector resolves which side applies; the
    /// pure core only compares bytes.
    pub fn effective_spelling(&self) -> &[u8] {
        match self.kind {
            TokenKind::Identifier | TokenKind::Keyword => &self.spelling,
            _ => &self.pp_spelling,
        }
    }
}

/// Narrow projection for the PA38 recovery computation.
#[derive(Clone, Debug)]
pub struct PaRecoveryInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token IDs in source order: the unconsumed suffix starting
    /// at the fault cursor (`tokens[0]` is the fault site; M1 requires at
    /// least one).
    pub tokens: Vec<TokenId>,
    /// Present tokens by ID.
    pub bodies: BTreeMap<TokenId, ProjectedRecoveryToken>,
}

/// Build the narrow PA38 projection for one dispatched task.
///
/// Payload convention (candidate): exactly the recovery-window token refs
/// in source order, starting at the fault cursor. A wrong kind, a
/// non-token ref, or an unknown task is a caller protocol fault;
/// emptiness/content is `compute`'s job so unrecoverable windows surface
/// as `Unsupported`, never as silent passes.
pub fn project_pa_recovery_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaRecoveryInput, DiagnosticDraft> {
    let record =
        bus.arenas.tasks.get(task).map_err(|_| {
            protocol_fault(format!("recovery parse of unknown task {}", task.index()))
        })?;
    if record.kind != pa38_task_kind() {
        return Err(protocol_fault(format!(
            "recovery task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    let mut tokens = Vec::new();
    let mut bodies = BTreeMap::new();
    for reference in &record.payload.refs {
        let RecordRef::Token(id) = reference else {
            return Err(protocol_fault(format!(
                "recovery task {} payload must carry tokens only",
                task.index()
            )));
        };
        tokens.push(*id);
        if let Ok(body) = bus.arenas.tokens.get(*id) {
            let spelling = body
                .name
                .and_then(|name| bus.intern.get(name).ok().map(|bytes| bytes.to_vec()))
                .unwrap_or_default();
            let pp_spelling = bus
                .arenas
                .pp_tokens
                .get(body.pp_token)
                .map(|pp| pp.spelling.clone())
                .unwrap_or_default();
            bodies.insert(
                *id,
                ProjectedRecoveryToken {
                    kind: body.kind,
                    name: body.name,
                    spelling,
                    pp_spelling,
                },
            );
        }
    }
    Ok(PaRecoveryInput {
        task,
        state: record.state.clone(),
        tokens,
        bodies,
    })
}

/// The T05 PA38 parse-recovery worker (M1 slice: delimiter sync + `Ack`).
pub struct PaRecoveryChip;

impl Worker for PaRecoveryChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PA38_CHIP,
            chip_name: "PaRecoveryChip",
            group: TaskGroup::PARSE,
            task_kinds: vec![pa38_task_kind()],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Lex, "tokens"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Names, "entries"),
            ],
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c36_recovery.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pa_recovery_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl PaRecoveryChip {
    /// Pure PA38 computation over the narrow projection (no bus access).
    ///
    /// A grounded sync completes `Ack` with the [`RecoveredCursor`]
    /// documented (not committed) for the wiring layer; an unrecoverable
    /// window or protocol fault is an explicit fail. Exactly one
    /// diagnostic per task: the original fault's diagnostic stays owned by
    /// the failing caller, so one error surfaces as two diagnostics total
    /// (caller fault + this recovery outcome), never as a merged message.
    pub fn compute(&self, input: &PaRecoveryInput) -> Vec<Proposal> {
        match recover_cursor(input) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(diagnostic) => vec![fail(input.task, diagnostic)],
        }
    }
}

/// Pure M1 recovery scan: synchronize the fault suffix to an explicit
/// delimiter and report the resumption index. No bus access; no
/// shared-schema invention; no spelling interning or synthesis.
///
/// The suffix starts at the fault site (`tokens[0]`). The scan tracks
/// explicit `(paren, bracket, brace)` depth counters over
/// spelling-grounded delimiters and stops at the first:
///
/// - `;` with all depths zero → consume it (`index = pos + 1`);
/// - `)` with `paren == 0`, or `}` with all depths zero → stop without
///   consuming (`index = pos`), so the owning frame keeps its closer;
/// - `{` with all depths zero → stop without consuming (`index = pos`),
///   refusing to enter a following function body (DEFECT-4 anti-swallow);
/// - committed `Eof` → resume at end of input (`index = pos`).
///
/// Finite advance guarantee: every `Ok` path satisfies
/// `0 < index <= tokens.len()`. The only zero-advance shapes (fault site
/// itself is a closer/open-brace, or `pos == 0`) consume exactly that one
/// token (`index = 1`); a fault already at `Eof`, an empty window, or a
/// window with no sync token fails loudly instead of spinning or
/// fabricating success.
pub fn recover_cursor(input: &PaRecoveryInput) -> Result<RecoveredCursor, DiagnosticDraft> {
    if input.state != TaskState::Running {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("recovery task {} is not running", input.task.index()),
        ));
    }
    if input.tokens.is_empty() {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!(
                "recovery task {} has an empty window: no fault site to advance from",
                input.task.index()
            ),
        ));
    }
    for id in &input.tokens {
        if !input.bodies.contains_key(id) {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("parse reads missing token {}", id.index()),
            ));
        }
    }
    // Fault already at EOF: no advance is possible; the caller must fail
    // the TU rather than retry recovery on the same cursor forever.
    if input.bodies[&input.tokens[0]].kind == TokenKind::Eof {
        return Err(RecoveryError::Unsupported(
            "non-recoverable fault: fault site is already at EOF, no advance possible",
        )
        .diagnostic());
    }
    let mut paren: usize = 0;
    let mut bracket: usize = 0;
    let mut brace: usize = 0;
    for (position, id) in input.tokens.iter().enumerate() {
        let projected = &input.bodies[id];
        if projected.kind == TokenKind::Eof {
            return Ok(RecoveredCursor {
                index: position,
                sync: SyncKind::Eof,
                consumed_sync: false,
            });
        }
        if projected.kind != TokenKind::Punctuator {
            continue;
        }
        let spelling = projected.effective_spelling();
        let at_zero = paren == 0 && bracket == 0 && brace == 0;
        if spelling == b";" {
            if at_zero {
                return Ok(RecoveredCursor {
                    index: position.saturating_add(1),
                    sync: SyncKind::Semicolon,
                    consumed_sync: true,
                });
            }
            continue;
        }
        if spelling == b"(" {
            paren = paren.saturating_add(1);
            continue;
        }
        if spelling == b"[" {
            bracket = bracket.saturating_add(1);
            continue;
        }
        if spelling == b"{" {
            if at_zero {
                // Do not enter the next body: stop here (anti-swallow).
                // A fault sitting exactly on the brace still advances one.
                return Ok(RecoveredCursor {
                    index: position.max(1).min(input.tokens.len()),
                    sync: SyncKind::OpenBrace,
                    consumed_sync: position == 0,
                });
            }
            brace = brace.saturating_add(1);
            continue;
        }
        if spelling == b")" {
            if paren > 0 {
                paren = paren.saturating_sub(1);
            } else {
                return Ok(RecoveredCursor {
                    index: position.max(1).min(input.tokens.len()),
                    sync: SyncKind::CloseParen,
                    consumed_sync: position == 0,
                });
            }
            continue;
        }
        if spelling == b"]" {
            // `]` is not in the PA38 sync set; it only closes a bracket
            // depth. A stray `]` at zero depth is ignored, never synced.
            if bracket > 0 {
                bracket = bracket.saturating_sub(1);
            }
            continue;
        }
        if spelling == b"}" {
            if brace > 0 {
                brace = brace.saturating_sub(1);
            } else if paren == 0 && bracket == 0 {
                return Ok(RecoveredCursor {
                    index: position.max(1).min(input.tokens.len()),
                    sync: SyncKind::CloseBrace,
                    consumed_sync: position == 0,
                });
            }
            continue;
        }
    }
    Err(RecoveryError::Unsupported(
        "non-recoverable window: no `;`, `)`, `}`, `{`-stop, or EOF in the recovery suffix",
    )
    .diagnostic())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(id: u32, kind: TokenKind, pp: &[u8]) -> (TokenId, ProjectedRecoveryToken) {
        let tid = TokenId::from_index(id);
        (
            tid,
            ProjectedRecoveryToken {
                kind,
                name: None,
                spelling: Vec::new(),
                pp_spelling: pp.to_vec(),
            },
        )
    }

    fn input(specs: &[(TokenKind, &[u8])]) -> PaRecoveryInput {
        let mut tokens = Vec::new();
        let mut bodies = BTreeMap::new();
        for (position, (kind, pp)) in specs.iter().enumerate() {
            let (id, projected) = token(position as u32, *kind, pp);
            tokens.push(id);
            bodies.insert(id, projected);
        }
        PaRecoveryInput {
            task: TaskId::from_index(0),
            state: TaskState::Running,
            tokens,
            bodies,
        }
    }

    #[test]
    fn semicolon_sync_consumes_and_advances() {
        let p = TokenKind::Punctuator;
        let k = TokenKind::Keyword;
        // `bad + ; return` → sync at `;` (index 3), consumed.
        let got = recover_cursor(&input(&[(k, b""), (p, b"+"), (p, b";"), (k, b"")])).unwrap();
        assert_eq!(got.index, 3);
        assert_eq!(got.sync, SyncKind::Semicolon);
        assert!(got.consumed_sync);
    }

    #[test]
    fn nested_semicolon_is_suppressed() {
        let p = TokenKind::Punctuator;
        let i = TokenKind::Integer;
        // `f ( 1 ; 2 ) ;` → first `;` is inside parens; sync at final `;`.
        let specs = [
            (TokenKind::Identifier, b"".as_slice()),
            (p, b"(".as_slice()),
            (i, b"1".as_slice()),
            (p, b";".as_slice()),
            (i, b"2".as_slice()),
            (p, b")".as_slice()),
            (p, b";".as_slice()),
        ];
        let got = recover_cursor(&input(&specs)).unwrap();
        assert_eq!(got.index, 7);
        assert_eq!(got.sync, SyncKind::Semicolon);
    }

    #[test]
    fn close_brace_stops_without_consuming() {
        let p = TokenKind::Punctuator;
        let k = TokenKind::Keyword;
        // `bad } void` → stop at `}` (index 1), not consumed.
        let got = recover_cursor(&input(&[(k, b""), (p, b"}"), (k, b"")])).unwrap();
        assert_eq!(got.index, 1);
        assert_eq!(got.sync, SyncKind::CloseBrace);
        assert!(!got.consumed_sync);
    }

    #[test]
    fn open_brace_refuses_next_body() {
        let p = TokenKind::Punctuator;
        let k = TokenKind::Keyword;
        // `bad { return` → stop at `{` (index 1): the next body is kept.
        let got = recover_cursor(&input(&[(k, b""), (p, b"{"), (k, b"")])).unwrap();
        assert_eq!(got.index, 1);
        assert_eq!(got.sync, SyncKind::OpenBrace);
        assert!(!got.consumed_sync);
    }

    #[test]
    fn fault_on_closer_still_advances_one() {
        let p = TokenKind::Punctuator;
        let got = recover_cursor(&input(&[(p, b"}"), (p, b";")])).unwrap();
        assert_eq!(got.index, 1);
        assert!(got.consumed_sync);
    }

    #[test]
    fn eof_terminates_without_consuming() {
        let p = TokenKind::Punctuator;
        let got = recover_cursor(&input(&[(p, b"+"), (TokenKind::Eof, b"")])).unwrap();
        assert_eq!(got.index, 1);
        assert_eq!(got.sync, SyncKind::Eof);
        assert!(!got.consumed_sync);
    }

    #[test]
    fn fault_at_eof_fails_instead_of_spinning() {
        let err = recover_cursor(&input(&[(TokenKind::Eof, b"")])).unwrap_err();
        assert_eq!(err.code.group, DiagGroup::Unsupported);
    }

    #[test]
    fn window_without_sync_fails() {
        let p = TokenKind::Punctuator;
        let k = TokenKind::Keyword;
        let err = recover_cursor(&input(&[(k, b""), (p, b"+"), (k, b"")])).unwrap_err();
        assert_eq!(err.code.group, DiagGroup::Unsupported);
    }

    #[test]
    fn kind_only_punctuator_without_spelling_never_syncs() {
        // A `Punctuator` with empty PP spelling preserves spelling-grounding:
        // it must not sync as `;`.
        let p = TokenKind::Punctuator;
        let k = TokenKind::Keyword;
        let err = recover_cursor(&input(&[(k, b""), (p, b""), (k, b"")])).unwrap_err();
        assert_eq!(err.code.group, DiagGroup::Unsupported);
    }

    #[test]
    fn stray_bracket_is_not_a_sync() {
        let p = TokenKind::Punctuator;
        let k = TokenKind::Keyword;
        // `]` at zero depth is ignored; the later `;` syncs.
        let got = recover_cursor(&input(&[(k, b""), (p, b"]"), (p, b";")])).unwrap();
        assert_eq!(got.index, 3);
        assert_eq!(got.sync, SyncKind::Semicolon);
    }

    #[test]
    fn close_paren_stops_for_owner() {
        let p = TokenKind::Punctuator;
        let k = TokenKind::Keyword;
        let got = recover_cursor(&input(&[(k, b""), (p, b")"), (p, b";")])).unwrap();
        assert_eq!(got.index, 1);
        assert_eq!(got.sync, SyncKind::CloseParen);
        assert!(!got.consumed_sync);
    }
}
