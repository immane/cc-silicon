# T01: Compiler Bus and Protocol Freeze (Prerequisite for Parallel Implementation)

Owner: contract integrator. The artifacts must first be compilable and have a schema/fixture; other LLMs must not each invent their own CompilerBus or duplicate a type system.

## 1. CPU Storage Extension Decision (Approved)

Status: **approved** on 2026-10-04; see
[ADR-0001](../architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md). The extension is
application-scoped: the root framework specification is unchanged, and its
fixed-array / no-heap rule applies to bus data, not to the whole crate. The
compiled foundation (C01–C06) lives in `compiler/`;
its frozen artifact version/hash is in `compiler/contracts/CONTRACT_VERSION`
and `compiler/src/contract.rs`. Language chips and a working compiler are still
not implemented and are not claimed.

The original paradigm specification/blueprint requires the bus to use only fixed arrays and be heapless. A general-purpose compiler needs non-fixed-length input; the following explicit extension is proposed:

- Semantic state still resides entirely in the CompilerBus; the arena is the bus's data storage, not a private backend cache.
- The CPU reference permits append-only `Vec` arenas, intern tables, and work queues; logical records reference each other only through stable newtype IDs, not `Rc/Arc/RefCell/Mutex` or an implicit pointer object graph.
- Map iteration must not determine scheduling, symbol output, or ID allocation; stable insertion order/sorting guarantees consistency across the same snapshot. An address must not be used as a semantic ID.
- Dynamic allocation is a public capability; **it is no longer claimed that the compiler is heapless per tick or directly hardware-synthesizable**. The framework's bus staying fixed-layout is not a claim about user chips.
- A bounded hardware implementation must separately define capacity and exhaustion behavior and provide a differential; it is not claimed that the CPU dynamic arena is portable across arbitrary HDL.
- Input size/task budget/arena upper bounds are specified explicitly by configuration; errors generate structured diagnostics rather than relying on panic control flow.

Acceptance: the extension is approved and linked to the application contract; the revision record is complete and does not overwrite the original framework specification to fake consistency. If the extension is rejected, first freeze a fixed-capacity profile and rewrite all tasks around bounded storage.

## 2. Register Partitions and Lifetimes

| Partition | Content/Permissions | Lifetime |
|---|---|---|
| `config` | target, dialect, option semantics, limits; read-only after initialization | entire job |
| `control` | phase, tick, job state, budget, selected_task | across ticks |
| `sources` | file bytes, source locations, macro expansion provenance; imported via Host responses | job |
| `pp` | pp tokens, macros, include/conditional/expansion stacks | TU |
| `lex` | C tokens, literal records, cursor | TU |
| `parse` | parse frames, declarators, AST, scope requests | TU |
| `symbols` | four namespaces, declarations, linkage/storage duration, scope tree | TU |
| `types` | canonical types, qualifiers, compat/composite results | TU |
| `sem` | typed AST, value category, effects, checked facts | TU |
| `constants` | fixed-width integers/target floating point/symbol address constants | TU |
| `layout` | size/alignment/member/bitfield/VLA descriptors | TU/function |
| `init` | initialization trees, byte/relocation plans, runtime init plans | TU |
| `ir` | functions/blocks/values/ops/CFG, version counters | TU/function |
| `opt` | analyses, worklists, rewrite proposals/version guards | function/pass |
| `machine` | MI, vregs, abi plans, liveness, frame, assembly fragments | function/TU |
| `ext` | GNU/builtin/asm handling records | TU/function |
| `tasks` | queue, parent/child, continuation, completed results | across ticks until consumed |
| `diagnostics` | ordered errors/warnings, locations, associated tasks | job |
| `artifacts` | PP/IR/asm output data; Host persists to disk | job |
| `wires` | current tick proposals, signals, commit slots | reset at tick start |

Multiple TUs create their frontend stores independently; external linking does not share mutable symbols. Multiple stores may share strings via read-only references/IDs, but the semantic owner is explicit.

## 3. IDs and Records

Must provide `SourceId, SpanId, ExpansionId, PpTokenId, TokenId, NameId, ScopeId, SymbolId, TypeId, NodeId, ConstId, LayoutId, InitId, FunctionId, BlockId, ValueId, InstructionId, VRegId, TaskId, ResultId, DiagnosticId`, as well as needed target record IDs.

For each ID, define the owning arena, the invalid-value policy, checked access, and TU/job ownership. Deletion does not reuse an ID that has already existed; IR tombstones + versions avoid stale rewrites. String intern/arena functions are only mechanical storage and do not hide language rules. A serialized snapshot contains all observable stores; pointer/cache addresses do not participate in the hash.

Integer values use target bit-width + bit-pattern + signedness; constant arithmetic may use explicit wide integers or a reliable software arithmetic library, not host overflow behavior. Floating-point records use format/raw bits + rounding rules; host f64 must not be used to implement all C floating point. **Part A / symbolic qualifier:** the M1 Part A frontend type/constant records carry symbolic rank + signedness with no target width or bit-pattern (concrete width/bit-pattern is Part B/probe-gated); the target-width wording above describes the frozen full-contract model, not the M1 symbolic records.

## 4. Task and Completion Protocol

The following are the semantic shapes that must be implemented and frozen; they are not the existing Rust API:

```text
Task { id, kind, payload, owner, parent, continuation, state }
TaskState = Ready | Running | Waiting(child/request_ids) | Completed(result_id) | Failed(diag_id)
Result = tagged payload (token/node/type/const/layout/IR/ABI/etc), matching the task kind
Proposal = Enqueue | Complete | Fail | AwaitHost | StorePatch (including owner/version)
```

- External input pins = source responses, job control, and target fixed-configuration references injected in the current tick; wall-clock time does not participate in language semantics.
- The control chip selects ready tasks; the motherboard routes fixedly to the chip/layers corresponding to that kind; workers do not call each other.
- Each worker reads `tasks.active` + the exact fields pointed to by the payload's IDs; it writes its own output slot/records first, then proposes completion. All proposals for the current tick are recorded with wires; the commit chip writes persistent queues/results in the last layer before latching.
- `Bus::latch` only handles edges, the post-commit control snapshot, and the tick count; it does not covertly perform type checking, AST construction, or optimization.
- A continuation must record the production state, cursor, operands, scope, and awaited child IDs. C recursive expressions use explicit frames; the entire parser must not be hidden behind a "recursive pure helper".
- A task newly enqueued in the current tick is by default executable in the next tick, avoiding dynamic same-tick re-entry. The ordering rule is fixed as `(phase priority, enqueue ordinal, TaskId)`; results are consumed exactly-once. Callbacks are prohibited.
- Service requests such as scope lookup, type conversion, and layout share a typed protocol rather than thread RPC. A worker that is not complete and has no Wait/Progress/Fault is a protocol error.
- The single runtime is sequential CPU first and does not write the bus concurrently. Parallel LLM development ≠ parallel compiler runtime. Future parallelism requires conflict checking and a deterministic commit order.

Unified interpretation of record writes: "writing a store/generating a record" in the catalog means generating a patch/append proposal for that store, submitted by CT06; a worker must not bypass commit to directly modify a shared arena. When a proposal needs to reference a new record, C01/C03 must freeze a deterministic reserved-ID or local-reference relocation protocol, and accept "no partial commit on verification failure". Task output slots and frames are likewise explicitly owned; interface implementations must not each choose between the two incompatible strategies of direct write and deferred commit.

### 4.1 Rust Interface Constraint Grading

The `RestrictedChip` path already landed in the `cc-silicon` base crate requires chips to `compute(&self, &Input) -> Output`: the chip has no Bus parameter, reads only the input projection, and returns only a typed proposal; `silicon_chip!` accepts only unit struct declarations and generates a ZST const assert; `ChipAdapter`/`ProjectedChip` are used by the application to project the Bus into input and apply proposals; `Motherboard::install_projected` is used to integrate with the existing motherboard. The old `LogicChip` interface is retained for compatibility, but new chips in the compiler application must use RestrictedChip. This API does not mean the compiler CompilerBus/task protocol is already implemented or approved.

Graded constraints and gaps:

| Rule | Mechanism | Guarantee Scope/Residual Risk |
|---|---|---|
| chip has no instance state | macro generates only a unit struct + compile-time size=0 assertion | only the macro declaration path is used; calling external globals may still hide state |
| chip cannot directly touch the bus | the RestrictedChip signature has no bus parameter beyond pins | enforced on the chip body; the Adapter is the trust boundary |
| pins/projection read-only | `&Input` and compile-fail doctest | guaranteed within Rust borrowing; internal global side effects are checked separately |
| chip may only propose, not commit | Output associated type; the adapter commits separately | the Output itself may be mistakenly designed by the application as an overly broad type; a field-scoped proposal is needed |
| adapter exact read/write set | per-chip projection/adapter + manifest + trace audit | an ordinary trait cannot prove which bus fields the adapter reads/writes; later needs code-generated private views/setters or structured patches |
| does not call other chips | RestrictedChip holds no scheduler; static lint/dependency graph | can be called indirectly through public global functions/closures; lint is not a formal proof |
| I/O, clock, env, randomness prohibited | crate dependency whitelist + AST call lint + review | general Rust APIs can be reached through indirect dependencies; this cannot be proven by traits alone |
| determinism | replay/property/differential tests | test evidence rather than a mathematical proof; the inputs and the execution path under test must be bounded |

