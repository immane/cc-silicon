// ============================================================================
// chips/parse/pa_decl_spec.rs — T05 PA08/PA10 array + abstract workers (M1).
//
// UNREGISTERED draft (candidate): this file is new-work authoring output and
// claims NO frozen row. The integrator owns `mod.rs`, bus/task/ids/manifest
// schemas, kind registration, stage rows, allowlist rows, and test-suite
// placement (see docs/tasks/PARALLEL_EXECUTION.md §2); needing any of those
// is a DEFECT, never a silent local invention.
//
// Task header (per docs/tasks/TASK_TEMPLATE.md §1):
//
// ID / Name / Group: PA08 ArrayDeclaratorChip + PA10 AbstractDeclaratorChip /
//   PARSE (CANDIDATE: `PARSE` locals 27 (`parse.array_suffix`) and 28
//   (`parse.abstract_declarator`) — the first free locals after
//   `PARSE_RECOVERY` (local 26); single fused candidate chip `ChipId(58)` —
//   the next free ID after `SE_FUNC_CHIP` (`ChipId(57)`). All three values
//   are documented next-free guesses for the integrator to arbitrate; the
//   file never registers them itself).
// Contract version/hash: T05 PENDING; no `/6` freeze claimed by this file.
//   This file claims the PA08 (`Suffix → ArrayDescriptor`) and PA10
//   (`TypeNameCursor → AbstractTree`) T05 rows only.
// Owned files: ONLY this file
//   (`compiler/src/chips/parse/pa_decl_spec.rs`). Never edits `mod.rs`,
//   bus/task/ids/manifest schemas, or registrations.
// Task kind / payload fields / result tag: CANDIDATE `parse.array_suffix`
//   (PA08) / `parse.abstract_declarator` (PA10); payload = the suffix /
//   type-name token refs in source order (PA08 allows the empty slice: an
//   absent suffix is the M1 case; PA10 requires at least one token);
//   result = `Ack` (no cursor carrier: OB-30 open, caller holds the cursor;
//   no committed node links: the `ArrayDescriptor`/`AbstractTree` shapes
//   below are chip-local documentation for the future split, NOT appended).
// Category / backend_class / phase / deterministic: Emulable / CpuReference
//   (non-TARGET group rule) / Propagation / true.
// Allowed dialects/targets: M1 Part A only (no array suffix; the bare `int`
//   type name). Every other shape — `[]`, bound expressions, `static` /
//   qualifiers, `[*]`, VLAs, pointers, parenthesized abstracts, array
//   abstracts, non-`int` specifiers — is explicit `Unsupported` (never a
//   pass; non-M1 stays in the denominator per the wave-dispatch shell rule).
// Reads: `tasks.active.{id,kind,payload,state,owner}` (foundation),
//   `lex.tokens` (`/11`), `pp.tokens` (punctuator ground truth),
//   `names.entries` (`/11`). Pure query: no bus mutation; workers emit
//   `Proposal`s only.
// Writes: NONE (Ack-only). No `AppendRecords`, no `StorePatch`, no `Enqueue`,
//   no `AwaitChildren` (a second `parse.nodes` writer or any cross-group
//   enqueue would need allowlist/registration rows this file must not
//   invent).
// Dependencies: T04 committed tokens + interned spellings + committed PP
//   spellings (producer facts only; never calls another chip). Array bound
//   semantics stay PA15/T08-owned; type interpretation stays T06/T07-owned.
// Preconditions: dispatched task is `Running` with the matching candidate
//   kind; payload carries `Token` refs only; referenced tokens are committed.
// Transition states: Running → Complete(Ack) on the M1 shape (absent array
//   suffix; bare `int` abstract); Running → Fail (`Unsupported` group) on
//   any non-M1 shape; Running → Fail (`Task` group code 4) on
//   not-running/missing-record/unterminated/empty-type-name defects.
// Algorithm obligations: PA08 scans one `[...]` suffix at bracket depth 0
//   and classifies its interior (`[]`, bound expression, `static`,
//   qualifiers, `[*]`) without evaluating it; PA10 accepts the single
//   Keyword `int` token and rejects every operator-bearing or non-`int`
//   type name. Neither guesses across modes nor derives typedef-ness
//   locally (PA04 owns that query).
// Invariants: deterministic; no I/O; no cross-chip calls; no global state;
//   ZST worker; `handle()` = dispatch-then-project-then-compute; all paths
//   yield proposals; no `panic!`/`unwrap!` on any bus path (the two
//   `expect`s below resolve compile-time-known candidate locals and are
//   unreachable for 27/28 — see the resolver docs).
// Error codes / recovery: `Unsupported/1` for non-M1 shapes (explicit
//   rejection, stays in the denominator); `Task/4` for protocol and
//   unterminated/empty defects; `Protocol/1` for unknown-task/wrong-kind
//   scaffolding. No recovery sync is attempted here (PA38 owns it).
// Required fixtures: in-file `#[cfg(test)]` unit tests below (the shared
//   acceptance suite is integrator-owned; the `tests` manifest row points
//   at this file until the integrator repoints it).
// Integration acceptance: NONE claimed here (candidate kinds/chip/stage
//   rows are integrator-owned; cursor carrier, parent linkage, child-task
//   nesting open).
// Known unsupported: every array form (`[]`, constant/variable bounds,
//   `static`, type qualifiers, `[*]`, multi-dimensional suffixes, trailing
//   suffixes), every non-trivial abstract declarator (pointers,
//   parenthesized abstracts such as `int(*)[3]`, array abstracts, function
//   abstracts), every non-`int` specifier in type-name position, and named
//   declarators in abstract position (PA05 territory).
//
// Candidate registration (integrator-owned; NOT registered by this file):
//   `PA08_TASK_KIND_CANDIDATE` (`PARSE` local 27, `parse.array_suffix`),
//   `PA10_TASK_KIND_CANDIDATE` (`PARSE` local 28,
//   `parse.abstract_declarator`), `PA_DECL_SPEC_CANDIDATE_CHIP`
//   (`ChipId(58)`). The single fused chip serves both productions and
//   dispatches on the dispatched task's kind (the production field,
//   mirroring `ContinuationRecord.production`), exactly like the fused
//   PA28/PA32 precedent. If the integrator prefers two chips, the second
//   takes the next free ID after 58.
//
// DEFECTs on ambiguity (reported, not silently resolved):
//   DEFECT-1 (cursor): `next_cursor` has no frozen `ResultValue` carrier
//     (T05 item G.4 OB-30 open). M1 completes `Ack`; cursor advance stays
//     caller-held. No scalar/tuple variant is invented here.
//   DEFECT-2 (nesting): the full design nests suffix/type-name parses via
//     child tasks and continuations with await-all joins (T05 items A–C);
//     M1 validates the fixed shape in one tick with no `Enqueue`,
//     `AwaitChildren`, or `Progress`. Parse-depth accounting
//     (`ParseDepthExceeded`, item D) stays a future integration check.
//   DEFECT-3 (bounds): array bound constant-expression evaluation is
//     PA15/T08-owned. This chip classifies the interior spelling only and
//     never folds it, so VLA-vs-constant is never decided here.
//   DEFECT-4 (scope): abstract type names introduce no scope and register
//     no names here; scope/declare work stays T06-owned.
// ============================================================================

