# Documentation Review: 2026-10-05

## Status and scope

- Status: review record; findings have not been fixed or closed item by item. This document is not an ADR, contract approval, interface freeze, or implementation authorization.
- Review date: 2026-10-05.
- Baseline: the working tree at review time, including the pre-existing uncommitted and untracked documents, not limited to Git HEAD. Line numbers correspond to the versions read at review time; later edits may cause drift.
- Scope: root README, architecture and design documents, the compiler task catalog, the compiler contract and README, the torture tooling and target-probe documents; the relevant implementation and test sources were cross-checked where necessary.
- Method: a first round of 5 subagents auditing by domain in parallel, then a second round of 3 subagents doing targeted audits while excluding problems already recorded in the first round. Both rounds were cross-checked by the main agent on the key findings.
- This review round did not modify the audited files and did not commit code. This document only preserves the review results.

Overall conclusion: the documentation keeps the distinction between "implemented", "accepted", and "proposed" reasonably clear in general, but there remain problems where the contract guarantees do not match the implementation and where the acceptance steps contradict themselves. These should be corrected before freezing `/6`. Gaps explicitly marked open in the candidate contracts should not be read as defects of an already-implemented compiler.

## Priority fixes

### DOC-01: The commit-permission guarantee is stronger than the actual checks

- Severity: high.
- Status: pending.
- Location: `compiler/contracts/COMPILER_SFL_MANIFEST.md:97–110`.
- Problem: the document claims that a producer is rejected when it has not declared the task kind it wants to commit, but in fact only `StorePatch` checks the producer registration and kind; `Complete`, `Fail`, and `AwaitHost` have no equivalent check. `Enqueue` checks the destination chip registration and the destination kind.
- Evidence: `compiler/src/commit.rs:384–475,608–617`; `compiler/tests/c03_task.rs:104–118` completes a basic task with no registered producer manifest.
- Recommendation: unify the producer validation for all proposals; if basic tasks are intended as an exception, state the exception scope explicitly and add a negative case for an unregistered or non-accepting kind per proposal.

### DOC-02: The Part A acceptance command violates the target-probe gate

- Severity: high.
- Status: pending.
- Location: `docs/tasks/M1_TARGET_ACCEPTANCE.md:466–479`.
- Problem: Part A requires running `candidate … -S -o main.s`, but lines `330–341` of the same document require target code generation to be refused while the target is unverified, and Part A generates no target code. Following these steps either fails Part A or breaks the gate.
- Recommendation: Part A should only emit snapshots/IR and interpret them; move `-S` and the assembly-hash comparison to Part B after the probe is complete, together with assembling and linking.

## Other substantive problems

### DOC-03: The C-language judgement in a negative fixture is wrong

- Severity: medium.
- Status: pending.
- Location: `docs/tasks/M1_VERTICAL_SLICE_ACCEPTANCE.md:306`.
- Problem: the undeclared object `x` in `return x+3;` cannot be judged by the GNU89 implicit-function-declaration rule. The current acceptance oracle may accept the wrong behavior as a pass.
- Recommendation: require an undeclared-identifier diagnostic; test implicit function declaration separately with a call expression and declare the dialect policy explicitly.

### DOC-04: The T02 package permission ceiling is insufficient for its own tasks

- Severity: medium.
- Status: pending.
- Location: `docs/tasks/T02_CONTROL_CHIPS.md:3,99,108,111`.
- Problem: the package-level ceiling only permits reading `config/control/tasks` and writing its own control records, but CT02 imports sources, CT11 handles diagnostics, and CT14 reads artifacts. T01 treats the package-level envelope as the permission ceiling, so refining per-chip fields alone cannot remove the contradiction.
- Recommendation: complete the package-level permission ceiling and keep per-chip manifests to narrow the concrete fields, preserving commit-only mutation.

### DOC-05: The macro-expansion dependency graph puts pasting before argument substitution

- Severity: medium.
- Status: pending.
- Location: `docs/tasks/T03_PREPROCESS_CHIPS.md:19–23,38`.
- Problem: the dependency graph reads `…11/13/14→12→15`, but PP14 must use PP12's substituted argument tokens/placemarkers. Implementing `CAT(a,b)` in that order easily pastes the parameter names instead of the argument tokens.
- Status boundary: lines `42–44` of the same document already mark that table and the scheduling text as historical; this item is a problem where outdated guidance may still mislead an implementer, not a defect of a currently implemented preprocessor.
- Recommendation: state explicitly that raw-argument substitution at `##` positions comes first, then pasting/placemarker processing, then rescanning; distinguish pre-expansion of ordinary arguments. Annotate the historical table with a superseding dependency graph.

