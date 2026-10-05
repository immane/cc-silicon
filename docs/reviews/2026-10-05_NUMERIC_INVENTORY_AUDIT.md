# Numeric Inventory Audit: 2026-10-05

## Status and scope

- Status: **read-only audit record**. This is not an ADR, not a contract
  acceptance, not a `/6` freeze, and not an implementation authorization. It
  selects no hash-scope option, invents no tag/ordinal/code, and changes no
  checked-in `/5` code.
- Audit date: 2026-10-05.
- Baseline: the working tree (including uncommitted and untracked docs). The
  frozen `/5` artifact is `t01-c01-c06/5`,
  `61877601386166eea24b469ad382cff354f8e3bf31a6665f2cd16c287af63bb5`
  (`compiler/contracts/CONTRACT_VERSION`), verified by
  `compiler/tests/freeze.rs`. Candidate documents referenced: the
  [M1 CDR](../tasks/CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md)
  (rev 55) and the
  [M1 Part A proposal](../tasks/M1_PART_A_CONTRACT_PROPOSAL.md)
  (rev 40).
- Scope requested: `RecordRef` wire tags 0–26, `RecordFamily` ordinals,
  `StoreId` 20 + `Names`, numeric enum discriminants, diagnostic/`ConfigError`/
  `CommitError` codes; classification **assigned / proposed / unassigned**;
  **hash membership**.
- Method: read-only inspection of `compiler/src/` and `compiler/tests/`
  (source is the evidence for `/5`; task/CDR/proposal text is evidence for the
  candidate inventory). No code, contract, manifest, task package, or CDR file
  was edited. The only file written is this audit record.
- Classification used below:
  - **assigned** — a concrete numeric value fixed by the checked-in `/5` code
    (whether or not the value is hash-covered);
  - **proposed** — a numeric value or ordering appears in the M1 candidate
    documents but is not accepted, frozen, or implemented; several are
    explicitly marked "proposed/unverified";
  - **unassigned** — no numeric value exists anywhere; reserved ranges or open
    `/6` items.

## 0. Verification evidence

Commands run for this audit (read-only with respect to repository content;
`cargo test` writes only build artifacts under `compiler/target/`):

```sh
cargo test --locked --manifest-path compiler/Cargo.toml --test freeze
# 5 passed: frozen_hash_matches_recompute, contract_version_file_matches_constants,
# target_and_probe_policy_are_recorded, normative_rules_are_identifiers..., fingerprint_changes...
grep -rn "RecordFamily" compiler/src compiler/tests   # no matches (absent)
grep -rnE '^\s+[A-Z][A-Za-z0-9]* = [0-9]+,' compiler/src | grep -v '=>'   # no matches
git diff --check                                       # clean (exit 0)
```

Facts verified directly from `/5` source:

- `compiler/src/ids.rs` declares **24** `RecordRef` variants.
- `compiler/src/snapshot.rs::push_record_ref` assigns tags **0–23** in the
  declaration order (table in §1).
- `compiler/src/task.rs` declares `RECORD_KINDS` with **24** names in the same
  order, `StoreId::ALL` with **20** entries, and `StoreId::COUNT = 20`.
- `compiler/src/contract.rs` encodes **48** `NORMATIVE_RULES`, hashes 24 record
  kind names, 20 store names, and nine enum name lists; `compiler/src/commit.rs`
  declares **20** `CommitError` variants; `compiler/src/manifest.rs` declares
  **13** `ManifestError` variants and no `StoreOwnerViolation`; six
  `DiagnosticCode::new(...)` call sites assign six `(group, code)` pairs.
- `RecordFamily` does not exist in `/5` code; no `/5` enum uses explicit Rust
  `= N` discriminants.

Limitations: line numbers may drift after this audit; the enum inventory was
collected by scanning every `pub enum` in `compiler/src/` (39 enums), but the
audit does not prove semantic completeness of any candidate contract.

## 1. `RecordRef` wire tags (0–26)

### 1.1 Assigned in `/5` (24 tags, 0–23)

Verified in `compiler/src/snapshot.rs::push_record_ref`; the variant declaration
order in `compiler/src/ids.rs` and the `RECORD_KINDS` name list both match this
order:

| tag | variant | arena label | tag | variant | arena label |
|---:|---|---|---:|---|---|
| 0 | `Source` | `sources` | 12 | `Init` | `inits` |
| 1 | `Span` | `spans` | 13 | `Function` | `functions` |
| 2 | `Expansion` | `expansions` | 14 | `Block` | `blocks` |
| 3 | `PpToken` | `pp_tokens` | 15 | `Value` | `values` |
| 4 | `Token` | `tokens` | 16 | `Instruction` | `instructions` |
| 5 | `Name` | `names` | 17 | `VReg` | `vregs` |
| 6 | `Scope` | `scopes` | 18 | `Continuation` | `continuations` |
| 7 | `Symbol` | `symbols` | 19 | `Task` | `tasks` |
| 8 | `Type` | `types` | 20 | `Result` | `results` |
| 9 | `Node` | `nodes` | 21 | `Diagnostic` | `diagnostics` |
| 10 | `Const` | `constants` | 22 | `HostRequest` | `host_requests` |
| 11 | `Layout` | `layouts` | 23 | `Artifact` | `artifacts` |

### 1.2 Proposed (append-only, tags 24–26)

Proposal §6.2 / §12 item 2: `Sem` = **24**, `ScopeEvent` = **25**,
`Literal` = **26**; tags 0–23 must not renumber. Candidate total: **27**
variants / tags 0–26. `RECORD_KINDS` is proposed to gain `sem`,
`scope_events`, `literals` (24 → 27 names). The 27-count is **proposed, not
verified**; the CDR rev-48 audit already marked the future `/6` count as not
claimed to equal the current 24.

### 1.3 Unassigned

Tags 27 and above. No candidate document assigns them.

### 1.4 Hash membership

- `/5`: the contract hash encodes the 24 **names** (`RECORD_KINDS`) but **not**
  the numeric tags. Tags exist only as hardcoded match arms in `snapshot.rs`
  and are not derived from or cross-checked against the hashed list (T01 C05
  row records this limitation; proposal OB-14/OB-34).
- Proposed `/6`: both the wire tags and the separate `RecordFamily` ordinals are
  "proposed to be hashed separately at `/6`" (proposal §6.2/§12); the rev-48
  audit requires numeric values in the `/6` seed hash.

### 1.5 Coupled update points

Adding a variant requires coordinated changes in: `ids.rs` (`RecordRef` +
`label()`), `snapshot.rs::push_record_ref`, `contract.rs` (`RECORD_KINDS`),
proposal `family()`/`make()` and `RecordDraft`, plus any exhaustive match. A
missing site is a compile error for `match`, but **not** for the hashed name
list, which is a plain slice.

## 2. `RecordFamily` ordinals

### 2.1 `/5` state

`RecordFamily` is **absent** from `compiler/src/` and `compiler/tests/`
(verified). The closest `/5` inventory is `RECORD_KINDS`: 24 names, no
ordinals, no `family()`/`make()`.

### 2.2 Proposed ordinals are NOT order-aligned with wire tags

Proposal §6.2 declares `RecordFamily` in this order (implicit ordinals 0–26):
`Source, Span, Expansion, Name, PpToken, Token, Literal, Node, Scope,
ScopeEvent, Symbol, Type, Sem, Const, Layout, Init, Function, Block, Value,
Instruction, VReg, Continuation, Task, Result, Diagnostic, HostRequest,
Artifact`.

The resulting ordinal vs the proposed wire tag of the same family:

| family | proposed ordinal | proposed wire tag | aligned |
|---|---:|---:|---|
| `Source` | 0 | 0 | yes |
| `Span` | 1 | 1 | yes |
| `Expansion` | 2 | 2 | yes |
| `Name` | 3 | 5 | no |
| `PpToken` | 4 | 3 | no |
| `Token` | 5 | 4 | no |
| `Literal` | 6 | 26 | no |
| `Node` | 7 | 9 | no |
| `Scope` | 8 | 6 | no |
| `ScopeEvent` | 9 | 25 | no |
| `Symbol` | 10 | 7 | no |
| `Type` | 11 | 8 | no |
| `Sem` | 12 | 24 | no |
| `Const` | 13 | 10 | no |
| `Layout` | 14 | 11 | no |
| `Init` | 15 | 12 | no |
| `Function` | 16 | 13 | no |
| `Block` | 17 | 14 | no |
| `Value` | 18 | 15 | no |
| `Instruction` | 19 | 16 | no |
| `VReg` | 20 | 17 | no |
| `Continuation` | 21 | 18 | no |
| `Task` | 22 | 19 | no |
| `Result` | 23 | 20 | no |
| `Diagnostic` | 24 | 21 | no |
| `HostRequest` | 25 | 22 | no |
| `Artifact` | 26 | 23 | no |

