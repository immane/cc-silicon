// ============================================================================
// contract.rs — frozen contract artifact and content hash (T01 C05 / C06)
//
// The frozen artifact is a canonical encoding of the contract's normative
// *shape and rule identifiers*: record families, task groups and foundation
// kinds, target profile (with UNVERIFIED values by default), limits, policy,
// the foundation store schema, the normative enumerations, and the list of
// normative rule identifiers ([`NORMATIVE_RULES`]).
//
// The hash is a content fingerprint of declared shapes and rule identifiers.
// It is NOT a source-code hash and does NOT prove semantic equivalence: logic
// inside a function can change without changing any identifier here. It also
// deliberately excludes runtime registrations (registered `ManifestRegistry`
// content, the live `RoutingTable`), group-declared `StoreSchema` fields added
// after `StoreSchema::foundation`, and any chip implementation. When a
// normative enum, schema, rule, or policy changes, bump [`CONTRACT_VERSION`],
// recompute the hash, and update `contracts/CONTRACT_VERSION`.
// ============================================================================

use crate::codec::{hex32, sha256, Writer};
use crate::limits::Limits;
use crate::manifest::StoreSchema;
use crate::target::{CorpusPolicy, ProbeSubstrate, TargetSpec};
use crate::task::{KindStatus, StoreId, TaskGroup, TaskKindRegistry, RECORD_KINDS};

/// Frozen contract version for T01 C01-C06.
pub const CONTRACT_VERSION: &str = "t01-c01-c06/5";

/// SHA-256 of the frozen schema. Recomputed by the freeze test.
///
/// This is a content fingerprint, not a cryptographic signature. It is updated
/// only by the T01 integrator when the frozen shape changes.
pub const CONTRACT_HASH: &str = "61877601386166eea24b469ad382cff354f8e3bf31a6665f2cd16c287af63bb5";

/// Normative rule identifiers covered by the contract hash.
///
/// These are identifiers for declared rules, not executable logic. Encoding
/// them makes an intentional rule change visible in the hash; it does not make
/// the hash a proof that the implementation satisfies the rule.
pub const NORMATIVE_RULES: &[&str] = &[
    "target.identity.frozen",
    "target.values.unverified-until-attested",
    "target.macos-values-forbidden",
    "probe.attestation.private-state-not-provenance",
    "probe.attestation.required-fields",
    "probe.wchar-encoding-required",
    "probe.attestation.toctou-integration-responsibility",
    "storage.append-only.no-id-reuse",
    "storage.checked-access.structured-errors",
    "storage.application-scoped-vec-only",
    "source.content-hash-computed-internally",
    "limits.every-configured-bound-enforced",
    "task.enqueue.next-tick",
    "task.select.phase-enqueue-taskid",
    "task.completion.exactly-once",
    "task.single-transition-per-batch",
    "task.result.consume-once",
    "commit.result-consume-error-accurate",
    "task.reserved-local-code-range",
    "tasktree.dangling-parent-rejected",
    "commit.inner-task-binding",
    "commit.enqueue-parent-integrity",
    "commit.enqueue-destination-authorized",
    "commit.atomic-no-partial",
    "commit.no-mutation-before-preflight",
    "commit.proposal-budget-lossless",
    "commit.field-scoped-manifest",
    "commit.owner-and-task-attribution",
    "commit.task-kind-accepted",
    "commit.config-read-only",
    "commit.store-version-guard",
    "manifest.undeclared-field-rejected",
    "manifest.config-write-rejected",
    "manifest.registration-validated",
    "manifest.phase-propagation-only",
    "manifest.tests-required",
    "manifest.backend-class-rule",
    "manifest.globally-unique-kinds",
    "routing.in-bus-deterministic",
    "routing.noop-terminates",
    "routing.unsupported-fails",
    "routing.commit-failure-not-stranded",
    "routing.propagate-mutation-after-read",
    "config.structurally-immutable",
    "bootstrap.integration-only",
    "snapshot.wire-payloads-encoded",
    "snapshot.reserved-tombstones-visible",
    "serialize.canonical-deterministic",
];