use crate::bus::TokenKind;
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{ChipId, RecordRef, TaskId, TokenId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};
use std::collections::BTreeMap;

/// Candidate `PARSE` local code for the PA08 array-suffix production.
///
/// Local 27 — the first free local after `PARSE_RECOVERY` (local 26).
pub const PA08_CANDIDATE_LOCAL: u16 = 27;

/// Candidate `PARSE` local code for the PA10 abstract-declarator production.
///
/// Local 28 — the next free local after the PA08 candidate.
pub const PA10_CANDIDATE_LOCAL: u16 = 28;

/// Candidate chip ID for the fused PA08/PA10 worker.
///
/// `ChipId(58)` — the next free ID after `SE_FUNC_CHIP` (`ChipId(57)`).
pub const PA_DECL_SPEC_CANDIDATE_CHIP: ChipId = ChipId(58);

/// Candidate task kind for PA08 array-suffix parsing (`parse.array_suffix`).
///
/// `None` only if the local exceeds `LOCAL_MAX` (impossible for 27).
pub const PA08_TASK_KIND_CANDIDATE: Option<TaskKind> =
    TaskKind::new(TaskGroup::PARSE, PA08_CANDIDATE_LOCAL);

/// Candidate task kind for PA10 abstract-declarator parsing
/// (`parse.abstract_declarator`).
///
/// `None` only if the local exceeds `LOCAL_MAX` (impossible for 28).
pub const PA10_TASK_KIND_CANDIDATE: Option<TaskKind> =
    TaskKind::new(TaskGroup::PARSE, PA10_CANDIDATE_LOCAL);

/// Resolve the PA08 candidate task kind.
///
/// The `expect` is unreachable: candidate local 27 always fits the 12-bit
/// local space (`LOCAL_MAX` is 4095). It is loud rather than silent so a
/// future edit changing the local cannot fabricate a wrong kind.
pub fn pa08_task_kind() -> TaskKind {
    PA08_TASK_KIND_CANDIDATE.expect("PARSE local 27 fits in the 12-bit local space")
}