Only the first three families align. The proposal calls the two inventories
"distinct" and requires separate pinning; this audit confirms they are also
**differently ordered**, so no consumer may derive one from the other by
position. A single explicit mapping table and round-trip tests
(`family(make(f, i)) == f`, tag↔family table) are required at `/6`.

### 2.3 Count reconciliation (19 / 24 / 27 / 27)

The CDR's "19/27/27" draft counts are not all the same inventory:

| count | inventory | status |
|---:|---|---|
| 19 | `RecordDraft` variants in proposal §6.2 (18 arena-backed + `Name`) | proposed; distinct set |
| 24 | `/5` `RECORD_KINDS` names and `/5` `RecordRef` variants (tags 0–23) | assigned, verified |
| 27 | proposed `RecordRef` variants (tags 0–26) | proposed |
| 27 | proposed `RecordFamily` ordinals (0–26) | proposed, order differs from tags |

The 19 drafts omit `Source`, `Layout`, `Init`, `VReg`, `Task`, `Result`,
`Diagnostic`, and `HostRequest` (27 − 19 = 8), so the 19 figure cannot be used
as a `RecordRef` count.

### 2.4 Hash membership

- `/5`: none (type absent).
- Proposed `/6`: family ordinals proposed to be hashed separately from wire
  tags; neither is hashed today.

## 3. `StoreId` (20 + proposed `Names`)

### 3.1 Assigned in `/5` (20 stores, indices 0–19)

`StoreId::ALL` order, explicit `index()` values, and the name-encoding used by
the snapshot and the contract hash:

| index | store | index | store |
|---:|---|---:|---|
| 0 | `Config` | 10 | `Layout` |
| 1 | `Control` | 11 | `Init` |
| 2 | `Sources` | 12 | `Ir` |
| 3 | `Pp` | 13 | `Opt` |
| 4 | `Lex` | 14 | `Machine` |
| 5 | `Parse` | 15 | `Ext` |
| 6 | `Symbols` | 16 | `Tasks` |
| 7 | `Types` | 17 | `Diagnostics` |
| 8 | `Sem` | 18 | `Artifacts` |
| 9 | `Constants` | 19 | `Wires` |

Assigned facts: `ALL: [StoreId; 20]`, `COUNT = 20`, `from_index` bound `< 20`,
`StoreVersions { versions: [u64; StoreId::COUNT] }`, snapshot encodes store
**names**, and `FrozenSchema::encode` hashes the 20 names in `ALL` order.

### 3.2 Proposed (`Names`, index 20)

Proposal §12 item 1 / §6.2: `StoreId` gains **`Names` appended at the end**
(index **20**; `ALL` **21**, `COUNT` **21**, `from_index` bound 21), mapped to
the `InternTable` with field `entries`, and the store version bumps only when
at least one genuinely new name is interned. Indices 0–19 do not shift. This
is a new **store**, distinct from the already-assigned `Name` record family /
`names` arena (which is record kind 5 and has no `StoreId` today).

### 3.3 Unassigned

Indices 21 and above.

### 3.4 Hash membership and a placement question

- `/5`: the 20 store **names** are hashed (not the indices).
- Proposed: adding `Names` changes the store-name list inside
  `FrozenSchema::encode`'s current **foundation** section. The rev-50 two-tier
  model says the `/6` seed is `StoreSchema::foundation + M1AppendSchema`, with
  post-seed runtime `declare()` extensions excluded. A new closed-enum store id
  is not a runtime `declare()`, so `/6` must state explicitly whether the
  `names` store-name entry belongs to the foundation half or the append half of
  the seed; either way it must be inside the hashed seed. This is an open
  placement item, not a contradiction by itself.

## 4. Enum discriminants

### 4.1 General finding: no explicit Rust discriminants

No `/5` enum uses `Variant = N` discriminants (verified). Every numeric value
is assigned indirectly by one of: a `match` returning `u8`/`u16` (snapshot
encoders, contract encoder), an `index()`/`raw()` function, newtype constants,
or a name list. Consequences:

- numeric encodings have no single source of truth and can drift between
  duplicated match sites;
- the contract hash covers **names**, so a numeric change is invisible to the
  frozen hash (T01 C05 limitation).

Known duplication: `KindStatus` 0/1/2 is mapped twice (`contract.rs` and
`snapshot.rs`); `PatchOp` 0/1/2 is mapped twice inside `snapshot.rs` (wires and
patch log); `StoreId` index vs name are separate paths; `TaskGroup` raw is
hashed while most enum numbers are not.