The subsequent compiler application needs a strict crate boundary: chip modules are prohibited from depending on `std::fs/process/env/time/thread/net`, OS/runtime crates, and external compiler calls; a separate Host crate holds IO permissions. Lint should be based on the Rust AST/HIR/compiler lint or a closed dependency export, rather than only grepping keywords; an unknown macro/indirect call must report unknown and block strict CI. The strongest field-level isolation should be generated by the schema as a dedicated Input view and typed write-set patch, avoiding a hand-written Adapter that obtains the full `&mut Bus`. Strict Rust APIs and tests are partial mechanical guarantees of the invariants; a complete pure-function proof is not claimed.

## 5. Per-Chip Table Inheritance Rules

In each row of T02–T13, `Input → Output` defines the task kind and result semantics. An input request contains at least `TaskId`, scope/span/mode, and the explicitly stated IDs; the fields follow the typed records generated by T01.

**Default read set**: `tasks.active.{id,kind,payload}`, `config.{target,dialect,options}` (declared only when the rule requires it), and the specified record fields of the corresponding table input IDs. **Default write set**: the record append area/specified patch fields owned by this chip + `wires.proposals[ChipId]` + the output slot dedicated to this task. Diagnostics are submitted through `Fail/DiagnosticProposal`; the global phase must not be changed arbitrarily.

The store envelope declared by each task package is the upper bound of permissions, not "permission to read/write the entire store". Before implementation, each chip must refine its input IDs into a field list; if it cannot be refined, pause and find the integrator to freeze the interface. A result may contain requests for child services; without receiving the result, it must not prematurely report complete.

All workers: `phase=propagation`, `deterministic=true`. Language-independent/frontend/IR chips are `category=emulable, backend_class=cpu-reference` (the dynamic arena is not claimed to be portable across arbitrary backends); target chips are `device_specific, backend_class=aarch64-linux`. A chip may be marked batchable only after optimization batching, together with equivalence evidence. Control and validation follow their task package agreements.

## 6. Target and Language Model Must Be Frozen

- sizeof/alignment for all scalars (including `long double`, `__int128`), signed char, endianness, pointer representation, bitfield rules, and struct/union layout.
- AAPCS64 general/FP argument registers, HFA/HVA, aggregate return, variadic, stack, TLS, symbol visibility, and PIC.
- `-std`/GNU mode, old C compatibility, `-fwrapv`/signed overflow, strict aliasing, floating environment/fast-math, packing, and common/no-common.
- Literal candidate type list, enum underlying selection, wide character encoding, and execution character set.
- Provide a reproducible target probe fixture, but compile chips do not dynamically read the host ABI.

Frozen identity (2026-10-04): `aarch64-unknown-linux-gnu`, ELF, LP64,
little-endian, AAPCS64. Only the identity is frozen; every concrete scalar,
`long double`, and ABI value remains **UNVERIFIED** until an actual probe runs
on the planned (not yet provisioned) Linux CI/VM substrate. The corpus is
fetched on demand with a hash lock. A reference-only DejaGnu baseline is
authorized as an oracle but is not available and is never candidate compiler
evidence. The probe fixture and its unverified status are recorded in
`compiler/contracts/target/aarch64-linux-probe.txt`.

## 7. Work Items and Acceptance

| ID | Delivery | Acceptance |
|---|---|---|
| C01 | arena/IDs, typed records, checked access | roundtrip, stable IDs, out-of-bounds diagnostics, capacity failure |
| C02 | target/dialect/config schema | Linux target probe comparison; macOS values are not mixed in |
| C03 | task/result enums for all groups, error protocol | multi-package independent compilation fixtures; no same name with different semantics |
| C04 | SFL manifest extension: task guards, store/field paths, routing, capabilities | lint judges undeclared fields/wrong phase/missing tests; the draft must not claim to already be a parser implementation |
| C05 | snapshot/trace and deterministic serializer | replay with identical input/configuration/initial state is consistent per tick |
| C06 | minimal routing integration shell/registration generation | no-op tasks terminate normally and not-implemented tasks fail explicitly; carries no language logic |

Frozen artifacts carry a version/hash; protocol changes are published in one place through the integrator, and all dependent tasks are retested accordingly.

### 7.1 C01–C06 implementation status (2026-10-04; audit notes added 2026-10-05)

Frozen artifact: version `t01-c01-c06/6`, hash
`60935783b7b46cc62fc6fff64c532e840e7019a544055c594093d72dce0bf6d8`
(`compiler/contracts/CONTRACT_VERSION`), verified by `compiler/tests/freeze.rs`.
The hash fingerprints normative shapes and rule identifiers, not source code
and not semantic equivalence. The `/5` artifact (`t01-c01-c06/5`,
`61877601386166eea24b469ad382cff354f8e3bf31a6665f2cd16c287af63bb5`) is
preserved as history; the rows below describe `/5` as implemented, followed by
the `/6` amendment deltas.

`/6` amendment (2026-10-05; user-confirmed freeze of the §24.15–§24.19
recommendations; T01/owner co-freeze signatures still pending): C01 gains the
named reserved-ID reservation core (P1.0 inventory, 2a name plan, 2b predicted
refs, 2c link validation) with structural `AppendRecords` validation and an
explicit pending-rejection until the records track lands typed
materialization; C03 gains the five-outcome set (`AppendRecords`/`Progress`/
`AwaitChildren`), per-task empty-proposal `Fail`, bounded dispatch-order
recovery, await-all join without consumption, Progress reinsert with persisted
count/ordinal and exceedance-to-`Fail`, and the 9-field continuation (formal
T01 §4 supersession entries are catalogued, not yet signed); C04 gains
`StoreOwnerViolation`/`StageUnassigned`/`StageLayerMismatch` plus the
allowlist skeleton (per-chip rows wave-gated); C05 gains span-u64, tags 24–26,
new wire arms, per-record `encode_*` with round-trip decode, snapshot coverage
of the new reserved arenas/`in_flight`/`report`, and the `m1-append/1` seed
(inventories + `dispatched`/`selected` + encode presence; magnitudes/widths
and full metrics bodies excluded until probe-gated Part B); C06 gains the
quota-bound dispatcher with `in_flight`, cancel precedence, and the bounded
`report`. `SelectionBatchOverflow` is carried on `CommitError` although
proposal §6.2.1 classifies dispatch-count failures as dispatcher failures
(`/5` has no such carrier; the carrier stays the open T01 item from C17-9).
`CompilerConfig::new`/`CompilerBus::new` stay `pub` (narrowing deferred);
`CompilerBus::try_new` lives in `target.rs` pending relocation next to `new`.

`/7` amendment — M1 Gate 1 const-fold slice (2026-10-06; user decisions:
new amendment version, magnitude-bytes `ConstRecord` carrier, M1-closed
`{Int}` enum; branch `feat/gate1-const-fold`; item list and execution
record in [GATE_1_M1_FIRST_SLICE.md](GATE_1_M1_FIRST_SLICE.md)):
`LiteralRecord`/`ConstRecord` typed arenas with snapshot bodies;
M1-closed const enums plus `ConstantRequest` decode and
`ConstantResult` legality routing (no new `ResultValue` variant);
three frozen slice kinds in `TaskKindRegistry::m1_slice()`;
`STAGE_ASSIGNMENT` with per-kind enforcement plus stage/layer
agreement; one wave-gated allowlist row (fold chip,
`Constants/records`); `AppendRecords` materialization for
`Literal`/`Const` with 1:1 bodies, M1-subset gate, per-arena
preflight, and infallible apply; `StoreSchema::m1_slice()`; new
hashed rule IDs and enum/schema sections. New artifact
`t01-c01-c06/7` (`a56de65b…89d5c`); `/6` preserved as history.
`M1-CL-05` on real upstream artifacts stays the Wave-2 acceptance.

`/8` amendment — worker integration (review-driven; R1 auto-bump; `/7`
preserved as history): strict kind→shape `ConstantRequest` decode
(`const_eval_literal` literal-only, `const_eval_binary` binary-only,
`const_fold` either forwarded shape); unified total-record preflight
(appends join the single `ensure_total_records` sum); `AppendRecords`
authorization (registered manifest, accepted kind, declared write, schema
field); predicted-reference checks for both `Record` and `Records`
carriers; fold bit-budget enforcement as the accepted `ConstOverflow`
chip diagnostic; narrow `FoldInput` projection with pure `compute`; shell
`propagate_with`/`clock_tick_with` worker integration. New hashed rules
`append.authorized-registered-declared`,
`commit.total-budget-unified`, `commit.predicted-records-checked`,
`request.kind-shape-strict`,
`request.const-fold-forwards-identical-refs`,
`const.budget-enforced-chip-overflow`. New artifact `t01-c01-c06/8`
(`9216c594…855`); item list in
[GATE_1_M1_FIRST_SLICE.md](GATE_1_M1_FIRST_SLICE.md) §8.

