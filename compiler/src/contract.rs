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
use crate::ids::{ChipId, RecordFamily};
use crate::limits::Limits;
use crate::manifest::{is_gate1_slice_kind, StoreSchema, STAGE_ASSIGNMENT, STORE_OWNER_ALLOWLIST};
use crate::target::{CorpusPolicy, ProbeSubstrate, TargetSpec};
use crate::task::{KindStatus, StoreId, TaskGroup, TaskKindRegistry, RECORD_KINDS};

/// Frozen contract version: T01 C01-C06 foundation plus the M1 Gate 1
/// const-fold slice, the `/8` worker-integration amendment, the `/9`
/// pre-chip readiness fixes, the Wave 2 PP01 slice (`/10`), the Wave 2 LX
/// slice (`/11`), the Wave 2 PA slice (`/12`), and the Wave 2 TY slice
/// (`/13`).
pub const CONTRACT_VERSION: &str = "t01-c01-c06/17";

/// SHA-256 of the frozen schema. Recomputed by the freeze test.
///
/// This is a content fingerprint, not a cryptographic signature. It is updated
/// only by the T01 integrator when the frozen shape changes.
pub const CONTRACT_HASH: &str = "c519c5b4cf2b2963358ec9c300d25a138e28005f88b9762c608c6692a60b2043";

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
    // `/6` scheduler/commit additions (Groups A/B decisions).
    "dispatch.inflight-cleared-at-latch",
    "dispatch.cancel-precedes-budget",
    "commit.empty-proposal-fails-explicitly",
    "commit.batch-recovery-bounded-dispatch-order",
    "commit.no-running-at-latch",
    "task.progress-reinsert-exactly-once",
    "task.progress-limit-fails-task",
    "join.await-all-terminal-state-only",
    "join.no-consume-at-join",
    "result.file-enter-structural",
    "const.legal-completes-record",
    "const.non-legal-fails-no-record",
    "const.one-record-per-request",
    "sem.effect-mask-zero-only",
    "parse.cursor-via-continuation",
    "manifest.store-owner-allowlist",
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
    "bootstrap.integration-only",
    "snapshot.wire-payloads-encoded",
    "snapshot.reserved-tombstones-visible",
    "serialize.canonical-deterministic",
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
    "vf12.interpret-symbolic-m1",
    "vf12.unsupported-never-pass",
];

/// The `TaskState` variant names, in encoding order.
pub const TASK_STATE_NAMES: &[&str] = &["ready", "running", "waiting", "completed", "failed"];
/// The `ResultValue` variant names, in encoding order.
pub const RESULT_VALUE_NAMES: &[&str] = &["empty", "ack", "record", "records", "diagnostic"];
/// The `/6` M1 append seed version: inventories plus encode presence.
///
/// Full metrics/report bodies stay deferred to Part B with a re-entry
/// criterion; only scheduler-observable facts are hashed.
pub const M1_SEED_VERSION: &str = "m1-append/1";

/// Store-to-family-to-backing-arena mapping, in proposal §8 row order.
///
/// `(store name, append field, family name, backing arena)`. The `Names`
/// family maps to the `InternTable`, not an arena. Order is part of the
/// frozen seed.
pub const M1_STORE_FAMILY_ARENA: &[(&str, &str, &str, &str)] = &[
    ("sources", "spans", "span", "spans"),
    ("sources", "expansions", "expansion", "expansions"),
    ("names", "entries", "name", "intern-table"),
    ("pp", "tokens", "pp_token", "pp_tokens"),
    ("lex", "tokens", "token", "tokens"),
    ("lex", "literals", "literal", "literals"),
    ("parse", "nodes", "node", "nodes"),
    ("tasks", "continuations", "continuation", "continuations"),
    ("symbols", "scopes", "scope", "scopes"),
    ("symbols", "scope_events", "scope_event", "scope_events"),
    ("symbols", "symbols", "symbol", "symbols"),
    ("types", "records", "type", "types"),
    ("sem", "records", "sem", "sem"),
    ("constants", "records", "const", "consts"),
    ("ir", "functions", "function", "functions"),
    ("ir", "blocks", "block", "blocks"),
    ("ir", "values", "value", "values"),
    ("ir", "instructions", "instruction", "instructions"),
    ("artifacts", "fragments", "artifact", "artifacts"),
];

