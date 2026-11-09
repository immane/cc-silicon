# A18 Diagnostic and Error Taxonomy Audit

| Field | Value |
|---|---|
| Status | Read-only audit complete; no source, test, contract, Cargo, `mod.rs`, or shared index was modified |
| Date | 2026-11-07 |
| Task | Stage A `A-CLI-01`: compiler diagnostic/error taxonomy and ownership requirements |
| Repository | `/Volumes/Nayuki/Development/Rust/cc-silicon` |
| Branch / HEAD | `wave4-development` / `430ae29162efef90f86667f384097964d611604f` |
| Contract baseline | `t01-c01-c06/39`, hash `07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24` (`compiler/contracts/CONTRACT_VERSION`) |
| Scope | `compiler/src/diagnostic.rs`, `contract.rs`, `commit.rs`, `task.rs`, `manifest.rs`, `target.rs`, `routing.rs`, `bus.rs`, `snapshot.rs`, `chips/**`, V1-V3 error paths, and tests asserting diagnostics |
| Out of scope | Implementing or renaming codes, editing contracts or tests, full-suite/clippy runs, target probe, torture corpus, pass-rate claims |
| Deliverable | This file only: `docs/reviews/dispatch/A18_DIAGNOSTIC_TAXONOMY.md` |

**Bottom line.** The frozen surface today is only the set and order of the nine
`DiagGroup` names. Specific numeric codes are *not* in the contract hash; they are
pinned only by comments, a handful of named constants, and a few tests. In
production-like code there is one de facto catch-all (`DiagGroup::Task` code `4`,
**339 sites**), one generic capability code (`DiagGroup::Unsupported` code `1`,
**106 sites**), and a largely unused `DiagGroup::Arena` (**0 sites**). `Invalid`
exists in design prose and in chip comments but has no group or code. Source
spans are attached at only **3** sites, task attribution at only **1** site, and
arena/limit failures collapse into `Protocol:1` (intern into `Protocol:18`). The
proposal below keeps the nine frozen
groups, freezes group charters plus an append-only symbolic code registry, maps
`Invalid` to `Semantic`, capacity to `Arena`, producer protocol to `Protocol`,
and compiler/verification defects to `Internal`, then migrates V1 before V2/V3.
All of this is **proposal only**; no contract or code was changed.

---

## 1. Method, Evidence Classes, and Checks Run

### 1.1 Required reading and inspected surfaces

Read for this audit: `AGENTS.md`; `docs/tasks/README.md`;
`docs/tasks/T01_COMPILER_CONTRACT.md`; `docs/tasks/PARALLEL_EXECUTION.md`;
`docs/tasks/COMPILER_DEVELOPMENT_GUARDRAILS.md`;
`docs/tasks/TASK_TEMPLATE.md`; `docs/tasks/MASS_PARALLEL_DISPATCH.md`
(especially §6.2 V1-V3); `docs/tasks/T02_CONTROL_CHIPS.md` rows CT11-CT13;
`docs/tasks/T13_VERIFICATION_CHIPS.md`; `docs/tasks/PP_DIRECTIVE_SLICE.md`;
`compiler/src/diagnostic.rs`; `compiler/src/contract.rs`;
`compiler/src/chips/mod.rs`; all chip group modules; the diagnostic-consuming
commit/routing/task/manifest/target/bus/snapshot paths; and the tests that assert
diagnostic groups, codes, messages, ordering, capacity, or sentinels.

The repository worktree was dirty before this audit. Untracked
`compiler/src/chips/lex/lx_error.rs`, `lx_extension.rs`, the parse drafts, and
`docs/tasks/*_FREEZE*.md` are user work; they were read only where relevant and
were not modified. Other dispatch reports under `docs/reviews/dispatch/` were
not edited.

### 1.2 Evidence classes used below

- **[OBSERVED]** direct source/document fact, with `file:line`.
- **[DERIVED]** static count or classification from source text (not runtime
  telemetry).
- **[PROPOSED]** taxonomy, code, ownership, or migration recommendation.
- **Frozen** means participating in the `t01-c01-c06/39` contract hash or pinned
  by a regression test. A code that is only described in a comment is **not**
  hash-frozen.

### 1.3 Checks actually run

Only two targeted suites were run, because this is a documentation audit and the
user prohibited the full suite:

```text
cargo test --manifest-path compiler/Cargo.toml --test c03_task --test c37_const_branch --quiet
  c03_task        46 passed; 0 failed
  c37_const_branch 10 passed; 0 failed
```

Not run: full compiler suite, root suite, clippy, rustfmt, target probe, torture
corpus. No probe, ABI, execution, or pass-rate claim is made. The ledger records
that the current dirty workspace has clippy failures from uncommitted parse
drafts; that is outside this audit.

### 1.4 Count methodology

Counts are regex counts over `compiler/src`. "Production-like" means text before
the first `#[cfg(test)]` in each file. Raw counts include inline test assertions.
Examples: `Task:4` is **355** raw occurrences and **339** production-like;
`DiagnosticDraft::unsupported(` is **116** raw and **106** production-like.
These are static occurrence counts, not unique diagnostic paths; a single helper
can be reached from several chips.

---

## 2. Current-State Inventory (Observed)

### 2.1 What is actually frozen

**[OBSERVED]** The contract hash covers the `DiagGroup` variant names and order,
not numeric codes:

- `DiagGroup` has exactly nine variants: `Protocol`, `Arena`, `Config`,
  `Target`, `Manifest`, `Task`, `Semantic`, `Unsupported`, `Internal`
  (`compiler/src/diagnostic.rs:37-57`).
- `contract.rs:325-335` duplicates those names as `DIAG_GROUP_NAMES`;
  `contract.rs:552` feeds that list into `FrozenSchema::encode`.
- `compiler/tests/freeze.rs:246-259` pins the nine names and their order.
- `DiagnosticCode` is only `{ group: DiagGroup, code: u16 }`
  (`diagnostic.rs:76-83`). Its numeric values are **not** encoded by
  `FrozenSchema::encode`.
- `Severity` (`Error`, `Warning`, `Note`) is **not** in `DIAG_GROUP_NAMES` and
  is not encoded; only `snapshot.rs:948` maps it to strings.
- `pp.diagnostic-taxonomy-frozen` is a hashed rule *identifier*
  (`contract.rs:185`), but the taxonomy content and numeric codes are not in the
  hash.

Consequences:

1. Reclassifying a code's group or number does not change the contract hash, but
   it **does** change committed diagnostics and snapshot hashes. It must still be
   treated as a contract-visible behavior change.
2. Comments that say a code is "frozen" (for example
   `commit.rs:450-462`, `target.rs:1167-1172`) are freeze records by convention,
   not hash evidence.
3. None of the defect patterns below are prevented by the current freeze test.

### 2.2 Diagnostic record shape and lifecycle

**[OBSERVED]** `DiagnosticDraft` carries `severity`, `code`, `message: String`,
optional `span: SpanId`, optional `task: TaskId` (`diagnostic.rs:105-116`).
`DiagnosticRecord` adds `ordinal: u64` (`diagnostic.rs:150-163`). There is no
`notes`, `related`, `expansion`, or stable `DiagnosticId` field inside the
record; the arena ID is external.

Lifecycle:

- A worker returns `Proposal::Fail { task, diagnostic }`.
- `commit_proposals` preflights and then commits each `Fail` verbatim:
  `DiagnosticRecord::commit(ordinal, diagnostic.clone())`, increments
  `next_diagnostic_ordinal`, and stores it (`commit.rs:1466-1477`). It does **not**
  auto-fill `task` or `span`.
- The recovery path `fail_selected` calls `error.to_diagnostic().with_task(task)`
  (`routing.rs:577-591`); this is the only `.with_task` call in the crate.
- `try_fail_task` commits a recovery diagnostic when total-record and diagnostic
  capacity allow; otherwise the task becomes
  `TaskState::Failed(DiagnosticId::NONE)` (`commit.rs:2837-2868`).
- Preflight counts one new diagnostic per `Fail` (`commit.rs:969`) and
  `ensure_diagnostics` enforces `max_diagnostics` (`bus.rs:1081-1090`).
- Snapshot encoding includes severity, group name, numeric code, message, span,
  and task (`snapshot.rs:499-516`, `snapshot.rs:903-925`). Messages therefore
  participate in snapshot hashes (confirmed by `c05_codec.rs:137-149`).
- Diagnostic capacity exhaustion is silent except for the `NONE` sentinel; there
  is no report field that counts diagnostics dropped for capacity.

### 2.3 Production code inventory

**[DERIVED]** Production-like construction sites, grouped by current code:

| Group | Codes seen in production | Sites | Status / evidence |
|---|---|---:|---|
| `Protocol` | `1`, `10`-`18` | 12 | Legacy generic plus pinned commit codes; `commit.rs:463-477`; `chips/mod.rs:428-430`; `task.rs:1384-1390`; `c03_task.rs:980-1040` |
| `Arena` | none | **0** | Declared at `diagnostic.rs:40-41`; no production construction; the only test reference is `freeze.rs:248-249` |
| `Config` | `1`-`7` | 10 | `1` is genuine `DuplicateOption` (`target.rs:1179-1182`) but also reused by chips (see C7); `2`-`7` are config bounds (`target.rs:1183-1222`) |
| `Target` | `1` | 1 | `TargetUnverified` (`target.rs:1175-1178`); no probe/ABI code |
| `Manifest` | `1` | 1 | All 14 `ManifestError` variants collapse to one code (`manifest.rs:631-639`) |
| `Task` | `2`, `3`, `4`, `5`, `10` | 345 | `4` alone is 339 sites; `3` is 2 sites; `5` is 2 sites; `2` is registry; `10` is terminator |
| `Semantic` | `1`, `2` | 2 | Redeclaration and undeclared only (`ty_symbol.rs:307`, `:373`); `c13_ty.rs:678-731` |
| `Unsupported` | `1` (106), `2` (2), `3` (0), `4` (1) | 109 | `CONST_OVERFLOW` = `Unsupported:2` (`diagnostic.rs:93`); `CONST_UNSUPPORTED` = `Unsupported:3` is **defined but unused**; `CONST_NOT_CONSTANT_EXPRESSION` = `Unsupported:4` (`diagnostic.rs:96-100`); generic helper at `diagnostic.rs:130-133` |
| `Internal` | `1` | 4 | Generic "invariant" used by `pp_substitute.rs:469`, `pp_variadic.rs:736`, `:1408`, `pp_emit.rs:352` |

Total production-like typed construction sites: **484** (including the three
named constant sites in the `Unsupported` group).

### 2.4 The `Task:4` catch-all

**[DERIVED]** Heuristic static classification of the 339 production `Task:4`
sites by the message/context text. This is a reading aid, not a semantic proof;
"other" contains false positives that need per-site review.

| Category | Approx. sites | Representative messages / files |
|---|---:|---|
| Dangling/missing projection input | 143 | `"reads missing token {}"`, `"reads missing pp-token {}"`, `"canonical int type is not committed"`, `"token {} with no spelling"` |
| Task lifecycle "not running" | 60 | `"{label} task {} is not running"` repeated in almost every chip |
| Other (scope, verification, directives, headers, macro redefinition) | 51 | `"duplicate file-scope enter"`, `"vf06: bad value category"`, `"redefine mismatch: ..."`, `"#error"` messages |
| Invalid source/input | 48 | `"unterminated character literal"`, `"malformed decimal exponent"`, `"malformed _Pragma string"` |
| Semantic constraint | 16 | `"duplicate symbol for declarator"`, `"lower reads undeclared function"`, `"effectful node"` |
| Protocol/shape | 11 | `"payload must carry exactly one node"`, `"task {} carries no token refs"` |
| Internal/defect | 10 | `"DEFECT: ..."`, `"duplicate canonical int type"`, `"ambiguous folded constants"`, `"rescan circuit breaker"` |

The buckets describe today's message text, not the proposed mapping: §4.5
re-homes, for example, a missing global prerequisite to `Internal`, and
payload-referenced missing records to `Protocol`.

Per chip group (production-like):

| Group directory | `Task:4` | `Unsupported:1` |
|---|---:|---:|
| `chips/preprocess` | 162 | 22 |
| `chips/parse` | 41 | 27 |
| `chips/lex` | 39 | 19 |
| `chips/semantic` | 31 | 17 |
| `chips/verify` | 25 | 2 |
| `chips/symbols` | 18 | 4 |
| `chips/ir_lower` | 13 | 5 |
| `chips/constant_layout_init` | 5 | 3 |
| `chips/types` | 5 | 2 |
| `chips/mod.rs` | 0 | 0 |

**[OBSERVED]** The problem is not that the compiler reports errors; it is that a
single code means "the caller broke the task protocol", "a record is missing",
"the source is malformed", "the source is valid C outside M1", "the compiler
found an internal duplicate", and "a verifier rejected the store" at the same
time. This is exactly the `Unsupported` vs `Invalid` vs `Task` ambiguity named in
the task scope.

### 2.5 Source spans, task attribution, severity, dedup, notes

**[OBSERVED]**

- `DiagnosticDraft::with_span` is called at only three production sites:
  `chips/lex/lx_escape.rs:439`, `chips/lex/lx_string.rs:598`, and
  `chips/lex/lx_string.rs:662`. All other PP, parse, type, semantic, constant,
  IR, and verification diagnostics have `span: None`, even where the input
  carries committed spans.
- `with_task` is called at one site: `routing.rs:585` (commit-failure recovery).
  Worker `Proposal::Fail` diagnostics are committed without the enclosing task
  (`commit.rs:1466-1477`).
- `Severity::Warning` and `Severity::Note` are declared (`diagnostic.rs:12-21`)
  but never constructed anywhere in `compiler/src`; `DiagnosticDraft::error` is
  the only constructor (`diagnostic.rs:118-128`). `#warning` is explicitly
  deferred because no non-fatal wire exists (`PP_DIRECTIVE_SLICE.md:9`, `:117`).
- There is no deduplication. Every `Fail` becomes a new record; CT11's
  "deduplicate" requirement (`T02_CONTROL_CHIPS.md:118`) is unimplemented.
- There is no `notes`/related-diagnostic field, no macro-origin field, and no
  stable `DiagnosticId` inside the record. Macro provenance can only be inferred
  from `SpanRecord.expansion` (`contract.rs:455-456`), but no chip attaches that
  span to a diagnostic.
- Diagnostic ordering is commit ordinal only; there is no severity sort or
  deterministic dedup key.

### 2.6 Capacity, overflow, and the sentinel

**[OBSERVED]**

- `DiagGroup::Arena` is unused. Arena failures (`ArenaError::OutOfBounds`,
  `Tombstoned`, `Sentinel`, `CapacityExceeded`, `arena.rs:45-76`) enter
  `CommitError::Capacity` and then map through the wildcard to
  `Protocol:1` (`commit.rs:475`).
- `LimitError` (`limits.rs:189-269`) and `InternError` (`intern.rs:17-30`) enter
  `CommitError::Limit` / `CommitError::Intern`. `Limit` also falls through to
  `Protocol:1`; `Intern` is the odd `Protocol:18` (`commit.rs:473`).
- `SelectionBatchOverflow` and `BackpressureCapacity` are deliberately
  `Protocol:10`/`11` (`commit.rs:465-466`), which is defensible as dispatch
  protocol, but they are also capacity facts.
- `CONST_OVERFLOW` is `Unsupported:2` (`diagnostic.rs:93`). Its two production
  sites are `max_const_bits` budget checks (`fold.rs:273-280`,
  `fold_branch.rs:520-530`), i.e. a configured resource bound, not an unsupported
  C feature.
- `CONST_UNSUPPORTED` = `Unsupported:3` is never used; non-subset constants use
  the generic `unsupported()` helper = `Unsupported:1`.
- The sentinel `TaskState::Failed(DiagnosticId::NONE)` loses the reason
  (`commit.rs:2830-2868`). A tool can detect the sentinel but cannot recover the
  code, span, or message.
- `max_diagnostics = 0` is tested (`c03_task.rs:1501-1522`,
  `c07_limits.rs:180-204`), so the sentinel is a real, supported state, not an
  accident.

### 2.7 Cancellation, recovery, and joins

**[OBSERVED]**

- Cancellation is a control outcome, not a diagnostic. `CompilerPins.cancel`
  sets `control.cancel_requested`, sets `control.job_state = Failed`, clears
  selection/in-flight, returns `TickOutcome::Cancelled`, and emits no
  `DiagnosticRecord` (`routing.rs:611-625`, `:654-668`; `bus.rs:803-804`,
  `:927-928`). Repeated cancel is idempotent by construction.
- The commit-failure recovery (`fail_selected`, H6) attaches the `CommitError`
  to every dispatched task via `with_task` and uses `DiagnosticId::NONE` when
  capacity is exhausted (`routing.rs:565-591`).
- The await-all join fails a waiter by reusing the first failed child's
  committed diagnostic, preserving severity/code/message/span/task
  (`pp_variadic.rs:721-742`). This is a good pattern but is implemented locally,
  not as a shared contract mechanism.
- PA38 parse recovery exists (`chips/parse/pa_recovery.rs`) and is reserved in
  `manifest.rs:1138-1141`, but is not registered in the candidate driver's
  worker registry (`compiler/src/bin/candidate.rs:381-465`). CT12 recovery is
  unimplemented.
- The `chips/control/` directory does not exist. CT11 (diagnostic commit),
  CT12 (recovery), and CT13 (cancel) are design rows only
  (`T02_CONTROL_CHIPS.md:118-120`); the de facto diagnostic owner is the commit
  path plus `routing.rs`.