### DOC-06: The comment-replacement task omits the literal-protection constraint

- Severity: medium.
- Status: pending.
- Location: `docs/tasks/T03_PREPROCESS_CHIPS.md:11–12,38`.
- Problem: PP03 replaces comments before token scanning but does not state the requirement to protect string literals, character constants, and the related header-name context. A naive byte-wise replacement can break `"https://example"` or `"/*not a comment*/"`.
- Recommendation: define the scan states and the collaboration contract with PP04, and add tests for the literals above, escaped quotes, include context, and line splicing.

### DOC-07: An accepted ADR incorrectly claims the whole framework is heap-free

- Severity: medium.
- Status: pending.
- Location: `docs/architecture/ADR-0001-COMPILER-DYNAMIC-ARENA.md:5–8,50–51`.
- Problem: the ADR describes the root framework and the example as heap-free, but `Motherboard` actually uses `Vec` and `Box` and allocates during construction/installation.
- Evidence: `src/motherboard.rs:20–38,57–71`.
- Recommendation: distinguish the fixed-layout constraint for an ordinary application bus, allocation during topology initialization, and the fact that the default tick scheduler itself does not allocate. Do not widen a bus-storage constraint into a whole-framework resource guarantee.

### DOC-08: The probe's fail-closed description does not cover behavioral self-check failures

- Severity: medium.
- Status: pending.
- Location: `tools/torture/probe/README.md:23–34`.
- Problem: a failed ABI self-check only prints `.ok=0` and the program still returns 0; the normalizer merely copies these fields, and the harness does not reject the report on their basis.
- Evidence: `tools/torture/probe/src/abi-args.c:139–150`, `tools/torture/probe/normalize/normalize.py:362–375`; a subagent changed `abi.gp_ten.ok` to `0` in a synthetic capture and observed that `build_report()` still accepts it.
- Recommendation: return a non-zero status on self-check failure and validate the required `.ok` fields; otherwise state explicitly that successful report generation does not mean the self-checks passed. A synthetic-capture check is not a real AArch64 probe execution.

### DOC-09: The complete SFL example violates its own field rules

- Severity: medium.
- Status: pending.
- Location: `docs/architecture/SFL_SCHEMA_DRAFT.md:295–324`.
- Problem: `EdgeDetect` reads `prev_pulse`, but the bus never declares that register and its latch update is not described, which is inconsistent with the field-reference rules in the same document.
- Recommendation: complete the declaration, initial value, and update semantics so the example can be accepted by a future validator. This item is an internal inconsistency in a draft example, not a defect of an implemented validator.

## Second-round findings

This round adds 9 items, with numbering continuing from the first round. Contradictions in candidate contracts do not mean the corresponding language chips are implemented, and they do not constitute authorization to modify an accepted decision or to freeze `/6`.

### DOC-10: Literal decoding has a prerequisite dependency cycle

- Severity: high.
- Status: pending; T04 `/6` candidate-contract problem.
- Location: `docs/tasks/T04_LEX_CHIPS.md:84–86,115,158`.
- Problem: the literal-decode subtask requires a committed C `TokenId`, but the token and the decoded literal must be published in the same task append batch. There is no such committed token before decoding; publishing the token first violates the same-batch requirement.
- Evidence: the whole-batch ID-prediction mechanism at `87–90` of the same document can only resolve reciprocal links at commit time and cannot supply the committed input the next subtask needs earlier.
- Recommendation: decode from the spelling/kind of the committed PP token and resolve the C-token back-link at publication time; or explicitly design a token-first publish/update protocol. Add an integration fixture that covers decode-input availability and publication order.

### DOC-11: Global resource limits can be bypassed through public mutation APIs

