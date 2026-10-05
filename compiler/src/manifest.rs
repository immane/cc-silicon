// ============================================================================
// manifest.rs — compiler SFL manifest extension (T01 C04)
//
// This is the *compiler application* extension of the SFL schema draft. The
// draft in docs/architecture/SFL_SCHEMA_DRAFT.md does not ship a validator;
// this module therefore defines a concrete, dependency-free manifest shape
// plus the validator that the routing shell and CI consume.
//
// The validator is a lint, not a proof: it checks declared store/field paths,
// phase, capability, backend class, task kind registration, and the presence
// of tests. It cannot inspect code.
// ============================================================================

use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::ChipId;
use crate::limits::STAGE_COUNT;
use crate::routing::RoutingTable;
use crate::task::{StoreId, TaskGroup, TaskKind, TaskKindRegistry};

/// The tick phase in which a chip is allowed to run.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChipPhase {
    /// Combinational propagation (all compiler chips).
    Propagation,
    /// The motherboard's latching phase; never assignable to a chip.
    Latch,
}

impl ChipPhase {
    /// Stable label used in snapshots.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Propagation => "propagation",
            Self::Latch => "latch",
        }
    }
}

/// SFL chip capability category.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Capability {
    /// Directly representable on every backend.
    Portable,
    /// Groupable with neighbours.
    Batchable,
    /// Runs on the host when the backend cannot represent it.
    Emulable,
    /// Intentionally limited to one backend family.
    DeviceSpecific,
    /// Opt-in experimental.
    Experimental,
}

impl Capability {
    /// Stable label used in snapshots.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Portable => "portable",
            Self::Batchable => "batchable",
            Self::Emulable => "emulable",
            Self::DeviceSpecific => "device_specific",
            Self::Experimental => "experimental",
        }
    }
}

/// Execution backend class for a chip.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BackendClass {
    /// Sequential CPU reference.
    CpuReference,
    /// AArch64 GNU/Linux target code.
    Aarch64Linux,
}

impl BackendClass {
    /// Stable label used in snapshots.
    pub const fn name(self) -> &'static str {
        match self {
            Self::CpuReference => "cpu-reference",
            Self::Aarch64Linux => "aarch64-linux",
        }
    }
}

/// A declared store/field path, e.g. `tasks.active.id`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FieldPath {
    /// Target store.
    pub store: StoreId,
    /// Field path within the store.
    pub field: &'static str,
}

impl FieldPath {
    /// Build a path.
    pub const fn new(store: StoreId, field: &'static str) -> Self {
        Self { store, field }
    }

    /// Parse `store.field` (the split is on the first `.`).
    pub fn parse(raw: &'static str) -> Option<Self> {
        let (store_name, field) = raw.split_once('.')?;
        let store = StoreId::parse(store_name)?;
        if field.is_empty() {
            return None;
        }
        Some(Self { store, field })
    }

    /// Canonical `store.field` string.
    pub fn canonical(&self) -> String {
        format!("{}.{}", self.store.name(), self.field)
    }
}

/// Structured schema-shape failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchemaError {
    /// The field is already declared for the store.
    DuplicateField {
        /// Offending path.
        path: FieldPath,
    },
    /// The store has no schema entry (an internal construction bug).
    UnknownStore {
        /// Offending store.
        store: StoreId,
    },
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateField { path } => {
                write!(f, "field `{}` already declared", path.canonical())
            }
            Self::UnknownStore { store } => {
                write!(f, "store `{}` has no schema entry", store.name())
            }
        }
    }
}

impl std::error::Error for SchemaError {}

/// Declared store fields for the compiler application.
#[derive(Clone, Debug)]
pub struct StoreSchema {
    /// One entry per store, sorted by store discriminant.
    entries: Vec<(StoreId, Vec<&'static str>)>,
}

impl Default for StoreSchema {
    fn default() -> Self {
        Self::new()
    }
}

impl StoreSchema {
    /// An empty schema with one entry per store.
    pub fn new() -> Self {
        Self {
            entries: StoreId::ALL
                .into_iter()
                .map(|store| (store, Vec::new()))
                .collect(),
        }
    }

    /// The frozen foundation field set.
    ///
    /// Group-owned stores (`pp`, `parse`, `ir`, ...) start empty; their owners
    /// call [`StoreSchema::declare`] when they freeze their record fields.
    pub fn foundation() -> Self {
        let mut schema = Self::new();
        let foundation: &[(StoreId, &str)] = &[
            (StoreId::Config, "target"),
            (StoreId::Config, "dialect"),
            (StoreId::Config, "options"),
            (StoreId::Config, "limits"),
            (StoreId::Control, "phase"),
            (StoreId::Control, "tick"),
            (StoreId::Control, "job_state"),
            (StoreId::Control, "budget"),
            (StoreId::Control, "selected_task"),
            (StoreId::Control, "enqueue_ordinal"),
            (StoreId::Sources, "bytes"),
            (StoreId::Sources, "name"),
            (StoreId::Sources, "span_root"),
            (StoreId::Sources, "expansion"),
            (StoreId::Tasks, "active.id"),
            (StoreId::Tasks, "active.kind"),
            (StoreId::Tasks, "active.payload"),
            (StoreId::Tasks, "active.owner"),
            (StoreId::Tasks, "active.parent"),
            (StoreId::Tasks, "active.continuation"),
            (StoreId::Tasks, "active.state"),
            (StoreId::Tasks, "queue.ready"),
            (StoreId::Tasks, "results"),
            (StoreId::Tasks, "completed"),
            (StoreId::Diagnostics, "entries"),
            (StoreId::Artifacts, "fragments"),
            (StoreId::Wires, "proposals.self"),
            (StoreId::Wires, "selected_task"),
        ];
        for &(store, field) in foundation {
            // The table is constant and valid; a failure here would be a bug.
            let _ = schema.declare(store, field);
        }
        schema
    }