### 2.8 Evidence boundary and message content

**[OBSERVED]**

- The candidate driver distinguishes exit 0 (evidence), 1 (candidate
  diagnostic), and 2 (driver/usage/target refusal) (`candidate.rs:17-19`,
  `:900-974`). On a pipeline diagnostic it prints the record message
  (`candidate.rs:355-362`).
- Diagnostic messages are derived from committed bus data (IDs, interned
  spellings, token text) and `#error` source text. Chips perform no I/O, so
  diagnostic records cannot read host paths, environment, clock, or secrets
  directly.
- However, messages are unbounded `String`s. `#error` joins all token spellings
  (`pp_diagnostic.rs:130-143`), and invalid-token messages quote the offending
  spelling with `String::from_utf8_lossy` (`lx_error.rs:81-92`). A pathological
  source can inflate a diagnostic and, because messages are hashed in snapshots,
  change evidence size and hashes.
- Host driver errors (file read/write) do include paths, but they are printed by
  the Host driver and are not `DiagnosticRecord`s. That separation is correct
  and should remain.
- `DiagnosticRecord` has no redaction/truncation marker, so an evidence consumer
  cannot distinguish a complete quoted token from a truncated one.

### 2.9 Documentation versus implementation

**[OBSERVED]**

- `diagnostic.rs:4-6` claims "Every error type in this crate can be mapped to a
  `DiagnosticDraft`". This is false for `RouteError` (`routing.rs:42-68`),
  `ManifestRegistryError` (`manifest.rs:642-669`), `ProbeError`
  (`target.rs:721-801`), and direct `ArenaError`/`InternError`/`LimitError` use.
- T01 §2 says `diagnostics` holds "ordered errors/warnings, locations,
  associated tasks" (`T01_COMPILER_CONTRACT.md:48`). Warnings, locations, and
  task association are mostly absent as described above.
- T01 C06 acceptance says a commit failure transitions the task "with the task
  attached to the diagnostic" (`T01_COMPILER_CONTRACT.md:881`). That holds for
  `fail_selected`, not for worker `Fail` proposals.
- T02 CT11 requires "severity/span/code/notes by stable ordinal and
  deduplicate" (`T02_CONTROL_CHIPS.md:118`). Severity/code/ordinal exist;
  notes and dedup do not.
- The PP directive slice freezes a taxonomy table ("`error` → PP26 child;
  `warning` and every other name → explicit `Fail(Unsupported)`",
  `PP_DIRECTIVE_SLICE.md:35-39`, `:88`) and names the rule
  `pp.diagnostic-taxonomy-frozen`, but the implementation uses the generic
  `Task:4` and `Unsupported:1`; there is no numeric code table.
- T02 says the exact error-code inventories remain open
  (`T02_CONTROL_CHIPS.md:32`, `:47`, `:63-76`). This audit confirms that state.

### 2.10 Freeze classification of current codes

**[DERIVED]** "Pinned" means a test or a documented "frozen" comment names the
numeric value; it is not hash evidence. "Ad hoc" means the value is a local
choice with no registry or contractual status. "Missing" means no code exists
for a required V1-V3 category.

| Code / surface | Classification | Evidence |
|---|---|---|
| `DiagGroup` names/order | **Frozen by hash** | `contract.rs:325-335`, `:552`; `freeze.rs:246-259` |
| `Severity` variant set | Frozen by type only; not hashed | `diagnostic.rs:12-21` |
| `Protocol:1` | Pinned by tests/comments; generic | `commit.rs:475`; `c03_task.rs:1030-1032` |
| `Protocol:10`-`18`, `Task:10` | Pinned by tests/comments | `commit.rs:463-477`; `c03:980-1040` |
| `Config:1`-`7`, `Target:1` | Pinned by comments/tests; `Config:1` overloaded | `target.rs:1167-1222`; `c02_target.rs:242-245` |
| `Manifest:1` | Pinned by group test only | `manifest.rs:631-639`; `c04_manifest.rs:278-292` |
| `Semantic:1`-`2` | Pinned by group tests | `ty_symbol.rs:307`, `:373`; `c13_ty.rs:678-731` |
| `Task:2` | Pinned by code only (registry) | `task.rs:626-631` |
| `Task:3` | **Ad hoc** shared const-failure slot | `task.rs:1523-1543`; `fold_branch.rs:340-347` |
| `Task:4` | **Ad hoc catch-all** | 339 production sites; §2.4 |
| `Task:5` | **Ad hoc** constraint slot (2 sites) | `pp_substitute.rs:461-465`; `pp_variadic.rs:1402-1405` |
| `Unsupported:1` | Pinned by helper/tests; semantically overloaded | `diagnostic.rs:130-133`; `routing.rs:421-474` |
| `Unsupported:2` (`CONST_OVERFLOW`) | Pinned by tests; misgrouped resource bound | `diagnostic.rs:91-93`; `c08_gate1.rs:1288-1289`; `c37_const_branch.rs:441-454` |
| `Unsupported:3` (`CONST_UNSUPPORTED`) | **Dead / unused** | `diagnostic.rs:96` |
| `Unsupported:4` (`CONST_NOT_CONSTANT_EXPRESSION`) | Pinned by test; duplicates `Task:3` semantics | `diagnostic.rs:100`; `c37_const_branch.rs:402` |
| `Internal:1` | **Ad hoc generic** (4 sites) | `pp_substitute.rs:469`; `pp_variadic.rs:736`, `:1408`; `pp_emit.rs:352` |
| `Arena:*` | **Missing** (group unused) | `diagnostic.rs:40-41` |
| Capacity/limit/intern detail | Missing; collapses to `Protocol:1`/`18` | `commit.rs:463-477` |
| Cancellation reason | Missing (control outcome only) | `routing.rs:611-625` |
| Warnings/notes, dedup, notes/macro-origin fields | Missing | §2.5 |
| Span/task on diagnostics | Mostly missing (3 spans, 1 task) | §2.5 |
| V1/V2/V3 language/capacity/ABI codes | Missing; see §4.6 | §4.6 |

---

## 3. Conflicts and Gaps

The following are observed defects or ambiguities; each is a candidate contract
change request, not a change made here.

| ID | Finding | Evidence |
|---|---|---|
| C1 | **`Task:4` is a catch-all.** It mixes producer protocol, missing committed records, malformed source, C constraints, internal duplicates, and verifier rejections. | 339 production sites; §2.4 |
| C2 | **`Task:5` is an orphan.** Only two sites use it for `#`/`##`/`__VA_ARGS__` constraint violations; equivalent "constraint" failures elsewhere use `Task:4`. | `pp_substitute.rs:461-465`, `pp_variadic.rs:1402-1405` |
| C3 | **One condition, two codes; a second condition shares one.** "Not a constant expression" is `Unsupported:4` in `eval_assert` and `Task:3` in `ConstantResult::route`; a failed static assertion reuses `Task:3` with a comment that the exact code is open. | `fold_branch.rs:322-347`, `task.rs:1523-1543`, `c37_const_branch.rs:394-402` |
| C4 | **`CONST_UNSUPPORTED` is dead.** The named constant `Unsupported:3` has zero production uses; non-subset constants use generic `Unsupported:1`. | `diagnostic.rs:96`, `rg` result |
| C5 | **Constant overflow is grouped as "unsupported".** `CONST_OVERFLOW` is a configured `max_const_bits` budget failure, not an unimplemented language feature. | `diagnostic.rs:91-93`, `fold.rs:273-280` |
| C6 | **`Unsupported` conflates capability gaps with missing wiring.** `routing.rs` reports unregistered kinds and missing handlers as `Unsupported:1`; those are infrastructure/protocol states, not C features. | `routing.rs:421-474` |
| C7 | **`Config:1` is overloaded.** It means `DuplicateOption` in `target.rs`, artifact-size budget in `pp_normalize.rs:160` and `pp_emit.rs:321`, and wrong `wchar_t` encoding in `lx_string.rs:659`. | `target.rs:1179-1182`; three chip sites |
| C8 | **`Internal:1` is a generic bug bucket.** Four sites use it for unrelated invariants (join missing diagnostic, rescan breaker, invalid location map). | `pp_substitute.rs:469`, `pp_variadic.rs:736`, `:1408`, `pp_emit.rs:352` |
| C9 | **`Arena` is dead; capacity collapses to `Protocol:1`.** Arena, limit, and intern exhaustion lose their specific class. | `arena.rs:45-76`, `limits.rs:189-269`, `intern.rs:17-30`, `commit.rs:463-477` |
| C10 | **`Semantic` is too narrow for "Invalid".** It has only redeclaration/undeclared; lexical and parse malformation have no semantic home. | `ty_symbol.rs:307`, `:373` |
| C11 | **One code per error enum.** `Manifest:1` covers 14 variants; `Task:2` covers 5 registry variants; `Config` has seven codes but a parallel string `code()` system. | `manifest.rs:434-639`, `task.rs:570-631`, `target.rs:1150-1223` |
| C12 | **No span/task attachment or dedup.** Three spans, one task, no notes, no dedup. | §2.5 |
| C13 | **Warnings/notes are dead.** `Severity::Warning`/`Note` are never constructed; `#warning` needs a wire. | `diagnostic.rs:12-21`, `PP_DIRECTIVE_SLICE.md:117` |
| C14 | **Cancellation has no reason record.** `job_state = Failed` is set with no diagnostic or structured cancel reason. | `routing.rs:611-625` |
| C15 | **Verification failures are `Task:4`.** VF01/VF05/VF06/VF12/VF14 rejections are indistinguishable from protocol misuse. | `vf_evidence.rs:414-423`, `vf_invariant.rs`, `vf_store.rs`, `vf_syntax.rs` |
| C16 | **Several error types have no diagnostic mapping.** `RouteError`, `ManifestRegistryError`, `ProbeError`, and direct arena/intern/limit errors. | §2.9 |
| C17 | **Dual code systems.** `DiagnosticCode{group,u16}` versus `ConfigError::code()` / `LimitError::code()` string codes (`config.*`, `limits.*`); no mapping table, and the strings are not stored in records. | `target.rs:1150-1163`, `limits.rs:318-335` |
| C18 | **Numeric codes are not hash-frozen.** The freeze test cannot detect a renumbering; only scattered tests/comments do. | §2.1 |
| C19 | **Message size/quoting is unbounded.** Snapshot hashes include messages, so source-controlled text can affect evidence size and hash. | `pp_diagnostic.rs:130-143`, `lx_error.rs:81-92`, `snapshot.rs:499-516` |
| C20 | **The generic `Protocol:1` fallback hides commit variants.** Pre-existing `CommitError` variants (unknown result, duplicate completion, ownership, patch shape, etc.) all map to `Protocol:1`. | `commit.rs:447-477` |

