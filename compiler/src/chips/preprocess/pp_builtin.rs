// ============================================================================
// chips/preprocess/pp_builtin.rs — T03 PP24 builtin-macro worker
//
// Expands one builtin-macro use (`MacroName/context -> Tokens`, T03:32,
// CHIP_PLAN.md:100; PP_EXPAND_SLICE.md lists builtins as an explicit
// non-goal of the `/24` slice, so this file is the PP24 worker written
// against that deferred scope).
//
// Payload convention (provisional, pending T03/T01 co-freeze): exactly one
// `RecordRef::PpToken` naming the builtin use, which must be an
// `Identifier` token. All context (`__FILE__` source name, `__LINE__`
// span, `__COUNTER__` seed) derives from that token's span chain, so no
// fan-out is needed: this chip is pure single-task (one dispatch, at most
// one quota-1 `PpToken` append, `Complete(Records)`), and the frozen-join
// path never applies (no children are ever enqueued).
//
// Documented fallbacks (no guessing, no host reads):
// - `__LINE__`: PP23 (`LineDirectiveChip`) is not implemented and no
//   logical-location carrier exists on the bus, so the physical line is
//   used: newlines before `span.start` in the owning source bytes, plus
//   one. The projector must prefer the frozen PP23 logical location once
//   that carrier lands.
// - `__COUNTER__`: the bus-state source is the task-associated counter
//   record when present, else the task-local seed 0. No counter record
//   family exists on the bus (see the closed `RecordFamily` inventory in
//   `ids.rs`; a task `continuation` carries parse state only), so the
//   projector always yields the seed 0 today. Each dispatch performs
//   exactly one expansion, so the emitted value equals the projected base.
//   Cross-tick monotonicity awaits a frozen counter carrier.
// - `__DATE__` / `__TIME__`: replayable values must be read from the
//   frozen config record, never from wall-clock time. `CompilerConfig`
//   carries no date/time record, so these names always fail as explicit
//   `Unsupported` naming PP24 (the spec's missing-config branch).
// - Frozen-target predefined macros are read off the projected target
//   model (`TargetSpec` profile plus `Dialect`), never off the host. The
//   closed table below is a provisional candidate pending co-freeze: it
//   names only frozen T01 facts (triple arch/os, ELF object format, LP64
//   data model, little-endian order) plus `__STDC__` and the
//   dialect-derived `__STDC_VERSION__`. Any other name fails as explicit
//   `Unsupported` (unknown builtin name).
//
// Frozen registration (`/27` integrator-owned): `PP24_TASK_KIND` aliases
// `TaskKind::PREPROCESS_MACRO_BUILTIN` (PREPROCESS local 31),
// `PP24_CHIP` is `crate::manifest::PP24_CHIP` (`ChipId(34)`), the
// kind-registry row lives in `TaskKindRegistry::pp_builtin_slice()`, the
// stage-1 row in `STAGE_ASSIGNMENT`, the routed layer is 1, the
// `(PP24_CHIP, Pp, "tokens")` allowlist row authorizes the single
// synthesized-token append, and the acceptance test is
// `compiler/tests/c27_builtin.rs`. Identical pins produce identical
// tokens (or an identical typed `Fail`); the chip performs no I/O, reads
// no clock, and samples no host environment.
// ============================================================================