`/9` amendment — pre-chip readiness fixes (review-driven; R1 auto-bump; `/8`
preserved as history): canonical config encodes all bounds
(`max_inflight_per_tick`, `stage_queue_bound`, `max_const_bits`,
`max_task_progress`); every batch task needs exactly one transition
(append/patch-only rejected before mutation); await-all requires all-terminal
children; idle ticks drain joins with bounded closure (`Joined` outcome);
`OwnBatch` requires the draft parent link; draft indexes must equal positions;
typed-append vs `StorePatch` append conflicts rejected; binary fold requires a
live node; driver enforces stage/layer agreement; workers must be unit structs
with declared mechanical reads; chip-lint scans inherent `compute` and
allowlists `BTreeMap`/`BTreeSet`/`vec!`/`format!`. New hashed rules
`snapshot.config-encodes-all-bounds`, `commit.transition-required-per-task`,
`join.await-all-requires-all-terminal`, `join.idle-drains-with-closure`,
`commit.ownbatch-requires-parent`, `append.draft-index-canonical`,
`append.patch-conflict-rejected`, `const.binary-node-must-be-live`,
`dispatch.stage-layer-enforced`, `worker.stateless-unit-required`. New artifact
`t01-c01-c06/9` (`f9539895…eafb`); regression suite `compiler/tests/c09_readiness.rs`
(14 tests); item list in [GATE_1_M1_FIRST_SLICE.md](GATE_1_M1_FIRST_SLICE.md) §9.

`/10` amendment — Wave 2 slice 1, PP01 source-normalize (R1 auto-bump; `/9`
preserved as history): `ArtifactRecord { kind, source, bytes, raw_offsets }`
(rev-44) + total 8 `ArtifactKind` with `requires_map`; mandatory-map
invariants + optional-kind empty-offset rule (rev-45/47); M1 boundary
convention frozen for the exercised scope (identity / inserted-LF zero-width /
CRLF collapse); `preprocess.normalize` kind (`group 2`, local `16`) with
`pp01_slice()` registry; stage row (`normalize → 1`) + allowlist row (`PP01`,
`Artifacts/fragments`); `AppendRecords` materialization for the `Artifact`
family (`Normalized`-only, map validation, capacity, predicted refs);
snapshot bodies + `ARTIFACT_KIND_NAMES` hash participation;
`PpNormalizeChip` worker (`PpInput`, pure `compute`, ZST, stage/layer, lint).
New hashed rules `artifact.normalized-single-source`,
`artifact.map-mandatory-invariants`, `artifact.total-eight-kinds`,
`pp.normalize-single-source-convention`, `commit.artifact-materialized`.
New artifact `t01-c01-c06/10` (`9d2479e7…81a5e9`); acceptance
`compiler/tests/c10_pp01.rs` (7 tests); item list in
[PP01_NORMALIZE_SLICE.md](PP01_NORMALIZE_SLICE.md).

`/11` amendment — Wave 2 slice 2, LX tokenize/classify/decode (R1 auto-bump; `/10` preserved as history): `PpTokenRecord`/`PpTokenKind` and
`TokenRecord`/`TokenKind` (M1-closed) with `pp.tokens`/`tokens` arenas
becoming typed on freeze; full C11 keyword table (membership); identifier +
keyword interning with four-name M1 order (NI-02 resolved); integer-only
decode with the commit-side token back-link (DOC-10); `lex.intern(16)` /
`lex.classify(17)` / `lex.decode_literal(18)` with stage-2 rows and three
allowlist rows (chips 4/5/6); `Name`/`Token` append materialization
(lookup-first interning with read-only capacity simulation, `Intern`
protocol-18 error, future-`Name` refs rejected); snapshot bodies +
`PPTOKEN/TOKEN_KIND_NAMES` and record-field lists in the hash. New hashed
rules `lex.intern-first-seen-order`, `lex.keyword-table-membership`,
`lex.integer-decimal-only`, `lex.token-back-link-committed`,
`commit.name-interned-lookup-first`, `commit.token-materialized`. New
artifact `t01-c01-c06/11` (`4484ae13…ce69`); acceptance
`compiler/tests/c11_lex.rs` (8 tests); item list in
[LX_SLICE.md](LX_SLICE.md).

`/12` amendment — Wave 2 slice 3, PA translation-unit (R1 auto-bump; `/11`
preserved as history): `NodeRecord` (7 fields) + M1-closed `NodeKind` (8
variants) with the `nodes` arena becoming typed on freeze; the fixed
nine-node M1 tree (pre-order prediction, reciprocal coherence test-pinned);
`parse.translation_unit` (`group 4`, local `16`) with `pa_slice()`
registry; stage row (`translation_unit → 2`) + allowlist row (`PA_TU_CHIP =
7`, `Parse/nodes`); `AppendRecords` materialization for the `Node` family;
snapshot bodies + `NODE_KIND_NAMES`/`NODE_RECORD_FIELDS` in the hash;
`PaTuChip` worker (`PaTuInput`, pure `compute`, ZST, stage/layer, lint).
The File-Enter edge stays deferred to T06. New hashed rules
`parse.tu-fixed-nine-node-tree`, `parse.token-range-committed`,
`commit.node-materialized`. New artifact `t01-c01-c06/12`
(`246d37cc…f12b3d`); acceptance `compiler/tests/c12_parse.rs` (5 tests);
item list in [PA_SLICE.md](PA_SLICE.md).

`/13` amendment — Wave 2 slice 4, TY scope/symbol/type (R1 auto-bump; `/12`
preserved as history): `TypeRecord`/`TypeKind` (M1-closed) + `IntRank` +
`CharKind`; `SymbolRecord` (7 fields) + `SymbolKind{Function,Object}` +
`Linkage` + `StorageDuration`; `ScopeRecord`/`ScopeEventRecord` with
owner-node identity; `Semantic` diagnostic group (conflict = 1, undeclared
= 2); nine TY kinds (locals 16–24, stage 3, chips 8–11, six allowlist rows,
`ty_slice()` registry); `Type`/`Symbol`/`Scope`/`ScopeEvent` append
materialization (`Intern` protocol-18, future `Name`/`ScopeEvent` refs
rejected); snapshot bodies + twelve `*_NAMES`/field lists in the hash.
Identity conversions complete plan-free; the File-Enter edge firing stays
deferred. New hashed rules `ty.canonical-int-reuse-scan`,
`ty.func-single-producer`, `scope.enter-after-tu-guarded-once`,
`scope.lifecycle-worker-enforced`, `symbol.declare-no-duplicate`,
`symbol.lookup-chain-hit-or-typed-miss`, `ty.identity-completes-no-plan`,
`commit.type-symbol-scope-materialized`. New artifact `t01-c01-c06/13`
(`0de77f06…5fa6d`); acceptance `compiler/tests/c13_ty.rs` (8 tests); item
list in [TY_SLICE.md](TY_SLICE.md).

`/14` amendment — Wave 2 slice 5, SE semantic-check + VF06 (R1 auto-bump;
`/13` preserved as history): `SemRecord` (4 fields, no `conversions` field
at slice scope) + `ValueCategory` (4 names) + `EffectMask(u32)`; M1 identity
is plan absence; binary two-phase handoff through a real `const_fold`
child; `semantic.literal_expr(18)` / `binary_expr(19)` / `return_stmt(20)`
plus `verification.typed_invariant(16)`, all stage 4, chips 12–15, three
allowlist rows; `Sem` append materialization; `Semantic` diagnostic group
(conflict = 1, undeclared = 2); snapshot bodies + `VALUE_CATEGORY_NAMES` /
`SEM_RECORD_FIELDS` in the hash. New hashed rules
`se.literal-checked-nonlvalue`, `se.binary-forwards-identical-fold`,
`se.return-identity-no-plan`, `vf06.checked-set-complete`,
`commit.sem-materialized`. New artifact `t01-c01-c06/14`
(`881f9a3f…19d44`); acceptance `compiler/tests/c14_se.rs` (9 tests); item
list in [SE_SLICE.md](SE_SLICE.md).

`/15` amendment — Wave 2 slice 6, IR function lowering (R1 auto-bump;
`/14` preserved as history): `FunctionRecord`/`BlockRecord`/`ValueRecord`/
`InstructionRecord` + M1-closed `IrOp{Constant, Return}` with the four IR
arenas becoming typed on freeze; the exactly-one-`Const` handoff rule;
`ir.function` (`group 8`, local `16`) with `ir_slice()` registry; stage
row (`function → 5`) + four allowlist rows (`IR_FUNCTION_CHIP = 16`);
`AppendRecords` materialization for the four IR families; snapshot bodies
+ `IR_OP_NAMES` and four record-field lists in the hash; `IrFunctionChip`
worker (no arithmetic, ZST, stage/layer, lint). FunctionEnd/IR28 stays
deferred. New hashed rules `ir.constant-no-refold`,
`ir.return-single-terminator`, `ir.function-single-entry`,
`commit.ir-materialized`. New artifact `t01-c01-c06/15`
(`d6c06cc4…2ef05`); acceptance `compiler/tests/c15_ir.rs` (7 tests); item
list in [IR_SLICE.md](IR_SLICE.md).