- Severity: high.
- Status: pending; the current documented guarantee does not match the implementation boundary.
- Location: `compiler/README.md:130–133`; `docs/tasks/T01_COMPILER_CONTRACT.md:152`.
- Problem: the documentation promises that every configured limit is checked before mutation, but the public `bus.arenas` can allocate directly; an arena checks only its own capacity, not the total record count, source bytes, task count, or diagnostic budget. Allocating into several arenas separately can exceed the global record budget while staying within each per-arena capacity; the bytes of an existing source can also grow through public mutable access.
- Evidence: `compiler/src/bus.rs:101–147,355`; `compiler/src/arena.rs:275–287,305–310,377–389`. `compiler/tests/c07_limits.rs:55–106` covers the checked wrapper paths and establishes no budget guarantee for all public mutation entry points.
- Recommendation: concentrate budget-aware mutation behind a single encapsulated entry point; or state explicitly that the guarantee applies only to the checked bus/commit paths and record the boundary that trusted integration code can bypass. An unconditional global guarantee cannot be supported by wrapper-path tests alone.

### DOC-12: The point of declaration for declarations is taken at the wrong place

- Severity: medium.
- Status: pending; T06 `/6` candidate semantic-rule problem. The current M1 positive fixture does not expose the error.
- Location: `docs/tasks/T06_SYMBOL_TYPE_CHIPS.md:37–41`; `docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md:1300–1319`.
- Problem: the candidate rule uses the position of the identifier leaf as the point of declaration. C11 §6.2.1 ¶7 specifies for an ordinary identifier that its scope begins after the end of the complete declarator, not immediately after the name appears.
- Counter-example: in the code below the array bound must look up the outer `n`, because the inner declarator is not yet complete; judging by identifier position would expose the inner `n` too early.

```c
int n = 3;
void f(void) {
    int n[n];
}
```

- Recommendation: keep the identifier leaf as the declaration identity and diagnostic position, and derive the semantic visibility boundary separately from the complete declarator; add a contrast test between an array bound and an initializer lookup. If the rule only applies to the restricted M1 subset, state explicitly that it must not be read as a general C lookup rule.

### DOC-13: The scope-uniqueness rule rejects legal sibling scopes

- Severity: medium.
- Status: pending; T06 `/6` candidate structural-invariant problem.
- Location: `docs/tasks/T06_SYMBOL_TYPE_CHIPS.md:39`; `docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md:1236–1241`.
- Problem: duplicate live `(parent, kind)` is forbidden, but two function bodies or sibling blocks can have the same parent and the same `Block` kind. A closed scope keeps its record, so detecting duplicates through a live arena record does not resolve this collision.
- Evidence: `docs/tasks/T06_SYMBOL_TYPE_CHIPS.md:72` requires a closed scope to keep its record; the two blocks of `int f(void){return 1;} int g(void){return 2;}` need independent scopes.
- Recommendation: distinguish scopes by the owning lexical node or another explicit identity and reject duplicate creation for the same owner instead of the same `(parent, kind)`. If only the single-function cardinality of M1 is limited, separate the fixture restriction from the general structural rule.

### DOC-14: The claim that every legal bit budget accommodates the M1 constants does not hold

- Severity: medium.
- Status: pending; T08 M1 candidate-assertion problem.
- Location: `docs/tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md:10,39–41,46`.
- Problem: the document permits budgets less than or equal to 128 yet claims that `2`, `3`, and `5` are representable for every legal `max_const_bits`. With a budget of 1, the magnitude of `2` already needs 2 bits and `5` needs 3 bits, before any sign representation is counted.
- Status boundary: the general signed-range formula remains an unselected draft; this item does not treat that formula as an accepted rule. It points out that withdrawing the formula does not establish an alternative universal representability guarantee.
- Recommendation: limit successful M1 acceptance to a sufficient budget (for example the default 128); under an insufficient budget, verify a typed overflow/capacity result. The configuration minimum must not be raised without approval.

### DOC-15: The optional-artifact source policy is not synchronized into the commit algorithm

- Severity: medium.
- Status: pending; cross-file `/6` candidate consistency problem.
- Location: `docs/tasks/CONTRACT_CHANGE_REQUEST_M1_PIPELINE_AND_PART_A_SCHEMA.md:136`; `docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md:2460–2462,2540–2544`.
- Problem: the CDR rev-47 candidate default already allows a map-optional artifact to carry a valid `Some(source)` and requires empty offsets, but the proposal's commit algorithm still rejects any source on an optional kind and requires `source=None`.
- Counter-example: a `Trace` artifact with a valid source and empty `raw_offsets` matches the candidate default but is rejected by the algorithm.
- Recommendation: synchronize the algorithm with the draft validation table: optional kinds require empty offsets, and `Some(source)` should have its validity checked rather than being rejected outright. `Trace` is not in the M1 positive-fixture production scope, but a synthetic contract test of the declared total schema must still be consistent.