### 4.2 Enums with assigned numeric values in `/5`

| enum | count | assigned values | where | contract hash |
|---|---:|---|---|---|
| `RecordRef` | 24 | wire tags 0–23 | `snapshot.rs::push_record_ref` | names only (`RECORD_KINDS`) |
| `TaskState` | 5 | 0 ready, 1 running, 2 waiting, 3 completed, 4 failed | `snapshot.rs::push_task_state` | names only |
| `ResultValue` | 5 | 0 empty, 1 ack, 2 record, 3 records, 4 diagnostic | `snapshot.rs::push_result_value` | names only |
| `Proposal` | 5 | 0 enqueue, 1 complete, 2 fail, 3 await_host, 4 store_patch | `snapshot.rs::push_wires` | names only |
| `PatchOp` | 3 | 0 append, 1 replace, 2 tombstone | `snapshot.rs` (twice) | names only |
| `KindStatus` | 3 | 0 frozen, 1 reserved, 2 group_owned | `contract.rs`, `snapshot.rs` | raw value hashed for the 4 foundation entries |
| `StoreId` | 20 | indices 0–19 | `task.rs::index()` | names only |
| `ScalarKind` | 21 | indices 0–20 | `target.rs::index()` | name+size+align+format; index not hashed |
| `TaskGroup` | 13 | raw 0–12 | `task.rs` constants | raw+name hashed |
| `TaskKind` | — | composite `group<<12 \| local`; foundation 0–3; reserved local 0–15; group owners ≥16 | `task.rs` | raw+name+group+status for the 4 foundation entries |
| `HostRequestKind` | 4 | none (name-encoded) | `snapshot.rs` | names hashed |
| `Severity` | 3 | none (name-encoded) | `snapshot.rs` | **not hashed** |
| `DiagGroup` | 8 | none (name-encoded) | `snapshot.rs` | names hashed |
| `ArtifactKind` | 5 | none (name-encoded) | `snapshot.rs` | **not hashed** |
| `JobState` | 4 | none (name-encoded) | `snapshot.rs` | **not hashed** |
| `Stage` | 11 | none (name-encoded) | `snapshot.rs` | **not hashed** |
| `ChipPhase` | 2 | none (name-encoded) | `snapshot.rs` | names hashed |
| `Capability` | 5 | none (name-encoded) | `snapshot.rs` | names hashed |
| `BackendClass` | 2 | none (name-encoded) | `snapshot.rs` | names hashed |
| `Endianness` | 2 | none (name-encoded) | contract/profile | name hashed |
| `ObjectFormat` | 3 | none (name-encoded) | contract/profile | name hashed |
| `DataModel` | 4 | none (name-encoded) | contract/profile | name hashed |
| `FloatFormat` | 5 | none (name-encoded) | contract/scalars | name hashed |
| `WideCharEncoding` | 2 | none (name-encoded) | contract | name hashed |
| `Dialect` | 10 | none (name-encoded) | config snapshot | not in contract hash |
| `OptLevel` | 6 | none (name-encoded) | config snapshot | not in contract hash |
| `OptionFlag` | 7 | none (name-encoded) | config snapshot | not in contract hash |

### 4.3 Error enums (no numeric discriminants; diagnostic mapping in §5)

`ArenaError` 4, `InternError` 2, `LimitError` 8, `SchemaError` 2,
`RegistryError` 5, `RouteError` 2, `ConfigError` 2, `CommitError` 20,
`ManifestError` 13, `ManifestRegistryError` 2, `ProbeError` 9. None is encoded
numerically in the snapshot; several map to a `DiagnosticCode` (§5).

### 4.4 Proposed numeric-bearing enums and constants