    /// The Wave 2 (`/16`) PP-slice field set: the IR slice plus the PP
    /// scan append field (`sources.spans` for `SpanRecord`; `pp.tokens` and
    /// `artifacts.fragments` are already declared). Post-seed runtime
    /// declarations stay excluded from the frozen hash per the two-tier
    /// model.
    pub fn pp_slice() -> Self {
        let mut schema = Self::ir_slice();
        let slice: &[(StoreId, &str)] = &[(StoreId::Sources, "spans")];
        for &(store, field) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = schema.declare(store, field);
        }
        schema
    }

    /// The Wave 2 (`/15`) IR-slice field set: the SE slice plus the IR
    /// append fields (`ir.functions`, `ir.blocks`, `ir.values`,
    /// `ir.instructions`). Post-seed runtime declarations stay excluded
    /// from the frozen hash per the two-tier model; the IR inventory is
    /// pinned by the `IR_*_NAMES` contract lists instead.
    pub fn ir_slice() -> Self {
        let mut schema = Self::se_slice();
        let slice: &[(StoreId, &str)] = &[
            (StoreId::Ir, "functions"),
            (StoreId::Ir, "blocks"),
            (StoreId::Ir, "values"),
            (StoreId::Ir, "instructions"),
        ];
        for &(store, field) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = schema.declare(store, field);
        }
        schema
    }

    /// The Wave 2 (`/14`) SE-slice field set: the TY slice plus the SE
    /// append field (`sem.records` for `SemRecord`). Post-seed runtime
    /// declarations stay excluded from the frozen hash per the two-tier
    /// model; the SE inventory is pinned by the `SEM_*` contract lists.
    pub fn se_slice() -> Self {
        let mut schema = Self::ty_slice();
        let slice: &[(StoreId, &str)] = &[(StoreId::Sem, "records")];
        for &(store, field) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = schema.declare(store, field);
        }
        schema
    }

    /// The Wave 2 (`/13`) TY-slice field set: the PA slice plus the TY
    /// append fields (`types.records`, `symbols.symbols`, `symbols.scopes`,
    /// `symbols.scope_events`). Post-seed runtime declarations stay excluded
    /// from the frozen hash per the two-tier model; the TY inventory is
    /// pinned by the `*_NAMES` contract lists instead.
    pub fn ty_slice() -> Self {
        let mut schema = Self::pa_slice();
        let slice: &[(StoreId, &str)] = &[
            (StoreId::Types, "records"),
            (StoreId::Symbols, "symbols"),
            (StoreId::Symbols, "scopes"),
            (StoreId::Symbols, "scope_events"),
        ];
        for &(store, field) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = schema.declare(store, field);
        }
        schema
    }

    /// The Wave 2 (`/12`) PA-slice field set: the LX slice plus the PA
    /// append field (`parse.nodes` for `NodeRecord`). Post-seed runtime
    /// declarations stay excluded from the frozen hash per the two-tier
    /// model; the PA inventory is pinned by the `NODE_*_NAMES` lists.
    pub fn pa_slice() -> Self {
        let mut schema = Self::lx_slice();
        let slice: &[(StoreId, &str)] = &[(StoreId::Parse, "nodes")];
        for &(store, field) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = schema.declare(store, field);
        }
        schema
    }

    /// The Wave 2 (`/11`) LX-slice field set: the M1 slice plus the LX
    /// append fields (`names.entries` for interned names, `pp.tokens` for
    /// `PpTokenRecord`, `lex.tokens` for `TokenRecord`; `lex.literals` is
    /// already declared). Post-seed runtime declarations stay excluded from
    /// the frozen hash per the two-tier model; the LX inventory is pinned by
    /// the `LX_*_NAMES` contract lists instead.
    pub fn lx_slice() -> Self {
        let mut schema = Self::m1_slice();
        let slice: &[(StoreId, &str)] = &[
            (StoreId::Names, "entries"),
            (StoreId::Pp, "tokens"),
            (StoreId::Lex, "tokens"),
        ];
        for &(store, field) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = schema.declare(store, field);
        }
        schema
    }

    /// The Gate 1 (`/7`) M1 slice field set: foundation plus the two frozen
    /// language append fields (`lex.literals` for the `LiteralRecord`
    /// schema, `constants.records` for the `ConstRecord` schema).
    ///
    /// Group-owned stores otherwise start empty; their owners call
    /// [`StoreSchema::declare`] when they freeze their record fields.
    /// `foundation()` itself is unchanged and stays the `/6` record.
    pub fn m1_slice() -> Self {
        let mut schema = Self::foundation();
        let slice: &[(StoreId, &str)] =
            &[(StoreId::Lex, "literals"), (StoreId::Constants, "records")];
        for &(store, field) in slice {
            // The table is constant and valid; a failure here would be a bug.
            let _ = schema.declare(store, field);
        }
        schema
    }

    /// Declare a field for a store.
    pub fn declare(&mut self, store: StoreId, field: &'static str) -> Result<(), SchemaError> {
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|(candidate, _)| *candidate == store)
        else {
            return Err(SchemaError::UnknownStore { store });
        };
        if entry.1.contains(&field) {
            return Err(SchemaError::DuplicateField {
                path: FieldPath::new(store, field),
            });
        }
        entry.1.push(field);
        Ok(())
    }

    /// Whether a path is declared.
    pub fn has_field(&self, path: &FieldPath) -> bool {
        self.fields(path.store).contains(&path.field)
    }

    /// Declared fields for a store, in declaration order.
    pub fn fields(&self, store: StoreId) -> &[&'static str] {
        self.entries
            .iter()
            .find(|(candidate, _)| *candidate == store)
            .map(|(_, fields)| fields.as_slice())
            .unwrap_or(&[])
    }
}