### DOC-16: The ABI probe's aggregate classification and HVA coverage claim are inaccurate

- Severity: medium.
- Status: pending.
- Location: `tools/torture/probe/src/abi-args.c:38–42,64–68`; `tools/torture/probe/README.md:23–26`.
- Problem: the non-homogeneous 16-byte aggregate `struct { long i; double d; }` is described as a GP+FP split. Under the AAPCS64 classification of the frozen target, its arguments use consecutive GP registers and its return uses `x0/x1`; the presence of a double does not by itself move the representation into FP registers. The README also claims HVA coverage, yet the source has no short-vector aggregate fixture.
- Recommendation: correct the aggregate-classification description and distinguish mixed scalar arguments from aggregate classification; add a real HVA fixture or state explicitly that HVA is untested. This incorrect description must not be used to explain assembly evidence or to claim that the corresponding ABI class is covered.

### DOC-17: Purity and totality are written as unconditional guarantees

- Severity: medium.
- Status: pending.
- Location: `README.md:325–328`.
- Problem: `Because F is pure and total` is stated as an existing guarantee, but the public API accepts any `LogicChip` and guarantees neither purity, termination, nor the absence of panics. The ordinary integer addition in the README example also has no explicit overflow result at boundary states.
- Evidence: `src/chip.rs:29–36`; `src/backend.rs:31–40`; `README.md:231,244`. Lines `129–131` of the same document already acknowledge that Rust cannot prove the absence of I/O or nondeterminism.
- Recommendation: restate the mathematical model as a conditional property satisfied by chips, hooks, adapters, and backends that respect purity, determinism, termination, and an explicit error/overflow contract; distinguish replay tests from proof.

### DOC-18: The H00 status document overstates the strength of the provenance check

- Severity: medium.
- Status: pending.
- Location: `docs/tasks/T00_H00_IMPLEMENTATION_STATUS.md:107–110`.
- Problem: the status record describes the provenance revision token as a checked condition, but in fact a missing token only produces a warning and does not make verification fail even under a frozen lock.
- Evidence: `tools/torture/src/lock.rs:1337–1347`, `tools/torture/src/verify.rs:68–95`; `tools/torture/README.md:112–116` correctly describes it as a policy nudge.
- Recommendation: state explicitly that it is warning-only and that successful frozen verification establishes no per-asset revision linkage; a stronger freeze gate requires separate approval, implementation, and tests.

## Design questions that must still be closed before the `/6` freeze

The following items record unclosed paths in the proposal (a resolution direction and test requirements are recorded for OPEN-01/OPEN-02, see below; the rest remain unclosed). They do not claim that the corresponding compiler behavior is implemented, and "awaiting design" by itself is not a defect in the code.

### OPEN-01: Scope startup order

- Status: a candidate resolution direction and test requirements have been submitted (proposal §5/§24.14; T05 item F point 6; T06 item 8); selection/freeze by T01/T05/T06 is still outstanding, so this item is not closed.
- Location: `docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md:1241–1248` (line numbers as originally reviewed); `docs/tasks/T05_PARSE_CHIPS.md:3–5`.
- Risk: the proposal opens the file scope after the TU node is committed, while T05 parsing depends on scope lookup. It must be explicit whether the TU root is committed early; if it is only committed after the whole TU has been parsed, a startup dependency cycle can form.
- Recommendation: fix the order that commits the TU root early, or explicitly bound a no-scope parse path for M1, and test the complete startup without any pre-created scope.
- Handling record (2026-10-05, doc-only): proposal §24.14 records the selected candidate "early TU-root commit order" — after the lex stage commits the complete token stream (including EOF), the registered `parse.TranslationUnit` task commits the TU root in the first commit-visible batch (`parent: None`, ranging from the first committed token to the committed EOF; an empty TU is an empty range at EOF), and the accepted `parse.TranslationUnit -> symbol_type.scope-enter` edge fires the file-scope Enter exactly once on that committed root; the parse task stays `Waiting` while the `symbol_type` task can be scheduled (under the `(stage ordinal, …)` scheduling order a ready parse task precedes the symbol_type task and therefore must wait), and after the file Enter commits, parsing resumes carrying the committed file `ScopeId`, which preserves T05's parse-time scope/typedef visibility without a no-scope special case. The fallback is an explicitly bounded M1 no-scope parse path (no scope/typedef queries that parse decisions depend on during parse; block-scope requests deferred until after the file Enter; it must not be read as a general scope-free parser). Both paths are validated by the newly required, to-be-registered fixture `M1-START-01` (full startup test with no pre-created scope): zero scopes/events before the TU root commit, exactly one file scope and one Enter (`at` is the committed TU root, with no Exit), the edge fires exactly once and re-observation produces no second firing, replay is consistent, and any scope/typedef query before the file Enter is a typed unsupported/diagnostic rather than a guess. This candidate also bounds the T05 item F point 3 and T06 item 7 triggers to apply only to the File-Enter edge (the trigger is a committed root append, not task termination; exactly-once is structurally guaranteed by the single root append, so `ResultRecord.consumed` is no longer required for that edge, while OB-11/OB-50 stay open). The concrete edge-enqueue realization (same-batch link or a T01 commit-apply hook), duplicate-root validation, the root range/descendant invariants, and the bootstrap task kind/stage remain a T01/T05/T06 `/6` co-freeze (OB-53); `M1-PA-04`, `M1-PA-05`/`M1-TY-05`, and `M1-TY-06` are not weakened.

