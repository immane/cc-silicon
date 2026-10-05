use cc_silicon_compiler::codec::sha256;
use cc_silicon_compiler::contract::{
    compute_contract_hash, contract_version_file, FrozenSchema, CONTRACT_HASH, CONTRACT_VERSION,
    M1_PROPOSAL_WIRE_TAGS, M1_SEED_VERSION, M1_STORE_FAMILY_ARENA, NORMATIVE_RULES, PROPOSAL_NAMES,
};
use cc_silicon_compiler::ids::RecordFamily;
use cc_silicon_compiler::task::StoreId;

const VERSION_FILE: &str = include_str!("../contracts/CONTRACT_VERSION");

fn field(name: &str) -> &str {
    VERSION_FILE
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.trim() == name).then(|| value.trim())
        })
        .unwrap_or_else(|| panic!("CONTRACT_VERSION is missing `{name}`"))
}

#[test]
fn frozen_hash_matches_recompute() {
    assert_eq!(compute_contract_hash(), CONTRACT_HASH);
}

#[test]
fn contract_version_file_matches_constants() {
    assert_eq!(VERSION_FILE, contract_version_file());
    assert_eq!(field("version"), CONTRACT_VERSION);
    assert_eq!(field("hash"), CONTRACT_HASH);
    assert_eq!(CONTRACT_VERSION, "t01-c01-c06/30");
}

#[test]
fn target_and_probe_policy_are_recorded() {
    assert_eq!(field("target"), "aarch64-unknown-linux-gnu");
    assert_eq!(field("target_verification"), "unverified");
    assert!(field("probe_substrate").contains("not provisioned"));
    assert!(field("corpus_fetch").contains("hash-locked"));
    let baseline = field("reference_baseline");
    assert!(baseline.contains("authorized"));
    assert!(baseline.contains("not available"));
    assert!(baseline.contains("not candidate evidence"));
    assert!(field("hash_scope").contains("not source code"));
    assert!(field("hash_excludes").contains("runtime-registrations"));
    assert!(field("hash_excludes").contains("post-seed-declarations"));
    assert!(field("verified_state").contains("not authenticity"));
    assert!(field("verified_state").contains("physical provenance"));
}

#[test]
fn normative_rules_are_identifiers_and_change_the_hash_space() {
    // The rule list is non-trivial and contains no duplicates.
    assert!(NORMATIVE_RULES.len() >= 30);
    let mut sorted = NORMATIVE_RULES.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        NORMATIVE_RULES.len(),
        "rule IDs must be unique"
    );
    for rule in [
        "commit.field-scoped-manifest",
        "commit.inner-task-binding",
        "commit.enqueue-parent-integrity",
        "commit.no-mutation-before-preflight",
        "tasktree.dangling-parent-rejected",
        "task.single-transition-per-batch",
        "commit.proposal-budget-lossless",
        "routing.propagate-mutation-after-read",
        "source.content-hash-computed-internally",
        "commit.enqueue-destination-authorized",
        "commit.result-consume-error-accurate",
        "manifest.config-write-rejected",
        "manifest.registration-validated",
        "manifest.stage-assignment-enforced",
        "manifest.store-owner-wave-gated",
        "append.materialize-g1-typed",
        "append.bodies-match-records",
        "append.authorized-registered-declared",
        "commit.total-budget-unified",
        "commit.predicted-records-checked",
        "request.const-evaluate-convention",
        "request.const-fold-forwards-identical-refs",
        "request.kind-shape-strict",
        "const.budget-enforced-chip-overflow",
        "snapshot.config-encodes-all-bounds",
        "commit.transition-required-per-task",
        "join.await-all-requires-all-terminal",
        "join.idle-drains-with-closure",
        "commit.ownbatch-requires-parent",
        "append.draft-index-canonical",
        "append.patch-conflict-rejected",
        "const.binary-node-must-be-live",
        "dispatch.stage-layer-enforced",
        "worker.stateless-unit-required",
        "artifact.normalized-single-source",
        "artifact.map-mandatory-invariants",
        "artifact.total-eight-kinds",
        "pp.normalize-single-source-convention",
        "commit.artifact-materialized",
        "lex.intern-first-seen-order",
        "lex.keyword-table-membership",
        "lex.integer-decimal-only",
        "lex.token-back-link-committed",
        "commit.name-interned-lookup-first",
        "commit.token-materialized",
        "parse.tu-fixed-nine-node-tree",
        "parse.token-range-committed",
        "commit.node-materialized",
        "ty.canonical-int-reuse-scan",
        "ty.func-single-producer",
        "scope.enter-after-tu-guarded-once",
        "scope.lifecycle-worker-enforced",
        "symbol.declare-no-duplicate",
        "symbol.lookup-chain-hit-or-typed-miss",
        "ty.identity-completes-no-plan",
        "commit.type-symbol-scope-materialized",
        "se.literal-checked-nonlvalue",
        "se.binary-forwards-identical-fold",
        "se.return-identity-no-plan",
        "vf06.checked-set-complete",
        "commit.sem-materialized",
        "ir.constant-no-refold",
        "ir.return-single-terminator",
        "ir.function-single-entry",
        "commit.ir-materialized",
        "pp.splice-exact-map",
        "pp.comment-m1-scope",
        "pp.scan-maximal-munch",
        "commit.span-pptoken-materialized",
        "probe.wchar-encoding-required",
        "probe.attestation.private-state-not-provenance",
        "config.structurally-immutable",
        "bootstrap.integration-only",
        "snapshot.wire-payloads-encoded",
        "snapshot.reserved-tombstones-visible",
        "limits.every-configured-bound-enforced",
    ] {
        assert!(NORMATIVE_RULES.contains(&rule), "missing rule `{rule}`");
    }
}

