use cc_silicon_compiler::diagnostic::DiagGroup;
use cc_silicon_compiler::ids::ChipId;
use cc_silicon_compiler::manifest::{
    validate_manifest, BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, ManifestError,
    ManifestRegistry, StoreSchema, STORE_OWNER_ALLOWLIST,
};
use cc_silicon_compiler::task::{KindStatus, StoreId, TaskGroup, TaskKind, TaskKindRegistry};

fn base_manifest() -> ChipManifest {
    ChipManifest {
        id: ChipId(1),
        chip_name: "NoopChip",
        group: TaskGroup::CONTROL,
        task_kinds: vec![TaskKind::CONTROL_NOOP],
        reads: vec![FieldPath::new(StoreId::Tasks, "active.id")],
        writes: vec![FieldPath::new(StoreId::Wires, "proposals.self")],
        capability: Capability::Emulable,
        backend_class: BackendClass::CpuReference,
        phase: ChipPhase::Propagation,
        deterministic: true,
        tests: vec!["tests/chips/control/noop.rs"],
        dependencies: vec![],
    }
}

fn errors(manifest: &ChipManifest) -> Vec<ManifestError> {
    validate_manifest(
        manifest,
        &StoreSchema::foundation(),
        &TaskKindRegistry::foundation(),
    )
    .unwrap_err()
}

#[test]
fn valid_manifest_passes() {
    let manifest = base_manifest();
    assert!(validate_manifest(
        &manifest,
        &StoreSchema::foundation(),
        &TaskKindRegistry::foundation()
    )
    .is_ok());
}

#[test]
fn undeclared_field_is_rejected() {
    let mut manifest = base_manifest();
    manifest
        .reads
        .push(FieldPath::new(StoreId::Parse, "not_declared"));
    assert!(errors(&manifest)
        .iter()
        .any(|error| matches!(error, ManifestError::UnknownReadField { .. })));

    let mut manifest = base_manifest();
    manifest
        .writes
        .push(FieldPath::new(StoreId::Parse, "not_declared"));
    assert!(errors(&manifest)
        .iter()
        .any(|error| matches!(error, ManifestError::UnknownWriteField { .. })));
}

#[test]
fn wrong_phase_is_rejected() {
    let mut manifest = base_manifest();
    manifest.phase = ChipPhase::Latch;
    assert!(errors(&manifest)
        .iter()
        .any(|error| matches!(error, ManifestError::WrongPhase { .. })));
}

#[test]
fn missing_tests_and_nondeterminism_are_rejected() {
    let mut manifest = base_manifest();
    manifest.tests.clear();
    manifest.deterministic = false;
    let found = errors(&manifest);
    assert!(found
        .iter()
        .any(|error| matches!(error, ManifestError::MissingTests { .. })));
    assert!(found
        .iter()
        .any(|error| matches!(error, ManifestError::NonDeterministic { .. })));
}

#[test]
fn unregistered_kind_is_rejected() {
    let mut manifest = base_manifest();
    manifest.task_kinds = vec![TaskKind::new(TaskGroup::CONTROL, 200).unwrap()];
    assert!(errors(&manifest)
        .iter()
        .any(|error| matches!(error, ManifestError::UnregisteredKind { .. })));
}

#[test]
fn kind_group_mismatch_is_rejected() {
    let mut manifest = base_manifest();
    manifest.group = TaskGroup::LEX;
    assert!(errors(&manifest)
        .iter()
        .any(|error| matches!(error, ManifestError::KindGroupMismatch { .. })));
}

#[test]
fn backend_class_rule_is_enforced() {
    let mut kinds = TaskKindRegistry::foundation();
    let cg = TaskKind::new(TaskGroup::TARGET_CODE, 16).unwrap();
    kinds
        .register(
            cg,
            "cg.emit",
            TaskGroup::TARGET_CODE,
            KindStatus::GroupOwned,
        )
        .unwrap();
    let manifest = ChipManifest {
        id: ChipId(2),
        chip_name: "EmitChip",
        group: TaskGroup::TARGET_CODE,
        task_kinds: vec![cg],
        reads: vec![],
        writes: vec![],
        capability: Capability::Emulable,
        backend_class: BackendClass::CpuReference,
        phase: ChipPhase::Propagation,
        deterministic: true,
        tests: vec!["tests/chips/target/emit.rs"],
        dependencies: vec![],
    };
    let found = validate_manifest(&manifest, &StoreSchema::foundation(), &kinds).unwrap_err();
    assert!(found
        .iter()
        .any(|error| matches!(error, ManifestError::BackendClassMismatch { .. })));

    let mut correct = manifest;
    correct.capability = Capability::DeviceSpecific;
    correct.backend_class = BackendClass::Aarch64Linux;
    assert!(validate_manifest(&correct, &StoreSchema::foundation(), &kinds).is_ok());
}