/// Resolve the PA10 candidate task kind.
///
/// The `expect` is unreachable: candidate local 28 always fits the 12-bit
/// local space (`LOCAL_MAX` is 4095). It is loud rather than silent so a
/// future edit changing the local cannot fabricate a wrong kind.
pub fn pa10_task_kind() -> TaskKind {
    PA10_TASK_KIND_CANDIDATE.expect("PARSE local 28 fits in the 12-bit local space")
}

/// Production served by [`PaDeclSpecChip`]: the dispatch target resolved
/// from the dispatched task's kind (the production field).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaDeclSpecProduction {
    /// PA08 array-suffix production (`[...]` suffix or its M1 absence).
    Array,
    /// PA10 abstract-declarator production (bare `int` in M1).
    Abstract,
}

impl PaDeclSpecProduction {
    /// Resolve the production for a dispatched task kind.
    pub fn of_kind(kind: TaskKind) -> Option<Self> {
        if kind == pa08_task_kind() {
            Some(Self::Array)
        } else if kind == pa10_task_kind() {
            Some(Self::Abstract)
        } else {
            None
        }
    }
}

/// One projected token: kind plus resolved spellings.
///
/// Identifier/keyword spellings resolve through the intern table
/// (`spelling`); punctuator spellings resolve through the committed PP
/// token (`pp_spelling`), because committed C punctuator records carry no
/// interned name. Either side is empty only when the backing record is
/// missing, which the computation reports loudly instead of guessing.
#[derive(Clone, Debug)]
pub struct ProjectedDeclSpecToken {
    /// C-token kind.
    pub kind: TokenKind,
    /// Interned spelling bytes for named tokens.
    pub spelling: Vec<u8>,
    /// Committed PP spelling bytes (punctuator ground truth).
    pub pp_spelling: Vec<u8>,
}

/// Narrow projection for the PA08 array computation.
#[derive(Clone, Debug)]
pub struct PaArrayInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token IDs in source order: the suffix slice (empty means
    /// the suffix is absent — the M1 case — not a truncated payload).
    pub tokens: Vec<TokenId>,
    /// Present tokens by ID.
    pub bodies: BTreeMap<TokenId, ProjectedDeclSpecToken>,
}

/// Narrow projection for the PA10 abstract computation.
#[derive(Clone, Debug)]
pub struct PaAbstractInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token IDs in source order: the type-name slice (M1
    /// requires exactly the single `int` token).
    pub tokens: Vec<TokenId>,
    /// Present tokens by ID.
    pub bodies: BTreeMap<TokenId, ProjectedDeclSpecToken>,
}

/// Projected token stream for one array/abstract task: token IDs in source
/// order, present token bodies, dispatch state.
type DeclSpecTokenProjection = (
    Vec<TokenId>,
    BTreeMap<TokenId, ProjectedDeclSpecToken>,
    TaskState,
);