use crate::bus::{PpTokenKind, PpTokenRecord, SpanRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{PpTokenId, RecordFamily, RecordRef, SourceId, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP24_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::target::{DataModel, Dialect, Endianness, ObjectFormat, TargetSpec};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// Frozen PP24 task kind (aliases `TaskKind::PREPROCESS_MACRO_BUILTIN`,
/// frozen by the `/27` integrator; the local code is `PREPROCESS` 31,
/// first code after `/26`).
pub const PP24_TASK_KIND: TaskKind = TaskKind::PREPROCESS_MACRO_BUILTIN;

/// Narrow projection for the builtin-macro computation.
#[derive(Clone, Debug)]
pub struct PpBuiltinInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// The builtin-use token ID (sole payload ref).
    pub name_id: PpTokenId,
    /// The builtin-use token body (must be an `Identifier`).
    pub name: PpTokenRecord,
    /// Span of the builtin-use token (line context and token span).
    pub span: SpanRecord,
    /// Owning source of that span.
    pub source_id: SourceId,
    /// Owning source bytes (physical-line context).
    pub source_bytes: Vec<u8>,
    /// Owning source name bytes (`__FILE__` spelling).
    pub source_name: Vec<u8>,
    /// Projected counter base: the task-associated counter record when
    /// present, else the task-local seed 0 (no counter carrier exists on
    /// the bus today, so this is always the seed; see the file header).
    pub counter_base: u32,
    /// Frozen target model snapshot (predefined-macro source, never host).
    pub target: TargetSpec,
    /// Frozen dialect snapshot (`__STDC_VERSION__` source).
    pub dialect: Dialect,
    /// `pp_tokens` arena count at dispatch (single-append prediction base).
    pub pp_tokens_allocated: u32,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pp_builtin_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpBuiltinInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("builtin of unknown task {}", task.index())))?;
    if record.kind != PP24_TASK_KIND {
        return Err(protocol_fault(format!(
            "builtin task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "builtin task {} payload must carry exactly one pp-token ref",
            task.index()
        )));
    }
    let name_id = match record.payload.refs[0] {
        RecordRef::PpToken(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "builtin task {} payload must be a pp-token ref",
                task.index()
            )));
        }
    };
    let name = match bus.arenas.pp_tokens.get(name_id) {
        Ok(found) => found.clone(),
        Err(_) => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("builtin reads missing pp-token {}", name_id.index()),
            ));
        }
    };
    if name.kind != PpTokenKind::Identifier {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("builtin task {} name must be an identifier", task.index()),
        ));
    }
    let span = match bus.arenas.spans.get(name.span) {
        Ok(found) => *found,
        Err(_) => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("builtin reads missing span {}", name.span.index()),
            ));
        }
    };
    let source_id: SourceId = span.source;
    let source_bytes = match bus.arenas.sources.get(source_id) {
        Ok(found) => found.bytes.clone(),
        Err(_) => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("builtin reads missing source {}", source_id.index()),
            ));
        }
    };
    let source_name = match bus.arenas.sources.get(source_id) {
        Ok(found) => match bus.intern.get(found.name) {
            Ok(bytes) => bytes.to_vec(),
            Err(_) => {
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!(
                        "builtin reads missing interned source name for source {}",
                        source_id.index()
                    ),
                ));
            }
        },
        Err(_) => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("builtin reads missing source {}", source_id.index()),
            ));
        }
    };
    // No task-associated counter record family exists on the bus, so the
    // projection yields the task-local seed 0 (file header documents the
    // seam the frozen carrier will fill).
    let counter_base: u32 = 0;
    Ok(PpBuiltinInput {
        task,
        state: record.state.clone(),
        name_id,
        name,
        span,
        source_id,
        source_bytes,
        source_name,
        counter_base,
        target: *bus.config().target(),
        dialect: bus.config().dialect(),
        pp_tokens_allocated: bus.arenas.pp_tokens.allocated(),
    })
}

/// The T03 builtin-macro worker (PP24 slice scope).
pub struct PpBuiltinChip;

impl Worker for PpBuiltinChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP24_CHIP,
            chip_name: "PpBuiltinChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![PP24_TASK_KIND],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Sources, "spans"),
                FieldPath::new(StoreId::Sources, "bytes"),
                FieldPath::new(StoreId::Names, "entries"),
                FieldPath::new(StoreId::Config, "target"),
                FieldPath::new(StoreId::Config, "dialect"),
            ],
            writes: vec![FieldPath::new(StoreId::Pp, "tokens")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c27_builtin.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pp_builtin_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl PpBuiltinChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// Pure single-task: exactly one builtin use expands to exactly one
    /// synthesized `PpToken` (span = the use token's span, per the PP12
    /// synthesized-token precedent), appended quota-1 with the predicted
    /// ID and completed as `Records`. No children are ever enqueued, so no
    /// frozen-join resume path exists. Every failure is a typed `Fail`,
    /// never a panic; identical inputs yield identical proposals.
    pub fn compute(&self, input: &PpBuiltinInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(input.task, not_running(input.task))];
        }
        let token = match expand_builtin(input) {
            Ok(token) => token,
            Err(diagnostic) => return vec![fail(input.task, diagnostic)],
        };
        let predicted = PpTokenId::from_index(input.pp_tokens_allocated);
        vec![
            Proposal::AppendRecords {
                task: input.task,
                batch: AppendBatch {
                    records: vec![RecordDraft {
                        family: RecordFamily::PpToken,
                        index: DraftRef(0),
                    }],
                    bodies: vec![G1DraftBody::PpToken(token)],
                },
            },
            Proposal::Complete {
                task: input.task,
                value: ResultValue::Records(vec![RecordRef::PpToken(predicted)]),
            },
        ]
    }
}

