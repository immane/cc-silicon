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