---

## 4. Proposed Minimal Stable Taxonomy

> **[PROPOSED]** Everything in §4-§7 is a recommendation for the T01 integrator
> and group owners. It changes no file. Numeric values below are illustrative
> unless they already exist; the stable part is the group charter plus symbolic
> code names.

### 4.1 Design principles

1. **Keep the nine groups.** Adding a group changes `DIAG_GROUP_NAMES` and the
   contract hash (`contract.rs:325-335`, `:552`); it also requires a freeze-test
   update. Reuse the existing groups and carry stage specificity in code ranges.
2. **`Invalid` is a role, not a group.** Map it to `Semantic`. "Invalid" means
   the source input is malformed or violates a C constraint; "Unsupported" means
   the source is well-formed C but deliberately outside the implemented subset.
3. **Group = blame class; code = specific cause; message = prose.** Evidence
   consumers should match on `(group, code, span, task)`, never on message text.
4. **Preserve every existing numeric code during migration.** New semantics get
   new codes in reserved ranges; old codes become deprecated aliases. Never
   renumber a code that has appeared in a committed diagnostic.
5. **Append-only registry.** The integrator owns a code registry; chip owners
   request new symbolic codes. No chip owner invents a raw
   `DiagnosticCode::new` value in a chip file.
6. **`Unsupported` is never a pass and never a protocol fault.** A missing
   handler, unregistered kind, or malformed payload is `Protocol`; a compiler
   defect is `Internal`.
7. **A diagnostic is typed evidence.** Include the associated task
   automatically and the source span whenever the input carries one.

### 4.2 Group charters

| Group | Charter | Belongs here | Does **not** belong here |
|---|---|---|---|
| `Protocol` | Producer/caller violated the frozen task, proposal, commit, request, selection, or routing protocol | unknown task, wrong kind, payload arity/family, dangling payload reference, owner/attribution, duplicate/consume, stage/layer, selection/backpressure, request decode, missing route/handler | malformed C, capacity exhaustion, compiler defects |
| `Arena` | A configured runtime resource bound was hit | arena capacity, total records, source bytes, intern entries/bytes, queue length, task count/depth, diagnostic capacity, proposal budget, constant-bit budget | invalid config value, protocol misuse |
| `Config` | The configuration/model itself is invalid | option duplicates, invalid limits, invalid dialect/target option combination, unsupported `wchar_t` model | runtime exhaustion |
| `Target` | Target identity/model/probe/ABI gate | target unverified, probe report invalid, ABI/model mismatch, codegen disabled | generic config, language errors |
| `Manifest` | SFL manifest/schema/registration/ownership | duplicate ID/name/kind, undeclared field, store owner violation, stage assignment, capability/backend mismatch, routing table registration | runtime commit failure |
| `Task` | Task lifecycle/outcome state | not running, no transition, progress non-advancing/limit, cancellation record if required, join/wait state | producer protocol, source errors |
| `Semantic` | Source-language fault ("Invalid") | malformed lexical/parse input, C constraint violations, type/conversion/lvalue/call/control/init errors, not-constant, `#error`, constant representability | well-formed unsupported features, capacity, defects |
| `Unsupported` | Explicit capability gap | well-formed construct outside the implemented subset/dialect/target; deferred owner | malformed input, capacity, internal defect, missing registration |
| `Internal` | Compiler/integration defect or invariant | missing global prerequisite, duplicate producer, ambiguous handoff, verification rejection, engine/recovery invariant, diagnostic-join miss | source-language errors, protocol misuse |

### 4.3 Classification decision procedure

Apply in order; the first match wins.

1. **Protocol?** Did a task/proposal/commit/request field violate the frozen
   protocol (identity, kind, arity, family, ownership, reference, transition,
   stage/layer, route)? → `Protocol`.
2. **Task lifecycle?** Is the task not `Running`, did it fail to transition, or
   is this a progress/join/wait/cancel state? → `Task`.
3. **Resource bound?** Would a configured arena/queue/source/intern/diagnostic/
   proposal/constant-bit bound be exceeded? → `Arena`. If the configuration
   value itself is invalid at construction → `Config`.
4. **Target gate?** Is the target unverified, the probe/attestation invalid, or
   the requested ABI/model unavailable? → `Target`.
5. **Manifest/registration?** Is the manifest, stage, capability, ownership, or
   routing table structurally invalid? → `Manifest`.
6. **Invalid source?** Is the input malformed or a C constraint violation? →
   `Semantic`.
7. **Well-formed but unimplemented?** Is the input valid C (or valid in the
   selected dialect) but outside the frozen subset? → `Unsupported`.
8. **Compiler defect?** Does verification prove a committed artifact violates an
   invariant, or is a global prerequisite/duplicate/ambiguous handoff missing?
   → `Internal`.
9. **Defensive "should not happen"?** → `Internal`, never `Task:4` and never a
   silent success.

### 4.4 Suggested code blocks

Existing codes are kept. New codes append; the numbers below are illustrative
but chosen to avoid collisions.

| Group | Existing (keep) | New block (suggested) | Notes |
|---|---|---|---|
| `Protocol` | `1`, `10`-`18` | `2`-`9`, `19`-`99` | `2` unknown task, `3` wrong kind, `4` payload shape, `5` reserved (not-running lives in `Task:6`), `6` owner, `7` invalid reference, `8` duplicate transition, `9` consume, `19` stage/layer, `20` request decode, `21` route/handler, `22` pre-dispatch selection |
| `Arena` | none | `1`-`99` | `1` out-of-bounds, `2` tombstone, `3` sentinel, `4` capacity, `5` total records, `6` source bytes, `7` intern entries, `8` intern bytes, `9` queue, `10` task count, `11` task depth, `12` diagnostics, `13` proposals, `14` constant bits, `15` stage queue |
| `Config` | `1`-`7` | `8`-`99` | `8` unsupported `wchar_t` model, `9` invalid dialect/target option, `10` invalid module configuration |
| `Target` | `1` | `2`-`99` | `2` probe/attestation invalid, `3` ABI/model mismatch, `4` codegen disabled |
| `Manifest` | `1` | `2`-`99` | `2` duplicate identity, `3` undeclared field, `4` owner violation, `5` stage/phase, `6` capability/tests, `7` routing registration |
| `Task` | `2`-`5`, `10` | `6`-`9`, `11`-`99` | `6` not running, `7` no transition, `8` progress non-advancing, `9` progress limit, `11` cancelled record, `12` join/wait state, `13` budget exhausted |
| `Semantic` | `1`-`2` | `3`-`99` core; `100`-`199` PP; `200`-`299` lex; `300`-`399` parse; `400`-`499` types/symbols; `500`-`599` semantics; `600`-`699` constants/layout/init; `700`-`799` directives/IR-source-facing | Keep `1` redeclaration, `2` undeclared; stage ranges make ownership obvious |
| `Unsupported` | `1`-`4` | `5`-`99` generic; `100`-`199` PP; `200`-`299` lex; `300`-`399` parse; `400`-`499` types/symbols; `500`-`599` semantics; `600`-`699` constants; `700`-`799` IR; `800`-`899` target/codegen; `900`-`999` extensions | `2`/`4` become legacy aliases; `3` is retired |
| `Internal` | `1` | `2`-`99` core; `100`-`199` PP/LX; `200`-`299` parse; `300`-`399` types/symbols; `400`-`499` semantics; `500`-`599` constants/IR; `600`-`699` verification; `700`-`799` control/commit; `800`-`899` target | `1` remains generic; `2` missing prerequisite, `3` duplicate producer, `4` ambiguous input, `5` verification failure, `6` recovery invariant, `7` diagnostic join miss |