| item | proposed value(s) | status |
|---|---|---|
| `RecordRef` additions | `Sem`=24, `ScopeEvent`=25, `Literal`=26 | proposed; no numeric in `/5` |
| `RecordFamily` | implicit ordinals 0–26, order differs from tags (§2.2) | proposed |
| `StoreId::Names` | index 20, `ALL` 21 | proposed |
| `ValueCategory` | `Lvalue=0, NonLvalue=1, FunctionDesignator=2, Void=3` | user-accepted rev 46; not implemented, not hashed |
| `ArtifactKind` | total 8: existing 5 + `Normalized`, `Spliced`, `CommentFree` | names proposed; numeric encodings unassigned |
| `Proposal` additions | `AppendRecords`, `Progress`, `AwaitChildren` | proposed; wire numbers unassigned |
| `ResultValue` addition | `DraftRecords` | proposed; wire number unassigned |
| `StageId` | total kind→stage ordinal mapping | proposed to be hashed at `/6`; values unassigned |
| `LiteralKind` / `LiteralSuffix` | `{Integer, Character, String}` / `{None, U, L, UL, LL, ULL}` | M1 scope selected; numeric encodings unassigned |
| `RequiredKind` / `ConstLegality` | `IntegerConstantExpression` / `Legal, NotConstantExpression, Unsupported` | values selected; numeric encodings unassigned |
| `Lx08CandidateType` | M1 `{Int}`, symbolic/target-independent | no numeric encoding by design |
| `ConstExprOp` | `Add` (M1) | numeric encoding unassigned |
| `TaskKind` group local codes | M1 proposed ranges (see below) | proposed, not registered |

Proposed M1 task-kind local ranges (proposal §9; `local 0..=15` reserved,
foundation `0..=3` assigned in the Control group):

| group | proposed local range | group | proposed local range |
|---|---|---:|---|
| Control (0) | 16–31 | Semantic (6) | 16–47 |
| Host (1) | 16–23 | Constant/layout (7) | 16–31 |
| Preprocess (2) | 16–39 | IR (8) | 16–47 |
| Lex (3) | 16–47 | Verification (12) | 16–31 |
| Parse (4) | 16–63 | Optimize (9), Target (10), GNU/builtin (11) | no M1 proposal (unassigned) |
| Symbol/type (5) | 16–79 | | |

### 4.5 Reserved numeric sentinels (assigned)

- Every typed arena ID: `NONE = u32::MAX` (never allocated).
- `ChipId::NONE = u16::MAX`; `ROUTING_SHELL_CHIP = u16::MAX - 1`.
- `TaskKind` reserved local codes `0..=15`; foundation kinds `0..=3`
  (`control.noop`=0, `control.unsupported`=1, `control.start_job`=2,
  `control.import_source`=3); group owners start at 16.
- `DiagnosticId::NONE` is used as the failure-recovery sentinel when the
  diagnostic budget cannot commit a record.

## 5. Diagnostic, `ConfigError`, and `CommitError` codes

### 5.1 Shape

`DiagnosticCode { group: DiagGroup, code: u16 }`. The snapshot encodes the
group **name** plus the numeric `code`; the contract hash includes the eight
`DiagGroup` names but **no numeric codes**. Codes are per-family `u16` values
and are therefore a numeric inventory in their own right.

### 5.2 Assigned `/5` diagnostic codes (6 pairs)

| group | code | produced by | covers |
|---|---:|---|---|
| `Protocol` | 1 | `CommitError::to_diagnostic` | all 20 `CommitError` variants; commit-failure recovery diagnostics |
| `Task` | 2 | `RegistryError::to_diagnostic` | 5 registry errors |
| `Unsupported` | 1 | `DiagnosticDraft::unsupported` | routing: explicit unsupported kind, no handler installed, unregistered kind |
| `Target` | 1 | `ConfigError::TargetUnverified` | unverified target values |
| `Config` | 1 | `ConfigError::DuplicateOption` | duplicate semantic option |
| `Manifest` | 1 | `ManifestError::to_diagnostic` | 13 manifest errors |

`ArenaError`, `LimitError`, `InternError`, `SchemaError`, `RouteError`, and
`ProbeError` have **no direct** `DiagnosticCode`; `ArenaError`/`LimitError`
reach diagnostics only through `CommitError::Capacity`/`Limit` → `Protocol/1`.

### 5.3 `ConfigError`

- `/5`: 2 variants (`TargetUnverified`, `DuplicateOption`), codes assigned as
  above; no per-variant numeric code of its own.
- Proposed: `MaxTicksOverflow` with a stable **string** code
  `config.max_ticks_overflow` (numeric code unassigned), plus invalid
  quota/queue/fairness variants (names and codes unassigned). CDR §3.3 lists
  `MaxTicksOverflow` + quota/queue/fairness; codes are explicitly a `/6` item.

### 5.4 `CommitError`