/// The `TaskState` variant names, in encoding order.
pub const TASK_STATE_NAMES: &[&str] = &["ready", "running", "waiting", "completed", "failed"];
/// The `ResultValue` variant names, in encoding order.
pub const RESULT_VALUE_NAMES: &[&str] = &["empty", "ack", "record", "records", "diagnostic"];
/// The `Proposal` variant names, in encoding order.
pub const PROPOSAL_NAMES: &[&str] = &["enqueue", "complete", "fail", "await_host", "store_patch"];
/// The `PatchOp` variant names, in encoding order.
pub const PATCH_OP_NAMES: &[&str] = &["append", "replace", "tombstone"];
/// The `Capability` variant names.
pub const CAPABILITY_NAMES: &[&str] = &[
    "portable",
    "batchable",
    "emulable",
    "device_specific",
    "experimental",
];
/// The `BackendClass` variant names.
pub const BACKEND_CLASS_NAMES: &[&str] = &["cpu-reference", "aarch64-linux"];
/// The `ChipPhase` variant names.
pub const CHIP_PHASE_NAMES: &[&str] = &["propagation", "latch"];
/// The `HostRequestKind` variant names.
pub const HOST_REQUEST_NAMES: &[&str] = &[
    "read_source",
    "write_artifact",
    "invoke_toolchain",
    "cancel",
];
/// The `DiagGroup` variant names.
pub const DIAG_GROUP_NAMES: &[&str] = &[
    "protocol",
    "arena",
    "config",
    "target",
    "manifest",
    "task",
    "unsupported",
    "internal",
];

/// The static, hashable shape of the compiler contract.
#[derive(Clone, Debug)]
pub struct FrozenSchema {
    /// Contract version.
    pub version: &'static str,
    /// The complete target model, including unverified proposals.
    pub target: TargetSpec,
    /// Planned probe substrate.
    pub probe_substrate: ProbeSubstrate,
    /// Corpus policy.
    pub corpus_policy: CorpusPolicy,
    /// Default limits.
    pub limits: Limits,
}

impl FrozenSchema {
    /// Build the frozen schema from the current definitions.
    pub fn current() -> Self {
        Self {
            version: CONTRACT_VERSION,
            target: TargetSpec::aarch64_unknown_linux_gnu_unverified(),
            probe_substrate: probe_substrate(),
            corpus_policy: CorpusPolicy::t01(),
            limits: Limits::fixture(),
        }
    }

    /// Canonical bytes of the frozen shape.
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.str(self.version);

        // Record families (arena IDs).
        push_str_list(&mut w, RECORD_KINDS);

        // Task groups.
        w.u64(TaskGroup::ALL.len() as u64);
        for group in TaskGroup::ALL {
            w.u8(group.raw());
            w.str(group.name());
        }

        // Foundation task kinds.
        let registry = TaskKindRegistry::foundation();
        w.u64(registry.len() as u64);
        for entry in registry.iter() {
            w.u16(entry.kind.raw());
            w.str(entry.name);
            w.u8(entry.group.raw());
            w.u8(match entry.status {
                KindStatus::Frozen => 0,
                KindStatus::Reserved => 1,
                KindStatus::GroupOwned => 2,
            });
        }

        // Normative enumerations.
        push_str_list(&mut w, TASK_STATE_NAMES);
        push_str_list(&mut w, RESULT_VALUE_NAMES);
        push_str_list(&mut w, PROPOSAL_NAMES);
        push_str_list(&mut w, PATCH_OP_NAMES);
        push_str_list(&mut w, CAPABILITY_NAMES);
        push_str_list(&mut w, BACKEND_CLASS_NAMES);
        push_str_list(&mut w, CHIP_PHASE_NAMES);
        push_str_list(&mut w, HOST_REQUEST_NAMES);
        push_str_list(&mut w, DIAG_GROUP_NAMES);
        let store_names: Vec<&'static str> =
            StoreId::ALL.iter().map(|store| store.name()).collect();
        push_str_list(&mut w, &store_names);

        // Normative rule identifiers.
        push_str_list(&mut w, NORMATIVE_RULES);