If the integrator prefers not to add stage ranges, the fallback is a flat
append-only symbolic registry per group; the mapping rules in §4.5 are the
important part.

### 4.5 Mapping rules for current patterns

| Current pattern | Current code | Proposed code | Rationale |
|---|---|---|---|
| `"{label} task {} is not running"` | `Task:4` | `Task:6` | Lifecycle, not source |
| `"of unknown task {}"` / wrong kind | `Protocol:1` / `Task:4` | `Protocol:2` / `Protocol:3` | Caller/task identity |
| `"payload must ..."` / `"carries no token refs"` | `Task:4` / `Protocol:1` | `Protocol:4` | Payload shape |
| `"reads missing token/pp-token/span/source/literal {}"` where the ID came from the payload | `Task:4` | `Protocol:7` | Dangling payload reference |
| `"canonical int type is not committed"` / `"function type is not committed"` / `"unchecked return"` | `Task:4` | `Internal:2` | Missing global prerequisite produced upstream |
| `"duplicate canonical int type"` / `"duplicate function type"` / `"duplicate typed fact"` | `Task:4` | `Internal:3` | Duplicate producer |
| `"ambiguous folded constants"` / `"ambiguous ..."` | `Task:4` | `Internal:4` | Ambiguous handoff |
| `"DEFECT: ..."` / `"rescan circuit breaker"` / `"unreachable"` | `Task:4` | `Internal:1`/`3`/`6` | Defect, never source |
| `"unterminated character literal"` / `"unknown escape"` / `"invalid token"` | `Task:4` | `Semantic:200`-`299` | Malformed lexical input |
| `"malformed/unterminated/unbalanced ..."` in parse | `Task:4` | `Semantic:300`-`399` | Malformed parse input |
| `"duplicate symbol"` / `"undeclared"` / `"untyped"` / `"effectful"` | `Task:4` | `Semantic:1`/`2`/`500`+ | C constraint |
| `"non-M1 ..."` / `"deferred past this slice"` / `"not supported"` | `Unsupported:1` | `Unsupported:<stage>` | Well-formed C outside subset |
| `#error` message | `Task:4` | `Semantic:700` (or a dedicated symbolic code) | Source-requested error; not a task fault |
| `"not registered"` / `"has no handler installed"` | `Unsupported:1` | `Protocol:21` | Missing wiring, not a C gap |
| `"artifact needs N bytes, budget is M"` | `Config:1` | `Arena:6` | Runtime source-byte bound |
| `"needs utf32 wchar_t"` | `Config:1` | `Config:8` | Invalid model/configuration |
| `"option declared more than once"` | `Config:1` | `Config:1` (keep) | Genuine config |
| `CONST_OVERFLOW` (bit budget) | `Unsupported:2` | `Arena:14` | Configured bound; keep `CONST_OVERFLOW` as a legacy alias if needed |
| `CONST_NOT_CONSTANT_EXPRESSION` / `"expression is not a constant expression"` | `Unsupported:4` / `Task:3` | `Semantic:601` | C constraint; one code for one condition |
| Static-assert zero condition | `Task:3` | `Semantic:602` | C constraint failure, distinct from non-ICE |
| `CONST_UNSUPPORTED` | `Unsupported:3` (unused) | `Unsupported:6xx` or delete | No production use |
| Join reuses child diagnostic | child's code | child's code (preserve) | Correct behavior; formalize |
| Verifier rejection | `Task:4` | `Internal:5` | Candidate artifact violates invariant |
| `CommitError::Limit`/`Capacity`/`Intern` | `Protocol:1` / `Protocol:18` | `Arena:4`-`14` | Resource exhaustion |
| `ManifestError` variants | `Manifest:1` | `Manifest:2`+ (optional) | Per-family code if evidence needs it |
| `RegistryError` variants | `Task:2` | `Task:2` (keep) | Task-kind registry; could add subcodes |
| `RouteError` | none | `Manifest:7` | Routing table registration |
| `ProbeError` | none | `Target:2` | Probe/evidence gate |
| `ConfigError::code()` strings | string | keep as host alias; numeric `to_diagnostic` is authoritative in records | Avoid two evidence keys |

### 4.6 Missing codes for V1-V3

The following are the minimum semantic categories the three vertical trains
need. Exact numeric values remain a freeze decision; the symbolic names are the
stable part.

**V1 — integer expressions, locals, declarations, initialization, assignment,
conversions:**

| Area | Required symbolic codes | Group |
|---|---|---|
| Lexical malformation | `lex_invalid_token`, `lex_unterminated_literal`, `lex_invalid_escape`, `lex_invalid_number`, `lex_invalid_char`, `lex_invalid_string` | `Semantic:2xx` |
| Parse malformation | `parse_unexpected_token`, `parse_missing_token`, `parse_malformed_declarator`, `parse_malformed_expression`, `parse_unbalanced_delimiter` | `Semantic:3xx` |
| Types | `invalid_type_construction`, `type_incompatible`, `invalid_conversion`, `incomplete_type` | `Semantic:4xx` |
| Semantics | `redeclaration` (exists), `undeclared` (exists), `not_lvalue`, `invalid_assignment`, `invalid_operand`, `invalid_initializer` | `Semantic:1/2/5xx` |
| Constants | `not_constant_expression`, `constant_not_representable` | `Semantic:6xx` |
| Subset gaps | `unsupported_lex_*`, `unsupported_parse_*`, `unsupported_type_*`, `unsupported_conversion` | `Unsupported:*` |
| Prerequisites/defects | `missing_prerequisite`, `duplicate_producer`, `ambiguous_input` | `Internal:2/3/4` |
| Verification | `store_invariant`, `syntax_invariant`, `typed_invariant`, `interpret_unsupported` | `Internal:6xx` |
| Capacity | `arena_capacity`, `total_records`, `source_bytes`, `intern_entries`, `intern_bytes`, `diagnostics`, `const_bits` | `Arena:4`-`14` |
| Protocol/task | `wrong_task_kind`, `payload_shape`, `invalid_reference`, `not_running`, `no_transition` | `Protocol:3/4/7`; `Task:6/7` |
| Config | `duplicate_option`, `invalid_limit`, `wchar_model` | `Config:1/2/8` |

**V2 — `if`/`else`, loops, `break`/`continue`, short-circuit, CFG:**

| Area | Required symbolic codes | Group |
|---|---|---|
| Statement parse | `parse_malformed_statement`, `unexpected_else`, `missing_condition_delimiter` | `Semantic:3xx` |
| Control legality | `invalid_condition_scalar`, `break_continue_outside_loop`, `invalid_switch_case`, `duplicate_label`, `undefined_label` | `Semantic:5xx` |
| CFG/IR | `cfg_missing_terminator`, `cfg_unreachable_block`, `cfg_branch_arity`, `cfg_block_version` | `Internal:5xx` (or `CommitError::TerminatorMissing` legacy `Task:10`) |
| Progress/cancel | `progress_non_advancing`, `progress_limit`, `cancelled` | `Task:8/9/11` |
| Dispatch/backpressure | `selection_batch_overflow`, `backpressure_capacity` | `Protocol:10/11` (keep) or `Arena:15` |
| Subset gaps | `unsupported_statement_*`, `unsupported_control_*` | `Unsupported:3xx` |

**V3 — functions, parameters, calls, returns, globals, pointers, arrays:**

| Area | Required symbolic codes | Group |
|---|---|---|
| Function signature | `invalid_function_signature`, `parameter_count_mismatch`, `parameter_type_mismatch`, `invalid_return_type` | `Semantic:4xx` |
| Calls/expressions | `invalid_call`, `invalid_argument`, `invalid_dereference`, `invalid_address`, `invalid_member_access` | `Semantic:5xx` |
| Arrays/storage | `invalid_array_bound`, `incomplete_type`, `storage_linkage_conflict`, `qualifier_violation` | `Semantic:4xx` |
| ABI/target | `abi_mismatch`, `codegen_unverified`, `probe_missing` | `Target:2/3/4` |
| Layout/initialization | `invalid_layout`, `invalid_initializer`, `unsupported_vla` | `Semantic:6xx` / `Unsupported:6xx` |
| Verification | `ir_invariant`, `cfg_invariant`, `abi_invariant`, `evidence_incomplete` | `Internal:6xx` |
| Registration/ownership | `duplicate_id`, `duplicate_kind`, `store_owner_violation`, `stage_assignment` | `Manifest:2`-`7` |

