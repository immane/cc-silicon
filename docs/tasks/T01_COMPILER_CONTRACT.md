# T01: Compiler Bus and Protocol Freeze (Prerequisite for Parallel Implementation)

Owner: contract integrator. The artifacts must first be compilable and have a schema/fixture; other LLMs must not each invent their own CompilerBus or duplicate a type system.

## 1. CPU Storage Extension Decision (Pending Approval)

The original paradigm specification/blueprint requires the bus to use only fixed arrays and be heapless. A general-purpose compiler needs non-fixed-length input; the following explicit extension is proposed:

- Semantic state still resides entirely in the CompilerBus; the arena is the bus's data storage, not a private backend cache.
- The CPU reference permits append-only `Vec` arenas, intern tables, and work queues; logical records reference each other only through stable newtype IDs, not `Rc/Arc/RefCell/Mutex` or an implicit pointer object graph.
- Map iteration must not determine scheduling, symbol output, or ID allocation; stable insertion order/sorting guarantees consistency across the same snapshot. An address must not be used as a semantic ID.
- Dynamic allocation is a public capability; **it is no longer claimed that the compiler is heapless per tick or directly hardware-synthesizable**. The framework not allocating by itself does not mean user chips do not allocate.
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

Integer values use target bit-width + bit-pattern + signedness; constant arithmetic may use explicit wide integers or a reliable software arithmetic library, not host overflow behavior. Floating-point records use format/raw bits + rounding rules; host f64 must not be used to implement all C floating point.

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
