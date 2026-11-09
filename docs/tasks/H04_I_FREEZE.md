# H04 `-I` + Multi-Source Freeze — DRAFT ONLY (OPEN)

| Field | Value |
|---|---|
| Status | **DRAFT ONLY — OPEN (2026-10-10). Needs H01 + T01 sign-off. NOT frozen, NOT accepted, authorizes NO code, NO contract change, NO version bump** |
| Scope | Host tooling proposal only (`candidate` driver + host fetch loop). No chip logic, no bus/schema change, no `t01-c01-c06` amendment proposed here |
| Authority | None. Pending H01 (substrate/sysroot/probe) + T01 integrator (config/contract) sign-off |
| Depends on | [T00](T00_GCC_TORTURE_GATE.md) H04 + §4.1 multi-file rules, [H04 candidate driver](H04_CANDIDATE_DRIVER.md) (Part A + `-E`/`-D`/`-U`), [PP include slice](PP_INCLUDE_SLICE.md) (`/25`), [PP pragma slice](PP_PRAGMA_SLICE.md) (`/29`), [T03](T03_PREPROCESS_CHIPS.md) PP17/PP18/PP25 rows, [T01](T01_COMPILER_CONTRACT.md) §2/§4/§6 |
| Non-goal | Freezing anything. Implementing anything. Changing PP17/PP18 path policy (`/25` stays current). Changing PP25 classification (`/29` stays current). Claiming any pass rate, probe result, or M1 acceptance |

This document is a **proposal draft** for the deferred H04 rows (`-I`, multi-source compilation, quoting edge cases; see `H04_CANDIDATE_DRIVER.md` §5). Today the driver **refuses** `-I`/multi-source with an explicit error (exit 2), and `/25` resolves includes **only** over already-committed sources with quote/angle behaving **identically**. Nothing here overrides that. The `-I`/multi-source path stays fail-closed until H01+T01 accept a freeze and the T01 integrator lands it.

**B3-03b note (2026-11-09).** A separate bounded subset — quoted `#include "..."` resolved only beside the including file, single-level, no `-I`/system roots/nesting/guards/`once` — is accepted and implemented in [B3_03B_LEAF_INCLUDE_POLICY.md](B3_03B_LEAF_INCLUDE_POLICY.md). It does not accept or narrow this draft's `-I`/multi-source policy; those remain open.

## 1. What this draft proposes (and does not decide)

Four items, all OPEN:

1. **Search-path policy** (§2) — ordered candidate proposal for how `-I` dirs, the including file's directory, and the H01 system/sysroot dirs combine.
2. **Quote/angle divergence** (§3) — candidate delta from the frozen `/25` identical behavior.
3. **Header-guard interaction (PP25 `once`)** (§4) — how a future once-set would consult PP25 classification without changing `/29`.
4. **Candidate orchestration order** (§5) — host-side order of flag seeding, source loading, fetch-and-retry, per-TU pipelining, and multi-source link rules.

Every name, numeric code, config field, record carrier, and hash rule below is **proposed, not frozen**. Where this draft names a carrier it does so only to identify the co-freeze question for T01; it invents no encoding.

## 2. Search-path policy (candidate, OPEN)

### 2.1 Proposed inputs (all pending H01/T01 freeze)

- `-I <dir>` repeatable; order = flag order on the command line. Relative dirs resolve against the candidate's invocation CWD, pinned at startup (no mid-run CWD change).
- Quoting edge cases (spaces, `--`, `=` in paths, empty `-I ""`) follow shell-tokenization-then-driver rules; the driver must never silently drop or merge entries. Exact quoting/escaping table is an H04-contract freeze item, still open.
- System/sysroot roots come **only** from the H01-pinned substrate (board/sysroot/reference toolchain pins, `T00` §4.1). The candidate never hardcodes `/usr/include`, never samples the host environment for defaults, and never falls back to another compiler's search list.

### 2.2 Proposed resolution order (candidate)

For one `#include` header name (after PP17 macro-expansion of the name, T03:25 — macro-generated names stay `Unsupported` per `/25` until T03 says otherwise):

1. Apply the §4 once-skip check first (proposed; no behavior today).
2. Scan candidates in order; **first hit wins**; hits are host-loaded files, never committed-source scans (the `/25` committed-source scan stays the chip-side rule; this host order feeds the host fetch loop in §5):
   - Quote form `"..."`: (a) directory of the including file, (b) `-I` dirs in flag order, (c) H01 system/sysroot dirs in pinned order.
   - Angle form `<...>`: (a) `-I` dirs in flag order, (b) H01 system/sysroot dirs in pinned order. No including-file-dir step.