#[test]
fn registry_rejects_duplicate_id_name_and_kind_claim() {
    use cc_silicon_compiler::manifest::ManifestRegistryError;
    let schema = StoreSchema::foundation();
    let kinds = TaskKindRegistry::foundation();
    let mut registry = ManifestRegistry::new();
    registry.register(base_manifest(), &schema, &kinds).unwrap();

    let mut duplicate_id = base_manifest();
    duplicate_id.chip_name = "OtherChip";
    assert!(matches!(
        registry.register(duplicate_id, &schema, &kinds),
        Err(ManifestRegistryError::Registry(
            ManifestError::DuplicateId { .. }
        ))
    ));

    let mut duplicate_name = base_manifest();
    duplicate_name.id = ChipId(9);
    assert!(matches!(
        registry.register(duplicate_name, &schema, &kinds),
        Err(ManifestRegistryError::Registry(
            ManifestError::DuplicateName { .. }
        ))
    ));

    let mut duplicate_kind = base_manifest();
    duplicate_kind.id = ChipId(10);
    duplicate_kind.chip_name = "AnotherChip";
    assert!(matches!(
        registry.register(duplicate_kind, &schema, &kinds),
        Err(ManifestRegistryError::Registry(
            ManifestError::DuplicateKindClaim { .. }
        ))
    ));
}

#[test]
fn registry_rejects_invalid_manifest_before_recording_it() {
    use cc_silicon_compiler::manifest::ManifestRegistryError;
    let schema = StoreSchema::foundation();
    let kinds = TaskKindRegistry::foundation();
    let mut registry = ManifestRegistry::new();
    let mut invalid = base_manifest();
    invalid.phase = ChipPhase::Latch;
    let error = registry.register(invalid, &schema, &kinds).unwrap_err();
    assert!(matches!(error, ManifestRegistryError::Validation { .. }));
    assert!(registry.is_empty());
}

#[test]
fn config_store_write_is_rejected_by_the_validator() {
    let mut manifest = base_manifest();
    manifest
        .writes
        .push(FieldPath::new(StoreId::Config, "target"));
    let found = errors(&manifest);
    assert!(found
        .iter()
        .any(|error| matches!(error, ManifestError::ConfigStoreWrite { .. })));
}

#[test]
fn group_owners_can_declare_new_store_fields() {
    let mut schema = StoreSchema::foundation();
    assert!(!schema.has_field(&FieldPath::new(StoreId::Parse, "frame.cursor")));
    schema.declare(StoreId::Parse, "frame.cursor").unwrap();
    assert!(schema.has_field(&FieldPath::new(StoreId::Parse, "frame.cursor")));
    assert!(schema.declare(StoreId::Parse, "frame.cursor").is_err());
}

#[test]
fn names_store_entry_resolves_for_group_manifests() {
    // (Names, "entries") is the intern backing label: the manifest model can
    // authorize name interning like every other append family. Registration
    // enforcement itself (the chip-keyed allowlist and any
    // store-ownership/stage-assignment errors) is manifest-track work in
    // `manifest.rs` and is not constructed here.
    assert_eq!(StoreId::Names.index(), 20);
    assert_eq!(StoreId::Names.name(), "names");
    assert_eq!(
        FieldPath::parse("names.entries"),
        Some(FieldPath::new(StoreId::Names, "entries"))
    );
    let mut schema = StoreSchema::foundation();
    assert!(!schema.has_field(&FieldPath::new(StoreId::Names, "entries")));
    schema.declare(StoreId::Names, "entries").unwrap();
    let mut manifest = base_manifest();
    manifest
        .writes
        .push(FieldPath::new(StoreId::Names, "entries"));
    assert!(validate_manifest(&manifest, &schema, &TaskKindRegistry::foundation()).is_ok());
}

#[test]
fn store_owner_allowlist_seed_holds_gate1_row_with_zero_ready_writers() {
    // Seed: the T08 fold-chip row (`/7`), the PP01 row (`/10`), the three
    // LX rows (`/11`), the PA row (`/12`), and the six TY rows (`/13`).
    assert_eq!(STORE_OWNER_ALLOWLIST.len(), 12);
    // `tasks.ready` (`Tasks`, `"queue.ready"`) gets zero allowlisted chip
    // writers, now and for every future seed this test guards.
    assert!(
        !STORE_OWNER_ALLOWLIST
            .iter()
            .any(|&(_, store, field, _)| store == StoreId::Tasks && field == "queue.ready"),
        "tasks.ready must never gain an allowlisted chip writer"
    );
}

#[test]
fn register_accepts_valid_manifests_outside_the_gate1_wave() {
    let schema = StoreSchema::foundation();
    let kinds = TaskKindRegistry::foundation();
    let mut registry = ManifestRegistry::new();
    // Wave-gated enforcement: manifests that claim no Gate 1 slice kind
    // pass untouched, so the seeded row breaks no existing validation.
    registry.register(base_manifest(), &schema, &kinds).unwrap();
}

#[test]
fn new_manifest_error_variants_render() {
    let violation = ManifestError::StoreOwnerViolation {
        chip: "NoopChip",
        store: StoreId::Tasks,
        field: "queue.ready",
        expected_kind: TaskKind::CONTROL_NOOP,
    };
    assert!(violation.to_string().contains("allowlist"));
    assert_eq!(violation.to_diagnostic().code.group, DiagGroup::Manifest);

    let unassigned = ManifestError::StageUnassigned {
        kind: TaskKind::CONTROL_NOOP,
    };
    assert!(unassigned.to_string().contains("stage"));
    assert_eq!(unassigned.to_diagnostic().code.group, DiagGroup::Manifest);

    let mismatch = ManifestError::StageLayerMismatch {
        kind: TaskKind::CONTROL_NOOP,
        stage: 2,
        layer: 7,
    };
    assert!(mismatch.to_string().contains("layer"));
    assert_eq!(mismatch.to_diagnostic().code.group, DiagGroup::Manifest);
}