- `/5`: 20 variants, all mapping to the single `Protocol/1` diagnostic:
  `UnknownTask`, `UnknownResult`, `ResultAlreadyConsumed`, `TaskNotRunning`,
  `DuplicateCompletion`, `InnerTaskMismatch`, `DanglingParent`, `ChipNotOwner`,
  `UnregisteredChip`, `TaskKindNotAccepted`, `TaskAttributionMismatch`,
  `PatchOwnerMismatch`, `WriteNotDeclared`, `ReadOnlyStore`,
  `UndeclaredStoreField`, `StaleVersion`, `InvalidPatchShape`,
  `TooManyProposals`, `Limit`, `Capacity`.
- Proposed: **46** additional variants (44 append/record/task/IR
  reject-before-apply, `CrossTaskWriteConflict`, `BackpressureCapacity`). The
  proposal marks their "stable codes fixed at `/6`" and gives **names only**;
  no numeric code is proposed anywhere. `StageUnassigned`/`StageLayerMismatch`
  are classified as `ManifestError`, and `DispatchBudgetExceeded`/
  `DuplicateSelection`/`SelectionBatchOverflow` as a separate pre-mutation
  dispatcher category — not `CommitError`. The single `Protocol/1` mapping
  means today's numeric code carries no variant discrimination.

### 5.5 `ManifestError` and other families

- `/5` `ManifestError`: 13 variants (listed in §0), all `Manifest/1`.
- Proposed: `StageUnassigned`, `StageLayerMismatch`, and
  `StoreOwnerViolation { chip, store, field, expected_kind }`.
- **Discrepancy found:** CDR §3.2/§3.3's `ManifestError` row lists
  `StoreOwnerViolation` in the **Current (`/5`)** cell. It is absent from
  `/5` code; it is a **proposed** variant (proposal §6.4/§13). The current cell
  should be corrected, and the proposed cell should also name
  `StoreOwnerViolation` (the row currently names only `StageUnassigned` and
  `StageLayerMismatch`).
- `ManifestRegistryError` (2), `SchemaError` (2), `RouteError` (2),
  `InternError` (2), `ProbeError` (9): no diagnostic codes assigned.

### 5.6 Unassigned

Every numeric code beyond the six assigned pairs: all proposed `CommitError`
codes, `ConfigError` quota/queue/fairness codes, `StageUnassigned`/
`StageLayerMismatch`/`StoreOwnerViolation` codes, all T03/T04/T05/T06/T07/T08/
T09/T13 diagnostics named in the task packages (for example
`ParseDepthExceeded`, `ParseCursorDidNotAdvance`, `OwnBatch` errors,
`TerminatorMissing`), and any new `DiagGroup`. No proposed numeric-code table
exists yet; the proposal uses string names ("stable codes fixed at `/6`").

## 6. Hash membership

### 6.1 What the `/5` contract hash covers (`FrozenSchema::encode`)

| item | form in hash | numeric? |
|---|---|---|
| `CONTRACT_VERSION` | string | — |
| `RECORD_KINDS` (24) | names | no tags |
| `TaskGroup` (13) | raw `u8` + name | yes + name |
| foundation `TaskKind` (4) | raw `u16` + name + group raw + status | yes + name |
| `TASK_STATE_NAMES` (5) | names | no |
| `RESULT_VALUE_NAMES` (5) | names | no |
| `PROPOSAL_NAMES` (5) | names | no |
| `PATCH_OP_NAMES` (3) | names | no |
| `CAPABILITY_NAMES` (5) | names | no |
| `BACKEND_CLASS_NAMES` (2) | names | no |
| `CHIP_PHASE_NAMES` (2) | names | no |
| `HOST_REQUEST_NAMES` (4) | names | no |
| `DIAG_GROUP_NAMES` (8) | names | no codes |
| `StoreId::ALL` (20) | names | no indices |
| `NORMATIVE_RULES` (48) | identifiers | — |
| target profile | triple/format/model/endian/ABI names + report hash | partial |
| `ScalarKind` models (21) | name + size + align + signed/format | yes + name |
| probe requirements/substrate | strings + bool | — |
| corpus policy | strings + bools | — |
| `Limits` (11 fields) | `u32`/`u64` values | yes |
| `StoreSchema::foundation()` | field names per store | no |

### 6.2 What is excluded (`CONTRACT_VERSION` token)

`hash_excludes=runtime-registrations, routing-content,
group-declared-store-fields, chip-logic`.

### 6.3 Numeric values not in the `/5` hash