### 4.7 Record, span, task, severity, and message policy

**[PROPOSED]**

1. **Auto-attach the enclosing task** in `commit_proposals` when the draft has
   `task: None`; workers may still override. This closes the CT11 "associated
   task" gap without trusting every chip.
2. **Require a span** when the input projection contains one (PP tokens, C
   tokens, nodes, semantic records). At minimum, every lexical/parse/semantic
   diagnostic in V1-V3 must carry the offending token span. A missing span on a
   source-facing diagnostic is a test failure, not a warning.
3. **Macro origin:** record the `SpanRecord.expansion` chain through the span;
   no new `DiagnosticDraft` field is required for V1-V3. If CT11 needs a
   consumer-facing origin list, add it as a derived evidence field, not a
   duplicated semantic store.
4. **Severity:** keep `Error` for V1-V3. `Warning`/`Note` are reserved until a
   non-fatal diagnostic wire (for `#warning` and diagnostics that must continue)
   is frozen.
5. **Dedup:** deduplicate by `(severity, group, code, span, task)` with a stable
   first-seen order; do not dedup by message (messages may vary by context).
   CT11's dedup requirement needs this definition.
6. **Message bounds:** cap the machine-record message (for example 4 KiB) and
   quoted excerpts (for example 128 bytes) with a stable truncation marker.
   `#error` preserves user text up to the cap. Never include host paths, env,
   clock, PIDs, pointer values, or toolchain stderr in a `DiagnosticRecord`.
7. **Evidence key:** `(group, code)` is stable; `message` is prose. Tests must
   not assert exact prose unless the prose is itself the product (`#error`).
8. **Capacity visibility:** expose a deterministic report counter or sentinel
   accounting for `DiagnosticId::NONE` so evidence can state how many terminal
   failures lacked a diagnostic.

### 4.8 Cancellation and recovery policy

**[PROPOSED]**

1. Normal cancellation is a control outcome, not an error. It should not mint a
   source diagnostic. `TickOutcome::Cancelled` plus a structured cancel reason
   in the control/report record is sufficient for evidence.
2. If a task must be terminated by cancellation, use one `Task` code
   (`Task:11 cancelled`) and do not fabricate a source span.
3. Commit-failure recovery keeps the `CommitError` code and attaches the task
   (current `fail_selected` behavior). Extend the same auto-attachment to
   worker `Fail` diagnostics.
4. A join failure reuses the first failed child's code/span/task exactly; do not
   rewrite the code when re-emitting the parent failure.
5. `DiagnosticId::NONE` remains the capacity sentinel, but evidence must count
   it and must not treat it as a successful terminal outcome.

---

## 5. Ownership and Contract Process

### 5.1 Proposed ownership

| Surface | Owner | Change gate |
|---|---|---|
| `diagnostic.rs` (`Severity`, `DiagGroup`, `DiagnosticCode`, draft/record shape) | T01 integrator | Contract change request; hash-impact review |
| `contract.rs` `DIAG_GROUP_NAMES` / `FrozenSchema::encode` / freeze test | T01 integrator | Any group add/remove/reorder is a hash bump |
| `commit.rs` `CommitError::code()`, diagnostic allocation, preflight, recovery | T01 integrator + T02 control owner | `c03_task`, `c06_routing`, `c07_limits` |
| `task.rs` `RegistryError`, `RequestError`, `ConstantResult::route` | T01 integrator + group owners | `c03_task`, `c08_gate1` |
| `manifest.rs` `ManifestError`/registry | T01 integrator | `c04_manifest` |
| `target.rs` `ConfigError`, `ProbeError` | T01 integrator + T11 target owner | `c02_target`; probe-gated values stay unverified |
| `routing.rs` `RouteError`, `Resolution`, `TickOutcome`, unsupported proposals | T01 integrator + T02 | `c06_routing` |
| `chips/mod.rs` `DriveError`, `protocol_fault`, shared diagnostic helpers | T01 integrator | Shared by all chip groups |
| `chips/<group>/*` | that group's chip owner | Only own files; codes requested from the integrator |
| CT11 diagnostic commit, CT12 recovery, CT13 cancel | T02 control owner | No `chips/control/` exists today; design-only |
| Verification code mapping (VF01/VF05/VF06/VF12/VF14) | T13 verification owner | `c17`-`c19`, `c38`, `c39` |
| Code registry (new symbolic table) | T01 integrator | Append-only; no chip owner edits it |
| Shared test entry points | T01 integrator | Tests updated in the same batch as the producer |

### 5.2 Contract-change rule

- A change that only **maps existing behavior to existing groups/codes** is a
  behavior change plus a test update; it needs an integrator-reviewed change
  request but not necessarily a hash bump.
- Adding/removing/reordering a `DiagGroup` **always** changes
  `DIAG_GROUP_NAMES` and the hash. It requires a full T01 freeze amendment and
  dependent-test rerun.
- Encoding numeric codes or the code registry into `FrozenSchema::encode` would
  change the hash. Recommendation: keep the registry **non-hashed** initially,
  version it in documentation and tests, and only hash it when the T01
  integrator accepts the stability trade-off.
- Any change that alters existing diagnostic snapshots (task attachment, spans,
  message truncation) must be treated as an evidence re-baseline, even if the
  contract hash does not move.

---

## 6. Migration Plan

### 6.1 Phases

| Phase | Scope | Exit criteria |
|---|---|---|
| **P0 Charter freeze** | Accept §4 group charters, decision procedure, code registry policy, V1-V3 minima | Integrator sign-off; no code or hash change |
| **P1 Machinery** | Typed constructors/registry; auto task attachment; Arena mapping for `Limit`/`Capacity`/`Intern`; `Config:1` split; `RouteError`/`ProbeError` mappings; diagnostics for capacity sentinel | `c03`, `c05`, `c06`, `c07` green; no existing semantic behavior changed beyond group/code |
| **P2 V1** | Migrate lex, parse, types, symbols, semantics, constants, IR, verify for V1 paths; add taxonomy/attachment tests | V1 fixtures and `c08`, `c10`-`c19`, `c32`-`c33`, `c34`-`c35`, `c37`, `c39` green; no `Task:4` left in migrated V1 files except explicit legacy aliases |
| **P3 V2** | Control-flow parse/semantic/IR; cancel/progress/backpressure; CFG invariants | V2 fixtures; `c06`, `c03`, `c07`; new cancel/CFG tests green |
| **P4 V3** | Functions, calls, globals, pointers, arrays, ABI/target; verification | V3 fixtures; ABI/probe-gated cases stay blocked, not faked; `c18`, `c19`, `c38`, `c39` green |
| **P5 Cleanup** | Retire `Task:3/4/5`, `CONST_UNSUPPORTED`, `Config:1` overloads; add lint to reject raw `Task:4`/`Unsupported` misuse | Zero production `Task:4`/`Task:5`/`Task:3` (except kept aliases), zero misgrouped constants, relevant suite green |

### 6.2 Migration table