/// Typed failure for a dispatch that is not `Running`.
fn not_running(task: TaskId) -> DiagnosticDraft {
    DiagnosticDraft::error(
        DiagnosticCode::new(DiagGroup::Task, 4),
        format!("builtin task {} is not running", task.index()),
    )
}

/// Explicit `Unsupported` naming PP24.
fn unsupported_pp24(detail: String) -> DiagnosticDraft {
    DiagnosticDraft::unsupported(format!("PP24: {detail}"))
}

/// Route one builtin name to its single expansion token. `__DATE__` and
/// `__TIME__` fail (missing frozen config record); anything outside the
/// closed table fails as an unknown builtin name.
fn expand_builtin(input: &PpBuiltinInput) -> Result<PpTokenRecord, DiagnosticDraft> {
    let name = input.name.spelling.as_slice();
    if name == b"__FILE__" {
        return Ok(PpTokenRecord {
            kind: PpTokenKind::StringLiteral,
            span: input.name.span,
            spelling: quoted_spelling(&input.source_name),
        });
    }
    if name == b"__LINE__" {
        return Ok(PpTokenRecord {
            kind: PpTokenKind::PpNumber,
            span: input.name.span,
            spelling: decimal_spelling(physical_line(&input.source_bytes, input.span.start)),
        });
    }
    if name == b"__COUNTER__" {
        return Ok(PpTokenRecord {
            kind: PpTokenKind::PpNumber,
            span: input.name.span,
            spelling: decimal_spelling(input.counter_base as u64),
        });
    }
    if name == b"__DATE__" || name == b"__TIME__" {
        return Err(date_time_unsupported(&input.name.spelling));
    }
    if name == b"__STDC_VERSION__" {
        return match stdc_version(input.dialect) {
            Some(spelling) => Ok(number_token(input, spelling)),
            None => Err(unsupported_pp24(format!(
                "builtin macro `__STDC_VERSION__` has no frozen spelling for dialect `{}`",
                input.dialect.name()
            ))),
        };
    }
    match target_macro_value(&input.target, name) {
        Some(spelling) => Ok(number_token(input, spelling)),
        None => Err(unknown_builtin_unsupported(&input.name.spelling)),
    }
}

/// Explicit `Unsupported` for the replayable date/time builtins: the frozen
/// config carries no date/time record, so there is nothing replayable to
/// read (wall-clock sampling is forbidden).
fn date_time_unsupported(spelling: &[u8]) -> DiagnosticDraft {
    let text = String::from_utf8_lossy(spelling);
    unsupported_pp24(format!(
        "builtin macro `{text}` is unsupported: replaying it needs a frozen config date/time record and the frozen config carries none"
    ))
}

/// Explicit `Unsupported` for a name outside the closed builtin table.
fn unknown_builtin_unsupported(spelling: &[u8]) -> DiagnosticDraft {
    let text = String::from_utf8_lossy(spelling);
    unsupported_pp24(format!(
        "unknown builtin macro `{text}` is unsupported: PP24 has no frozen expansion for this name"
    ))
}

/// One predefined-macro token: a `PpNumber` spelled by the frozen table,
/// with the use token's span.
fn number_token(input: &PpBuiltinInput, spelling: &[u8]) -> PpTokenRecord {
    PpTokenRecord {
        kind: PpTokenKind::PpNumber,
        span: input.name.span,
        spelling: spelling.to_vec(),
    }
}