Not in the `/5` **contract** hash: `RecordRef` wire tags; `StoreId` indices;
`TaskState`/`ResultValue`/`Proposal`/`PatchOp` numeric encodings; all
diagnostic numeric codes; and the `Severity`, `ArtifactKind`, `JobState`, and
`Stage` names/values (which are not hashed anywhere). `Dialect`/`OptLevel`/
`OptionFlag` are name-encoded into the config snapshot (`config_hash`), which
is separate from the contract hash. Only
`TaskGroup`/foundation-`TaskKind`/`KindStatus`/`ScalarKind` values and `Limits`
values are hashed numerically by the contract.

### 6.4 Proposed `/6` membership

- Rev 50 (`[USER]`, accepted conceptual scope): `StoreSchema::foundation +
  M1AppendSchema` is the frozen `/6` seed **participating in the `/6` contract
  hash**; post-seed runtime `declare()` extensions stay excluded and are
  captured by runtime snapshot/schema mechanisms.
- Rev 48 requirement: the `/6` must include **numeric values in the hash** and
  specify **dual-inventory encoding** (wire tags separate from `RecordFamily`
  ordinals) plus a self-consistency mechanism.
- `limits.max_const_bits`: accepted as a **hashed** `Limits` value (rev 44),
  cap/default 128, `config` rejects > 128; exact wire field/diagnostic
  code/hash encoding remain `/6`.
- Proposed pipeline limits (`max_inflight_per_tick` default 1,
  `stage_queue_bound`, `max_task_progress`; `max_inflight_total` removed in
  principle; `max_dispatches_per_tick` dropped by the rev-51 candidate): values
  and hash treatment pending.
- Placement question (same as §3.4): the new `StoreId::Names`, the new
  `RECORD_KINDS` entries (`sem`, `scope_events`, `literals`), and any new enum
  name lists are currently written into the **foundation** part of
  `FrozenSchema::encode`; `/6` must define whether these move to
  `M1AppendSchema` or stay foundation. The two-tier decision does not by itself
  answer this.

### 6.5 Enforcement

`compiler/tests/freeze.rs` verifies hash recomputation, version-file
consistency, rule-ID uniqueness, and hash sensitivity to a `Limits` change and
a version-string change. It does **not** verify numeric tag/ordinal stability
(T01 C05), which is exactly the gap the rev-48 numeric-inventory requirement
addresses.

## 7. Assigned / proposed / unassigned summary

| inventory | assigned (`/5`) | proposed (candidate) | unassigned |
|---|---|---|---|
| `RecordRef` tags | 0–23 (24 variants) | 24–26 (`Sem`, `ScopeEvent`, `Literal`) | 27+ |
| `RecordFamily` ordinals | absent | 0–26 in declaration order, not tag-aligned | any ordinal beyond the proposed set |
| `RECORD_KINDS` names | 24 | +`sem`, `scope_events`, `literals` (27) | — |
| `StoreId` | 20 stores, indices 0–19 | `Names` index 20 (`ALL` 21) | 21+ |
| `TaskGroup` | raw 0–12 | — | 13+ |
| `TaskKind` local codes | foundation 0–3; reserved 0–15 | M1 ranges per group (16–31/39/47/63/79) | all other ≥16 codes; Optimize/Target/GNU ranges |
| `TaskState` / `ResultValue` / `Proposal` / `PatchOp` | wire 0–4 / 0–4 / 0–4 / 0–2 | `ResultValue::DraftRecords`, `Proposal::{AppendRecords, Progress, AwaitChildren}` (numbers unassigned) | added-variant numbers |
| `KindStatus` | 0–2 | — | — |
| `ScalarKind` | index 0–20 | — | — |
| `ArtifactKind` | 5 variants (names) | total 8 (names; numeric unassigned) | numeric encodings |
| `ValueCategory` | absent | 0–3 (rev 46 accepted) | hash slot |
| diagnostic codes | 6 pairs (Protocol/1, Task/2, Unsupported/1, Target/1, Config/1, Manifest/1) | string-named variants; "codes fixed at `/6`" | all proposed numeric codes |
| `ConfigError` | 2 variants (codes as above) | `MaxTicksOverflow` (string code `config.max_ticks_overflow`), quota/queue/fairness variants | numeric codes |
| `CommitError` | 20 variants (all Protocol/1) | 46 additional variants | per-variant numeric codes |
| `ManifestError` | 13 variants (all Manifest/1) | `StageUnassigned`, `StageLayerMismatch`, `StoreOwnerViolation` | numeric codes |
| `Limits` | 11 hashed fields | `max_inflight_per_tick`, `stage_queue_bound`, `max_const_bits`, `max_task_progress`, optional fairness weights | exact names/defaults/codes |
| sentinels | `u32::MAX` IDs, `ChipId::NONE`, routing-shell chip, `DiagnosticId::NONE` | — | — |

