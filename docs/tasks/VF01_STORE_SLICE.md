# Wave 2 Slice 10: VF01 Store-Invariant Freeze (`/19`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-06** as `t01-c01-c06/19` (hash `76155ee8…476a4`) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/18` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/19` delta), [T13](T13_VERIFICATION_CHIPS.md) VF01 row |
| Non-goal | Host-request references, tombstone liveness, task-graph cycles, wire lifetime |

## 1. What this slice is

One read-only verifier over the whole committed snapshot: `Vf01Chip`
(VF01) checks the M1 store contract with an empty payload (global check,
no focus record) — every task-payload and result reference resolves to
an allocated record (dense IDs, never reused), every task parent and
continuation resolves, every span names a committed source with
`start <= end <= source length` (and a committed expansion when
present), and the still-reserved stores (`layouts`, `inits`, `vregs`)
hold no records. Completes `Ack`; any gap fails loudly. The acceptance
fixture is the full M1 chain ending in **`Ack` over the healthy bus**.

## 2. Freeze items

| # | Item | Acceptance |
|---|---|---|
| U1-1 | Reference ownership: all payload refs (every task, every state), all result `Record`/`Records`/`Diagnostic` refs, all parents/continuations resolve against dispatch-time allocated bounds | positive + dangling-ref negative |
| U1-2 | Span bounds: source committed, `start <= end <=` source byte length, expansion committed when present | escaping-span negative |
| U1-3 | Reserved-unused M1 rule: `layouts`/`inits`/`vregs` allocated counts are all zero; any reference into them is dangling by construction | reserved-alloc negative |
| U1-4 | `verification.store_invariant` (`VERIFICATION` local `19`, `Frozen`) + `vf01_slice()` registry (32 entries, cumulative over `vf05_slice`); stage row (`→ 6`, runs after the lowered snapshot); routed layer 6; `VF01_CHIP = ChipId(22)`; no writes so no allowlist rows | kind/stage/allowlist/layer gates green |
| U1-5 | No schema change: the slice reuses the PP-slice schema (29 read paths, all declared in `/6`–`/16`); no new record bodies, no snapshot encoding change | schema-reuse assertion |
| U1-6 | Empty-payload convention, non-`Running` state, and wrong kind fail loudly | negative cases |
| U1-7 | Worker template (narrow projection, pure `compute` + same-file helpers, ZST, stage/layer, lint) | `c19_vf01` 7 tests green |
| U1-8 | Hash rules `vf01.refs-resolve`, `vf01.span-bounds`, `vf01.reserved-unused-m1` | freeze + replay determinism |

## 3. Execution record

Implemented on the current branch, verified by `compiler/tests/c19_vf01.rs`
(7 tests: kind/stage freeze, end-to-end `Ack` over the full M1 chain,
malformed-payload negative, reserved-record + escaping-span negatives,
compute-level dangling-reference negative, stage/layer + manifest gates
including the stale-registry rejection, snapshot replay determinism).

## 4. Explicitly deferred

- `HostRequest` references (no frozen schema; fail as out-of-M1-scope,
  never silently passed; M1 produces none).
- Tombstone liveness past the allocated bound (lifetime verifier scope).
- Task-graph shape (ready/wait/complete exclusivity, cycles, lost
  responses — VF02 scope, H6-dependent).
- Wire lifetime and batch write-conflict audit (VF03/VF04 scope).
- Replay-across-dispatch-order and evidence classification (VF13/VF14).