3. Within one directory: exact filename match only (byte-exact spelling between delimiters; no case folding, no extension guessing). Basename-substring fallback does **not** cross into the filesystem search — basename matching is the frozen `/25` **committed-source** rule only.
4. Determinism: directory entries are never enumerated for guessing; each candidate path is a single `open` attempt in the frozen order, so result order depends only on flag order + pinned system order, never on filesystem iteration order.
5. Failures are loud and typed (names/codes open): not-found (searched N dirs, list count only, never contents), ambiguous only if two pinned roots canonically coincide (proposed: canonical-path equality → first wins, recorded; never guess), empty header name → malformed, `include_next` → explicit `Unsupported` (T12 scope, unchanged).

### 2.3 Boundaries (unchanged by this draft)

- Chips stay host-passive: PP17 `not-loaded` remains the chip-side signal; the host loop in §5 does the fetching. No `AwaitHost`/CT02 change is proposed here (T02-owned).
- Sandboxing: configured include roots only (per repo security rules); absolute paths, `..` escapes, and symlink escapes outside the pinned roots fail closed. Exact escape-detection rule is an H01/H04 co-freeze item.
- Depth/nesting: single-pass stitch (`/25`) stays; fixpoint multi-level nesting waits on the T02 control-loop slice. The host loop (§5) may pre-load leaf headers but must not fake recursion the chips have not performed.

## 3. Quote/angle divergence (candidate delta from `/25`, OPEN)

- Frozen today (`/25` §2): quote vs angle behave **identically** (delimiters stripped, exact-then-basename over committed sources).
- Proposed future (this draft only): the §2.2 order above — quote gets the including-file-dir step, angle does not. Everything else (exact match, loud failures, `include_next`/`empty`/`macro-generated` taxonomy) stays as `/25` defines it.
- Config-search-path coupling: divergence is meaningless without pinned `-I`/sysroot dirs, which is why this draft needs **H01 sign-off** (substrate pins) and **T01 sign-off** (`config` ownership: include-path fields, if any, live in T01-owned config; H04 must not invent a parallel config).
- Migration rule (proposed): while `-I`/system dirs are absent (today), behavior must remain exactly `/25`-identical. Divergence activates only under the accepted freeze with at least one pinned search dir; no silent behavior change.

## 4. Header-guard interaction — PP25 `once` (OPEN, needs T01 record freeze)

Frozen today:

- `/29`: `PpPragmaChip` (PP25) classifies `#pragma once` / `_Pragma("once")` as `Once` (and `pack` push/pop, else `Opaque`), completes `Ack`, **no bus writes — no store field lands**. Explicitly deferred: "`PragmaRecord` carrier on the bus (once-consultation by PP17/PP18 waits on the record freeze)".
- `/25` non-goal: "`#pragma once`" — PP17/PP18 never consult `once`.
- Classic include guards (`#ifndef FOO_H` / `#define` / `#endif`) are ordinary PP19 conditional state, not `once`; they need no new record and are unaffected by this section.

Candidate interaction (proposal only):

1. A future T01-owned once-set (carrier/store/encoding all open — candidate names only: header identity → once-mark) would be populated from PP25 `Once` classifications at the wiring layer and consulted by the host fetch loop (§5) **before** re-entering PP18 for an already-once-marked header: second and later inclusions skip the stitch and succeed with no tokens appended (proposed; exact empty-vs-elided representation open).
2. Identity question (OPEN, T01-owned): canonical host path vs content hash vs `SourceId`. Same file reached via two spellings (e.g. `-I` dir + quoted relative path) must resolve to one identity or the guard is unsound; the rule is not decided here.
3. `_Pragma("once")` equivalence (proposed: identical to `#pragma once`, per `/29` classifying both as `Once`); placement (proposed: a `once` anywhere in the header marks the whole header, matching de-facto compiler behavior — still needs T03 confirmation); interaction with `-I` remapping (§2) and with the `/25` exact/basename committed-source match (OPEN: once-check applies to the resolved identity, not the spelling).
4. No silent skip: every once-skip is traceable in the host log / evidence (which header, which identity, which including location). A missing/unfrozen once-set must fail closed to today's behavior (re-include, single-pass stitch), never to an invented skip.
5. `pack`/`Opaque` classifications have no include interaction.

## 5. Candidate orchestration order (host-side proposal, OPEN)

