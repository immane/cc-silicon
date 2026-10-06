# Wave 3 Slice 13: VF14 Evidence-Classify Freeze (`/38`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/38` (hash recomputed at integration) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/37` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/38` delta), [T13](T13_VERIFICATION_CHIPS.md) VF14 row |
| Non-goal | `HostTestEvidence` record family, frozen test-instance schema, frozen boolean pass/fail carrier, batch verdicts, H6 replay fixtures |

## 1. What this slice is

One read-only verifier classifying the T00 gate outcome over the
complete compile/link/run/check evidence vector: `Vf14Chip` (VF14)
reads one Host-fed evidence pin set plus a frozen test-instance ref
and returns exactly one verdict. Every required stage present and
passing is PASS (completes `Ack`); any present failure — in
particular compile-ok with run-fail — is FAIL at the earliest
failing stage in pipeline order (typed `Fail`, never PASS); a
required-but-missing stage is Reject (typed `Fail`, never PASS); an
undecodable carrier or an invalid stage gate is Reject (typed `Fail`,
never PASS). The acceptance fixture is seeded gate/result pins ending
in **one `Ack` plus three typed `Fail`s**.

## 2. Validation table (frozen)

| Input | Outcome |
|---|---|
| Check gate, all four stages passing | `Ack` (PASS) |
| Compile gate, compile passing, later stages unpinned | `Ack` (missing past the gate is not required) |
| Run gate, compile/link passing, run failing | `Fail` (FAIL at `run`, never PASS) |
| Check gate, link failing + run failing | `Fail` (earliest: `link`) |
| Check gate, link missing | `Fail` (`Missing` at `link`, never PASS) |
| Check gate, run carried by a `Record` | `Fail` (`Undecodable` at `run`, never PASS) |
| Gate raw `9` (any evidence) | `Fail` (`InvalidStage`, never PASS) |
| Missing required + later failure | `Fail` (`Missing` — missing beats failure) |
| Undecodable slot + failing slot | `Fail` (`Undecodable` — undecodable beats failure) |
| Failure past the gate (compile gate, run failing) | `Fail` (present failures count anywhere in the vector) |
| Non-running task, wrong arity, non-const gate, non-result pin | `Fail` (protocol fault) |
| Dangling instance ref, missing gate const, missing evidence result | `Fail` (dangling/missing read) |
| Wrong stage layer, pre-`/38` registry | Driver/gate `Fail` (never silent) |

## 3. ID arbitration (frozen)

The delivered draft (`vf_evidence.rs`) defined file-local
`VF14_CANDIDATE_LOCAL` (20) / `VF14_CANDIDATE_CHIP` (`ChipId(56)`) plus
a `candidate_kind()` helper. The integrator verified the `/37` head
(`VERIFICATION` locals 16–19 taken by VF06/VF12/VF05/VF01,
`CL07_ASSERT_CHIP = ChipId(55)`) and froze the draft IDs linearly with
no logic change beyond kind/chip-id/test-path repointing:

- Kind: `evidence_classify` (local 20) — `VERIFICATION` owners start
  new codes at local 21.
- Chip: `VF14_CHIP = 56`.
- The file's draft consts (`VF14_CANDIDATE_LOCAL`,
  `VF14_CANDIDATE_CHIP`, `candidate_kind()`,
  `VF14_FALLBACK_KIND`) are re-pointed at the frozen canonical
  (`VF14_TASK_KIND` alias, manifest chip ID, test path →
  `c38_vf14.rs`); header rewritten from candidate-UNREGISTERED to
  frozen `/38`. No chip logic changed beyond kind/chip-id/test-path
  repointing.

## 4. Frozen registration