fn project_decl_spec_tokens(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
    want: TaskKind,
    label: &str,
) -> Result<DeclSpecTokenProjection, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("{label} of unknown task {}", task.index())))?;
    if record.kind != want {
        return Err(protocol_fault(format!(
            "{label} task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    // No empty-payload rejection here: an empty suffix slice is the
    // meaningful M1 PA08 input (suffix absent), and an empty PA10 slice is
    // the empty-type-name defect the computation reports as `Task/4`.
    let mut tokens = Vec::new();
    let mut bodies = BTreeMap::new();
    for reference in &record.payload.refs {
        let RecordRef::Token(id) = reference else {
            return Err(protocol_fault(format!(
                "{label} task {} payload must carry tokens only",
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
                ProjectedDeclSpecToken {
                    kind: body.kind,
                    spelling,
                    pp_spelling,
                },
            );
        }
    }
    Ok((tokens, bodies, record.state.clone()))
}

/// Build the narrow PA08 projection for one dispatched task.
///
/// Payload convention (candidate): the array-suffix token refs in source
/// order — empty when the declarator carries no array suffix (the M1
/// case), otherwise the single `[...]` suffix starting at `[`.
pub fn project_pa_array_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaArrayInput, DiagnosticDraft> {
    let (tokens, bodies, state) =
        project_decl_spec_tokens(bus, task, pa08_task_kind(), "array parse")?;
    Ok(PaArrayInput {
        task,
        state,
        tokens,
        bodies,
    })
}

/// Build the narrow PA10 projection for one dispatched task.
///
/// Payload convention (candidate): the type-name token refs in source
/// order — exactly the single Keyword `int` token for M1.
pub fn project_pa_abstract_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PaAbstractInput, DiagnosticDraft> {
    let (tokens, bodies, state) =
        project_decl_spec_tokens(bus, task, pa10_task_kind(), "abstract parse")?;
    Ok(PaAbstractInput {
        task,
        state,
        tokens,
        bodies,
    })
}

/// The fused T05 PA08/PA10 worker (M1 slice: absent-suffix / bare-`int`
/// validate + `Ack`).
pub struct PaDeclSpecChip;

impl Worker for PaDeclSpecChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PA_DECL_SPEC_CANDIDATE_CHIP,
            chip_name: "PaDeclSpecChip",
            group: TaskGroup::PARSE,
            task_kinds: vec![pa08_task_kind(), pa10_task_kind()],
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
            // Candidate row: the shared acceptance suite is
            // integrator-owned; until it lands, the covering tests are the
            // in-file unit tests below.
            tests: vec!["compiler/src/chips/parse/pa_decl_spec.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let kind = match bus.arenas.tasks.get(task) {
            Ok(record) => record.kind,
            Err(_) => {
                return vec![fail(
                    task,
                    protocol_fault(format!("decl-spec parse of unknown task {}", task.index())),
                )];
            }
        };
        match PaDeclSpecProduction::of_kind(kind) {
            Some(PaDeclSpecProduction::Array) => match project_pa_array_input(bus, task) {
                Ok(input) => self.compute_array(&input),
                Err(diagnostic) => vec![fail(task, diagnostic)],
            },
            Some(PaDeclSpecProduction::Abstract) => match project_pa_abstract_input(bus, task) {
                Ok(input) => self.compute_abstract(&input),
                Err(diagnostic) => vec![fail(task, diagnostic)],
            },
            None => vec![fail(
                task,
                protocol_fault(format!(
                    "decl-spec task {} has unexpected kind {}",
                    task.index(),
                    kind.raw()
                )),
            )],
        }
    }
}

/// One suffix / type-name token for the pure grammar core.
///
/// `spelling` is the effective spelling: intern bytes for identifiers and
/// keywords, committed PP bytes for punctuators. The projector resolves
/// which side applies; the pure core only compares bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclSpecToken {
    /// C-token kind.
    pub kind: TokenKind,
    /// Effective spelling bytes.
    pub spelling: Vec<u8>,
}

/// Why a token slice is not the M1 array suffix / abstract declarator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclSpecError {
    /// A well-formed (or malformed) shape outside the M1 subset: any
    /// present array suffix, any operator-bearing or non-`int` abstract,
    /// any named declarator in abstract position. Maps to `Unsupported`.
    Unsupported(&'static str),
    /// A `[` suffix with no closing `]` at bracket depth 0: an
    /// unterminated suffix, never an implicit close. Maps to the `Task/4`
    /// defect diagnostic.
    UnterminatedArray,
    /// An empty PA10 type-name slice: a type name requires at least the
    /// specifier. Maps to the `Task/4` defect diagnostic.
    EmptyTypeName,
}

impl DeclSpecError {
    /// Map to the structured failure diagnostic for this rejection.
    ///
    /// Numeric codes are chip-local candidates; the frozen code table is a
    /// T01 `/6` detail.
    pub fn diagnostic(&self) -> DiagnosticDraft {
        match self {
            Self::Unsupported(message) => DiagnosticDraft::unsupported(*message),
            Self::UnterminatedArray => DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "DEFECT: unterminated array suffix; M1 declarators carry no array suffix",
            ),
            Self::EmptyTypeName => DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                "DEFECT: empty abstract type name; M1 requires the bare `int` specifier",
            ),
        }
    }
}

/// Validated M1 array suffix: absent.
///
/// M1 declarators carry no array suffix, so the only `Ok` shape is absence
/// (the empty slice). Every present `[...]` suffix — including `[]` — is
/// well-formed C deferred past M1 and fails `Unsupported`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArraySuffixShape {
    /// Whether the suffix is absent (always true for the M1 shape).
    absent: bool,
}

impl ArraySuffixShape {
    /// The M1 absent-suffix shape.
    pub fn absent() -> Self {
        Self { absent: true }
    }
    /// Whether the declarator carries no array suffix.
    pub fn is_absent(&self) -> bool {
        self.absent
    }
}

/// Validated M1 abstract type name: bare `int`.
///
/// Carries the specifier spelling so the wiring layer can lift it verbatim;
/// positions stay caller-held (the single token is `tokens[0]`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbstractTypeName {
    /// Specifier spelling bytes (`int` in M1).
    spelling: Vec<u8>,
}