`/16` amendment — Wave 2 slice 7, PP splice/comment/scan (R1 auto-bump;
`/15` preserved as history): real line-splice with composed maps;
M1-scoped comment replacement (line/block, unterminated failure,
literal-input unsupported); maximal-munch scan with raw-remapped spans
and a single zero-width EOF; `preprocess.splice(17)` / `comment(18)` /
`scan(19)`, all stage 1, chips 17–19, four allowlist rows, `pp_slice()`
registry (29 entries); `Span`/`PpToken` append materialization (map-only
`Normalized`/`Spliced`/`CommentFree` kinds); `SPAN_RECORD_FIELDS` in the
hash. The source-bytes gap is closed: the M1 frontend runs end-to-end
without seeded PP fixtures. New hashed rules `pp.splice-exact-map`,
`pp.comment-m1-scope`, `pp.scan-maximal-munch`,
`commit.span-pptoken-materialized`. New artifact `t01-c01-c06/16`
(`04d8e4b4…f9f10`); acceptance `compiler/tests/c16_pp.rs` (8 tests);
item list in [PP_SCAN_SLICE.md](PP_SCAN_SLICE.md).

`/17` amendment — Wave 2 slice 8, VF12 symbolic interpret (R1 auto-bump;
`/16` preserved as history): read-only `Vf12Chip` modeling the M1 covered
subset (single ordinal-0 entry block, `Constant` then `Return`, `int`
value) with no arithmetic and no appends; `verification.ir_interpret`
(`VERIFICATION` local 17), stage 6, chip 20, `vf12_slice()` registry (30
entries, cumulative over `pp_slice()`); no schema change (PP-slice schema
reused); `Complete` carries `Record(Const)` of the already-committed
folded const. The M1 meaning gap is closed: the lowered IR symbolically
models return `5` on real tasks. New hashed rules
`vf12.interpret-symbolic-m1`, `vf12.unsupported-never-pass`. New artifact
`t01-c01-c06/17` (`c519c5b4…2043`); acceptance
`compiler/tests/c17_vf12.rs` (6 tests); item list in
[VF12_INTERPRET_SLICE.md](VF12_INTERPRET_SLICE.md).

`/18` amendment — Wave 2 slice 9, VF05 token-AST invariant (R1 auto-bump;
`/17` preserved as history): read-only `Vf05Chip` re-verifying the M1
syntax contract by re-walk (parent/children reciprocity, ranges contained
with ordered non-overlapping siblings, per-kind child counts over the
closed 8-kind set, `Declarator` name + `IntLiteral` literal with origin
token, unique trailing EOF); `verification.token_ast_invariant`
(`VERIFICATION` local 18), stage 2, chip 21, `vf05_slice()` registry (31
entries, cumulative over `vf12_slice()`); no schema change (PP-slice
schema reused); completes `Ack`. The token↔AST link is now checked on
real tasks. New hashed rules `vf05.syntax-ranges-ordered`,
`vf05.required-fields-complete`. New artifact `t01-c01-c06/18`
(`7ceeaee5…8256`); acceptance `compiler/tests/c18_vf05.rs` (6 tests);
item list in [VF05_SYNTAX_SLICE.md](VF05_SYNTAX_SLICE.md).

`/19` amendment — Wave 2 slice 10, VF01 store invariant (R1 auto-bump;
`/18` preserved as history): read-only `Vf01Chip` over the whole
committed snapshot with an empty payload (payload/result/parent/
continuation references resolve against dispatch-time dense bounds;
spans name committed sources within byte bounds; reserved
`layouts`/`inits`/`vregs` hold nothing); `verification.store_invariant`
(`VERIFICATION` local 19), stage 6, chip 22, `vf01_slice()` registry (32
entries, cumulative over `vf05_slice()`); no schema change (PP-slice
schema reused, 29 declared read paths); completes `Ack`. Host-request
references stay out-of-M1-scope failures; tombstone liveness stays
deferred. New hashed rules `vf01.refs-resolve`, `vf01.span-bounds`,
`vf01.reserved-unused-m1`. New artifact `t01-c01-c06/19`
(`76155ee8…476a4`); acceptance `compiler/tests/c19_vf01.rs` (7 tests);
item list in [VF01_STORE_SLICE.md](VF01_STORE_SLICE.md).

`/20` amendment — Wave 2 slice 11, PP full-token scan (R1 auto-bump;
`/19` preserved as history): in-place amendments of `PpCommentChip` (chip
18) and `PpScanChip` (chip 19) with literal/header-name-aware comment
replacement and the full C11 punctuator table plus string/char/header
tokens and dotted pp-numbers, under one shared frozen predicate
(rev-55 subset; single-tick whole-input model, no chunk persistence);
three new `PpTokenKind` variants (`StringLiteral`, `CharLiteral`,
`HeaderName`) with wire names in the hash; snapshot kind maps extended
both ways; LX classify rejects the new kinds as explicit `Unsupported`.
Same kinds/stages/registry (29 entries), same manifests, same schemas.
New hashed rules `pp.comment-literal-aware`,
`pp.scan-full-punctuators`, `pp.scan-literal-header-tokens`. New artifact
`t01-c01-c06/20` (`e5600564…f97c8b`); acceptance
`compiler/tests/c20_ppscan.rs` (8 tests, plus 4 `/16` assertions updated
to the superseded expectations); item list in
[PP_FULL_SCAN_SLICE.md](PP_FULL_SCAN_SLICE.md).