        // Target profile (frozen identity) and value verification state.
        let target = &self.target;
        let profile = target.profile();
        w.str(profile.triple);
        w.str(profile.object_format.name());
        w.str(profile.data_model.name());
        w.str(profile.endianness.name());
        w.str(profile.abi_name);
        match target.verification().report_hash() {
            Some(report_hash) => {
                w.u8(1);
                w.raw(&report_hash);
            }
            None => w.u8(0),
        }
        for scalar_kind in crate::target::ScalarKind::ALL {
            let scalar = target.scalar(scalar_kind);
            w.str(scalar_kind.name());
            w.u8(scalar.size);
            w.u8(scalar.align);
            match scalar.signed {
                Some(signed) => {
                    w.u8(1);
                    w.bool(signed);
                }
                None => w.u8(0),
            }
            match scalar.float {
                Some(format) => {
                    w.u8(1);
                    w.str(format.name());
                }
                None => w.u8(0),
            }
        }
        let abi = target.abi();
        w.str(abi.name);
        w.u8(abi.gp_arg_regs);
        w.u8(abi.fp_arg_regs);
        w.u8(abi.stack_align);
        w.bool(abi.variadic_register_save_area);
        w.str(target.wchar_encoding().name());

        // Probe requirements and substrate.
        push_str_list(&mut w, crate::target::REQUIRED_PROBE_FIELDS);
        w.str(self.probe_substrate.kind);
        w.bool(self.probe_substrate.available);
        w.str(self.probe_substrate.notes);

        // Corpus policy.
        w.str(self.corpus_policy.fetch);
        w.bool(self.corpus_policy.hash_locked);
        w.bool(self.corpus_policy.reference_baseline_authorized);
        w.bool(self.corpus_policy.reference_baseline_available);
        w.bool(self.corpus_policy.reference_counts_as_candidate_evidence);

        // Limits.
        w.u32(self.limits.max_records_per_arena);
        w.u64(self.limits.max_records_total);
        w.u64(self.limits.max_source_bytes);
        w.u32(self.limits.max_intern_entries);
        w.u64(self.limits.max_intern_bytes);
        w.u32(self.limits.max_queue_len);
        w.u64(self.limits.max_tasks_total);
        w.u32(self.limits.max_task_depth);
        w.u32(self.limits.max_diagnostics);
        w.u64(self.limits.max_ticks);
        w.u32(self.limits.max_proposals_per_tick);

        // Foundation store schema.
        let schema = StoreSchema::foundation();
        for store in StoreId::ALL {
            w.str(store.name());
            let fields = schema.fields(store);
            w.u64(fields.len() as u64);
            for field in fields {
                w.str(field);
            }
        }

        w.finish()
    }
}

fn push_str_list(w: &mut Writer, values: &[&str]) {
    w.u64(values.len() as u64);
    for value in values {
        w.str(value);
    }
}

/// The frozen probe substrate plan: Linux CI/VM, not yet available.
pub const fn probe_substrate() -> ProbeSubstrate {
    ProbeSubstrate {
        kind: "linux-ci-vm",
        available: false,
        notes: "Planned substrate for the AArch64 GNU/Linux target probe; not yet provisioned.",
    }
}

/// Hex SHA-256 of the current frozen schema.
pub fn compute_contract_hash() -> String {
    hex32(&sha256(&FrozenSchema::current().encode()))
}

/// The full text of `contracts/CONTRACT_VERSION`.
pub fn contract_version_file() -> String {
    format!(
        "# Frozen T01 compiler contract artifact (C01-C06).\n\
         # Decision: docs/architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md\n\
         version={CONTRACT_VERSION}\n\
         hash={CONTRACT_HASH}\n\
         target=aarch64-unknown-linux-gnu\n\
         target_verification=unverified\n\
         probe_substrate=linux-ci-vm (planned, not provisioned)\n\
         corpus_fetch=on-demand hash-locked\n\
         reference_baseline=authorized, not available, not candidate evidence\n\
         hash_scope=normative-shapes-and-rule-ids (not source code, not semantic proof)\n\
         hash_excludes=runtime-registrations, routing-content, group-declared-store-fields, chip-logic\n\
         verified_state=private-VerifiedState; attest validates caller-supplied report data and hash, not authenticity or physical provenance\n"
    )
}