impl AbstractTypeName {
    /// Specifier spelling bytes.
    pub fn spec_spelling(&self) -> &[u8] {
        &self.spelling
    }
    /// Fixed shape name (mirrors the future split's node inventory).
    pub fn kind_name(&self) -> &'static str {
        "AbstractInt"
    }
    /// Whether the type name is the bare `int` specifier (always true).
    pub fn is_bare_int(&self) -> bool {
        self.spelling == b"int"
    }
}

fn views_of(
    tokens: &[TokenId],
    bodies: &BTreeMap<TokenId, ProjectedDeclSpecToken>,
    label: &str,
) -> Result<Vec<DeclSpecToken>, DiagnosticDraft> {
    let mut views = Vec::with_capacity(tokens.len());
    for id in tokens {
        let Some(projected) = bodies.get(id) else {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("{label} reads missing token {}", id.index()),
            ));
        };
        let spelling = match projected.kind {
            TokenKind::Identifier | TokenKind::Keyword => projected.spelling.clone(),
            _ => projected.pp_spelling.clone(),
        };
        if spelling.is_empty() {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("{label} reads token {} with no spelling", id.index()),
            ));
        }
        views.push(DeclSpecToken {
            kind: projected.kind,
            spelling,
        });
    }
    Ok(views)
}

fn check_running(task: TaskId, state: &TaskState, label: &str) -> Result<(), DiagnosticDraft> {
    if *state != TaskState::Running {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("{label} task {} is not running", task.index()),
        ));
    }
    Ok(())
}

impl PaDeclSpecChip {
    /// Pure PA08 computation over the narrow projection (no bus access).
    ///
    /// An absent suffix completes with `Ack` and appends nothing. Any
    /// present `[...]` suffix fails `Unsupported`; an unterminated `[`
    /// fails with the defect diagnostic.
    pub fn compute_array(&self, input: &PaArrayInput) -> Vec<Proposal> {
        if let Err(diagnostic) = check_running(input.task, &input.state, "array parse") {
            return vec![fail(input.task, diagnostic)];
        }
        let views = match views_of(&input.tokens, &input.bodies, "array parse") {
            Ok(views) => views,
            Err(diagnostic) => return vec![fail(input.task, diagnostic)],
        };
        match parse_array_suffix(&views) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(error) => vec![fail(input.task, error.diagnostic())],
        }
    }

    /// Pure PA10 computation over the narrow projection (no bus access).
    ///
    /// The bare `int` type name completes with `Ack` and appends nothing.
    /// Every other shape fails `Unsupported`; an empty slice fails with
    /// the defect diagnostic.
    pub fn compute_abstract(&self, input: &PaAbstractInput) -> Vec<Proposal> {
        if let Err(diagnostic) = check_running(input.task, &input.state, "abstract parse") {
            return vec![fail(input.task, diagnostic)];
        }
        let views = match views_of(&input.tokens, &input.bodies, "abstract parse") {
            Ok(views) => views,
            Err(diagnostic) => return vec![fail(input.task, diagnostic)],
        };
        match parse_abstract_type_name(&views) {
            Ok(_) => vec![Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            }],
            Err(error) => vec![fail(input.task, error.diagnostic())],
        }
    }
}

/// Parse an M1 array suffix (PA08): the suffix is absent.
///
/// An empty slice validates as the absent-suffix shape. A slice opening
/// with `[` is a well-formed C array suffix deferred past M1: its interior
/// is classified (`[]`, bound expression, `static`, qualifiers, `[*]`) so
/// the rejection names the deferred feature, but it is never evaluated
/// here (bound semantics stay PA15/T08-owned; see DEFECT-3). A `[` with no
/// closing `]` at bracket depth 0 is the unterminated-suffix defect. Any
/// other opening token is not an array suffix at all.
pub fn parse_array_suffix(tokens: &[DeclSpecToken]) -> Result<ArraySuffixShape, DeclSpecError> {
    if tokens.is_empty() {
        return Ok(ArraySuffixShape::absent());
    }
    let first = &tokens[0];
    if !(first.kind == TokenKind::Punctuator && first.spelling == b"[") {
        return Err(DeclSpecError::Unsupported(
            "non-array suffix; M1 declarators carry no array suffix",
        ));
    }
    let mut depth: u32 = 0;
    let mut close: Option<usize> = None;
    for (position, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Punctuator {
            continue;
        }
        if token.spelling == b"[" {
            depth = depth.saturating_add(1);
        } else if token.spelling == b"]" {
            if depth == 0 {
                continue;
            }
            depth -= 1;
            if depth == 0 {
                close = Some(position);
                break;
            }
        }
    }
    let Some(close) = close else {
        return Err(DeclSpecError::UnterminatedArray);
    };
    if close != tokens.len() - 1 {
        return Err(DeclSpecError::Unsupported(
            "trailing tokens after the array suffix; one suffix is validated per task",
        ));
    }
    classify_array_interior(&tokens[1..close])
}