#[test]
fn fingerprint_changes_with_a_normative_input() {
    let base = FrozenSchema::current();
    let base_digest = sha256(&base.encode());

    let mut changed = FrozenSchema::current();
    changed.limits.max_records_total = changed.limits.max_records_total.saturating_add(1);
    assert_ne!(base_digest, sha256(&changed.encode()));

    let mut renamed = FrozenSchema::current();
    renamed.version = "t01-c01-c06/test";
    assert_ne!(base_digest, sha256(&renamed.encode()));
}

#[test]
fn m1_seed_version_is_pinned() {
    assert_eq!(M1_SEED_VERSION, "m1-append/1");
}

#[test]
fn m1_seed_mapping_covers_every_family_exactly_once() {
    use std::collections::BTreeSet;
    // Every family appears exactly once, in RecordFamily::ALL order.
    let mut seen = BTreeSet::new();
    let mut order = Vec::new();
    for (store, _field, family, _arena) in M1_STORE_FAMILY_ARENA {
        assert!(
            StoreId::parse(store).is_some(),
            "unknown store `{store}` in seed mapping"
        );
        assert!(seen.insert(*family), "duplicate family `{family}`");
        order.push(*family);
    }
    // Every mapped family is a distinct member of the frozen inventory
    // (the 20 append families; non-appended families have no seed row).
    let all: BTreeSet<&str> = RecordFamily::ALL.iter().map(|f| f.name()).collect();
    assert_eq!(all.len(), RecordFamily::ALL.len());
    for family in &seen {
        assert!(all.contains(family), "family `{family}` outside inventory");
    }
    assert_eq!(
        seen.len(),
        20,
        "seed mapping carries the 20 proposal-§8 append families"
    );
    // Proposal wire inventory matches the frozen name list pairwise.
    assert_eq!(M1_PROPOSAL_WIRE_TAGS.len(), PROPOSAL_NAMES.len());
    for (i, (name, tag)) in M1_PROPOSAL_WIRE_TAGS.iter().enumerate() {
        assert_eq!(*name, PROPOSAL_NAMES[i]);
        assert_eq!(*tag as usize, i, "proposal wire tags are 0..8 in order");
    }
    // The frozen hash covers the seed: flipping any seed byte changes it.
    let base = FrozenSchema::current().encode();
    assert!(base.windows(11).any(|w| w == b"m1-append/1"));
}