| # | Legacy pattern (approx. sites) | Current | Proposed | Affected files/chips | Tests to update | Risk |
|---|---|---|---|---|---|---|
| M1 | Worker "not running" (60) | `Task:4` | `Task:6` | All chip groups; shared helper in `chips/mod.rs` | `c03`, `c34`, `c36`, `c39` (group stays `Task`) | Low |
| M2 | Payload-referenced missing/tombstoned record (≈100) | `Task:4` | `Protocol:7` | PP/lex/parse projectors | `c21`, `c24`, `c27`, `c34`, `c36`, `c39` | Medium (group `Task`→`Protocol`) |
| M3 | Missing global prerequisite / duplicate producer / ambiguous handoff (≈40-50, cross-bucket) | `Task:4` | `Internal:2/3/4` | `se_function`, `ir_function`, `ty_*`, `vf_*` | `c14`, `c15`, `c39` | Medium (group `Task`→`Internal`) |
| M4 | Malformed lexical input (≈48) | `Task:4` | `Semantic:2xx` | `lex/*` | `c20`, `c32`, `c33`, `c11` | Medium |
| M5 | Malformed parse input | `Task:4` | `Semantic:3xx` | `parse/*` | `c12`, `c34`, `c35`, `c36` | Medium |
| M6 | C constraint violations (16) | `Task:4` | `Semantic:1/2/5xx` | `se_*`, `ty_symbol`, `ir_function` | `c13`, `c14`, `c39` | Low/medium |
| M7 | DEFECT/duplicate/ambiguous (10) | `Task:4` | `Internal:3/4` | `parse/*`, `ir_function`, `pp_*` | Inline chip tests | Low |
| M8 | Verifier rejection (≈20) | `Task:4` | `Internal:5` | `verify/*` | `c17`-`c19`, `c38` | Medium |
| M9 | "Not a constant expression" | `Task:3` / `Unsupported:4` | `Semantic:601` | `task.rs`, `fold_branch.rs` | `c37`, `c08` | High (exact codes) |
| M10 | Constant-bit budget | `Unsupported:2` | `Arena:14` | `fold.rs`, `fold_branch.rs` | `c08`, `c37` | High (exact codes) |
| M11 | Unsupported constant form | `Unsupported:1` / `Unsupported:3` | `Unsupported:6xx` | `fold.rs`, `fold_branch.rs`, `se_*` | `c08`, `c37` | Low |
| M12 | Routing "not registered"/"no handler" | `Unsupported:1` | `Protocol:21` | `routing.rs` | `c06` | Medium |
| M13 | `Config:1` overloads | `Config:1` | `Arena:6` (artifact bytes) / `Config:8` (wchar) | `pp_normalize.rs`, `pp_emit.rs`, `lx_string.rs` | `c31`, `c33` | Medium |
| M14 | Generic `Internal:1` | `Internal:1` | `Internal:3/5/6/7` | `pp_emit.rs`, `pp_substitute.rs`, `pp_variadic.rs` | `c24`, `c26`, `c31` | Low |
| M15 | Commit capacity/limit/intern | `Protocol:1` / `Protocol:18` | `Arena:4`-`14` | `commit.rs` | `c03`, `c07` | High (many variants) |
| M16 | No task attribution on worker `Fail` | `None` | Auto-attach enclosing task | `commit.rs` | `c03`, `c05`, `c13`, all chip tests | Low code, high evidence impact |
| M17 | Sparse spans | 3 sites | Attach where input carries span | `lex/*`, `pp/*`, `parse/*`, `se_*`, `ir_*` | `c05`, `c32`, `c33` | Medium (snapshot re-baseline) |
| M18 | Cancellation has no reason record | `JobState::Failed` + `TickOutcome` | `Task:11` only if a record is required; otherwise structured control report | `routing.rs`, future control chips | `c06` | Open (T01/T02) |
| M19 | `Manifest:1`/`Task:2` single codes; string `code()` aliases | `Manifest:1`, `Task:2`, `config.*`/`limits.*` strings | Optional per-family codes; keep strings as host aliases | `manifest.rs`, `task.rs`, `target.rs`, `limits.rs` | `c04`, `c03`, `c02` | Low |
| M20 | Dead `CONST_UNSUPPORTED` | `Unsupported:3` | Delete or map to `Unsupported:6xx` | `diagnostic.rs` | `c08` (if referenced) | Low |

**Snapshot/evidence caveat.** M16 (task attachment) and M17 (span attachment)
change the serialized diagnostic body and therefore existing snapshot/trace
hashes, even though the contract hash is unchanged (`snapshot.rs:499-516`,
`:903-925`). These changes must be batched with an explicit evidence
re-baseline and recorded in the ledger; they must not be slipped into an
unrelated freeze.

### 6.3 Affected chips and tests

**Chip files (production-like counts from §2.4):**

- `chips/preprocess/*` (20 files): 162 `Task:4`, 22 `Unsupported:1`; highest
  migration volume. Start with the repeated "not running"/"missing pp-token"
  helpers, then directive/pragma/macro paths.
- `chips/parse/*` (16 files): 41 `Task:4`, 27 `Unsupported:1`; focus on V1
  declarator/expression/recovery, then V2 statements.
- `chips/lex/*` (10 files): 39 `Task:4`, 19 `Unsupported:1`; lexical malformed
  vs unsupported is the clearest V1 split.
- `chips/semantic/*` (4 files): 31 `Task:4`, 17 `Unsupported:1`; distinguish
  constraint violations from M1 gaps.
- `chips/verify/*` (5 files): 25 `Task:4`, 2 `Unsupported:1`; move verifier
  rejections to `Internal`.
- `chips/symbols/*` (2 files): 18 `Task:4`, 4 `Unsupported:1`; scope lifecycle
  and symbol constraints.
- `chips/ir_lower/*` (1 file): 13 `Task:4`, 5 `Unsupported:1`; duplicate/missing
  prerequisite and unsupported node/op.
- `chips/constant_layout_init/*` (2 files): 5 `Task:4`, 3 `Unsupported:1`;
  constant-not-constant and bit budget.
- `chips/types/*` (2 files): 5 `Task:4`, 2 `Unsupported:1`.
- `chips/mod.rs`: shared `protocol_fault`; `DriveError` should map to
  `Protocol`/`Manifest` codes.

**Tests that pin exact codes and must be updated in the same batch:**

- `compiler/tests/c03_task.rs:980-1040`: `Protocol:10`-`18`, `Task:10`,
  `Protocol:1`; `:1470-1530`: empty-proposal and `NONE` sentinel.
- `compiler/tests/c08_gate1.rs:1251-1290`: `CONST_OVERFLOW` and
  `Unsupported` group.
- `compiler/tests/c37_const_branch.rs:378-454`: `Unsupported:1`, `Task:4`,
  `Task:3`, `CONST_NOT_CONSTANT_EXPRESSION`, `CONST_OVERFLOW`.
- `compiler/tests/c02_target.rs:242-245`: `Config` code `2` and string
  `code()` aliases.
- `compiler/tests/c04_manifest.rs:278-292`: `Manifest` group only.
- `compiler/tests/c13_ty.rs:678-731`: `Semantic` group only.
- `compiler/tests/c34_parse.rs:429-618`, `c36_recovery.rs:404-431`,
  `c39_sefunc.rs:643-772`: group-only `Task`/`Unsupported` assertions; many
  survive M2-M8 only if the target group is unchanged.
- `compiler/tests/c05_codec.rs:137-149`: diagnostic messages affect snapshot
  hashes.
- `compiler/tests/c06_routing.rs:104`: diagnostic live count; `c07_limits.rs:180-204`:
  `max_diagnostics`.
- `compiler/tests/c21`, `c24`, `c27`, `c28`, `c29`, `c30`, `c31`: message-content
  assertions; review under the message-bound policy.
- `compiler/tests/h04_candidate.rs`: exit codes and diagnostic printing.
- `compiler/tests/freeze.rs:246-259`: group names/order; only changes if a group
  is added/removed/reordered.

**Tests that should be added (proposal; integrator owns test entry points):**

- `compiler/tests/diag_taxonomy.rs`: unique symbolic codes; no code in a
  reserved block; every production `DiagnosticCode` is a named constant; group
  charter assertions; legacy aliases documented.
- `compiler/tests/diag_attachment.rs`: every source-facing `Fail` has a task and,
  where the input has a span, a span; join reuse preserves code/span/task.
- `compiler/tests/diag_capacity.rs`: `ArenaError`/`LimitError`/`InternError`
  map to `Arena`; `NONE` sentinel is counted; `max_diagnostics` paths.
- `compiler/tests/diag_cancel.rs`: cancel is idempotent; no fabricated source
  diagnostic; exactly one terminal control outcome; in-flight tasks are not left
  `Running`.
- `compiler/tests/diag_evidence.rs`: deterministic bounded messages; no host
  path/env/timestamp/pointer patterns; `#error` preserved; snapshot stable.
- `compiler/tests/diag_recovery.rs`: PA38/CT12 preserves the original diagnostic;
  no duplicate diagnostic on retry.

### 6.4 Verification commands for future work orders

Use the narrowest relevant subset, for example:

```text
cargo test --manifest-path compiler/Cargo.toml --test c03_task --test c05_codec --test c06_routing --test c07_limits
cargo test --manifest-path compiler/Cargo.toml --test c08_gate1 --test c37_const_branch
cargo test --manifest-path compiler/Cargo.toml --test c34_parse --test c36_recovery --test c39_sefunc
cargo test --manifest-path compiler/Cargo.toml --test c21_directive --test c24_expand --test c27_builtin --test c29_pragma --test c31_emit
```

The full suite is for batch integration, not for a documentation-only audit.
Any code migration must report the exact commands actually run and any unrun
checks.

---

## 7. Test Implications and Acceptance

1. **Do not weaken exact-code assertions to group-only assertions during
   migration.** If a code is intentionally unstable, say so and keep a stable
   symbolic name; do not delete the assertion.
2. **Update producer and test in one batch.** A code change without its test
   update is a defect; a test update without a producer change is test-only
   rewriting and must be reviewed as such.
3. **Preserve `Unsupported` never-pass tests.** `vf12.unsupported-never-pass`
   and the VF14 evidence rules must keep `Unsupported` distinct from PASS
   (`T13_VERIFICATION_CHIPS.md:81`, `:85`).
4. **Evidence re-baseline is explicit.** M16/M17 (task/span attachment) and
   message-bound changes alter snapshot hashes; record the new baseline and the
   reason. Do not compare old and new hashes as if nothing changed.