`/21` amendment — Wave 2 slice 12, PP directive dispatch (R1 auto-bump;
`/20` preserved as history): new `PpDirectiveChip` (chip 23) grouping the
committed pp-token stream into raw lines, classifying directive lines by
raw walk-back (comment-killed and mid-line `#` stay dead; same-line block
comments skipped; bare `#` is an explicit no-op), fanning out one PP26
child per `#error` line with await-all and failing fast as explicit
`Unsupported` on the frozen diagnostic taxonomy otherwise; new
`PpDiagnosticChip` (chip 24) failing every `#error` line with its joined
message (negative-only by design; the frozen join reuses the first failed
child's diagnostic, so no aggregate record is minted).
`preprocess.directive` (local 20) + `preprocess.diagnostic` (local 21),
stage 1, layers 1, `pp_directive_slice()` registry (34 entries, cumulative
over `vf01_slice()`); no schema change; no writes. New hashed rules
`pp.directive-dispatch-lines`, `pp.error-fails-message`,
`pp.diagnostic-taxonomy-frozen`. New artifact `t01-c01-c06/21`
(`fba01a2b…754b5`); acceptance `compiler/tests/c21_directive.rs`
(8 tests); item list in [PP_DIRECTIVE_SLICE.md](PP_DIRECTIVE_SLICE.md).

`/22` amendment — Wave 2 slice 13, PP conditional inclusion (R1
auto-bump; `/21` preserved as history): new `PpConditionalChip` (chip
25) tracking the conditional stack over directive lines, evaluating
`#if`/`#elif` with a chip-local exact-`i128` PP-int evaluator
(recursive descent + `defined`-frozen-`false`; PP20–PP22 folded as pure
helpers with promotion criteria), completing `Records` of active-line
refs with conditional lines fully consumed and EOF always kept;
fail-closed on malformed/unterminated/stray directives and on hard
expression errors. `preprocess.conditional` (local 22), stage 1, layer
1, `pp_conditional_slice()` registry (35 entries, cumulative over
`pp_directive_slice()`); no schema change; no writes. New hashed rules
`pp.conditional-stack`, `pp.expr-ppint-exact`,
`pp.defined-frozen-false`. New artifact `t01-c01-c06/22`
(`b8400fca…6af5dc`); acceptance `compiler/tests/c22_conditional.rs`
(8 tests); item list in [PP_CONDITIONAL_SLICE.md](PP_CONDITIONAL_SLICE.md).

`/23` amendment — Wave 2 slice 14, PP macro definitions (R1 auto-bump;
`/22` preserved as history): new `Macro` record family (wire tag 27,
ordinal 27; byte-spelling names/params so no intern prediction;
tombstones for `#undef`) with full commit/snapshot/hash pipeline; new
`PpDefineChip` (chip 26, per-line fan-out by caller, fresh-append or
PP07-await), `PpRedefineChip` (chip 27, benign-equivalence verifier),
`PpUndefChip` (chip 28, tombstones + ignore-unknown); PP19 amended to
read the table for `defined`/`#ifdef` (the `/22` frozen-`false` rule
superseded as predicted). `preprocess.macro_define` (23) +
`macro_redefine` (24) + `macro_undef` (25), stage 1, layers 1,
`pp_macro_slice()` registry (38 entries, cumulative over
`pp_conditional_slice()`); schema gains `(Pp, "macros")`; allowlist
rows for the two writers. New hashed rules `pp.macrodef-record`,
`pp.redefine-benign-rule`, `pp.undef-tombstone`,
`pp.defined-reads-table`. New artifact `t01-c01-c06/23`
(`cf8f2194…5b1ca0`); acceptance `compiler/tests/c23_macro.rs`
(10 tests); item list in [PP_MACRO_DEFINE_SLICE.md](PP_MACRO_DEFINE_SLICE.md).

`/24` amendment — Wave 2 slice 15, PP macro expansion (R1 auto-bump;
`/23` preserved as history): new `PpInvokeChip` (chip 29) fanning out
one PP12 child per top-level invocation over the active stream
(directive lines verbatim) and stitching the expanded stream, plus new
`PpSubstituteChip` (chip 30) substituting with argument prescan (raw
`#`/`##` bypass), exact `#`/`##` (frozen-scanner paste validation),
recursive blue-paint rescan with a macro-count+2 breaker, and verbatim
ID reuse; PP10/PP11/PP13/PP14/PP15 folded as pure helpers with
promotion criteria. `preprocess.macro_invoke` (26) +
`macro_substitute` (27), stage 1, layers 1, `pp_expand_slice()`
registry (40 entries, cumulative over `pp_macro_slice()`); no new
families; allowlist rows for both `Pp.tokens` writers. Record-level
correction in the same slice: `MacroRecord.function_like` added
(zero-param function-like must not expand bare) with rule
`pp.macro-function-flag`. New hashed rules `pp.invoke-fanout-stitch`,
`pp.substitute-rescan-loop`, `pp.stringify-paste-exact`,
`pp.blue-paint-guard`, `pp.macro-function-flag`. New artifact
`t01-c01-c06/24` (`8f1ba410…bfbf53`); acceptance
`compiler/tests/c24_expand.rs` (12 tests); item list in
[PP_EXPAND_SLICE.md](PP_EXPAND_SLICE.md).

`/25` amendment — Wave 2 slice 16, PP include (R1 auto-bump; `/24`
preserved as history): new `PpIncludeResolveChip` (chip 31, pure
resolver over committed sources with exact/basename matching) and
`PpIncludeEnterChip` (chip 32, single-pass stitch replacing each
`#include` line with the header's scanned tokens, everything else
verbatim, nested includes surviving for the control-loop slice);
chips stay host-passive (missing files fail `not-loaded` for the host
loop; AwaitHost/CT02 async stays T02-owned). `preprocess.
include_resolve` (local 28) + `include_enter` (29), stage 1, layers 1,
`pp_include_slice()` registry (42 entries, cumulative over
`pp_expand_slice()`); no schema change; no writes. New hashed rules
`pp.include-path-policy`, `pp.include-single-pass-stitch`. New artifact
`t01-c01-c06/25` (`71317621…30d9a`); acceptance
`compiler/tests/c25_include.rs` (8 tests); item list in
[PP_INCLUDE_SLICE.md](PP_INCLUDE_SLICE.md).

`/26` amendment — Wave 3 slice 1, PP variadic (R1 auto-bump; `/25`
preserved as history): new `PpVariadicChip` (chip 33, PP16) with two
payload shapes under one kind — stream mode fans out one single-mode
child per variadic-definition invocation behind a single `AwaitChildren`
(frozen join: first `Failed` child fails the waiter reusing its
diagnostic; stitch runs only all-`Completed`) and single mode
substitutes one variadic invocation (`...` collection, `__VA_ARGS__`,
C23 `__VA_OPT__` policy — content kept iff the tail holds at least one
token, empty tail legal — `#`/`##` with prescan, blue-paint rescan with
a macro-count+2 breaker). Non-variadic definitions with matching arity
pass through verbatim (PP09 owns them); variadic-shaped misuse fails
naming PP16; GNU `, ## __VA_ARGS__` swallowing stays unimplemented
(dialect-gated follow-up). `preprocess.variadic_macro` (local 30),
stage 1, layer 1, `pp_variadic_slice()` registry (43 entries,
cumulative over `pp_include_slice()`); no schema change; one allowlist
row (`Pp.tokens` for single-mode appends). New hashed rules
`pp.variadic-collect`, `pp.va-opt-policy`, `pp.variadic-arity`. New
artifact `t01-c01-c06/26` (`59bdf0f5…9031a006`); acceptance
`compiler/tests/c26_variadic.rs` (10 tests); item list in
[PP_VARIADIC_SLICE.md](PP_VARIADIC_SLICE.md).

`/27` amendment — Wave 3 slice 2, PP builtins (R1 auto-bump; `/26`
preserved as history): new `PpBuiltinChip` (chip 34, PP24), pure
single-task (one dispatch, at most one quota-1 `PpToken` append,
`Complete(Records)`; no children, so the frozen-join path never
applies). One builtin use expands to one synthesized token with the use
token's span: `__FILE__` spells the quoted source name, `__LINE__`
spells the physical line (PP23 logical location pending — the
projector prefers the frozen PP23 carrier once it lands),
`__COUNTER__` spells the projected base (task-associated counter
record when present, else the task-local seed 0; no counter carrier
exists on the bus, so every dispatch replays 0 until one lands),
frozen-target predefined macros read off the projected target model
(triple arch/os, ELF, LP64, little-endian, `__STDC__`,
dialect-derived `__STDC_VERSION__`) plus nothing from the host.
`__DATE__`/`__TIME__` fail as explicit `Unsupported` (replayable values
need a frozen config date/time record the config does not carry) and
any other name fails as unknown-builtin `Unsupported`; every failure
names PP24. `preprocess.macro_builtin` (local 31), stage 1, layer 1,
`pp_builtin_slice()` registry (44 entries, cumulative over
`pp_variadic_slice()`); no schema change; one allowlist row
(`Pp.tokens` for the synthesized append). New hashed rules
`pp.builtin-file-line`, `pp.builtin-counter`, `pp.builtin-target`,
`pp.builtin-date-replayable`. New artifact `t01-c01-c06/27`
(`1c4c6547…9708faa`); acceptance `compiler/tests/c27_builtin.rs`
(9 tests); item list in [PP_BUILTIN_SLICE.md](PP_BUILTIN_SLICE.md).

`/28` amendment — Wave 3 slice 3, PP line (R1 auto-bump; `/27`
preserved as history): new `PpLineChip` (chip 35, PP23), pure
single-task (one dispatch, no bus writes, `Complete(Ack)`; no children,
so the frozen-join path never applies). One directive line's pp-token
refs validate to a `LogicalLocation`: `#line number "file"?` or a GNU
`# lineno "file" flags?` marker (`#` accepts the `%:` spelling; flag
values accepted and ignored). Numbers must be all-ASCII-digit spellings
in `1..=2^31-1` (zero, overflow, and non-digit forms like `1e5` fail);
files must be `"..."` strings decoded with only `\\` and `\"` escapes;
a missing file means retain-current-file (`file: None`). The physical
source (first token's span owner) and the declared logical line/file
stay distinct; the wiring layer persists the location (no store field
lands). `preprocess.line_directive` (local 32), stage 1, layer 1,
`pp_line_slice()` registry (45 entries, cumulative over
`pp_builtin_slice()`); no schema change; no allowlist row (Ack-only,
read-only). New hashed rules `pp.line-logical`, `pp.line-gnu-marker`,
`pp.line-range`. New artifact `t01-c01-c06/28`
(`5af3f3fb…48acc71`); acceptance `compiler/tests/c28_line.rs`
(10 tests); item list in [PP_LINE_SLICE.md](PP_LINE_SLICE.md).

`/29` amendment — Wave 3 slice 4, PP pragma (R1 auto-bump; `/28`
preserved as history): new `PpPragmaChip` (chip 36, PP25), pure
single-task (one dispatch, no bus writes, `Complete(Ack)`; no children,
so the frozen-join path never applies). One pragma construct's pp-token
refs classify to a `PragmaClass`: post-`#` refs starting at Identifier
`pragma`, or the four operator tokens `_Pragma ( StringLiteral )`.
`_Pragma("...")` strings decode with quotes stripped, simple escapes
mapped, and an optional `u8`/`u`/`U`/`L` prefix tolerated; `once` is the
header-guard flag, `pack` with a `push`/`pop` operator is the
alignment-stack op (further pack arguments uninterpreted), and every
other well-formed pragma — supported-set or unknown — is opaque and
benignly ignored per C11 6.10.6p1. Malformed `_Pragma` operands (wrong
arity, non-string literal, missing quotes, raw newline/quote, dangling
backslash, non-simple escapes) fail as typed `Invalid`; wrong-dispatch
input fails as a protocol fault. The classification is returned to the
wiring layer (no store field lands). `preprocess.pragma_directive`
(local 33), stage 1, layer 1, `pp_pragma_slice()` registry (46 entries,
cumulative over `pp_line_slice()`); no schema change; no allowlist row
(Ack-only, read-only). New hashed rules `pp.pragma-once`,
`pp.pragma-pack`, `pp.pragma-unknown-ignore`. New artifact
`t01-c01-c06/29` (`c500d9ff…e019324`); acceptance
`compiler/tests/c29_pragma.rs` (9 tests); item list in
[PP_PRAGMA_SLICE.md](PP_PRAGMA_SLICE.md).

`/30` amendment — Wave 3 slice 5, PP expansion map (R1 auto-bump; `/29`
preserved as history): new `PpExpandMapChip` (chip 37, PP27), pure
single-task (one dispatch, no bus writes, `Complete(Ack)`; no children,
so the frozen-join path never applies). For every payload pp-token ref
in payload order the chip rebuilds the per-token origin chain from the
frozen `SpanRecord` links and the committed `ExpansionRecord` records:
the frame's `spelling` span names the raw form (`#` operands read this
side), the `expanded` span names the product/prescanned form (`##`
products land here), `spelling` and `expanded` stay side by side per
frame (prescan-vs-raw), `parent` linkage plus per-frame
`ordinal`/`depth` recover the blue-paint rescan nesting, and every
frame resolves to a `SpanRecord` whose `source` names the physical file
at that nesting level (nested include origins). An unexpanded token
carries no frames. Dangling links (unprojected span, dangling
expansion or parent), expansion cycles, and over-long walks fail as
typed `Invalid`; wrong dispatch fails as a protocol fault. Nothing is
persisted: no `OriginChain` carrier exists yet, so the chains are
returned through `origin_chain` / `origin_root` for the wiring layer
(no store field lands). `preprocess.expand_map` (local 34), stage 1,
layer 1, `pp_expand_map_slice()` registry (47 entries, cumulative over
`pp_pragma_slice()`); no schema change; no allowlist row (Ack-only,
read-only). New hashed rules `pp.expand-origin-chain`,
`pp.origin-paste-prescan`, `pp.origin-blue-paint`. New artifact
`t01-c01-c06/30` (`76bf628e…f2f3592`); acceptance
`compiler/tests/c30_expand_map.rs` (10 tests); item list in
[PP_EXPAND_MAP_SLICE.md](PP_EXPAND_MAP_SLICE.md).

`/31` amendment — Wave 3 slice 6, PP emit (R1 auto-bump; `/30`
preserved as history): new `PpEmitChip` (chip 38, PP28), pure
single-task (one dispatch, one `Preprocessed` artifact append,
`Complete(Record(Artifact))`; no children, so the frozen-join path
never applies). The final pp-token stream serializes to re-lexable
bytes: `Eof` tokens and consumed directive lines (an unexpanded `#`/`%:`
opening a physical line plus the rest of that line) are dropped, every
other token spelling is emitted byte-identical separated by one space
(one newline when both neighbors show a line break in their shared
source bytes), and the output closes with a terminal newline. Separation
is unconditional, so `+ +` never becomes `++` and adjacent strings stay
two tokens; no `#line`/GNU markers are emitted (PP23 owns the logical
location at the wiring layer). The output carries the primary-source (first token's span source) location map, which satisfies
`check_map` by construction (foreign-source bytes collapse zero-width,
expansion-reordered offsets clamp forward). Malformed input fails as a
typed `Fail` (protocol, `Invalid`, config, or internal — never silent).
`preprocess.emit` (local 35), stage 1, layer 1, `pp_emit_slice()`
registry (48 entries, cumulative over `pp_expand_map_slice()`); no
schema change; one allowlist row (`PP28_CHIP`, `Artifacts`,
`fragments`, `PREPROCESS_EMIT`). New hashed rules
`pp.emit-directive-strip`, `pp.emit-no-gluing`, `pp.emit-map`. New
artifact `t01-c01-c06/31` (`ce4dd422…2e2998c1`); acceptance
`compiler/tests/c31_emit.rs` (11 tests); item list in
[PP_EMIT_SLICE.md](PP_EMIT_SLICE.md).

`/32` amendment — Wave 3 slice 7, LX float (R1 auto-bump; `/31`
preserved as history): two Ack-only workers closing the float USE
(T04:17–18). `LxFloatSyntaxChip` (chip 39, LX09) validates one
committed pp-number spelling (decimal with optional `e`/`E`
exponent, hex with mandatory `p`/`P` exponent, optional
`f`/`F`/`l`/`L` suffix); `LxFloatValueChip` (chip 40, LX10) checks
the same spelling converts to its suffix-selected format
(`f`/`F` → binary32, absent → binary64) with correct
round-to-nearest-even over integer arithmetic only. Both read
exactly one `RecordRef::PpToken` of kind `PpNumber` (DOC-10),
complete `Ack`, append nothing (no `LiteralRecord`: the M1 record
stays integer-only), and fail loud (`Invalid` on malformed shapes,
`Unsupported` on integer-shaped spellings and on the deferred
binary128 `l`/`L` path). Range rides as value flags, never as
value-level failure. The frozen interchange is the spelling (no
frozen `FloatParts`/`FloatBits` carrier yet; each chip keeps its own
local shape by copy precedent). `lex.float_syntax` (local 19) and
`lex.float_value` (local 20), stage 2, layer 2,
`lx_float_slice()` registry (50 entries, cumulative over
`pp_emit_slice()`); no schema change; no allowlist rows (Ack-only).
New hashed rules `lx.float-syntax`, `lx.float-value-rounding`,
`lx.float-overflow`. New artifact `t01-c01-c06/32`
(`0dd8da06…4a6e1167`); acceptance `compiler/tests/c32_float.rs`
(12 tests); item list in [LX_FLOAT_SLICE.md](LX_FLOAT_SLICE.md).

`/33` amendment — Wave 3 slice 8, LX string (R1 auto-bump; `/32`
preserved as history): three workers closing the escape/char/string USE
(T04:19–21). `LxEscapeChip` (chip 41, LX11) validates one literal body
against the frozen escape rule (simple/octal-3/greedy-hex/UCN with
scalar-range checks) and completes `Ack`, appending nothing (no
`CodeUnits` carrier frozen yet); `LxCharChip` (chip 42, LX12) decodes
one `prefix'body'` spelling to its typed value with the frozen
multicharacter policy (prefix-width masking, big-endian concatenation,
low 32 bits) and appends one `Character` `LiteralRecord` (radix `16`,
suffix `None`, `Int` candidate, publish-time token back-link);
`LxStringChip` (chip 43, LX13) decodes one `prefix"body"` spelling to
code units plus exactly one terminating zero with the frozen
element-type/width rule (narrow 1, `u` UTF-16 with surrogate pairs, `U`
UTF-32, `L` from the frozen `wchar_t` width) and appends one `String`
`LiteralRecord` (radix `0` non-numeric marker, suffix `None`, `Int`
candidate, back-link). All three read the committed PP spelling/kind
(payload `[Token, PpToken]`, the `/11` precedent); escapes fail loud,
empty chars fail, `""` decodes to `[0]`, embedded NULs are preserved.
The three escape copies stay chip-local by the `compose_map` copy
precedent (convergence deferred; deltas pinned loud). The commit
`Literal` gate admits exactly the M1 integer shape plus the two `/33`
shapes. `lex.escape_decode` (local 21), `lex.char_decode` (22),
`lex.string_decode` (23), stage 2, layer 2, `lx_string_slice()`
registry (53 entries, cumulative over `lx_float_slice()`); no schema
change; two allowlist rows (LX12/LX13 `lex.literals`; LX11 Ack-only).
New hashed rules `lx.escape-decode`, `lx.char-typed`,
`lx.string-record`. New artifact `t01-c01-c06/33`
(`e8400eb1…eef455`); acceptance `compiler/tests/c33_string.rs`
(14 tests); item list in [LX_STRING_SLICE.md](LX_STRING_SLICE.md).

`/34` amendment — Wave 3 slice 9, PA decl (R1 auto-bump; `/33`
preserved as history): four Ack-only workers splitting the M1
declaration path into individually testable productions (T05
PA02/PA03/PA05/PA07/PA09/PA28/PA32 M1 scope). `PaExternalChip` (chip
44, PA02) classifies one external declaration at the continuation
cursor (`ExternalDecl` context, 8-token lookahead window) as a
function definition (`{`) or a declaration (`;`); any third
discriminator — including EOF, `=`, `,`, or a K&R parameter name — is
an explicit `Fail`, never a guessed default. `PaSpecifierChip` (chip
45, PA03) accepts exactly Keyword `int` (one committed token) and
rejects every other bundle as `Unsupported`. `PaDeclaratorChip` (chip
46, fused PA05/PA07/PA09) accepts exactly `main(void)` (zero
parameters, prototype) and fails the ambiguous `()` shape as a typed
`Task`-channel DEFECT; pointer/parenthesized/array/non-`void`/non-`main`
shapes are explicit `Unsupported`. `PaBlockChip` (chip 47, fused
PA28/PA32) dispatches on the task kind: exactly
`{ return <int> + <int> ; }` (7 tokens) or exactly
`return <int> + <int> ;` (5 tokens), each integer leaf backed by one
committed literal; `return;`, `{}`, and every other shape fail
`Unsupported`. All four complete `Ack` and append nothing (no cursor
carrier — OB-30 stays open, caller holds the cursor; no node links —
committed nodes stay PA01-owned until the full catalog split).
`parse.external_declaration` (local 17), `parse.specifiers` (18),
`parse.declarator` (19), `parse.block` (20), `parse.return` (21),
stage 2, layer 2, `pa_decl_slice()` registry (58 entries, cumulative
over `lx_string_slice()`); no schema change; no allowlist rows
(Ack-only). New hashed rules `pa.external-dispatch`,
`pa.specifier-int`, `pa.declarator-void`, `pa.block-return`. New
artifact `t01-c01-c06/34` (`229ae1a7…169be74`); acceptance
`compiler/tests/c34_parse.rs` (12 tests); item list in
[PA_DECL_SLICE.md](PA_DECL_SLICE.md).

`/35` amendment — Wave 3 slice 10, PA expr (R1 auto-bump; `/34`
preserved as history): two Ack-only workers splitting the M1
expression path into individually testable productions (T05
PA16/PA20/PA22 M1 scope). `PaBinaryChip` (chip 48, fused PA16/PA22)
dispatches on the task kind: PA16 accepts exactly one integer-constant
token backed by one committed literal; PA22 accepts exactly
`<int> + <int>` (kinds `[Integer, Punctuator, Integer]`, middle
spelling `+` through the committed PP token, both integers
literal-backed) as a single left-associative precedence-climb step at
`min_bp = 0` (`binary_precedence` resolves `+` to `(10, 11)` only).
`PaUnaryChip` (chip 49, PA20) accepts exactly `+<int>` / `-<int>`
(operator through the committed PP bytes, operand integer-backed).
Every other shape — `-`, `*`, multi-operator chains, the 4-token
`2 + +3` sequence as a single PA22 shape (its `+3` suffix is the PA20
shape; composition is wiring-layer work), identifiers, juxtaposition —
fails `Unsupported`; missing literals and non-running tasks fail on the
`Task` channel; literal *values* are never interpreted
(T07/T08-owned). Both complete `Ack` and append nothing (no cursor
carrier — OB-30 stays open, caller holds the cursor; no node links —
committed nodes stay PA01-owned until the full catalog split).
`parse.primary` (local 22), `parse.binary` (23), `parse.unary` (24),
stage 2, layer 2, `pa_expr_slice()` registry (61 entries, cumulative
over `pa_decl_slice()`); no schema change; no allowlist rows
(Ack-only). New hashed rules `pa.primary-int`, `pa.binary-add`,
`pa.unary-plus-minus`. New artifact `t01-c01-c06/35`
(`88107dd3…7f8983`); acceptance `compiler/tests/c35_expr.rs`
(10 tests); item list in [PA_EXPR_SLICE.md](PA_EXPR_SLICE.md).

`/36` amendment — Wave 3 slice 11, PA recovery (R1 auto-bump; `/35`
preserved as history): two Ack-only workers closing the M1
declaration tail and the single-fault resumption path (T05 PA14/PA38
M1 scope). `PaPodChip` (chip 50, PA14) accepts exactly `main(void);`
and certifies the point-of-declaration registration (name, spelling,
declarator span) for the wiring layer, which owns the T06 declare
fan-out and the PA15 initializer ordering; comma lists and
initializers fail `Unsupported`, a missing `;` is the
unterminated-declaration defect on the `Task` channel (never an
implicit semicolon). `PaRecoveryChip` (chip 51, PA38) synchronizes
the fault suffix to an explicit delimiter with grounded
paren/bracket/brace counters: `;` consumed, `)` / `}` / `{`-stop /
EOF not consumed (the `{`-stop refuses to enter a following function
body); every `Ok` path satisfies the finite-advance guarantee `0 <
index <= tokens.len()`; a fault already at `Eof`, an empty window,
or a window with no sync token fails loudly instead of spinning.
Both complete `Ack` and append nothing (no cursor carrier — OB-30
stays open, caller holds the cursor; no node links; no child-task
fan-out — failed-frame cleanup and scope balancing stay T06
integration work). The two delivered files both claimed `PARSE`
local 25 / `ChipId(50)`; the integrator arbitrated the collision
linearly against the `/35` head (local 24, chip 49) into
`parse.decl_finish` (local 25) + `parse.recovery` (local 26), stage
2, layer 2, `pa_recovery_slice()` registry (63 entries, cumulative
over `pa_expr_slice()`); no schema change; no allowlist rows
(Ack-only). New hashed rules `pa.pod-finish`, `pa.recovery-sync`.
New artifact `t01-c01-c06/36` (`c8d2135b…b766e6`); acceptance
`compiler/tests/c36_recovery.rs` (11 tests); item list in
[PA_RECOVERY_SLICE.md](PA_RECOVERY_SLICE.md).

`/37` amendment — Wave 3 slice 12, T08 const-branch (R1 auto-bump;
`/36` preserved as history): four Ack-only workers certifying the M1
selected-branch and static-assert path (T08 CL04/CL07 M1 scope).
`BranchAndChip` (chip 52, CL04) evaluates the condition plus ONLY the
short-circuit-selected `&&` branch (`0 && <bad>` acks canonical `0`;
`2 && 3` acks canonical `1`); `BranchOrChip` (chip 53, CL04) mirrors
for `||` (`3 || <bad>` acks `1`; `0 || 3` acks `1`, `0 || 0` acks
`0`); `BranchCondChip` (chip 54, CL04) passes the selected `?:`
magnitude through verbatim (`1 ? 3 : <bad>` is never evaluated);
`StaticAssertChip` (chip 55, CL07) passes nonzero, fails zero as a
failed assertion, and fails a non-literal payload as
`NotConstantExpression` (never ICE, never a silent pass). The
unselected operand is never subset-checked, never budget-checked, and
may even dangle; every selected operand passes the M1 exercised-subset
gate (decimal `Integer`, no suffix, `Int` candidate — else explicit
`Unsupported`) and the configured bit-budget gate (else typed
`ConstOverflow` `Fail`). All four complete `Ack` and append nothing
(no branch-result commit carrier — committing branch values stays
future work; no node links; no child-task fan-out). The delivered
draft claimed `CONSTANT_CONST_FOLD` descriptively for all four shells
plus chip IDs 52–55; the integrator verified the `/36` head (no
`CONSTANT` local past 16, `PA38_CHIP = ChipId(51)`) and froze
`const_branch_and` (local 17) + `const_branch_or` (18) +
`const_branch_cond` (19) + `const_static_assert` (20), stage 2, layer
2, `const_branch_slice()` registry (67 entries, cumulative over
`pa_recovery_slice()`); no schema change; no allowlist rows
(Ack-only). New hashed rules `const.branch-selected`,
`const.static-assert`. New artifact `t01-c01-c06/37`
(`59721bd8…2fc8c`); acceptance `compiler/tests/c37_const_branch.rs`
(10 tests); item list in [CONST_BRANCH_SLICE.md](CONST_BRANCH_SLICE.md).

`/38` amendment — Wave 3 slice 13, VF14 evidence-classify (R1
auto-bump; `/37` preserved as history): one read-only verifier
classifying the T00 gate outcome over the complete
compile/link/run/check evidence vector (T13 VF14 scope).
`Vf14Chip` (chip 56) completes `Ack` when every required stage is
present and passing; any present failure — in particular compile-ok
with run-fail — is FAIL at the earliest failing stage in pipeline
order (typed `Fail`, never PASS); a required-but-missing stage is
rejected (typed `Fail`, never PASS); an undecodable carrier or an
invalid stage gate is rejected (typed `Fail`, never PASS).
Precedence is total: invalid gate, then earliest missing required
stage, then earliest undecodable slot anywhere, then earliest failing
slot anywhere. The payload convention is exactly six refs
`[instance, gate, compile, link, run, check]`; the gate is a
single-byte `Const` (0 compile / 1 link / 2 run / 3 check, required
evidence is the pipeline prefix); evidence slots decode the frozen
M1-scope carrier map (`Ack`→pass, `Diagnostic`→fail,
`Empty`→absent, `Record`/`Records`→undecodable, never PASS); the
instance ref is resolvability-checked only, never interpreted
(`HostTestEvidence` schema stays deferred as DEFECT-VF14-01, the
boolean carrier as DEFECT-VF14-02). No batch logic, no H6
dependency: exactly one transition proposal per handle. The
delivered draft defined file-local candidate kind/chip consts plus a
`candidate_kind()` helper; the integrator verified the `/37` head
(no `VERIFICATION` local past 19, `VF14` chip 56 free after
`CL07_ASSERT_CHIP = 55`) and froze `evidence_classify` (local 20),
stage 6, layer 6, `vf_evidence_slice()` registry (68 entries,
cumulative over `const_branch_slice()`); no schema change
(PP-slice reuse); no allowlist rows (zero writes).
New hashed rules `vf.evidence-complete`, `vf.evidence-stage`,
`vf.evidence-never-pass-missing`. New artifact `t01-c01-c06/38`
(`f18068f9…0d83`); acceptance `compiler/tests/c38_vf14.rs`
(10 tests); item list in [VF_EVIDENCE_SLICE.md](VF_EVIDENCE_SLICE.md).

`/39` amendment — Wave 3 slice 14, SE29 function-definition (R1
auto-bump; `/38` preserved as history): one semantic worker checking
the M1 function definition (T07 SE29 scope). `SeFuncChip` (chip 57)
reads one committed `FunctionDefinition` node, checks the M1
`(void)`-only declarator shape (any identifier list is an old-style
K&R parameter list, explicit `Unsupported`), the single committed
TY17 `int(void)` signature (zero is missing input, more than one is
an ambiguous-handoff DEFECT, a non-`(void)` shape is `Unsupported`),
the declared `main` symbol (missing or untyped is a loud failure, a
divergently-typed symbol is `Unsupported` as prototype-inconsistent),
and the committed `Return` child fact (missing is a loud failure, a
mistyped one is `Unsupported`, a nonzero effect mask is a typed
failure; the `Return` node owns the return-role facts). It then
appends one signature-carrying `SemRecord` (no `Return`-role plan) or
reuses the committed one (exactly-one per node). The delivered draft
defined a file-local chip const plus a `semantic_function_def_kind()`
helper; the integrator verified the `/38` head (no `SEMANTIC` local
past 20, chip 57 free after `VF14_CHIP = ChipId(56)`) and froze
`function_def` (local 21), stage 4, layer 4,
`se_function_slice()` registry (69 entries, cumulative over
`vf_evidence_slice()`); no schema change (SE-slice reuse); one
allowlist row (`SE_FUNC_CHIP` → `sem.records` for
`SEMANTIC_FUNCTION_DEF`, 32 → 33 rows).
New hashed rules `se.function-signature`,
`se.function-body-checked`. New artifact `t01-c01-c06/39`
(`07f4eade…f224`); acceptance `compiler/tests/c39_sefunc.rs`
(8 tests); item list in [SE_FUNC_SLICE.md](SE_FUNC_SLICE.md).

| ID | Implemented | Explicitly blocked / limited |
|---|---|---|
| C01 | Append-only typed arenas, stable IDs with no reuse, checked access, structured capacity/errors, intern table, all declared record families have an owning arena; every configured limit enforced before mutation on the checked bus/commit entry points (`alloc_source`, task bootstrap/allocation, `intern_name`, routing diagnostic emission, `commit_proposals`); `task_depth` rejects dangling parents; source content hashes computed internally from bytes | Language-store record schemas (pp/lex/parse/symbols/types/nodes/consts/layout/init/ir/opt/machine/ext) are `ReservedArena` placeholders owned by their task groups; they must be frozen before those groups are dispatched. The public mutable stores (`bus.arenas`, `bus.patch_log`, ...) are a trusted integration/host boundary: raw `TypedArena`/`ReservedArena` allocation checks only the per-arena capacity, and public `get_mut`/direct pushes bypass the global total/source/task/diagnostic budgets; worker chips must mutate only through the checked entry points and the commit path. §4's deterministic reserved-ID/local-reference relocation protocol is **not implemented or frozen**: `commit.rs` resolves only earlier predicted `Enqueue`-parent IDs inside one batch, store-patch `RecordRef`s are not existence-checked, and no named reservation/apply-map protocol or hashed rule exists ([M1 proposal](M1_PART_A_CONTRACT_PROPOSAL.md) OB-49). Limit tests cover the checked entry points only (`c07_limits`); no test establishes global budgets for direct public-store mutation |
| C02 | Frozen target identity; dialect/options/limits config (structurally immutable after init); unverified AArch64/Linux proposal; machine-readable probe fixture; private verified-state representation; attestation validates required fields (including `wchar_t.encoding`), identity, and report hash; macOS values rejected by test | The actual Linux probe has not run (no CI/VM substrate); all concrete values remain UNVERIFIED; `ensure_codegen_ready` fails closed. `attest` validates caller-supplied report data, not authenticity or physical provenance (TOCTOU). The H01 harness (not owned here) emits `wchar_t.encoding` as `utf32` only with positive ISO/IEC 10646/non-BMP evidence, otherwise `unresolved`, and leaves the four AAPCS64 fields `unresolved`; `attest` requires resolved numeric values, so H07 classification plus the T01 integrator's attest step must fill them first (T00 §4.1). Attestation performs no plausibility/consistency cross-checks beyond field presence, identity, and hash (for example, zero or oversized ABI register counts, or `align > size`, would be accepted); `c02_target::signed_report` recomputes the hash over a caller-built report and `attested_report_produces_a_verified_target` attests it, demonstrating the data-not-provenance boundary. The probe harness exists but has **never run** (no report), and the report→`attest` handoff — H07-owned classification of the report's `unresolved` ABI fields, then the T01-integrator-owned `attest`/`Probed` step — has not occurred |
| C03 | Task/TaskState/Result/Proposal/StorePatch envelope; next-tick enqueue; deterministic commit order; inner task IDs bound to the enclosing task; exactly one `Complete`/`Fail`/`AwaitHost` transition per task per batch; Enqueue parents resolved against existing or earlier predicted siblings; Enqueue destinations must be registered and accept the kind (bootstrap is integration-only); exactly-once completion and result consumption with an accurate `ResultAlreadyConsumed` error; lossless `usize` proposal-budget comparison; structured error protocol; unique kind registry with reserved local-code ranges | Per-group concrete task/result payload variants are not frozen (group owners add them; the envelope carries typed `RecordRef` payloads); patch `RecordRef`s are not existence-checked by the mechanical commit; the §4 reserved-ID/local-reference relocation protocol is absent (see C01); the frozen protocol enums are pinned by variant name only, not by numeric discriminant/wire tag (see C05) |
| C04 | Manifest schema extension, foundation store schema, validator, fixture tests, validated registry (schema + kind checks on registration); commit-time per-chip field-scoped write manifests, owner/task-kind attribution, read-only `config` rejected both at registration and commit | The extension is a lint, not a parser; group store fields must be declared as each group freezes them. The `/6` hash-scope reconciliation between `COMPILER_SFL_MANIFEST.md` §4 and the `hash_excludes=group-declared-store-fields` token is still open (see C05) |
| C05 | Canonical writer, SHA-256, full deterministic snapshot/trace/config hash (foundation records, sources with byte-recomputed hashes, full wire proposal payloads, routing, manifests, registry, schema, patches, versions, reserved-store allocated count + live IDs), sensitivity tests, contract hash freeze test | Reserved language-store record bodies are not encoded (schema unfrozen); record bodies excluded, tombstone positions visible. Frozen hash excludes runtime registrations, routing content, and group-declared store fields (covered by the snapshot). Enumeration coverage is name-only: `contract.rs` hashes the `*_NAMES` variant lists and `RECORD_KINDS` names, while the numeric tags in `snapshot.rs` (`push_task_state`, `push_result_value`, `push_record_ref`, `push_wires`) are hardcoded and neither derived from nor cross-checked against those lists; numeric-value hashing remains a `/6` item (M1 proposal OB-14/OB-34). The `/6` hash-scope reconciliation is also still open: the accepted two-tier model (frozen `foundation + M1AppendSchema` seed participates in the hash; post-seed `StoreSchema::declare()` stays excluded) must be applied atomically to `COMPILER_SFL_MANIFEST.md` §4, the `hash_excludes=group-declared-store-fields` token, `FrozenSchema::encode`, and `freeze.rs` |
| C06 | Routing table stored in the bus (replay-visible); deterministic selection; no-op terminates; unsupported/unregistered fails explicitly; commit failure transitions the task to `Failed` with the task attached to the diagnostic and without partial writes; transition-free worker vectors fail per-task (`TaskNotTransitioned`); idle ticks drain await-all joins with closure (`Joined`); stage/layer enforced on the driver path; cancel clears stale selection; defined cancel/budget pin behavior | Wave 1 fold worker installed via `drive_task`/`handler_for`; no broader language pipeline yet |

Test evidence (`/9`): `c01_arena` 7, `c02_target` 11, `c03_task` 46, `c04_manifest` 15, `c05_codec` 21, `c06_routing` 15, `c07_limits` 18, `c08_gate1` 21, `c09_readiness` 14, `c10_pp01` 7, `freeze` 14, plus two compile-fail doctests. `c07_limits` exercises only the checked bus/commit entry points (`max_intern_bytes` is enforced but has no dedicated test); `c01_arena` covers arena-local capacity and ID stability; no test establishes global budgets for direct public-store mutation. `freeze.rs` verifies hash recomputation, version-file consistency, and rule-ID uniqueness, not semantic equivalence or numeric tag stability.

Full test commands and results are in `compiler/README.md`.