#[test]
fn frozen_enum_names_match_implementations() {
    use cc_silicon_compiler::contract::{
        BACKEND_CLASS_NAMES, CAPABILITY_NAMES, CHIP_PHASE_NAMES, DIAG_GROUP_NAMES,
        HOST_REQUEST_NAMES, PATCH_OP_NAMES,
    };
    use cc_silicon_compiler::diagnostic::DiagGroup;
    use cc_silicon_compiler::manifest::{BackendClass, Capability, ChipPhase};

    let capabilities = [
        (Capability::Portable, "portable"),
        (Capability::Batchable, "batchable"),
        (Capability::Emulable, "emulable"),
        (Capability::DeviceSpecific, "device_specific"),
        (Capability::Experimental, "experimental"),
    ];
    assert_eq!(CAPABILITY_NAMES.len(), capabilities.len());
    for (i, (variant, name)) in capabilities.iter().enumerate() {
        assert_eq!(variant.name(), *name);
        assert_eq!(CAPABILITY_NAMES[i], *name);
    }

    let backend_classes = [
        (BackendClass::CpuReference, "cpu-reference"),
        (BackendClass::Aarch64Linux, "aarch64-linux"),
    ];
    assert_eq!(BACKEND_CLASS_NAMES.len(), backend_classes.len());
    for (i, (variant, name)) in backend_classes.iter().enumerate() {
        assert_eq!(variant.name(), *name);
        assert_eq!(BACKEND_CLASS_NAMES[i], *name);
    }

    let phases = [
        (ChipPhase::Propagation, "propagation"),
        (ChipPhase::Latch, "latch"),
    ];
    assert_eq!(CHIP_PHASE_NAMES.len(), phases.len());
    for (i, (variant, name)) in phases.iter().enumerate() {
        assert_eq!(variant.name(), *name);
        assert_eq!(CHIP_PHASE_NAMES[i], *name);
    }

    let groups = [
        (DiagGroup::Protocol, "protocol"),
        (DiagGroup::Arena, "arena"),
        (DiagGroup::Config, "config"),
        (DiagGroup::Target, "target"),
        (DiagGroup::Manifest, "manifest"),
        (DiagGroup::Task, "task"),
        (DiagGroup::Semantic, "semantic"),
        (DiagGroup::Unsupported, "unsupported"),
        (DiagGroup::Internal, "internal"),
    ];
    assert_eq!(DIAG_GROUP_NAMES.len(), groups.len());
    for (i, (variant, name)) in groups.iter().enumerate() {
        assert_eq!(variant.name(), *name);
        assert_eq!(DIAG_GROUP_NAMES[i], *name);
    }

    // `PatchOp` and `HostRequestKind` have no `name()` method; the frozen
    // lists pin declaration order instead.
    assert_eq!(PATCH_OP_NAMES, &["append", "replace", "tombstone"]);
    assert_eq!(
        HOST_REQUEST_NAMES,
        &[
            "read_source",
            "write_artifact",
            "invoke_toolchain",
            "cancel"
        ]
    );
}

#[test]
fn task_state_names_match_encoding_order() {
    use cc_silicon_compiler::contract::TASK_STATE_NAMES;
    use cc_silicon_compiler::ids::{DiagnosticId, ResultId};
    use cc_silicon_compiler::task::{TaskState, WaitSet};

    let states = [
        TaskState::Ready,
        TaskState::Running,
        TaskState::Waiting(WaitSet::default()),
        TaskState::Completed(ResultId::from_index(0)),
        TaskState::Failed(DiagnosticId::from_index(0)),
    ];
    assert_eq!(TASK_STATE_NAMES.len(), states.len());
    for (state, name) in states.iter().zip(TASK_STATE_NAMES.iter()) {
        assert_eq!(state.name(), *name);
    }
    assert_eq!(
        TASK_STATE_NAMES,
        &["ready", "running", "waiting", "completed", "failed"]
    );
}