/// Proposal wire tags, in `PROPOSAL_NAMES` order.
pub const M1_PROPOSAL_WIRE_TAGS: &[(&str, u8)] = &[
    ("enqueue", 0),
    ("complete", 1),
    ("fail", 2),
    ("await_host", 3),
    ("store_patch", 4),
    ("append_records", 5),
    ("progress", 6),
    ("await_children", 7),
];
/// The `Proposal` variant names, in encoding order.
pub const PROPOSAL_NAMES: &[&str] = &[
    "enqueue",
    "complete",
    "fail",
    "await_host",
    "store_patch",
    "append_records",
    "progress",
    "await_children",
];
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
    "semantic",
    "unsupported",
    "internal",
];
/// The Gate 1 (`/7`) `LiteralRecord` field names, in frozen field order.
pub const LITERAL_RECORD_FIELDS: &[&str] = &[
    "token",
    "kind",
    "radix",
    "suffix",
    "value",
    "negative",
    "spelling",
    "candidate_type",
];
/// The Gate 1 (`/7`) `ConstRecord` field names, in frozen field order.
pub const CONST_RECORD_FIELDS: &[&str] = &["value", "negative"];
/// The `LiteralKind` member names (only `integer` produced in M1).
pub const LITERAL_KIND_NAMES: &[&str] = &["integer", "character", "string"];
/// The `LiteralSuffix` member names (only `none` produced in M1).
pub const LITERAL_SUFFIX_NAMES: &[&str] = &["none", "U", "L", "UL", "LL", "ULL"];
/// The `Lx08CandidateType` member names (M1-closed `{Int}`).
pub const LX08_CANDIDATE_NAMES: &[&str] = &["Int"];
/// The `ConstExprOp` member names (M1-closed `{Add}`).
pub const CONST_EXPR_OP_NAMES: &[&str] = &["add"];
/// The `RequiredKind` member names (M1-closed).
pub const REQUIRED_KIND_NAMES: &[&str] = &["integer_constant_expression"];
/// The `ConstLegality` member names.
pub const CONST_LEGALITY_NAMES: &[&str] = &["legal", "not_constant_expression", "unsupported"];

/// Frozen `/11` `PpTokenKind` names, in declaration order.
pub const PPTOKEN_KIND_NAMES: &[&str] = &["identifier", "pp_number", "punctuator", "eof"];

/// Frozen `/11` `TokenKind` names, in declaration order.
pub const TOKEN_KIND_NAMES: &[&str] = &["keyword", "identifier", "punctuator", "integer", "eof"];

/// Frozen `/11` `PpTokenRecord` fields, in declaration order.
pub const PPTOKEN_RECORD_FIELDS: &[&str] = &["kind", "span", "spelling"];

/// Frozen `/12` `NodeKind` names, in declaration order.
pub const NODE_KIND_NAMES: &[&str] = &[
    "translation_unit",
    "function_definition",
    "specifiers",
    "declarator",
    "compound",
    "return",
    "binary_add",
    "int_literal",
];

/// Frozen `/13` `TypeKind` names (`Int`/`Function` carry inline payloads).
pub const TYPE_KIND_NAMES: &[&str] = &["void", "bool", "char", "int", "function"];

/// Frozen `/13` `IntRank` names, in declaration order.
pub const INT_RANK_NAMES: &[&str] = &["short", "int", "long", "long_long"];

/// Frozen `/13` `CharKind` names, in declaration order.
pub const CHAR_KIND_NAMES: &[&str] = &["plain", "signed", "unsigned"];

/// Frozen `/13` `TypeRecord` fields, in declaration order.
pub const TYPE_RECORD_FIELDS: &[&str] = &["kind"];

/// Frozen `/13` `SymbolKind` names, in declaration order.
pub const SYMBOL_KIND_NAMES: &[&str] = &["function", "variable"];

/// Frozen `/13` `Linkage` names, in declaration order.
pub const LINKAGE_NAMES: &[&str] = &["none", "internal", "external"];

/// Frozen `/13` `StorageDuration` names, in declaration order.
pub const STORAGE_DURATION_NAMES: &[&str] = &["none", "static", "automatic", "thread", "allocated"];

/// Frozen `/13` `SymbolRecord` fields, in declaration order.
pub const SYMBOL_RECORD_FIELDS: &[&str] =
    &["name", "scope", "kind", "ty", "linkage", "storage", "decl"];

/// Frozen `/13` `ScopeKind` names, in declaration order.
pub const SCOPE_KIND_NAMES: &[&str] = &["file", "block"];

/// Frozen `/13` `ScopeRecord` fields, in declaration order.
pub const SCOPE_RECORD_FIELDS: &[&str] = &["kind", "parent", "owner"];

/// Frozen `/13` `ScopeEventKind` names, in declaration order.
pub const SCOPE_EVENT_KIND_NAMES: &[&str] = &["enter", "exit"];

/// Frozen `/13` `ScopeEventRecord` fields, in declaration order.
pub const SCOPE_EVENT_RECORD_FIELDS: &[&str] = &["scope", "kind", "at"];