Proposed order when (and only when) the freeze lands. Today steps 3–6 stay refused (exit 2).

1. **Flag parse.** Accept `-std=` (still only the frozen values until T01 widens dialect), `-O` (recorded, semantics per frozen target/dialect), `-D`/`-U` (existing: flag-order commit, last-wins, empty-`-D`-stays-defined — `H04_CANDIDATE_DRIVER.md` §2, unchanged), `-E`, `-o`, `-I` (new, §2). Unknown flags → usage error exit 2. `-S`/`-c` stay refused while the target is unverified (Part B, probe-gated; unchanged).
2. **Macro-seed commit.** `-D`/`-U` seeds commit before preprocessing in flag order (existing behavior, unchanged).
3. **Substrate pin.** Resolve `-I` dirs + H01 sysroot roots to canonical absolute paths; enforce sandbox (§2.3); record the pinned list in the evidence/report (exact report field open). Any unresolvable/escaped root → driver error exit 2 before any compilation.
4. **Primary + auxiliary source load.** Load each command-line `.c` file plus H03-manifest auxiliary sources (torture multi-file tests: all `.c`, `.S`, extra sources enumerated by the official driver enter the list — `T00` §4.1). Each file becomes one committed source; **one TU per primary source** with independent frontend stores (T01 §2: multiple TUs create frontend stores independently; no shared mutable symbols; shared strings read-only with explicit owner).
5. **Preprocess fetch-and-retry loop (per TU).** Drive the frozen PP pipeline (PP01→…→PP18 with conditionals/macros per the frozen slices); on PP17/PP18 `not-loaded` for header `X`: resolve via §2.2, once-check via §4, load bytes, commit source, scan its pp-tokens (PP04-before-PP18 ordering per `/25` §4), retry the failed task. Bounded retries with a frozen depth/budget (limits owned by T01 config; values open). Any `ambiguous`/`malformed`/`Unsupported` propagates as a structured diagnostic (exit 1), never a silent skip or a fallback to another compiler's preprocessor (final-gate rule, `T00` §3: external-preprocessor runs never count as end-to-end).
6. **`-E` emission.** After full include expansion of that TU (directives consumed per PP28 rules), emit via the frozen PP28 path when `-E` is requested. Multi-TU `-E` writes one output per TU (naming rule open; no clobbering; H05 separate-directory discipline applies to harness runs).
7. **Multi-source compile/link rule.** Each TU compiles independently to its own object (Part B; today refused); linking uses the external assembler/linker + pinned sysroot libc (explicit toolchain deps, `T00` §3 — allowed) with the H03-frozen link rule (objects, order, extra sources, target predicates). `.S` sources assemble via preprocessing + external assembler, recorded distinctly from the C path (`T00` §4.1). No TU may resolve symbols against another TU's mutable bus state.
8. **Harness integration.** H02/H03 own instance enumeration and judging (official DejaGnu driver + manifest; one instance = suite/testcase/target/dialect/options/aux-sources); H05 owns isolation (separate directory, timeouts, resource limits, crash classification); H06 owns aggregation (dual denominators, stratification). This draft adds no runner, no manifest field, no gate change.

## 6. Acceptance sketch (illustrative; NOT acceptance until frozen)

- Policy matrix: quote finds including-dir file before `-I`; angle skips including-dir; `-I` order respected; system roots last; missing/empty/`include_next`/macro-generated all loud.
- Quoting matrix: spaces/`--`/`=`/empty entries never silently merged or dropped.
- Once matrix: second include of a once-marked header elides (traceable); distinct files with same basename do not elide each other; classic guards behave as conditionals with no once interaction.
- Orchestration: multi-file torture instance compiles each TU independently, links per the frozen rule, runs under H05 isolation; determinism (byte-identical evidence reruns) preserved.
- Negative: any use of `-I`/multi-source before the freeze stays refused (exit 2); no GCC/Clang delegation; no external-preprocessor end-to-end claim.

## 7. Explicitly deferred (still absent after this draft)

- All numeric codes, diagnostic names, config field paths, record carriers, snapshot encodings, and hash rule IDs.
- Fixpoint nested-include control loop (T02), async fetch protocol (CT02), `include_next` (T12), trigraph/`%:`/non-ASCII header-name gates (T03 dialect).
- Torture flag-propagation matrix (H02/H03), H05–H10 gates, target emission (Part B, probe-gated).
- This draft claims no corpus, no target values, no probe, no rate. `/25` and `/29` remain the frozen behavior.