#[test]
fn result_value_names_match_wire_tags() {
    use cc_silicon_compiler::contract::RESULT_VALUE_NAMES;
    use cc_silicon_compiler::ids::{DiagnosticId, RecordRef, SourceId};
    use cc_silicon_compiler::snapshot::result_value_tag;
    use cc_silicon_compiler::task::ResultValue;

    let values = [
        ResultValue::Empty,
        ResultValue::Ack,
        ResultValue::Record(RecordRef::Source(SourceId::from_index(0))),
        ResultValue::Records(Vec::new()),
        ResultValue::Diagnostic(DiagnosticId::from_index(0)),
    ];
    assert_eq!(RESULT_VALUE_NAMES.len(), values.len());
    for (i, value) in values.iter().enumerate() {
        assert_eq!(result_value_tag(value), i as u8);
    }
    assert_eq!(
        RESULT_VALUE_NAMES,
        &["empty", "ack", "record", "records", "diagnostic"]
    );
}

#[test]
fn proposal_names_match_wire_tags() {
    use cc_silicon_compiler::contract::PROPOSAL_NAMES;
    use cc_silicon_compiler::diagnostic::DiagnosticDraft;
    use cc_silicon_compiler::ids::{ChipId, TaskId};
    use cc_silicon_compiler::snapshot::{proposal_wire_name, proposal_wire_tag};
    use cc_silicon_compiler::task::{
        AppendBatch, ChildRef, HostRequestDraft, HostRequestKind, Payload, Proposal, StorePatch,
        TaskDraft,
    };

    let task = TaskId::from_index(0);
    let draft = TaskDraft {
        kind: cc_silicon_compiler::task::TaskKind::CONTROL_NOOP,
        payload: Payload::default(),
        owner: ChipId(0),
        parent: None,
        continuation: None,
    };
    let proposals = [
        Proposal::Enqueue(draft.clone()),
        Proposal::Complete {
            task,
            value: cc_silicon_compiler::task::ResultValue::Empty,
        },
        Proposal::Fail {
            task,
            diagnostic: DiagnosticDraft::unsupported("x"),
        },
        Proposal::AwaitHost {
            task,
            request: HostRequestDraft {
                kind: HostRequestKind::ReadSource,
                payload: Payload::default(),
            },
        },
        Proposal::StorePatch(StorePatch {
            owner: ChipId(0),
            task,
            version: 0,
            store: cc_silicon_compiler::task::StoreId::Tasks,
            field: "queue.ready",
            op: cc_silicon_compiler::task::PatchOp::Append,
            target: None,
            value: None,
        }),
        Proposal::AppendRecords {
            task,
            batch: AppendBatch::default(),
        },
        Proposal::Progress { task, ordinal: 1 },
        Proposal::AwaitChildren {
            task,
            children: Vec::<ChildRef>::new(),
        },
    ];
    assert_eq!(PROPOSAL_NAMES.len(), proposals.len());
    for (i, proposal) in proposals.iter().enumerate() {
        assert_eq!(proposal.wire_tag(), i as u8);
        assert_eq!(proposal_wire_tag(proposal), i as u8);
        assert_eq!(proposal_wire_name(i as u8).unwrap(), PROPOSAL_NAMES[i]);
    }
    assert!(proposal_wire_name(8).is_err());
}