/// Frozen `/14` `ValueCategory` names, in declaration order.
pub const VALUE_CATEGORY_NAMES: &[&str] = &["lvalue", "non_lvalue", "function_designator", "void"];

/// Frozen `/14` `SemRecord` fields, in declaration order.
pub const SEM_RECORD_FIELDS: &[&str] = &["node", "ty", "category", "effects"];

/// Frozen `/15` `IrOp` names, in declaration order.
pub const IR_OP_NAMES: &[&str] = &["constant", "return"];

/// Frozen `/15` `FunctionRecord` fields, in declaration order.
pub const FUNCTION_RECORD_FIELDS: &[&str] = &["symbol", "signature", "entry", "linkage"];

/// Frozen `/15` `BlockRecord` fields, in declaration order.
pub const BLOCK_RECORD_FIELDS: &[&str] = &["function", "ordinal"];

/// Frozen `/15` `ValueRecord` fields, in declaration order.
pub const VALUE_RECORD_FIELDS: &[&str] = &["ty"];

/// Frozen `/16` `SpanRecord` fields, in declaration order.
pub const SPAN_RECORD_FIELDS: &[&str] = &["source", "start", "end", "expansion"];

/// Frozen `/15` `InstructionRecord` fields, in declaration order.
pub const INSTRUCTION_RECORD_FIELDS: &[&str] = &["op", "block", "operands", "immediate", "result"];

/// Frozen `/12` `NodeRecord` fields, in declaration order.
pub const NODE_RECORD_FIELDS: &[&str] = &[
    "kind",
    "parent",
    "children",
    "first_token",
    "last_token",
    "name",
    "literal",
];

/// Frozen `/11` `TokenRecord` fields, in declaration order.
pub const TOKEN_RECORD_FIELDS: &[&str] = &["kind", "span", "name", "pp_token"];