/// `__STDC_VERSION__` spelling per frozen dialect. Strict C90 leaves the
/// macro undefined, so C89 modes have no frozen mapping and fail at the
/// call site instead of fabricating a value.
fn stdc_version(dialect: Dialect) -> Option<&'static [u8]> {
    match dialect {
        Dialect::C89 => None,
        Dialect::Gnu89 => None,
        Dialect::C99 => Some(b"199901L"),
        Dialect::Gnu99 => Some(b"199901L"),
        Dialect::C11 => Some(b"201112L"),
        Dialect::Gnu11 => Some(b"201112L"),
        Dialect::C17 => Some(b"201710L"),
        Dialect::Gnu17 => Some(b"201710L"),
        Dialect::C23 => Some(b"202311L"),
        Dialect::Gnu23 => Some(b"202311L"),
    }
}

/// Closed provisional candidate table of frozen-target predefined macros
/// (pending T03/T01 co-freeze). Every entry names a frozen T01 fact read
/// off the projected target model; nothing is sampled from the host. A
/// model check that stops matching yields `None` (unknown-name failure)
/// rather than a fabricated value.
fn target_macro_value(target: &TargetSpec, name: &[u8]) -> Option<&'static [u8]> {
    if name == b"__STDC__" {
        return Some(b"1");
    }
    if name == b"__ORDER_LITTLE_ENDIAN__" {
        return Some(b"1234");
    }
    if name == b"__ORDER_BIG_ENDIAN__" {
        return Some(b"4321");
    }
    let profile = target.profile();
    if name == b"__aarch64__" {
        if profile.triple == "aarch64-unknown-linux-gnu" {
            return Some(b"1");
        }
        return None;
    }
    if name == b"__linux__" || name == b"__gnu_linux__" {
        if profile.triple == "aarch64-unknown-linux-gnu" {
            return Some(b"1");
        }
        return None;
    }
    if name == b"__ELF__" {
        if profile.object_format == ObjectFormat::Elf {
            return Some(b"1");
        }
        return None;
    }
    if name == b"__LP64__" || name == b"_LP64" {
        if profile.data_model == DataModel::Lp64 {
            return Some(b"1");
        }
        return None;
    }
    if name == b"__BYTE_ORDER__" {
        if profile.endianness == Endianness::Little {
            return Some(b"1234");
        }
        return None;
    }
    None
}

/// Physical-line fallback for `__LINE__` (PP23 logical location pending):
/// one plus the count of newline bytes strictly before `offset`, clamped
/// to the source end so an out-of-range span can never panic.
fn physical_line(bytes: &[u8], offset: u64) -> u64 {
    let end = (offset as usize).min(bytes.len());
    let mut line: u64 = 1;
    let mut cursor = 0usize;
    while cursor < end {
        if bytes[cursor] == b'\n' {
            line += 1;
        }
        cursor += 1;
    }
    line
}

/// Decimal spelling of a builtin value (no formatting machinery, no host
/// locale: plain ASCII digits, never empty).
fn decimal_spelling(value: u64) -> Vec<u8> {
    if value == 0 {
        return vec![b'0'];
    }
    let mut digits: Vec<u8> = Vec::new();
    let mut rest = value;
    while rest > 0 {
        digits.push(b'0' + (rest % 10) as u8);
        rest /= 10;
    }
    digits.reverse();
    digits
}

/// `__FILE__` spelling: the source name wrapped in double quotes, escaping
/// only `"` and `\` (mirrors the PP13 stringize escape rule).
fn quoted_spelling(name: &[u8]) -> Vec<u8> {
    let mut spelling: Vec<u8> = Vec::new();
    spelling.push(b'"');
    let mut cursor = 0usize;
    while cursor < name.len() {
        let byte = name[cursor];
        if byte == b'"' || byte == b'\\' {
            spelling.push(b'\\');
        }
        spelling.push(byte);
        cursor += 1;
    }
    spelling.push(b'"');
    spelling
}