### OPEN-02: Result delivery after join

- Status: a resolution direction and test requirements have been recorded (proposal §5/§7/§13/§18.3 and §24 OB-4); T01/T02/T05 still have to select the concrete mechanism and freeze it, so this item is not closed.
- Location: `docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md:2621–2631` (line numbers as originally reviewed).
- Risk: the commit-apply step already consumes the child result when resuming the parent task, but how the parent task durably obtains and processes the result on its next tick is still unclosed. Exactly-once consumption is not the same as exactly-once semantic processing.
- Recommendation: keep consumption with the parent task's atomic commit, or transfer it at join into persistent state explicitly owned by the parent task; test replay/retry after join and before parent execution.
- Handling record (2026-10-05, doc-only): proposal §7 (join realization) and §5 removed the unclosed wording "consume the child result at join" and replaced it with the selected direction — join only decides the child's terminal state and consumes nothing; consumption is reserved for the parent task's own next-tick atomic commit (the result stays committed/unconsumed, and the parent commit guards exactly-once consumption with `ResultAlreadyConsumed`), and only if the `/6` freeze requires join-time consumption is it transferred to persistent state explicitly owned by the parent task; both realizations must guarantee that replay/retry between join and parent execution neither double-consumes nor loses results, and a new required test `join_then_replay_retry` is added (proposal §13; T13 H6-M15). The concrete carrier and the consumption/commit wrapper remain a T01/T05/T02 `/6` co-freeze (OB-4).

### OPEN-03: Constant-evaluation input for `2+3`

- Status: awaiting T01/T07/T08 co-freeze.
- Location: `docs/tasks/M1_PART_A_CONTRACT_PROPOSAL.md:1527–1535`; `docs/tasks/T08_CONSTANT_LAYOUT_INIT_CHIPS.md:49`.
- Risk: the example request carries a single literal, while the operator/operand protocol for a binary expression is still open; a fixed result of `5` is not enough to define the real handoff.
- Recommendation: freeze the committed expression/operand/operator input path together with the task/result types, and add a real T07→T08→T09 fixture; a hand-built `5` must not substitute for evaluation.

## Verification record and limitations

- Read-only Python checks were used to verify whether the local inline link targets exist across 36 Markdown files in the root README, `AGENTS`, `docs/`, `compiler/`, and `tools/`: no missing target was found.
- Those checks did not verify link anchors, reference-style links, or document semantics.
- The remaining conclusions of both rounds come from cross-reading the documentation, implementation, and test sources; DOC-08 additionally used a synthetic-capture check performed by a first-round subagent. The second round ran no additional execution tests.
- Cargo fmt/clippy/test were not run, no real Linux/AArch64 probe was executed, and neither the candidate compiler nor GCC torture acceptance was run.
- This document reports no compiler capability, no target verification result, no GCC pass rate, and no formal proof.
- This document preserves 18 findings and 3 pre-freeze design items; it does not claim to have exhausted every documentation defect. Apart from OPEN-02, which has a recorded resolution direction and test requirements (still unfrozen and unclosed), all items remain pending; nothing is deemed fixed or closed merely because this report was saved.