## 8. Findings (open items for `/6`)

1. **Dual inventory is order-misaligned.** Only `Source`/`Span`/`Expansion`
   share an ordinal and wire tag. `/6` needs one explicit mapping table and
   round-trip tests; position-based conversion is invalid.
2. **Numeric values are absent from the `/5` hash.** The contract hashes names;
   tags, indices, and diagnostic codes are hardcoded in snapshot match arms.
   The rev-48 numeric-inclusion requirement is not yet implemented or
   specified as a concrete list.
3. **No single source of truth for enum numbers.** `KindStatus` and `PatchOp`
   maps are duplicated; `/5` has no `#[repr]`/explicit discriminants. `/6`
   should define one authoritative table per inventory.
4. **Foundation-vs-append placement is undefined** for `StoreId::Names`, the
   new `RECORD_KINDS` entries, and any new enum name lists under the rev-50
   two-tier seed model.
5. **`StoreId::from_index` hardcodes `< 20`** and must become `< 21` with
   `Names`; `StoreVersions` size follows `StoreId::COUNT` automatically.
6. **`ManifestError` current-cell error in the CDR:** `StoreOwnerViolation` is
   proposed, not `/5`; the proposed cell also omits it.
7. **The diagnostic code space is almost entirely unassigned.** Only six
   `(group, code)` pairs exist; all proposed error variants carry string names
   only. No proposed numeric-code table exists.
8. **`ValueCategory` 0–3 is accepted but has no hash slot**, and no `/5` or
   proposed location currently hashes it.
9. **Task-kind local codes:** only Control foundation 0–3 and the reserved
   0–15 band are assigned; proposed M1 ranges are unregistered. Registration
   order and name uniqueness are enforced by `TaskKindRegistry`, but no
   numeric allocation ledger exists.
10. **`ArtifactKind` numeric encoding is unassigned** even though the total
    8-variant set is accepted; snapshot uses names, contract hash does not
    include it.
11. **`RecordRef` tag 23 = `Artifact` while the proposed `RecordFamily` puts
    `Artifact` at ordinal 26** — harmless only because the inventories are
    separate; a shared constant table would prevent accidental cross-use.
12. **Counts must be machine-checked.** The verified `/5` counts are 24
    record kinds / 24 refs / 20 stores / 20 commit errors / 13 manifest
    errors / 48 rules / 6 diagnostic-code pairs; the candidate counts are
    27/27/21/… The self-consistency test required by the CDR must pin both
    sets explicitly.

## 9. Verification record and limitations

- `cargo test --locked --manifest-path compiler/Cargo.toml --test freeze`:
  **5 passed** (hash recompute equals `6187…63bb5`; version file matches;
  rule IDs unique; limits change changes the fingerprint).
- `git diff --check`: clean.
- No `cargo fmt`/`clippy`/full test suite was run; this audit changes no code,
  so those checks are not applicable to it.
- No runtime snapshot/tag-stability test exists in `/5`; this audit's tag and
  ordinal tables come from source reading, not from executing an encoder.
- This audit makes no acceptance, selects no `/6` option, and does not close
  any CDR row. The CDR remains at its recorded revision; `/5` remains current;
  M1 remains DRAFT; no `/6` freeze and no code authorization is implied.

Related parallel audits in the same review set (cross-checked for consistency,
not modified by this audit):

- [Error and Diagnostic Inventory Audit](2026-10-05_ERROR_INVENTORY_AUDIT.md)
  covers the diagnostic/`ConfigError`/`CommitError`/`ManifestError` families in
  more depth; its "20 implemented + 46 proposed" `CommitError` count and its
  coarse-code findings match §5 here.
- [Name Interning Audit](2026-10-05_NAME_INTERNING_AUDIT.md) NI-04 covers the
  proposed `StoreId::Names` / `names.entries` store-vs-table question; it agrees
  that appending the store changes the `/6` hash (§3.4 here).
- [Proposal §9 Task-Kind → Stage Mapping Audit](2026-10-05_PROPOSAL_S9_KIND_STAGE_AUDIT.md)
  covers the proposed local-code ranges and the missing stage assignments; this
  agrees that `StageId` values are unassigned (§4.4 here).
- Other parallel audits (CI gating, framework core source, fixture audits,
  record-link, sole-writer field table, type conversion) are out of this
  audit's numeric scope.