#[test]
fn record_kinds_match_record_ref_wire_tags() {
    use cc_silicon_compiler::ids::{
        ArtifactId, BlockId, ConstId, ContinuationId, DiagnosticId, ExpansionId, FunctionId,
        HostRequestId, InitId, InstructionId, LayoutId, LiteralId, MacroId, NameId, NodeId,
        PpTokenId, RecordRef, ResultId, ScopeEventId, ScopeId, SemId, SourceId, SpanId, SymbolId,
        TaskId, TokenId, TypeId, VRegId, ValueId,
    };
    use cc_silicon_compiler::snapshot::record_ref_tag;
    use cc_silicon_compiler::task::RECORD_KINDS;

    let refs = [
        (RecordRef::Source(SourceId::from_index(0)), 0u8, "sources"),
        (RecordRef::Span(SpanId::from_index(0)), 1, "spans"),
        (
            RecordRef::Expansion(ExpansionId::from_index(0)),
            2,
            "expansions",
        ),
        (RecordRef::PpToken(PpTokenId::from_index(0)), 3, "pp_tokens"),
        (RecordRef::Token(TokenId::from_index(0)), 4, "tokens"),
        (RecordRef::Name(NameId::from_index(0)), 5, "names"),
        (RecordRef::Scope(ScopeId::from_index(0)), 6, "scopes"),
        (RecordRef::Symbol(SymbolId::from_index(0)), 7, "symbols"),
        (RecordRef::Type(TypeId::from_index(0)), 8, "types"),
        (RecordRef::Node(NodeId::from_index(0)), 9, "nodes"),
        (RecordRef::Const(ConstId::from_index(0)), 10, "constants"),
        (RecordRef::Layout(LayoutId::from_index(0)), 11, "layouts"),
        (RecordRef::Init(InitId::from_index(0)), 12, "inits"),
        (
            RecordRef::Function(FunctionId::from_index(0)),
            13,
            "functions",
        ),
        (RecordRef::Block(BlockId::from_index(0)), 14, "blocks"),
        (RecordRef::Value(ValueId::from_index(0)), 15, "values"),
        (
            RecordRef::Instruction(InstructionId::from_index(0)),
            16,
            "instructions",
        ),
        (RecordRef::VReg(VRegId::from_index(0)), 17, "vregs"),
        (
            RecordRef::Continuation(ContinuationId::from_index(0)),
            18,
            "continuations",
        ),
        (RecordRef::Task(TaskId::from_index(0)), 19, "tasks"),
        (RecordRef::Result(ResultId::from_index(0)), 20, "results"),
        (
            RecordRef::Diagnostic(DiagnosticId::from_index(0)),
            21,
            "diagnostics",
        ),
        (
            RecordRef::HostRequest(HostRequestId::from_index(0)),
            22,
            "host_requests",
        ),
        (
            RecordRef::Artifact(ArtifactId::from_index(0)),
            23,
            "artifacts",
        ),
        (RecordRef::Literal(LiteralId::from_index(0)), 24, "literals"),
        (RecordRef::Sem(SemId::from_index(0)), 25, "sem"),
        (
            RecordRef::ScopeEvent(ScopeEventId::from_index(0)),
            26,
            "scope_events",
        ),
        (RecordRef::Macro(MacroId::from_index(0)), 27, "macros"),
    ];
    assert_eq!(RECORD_KINDS.len(), refs.len());
    for (reference, tag, kind) in refs.iter() {
        assert_eq!(reference.wire_tag(), *tag);
        assert_eq!(record_ref_tag(*reference), *tag);
        assert_eq!(reference.label(), *kind);
        assert_eq!(RECORD_KINDS[*tag as usize], *kind);
    }
}

#[test]
fn record_family_make_round_trips() {
    use cc_silicon_compiler::ids::RecordRef;
    use cc_silicon_compiler::snapshot::record_ref_index;

    for family in RecordFamily::ALL {
        let reference = RecordRef::make(family, 7);
        assert_eq!(reference.family(), family);
        assert_eq!(record_ref_index(reference), 7);
        assert_eq!(RecordRef::make(family, 7), reference);
    }
    assert_eq!(RecordFamily::ALL.len(), 28);
    for (i, family) in RecordFamily::ALL.iter().enumerate() {
        assert_eq!(family.ordinal() as usize, i);
    }
}

#[test]
fn store_and_group_inventories_are_canonical() {
    use cc_silicon_compiler::task::TaskGroup;
    use std::collections::BTreeSet;

    assert_eq!(StoreId::COUNT, StoreId::ALL.len());
    assert_eq!(StoreId::COUNT, 21);
    let mut names = BTreeSet::new();
    for (i, store) in StoreId::ALL.iter().enumerate() {
        assert_eq!(store.index(), i);
        assert_eq!(StoreId::from_index(i), Some(*store));
        assert_eq!(StoreId::parse(store.name()), Some(*store));
        assert!(names.insert(store.name()), "duplicate store name");
    }
    assert_eq!(StoreId::from_index(21), None);
    assert_eq!(StoreId::parse("unknown-store"), None);

    assert_eq!(TaskGroup::ALL.len(), 13);
    let mut group_names = BTreeSet::new();
    for (i, group) in TaskGroup::ALL.iter().enumerate() {
        assert_eq!(group.raw() as usize, i);
        assert_eq!(TaskGroup::from_raw(i as u8), Some(*group));
        assert!(group_names.insert(group.name()), "duplicate group name");
    }
    assert_eq!(TaskGroup::from_raw(13), None);
}
