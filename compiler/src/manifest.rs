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