/// Frozen `/10` total 8-variant `ArtifactKind` names, in declaration order.
pub const ARTIFACT_KIND_NAMES: &[&str] = &[
    "normalized",
    "spliced",
    "comment_free",
    "preprocessed",
    "assembly",
    "object",
    "snapshot",
    "trace",
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
        push_str_list(&mut w, ARTIFACT_KIND_NAMES);
        push_str_list(&mut w, PPTOKEN_KIND_NAMES);
        push_str_list(&mut w, TOKEN_KIND_NAMES);
        push_str_list(&mut w, PPTOKEN_RECORD_FIELDS);
        push_str_list(&mut w, TOKEN_RECORD_FIELDS);
        push_str_list(&mut w, NODE_KIND_NAMES);
        push_str_list(&mut w, NODE_RECORD_FIELDS);
        push_str_list(&mut w, TYPE_KIND_NAMES);
        push_str_list(&mut w, INT_RANK_NAMES);
        push_str_list(&mut w, CHAR_KIND_NAMES);
        push_str_list(&mut w, TYPE_RECORD_FIELDS);
        push_str_list(&mut w, SYMBOL_KIND_NAMES);
        push_str_list(&mut w, LINKAGE_NAMES);
        push_str_list(&mut w, STORAGE_DURATION_NAMES);
        push_str_list(&mut w, SYMBOL_RECORD_FIELDS);
        push_str_list(&mut w, SCOPE_KIND_NAMES);
        push_str_list(&mut w, SCOPE_RECORD_FIELDS);
        push_str_list(&mut w, SCOPE_EVENT_KIND_NAMES);
        push_str_list(&mut w, SCOPE_EVENT_RECORD_FIELDS);
        push_str_list(&mut w, VALUE_CATEGORY_NAMES);
        push_str_list(&mut w, SEM_RECORD_FIELDS);
        push_str_list(&mut w, IR_OP_NAMES);
        push_str_list(&mut w, FUNCTION_RECORD_FIELDS);
        push_str_list(&mut w, BLOCK_RECORD_FIELDS);
        push_str_list(&mut w, VALUE_RECORD_FIELDS);
        push_str_list(&mut w, INSTRUCTION_RECORD_FIELDS);
        push_str_list(&mut w, SPAN_RECORD_FIELDS);
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
        // `/6` scheduler bounds (sole per-tick dispatch bound; the redundant
        // `max_dispatches_per_tick` stays dropped).
        w.u32(self.limits.max_inflight_per_tick);
        for bound in self.limits.stage_queue_bound {
            w.u32(bound);
        }
        w.u32(self.limits.max_const_bits);
        w.u32(self.limits.max_task_progress);

        // Gate 1 (`/7`) store schema: foundation plus the two frozen
        // language append fields.
        let schema = StoreSchema::m1_slice();
        for store in StoreId::ALL {
            w.str(store.name());
            let fields = schema.fields(store);
            w.u64(fields.len() as u64);
            for field in fields {
                w.str(field);
            }
        }

        // `/6` M1 append seed: inventories plus encode presence. Full
        // metrics/report bodies stay deferred to Part B with a re-entry
        // criterion; only the scheduler-observable facts below are hashed.
        w.str(M1_SEED_VERSION);
        // Record families: (name, wire tag, ordinal) in `RecordFamily::ALL`
        // order. Tags and ordinals are separate inventories by design.
        w.u64(RecordFamily::ALL.len() as u64);
        for family in RecordFamily::ALL {
            w.str(family.name());
            w.u8(family.ordinal());
        }
        // Store-to-family-to-arena mapping, in proposal §8 row order.
        w.u64(M1_STORE_FAMILY_ARENA.len() as u64);
        for (store, field, family, arena) in M1_STORE_FAMILY_ARENA {
            w.str(store);
            w.str(field);
            w.str(family);
            w.str(arena);
        }
        // Proposal wire tags, in `PROPOSAL_NAMES` order.
        w.u64(M1_PROPOSAL_WIRE_TAGS.len() as u64);
        for &(name, tag) in M1_PROPOSAL_WIRE_TAGS {
            w.str(name);
            w.u8(tag);
        }
        // Scheduler-observable report facts (presence only; bodies deferred).
        w.str("tick-record.dispatched");
        w.str("tick-record.selected");

        // Gate 1 (`/7`) M1 slice: typed record schemas, closed enum
        // vocabularies, slice task kinds, the kind-to-stage table, and the
        // seed allowlist rows.
        push_str_list(&mut w, LITERAL_RECORD_FIELDS);
        push_str_list(&mut w, CONST_RECORD_FIELDS);
        push_str_list(&mut w, LITERAL_KIND_NAMES);
        push_str_list(&mut w, LITERAL_SUFFIX_NAMES);
        push_str_list(&mut w, LX08_CANDIDATE_NAMES);
        push_str_list(&mut w, CONST_EXPR_OP_NAMES);
        push_str_list(&mut w, REQUIRED_KIND_NAMES);
        push_str_list(&mut w, CONST_LEGALITY_NAMES);
        // Slice task kinds (foundation kinds are encoded above; only the
        // Gate 1 additions are encoded here).
        let slice_registry = TaskKindRegistry::m1_slice();
        let mut slice_count = 0u64;
        for entry in slice_registry.iter() {
            if is_gate1_slice_kind(entry.kind) {
                slice_count += 1;
            }
        }
        w.u64(slice_count);
        for entry in slice_registry.iter() {
            if !is_gate1_slice_kind(entry.kind) {
                continue;
            }
            w.u16(entry.kind.raw());
            w.str(entry.name);
            w.u8(entry.group.raw());
            w.u8(match entry.status {
                KindStatus::Frozen => 0,
                KindStatus::Reserved => 1,
                KindStatus::GroupOwned => 2,
            });
        }
        // Kind-to-stage table, in table order.
        w.u64(STAGE_ASSIGNMENT.len() as u64);
        for &(kind, stage) in STAGE_ASSIGNMENT {
            w.u16(kind.raw());
            w.u8(stage);
        }
        // Seed allowlist rows, in table order.
        w.u64(STORE_OWNER_ALLOWLIST.len() as u64);
        for &(chip, store, field, kind) in STORE_OWNER_ALLOWLIST {
            w.u16(chip_index(chip));
            w.str(store.name());
            w.str(field);
            w.u16(kind.raw());
        }

        w.finish()
    }
}

/// Chip ID index for hashing (the stable `u16` code).
const fn chip_index(chip: ChipId) -> u16 {
    chip.0
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
        "# Frozen T01 compiler contract artifact (C01-C06 + M1 Gate 1 + Wave 2 PP slice).\n\
         # Decision: docs/architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md\n\
         version={CONTRACT_VERSION}\n\
         hash={CONTRACT_HASH}\n\
         target=aarch64-unknown-linux-gnu\n\
         target_verification=unverified\n\
         probe_substrate=linux-ci-vm (planned, not provisioned)\n\
         corpus_fetch=on-demand hash-locked\n\
         reference_baseline=authorized, not available, not candidate evidence\n\
         hash_scope=normative-shapes-and-rule-ids (not source code, not semantic proof)\n\
         hash_excludes=runtime-registrations, routing-content, post-seed-declarations, chip-logic\n\
         verified_state=private-VerifiedState; attest validates caller-supplied report data and hash, not authenticity or physical provenance\n"
    )
}