/// A per-chip SFL manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChipManifest {
    /// Unique chip ID.
    pub id: ChipId,
    /// Unique human-readable chip name.
    pub chip_name: &'static str,
    /// Owning task group.
    pub group: TaskGroup,
    /// Task kinds this chip accepts.
    pub task_kinds: Vec<TaskKind>,
    /// Declared read set.
    pub reads: Vec<FieldPath>,
    /// Declared write set.
    pub writes: Vec<FieldPath>,
    /// SFL capability category.
    pub capability: Capability,
    /// Execution backend class.
    pub backend_class: BackendClass,
    /// Tick phase.
    pub phase: ChipPhase,
    /// Whether output is deterministic for fixed input.
    pub deterministic: bool,
    /// Test paths that cover this chip.
    pub tests: Vec<&'static str>,
    /// Producer task kinds this chip depends on (data dependencies only).
    pub dependencies: Vec<TaskKind>,
}

impl ChipManifest {
    /// Whether this chip accepts the given task kind.
    pub fn accepts_kind(&self, kind: TaskKind) -> bool {
        self.task_kinds.contains(&kind)
    }

    /// Whether a field is declared in this chip's read set.
    pub fn declares_read(&self, store: StoreId, field: &str) -> bool {
        self.reads
            .iter()
            .any(|path| path.store == store && path.field == field)
    }

    /// Whether a field is declared in this chip's write set.
    pub fn declares_write(&self, store: StoreId, field: &str) -> bool {
        self.writes
            .iter()
            .any(|path| path.store == store && path.field == field)
    }
}

/// Structured manifest validation failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ManifestError {
    /// Chip ID already registered.
    DuplicateId {
        /// Offending ID.
        id: ChipId,
    },
    /// Chip name already registered.
    DuplicateName {
        /// Offending name.
        name: &'static str,
    },
    /// Two chips claim the same task kind.
    DuplicateKindClaim {
        /// Contended kind.
        kind: TaskKind,
        /// The first claimant.
        other: ChipId,
    },
    /// A chip declares no task kinds.
    NoTaskKinds {
        /// Chip name.
        chip: &'static str,
    },
    /// A task kind is not registered.
    UnregisteredKind {
        /// Chip name.
        chip: &'static str,
        /// Offending kind.
        kind: TaskKind,
    },
    /// A task kind belongs to a different group than the chip.
    KindGroupMismatch {
        /// Chip name.
        chip: &'static str,
        /// Offending kind.
        kind: TaskKind,
        /// Chip group.
        group: TaskGroup,
    },
    /// A read path is not declared.
    UnknownReadField {
        /// Chip name.
        chip: &'static str,
        /// Offending path.
        path: FieldPath,
    },
    /// A write path is not declared.
    UnknownWriteField {
        /// Chip name.
        chip: &'static str,
        /// Offending path.
        path: FieldPath,
    },
    /// No tests are declared.
    MissingTests {
        /// Chip name.
        chip: &'static str,
    },
    /// The chip is not deterministic.
    NonDeterministic {
        /// Chip name.
        chip: &'static str,
    },
    /// The chip is assigned to the latch phase.
    WrongPhase {
        /// Chip name.
        chip: &'static str,
        /// Offending phase.
        phase: ChipPhase,
    },
    /// Capability/backend class does not match the group rule.
    BackendClassMismatch {
        /// Chip name.
        chip: &'static str,
        /// Offending capability.
        capability: Capability,
        /// Offending backend class.
        backend_class: BackendClass,
    },
    /// The manifest writes to the read-only `config` store.
    ConfigStoreWrite {
        /// Chip name.
        chip: &'static str,
        /// Field path.
        field: &'static str,
    },
    /// A declared write has no `(ChipId, StoreId, field)` allowlist row
    /// authorizing it (rule `manifest.store-owner-allowlist-mandatory`).
    StoreOwnerViolation {
        /// Chip name.
        chip: &'static str,
        /// Target store.
        store: StoreId,
        /// Field path within the store.
        field: &'static str,
        /// The task kind authorized for this field by the seed; echoes the
        /// chip's first claimed kind when no seed row matches.
        expected_kind: TaskKind,
    },
    /// A claimed task kind has no stage assignment.
    StageUnassigned {
        /// Offending kind.
        kind: TaskKind,
    },
    /// A claimed kind's stage does not match its routed layer.
    StageLayerMismatch {
        /// Offending kind.
        kind: TaskKind,
        /// `/6` stage ordinal.
        stage: u16,
        /// Routed layer.
        layer: u16,
    },
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateId { id } => write!(f, "duplicate chip id {}", id.index()),
            Self::DuplicateName { name } => write!(f, "duplicate chip name `{name}`"),
            Self::DuplicateKindClaim { kind, other } => write!(
                f,
                "task kind {} already claimed by chip {}",
                kind.raw(),
                other.index()
            ),
            Self::NoTaskKinds { chip } => write!(f, "chip `{chip}` declares no task kinds"),
            Self::UnregisteredKind { chip, kind } => write!(
                f,
                "chip `{chip}` claims unregistered task kind {}",
                kind.raw()
            ),
            Self::KindGroupMismatch { chip, kind, group } => write!(
                f,
                "chip `{chip}` claims kind {} from group `{}` but belongs to `{}`",
                kind.raw(),
                kind.group().name(),
                group.name()
            ),
            Self::UnknownReadField { chip, path } => {
                write!(
                    f,
                    "chip `{chip}` reads undeclared field `{}`",
                    path.canonical()
                )
            }
            Self::UnknownWriteField { chip, path } => write!(
                f,
                "chip `{chip}` writes undeclared field `{}`",
                path.canonical()
            ),
            Self::MissingTests { chip } => write!(f, "chip `{chip}` declares no tests"),
            Self::NonDeterministic { chip } => {
                write!(f, "chip `{chip}` is not deterministic")
            }
            Self::WrongPhase { chip, phase } => {
                write!(f, "chip `{chip}` uses forbidden phase `{}`", phase.name())
            }
            Self::BackendClassMismatch {
                chip,
                capability,
                backend_class,
            } => write!(
                f,
                "chip `{chip}` has capability `{}` but backend class `{}`",
                capability.name(),
                backend_class.name()
            ),
            Self::ConfigStoreWrite { chip, field } => {
                write!(f, "chip `{chip}` writes read-only config field `{field}`")
            }
            Self::StoreOwnerViolation {
                chip,
                store,
                field,
                expected_kind,
            } => write!(
                f,
                "chip `{chip}` writes `{}.{}` without an allowlist row (expected kind {})",
                store.name(),
                field,
                expected_kind.raw()
            ),
            Self::StageUnassigned { kind } => {
                write!(f, "task kind {} has no stage assignment", kind.raw())
            }
            Self::StageLayerMismatch { kind, stage, layer } => write!(
                f,
                "task kind {} in stage {stage} does not match routed layer {layer}",
                kind.raw()
            ),
        }
    }
}