/// Classify the interior of one `[...]` array suffix for the rejection.
///
/// Every interior shape is well-formed C deferred past M1, so every path
/// returns `Err(Unsupported)` naming the deferred feature. Bound
/// expressions are never evaluated here (DEFECT-3).
fn classify_array_interior(interior: &[DeclSpecToken]) -> Result<ArraySuffixShape, DeclSpecError> {
    if interior.is_empty() {
        return Err(DeclSpecError::Unsupported(
            "incomplete array bound `[]` deferred past M1; M1 declarators carry no array suffix",
        ));
    }
    if interior.len() == 1
        && interior[0].kind == TokenKind::Punctuator
        && interior[0].spelling == b"*"
    {
        return Err(DeclSpecError::Unsupported(
            "`[*]` unspecified bound in a prototype deferred past M1; M1 declarators carry no array suffix",
        ));
    }
    if interior
        .iter()
        .any(|token| token.kind == TokenKind::Keyword && token.spelling == b"static")
    {
        return Err(DeclSpecError::Unsupported(
            "`static` array bound deferred past M1 (parameter-array minimum bound); M1 declarators carry no array suffix",
        ));
    }
    if interior.iter().any(|token| {
        token.kind == TokenKind::Keyword
            && (token.spelling == b"const"
                || token.spelling == b"volatile"
                || token.spelling == b"restrict"
                || token.spelling == b"_Atomic")
    }) {
        return Err(DeclSpecError::Unsupported(
            "qualified array bound deferred past M1; M1 declarators carry no array suffix",
        ));
    }
    if interior
        .iter()
        .any(|token| token.kind == TokenKind::Punctuator && token.spelling == b"*")
    {
        return Err(DeclSpecError::Unsupported(
            "variably-modified array bound deferred past M1; M1 declarators carry no array suffix",
        ));
    }
    Err(DeclSpecError::Unsupported(
        "array bound expression deferred past M1 (parameter arrays and VLAs need PA15/T08 evaluation); M1 declarators carry no array suffix",
    ))
}