- `evidence_classify` (local 20) — first free code after VF01
  `store_invariant` (local 19), `Frozen`; `vf_evidence_slice()`
  registry (68 entries, cumulative over `const_branch_slice()`);
  stage row (`→ 6`, terminal verification with VF01/VF12); routed
  layer 6; `VF14_CHIP = 56` (PASS completes `Ack`, every other path
  exactly one typed `Fail`, zero writes, no allowlist rows;
  `STORE_OWNER_ALLOWLIST` stays 32); PP-slice schema reuse (every read
  field declared — same read set as `Vf01Chip` minus
  `active.parent`/`active.continuation`, which this projector never
  reads).
- Hash rules: `vf.evidence-complete`, `vf.evidence-stage`,
  `vf.evidence-never-pass-missing`.

## 5. Freeze items

| # | Item | Acceptance |
|---|---|---|
| E1-1 | Complete evidence passes with `Ack` (every gate; missing past the gate is not required) | pass case |
| E1-2 | Compile-ok with run-fail is FAIL at `run` (never PASS); earliest failing stage wins | run-fail case |
| E1-3 | Required-but-missing stage is rejected (never PASS) | missing case |
| E1-4 | Invalid stage gate is rejected (never PASS) | invalid-stage case |
| E1-5 | Undecodable carrier is rejected (never PASS); frozen M1-scope carrier map (`Ack`→pass, `Diagnostic`→fail, `Empty`→absent, `Record`/`Records`→undecodable) | undecodable case |
| E1-6 | Total precedence (invalid gate, then earliest missing required stage, then earliest undecodable slot anywhere, then earliest failing slot anywhere) + determinism | precedence case |
| E1-7 | Registration freeze (kind/registry/stage/layer/chip/allowlist) | freeze + gates green |
| E1-8 | Bus dispatch (one `Ack` + three typed `Fail`s) + snapshot replay determinism | bus case |
| E1-9 | Non-running / malformed / dangling negatives fail loudly; well-formed projection accepted | negatives case |

## 6. Execution record

Delivered as one untracked chip file (`vf_evidence.rs`) against the
`/37` tree; integrated by the T01 integrator: verified the `/37` head
(no `VERIFICATION` local past 19, chip 55), froze local 20 /
`ChipId(56)` (§3) into `task.rs` (`vf_evidence_slice()`, 68
entries), `VF14_CHIP` + `is_vf_evidence_slice_kind` + one stage row +
zero allowlist rows into `manifest.rs`, three rule ids into
`contract.rs`, bumped to `t01-c01-c06/38` with recomputed hash
(`f18068f9…0d83`), wired `verify/mod.rs` + `chips/mod.rs` (pure
cores re-exported), and re-pointed the draft's kind/chip-id/test-path
at the frozen canonicals (header rewritten from candidate to frozen
`/38`). No chip logic changed beyond kind/chip-id/test-path
repointing. Verified by `compiler/tests/c38_vf14.rs` (10 tests:
freeze, pass, run-fail, missing, invalid stage, undecodable,
precedence + determinism, bus dispatch + replay, loud negatives,
stage-layer gates). Full §5 suite green at commit. Superseded pins
updated by the integrator (never the chip owner, never silent):
`freeze.rs`, `c08_gate1` (version, 68 stages, 32 allowlist rows, new
rule/marker pins), `c04_manifest` (no-row comment), `c20_ppscan` +
`h04_candidate` version markers, both READMEs,
`T01_COMPILER_CONTRACT.md` §7.1, `CHIP_PLAN.md`, `context.md`.

## 7. Explicitly deferred (all loud, never silent)

- `HostTestEvidence` record family and frozen test-instance schema
  (DEFECT-VF14-01): the projector accepts any committed record as the
  instance ref (resolvability only, never interpreted).
- Frozen boolean pass/fail carrier (DEFECT-VF14-02): the M1-scope
  `Ack`/`Diagnostic`/`Empty`/`Record` map above is frozen for this
  slice; a richer carrier awaits its own freeze.
- Batch verdicts and H6 replay fixtures (one transition proposal per
  handle; no `Progress`/`Await*`).
- The VF02–VF04/VF07–VF11/VF13 remainder (unchanged).