impl std::error::Error for ManifestError {}

impl ManifestError {
    /// Map to a structured diagnostic.
    pub fn to_diagnostic(&self) -> DiagnosticDraft {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Manifest, 1),
            self.to_string(),
        )
    }
}

/// Structured manifest-registration failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ManifestRegistryError {
    /// The manifest failed structural validation.
    Validation {
        /// Chip name.
        chip: &'static str,
        /// All validation errors.
        errors: Vec<ManifestError>,
    },
    /// The manifest conflicts with an already-registered one.
    Registry(ManifestError),
}

impl std::fmt::Display for ManifestRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation { chip, errors } => write!(
                f,
                "chip `{chip}` failed manifest validation ({} errors)",
                errors.len()
            ),
            Self::Registry(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ManifestRegistryError {}

/// Validate one manifest against the frozen schema and kind registry.
pub fn validate_manifest(
    manifest: &ChipManifest,
    schema: &StoreSchema,
    kinds: &TaskKindRegistry,
) -> Result<(), Vec<ManifestError>> {
    let mut errors = Vec::new();
    if manifest.chip_name.is_empty() {
        errors.push(ManifestError::DuplicateName { name: "" });
    }
    if manifest.task_kinds.is_empty() {
        errors.push(ManifestError::NoTaskKinds {
            chip: manifest.chip_name,
        });
    }
    for &kind in &manifest.task_kinds {
        match kinds.lookup(kind) {
            None => errors.push(ManifestError::UnregisteredKind {
                chip: manifest.chip_name,
                kind,
            }),
            Some(entry) if entry.group != manifest.group => {
                errors.push(ManifestError::KindGroupMismatch {
                    chip: manifest.chip_name,
                    kind,
                    group: manifest.group,
                });
            }
            Some(_) => {}
        }
    }
    for path in &manifest.reads {
        if !schema.has_field(path) {
            errors.push(ManifestError::UnknownReadField {
                chip: manifest.chip_name,
                path: *path,
            });
        }
    }
    for path in &manifest.writes {
        if path.store == StoreId::Config {
            errors.push(ManifestError::ConfigStoreWrite {
                chip: manifest.chip_name,
                field: path.field,
            });
        }
        if !schema.has_field(path) {
            errors.push(ManifestError::UnknownWriteField {
                chip: manifest.chip_name,
                path: *path,
            });
        }
    }
    if manifest.tests.is_empty() {
        errors.push(ManifestError::MissingTests {
            chip: manifest.chip_name,
        });
    }
    if !manifest.deterministic {
        errors.push(ManifestError::NonDeterministic {
            chip: manifest.chip_name,
        });
    }
    if manifest.phase != ChipPhase::Propagation {
        errors.push(ManifestError::WrongPhase {
            chip: manifest.chip_name,
            phase: manifest.phase,
        });
    }
    if !matches_backend_rule(manifest.group, manifest.capability, manifest.backend_class) {
        errors.push(ManifestError::BackendClassMismatch {
            chip: manifest.chip_name,
            capability: manifest.capability,
            backend_class: manifest.backend_class,
        });
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn matches_backend_rule(
    group: TaskGroup,
    capability: Capability,
    backend_class: BackendClass,
) -> bool {
    if group == TaskGroup::TARGET_CODE {
        capability == Capability::DeviceSpecific && backend_class == BackendClass::Aarch64Linux
    } else {
        capability == Capability::Emulable && backend_class == BackendClass::CpuReference
    }
}

/// A registry of per-chip manifests with unique IDs, names, and kind claims.
#[derive(Clone, Debug, Default)]
pub struct ManifestRegistry {
    manifests: Vec<ChipManifest>,
}

/// Seed store-owner allowlist: `(chip, store, field, kind)` rows.
///
/// Each row authorizes one chip ([`ChipId`]) to write one store field for one
/// task kind. Gate 1 (`/7`) seeds the single writer row the M1 const-fold
/// chain needs: the T08 fold chip ([`G1_FOLD_CHIP`]) appending the
/// `constants.records` field for [`TaskKind::CONSTANT_CONST_FOLD`].
/// Later waves add their rows with their tests; kinds outside the frozen
/// waves stay dormant (see [`check_store_owner_allowlist`]).
///
/// Pinned invariant: `tasks.ready` (the [`StoreId::Tasks`] `"queue.ready"`
/// field) gets zero allowlisted chip writers — it is a derived quota-1
/// compatibility view over the canonical per-stage queues, never a second
/// write target.
pub const STORE_OWNER_ALLOWLIST: &[(ChipId, StoreId, &str, TaskKind)] = &[
    (
        G1_FOLD_CHIP,
        StoreId::Constants,
        "records",
        TaskKind::CONSTANT_CONST_FOLD,
    ),
    (
        PP01_CHIP,
        StoreId::Artifacts,
        "fragments",
        TaskKind::PREPROCESS_NORMALIZE,
    ),
    (
        LX_INTERN_CHIP,
        StoreId::Names,
        "entries",
        TaskKind::LEX_INTERN,
    ),
    (
        LX_CLASSIFY_CHIP,
        StoreId::Lex,
        "tokens",
        TaskKind::LEX_CLASSIFY,
    ),
    (
        LX_DECODE_CHIP,
        StoreId::Lex,
        "literals",
        TaskKind::LEX_DECODE_LITERAL,
    ),
    (PA_TU_CHIP, StoreId::Parse, "nodes", TaskKind::PARSE_TU),
    (
        TY_TYPE_CHIP,
        StoreId::Types,
        "records",
        TaskKind::SYMBOL_INT_TYPE,
    ),
    (
        TY_TYPE_CHIP,
        StoreId::Types,
        "records",
        TaskKind::SYMBOL_FUNC_TYPE,
    ),
    (
        TY_SCOPE_CHIP,
        StoreId::Symbols,
        "scopes",
        TaskKind::SYMBOL_SCOPE_ENTER,
    ),
    (
        TY_SCOPE_CHIP,
        StoreId::Symbols,
        "scope_events",
        TaskKind::SYMBOL_SCOPE_ENTER,
    ),
    (
        TY_SCOPE_CHIP,
        StoreId::Symbols,
        "scope_events",
        TaskKind::SYMBOL_SCOPE_EXIT,
    ),
    (
        TY_SYMBOL_CHIP,
        StoreId::Symbols,
        "symbols",
        TaskKind::SYMBOL_DECLARE,
    ),
    (
        SE_LIT_CHIP,
        StoreId::Sem,
        "records",
        TaskKind::SEMANTIC_LITERAL_EXPR,
    ),
    (
        SE_BIN_CHIP,
        StoreId::Sem,
        "records",
        TaskKind::SEMANTIC_BINARY_EXPR,
    ),
    (
        SE_RET_CHIP,
        StoreId::Sem,
        "records",
        TaskKind::SEMANTIC_RETURN_STMT,
    ),
    (
        IR_FUNCTION_CHIP,
        StoreId::Ir,
        "functions",
        TaskKind::IR_FUNCTION,
    ),
    (
        IR_FUNCTION_CHIP,
        StoreId::Ir,
        "blocks",
        TaskKind::IR_FUNCTION,
    ),
    (
        IR_FUNCTION_CHIP,
        StoreId::Ir,
        "values",
        TaskKind::IR_FUNCTION,
    ),
    (
        IR_FUNCTION_CHIP,
        StoreId::Ir,
        "instructions",
        TaskKind::IR_FUNCTION,
    ),
    (
        PP_SPLICE_CHIP,
        StoreId::Artifacts,
        "fragments",
        TaskKind::PREPROCESS_SPLICE,
    ),
    (
        PP_COMMENT_CHIP,
        StoreId::Artifacts,
        "fragments",
        TaskKind::PREPROCESS_COMMENT,
    ),
    (
        PP_SCAN_CHIP,
        StoreId::Sources,
        "spans",
        TaskKind::PREPROCESS_SCAN,
    ),
    (
        PP_SCAN_CHIP,
        StoreId::Pp,
        "tokens",
        TaskKind::PREPROCESS_SCAN,
    ),
];

/// Gate 1 (`/7`) T08 fold chip reservation.
///
/// The Wave-1 fold chip registers with this ID and claims
/// [`TaskKind::CONSTANT_CONST_FOLD`]; the allowlist row above authorizes
/// its `constants.records` appends. (The T07 requester chip needs no write
/// rows: it produces requests through `Enqueue` proposals, never store
/// writes.)
pub const G1_FOLD_CHIP: ChipId = ChipId(2);

/// Wave 2 (`/10`) PP01 normalize chip reservation.
///
/// The PP01 chip registers with this ID and claims
/// [`TaskKind::PREPROCESS_NORMALIZE`]; the allowlist row above authorizes
/// its `artifacts.fragments` appends.
pub const PP01_CHIP: ChipId = ChipId(3);

/// Wave 2 (`/11`) LX name-intern chip reservation.
pub const LX_INTERN_CHIP: ChipId = ChipId(4);
/// Wave 2 (`/11`) LX token-classify chip reservation.
pub const LX_CLASSIFY_CHIP: ChipId = ChipId(5);
/// Wave 2 (`/11`) LX literal-decode chip reservation.
pub const LX_DECODE_CHIP: ChipId = ChipId(6);

/// Wave 2 (`/12`) PA TU-parse chip reservation.
pub const PA_TU_CHIP: ChipId = ChipId(7);

/// Wave 2 (`/13`) TY canonical-type chip reservation (int + func producers).
pub const TY_TYPE_CHIP: ChipId = ChipId(8);
/// Wave 2 (`/13`) TY scope chip reservation (enter + exit).
pub const TY_SCOPE_CHIP: ChipId = ChipId(9);
/// Wave 2 (`/13`) TY symbol chip reservation (declare + lookup).
pub const TY_SYMBOL_CHIP: ChipId = ChipId(10);
/// Wave 2 (`/13`) TY conversion chip reservation (identity-only promote,
/// common-type, return conversion).
pub const TY_CONV_CHIP: ChipId = ChipId(11);

/// Wave 2 (`/14`) SE literal-expression chip reservation.
pub const SE_LIT_CHIP: ChipId = ChipId(12);
/// Wave 2 (`/14`) SE binary-expression chip reservation (forwards to fold).
pub const SE_BIN_CHIP: ChipId = ChipId(13);
/// Wave 2 (`/14`) SE return-statement chip reservation.
pub const SE_RET_CHIP: ChipId = ChipId(14);
/// Wave 2 (`/14`) VF06 typed-invariant verifier reservation.
pub const VF06_CHIP: ChipId = ChipId(15);

/// Wave 2 (`/15`) IR function-lowering chip reservation.
pub const IR_FUNCTION_CHIP: ChipId = ChipId(16);

/// Wave 2 (`/16`) PP line-splice chip reservation.
pub const PP_SPLICE_CHIP: ChipId = ChipId(17);
/// Wave 2 (`/16`) PP comment-replace chip reservation.
pub const PP_COMMENT_CHIP: ChipId = ChipId(18);
/// Wave 2 (`/16`) PP token-scan chip reservation.
pub const PP_SCAN_CHIP: ChipId = ChipId(19);

/// Wave 2 (`/17`) VF12 symbolic-interpret chip reservation.
pub const VF12_CHIP: ChipId = ChipId(20);

/// Wave 2 (`/18`) VF05 token-AST invariant chip reservation.
pub const VF05_CHIP: ChipId = ChipId(21);

/// Wave 2 (`/19`) VF01 store-invariant chip reservation.
pub const VF01_CHIP: ChipId = ChipId(22);

/// Whether a task kind belongs to the Gate 1 (`/7`) M1 slice.
pub const fn is_gate1_slice_kind(kind: TaskKind) -> bool {
    kind.raw() == TaskKind::SEMANTIC_CONST_EVAL_LITERAL.raw()
        || kind.raw() == TaskKind::SEMANTIC_CONST_EVAL_BINARY.raw()
        || kind.raw() == TaskKind::CONSTANT_CONST_FOLD.raw()
}

/// Whether a task kind belongs to the Wave 2 (`/10`) PP01 slice.
pub const fn is_pp01_slice_kind(kind: TaskKind) -> bool {
    kind.raw() == TaskKind::PREPROCESS_NORMALIZE.raw()
}

/// Whether a task kind belongs to the Wave 2 (`/11`) LX slice.
pub const fn is_lx_slice_kind(kind: TaskKind) -> bool {
    kind.raw() == TaskKind::LEX_INTERN.raw()
        || kind.raw() == TaskKind::LEX_CLASSIFY.raw()
        || kind.raw() == TaskKind::LEX_DECODE_LITERAL.raw()
}

/// Whether a task kind belongs to the Wave 2 (`/12`) PA slice.
pub const fn is_pa_slice_kind(kind: TaskKind) -> bool {
    kind.raw() == TaskKind::PARSE_TU.raw()
}

/// Whether a task kind belongs to the Wave 2 (`/19`) VF01 slice.
pub const fn is_vf01_slice_kind(kind: TaskKind) -> bool {
    kind.raw() == TaskKind::VERIFICATION_STORE_INVARIANT.raw()
}

/// Whether a task kind belongs to the Wave 2 (`/18`) VF05 slice.
pub const fn is_vf05_slice_kind(kind: TaskKind) -> bool {
    kind.raw() == TaskKind::VERIFICATION_TOKEN_AST_INVARIANT.raw()
}

/// Whether a task kind belongs to the Wave 2 (`/17`) VF12 slice.
pub const fn is_vf12_slice_kind(kind: TaskKind) -> bool {
    kind.raw() == TaskKind::VERIFICATION_IR_INTERPRET.raw()
}

/// Whether a task kind belongs to the Wave 2 (`/16`) PP slice
/// (splice/comment/scan; PP01 lives in `is_pp01_slice_kind`).
pub const fn is_pp_slice_kind(kind: TaskKind) -> bool {
    kind.raw() == TaskKind::PREPROCESS_SPLICE.raw()
        || kind.raw() == TaskKind::PREPROCESS_COMMENT.raw()
        || kind.raw() == TaskKind::PREPROCESS_SCAN.raw()
}

/// Whether a task kind belongs to the Wave 2 (`/15`) IR slice.
pub const fn is_ir_slice_kind(kind: TaskKind) -> bool {
    kind.raw() == TaskKind::IR_FUNCTION.raw()
}

/// Whether a task kind belongs to the Wave 2 (`/14`) SE slice (including
/// its VF06 verifier kind).
pub const fn is_se_slice_kind(kind: TaskKind) -> bool {
    kind.raw() == TaskKind::SEMANTIC_LITERAL_EXPR.raw()
        || kind.raw() == TaskKind::SEMANTIC_BINARY_EXPR.raw()
        || kind.raw() == TaskKind::SEMANTIC_RETURN_STMT.raw()
        || kind.raw() == TaskKind::VERIFICATION_TYPED_INVARIANT.raw()
}

/// Whether a task kind belongs to the Wave 2 (`/13`) TY slice.
pub const fn is_ty_slice_kind(kind: TaskKind) -> bool {
    kind.raw() == TaskKind::SYMBOL_INT_TYPE.raw()
        || kind.raw() == TaskKind::SYMBOL_FUNC_TYPE.raw()
        || kind.raw() == TaskKind::SYMBOL_SCOPE_ENTER.raw()
        || kind.raw() == TaskKind::SYMBOL_SCOPE_EXIT.raw()
        || kind.raw() == TaskKind::SYMBOL_DECLARE.raw()
        || kind.raw() == TaskKind::SYMBOL_LOOKUP.raw()
        || kind.raw() == TaskKind::SYMBOL_PROMOTE.raw()
        || kind.raw() == TaskKind::SYMBOL_COMMON_TYPE.raw()
        || kind.raw() == TaskKind::SYMBOL_RETURN_CONVERT.raw()
}

/// Enforce the store-owner allowlist for one manifest.
///
/// Wave-gated: manifests that claim no Gate 1 slice kind pass untouched
/// (their waves land with their own rows and tests). A manifest that does
/// claim a slice kind needs a row matching the chip ID, store, field, and
/// one of its claimed task kinds for every declared write.
fn check_store_owner_allowlist(manifest: &ChipManifest) -> Result<(), ManifestError> {
    if STORE_OWNER_ALLOWLIST.is_empty() {
        return Ok(());
    }
    if !manifest.task_kinds.iter().any(|&kind| {
        is_gate1_slice_kind(kind)
            || is_pp01_slice_kind(kind)
            || is_lx_slice_kind(kind)
            || is_pa_slice_kind(kind)
            || is_ty_slice_kind(kind)
            || is_se_slice_kind(kind)
            || is_ir_slice_kind(kind)
            || is_pp_slice_kind(kind)
            || is_vf12_slice_kind(kind)
            || is_vf05_slice_kind(kind)
            || is_vf01_slice_kind(kind)
    }) {
        return Ok(());
    }
    let Some(&first_kind) = manifest.task_kinds.first() else {
        // No claimed kinds: structural validation already reported
        // `NoTaskKinds`; there is nothing to authorize against.
        return Ok(());
    };
    for path in &manifest.writes {
        let allowed = manifest.task_kinds.iter().any(|&kind| {
            STORE_OWNER_ALLOWLIST
                .iter()
                .any(|&(chip, store, field, row_kind)| {
                    chip == manifest.id
                        && store == path.store
                        && field == path.field
                        && row_kind == kind
                })
        });
        if !allowed {
            return Err(ManifestError::StoreOwnerViolation {
                chip: manifest.chip_name,
                store: path.store,
                field: path.field,
                expected_kind: first_kind,
            });
        }
    }
    Ok(())
}

/// Gate 1 (`/7`) kind-to-stage assignment: `(task kind, stage ordinal)`.
///
/// Foundation control kinds run at stage 0. The M1 slice follows chain
/// order (request at 1, fold at 2). Stage ordinals range over
/// `0..STAGE_COUNT`; what each stage *means* beyond this order, and the
/// rows for every other kind, are a later co-freeze — this table grows
/// wave by wave, never by silent default.
pub const STAGE_ASSIGNMENT: &[(TaskKind, u8)] = &[
    (TaskKind::CONTROL_NOOP, 0),
    (TaskKind::CONTROL_UNSUPPORTED, 0),
    (TaskKind::CONTROL_START_JOB, 0),
    (TaskKind::CONTROL_IMPORT_SOURCE, 0),
    (TaskKind::PREPROCESS_NORMALIZE, 1),
    (TaskKind::LEX_INTERN, 2),
    (TaskKind::LEX_CLASSIFY, 2),
    (TaskKind::LEX_DECODE_LITERAL, 2),
    (TaskKind::PARSE_TU, 2),
    (TaskKind::SYMBOL_INT_TYPE, 3),
    (TaskKind::SYMBOL_FUNC_TYPE, 3),
    (TaskKind::SYMBOL_SCOPE_ENTER, 3),
    (TaskKind::SYMBOL_SCOPE_EXIT, 3),
    (TaskKind::SYMBOL_DECLARE, 3),
    (TaskKind::SYMBOL_LOOKUP, 3),
    (TaskKind::SYMBOL_PROMOTE, 3),
    (TaskKind::SYMBOL_COMMON_TYPE, 3),
    (TaskKind::SYMBOL_RETURN_CONVERT, 3),
    (TaskKind::SEMANTIC_LITERAL_EXPR, 4),
    (TaskKind::SEMANTIC_BINARY_EXPR, 4),
    (TaskKind::SEMANTIC_RETURN_STMT, 4),
    (TaskKind::VERIFICATION_TYPED_INVARIANT, 4),
    (TaskKind::IR_FUNCTION, 5),
    (TaskKind::PREPROCESS_SPLICE, 1),
    (TaskKind::PREPROCESS_COMMENT, 1),
    (TaskKind::PREPROCESS_SCAN, 1),
    (TaskKind::VERIFICATION_IR_INTERPRET, 6),
    (TaskKind::VERIFICATION_TOKEN_AST_INVARIANT, 2),
    (TaskKind::VERIFICATION_STORE_INVARIANT, 6),
    (TaskKind::SEMANTIC_CONST_EVAL_LITERAL, 1),
    (TaskKind::SEMANTIC_CONST_EVAL_BINARY, 1),
    (TaskKind::CONSTANT_CONST_FOLD, 2),
];

/// Look up the frozen stage ordinal for a task kind.
pub const fn stage_of(kind: TaskKind) -> Option<u8> {
    let mut index = 0;
    while index < STAGE_ASSIGNMENT.len() {
        // `TaskKind` has no const equality; raw codes are stable and frozen.
        if STAGE_ASSIGNMENT[index].0.raw() == kind.raw() {
            return Some(STAGE_ASSIGNMENT[index].1);
        }
        index += 1;
    }
    None
}

/// Chip-to-stage declaration check.
///
/// Every claimed kind needs a [`STAGE_ASSIGNMENT`] row inside
/// `0..STAGE_COUNT`, else [`ManifestError::StageUnassigned`]. Both error
/// variants are now reachable: `StageUnassigned` here, and
/// [`ManifestError::StageLayerMismatch`] through
/// [`check_stage_layer_agreement`] once the kind is routed.
fn check_stage_assignment(manifest: &ChipManifest) -> Result<(), ManifestError> {
    for &kind in &manifest.task_kinds {
        let Some(stage) = stage_of(kind) else {
            return Err(ManifestError::StageUnassigned { kind });
        };
        // The table is frozen and valid; an out-of-range row would be a bug.
        debug_assert!((stage as usize) < STAGE_COUNT);
        if (stage as usize) >= STAGE_COUNT {
            return Err(ManifestError::StageUnassigned { kind });
        }
    }
    Ok(())
}

/// Check stage-vs-routed-layer agreement for one manifest.
///
/// For every claimed kind that has both a stage row and a routing entry,
/// the routed layer must equal the stage, else
/// [`ManifestError::StageLayerMismatch`]. Unrouted kinds (Wave-1 chips
/// register their routes with their manifests) pass untouched.
pub fn check_stage_layer_agreement(
    manifest: &ChipManifest,
    routing: &RoutingTable,
) -> Result<(), ManifestError> {
    for &kind in &manifest.task_kinds {
        let (Some(stage), Some(entry)) = (stage_of(kind), routing.lookup(kind)) else {
            continue;
        };
        if entry.layer != stage as u16 {
            return Err(ManifestError::StageLayerMismatch {
                kind,
                stage: stage as u16,
                layer: entry.layer,
            });
        }
    }
    Ok(())
}

impl ManifestRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a manifest, validating it against the schema and kind registry
    /// and enforcing global uniqueness.
    pub fn register(
        &mut self,
        manifest: ChipManifest,
        schema: &StoreSchema,
        kinds: &TaskKindRegistry,
    ) -> Result<(), ManifestRegistryError> {
        if let Err(errors) = validate_manifest(&manifest, schema, kinds) {
            return Err(ManifestRegistryError::Validation {
                chip: manifest.chip_name,
                errors,
            });
        }
        // Wave-gated ownership/stage gates: the allowlist enforces manifests
        // that claim Gate 1 slice kinds (other waves stay dormant until
        // their rows land), and every claimed kind needs a stage row.
        check_store_owner_allowlist(&manifest).map_err(ManifestRegistryError::Registry)?;
        check_stage_assignment(&manifest).map_err(ManifestRegistryError::Registry)?;
        if self.manifests.iter().any(|other| other.id == manifest.id) {
            return Err(ManifestRegistryError::Registry(
                ManifestError::DuplicateId { id: manifest.id },
            ));
        }
        if self
            .manifests
            .iter()
            .any(|other| other.chip_name == manifest.chip_name)
        {
            return Err(ManifestRegistryError::Registry(
                ManifestError::DuplicateName {
                    name: manifest.chip_name,
                },
            ));
        }
        for &kind in &manifest.task_kinds {
            if let Some(other) = self
                .manifests
                .iter()
                .find(|other| other.task_kinds.contains(&kind))
            {
                return Err(ManifestRegistryError::Registry(
                    ManifestError::DuplicateKindClaim {
                        kind,
                        other: other.id,
                    },
                ));
            }
        }
        self.manifests.push(manifest);
        Ok(())
    }

    /// Look up a manifest by ID.
    pub fn get(&self, id: ChipId) -> Option<&ChipManifest> {
        self.manifests.iter().find(|manifest| manifest.id == id)
    }

    /// Look up a manifest by chip name.
    pub fn get_name(&self, name: &str) -> Option<&ChipManifest> {
        self.manifests
            .iter()
            .find(|manifest| manifest.chip_name == name)
    }

    /// Iterate manifests in registration order.
    pub fn iter(&self) -> impl Iterator<Item = &ChipManifest> {
        self.manifests.iter()
    }

    /// Number of registered manifests.
    pub fn len(&self) -> usize {
        self.manifests.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.manifests.is_empty()
    }
}