/// Parse an M1 abstract type name (PA10): the bare `int` specifier.
///
/// Accepts exactly one Keyword `int` token. Pointer abstracts (`*`),
/// parenthesized abstracts (`(T)`, covering `int(*)[3]` and cast function
/// pointers), array abstracts (`T[...]`), named declarators in abstract
/// position (PA05 territory), and non-`int` specifiers are all well-formed
/// C deferred past M1 and fail `Unsupported`. An empty slice is the
/// empty-type-name defect, never an implicit `int`.
pub fn parse_abstract_type_name(
    tokens: &[DeclSpecToken],
) -> Result<AbstractTypeName, DeclSpecError> {
    if tokens.is_empty() {
        return Err(DeclSpecError::EmptyTypeName);
    }
    if tokens.len() == 1 && tokens[0].kind == TokenKind::Keyword && tokens[0].spelling == b"int" {
        return Ok(AbstractTypeName {
            spelling: b"int".to_vec(),
        });
    }
    if tokens
        .iter()
        .any(|token| token.kind == TokenKind::Punctuator && token.spelling == b"*")
    {
        return Err(DeclSpecError::Unsupported(
            "pointer abstract declarator deferred past M1; M1 accepts the bare `int` type name only",
        ));
    }
    if tokens[0].kind == TokenKind::Punctuator && tokens[0].spelling == b"(" {
        return Err(DeclSpecError::Unsupported(
            "parenthesized abstract declarator deferred past M1; M1 accepts the bare `int` type name only",
        ));
    }
    if tokens
        .iter()
        .any(|token| token.kind == TokenKind::Punctuator && token.spelling == b"[")
    {
        return Err(DeclSpecError::Unsupported(
            "array abstract declarator is PA08 territory; M1 accepts the bare `int` type name only",
        ));
    }
    if tokens
        .iter()
        .any(|token| token.kind == TokenKind::Identifier)
    {
        return Err(DeclSpecError::Unsupported(
            "named declarator in type-name position is PA05 territory; abstract type names carry no identifier",
        ));
    }
    Err(DeclSpecError::Unsupported(
        "non-`int` abstract type name deferred past M1; M1 accepts the bare `int` type name only",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ident(spelling: &[u8]) -> DeclSpecToken {
        DeclSpecToken {
            kind: TokenKind::Identifier,
            spelling: spelling.to_vec(),
        }
    }

    fn keyword(spelling: &[u8]) -> DeclSpecToken {
        DeclSpecToken {
            kind: TokenKind::Keyword,
            spelling: spelling.to_vec(),
        }
    }

    fn punct(spelling: &[u8]) -> DeclSpecToken {
        DeclSpecToken {
            kind: TokenKind::Punctuator,
            spelling: spelling.to_vec(),
        }
    }

    fn integer(spelling: &[u8]) -> DeclSpecToken {
        DeclSpecToken {
            kind: TokenKind::Integer,
            spelling: spelling.to_vec(),
        }
    }

    fn array_input_of(views: &[DeclSpecToken]) -> PaArrayInput {
        let (tokens, bodies) = projected_of(views);
        PaArrayInput {
            task: TaskId::from_index(0),
            state: TaskState::Running,
            tokens,
            bodies,
        }
    }

    fn abstract_input_of(views: &[DeclSpecToken]) -> PaAbstractInput {
        let (tokens, bodies) = projected_of(views);
        PaAbstractInput {
            task: TaskId::from_index(0),
            state: TaskState::Running,
            tokens,
            bodies,
        }
    }

    fn projected_of(
        views: &[DeclSpecToken],
    ) -> (Vec<TokenId>, BTreeMap<TokenId, ProjectedDeclSpecToken>) {
        let mut tokens = Vec::new();
        let mut bodies = BTreeMap::new();
        for (index, view) in views.iter().enumerate() {
            let id = TokenId::from_index(index as u32);
            tokens.push(id);
            let (spelling, pp_spelling) = match view.kind {
                TokenKind::Identifier | TokenKind::Keyword => (view.spelling.clone(), Vec::new()),
                _ => (Vec::new(), view.spelling.clone()),
            };
            bodies.insert(
                id,
                ProjectedDeclSpecToken {
                    kind: view.kind,
                    spelling,
                    pp_spelling,
                },
            );
        }
        (tokens, bodies)
    }

    #[test]
    fn absent_array_suffix_validates() {
        let shape = parse_array_suffix(&[]).expect("absent suffix is the M1 shape");
        assert!(shape.is_absent());
        assert_eq!(shape, ArraySuffixShape::absent());
    }

    #[test]
    fn array_suffix_forms_are_unsupported() {
        for views in [
            vec![punct(b"["), punct(b"]")],
            vec![punct(b"["), integer(b"3"), punct(b"]")],
            vec![punct(b"["), keyword(b"static"), integer(b"3"), punct(b"]")],
            vec![punct(b"["), keyword(b"const"), integer(b"2"), punct(b"]")],
            vec![punct(b"["), punct(b"*"), punct(b"]")],
            vec![
                punct(b"["),
                integer(b"2"),
                punct(b"]"),
                punct(b"["),
                integer(b"3"),
                punct(b"]"),
            ],
            vec![punct(b"("), keyword(b"void"), punct(b")")],
        ] {
            let error = parse_array_suffix(&views).expect_err("array suffix defers past M1");
            assert!(
                matches!(error, DeclSpecError::Unsupported(_)),
                "unexpected error for {views:?}: {error:?}"
            );
            assert_eq!(error.diagnostic().code.group, DiagGroup::Unsupported);
        }
    }

    #[test]
    fn unterminated_array_suffix_is_defect() {
        for views in [vec![punct(b"[")], vec![punct(b"["), integer(b"3")]] {
            assert_eq!(
                parse_array_suffix(&views),
                Err(DeclSpecError::UnterminatedArray)
            );
        }
        let diagnostic = DeclSpecError::UnterminatedArray.diagnostic();
        assert_eq!(diagnostic.code.group, DiagGroup::Task);
        assert_eq!(diagnostic.code.code, 4);
    }

    #[test]
    fn bare_int_abstract_validates_with_accessors() {
        let shape =
            parse_abstract_type_name(&[keyword(b"int")]).expect("bare `int` is the M1 type name");
        assert_eq!(shape.spec_spelling(), b"int");
        assert_eq!(shape.kind_name(), "AbstractInt");
        assert!(shape.is_bare_int());
    }

    #[test]
    fn abstract_shapes_are_unsupported() {
        for views in [
            vec![punct(b"*"), keyword(b"int")],
            vec![punct(b"("), keyword(b"int"), punct(b")")],
            vec![
                punct(b"("),
                keyword(b"int"),
                punct(b"("),
                punct(b"*"),
                punct(b")"),
                punct(b"["),
                integer(b"3"),
                punct(b"]"),
                punct(b")"),
            ],
            vec![keyword(b"int"), punct(b"["), integer(b"3"), punct(b"]")],
            vec![ident(b"x")],
            vec![keyword(b"unsigned")],
            vec![keyword(b"void")],
            vec![keyword(b"int"), keyword(b"int")],
        ] {
            let error =
                parse_abstract_type_name(&views).expect_err("non-M1 abstract defers past M1");
            assert!(
                matches!(error, DeclSpecError::Unsupported(_)),
                "unexpected error for {views:?}: {error:?}"
            );
            assert_eq!(error.diagnostic().code.group, DiagGroup::Unsupported);
        }
    }

    #[test]
    fn empty_abstract_is_defect() {
        assert_eq!(
            parse_abstract_type_name(&[]),
            Err(DeclSpecError::EmptyTypeName)
        );
        let diagnostic = DeclSpecError::EmptyTypeName.diagnostic();
        assert_eq!(diagnostic.code.group, DiagGroup::Task);
        assert_eq!(diagnostic.code.code, 4);
    }

    #[test]
    fn compute_acks_m1_and_fails_loud() {
        let chip = PaDeclSpecChip;

        let ok_array = chip.compute_array(&array_input_of(&[]));
        assert_eq!(
            ok_array,
            vec![Proposal::Complete {
                task: TaskId::from_index(0),
                value: ResultValue::Ack,
            }]
        );
        let bad_array =
            chip.compute_array(&array_input_of(&[punct(b"["), integer(b"3"), punct(b"]")]));
        let [Proposal::Fail { diagnostic, .. }] = bad_array.as_slice() else {
            panic!("present array suffix must fail");
        };
        assert_eq!(diagnostic.code.group, DiagGroup::Unsupported);

        let ok_abstract = chip.compute_abstract(&abstract_input_of(&[keyword(b"int")]));
        assert_eq!(
            ok_abstract,
            vec![Proposal::Complete {
                task: TaskId::from_index(0),
                value: ResultValue::Ack,
            }]
        );
        let bad_abstract =
            chip.compute_abstract(&abstract_input_of(&[punct(b"*"), keyword(b"int")]));
        let [Proposal::Fail { diagnostic, .. }] = bad_abstract.as_slice() else {
            panic!("pointer abstract must fail");
        };
        assert_eq!(diagnostic.code.group, DiagGroup::Unsupported);

        let defect_abstract = chip.compute_abstract(&abstract_input_of(&[]));
        let [Proposal::Fail { diagnostic, .. }] = defect_abstract.as_slice() else {
            panic!("empty type name must fail");
        };
        assert_eq!(diagnostic.code.group, DiagGroup::Task);
    }

    #[test]
    fn compute_rejects_non_running_task() {
        let chip = PaDeclSpecChip;
        let mut array = array_input_of(&[]);
        array.state = TaskState::Ready;
        let [Proposal::Fail { .. }] = chip.compute_array(&array).as_slice() else {
            panic!("non-running array task must fail");
        };
        let mut abstract_input = abstract_input_of(&[keyword(b"int")]);
        abstract_input.state = TaskState::Ready;
        let [Proposal::Fail { .. }] = chip.compute_abstract(&abstract_input).as_slice() else {
            panic!("non-running abstract task must fail");
        };
    }

    #[test]
    fn candidate_registration_is_next_free() {
        assert_eq!(PA08_CANDIDATE_LOCAL, 27);
        assert_eq!(PA10_CANDIDATE_LOCAL, 28);
        assert_eq!(PA_DECL_SPEC_CANDIDATE_CHIP, ChipId(58));
        assert_eq!(
            pa08_task_kind(),
            TaskKind::new(TaskGroup::PARSE, 27).expect("local 27 fits")
        );
        assert_eq!(
            pa10_task_kind(),
            TaskKind::new(TaskGroup::PARSE, 28).expect("local 28 fits")
        );
        assert_eq!(pa08_task_kind().group(), TaskGroup::PARSE);
        assert_eq!(pa10_task_kind().group(), TaskGroup::PARSE);
        assert_eq!(
            PaDeclSpecProduction::of_kind(pa08_task_kind()),
            Some(PaDeclSpecProduction::Array)
        );
        assert_eq!(
            PaDeclSpecProduction::of_kind(pa10_task_kind()),
            Some(PaDeclSpecProduction::Abstract)
        );
    }
}