5. **Capability tests stay separate from diagnostics.** A `Task:4`→`Protocol`
   migration must not turn an unimplemented feature into a pass; the
   `Unsupported` path must still fail closed.
6. **No message-only tests for machine decisions.** Keep message assertions for
   `#error` and user-facing prose, but assert `(group, code)` for semantic
   decisions.
7. **Acceptance for P1:** existing pinned codes either stay identical (legacy
   alias) or are changed with a reviewed migration row; `c03`, `c05`, `c06`,
   `c07` green.
8. **Acceptance for P2-P4:** the train's fixtures are green, no unclassified
   `Task:4` remains in migrated files, and every new failure path has a named
   code, a task, and (where available) a span.

---

## 8. Open Questions for the Integrator

| # | Question | Recommendation | Owner |
|---|---|---|---|
| Q1 | Should `Semantic` absorb all source-language faults, or should a new `Invalid`/`Syntax` group be added? | Absorb now; no hash bump. Revisit a dedicated group only if `Semantic` becomes unmanageable. | T01 + owners |
| Q2 | Should numeric codes enter the contract hash/registry now? | Keep the registry non-hashed in P0; hash only after stability is measured. Document the drift risk. | T01 |
| Q3 | Is a lexical/parse malformation `Semantic` or `Task`? | `Semantic` (language input fault). `Task` is lifecycle. | T01 + T03/T05 |
| Q4 | Missing committed record: `Protocol` or `Internal`? | Payload-referenced missing record → `Protocol:7`; global prerequisite/duplicate producer → `Internal:2/3`. | T01 + owners |
| Q5 | `CONST_OVERFLOW`: `Arena` or `Semantic`? | `Arena:14` for the `max_const_bits` budget; use a separate `Semantic` code if target-width representability is later distinguished. | T08 + T01 |
| Q6 | `CONST_NOT_CONSTANT_EXPRESSION` and `Task:3`: one code or two? | One `Semantic` code for "not a constant expression"; a separate code for a failed `_Static_assert` condition. | T08 + T01 |
| Q7 | `CONST_UNSUPPORTED`: keep or delete? | Delete the dead constant or alias it to the stage `Unsupported` code; do not leave an unused named code. | T01 |
| Q8 | Should cancellation produce a `DiagnosticRecord`? | No for normal cancel; add `Task:11` only if the control contract requires a terminal reason record. Define in-flight task disposition first. | T02 + T01 |
| Q9 | Should commit auto-attach `task` to worker `Fail` diagnostics? | Yes. It closes the CT11 gap and is cheap; it changes snapshots, so batch with an evidence re-baseline. | T01 |
| Q10 | Which spans are mandatory? | Every source-facing lexical/parse/semantic diagnostic must carry the offending token span; PP/IR diagnostics carry the best committed span. | T01 + T03-T09 |
| Q11 | What is the dedup key and ordering? | `(severity, group, code, span, task)`, first-seen ordinal; messages are not part of the key. | T02 + T01 |
| Q12 | Are `Warning`/`Note` part of the stable surface now? | Keep them reserved; add a non-fatal diagnostic wire before producing them (`#warning` is the first use case). | T02 + T03 |
| Q13 | How are `RouteError`, `ManifestRegistryError`, and `ProbeError` surfaced? | Build-time/host errors map to `Manifest:7`/`Target:2`; do not leave them without a diagnostic mapping if they enter the bus. | T01 + T11 |
| Q14 | Do `config.*`/`limits.*` string codes remain? | Yes, as host-facing aliases; the numeric `(group, code)` is authoritative in records. Add a mapping test. | T01 |
| Q15 | What message bound and quoting policy is evidence-safe? | Propose 4 KiB total / 128-byte excerpt / stable truncation marker; forbid host paths, env, time, pointers, toolchain stderr. | T01 + T02 |
| Q16 | When is a verification rejection `Internal` vs `Target`? | Candidate artifact invariant failure → `Internal`; external probe/attestation/ABI evidence failure → `Target`. | T13 + T11 |
| Q17 | Should the PP directive taxonomy (`pp.diagnostic-taxonomy-frozen`) get explicit numeric codes? | Yes: `#error` → `Semantic:700`; other names → stage `Unsupported` codes; malformed → `Semantic:701`. | T03 + T01 |
| Q18 | Are stage-range code blocks wanted, or a flat per-group registry? | Stage ranges make ownership obvious; a flat registry is simpler. The mapping rules matter more than the numbering. | T01 |

---

## 9. Bottom Line

The compiler does not currently have a diagnostic taxonomy in the contractual
sense. It has a nine-name group enum that is hash-frozen, plus a large body of
ad hoc numeric assignments. The two dominant codes are `Task:4` (339 production
sites) and `Unsupported:1` (106 production sites); `Arena` is unused; `Invalid`
has no home; capacity errors lose their class; spans/tasks/notes/dedup/warnings
are mostly missing; cancellation and verification have no dedicated
representation.

The minimal stable fix is **not** a new group enum. It is:

1. Freeze the group charters in §4.2 and the ordered decision procedure in §4.3.
2. Freeze an append-only symbolic code registry (non-hashed initially) with
   stage ranges for `Semantic`/`Unsupported`/`Internal`.
3. Map `Invalid` to `Semantic`, capacity to `Arena`, producer protocol to
   `Protocol`, lifecycle to `Task`, compiler/verification defects to `Internal`.
4. Auto-attach the enclosing task, require spans where the input carries them,
   define dedup and message bounds, and count the `NONE` sentinel.
5. Migrate V1 first, then V2/V3, updating exact-code tests in the same batch and
   treating task/span attachment as an evidence re-baseline.

No source, test, contract, Cargo, `mod.rs`, or shared index was modified by this
audit. The only file created is this report.

---

## Appendix A. Representative Evidence Index

| Topic | Path:line |
|---|---|
| Group enum | `compiler/src/diagnostic.rs:37-57` |
| Code struct | `compiler/src/diagnostic.rs:76-101` |
| Draft/record | `compiler/src/diagnostic.rs:105-177` |
| Unsupported helper | `compiler/src/diagnostic.rs:130-133` |
| Group names in hash | `compiler/src/contract.rs:325-335`, `:552`; `compiler/tests/freeze.rs:246-259` |
| Commit error variants | `compiler/src/commit.rs:97-313` |
| Commit code mapping | `compiler/src/commit.rs:447-482` |
| Fail commit | `compiler/src/commit.rs:1466-1477` |
| Capacity fail fallback | `compiler/src/commit.rs:2830-2868` |
| Preflight diagnostics | `compiler/src/commit.rs:969`, `:2059`; `compiler/src/bus.rs:1081-1090` |
| Cancel | `compiler/src/routing.rs:611-625`, `:654-668`; `compiler/src/bus.rs:803-804` |
| Recovery `fail_selected` | `compiler/src/routing.rs:565-591` |
| Unsupported routing | `compiler/src/routing.rs:421-474` |
| Registry/request errors | `compiler/src/task.rs:570-631`, `:1329-1390` |
| Constant route | `compiler/src/task.rs:1523-1543` |
| Manifest errors | `compiler/src/manifest.rs:434-639` |
| Config errors | `compiler/src/target.rs:1096-1224` |
| Probe errors | `compiler/src/target.rs:721-801` |
| Drive errors | `compiler/src/chips/mod.rs:191-251` |
| Protocol helper | `compiler/src/chips/mod.rs:426-430` |
| "Invalid" comment | `compiler/src/chips/lex/lx_error.rs:81-92` |
| Pragma "Invalid" | `compiler/src/chips/preprocess/pp_pragma.rs:202-225` |
| Constraint helper | `compiler/src/chips/preprocess/pp_substitute.rs:456-470` |
| Join reuse | `compiler/src/chips/preprocess/pp_variadic.rs:721-742` |
| Verification rejection | `compiler/src/chips/verify/vf_evidence.rs:414-423` |
| Snapshot diagnostics | `compiler/src/snapshot.rs:499-516`, `:903-925` |
| Candidate evidence | `compiler/src/bin/candidate.rs:17-19`, `:355-362`, `:900-974` |
| PP taxonomy doc | `docs/tasks/PP_DIRECTIVE_SLICE.md:35-39`, `:88` |
| CT11-CT13 rows | `docs/tasks/T02_CONTROL_CHIPS.md:118-120` |
| T01 diagnostics partition | `docs/tasks/T01_COMPILER_CONTRACT.md:48` |
| T01 C06 task attachment | `docs/tasks/T01_COMPILER_CONTRACT.md:881` |
| Exact-code tests | `compiler/tests/c03_task.rs:980-1040`; `c08_gate1.rs:1251-1290`; `c37_const_branch.rs:378-454` |
| Group-only tests | `compiler/tests/c04_manifest.rs:278-292`; `c13_ty.rs:678-731`; `c34_parse.rs:429-618`; `c36_recovery.rs:404-431`; `c39_sefunc.rs:643-772` |
