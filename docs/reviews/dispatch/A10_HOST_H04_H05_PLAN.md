# A-HST-01 — H04/H05 Host Acceptance Plan: Flags, Sources, and Isolated Execution

| Field | Value |
|---|---|
| Status | **PLANNING / READ-ONLY AUDIT — no implementation, no probe, no torture run, no H04/H05 completion claim** |
| Date | 2026-11-07 |
| Baseline | `t01-c01-c06/39`; branch `wave4-development`; HEAD `430ae29` |
| Scope | T00 Host tasks H04 (candidate driver flags/sources) and H05 (isolated execution, limits, cancellation, crash classification) |
| Authority | [T00](../../tasks/T00_GCC_TORTURE_GATE.md) §4 H04/H05; [M1 target acceptance](../../tasks/M1_TARGET_ACCEPTANCE.md) P7/P8; [H04 candidate driver](../../tasks/H04_CANDIDATE_DRIVER.md); [H04 `-I`/multi-source draft](../../tasks/H04_I_FREEZE.md) — **draft only** |
| Non-goal | Freezing flags or schemas, implementing code, running the corpus, running external compilers, creating probe evidence, or claiming Part B readiness |
| Only file written by this task | `docs/reviews/dispatch/A10_HOST_H04_H05_PLAN.md` |

This plan separates four states everywhere below:

- **EXISTING**: implemented and evidenced in the current tree.
- **FROZEN**: an accepted contract artifact (for example the `/25` include slice or the `/39` foundation envelope).
- **DRAFT/PROPOSED**: written but not accepted, with no implementation authorization (for example [H04_I_FREEZE](../../tasks/H04_I_FREEZE.md)).
- **ABSENT/DEFERRED**: explicitly refused or not implemented.

Terminology note: this report uses **Part A** = target-independent and **Part B** = target/probe-gated, following T00 and M1. `H04_CANDIDATE_DRIVER.md` also uses "Part A/Part B" for its host-tool batches; those labels are not identical to the M1 target split and are not used as evidence here.

## 1. Current Status and Evidence Boundary

### 1.1 Status table

| Area | Current state | Evidence | What may not be claimed |
|---|---|---|---|
| H04 Part A driver | **EXISTING** for the M1 evidence path: source bytes → PP/lex/parse/type/sem/IR/interpret; snapshot/trace/interpret output; one `.c` input | `compiler/src/bin/candidate.rs`; `compiler/tests/h04_candidate.rs`; targeted run in this audit: 18 passed | General C compilation, torture execution, target code generation |
| H04 `-E` | **EXISTING** host-only emission through PP28; golden and determinism tests | `candidate.rs`; `compiler/src/bin/candidate/emit_helper.rs`; `h04_candidate.rs` | GCC-compatible `-E` (no line markers; token-spaced emission) |
| H04 `-D`/`-U` | **EXISTING** flag-order commit, bare `-D` = `1`, empty value stays defined, last-wins | `candidate.rs`; `compiler/src/bin/candidate/macro_flags.rs`; `h04_candidate.rs` | Function-like command-line macros or non-ASCII macro names |
| H04 `-I` | **ABSENT/REFUSED**: `-I` and `-I<dir>` exit 2 with "deferred" | `candidate.rs` parse arm; no PP17/PP18 installation in the driver | Include search, quote/angle divergence, system headers |
| H04 multi-source | **ABSENT/REFUSED**: exactly one positional source; N≠1 exits 2 | `candidate.rs` positional check | Multi-TU compilation, linking, `.S` sources |
| H04 `-S`/`-c` | **REFUSED fail-closed**: exit 2 before reading input; no output file | `candidate.rs` parse arm; `h04_candidate::candidate_refuses_target_emission` | Part B assembly/object output |
| H04 contract | **NOT FROZEN**: flag spellings are explicitly illustrative until an H04 contract freezes | [H04_CANDIDATE_DRIVER](../../tasks/H04_CANDIDATE_DRIVER.md) §2; [H04_I_FREEZE](../../tasks/H04_I_FREEZE.md) status table | A complete frozen flag matrix |
| H05 isolated execution | **ABSENT**: no runner, no per-instance directory manager, no timeout/limit/cancellation/classification implementation | T00 §4, §4.1; M1 P8; `tools/torture/src/**` is only the H00 lock verifier; `tools/torture/Cargo.toml` has no process-runner code | Any H05 acceptance, any suite non-blocking claim |
| H01/C02 target | **UNVERIFIED**: probe harness exists but has never run; no report/hash | T00 §4.1; `tools/torture/probe/README.md`; `compiler/contracts/CONTRACT_VERSION` (`target_verification=unverified`) | Concrete ABI values, Part B execution |
| H02/H03 | **PLANNED**: no instance manifest, no official-driver integration, no frozen flag matrix | T00 §4.1; `docs/tasks/README.md` status paragraph | Torture run or denominator |
| H06 | **PLANNED**: no aggregation/evidence schema freeze | T00 §4; M1 §9 | Pass-rate or failure-classification claims |

### 1.2 Checks actually run in this audit

```sh
cargo test --locked --manifest-path compiler/Cargo.toml --test h04_candidate
# result: 18 passed, 0 failed (targeted H04 test only)
```

The build emitted warnings from unrelated uncommitted parse/lex draft files; those warnings are not H04 evidence and none of those files was modified by this audit.

Also checked manually, without modifying source:

- `candidate -S` / `-c` → exit 2, no output file;
- `candidate -I /tmp` and `-I/tmp` → exit 2;
- two positional sources → exit 2;
- `--` → exit 2;
- `-O2` → exit 2;
- `-o` without a path → exit 2;
- `-D` / `-U` with a source-looking next argument → exit 2 with a macro-name diagnostic;
- `-DFOO=@` → exit 2 with a scan diagnostic;
- a non-UTF-8 argv element → **exit 101 with a Rust panic** (defect F-01 below).

**Not run:** full compiler suite, root suite, `clippy`, `fmt`, external assembler/linker, any candidate C compilation beyond the M1 fixture, any harness/probe run, any GCC torture case, any untrusted test program, any H05 test (H05 does not exist).

### 1.3 Findings summary

| ID | Severity | Finding | Evidence |
|---|---|---|---|
| F-01 | High | Non-UTF-8 argv panics (exit 101, Rust backtrace) instead of a structured driver error. | `std::env::args()` in `candidate.rs`; reproduced manually |
| F-02 | High | `-std` and `-O` are validated but **not propagated**: `Invocation` stores neither; `pp_prefix` hardcodes `Dialect::C11` and `OptLevel::O0`. The H04 "flag propagation" acceptance is therefore not met even for the accepted values. | `candidate.rs` `Invocation` fields; `pp_prefix` config construction |
| F-03 | High | One exit code `2` conflates usage errors, unsupported/deferred features, and Part B refusal. H06 cannot distinguish `INFRASTRUCTURE_ERROR` from `UNSUPPORTED` from `ICE` without parsing free text. | `candidate.rs` exit paths; H04 doc §2 |
| F-04 | High | Input read failure and source-capacity failure exit 1 ("candidate diagnostic") although they are Host/resource errors, not source-semantic diagnostics. | `pp_prefix` `std::fs::read` mapping; `main` exit 1 |
| F-05 | High | H05 is entirely absent; the two T00 acceptance criteria (a hang must not block the suite; duplicate basenames must not collide) are untested and unimplemented. | T00 §4 H05; no runner source |
| F-06 | Medium | Source identity uses only the file stem/basename; a non-UTF-8 basename silently falls back to `"main.c"`. Duplicate basenames and include resolution lose provenance. | `candidate.rs` stem code |
| F-07 | Medium | The textual evidence report records only contract version, input, snapshot, trace, interpret; it omits dialect, option level, target verification state, and the actual `-D`/`-U`/other flags. Snapshot hashes cover config, but H06/H05 need explicit metadata. | `candidate.rs` evidence rendering; `snapshot.rs` config hash |
| F-08 | Medium | `-c` semantics and the assembler boundary are undefined. Candidate `-c` normally must produce an object; T00 compile acceptance requires an object, not `-S` only. | T00 §1; [H04_I_FREEZE](../../tasks/H04_I_FREEZE.md) §5 step 7 |
| F-09 | Medium | `-I` and multi-source remain blocked on H01, T01 config, T03 PP17/PP18 update, T02 control loop, H03 manifest, and H04 contract freeze. | [H04_I_FREEZE](../../tasks/H04_I_FREEZE.md); [PP_INCLUDE_SLICE](../../tasks/PP_INCLUDE_SLICE.md) |
| F-10 | Medium | Documentation drift: H04 evidence example says `t01-c01-c06/19`; test expects `/39`. T00 §4.1, `docs/tasks/README.md`, `PARALLEL_EXECUTION.md`, and M1 P7 variously say H04 is absent, `-E` is deferred, or 6 tests exist. Actual: H04 Part A + `-E`/`-D`/`-U` present; 18 tests. | See §1.4 |
| F-11 | Medium | The official GCC 15.2.0 torture invocations pass `-w` and option combinations (`-O1`, `-O2`, `-O3 ...`, `-O3 -g`, `-Os`, `-Og -g`, `-fno-inline`, `-DSIGNAL_SUPPRESS`); the candidate rejects all except `-O0`/`-std=c11`. It cannot run the official driver today. | `gcc-dg.exp`/`c-torture.exp`/`execute.exp`; T00 §2 |
| F-12 | Medium | Output writes use `std::fs::write` directly; a crash/write failure can leave a partial report/`.i` file. H05 evidence must never hash or accept a partial artifact as complete. | `candidate.rs` output paths |
| F-13 | Low | `--`, `-`, `@file`, and `-o -` have no defined semantics. `@file` is currently treated as a positional source path, not a response file. | `candidate.rs` parse fall-through |
| F-14 | Low | `-D`/`-U` names are ASCII-only; function-like command-line macros and non-ASCII names are unsupported (explicit rejection, but not documented in the flag contract). | `macro_flags.rs` `is_valid_name` |
| F-15 | Low | `-E` output is a token-spaced PP28 stream, not GCC line-marker output. It must not be used as a general GCC-compatible preprocessor or as end-to-end evidence. | PP28 golden tests |
| F-16 | Low | H05 process-group kill and rlimit enforcement conflict with `#![forbid(unsafe_code)]` in both the compiler and torture crates; an approved boundary or external supervisor is required. | `compiler/src/lib.rs`; `tools/torture/src/lib.rs` |
| F-17 | Low | The official c-torture driver derives output names from `tmpdir` + basename; H02/H03 must make `tmpdir` per-instance or duplicate basenames collide even if H05 has per-instance directories. | `c-torture.exp` `c-torture-compile`/`c-torture-execute` |
| F-18 | Low | Diagnostics print raw argv/path text without control-character escaping; a hostile filename can inject terminal/log lines. H00's verifier already escapes report text; H04/H05 must do the same. | `candidate.rs` `eprintln!` paths; H00 README F5 |

### 1.4 Documentation drift (recorded, not edited)

| Document | Stale statement | Actual current state |
|---|---|---|
| `docs/tasks/H04_CANDIDATE_DRIVER.md` §3 | Evidence sample uses `t01-c01-c06/19` | Contract is `/39`; `h04_candidate` asserts `/39` |
| `docs/tasks/T00_GCC_TORTURE_GATE.md` §4.1 | H04 is Part A only; `-E`/`-I`/`-D`/`-U` deferred; 6 tests | `-E`/`-D`/`-U` exist; 18 tests; `-I`/multi-source deferred |
| `docs/tasks/README.md` lines 64–66 | `-E`/`-I`/`-D`/`-U`/multi-source deferred | `-E`/`-D`/`-U` present; `-I`/multi deferred |
| `docs/tasks/PARALLEL_EXECUTION.md` §1 | H04 driver is absent | H04 Part A + `-E`/`-D`/`-U` present |
| `docs/tasks/M1_TARGET_ACCEPTANCE.md` P7 | 6 tests; `-E`/`-I`/`-D`/`-U` absent | 18 tests; `-E`/`-D`/`-U` present |
| `docs/tasks/H04_CANDIDATE_DRIVER.md` §5 | `-I`, multi-source, quoting edge cases need freezes | Still true; [H04_I_FREEZE](../../tasks/H04_I_FREEZE.md) is still DRAFT |

These are documentation defects for the integrator's docs batch; this report does not edit them.

### 1.5 Source anchors

| Claim | Source anchor |
|---|---|
| CLI parse, flag order, one-source rule, `-S`/`-c`/`-I` refusals, evidence clash | `compiler/src/bin/candidate.rs` lines 88–212 |
| Hard-coded unverified target, C11, O0, fixture limits | `compiler/src/bin/candidate.rs` lines 577–583 |
| Source stem/name and non-UTF-8 fallback | `compiler/src/bin/candidate.rs` lines 587–597 |
| Evidence rendering and output writes | `compiler/src/bin/candidate.rs` lines 900–974 |
| `-D`/`-U` parsing, ASCII name check, seed ordering, record shapes | `compiler/src/bin/candidate/macro_flags.rs` lines 61–156 |
| `-E` terminal through PP28 | `compiler/src/bin/candidate/emit_helper.rs` lines 38–62 |
| Contract version and hash | `compiler/contracts/CONTRACT_VERSION` lines 3–4 |
| Test assertion of `/39` | `compiler/tests/h04_candidate.rs` line 156 |
| Private verification, `attest`, `ensure_codegen_ready` | `compiler/src/target.rs` lines 364–412, 666, 1345 |
| Config hash in snapshot | `compiler/src/snapshot.rs` lines 51–115, 528–548 |
| Compiler-job limits | `compiler/src/limits.rs` lines 118–145 |
| Frozen include path policy (`/25`) | `docs/tasks/PP_INCLUDE_SLICE.md` lines 22–42, 44–72 |
| Frozen pragma classification (`/29`), no bus store | `docs/tasks/PP_PRAGMA_SLICE.md` lines 9, 59–62, 104–109 |
| H04 draft search/quoting/once/orchestration | `docs/tasks/H04_I_FREEZE.md` lines 24–55, 57–70, 73–84 |
| H05 absence / lock-verifier scope | `tools/torture/README.md`; `tools/torture/src/{diagnostic,digest,lock,path,verify}.rs` |
| H01 probe never run | `tools/torture/probe/README.md`; `compiler/contracts/CONTRACT_VERSION` (`target_verification=unverified`) |

## 2. Reference Driver Behavior That the Flag Matrix Must Match

T00 §2 requires the matrix to be derived from the frozen release driver, not guessed. The GCC 15.2.0 driver files referenced by T00 were read for this plan (not vendored, not executed):

- `gcc/testsuite/lib/gcc-dg.exp` defines `DG_TORTURE_OPTIONS` with six entries: `-O0`, `-O1`, `-O2`, `-O3 -fomit-frame-pointer -funroll-loops -fpeel-loops -ftracer -finline-functions`, `-O3 -g`, `-Os`.
- `gcc/testsuite/lib/c-torture.exp` defines `C_TORTURE_OPTIONS` with those six plus `-Og -g`.
- `compile/compile.exp` sets `dg-do-what-default "assemble"` and calls `gcc-dg-runtest ... "-w"`.
- `execute/execute.exp` sets `dg-do-what-default "run"` and calls `gcc-dg-runtest ... "-w"`.
- `ieee/ieee.exp` uses `C_TORTURE_OPTIONS`, adds `-fno-inline`, optionally `-DSIGNAL_SUPPRESS`, and target-specific `-ffloat-store`/`-mieee` where applicable.
- `c-torture-execute` passes the whole `sources` list to `gcc_target_compile`; output names are `tmpdir/basename` based.
- `dg-options`, per-test option add/remove, `.x` driver overrides, target predicates, and LTO rows are additional inputs owned by H02/H03, not something H04 may hand-select.
- Non-LTO is the primary profile; LTO is a separate profile (T00 §2).

Consequences for H04:

1. `-w` is mandatory for the official compile/execute invocation even though it is not in T00's H04 flag list.
2. `-O1`/`-O2`/`-O3`/`-Os`/`-Og` and the long `-O3` option set must be accepted or explicitly reported as unsupported; accepting and ignoring observable option effects is not allowed.
3. `-g` must be accepted for the `-O3 -g`/`-Og -g` rows, with the T00 disclosure that missing debug info makes debug-requiring cases fail.
4. `-fno-inline`, `-fomit-frame-pointer`, `-funroll-loops`, `-fpeel-loops`, `-ftracer`, `-finline-functions`, `-DSIGNAL_SUPPRESS` are real invocation tokens, not hypothetical.
5. The candidate must not be the only process in the pipeline: the H02/H03 wrapper or candidate `-c` path must assemble to an object; `-S` alone is not a compile pass (T00 §1 item 1).
6. The exact per-instance matrix is H03's frozen manifest, not this document.

## 3. H04 Existing Behavior (EXISTING)

### 3.1 CLI grammar as implemented

```text
candidate [-std=c11] [-O0] [-D NAME[=value]] [-U NAME] [-o <path>]
          [--emit-ir-snapshot] [--emit-trace] [--interpret-ir] file.c
candidate -E [-D NAME[=value]] [-U NAME] [-o <path>] file.c
```

- Flags and the positional source may be interleaved; exactly one positional source is required (`candidate.rs` parse loop).
- `-h`/`--help` prints usage and exits 0.
- `-S` and `-c` are rejected before input is read; exit 2; no output is created.
- `-E` selects preprocessed emission and conflicts with any evidence-selection flag; conflict exits 2.
- With no evidence-selection flag, all three evidence sections are emitted.
- `-D`/`-U` may be attached (`-DFOO=1`, `-UFOO`) or separate (`-D FOO=1`); a separate form consumes the next argv unconditionally.
- `-I` and any `-I<dir>` are rejected with a deferred-feature message, exit 2.
- Unknown flags and any second positional source exit 2.
- `-o` is used for either the evidence report or `-E` bytes; missing path exits 2.
- Outputs go to stdout unless `-o` is present.
- Exit codes: 0 success; 1 candidate diagnostic; 2 driver/usage/refusal; a Rust panic currently escapes as 101.

### 3.2 Source and configuration handling

- One input is read with `std::fs::read`; source bytes are committed under a name derived from `file_name()`.
- The compiler config is always `TargetSpec::aarch64_unknown_linux_gnu_unverified()`, `Dialect::C11`, `OptLevel::O0`, no option flags, `Limits::fixture()`.
- `-D`/`-U` are committed before the first PP task in flag order, with a synthetic `<command-line>` source for non-empty values.
- The PP prefix runs PP01→PP04, PP19 conditional, PP09 expansion, PP05 directive over the single source. PP17/PP18 include chips are registered as frozen kinds but are **not installed or routed** by the candidate driver.
- `DRAIN_BUDGET = 16` ticks per step; a step that is not terminal after the budget returns a driver error.
- `Limits::fixture()` caps one source at 1 MiB, total records at 4 Mi records, intern bytes at 4 MiB, and ticks at 1 Mi; these are compiler-job limits, not H05 process limits.

### 3.3 Evidence and determinism

- Evidence text: contract version, `input`, optional `snapshot`, optional `trace`, optional `interpret`.
- Snapshot and trace hashes are deterministic for the same initial state/config/source; existing tests pin byte-identical reruns.
- Snapshot encoding includes the config hash (target, dialect, opt level, options, limits), so the hash binds configuration even though the text report does not name it.
- `-E` output is deterministic for the M1 fixture; no line markers; one space between emitted tokens.

## 4. H04 Required Flag Matrix (Plan; Not Frozen)

### 4.1 Complete flag-family matrix

Legend: **E** = existing; **P** = partial; **D** = deferred/refused; **O** = open decision; **B** = blocked on a freeze/probe.

| Family | Spellings / arity | Current (E/P/D) | Required acceptance behavior | Status |
|---|---|---|---|---|
| Help | `-h`, `--help` | E (exit 0) | Keep; never executes a source | E |
| Dialect | `-std=c11`, `-std c11` | P: only `c11`; validated then discarded; config hardcodes C11 | Record dialect; default dialect must come from H07 reference probe; support the frozen dialect list or explicitly `UNSUPPORTED` | B (H07/T01) |
| Optimization | `-O0`, `-O`, `-O1`, `-O2`, `-O3`, `-Os`, `-Og` | P: only `-O0`; not stored; config hardcodes O0 | Accept the frozen profile list; record each level; implement observable option effects (notably `__OPTIMIZE__`) or fail explicitly; never silently treat as O0 | B (H03/T01) |
| Debug | `-g`, `-g0..-g3` | D: unknown | Accept `-g` for the frozen rows; disclose that no debug info is emitted; cases requiring debug/dump/scans fail | B (T00/T11) |
| Warnings | `-w`, `-W...`, `-Werror`, `-pedantic*` | D: unknown; `-w` is passed by the official driver | Accept `-w` as warning suppression (current compiler has no warning output to suppress, so record a no-op limitation); other warning flags explicit unsupported until frozen | B (H03) |
| Function options | `-fomit-frame-pointer`, `-funroll-loops`, `-fpeel-loops`, `-ftracer`, `-finline-functions`, `-fno-inline`, `-ffloat-store`, `-fwrapv`, `-fno-strict-aliasing`, ... | D: unknown | Record the frozen set; optimization hints may be no-ops only where semantics permit; semantic options (`-fwrapv`, fast-math, etc.) are not silently accepted | B (H03) |
| Target options | `-m*`, `-march`, `-mcpu`, `-mabi` | D: unknown | Reject unless the frozen target profile explicitly permits; never change the frozen identity silently | B (H01/T01) |
| Define | `-DNAME`, `-D NAME`, `-DNAME=value`, `-D NAME=value`, `-DNAME=` | E: bare = `1`; empty stays defined; first `=` splits; ASCII names; invalid name/value exit 2 | Keep; add the frozen quoting table; document function-like and non-ASCII omissions | E (extensions O) |
| Undefine | `-UNAME`, `-U NAME` | E: tombstone; last-wins; `=` rejected | Keep | E |
| Include | `-I<dir>`, `-I <dir>` | D: exit 2 | Ordered search path; see §5; no silent merge/drop; missing/escaped root fails closed | B (H01/T01/T03/T02) |
| Include variants | `-isystem`, `-iquote`, `-idirafter`, `-include`, `-imacros`, `-nostdinc`, `-undef` | D: unknown | Either implement with a frozen policy or report explicit `UNSUPPORTED`; never continue as if the flag had no effect | B (H03) |
| Dependency output | `-M`, `-MM`, `-MD`, `-MMD`, `-MF`, `-MT`, `-MQ` | D: unknown | Explicit unsupported unless H03 requires; no silent ignore | B (H03) |
| Preprocess emit | `-E` | E: PP28 stdout/`-o`; clashes with evidence flags; no line markers | Keep host-only; document not GCC-compatible; exact corpus use must be H03-defined | E (compatibility O) |
| Assembly | `-S` | D: exit 2 fail-closed | After target attestation and codegen, emit assembly to `-o` or a default; no partial artifact | B (C02/T11) |
| Object | `-c` | D: exit 2 fail-closed | After target attestation, produce an ELF relocatable object; `-c` must not be a disguised `-S`; assembler boundary decided in §6 | B (C02/T11/H04) |
| Link | `-o exe` plus objects/sources, `-l`, `-L`, `-Wl,`, `-static`, `-shared`, `-pie`, `-no-pie` | D: unknown / multi-source refused | H03-frozen link rule; external linker only; no C compiler; record exact argv | B (H01/H03) |
| Output | `-o <path>` | E: last occurrence wins; direct write | Freeze single vs multiple input rules; atomic write; no clobber; reject directory target | P |
| Source positionals | exactly one `.c` | E | Exactly one now; multi-source after §4.3 freeze | P |
| `.S` sources | positional `.S` | D: counted by the one-source rule but treated as a C pipeline input | Preprocess via candidate PP, assemble via external assembler; record distinctly from C | B (H03/H04) |
| Language override | `-x c`, `-x assembler-with-cpp`, ... | D: unknown | Explicit unsupported unless H03 requires; no guessing by extension | B (H03) |
| Response file | `@file` | Not recognized; currently falls through as a positional path | Explicit reject as unsupported; never read a response file implicitly (security/quoting) | O |
| Standard input | `-` | D: unknown flag | Explicit unsupported; no stdin source in the frozen profile | O |
| End of options | `--` | D: unknown flag | Freeze either end-of-options or explicit unsupported; do not treat the next token specially by accident | O |
| Driver internals | `-v`, `-###`, `-B`, `-wrapper`, `-dumpbase`, `-dumpdir`, `-save-temps` | D: unknown | Explicit unsupported unless H03 requires; H03 may add `-dumpbase` for output naming | B (H03) |
| LTO | `-flto*`, `-fno-fat-lto-objects`, ... | D: unknown | Explicit `UNSUPPORTED`; LTO rows are a separate profile and must not be silently counted | B (T00 profile) |
| Evidence | `--emit-ir-snapshot`, `--emit-trace`, `--interpret-ir` | E | Keep; they are Part A evidence flags, not target output flags | E |
| Target report | proposed `--target-report <path>` or equivalent | Absent | Required only to open Part B; the exact spelling/config interface is T01/H04-owned; do not invent a private config | B (T01) |
| Unknown | any other `-...` | E: exit 2 | Keep; structured error; no fallback | E |

### 4.2 Flag ordering, repetition, and environment rules

| Rule | Current | Required decision |
|---|---|---|
| Order of `-D`/`-U` | Flag order; last macro ID wins | Keep; test interleavings |
| Order of `-std`/`-O` | Validated only; no propagation | Freeze last-wins vs first-wins; GCC last option wins for these; record the effective value |
| Order of `-o` | Last wins | Freeze; multiple `-o` should at least be recorded; GCC behavior may warn |
| `-c` + `-S` | First one refused now | Freeze mutual exclusion or last-wins; do not silently prefer one |
| `-E` + `-S`/`-c` | `-S`/`-c` rejected before `-E` | Freeze precedence (GCC `-E` stops after preprocessing) |
| `-E` + evidence flags | Exit 2 | Keep |
| Duplicate source path | Exit 2 (N≠1) | Freeze reject vs dedupe for multi-source |
| Environment include variables (`CPATH`, `C_INCLUDE_PATH`, `CPLUS_INCLUDE_PATH`) | Not read | Explicitly ignore or fail closed; never let host environment change the search list |
| `TMPDIR`, `HOME`, `PATH` | Inherited by the process | H05 must clear/allowlist; candidate itself must not use them for language decisions |
| Locale/timezone | Inherited | H05 pins `LC_ALL=C`, `TZ=UTC`; candidate must not use locale for C semantics |
| Response files | Not read | Explicit unsupported |
| Shell | Never used; argv is passed directly | Keep; H04 must not introduce `sh -c` for quoting |

### 4.3 Quoting and multi-source rule table

| Scenario | Current behavior | Proposed acceptance rule | Status |
|---|---|---|---|
| Source path with spaces | Works when one argv (tests use `PathBuf`) | One argv = one path; no shell splitting | E |
| `-D` value with spaces | Works when one argv | Keep value verbatim to the frozen scanner | E |
| `-I` path with spaces | Refused with `-I` | Future: one argv = one directory; preserve exactly | B |
| `-I ""` | Refused with `-I` | Future: reject empty path as malformed; never treat as CWD | O |
| `-I` path starting with `-` | Refused with `-I` | Future: separate form consumes the next argv as the directory (freeze); attached form takes the remainder verbatim | O |
| `=` in `-I` path | Refused with `-I` | Literal character; no split | O |
| `=` in `-D` | First `=` splits name/value | Keep; subsequent `=` stay in the value | E |
| `--` | Unknown flag, exit 2 | Freeze end-of-options vs explicit unsupported | O |
| Filename beginning `-` | Unknown flag unless `./-name.c` | Document `./` workaround; do not guess | O |
| Backslash or quote characters in a path/value | Literal bytes; no shell interpretation | Preserve exactly; never unescape or re-quote; the harness supplies already-split argv | O (document) |
| Newline/control characters in path | Accepted; printed raw | Accept as path bytes; escape all diagnostics/evidence text | O (security) |
| Empty source path (`""`) | Treated as one positional path; read fails exit 1 | Explicit malformed driver error; no accidental default source | O |
| NUL byte in path | Impossible in argv | Reject embedded NUL from any non-argv config; never truncate silently | O |
| Trailing separator / `.` / `..` in path | Passed to OS read; no canonicalization | Freeze preserve-vs-canonicalize after the sandbox policy; never silently reorder `-I` entries | O |
| Non-UTF-8 argv | Panic (exit 101) | Never panic; either support OS-byte paths or reject with exit 2 and escaped text | Defect F-01 |
| Non-UTF-8 `-D` name/value | Panic at argv collection before parsing | Define byte policy; likely reject non-UTF-8 macro text explicitly | Defect F-01 |
| Duplicate `-I` directories | Refused with `-I` | Preserve order; do not silently dedupe; record duplicates | B |
| Relative `-I` | Refused with `-I` | Resolve against startup CWD, pin; no mid-run CWD change | B |
| Symlink `-I` | Refused with `-I` | Canonicalize; reject escapes outside pinned roots; document TOCTOU | B |
| `~`/environment/glob expansion | None | Never expand; the harness passes literal argv | E (document) |
| `@file` | Treated as a positional path | Explicit unsupported; do not read response files | O |
| `-o -` | Writes a file named `-` | Freeze stdout-vs-file semantics; do not guess | O |
| Output path equals input | Works for `-E` because input is read first | Future `-c`/`-S`: reject or use temp+rename; never clobber source | O |
| Multiple `.c` sources | Exit 2 | One TU per primary source; independent frontend stores; order = argv/manifest order; link per H03 | B |
| Multiple sources with same basename | Exit 2 now | Identity must be path-based, not basename-based; reject or disambiguate explicitly; test | B |
| Same source path repeated | Exit 2 now | Freeze reject vs dedupe; no double count | O |
| `.c` + `.S` mix | Exit 2 now | `.S` through PP + external assembler; record distinct path | B |
| Auxiliary sources from `.x`/manifest | Not accepted | H03 manifest owns exact list/order; no filename special-casing | B |
| `-E` multiple TUs | Not accepted | One output per TU; naming/no-clobber rule open; H05 instance dir applies | B |
| `-c` multiple TUs | Not accepted | One object per TU; `-o` only if one input; default names derived from primaries with collision check | B |
| Link multiple objects | Not accepted | H03-frozen link command; external linker only | B |
| Source with non-UTF-8 bytes | Bytes are read; PP scanner behavior not audited here | Preserve bytes; diagnostics must not assume UTF-8; no lossy decode of source content | O |
| Empty source | Pipeline diagnostic | Keep structured; test | E |
| Source > `max_source_bytes` | "source rejected" exit 1 | Classify as resource/limit, not a source-semantic failure; structured code | P |

### 4.4 Flag-to-phase matrix

This is the phase contract the H04 freeze must make explicit. Cells marked "planned" are not implemented today.

| Flag/group | Preprocess | Compile/codegen | Assemble | Link | Run | Notes |
|---|---|---|---|---|---|---|
| `-E` | Emit and stop | — | — | — | — | Host-only PP28 stream; not GCC-compatible |
| `-S` | Run | Emit assembly | — | — | — | Part B, probe-gated |
| `-c` | Run | Run | Produce object | — | — | Part B; assembler boundary §6.3 |
| default (no `-E/-S/-c`) | Run | Run | Produce object | Link executable | Optional run by harness | Link rule H03; candidate may not link |
| `-o` | — | — | — | — | — | Selects output for the active mode; multi rules open |
| `-I` | Search headers | — | — | — | — | Host fetch + PP17/PP18 after freeze |
| `-D`/`-U` | Seed macros | — | — | — | — | Flag order, last-wins |
| `-std` | Dialect | Dialect | — | — | — | Default from H07 probe |
| `-O*` | — | Optimization semantics | — | — | — | Observable macros/e.g. `__OPTIMIZE__` |
| `-g` | — | Debug info | — | — | — | Disclose if not emitted |
| `-w`/`-W*` | Diagnostics | Diagnostics | — | — | — | No silent warning policy |
| `-f*` | Some | Codegen semantics | Some | — | — | Frozen per option |
| `-m*` | Some | Target codegen | Some | — | — | Target identity constrained |
| `-l`/`-L`/`-Wl,`/`-static`/`-shared` | — | — | — | Link | — | H03 link rule |
| `--emit-ir-snapshot`/`--emit-trace`/`--interpret-ir` | Run M1 pipeline | Interpret | — | — | — | Part A evidence |
| target report flag | — | Gate | Gate | Gate | — | Part B only |

## 5. `-I` Search Path, Open Quoting, and Escape Rules

### 5.1 Frozen starting point (`/25`)

The frozen PP include slice resolves includes **only** over already committed sources:

- quote and angle behave identically;
- delimiters are stripped;
- exact full-name match first, then basename suffix;
- first match wins; multiple basename matches fail as ambiguous;
- `include_next` is `Unsupported`;
- empty header name is malformed;
- macro-generated header names are `Unsupported`;
- single-pass stitch only; nested `#include` survives for a future control loop;
- the chip is host-passive; missing headers are `not-loaded` for a host loop to fetch.

This is **FROZEN** behavior. It is not an `-I` search policy and does not authorize filesystem access.

### 5.2 Draft proposal (NOT frozen)

[H04_I_FREEZE](../../tasks/H04_I_FREEZE.md) proposes, among other things:

- `-I <dir>` repeatable, flag order, relative to invocation CWD pinned at startup;
- quote form: including-file directory, then `-I` dirs, then H01 system/sysroot roots;
- angle form: `-I` dirs, then H01 system/sysroot roots (no including-file directory);
- exact filename match per directory; no enumeration, no extension guessing;
- first hit wins; deterministic order independent of filesystem iteration;
- system roots only from H01 pins; never hardcode `/usr/include`, never sample host environment, never use another compiler's search list;
- sandbox: configured roots only; absolute paths, `..`, and symlink escapes outside pinned roots fail closed;
- missing/empty/ambiguous/`include_next`/macro-generated names are loud typed failures;
- host fetch loop, bounded retries, one TU per primary source, and external assembler/linker rules.

Every item is **OPEN** and requires H01 + T01 sign-off. Nothing in this plan freezes it.

### 5.3 Decisions required before `-I` implementation

| ID | Decision | Proposed direction | Owner/gate |
|---|---|---|---|
| D-I-01 | Search order: quote vs angle, including-file directory, `-I`, system roots | H04_I_FREEZE §2.2 order; first hit wins | H04 + T01 + H01 |
| D-I-02 | Config owner for include roots and system roots | T01-owned `config`; H04 must not invent a parallel config | T01 integrator |
| D-I-03 | Relative path base | Invocation CWD, pinned at startup; no CWD changes during a job | H04 |
| D-I-04 | Canonicalization/sandbox | Canonical absolute roots; reject missing roots; reject `..`/symlink escapes; document TOCTOU | H01 + H04 security review |
| D-I-05 | Exact match policy | Byte-exact filename between delimiters; no case folding, extension guessing, or directory enumeration | H04 + T03 |
| D-I-06 | Duplicate `-I` dirs | Preserve order; do not silently dedupe; record duplicates | H04 |
| D-I-07 | Empty/control/non-UTF-8 `-I` | Empty is malformed; control bytes escaped; non-UTF-8 policy per argv decision | H04 |
| D-I-08 | Failure taxonomy | not-found (count only), ambiguous (only canonical equality), malformed, unsupported; typed and loud | T03/H04 |
| D-I-09 | Header identity / `#pragma once` | T01-owned once-set; path vs content hash vs `SourceId` unresolved; PP25 has no store carrier today | T01 + T03 |
| D-I-10 | Nested include/fixpoint | T02 control loop; host pre-loading must not fake recursion | T02 |
| D-I-11 | Host fetch retry/depth | Bounded retries/depth in T01 config; any exhaustion is a diagnostic, not a spin | T01/T02 |
| D-I-12 | PP17/PP18 integration | Host must commit the fetched header with a name PP17 can resolve; duplicate basenames must not create ambiguity | T03 + H04 |
| D-I-13 | System header availability | H01 sysroot pins; `<stdio.h>` etc. fail closed until pinned | H01 |
| D-I-14 | Environment include vars | Ignore or fail closed; never change search order based on host env | H04/H05 |
| D-I-15 | Evidence | Record pinned roots, resolution result, and searched count; normalize host paths; no header contents in diagnostics beyond necessary location | H04/H06 |
| D-I-16 | Absolute `#include "/x"` | Fail closed outside roots; do not silently open absolute paths | H04 security |

### 5.4 Critical integration gap

PP17 resolves over committed sources by name/basename. If the host search selects `a/foo.h` but the driver commits it under the stem `foo.h`, a second `b/foo.h` can become ambiguous or be selected incorrectly. The H04/H03 freeze must define a source-name scheme (for example a path relative to the instance root) that preserves uniqueness and matches PP17's exact-name rule, or T03 must change PP17's policy. Simply pre-loading a file is not sufficient. This is an **open defect risk**, not a current behavior claim.

## 6. `-S`/`-c` Fail-Closed Gating

### 6.1 Current gate (EXISTING)

`candidate` rejects `-S` and `-c` in `parse_args`, before reading the input:

- exit 2;
- message: "target emission is fail-closed while the target is unverified (Part B, probe-gated)";
- no output file is created;
- the test asserts the output path does not exist.

The gate is currently **hard-coded** because the driver always constructs `TargetSpec::aarch64_unknown_linux_gnu_unverified()`. It is not driven by a runtime verification check, and there is no flag to supply a probe report.

### 6.2 Gate conditions to open Part B

All of the following must be satisfied; none is satisfied today:

1. H01 has run the probe on the frozen Linux AArch64 substrate and produced a hashed report.
2. H07 has classified the four deliberately `unresolved` ABI fields (`abi.gp_arg_regs`, `abi.fp_arg_regs`, `abi.stack_align`, `abi.variadic_register_save_area`).
3. The T01 integrator has called `TargetSpec::attest` with resolved values and a matching report hash; the target is `VerificationState::Probed`.
4. T11 codegen exists for the relevant subset and verifies target invariants.
5. H04 contract has frozen `-S`/`-c` output naming, `-o` rules, mode precedence, and the `-c` assembler boundary (§6.3).
6. H05 can supervise the assembler/linker with timeout/output/resource limits and record evidence.
7. H01 has pinned the assembler, linker, sysroot, runtime libraries, and inspection tools.

### 6.3 `-c` boundary decision (OPEN)

`-c` must produce an object for the T00 compile suite; `-S` alone is not a pass. Three options:

| Option | Description | Advantage | Risk |
|---|---|---|---|
| A | Candidate `-c` emits assembly internally, invokes the H01-pinned assembler, and writes the object | Matches compiler-driver convention; one invocation for the harness | Candidate needs a supervised subprocess boundary; must not duplicate H05 poorly; assembler failure handling/atomicity required |
| B | Candidate implements only `-S`; H02/H03 harness invokes the assembler and names the object | Clean separation; reuses H05 supervisor | H04 `-c` flag requirement remains unmet unless the H04 contract explicitly defers it; instance command classification must not pretend `-c` ran |
| C | Candidate `-c` writes assembly to a `.o` path | No subprocess | **Rejected**: a non-object `.o` is fake success and violates T00 |

Recommendation: **Option A after the H04 contract and H05 supervisor are approved**, or **Option B only with an explicit H04 contract amendment** that records `-c` as harness-composed. Do not silently substitute `-S` for `-c`.

### 6.4 Pre/post gate behavior matrix

| Invocation | Before gate | After gate (planned) |
|---|---|---|
| `-S -o x.s f.c` | Exit 2; no `x.s` | Assembly emitted only if target attested and codegen ready; atomic write |
| `-c -o x.o f.c` | Exit 2; no `x.o` | ELF object only if assembler path accepted; atomic |
| `-S f.c` | Exit 2 | Default output name frozen (`f.s` or equivalent); H05 cwd rules |
| `-c f.c` | Exit 2 | Default `f.o`; duplicate basenames must not collide |
| `-S -c f.c` | First flag refused | Freeze mutual exclusion / precedence |
| `-E -S f.c` | `-S` refused | Freeze `-E` precedence |
| `-S` with unverified target | Exit 2 | Still exit 2 with the same fail-closed reason |
| `-S` with a forged/absent report | N/A | Reject; no env/CLI bypass; `ensure_codegen_ready` called before writes |

### 6.5 No-bypass rules

- No environment variable, hidden flag, default target, or test-only constructor may mark the target verified.
- `VerificationState` remains private; only `TargetSpec::attest` may produce it.
- The driver must not call `TargetSpec::attest` on a report the user did not explicitly supply and the integrator did not accept.
- A missing/invalid report is a driver error, not a fallback to macOS arm64 or host ABI.
- `ensure_codegen_ready()` must be called on the exact config used for emission, before any artifact is opened.
- No partial `.s`/`.o` may survive a failed gate or tool invocation.

## 7. H05 Isolated Execution — Requirements and Decisions

### 7.1 Current state (ABSENT)

There is no runner in `tools/torture/src` (only `diagnostic`, `digest`, `lock`, `path`, `verify`), no process-spawn module, no timeout, no resource limit, no cancellation, no crash taxonomy, and no per-instance directory manager. `tools/torture/probe` is the H01 reference probe; it runs only the pinned reference GCC and is not a candidate runner. The H04 test temp directories are test scaffolding, not H05.

T00 H05 acceptance is unmet:

> A malicious hang does not block the whole suite; identical basenames do not collide.

### 7.2 Requirement table

| ID | Requirement | Current | Acceptance evidence needed |
|---|---|---|---|
| R-H05-01 | One fresh directory per instance, not derived from basename | Absent | Two same-basename instances run concurrently without file/summary collision |
| R-H05-02 | Stable instance identity and evidence paths | Absent | Instance ID stable across reruns; path relative; no host path in canonical record |
| R-H05-03 | Wall/CPU timeout per phase and total | Absent | Hang test terminates within bound; suite continues |
| R-H05-04 | stdout/stderr limits without pipe deadlock | Absent | Flood test completes/terminates; truncation recorded; no hang |
| R-H05-05 | Memory/file/process/file-descriptor/core limits | Absent | Bomb tests classified, not passed or hung |
| R-H05-06 | Cancellation with grace and kill escalation | Absent | Cancel test stops new work, kills active, marks CANCELLED |
| R-H05-07 | Crash classification (exit/signal/timeout/limit/spawn/ICE) | Absent | Fixture matrix classifies each case |
| R-H05-08 | One hang does not block unrelated instances | Absent | 3-instance test with one hang; others complete |
| R-H05-09 | Process-tree cleanup/orphan policy | Absent | Descendant cleanup test or documented residual |
| R-H05-10 | Sanitized env/cwd/stdin | Absent | Env probe shows no secrets; cwd is instance dir |
| R-H05-11 | Assembler/linker boundary recorded and supervised | Absent | Command log shows no C compiler; tool hashes/versions |
| R-H05-12 | Target runner identity/exit propagation | Absent | Linux/qemu run record; exit status verified |
| R-H05-13 | Deterministic evidence and replay | Absent | Three-run identical canonical records |
| R-H05-14 | Concurrency without global CWD/env mutation | Absent | Parallel tests use `Command` env/cwd only |
| R-H05-15 | No false PASS on harness/limit/timeout error | Absent | Self-test rejects fake PASS/skip/drop |
| R-H05-16 | Flat retry policy for infrastructure errors only | Absent | Timeout not retried; spawn error retried once |
| R-H05-17 | Approved process-group/rlimit boundary | Absent | Security review or external supervisor pin |
| R-H05-18 | Platform guard: Linux AArch64 for target execution | Absent | macOS logic tests do not claim target results |
| R-H05-19 | H02/H03/H06 integration contract | Absent | Runner record consumed without reinterpretation |
| R-H05-20 | Hostile-fixture acceptance suite | Absent | §9.3 matrix passes |

### 7.3 Decision table

| ID | Decision | Proposed direction | Status |
|---|---|---|---|
| D-H05-01 | Instance directory naming | `<run_root>/<run_id>/<instance_id>/attempt-<n>/`; `instance_id` = hex of canonical instance tuple; never basename-only; create exclusive; 0700 | OPEN |
| D-H05-02 | Run root | Configured path outside corpus/toolchain; disk quota; no shared `tmpdir`; no world-writable parent | OPEN |
| D-H05-03 | Source staging | Per-instance read-only copy or hash-pinned absolute read; source tree never written; input hash recorded | OPEN |
| D-H05-04 | Working directory | Instance dir; no write to corpus/toolchain; all relative outputs land there | OPEN |
| D-H05-05 | Environment | Clear then allowlist (`PATH`, `LC_ALL=C`, `LANG=C`, `TZ=UTC`, `HOME`, `TMPDIR` inside instance); no credentials; record env hash not full env | OPEN |
| D-H05-06 | stdin | `/dev/null` or frozen bytes; close write end; no inherited terminal | OPEN |
| D-H05-07 | Timeouts | Per compile/assemble/link/run and total; monotonic clock; grace SIGTERM then SIGKILL; timeout not retried; record configured values | OPEN |
| D-H05-08 | Output limits | Concurrent draining; retain first/last bounded bytes or first N; hash full stream while counting; truncation flag; no pipe deadlock; no disk fill | OPEN |
| D-H05-09 | Resource limits | CPU, wall, AS/RSS, FSIZE, NOFILE, NPROC/threads, STACK, CORE=0; Linux mechanism (`prlimit`/cgroup/approved helper); fail closed if required limit unavailable | OPEN |
| D-H05-10 | Cancellation | Token/API; stop scheduling; signal active with grace; kill; `CANCELLED`; partial evidence; distinct exit; no PASS | OPEN |
| D-H05-11 | Classification taxonomy | `EXIT(code)`, `SIGNAL(name)`, `TIMEOUT`, `OUTPUT_LIMIT`, `RESOURCE_LIMIT`, `SPAWN_ERROR`, `TOOL_CRASH`, `CANDIDATE_DIAGNOSTIC`, `UNSUPPORTED`, `ICE`, `CANCELLED`, `INFRASTRUCTURE_ERROR`; H02/H03 judge semantics | OPEN |
| D-H05-12 | Duplicate basenames | Instance ID + per-instance output paths; never `tmpdir/basename`; test concurrent same-name cases; same-instance multi-source collisions explicitly handled | OPEN |
| D-H05-13 | Concurrency | Bounded workers; per-instance state; no `set_current_dir`/global env; deterministic result order by instance ID | OPEN |
| D-H05-14 | Target runner | Native Linux AArch64 preferred; qemu only with verified exit propagation; record runner identity; ELF/architecture check before execution | OPEN |
| D-H05-15 | Assembler/linker boundary | Explicit allowlist; exact argv arrays; no shell; no `.c`/AST/IR to GCC/Clang; versions/hashes; timeout/output limits; no fallback | OPEN |
| D-H05-16 | Evidence retention | Raw logs bounded; failure evidence retained; canonical JSON deterministic; timestamps only in non-canonical sidecar; no deletion to improve rate | OPEN |
| D-H05-17 | Retry policy | At most one retry for spawn/transport/infrastructure; never for TIMEOUT/FAIL/WRONG_CODE; original attempt retained | OPEN |
| D-H05-18 | Isolation strength | Per-instance dir + process/env control, **not** a security sandbox; document filesystem/network residual risk; optional namespace/container is a separate approval | OPEN |
| D-H05-19 | Platform guard | Reject target execution on non-Linux/non-AArch64 unless a verified emulator is explicitly selected; macOS is for logic tests only | OPEN |
| D-H05-20 | Ownership | H05 owns runner; H02/H03 call it; H06 consumes records; H04 does not duplicate isolation; new package/CI is integrator-owned | OPEN |

### 7.4 Classification mapping to T00/H06

H05 observes process facts; it must not decide that a program is semantically correct.

| H05 fact | H06/T00 mapping | Notes |
|---|---|---|
| Exit 0 and expected assertion holds | PASS | H02/H03 judge the assertion, not H05 |
| Exit nonzero | FAIL / WRONG_CODE depending on test | Not automatically a crash |
| Signal (self) | RUN_CRASH | Record signal name/number; not a timeout |
| Timeout kill | TIMEOUT | Never convert to PASS; retain in denominator |
| Output limit | OUTPUT_LIMIT (fail) unless the instance policy says truncation is benign | Freeze policy per H03 |
| Resource limit | RESOURCE_LIMIT (fail) | Distinguish from timeout where possible |
| Spawn/tool failure | INFRASTRUCTURE_ERROR or LINK_FAIL | Retains instance; blocks final conclusion if infrastructure |
| Candidate structured diagnostic | FAIL/UNSUPPORTED | H04 exit/diagnostic taxonomy must disambiguate |
| Candidate panic/ICE | ICE | Never PASS; retain raw evidence |
| Cancelled | CANCELLED (run incomplete) | The instance stays in the frozen denominator; the cancelled attempt is not a PASS/FAIL result; rerun required before any gate conclusion |
| Harness bug | INFRASTRUCTURE_ERROR | Never attribute to candidate |

Signal-origin note: a `SIGKILL` observed by the parent may originate from the runner's timeout/cancel escalation, an external OOM killer, or a process-group cleanup. H05 must record a `kill_origin` (`timeout`, `cancel`, `external`, `unknown`) alongside the raw signal and must not infer a self-inflicted crash from the signal number alone. `SIGXCPU`/`SIGXFSZ` are resource-limit evidence, not semantic failures.

## 8. Runner / Assembler / Linker Boundary

### 8.1 Role separation

| Role | Owns | Must not do |
|---|---|---|
| Candidate `candidate` | Read source bytes, run compiler chips, emit evidence/preprocessed/assembly/object per frozen flags | Invoke GCC/Clang on C/AST/IR; perform link-time semantic work; read corpus/test manifests |
| H04 driver contract | Flag/source/quoting semantics, output naming, `-c` boundary decision, exit taxonomy | Own H05 isolation or H02/H03 judging |
| H05 runner | Spawn/supervise pinned tools and untrusted target programs, isolate each instance, enforce limits, classify facts | Decide semantic PASS/FAIL; compile C; rewrite test expectations |
| H01 substrate | Pin Linux AArch64 runner, reference GCC, assembler, linker, sysroot, runtime, versions/hashes | Run candidate or produce candidate evidence |
| H02/H03 | Official driver, instance enumeration/manifest, judging, exact flag matrix, link rule | Fabricate flags/denominators; let candidate fall back |
| H06 | Aggregate, dual denominators, evidence retention schema | Hide timeouts/unsupported/crashes |

### 8.2 Allowed and forbidden tool use

Allowed explicit dependencies (T00 §3):

- external assembler;
- external linker;
- system libc/libm/compiler-rt/libgcc;
- a documented emulator with verified exit-code propagation;
- H01-pinned sysroot and runtime.

Forbidden:

- passing candidate C source, preprocessed source, AST, or IR to GCC/Clang to generate the candidate program;
- using an external preprocessor for the final end-to-end claim;
- shell wrappers that re-split arguments or expand globs;
- silent fallback to another C compiler;
- accepting a flag and doing nothing when it has observable semantics;
- writing an assembly file with an object extension and calling it `-c` success.

### 8.3 Link-only GCC/Clang caveat

If the frozen link rule uses `gcc`/`clang` as a **link-only driver**, the recorded argv must contain only objects, libraries, linker flags, and runtime arguments — never `.c`, `.i`, `.s` intended for compilation, AST, or IR. The command must be labeled `link-only` and reviewed under T00. Prefer direct `ld` with pinned startup objects/sysroot where practical.

### 8.4 Command evidence fields

Every tool invocation must record:

- tool identity (name, absolute path, version, binary hash);
- argv as an array (not a shell string);
- cwd (relative to instance root);
- env allowlist hash;
- input artifact hashes;
- output artifact hashes;
- exit code or signal;
- timeout/limit flags;
- stdout/stderr byte counts, retained byte counts, truncation flags, hashes;
- start sequence/attempt, not wall-clock in canonical fields.

## 9. Test and Acceptance Matrix

### 9.1 H04 existing tests (verified in this audit)

Command:

```sh
cargo test --locked --manifest-path compiler/Cargo.toml --test h04_candidate
```

| Test | Expected | Result |
|---|---|---|
| `candidate_emit_golden` | Bare `-E` emits the golden token stream | PASS |
| `candidate_emit_is_deterministic` | Two `-E` runs byte-identical | PASS |
| `candidate_emit_writes_to_output_path` | `-E -o` writes the file, stdout empty | PASS |
| `candidate_emit_rejects_error_directive` | `#error` exits 1, no output file | PASS |
| `candidate_emit_clashes_with_evidence_selection` | `-E` + evidence flag exits 2 | PASS |
| `candidate_models_m1_return_5` | Evidence value `05`, snapshot/trace present | PASS |
| `candidate_evidence_is_deterministic` | Two evidence runs identical | PASS |
| `candidate_rejects_invalid_input_with_diagnostic` | Exit 1; no snapshot/interpret | PASS |
| `candidate_refuses_target_emission` | `-S` exit 2; no file; "fail-closed" | PASS |
| `candidate_rejects_unknown_flags` | Exit 2 | PASS |
| `candidate_writes_evidence_to_output_path` | `-o` report file | PASS |
| `candidate_define_bare_defaults_to_one` | Bare `-D` = `1` | PASS |
| `candidate_define_value_expands` | `-DFOO=2+3` expands | PASS |
| `candidate_undef_removes_prior_define` | `-U` leaves spelling | PASS |
| `candidate_define_last_wins` | Repeated `-D/-U` last wins | PASS |
| `candidate_empty_define_stays_defined` | `-DFOO=` stays defined | PASS |
| `candidate_rejects_bad_macro_flags` | Bad names/values exit 2 | PASS |
| `candidate_define_leaves_m1_evidence_intact` | `-D` does not break evidence | PASS |

### 9.2 H04 required new tests (not implemented)

| ID | Case | Expected | Gate |
|---|---|---|---|
| T-H04-01 | `-I dir`, `-I=dir`, `-I` at end | Exit 2; no source read; explicit deferred/unsupported message | Now (behavior already exists; add test) |
| T-H04-02 | Two `.c` positionals; `.c` + `.S` | Exit 2; no output; no partial compile | Now |
| T-H04-03 | Non-UTF-8 argv path/flag | Exit 2 or a defined OS-path behavior; never exit 101/panic | Now (fails today) |
| T-H04-04 | Input file missing | Structured driver/IO error; no evidence; classification decided | Now |
| T-H04-05 | Source exceeds `max_source_bytes` | Structured limit result; no panic; no partial evidence | Now |
| T-H04-06 | Output path unwritable/directory | Exit 2; no partial file; original destination untouched | Now |
| T-H04-07 | `--`, `-`, `@file`, `-o -` | Explicit defined behavior; no accidental file read | Now (freeze first) |
| T-H04-08 | Path with spaces, `=`, newline, control, leading `-` via `./` | Accepted or explicit error; diagnostics escaped; no log injection | Now |
| T-H04-09 | `-D` separate/attached, empty, first `=`, spaces, newline | Same semantics; no shell splitting | Now |
| T-H04-10 | `-D` function-like / non-ASCII name | Explicit rejection with clear message | Now |
| T-H04-11 | Repeated `-o`, `-std`, `-O`, `-c/-S` | Frozen precedence; record effective values | After H04 freeze |
| T-H04-12 | `-E` output atomicity and deterministic bytes | No partial `.i`; byte-identical rerun | Now |
| T-H04-13 | Evidence contains dialect/opt/target state/flags | All required metadata present; deterministic order | Now |
| T-H04-14 | `-w`, `-g`, `-O1..`, `-std=gnu*`, `-f*` | Accepted/recorded with semantics or explicit UNSUPPORTED; never silent | After H03/H04 freeze |
| T-H04-15 | `-I` quote/angle/order/system roots/escaping | §5 matrix after freeze | Blocked H01/T01/T03/T02 |
| T-H04-16 | Same basename in two source dirs, `-I` ambiguity | No collision; explicit ambiguity/error | Blocked H04/H03 |
| T-H04-17 | Multi-source `-E`/`-c`/`-S`/link naming | Frozen rules; no clobber | Blocked H03/H04 |
| T-H04-18 | `-c` emits ELF relocatable object; `-S` is not a compile pass | ELF magic/type checked; `-S` alone rejected for compile acceptance | Blocked C02/T11/H05 |
| T-H04-19 | `-c`/`-S` with unverified target, missing report, forged report | Exit 2; no artifact; no fallback | Blocked C02 |
| T-H04-20 | No C compiler invocation audit | Command log contains only candidate, pinned assembler/linker; no `.c` to GCC/Clang | Blocked Part B |

### 9.3 H05 acceptance matrix (hostile cases)

All tests are future controlled fixtures, not corpus cases and not run in this audit. They must run under H05 with short configured limits.

| ID | Scenario | Fixture idea | Expected result | Gate |
|---|---|---|---|---|
| T-H05-01 | Program hangs in run | `sleep 3600` child / busy loop | `TIMEOUT`; suite continues; elapsed bounded | H05 core |
| T-H05-02 | Compiler/tool hangs | Fake candidate/exe wrapper that sleeps | Phase `TIMEOUT`; no suite block; no artifact accepted | H05 core |
| T-H05-03 | Program self-crashes | `kill -SEGV $$` | `SIGNAL(SIGSEGV)`/RUN_CRASH; not timeout | H05 core |
| T-H05-04 | Program exits nonzero | `exit 42` | `EXIT(42)`; H02/H03 judge; not crash | H05 core |
| T-H05-05 | stdout flood | `yes` / unbounded write | No deadlock; bounded retained bytes; truncation/count recorded; timeout if needed | H05 core |
| T-H05-06 | stderr flood | Same on stderr | Same as T-H05-05; streams separate | H05 core |
| T-H05-07 | Duplicate basenames, concurrent | Two dirs both `same.c` | Separate instance dirs/outputs; no overwrite/cross-contamination | H05 core |
| T-H05-08 | Same basename within one multi-source instance | `a/same.c`, `b/same.c` | Explicit collision/error; no silent selection | Blocked H04 multi-source |
| T-H05-09 | Cancellation during run | Long sleep; send cancel | `CANCELLED`; active killed within grace; no new launches | H05 core |
| T-H05-10 | Cancellation during compile/link | Fake tool sleep | Same; no partial artifact accepted | H05 core |
| T-H05-11 | SIGTERM ignored | `trap '' TERM; sleep` | SIGKILL after grace; `TIMEOUT`/`CANCELLED`, not PASS | H05 core |
| T-H05-12 | Memory bomb | Allocate until failure | `RESOURCE_LIMIT`/signal classification; no host OOM | H05 limits |
| T-H05-13 | File-size bomb | Write huge file | FSIZE limit/SIGXFSZ; no disk fill | H05 limits |
| T-H05-14 | Fork/process bomb | Spawn many children | NPROC/limit or cleanup; no runaway; documented residual | H05 limits |
| T-H05-15 | Missing executable | Bad path | `SPAWN_ERROR`; no false crash attribution | H05 core |
| T-H05-16 | stdin read | Program reads stdin | Immediate EOF; no hang | H05 core |
| T-H05-17 | Env/cwd probe | Print env/cwd | No host secrets; cwd inside instance; pinned locale/TZ | H05 core |
| T-H05-18 | Symlink/path escape | Program writes `../../x` | Documented isolation boundary; harness never follows program-created symlinks for its own evidence | H05 security |
| T-H05-19 | Non-UTF-8 output | Write invalid bytes | Bytes hashed/escaped; no panic; no log corruption | H05 core |
| T-H05-20 | Control chars in filenames/output | Hostile names | Escaped in reports; no forged lines | H05 core |
| T-H05-21 | Disk full | Fill output filesystem | `INFRASTRUCTURE_ERROR`; no PASS; cleanup | H05 limits |
| T-H05-22 | Clock jump | Adjust system clock | Monotonic timeout unaffected | H05 core |
| T-H05-23 | Retry policy | Spawn failure then success; timeout | Infra retry bounded; timeout not retried; attempts retained | H05 evidence |
| T-H05-24 | Process-tree cleanup | Parent spawns child and exits/hangs | No orphan or documented kill policy | H05 core/security |
| T-H05-25 | Concurrent workers | N instances, mixed hangs | Other instances complete; no global CWD/env race | H05 core |
| T-H05-26 | Deterministic JSON | Three reruns | Canonical records identical; only non-canonical sidecar differs | H05 evidence |
| T-H05-27 | Assembler failure | Bad assembly / missing tool | `TOOL_CRASH`/LINK_FAIL; no object accepted; raw stderr retained | Blocked H01/H04 |
| T-H05-28 | Linker failure | Undefined symbol | LINK_FAIL; no executable; H06 mapping | Blocked H01/H03 |
| T-H05-29 | Target exit propagation | AArch64 ELF returns 5; qemu variant | Native: exit 5; qemu: exit 5 only if propagation verified | Blocked H01/Linux |
| T-H05-30 | Non-ELF executable artifact | Script named `a.out` | Rejected before execution; no shell execution | H05 security |
| T-H05-31 | Cancellation propagates | Suite cancel while queued | Queued instances never launched; active killed; partial evidence | H05 core |
| T-H05-32 | Same instance rerun | Previous attempt artifacts present | Fresh attempt dir; no stale artifact reuse; old evidence retained | H05 core |
| T-H05-33 | Output cap policy | Program writes exactly cap/cap+1 | Deterministic truncation/kill rule; no false PASS | H05 limits |
| T-H05-34 | Harness self-test | Fake PASS/skip/dropped record | Rejected; no denominator loss | H05/H06 |

### 9.4 Test commands (future; exact)

H04:

```sh
cargo fmt --manifest-path compiler/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path compiler/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path compiler/Cargo.toml --test h04_candidate
# proposed new files, if the integrator keeps them separate:
cargo test --locked --manifest-path compiler/Cargo.toml --test h04_flags_matrix
cargo test --locked --manifest-path compiler/Cargo.toml --test h04_sources
cargo test --locked --manifest-path compiler/Cargo.toml --test h04_argv
```

H05 (proposed new standalone package; see §10):

```sh
cargo fmt --manifest-path tools/torture/runner/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path tools/torture/runner/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path tools/torture/runner/Cargo.toml
# Linux-only signal/rlimit/process-group tests:
cargo test --locked --manifest-path tools/torture/runner/Cargo.toml --test isolation_linux
```

Do not run the entire repository suite for an H04/H05 package review; run the affected package and the dependent integration package.

### 9.5 Exit criteria

| Gate | Accepted only when |
|---|---|
| H04-A | Existing 18 H04 tests still pass and no existing behavior regresses |
| H04-B | New host-only tests for non-UTF-8 argv, exit/IO taxonomy, atomic output, evidence metadata, and quoting pass |
| H04-C | A frozen H04 flag/source contract exists (G-02) and does not contradict T00/H03 |
| H04-D | Every flag in the frozen H03 matrix is accepted with recorded semantics or explicitly `UNSUPPORTED`; no silent ignore |
| H04-E | `-I`/multi-source are either implemented per §5/§4.3 or remain explicitly refused; no partial half-support |
| H04-F | `-S`/`-c` remain fail-closed until G-05/G-06/G-07; after opening, `-c` produces a real ELF object and `-S` is not counted as compile success |
| H04-G | No GCC/Clang C compilation fallback; command logs show only candidate + pinned assembler/linker |
| H05-A | Per-instance directories and stable IDs prevent duplicate-basename collisions |
| H05-B | Hostile matrix (§9.3) passes for hang, crash, flood, limit, duplicate basename, cancellation |
| H05-C | Cancellation stops scheduling, kills active work within grace, and never creates a PASS |
| H05-D | Classification distinguishes timeout, signal, nonzero exit, spawn/tool failure, candidate diagnostic, ICE, and cancellation |
| H05-E | Target runner identity and exit propagation are recorded on the Linux substrate (or explicitly blocked) |
| H05-F | Canonical evidence is deterministic across three runs; failures are retained |
| H05-G | H02/H03/H06 consume the runner record without reinterpreting or hiding failures |
| H05-H | No fake PASS/skip/dropped case survives the H06 self-test |

## 10. Implementation Work Orders

Each order has one owner, exact paths, forbidden shared paths, and test commands. All orders are **proposed**; none is authorized by this report. For compactness, `cargo test ... --test <name>` in the tables below means `cargo test --locked --manifest-path <the order's package>/Cargo.toml --test <name>`; the fully expanded commands are in §9.4.

Several H04 orders name `compiler/src/bin/candidate.rs`; they must be serialized or partitioned into disjoint helper modules before parallel dispatch (one active writer per file). H05 orders that share `instance.rs` or `evidence.rs` follow the same rule. Do not run two writers against one file.

### 10.1 H04 work orders

| ID | Owner | Owned paths | Purpose | Gate | Test command |
|---|---|---|---|---|---|
| WO-H04-01 | H04 implementer | `compiler/src/bin/candidate.rs`; `compiler/src/bin/candidate/source_flags.rs` (new); `compiler/tests/h04_candidate.rs` or new `compiler/tests/h04_argv.rs` | Replace `std::env::args()` with OS-safe argv; never panic; define non-UTF-8 flag/path policy; fix lossy basename fallback | None (host-only defect fix; H04 contract review for policy) | `cargo test --locked --manifest-path compiler/Cargo.toml --test h04_argv`; `cargo test ... --test h04_candidate` |
| WO-H04-02 | H04 implementer | `compiler/src/bin/candidate.rs`; `compiler/src/bin/candidate/io.rs` (new); `compiler/tests/h04_candidate.rs` | Exit taxonomy (success/diagnostic/usage/unsupported/ICE); classify read/capacity errors; atomic temp+rename output; no partial artifact | H04 contract freeze for exit codes | `cargo test ... --test h04_candidate` |
| WO-H04-03 | H04 implementer | `compiler/src/bin/candidate/evidence.rs` (new); `compiler/src/bin/candidate.rs`; `compiler/tests/h04_candidate.rs` | Record effective dialect/opt level/options, target verification state, flag list/order, config hash, input hash; deterministic field order | None for metadata addition; H06 schema review | `cargo test ... --test h04_candidate` |
| WO-H04-04 | H04 + T01 integrator | `docs/tasks/H04_CANDIDATE_DRIVER.md` (current draft home) and any integrator-assigned H04 contract path | Freeze flag/source/quoting/exit contract, default dialect/options, `-o`/multi rules, `-E` limitations | H03 manifest + H07 reference probe + T01 review | Documentation/content review; no runtime gate |
| WO-H04-05 | H04 implementer | `compiler/src/bin/candidate.rs`; `compiler/src/bin/candidate/flags.rs` (new); `compiler/tests/h04_flags_matrix.rs` | Implement the frozen `-w`/`-g`/`-O*`/`-f*`/`-std` matrix: accept+record+semantics or explicit `UNSUPPORTED`; no silent no-op | WO-H04-04 + H03 manifest | `cargo test ... --test h04_flags_matrix` |
| WO-H04-06 | H04 implementer | `compiler/src/bin/candidate.rs`; `compiler/src/bin/candidate/flags.rs`; `compiler/tests/h04_sources.rs` | Quoting/source table tests: spaces, `=`, `--`, empty, leading dash, control chars, `@file`, `-o -`, duplicate basenames | WO-H04-04 | `cargo test ... --test h04_sources` |
| WO-H04-07 | H04 implementer | `compiler/src/bin/candidate.rs`; `compiler/src/bin/candidate/include_flags.rs` (new); `compiler/tests/h04_include.rs` (new) | `-I` parse/pin/ordering/escaping/search-path integration; install PP17/PP18 and fetch loop | H01 + T01 config + T03 PP17/18 update + T02 loop + WO-H04-04 | Blocked; future `cargo test ... --test h04_include` |
| WO-H04-08 | H04 implementer | `compiler/src/bin/candidate.rs`; `compiler/src/bin/candidate/multi_source.rs` (new); `compiler/tests/h04_sources.rs` | Multi-source TU orchestration, output naming, duplicate basename policy, `.S` path, link handoff | H03 link rule + WO-H04-04 | Blocked; future `cargo test ... --test h04_sources` |
| WO-H04-09 | H04 implementer | `compiler/src/bin/candidate.rs`; `compiler/src/bin/candidate/emit_helper.rs`; `compiler/tests/h04_candidate.rs` | `-E` atomic output, deterministic bytes, no line-marker claim; clash rules | None (host-only) | `cargo test ... --test h04_candidate` |
| WO-H04-10 | H04 implementer | `compiler/src/bin/candidate.rs`; `compiler/src/bin/candidate/target_gate.rs` (new); `compiler/tests/h04_candidate.rs` | `-S`/`-c` gate via `ensure_codegen_ready`; target report input; `-c` assembler boundary; no bypass | C02 attestation + T11 + H04 contract + H05 supervisor | Blocked; future `cargo test ... --test h04_target` |
| WO-H04-11 | T01 integrator/docs batch | `docs/tasks/H04_CANDIDATE_DRIVER.md`, `docs/tasks/README.md`, `docs/tasks/T00_GCC_TORTURE_GATE.md`, `docs/tasks/PARALLEL_EXECUTION.md`, `docs/tasks/M1_TARGET_ACCEPTANCE.md` | Fix version/test-count/status drift; do not change acceptance criteria | Integrator docs batch | Content/link/count check |

Forbidden for H04 work orders: `compiler/src/chips/**` (owned by T02–T13), `compiler/src/{bus,task,manifest,routing,snapshot,target,limits}.rs` (integrator), `compiler/contracts/CONTRACT_VERSION`, `compiler/Cargo.toml`, `compiler/src/mod.rs`/group `mod.rs`, `tools/torture/**`, `.github/workflows/ci.yml`.

### 10.2 H05 work orders

| ID | Owner | Owned paths | Purpose | Gate | Test command |
|---|---|---|---|---|---|
| WO-H05-01 | H05 + integrator | `tools/torture/runner/Cargo.toml`; `tools/torture/runner/src/lib.rs`; `tools/torture/runner/README.md` | Create standalone runner package skeleton; no dependency on compiler/H00 verifier; document taxonomy | Integrator approves new package/CI | `cargo test --locked --manifest-path tools/torture/runner/Cargo.toml` (after creation) |
| WO-H05-02 | H05 implementer | `tools/torture/runner/src/supervise.rs`; `tools/torture/runner/src/instance.rs`; `tools/torture/runner/tests/classification.rs` | Spawn with `Command` (no shell), per-instance cwd/env/stdin, bounded capture, process facts | WO-H05-01 | `cargo test ... --test classification` |
| WO-H05-03 | H05 implementer | `tools/torture/runner/src/timeout.rs`; `tools/torture/runner/src/cancel.rs`; `tools/torture/runner/tests/isolation_linux.rs` | Monotonic timeouts; grace SIGTERM/SIGKILL; cancellation token; no suite block | Approved kill/process-group decision | Linux: `cargo test ... --test isolation_linux` |
| WO-H05-04 | H05 implementer | `tools/torture/runner/src/limits.rs`; `tools/torture/runner/src/capture.rs`; `tools/torture/runner/tests/limits.rs` | Output caps/streaming/hash; memory/file/CPU/NPROC/core limits; fail closed if unavailable | Approved rlimit mechanism | `cargo test ... --test limits` |
| WO-H05-05 | H05 implementer | `tools/torture/runner/src/instance.rs`; `tools/torture/runner/src/evidence.rs`; `tools/torture/runner/tests/isolation_linux.rs` | Per-instance dirs, stable IDs, duplicate basenames, cleanup/retention | WO-H05-01 | `cargo test ... --test isolation_linux` |
| WO-H05-06 | H05 implementer | `tools/torture/runner/src/classify.rs`; `tools/torture/runner/src/evidence.rs`; `tools/torture/runner/tests/classification.rs` | Normalize exit/signal/timeout/limit/spawn/tool facts; no semantic PASS | WO-H05-02 | `cargo test ... --test classification` |
| WO-H05-07 | H05 implementer | `tools/torture/runner/src/runner.rs`; `tools/torture/runner/tests/concurrency.rs` | Bounded concurrent workers; deterministic result ordering; one hang does not block others | WO-H05-02/03 | `cargo test ... --test concurrency` |
| WO-H05-08 | H05 + H01 | `tools/torture/runner/src/toolchain.rs`; `tools/torture/runner/tests/toolchain_linux.rs` | Assembler/linker allowlist, exact argv, versions/hashes, no C compiler, timeout/limits | H01 pins + H03 link rule | Blocked; future Linux test |
| WO-H05-09 | H05 + H01 | `tools/torture/runner/src/target.rs`; `tools/torture/runner/tests/target_linux.rs` | ELF/architecture check; native/qemu runner identity; exit propagation | Linux AArch64 substrate | Blocked; future Linux test |
| WO-H05-10 | H05 + H06 | `tools/torture/runner/src/evidence.rs`; `tools/torture/runner/tests/replay.rs` | Canonical evidence schema, hashes, retention, three-run replay; H06 integration | H06 schema freeze | `cargo test ... --test replay` |
| WO-H05-11 | Integrator | `.github/workflows/ci.yml` | Add fmt/clippy/test steps for the new package; Linux job for signal/rlimit tests | WO-H05-01 | CI dry review; no source edit by H05 |
| WO-H05-12 | Security reviewer | `tools/torture/runner/src/**` review only | Approve process-group/rlimit/kill boundary; no unsafe without design | Security review | Review record |

H05 package location alternatives (integrator decision):

- **Preferred:** new standalone `tools/torture/runner/` package (above), so H00's `tools/torture/src/**` stays untouched and the H00 lock verifier's tests remain isolated.
- **Fallback:** `tools/torture/src/runner/**` modules, but that mixes H00 and H05 ownership and requires H00 owner sign-off. Do not modify the existing verifier modules.
- **Rejected for now:** putting the harness runner inside `compiler/src/bin/candidate` or making the compiler crate depend on a `tools/` package.

Forbidden for H05 work orders: all existing `tools/torture/src/**`, `tools/torture/tests/**`, `tools/torture/Cargo.toml`, `compiler/**`, `.github/workflows/ci.yml` (integrator only), and any corpus/test assets.

### 10.3 Dependency gates

| Gate | Meaning | Blocks |
|---|---|---|
| G-00 | Baseline `/39`; no source edits; current targeted H04 tests green | None |
| G-01 | H04 host-only defect fixes approved (argv, exit, atomicity) | WO-H04-01/02/03/06/09 |
| G-02 | H04 contract freeze (flags/sources/quoting/exit/default dialect) | WO-H04-04/05/07/08/10 |
| G-03 | H03 manifest freeze (exact flags, aux sources, link rule) | WO-H04-05/08, H02/H03 |
| G-04 | H00 corpus lock (not just scaffold) | H02/H03, any torture denominator |
| G-05 | H01 probe run + hashed report | C02, Part B, H05 target runner |
| G-06 | H07 ABI classification of the four unresolved fields | C02 attestation |
| G-07 | T01 integrator attestation (`TargetSpec::attest`) | `-S`/`-c`, Part B |
| G-08 | T01 config include fields + T03 PP17/PP18 policy update | `-I` (WO-H04-07) |
| G-09 | T02 fetch-and-retry/control loop | `-I` nesting/multi-level |
| G-10 | H05 runner core approved | `-c` subprocess boundary, H02/H03 run stage |
| G-11 | Linux AArch64 substrate + toolchain/sysroot pins | target execution, H05 target tests |
| G-12 | H06 evidence schema freeze | H05 replay/aggregation |
| G-13 | Security approval of rlimit/process-group boundary | H05 limits/kill, no-unsafe compliance |

## 11. Reproducibility and Evidence Retention

### 11.1 H04 reproducibility rules

| ID | Rule | Current | Required |
|---|---|---|---|
| R-H04-R1 | Same source/config/flags → byte-identical evidence | Existing for snapshot/trace/`-E` on M1 | Keep after flag wiring; include effective flags in evidence |
| R-H04-R2 | `-D`/`-U` order deterministic and recorded | Existing behavior; not textually recorded | Record seed list/order or hash |
| R-H04-R3 | Configuration visible in evidence | Only via snapshot hash | Add dialect/opt/options/target verification/config hash |
| R-H04-R4 | Input identity | Basename only; lossy fallback | Path/hash; no lossy fallback |
| R-H04-R5 | No host paths/timestamps in canonical fields | Input path is printed raw | Normalize/escape; timestamps excluded |
| R-H04-R6 | `-E` rerun byte-identical | Existing golden | Keep; atomic write does not change bytes |
| R-H04-R7 | No external preprocessor/compiler in end-to-end claim | Existing (candidate does not shell out) | Keep; audit H04/H05 commands |
| R-H04-R8 | Three-run stability | Existing two-run tests | Add three-run evidence for frozen commands |

M1 replay/determinism mapping (from [M1 target acceptance](../../tasks/M1_TARGET_ACCEPTANCE.md) §6):

| M1 rule | H04/H05 acceptance |
|---|---|
| D1 | Two full compiles from identical source/config/pins produce identical snapshot and trace hashes |
| D2 | At least three consecutive runs produce equal evidence hashes |
| D3 | Same IR/assembly input produces byte-identical assembly |
| D4 | Snapshot/evidence excludes addresses, host paths, and wall-clock data; includes contract/config identity |
| D5 | Stable scheduling/queue order; results consumed once; no address-derived ordering |
| D6 | ELF artifacts compare semantically (structure/symbols/exit status), not byte-identically |
| D7 | Two-trace comparison reports first divergence or none |
| D8 | Quota-1 equivalence under the canonical semantic comparison projection; no byte-identical claim for scheduler-only registers |

### 11.2 H05 evidence schema (proposed fields; names not frozen)

| Group | Fields | Canonical? |
|---|---|---|
| Identity | contract version, stable instance ID, attempt, suite, testcase, target identity; run ID only when the caller supplies a deterministic value | Yes |
| Configuration | dialect, options, aux sources, candidate argv, assembler/linker argv, runner argv, tool versions/hashes, env allowlist hash | Yes |
| Inputs | source hashes, manifest hash, corpus lock hash | Yes |
| Outputs | preprocessed/assembly/object/executable hashes, artifact type, size | Yes |
| Process facts | exit code, signal name/number, kill origin, timeout flag, limit flag, output byte counts, retained bytes, truncation flag, stdout/stderr hashes | Yes |
| Classification | H05 fact classification, H02/H03 judge result, H06 failure reason, exclusion reason | Yes |
| Non-canonical | wall-clock start/end, duration, host path, PID, runner host name, generated run nonce | No (sidecar only; excluded from canonical hash) |
| Retention | raw log paths relative to instance dir, retention policy, deletion reason | Yes (paths only) |

Rules:

- Canonical serialization is sorted, timestamp-free, and path-normalized.
- Full stdout/stderr are not retained if over cap; the full-stream hash and total count are retained, plus bounded head/tail or head bytes with an explicit truncation marker.
- Failure evidence is retained; success evidence may be summarized, but the retention policy must be recorded and must not delete failures to improve a rate.
- Partial artifacts are never hashed as complete; the record says `absent`/`partial`/`infrastructure_error`.
- No secrets, environment dumps, or host home paths in evidence; diagnostics escape control characters.
- Three consecutive runs with the same profile must produce identical canonical instance records (H09).
- Reruns use a fresh attempt directory; the prior attempt's evidence is immutable.

### 11.3 Retention vs denominator integrity

- A missing, cancelled, timed-out, crashed, unsupported, or infrastructure-error instance remains in the frozen denominator; only a complete assertion pass enters P.
- Empty denominators never count as a pass.
- Evidence deletion, log truncation, or retry must not remove the original failure from the ledger.
- H05 must expose raw facts to H06; H06 owns dual denominators and stratification.

## 12. Security and Ownership Risks

| Risk | Impact | Mitigation / owner |
|---|---|---|
| Non-UTF-8 argv panic | Denial of service; untrusted path triggers ICE | WO-H04-01; OS-string policy; no panic |
| Raw control characters in diagnostics/evidence | Log/report injection; forged PASS lines | Escape all dynamic text; H04/H05 tests; H00 F5 precedent |
| Path traversal in `-I`/`-o`/source | Reads/writes outside configured roots | Canonicalize; reject `..`/absolute escapes outside roots; H05 instance dirs; security review |
| Symlink/TOCTOU | Escape or wrong file read | Canonicalize then open once; document residual; no `O_NOFOLLOW` guarantee under current safe std |
| `@file`/response file | Unexpected file read or quoting confusion | Explicit unsupported; no implicit response file |
| Shell usage | Argument injection, glob expansion | Never use `sh -c`; argv arrays only |
| Tool substitution via `PATH` | Hostile assembler/linker/candidate | H01 absolute pins + hashes; H05 env allowlist; no PATH fallback |
| C compiler fallback | Fake candidate pass rate | Audit command logs; forbid `.c`/AST/IR to GCC/Clang; T00 gate |
| Process tree escape | Orphan/hang after timeout | Process group/session kill or approved supervisor; document residual |
| PID reuse in kill | Killing unrelated process | Signal only live `Child`; use process group; no stale PID files |
| Output flood/disk fill | Suite denial of service | Bounded capture; drain concurrently; FSIZE/quota; timeout |
| Fork bomb | Host resource exhaustion | NPROC/cgroup; test; fail closed |
| Secrets in env/logs | Credential leak | Clear env; allowlist; no env dump; escape logs |
| Partial artifacts | False evidence | Temp+rename; hash only complete files; classify partial as infrastructure |
| Duplicate basenames | Cross-instance overwrite | H05 instance dirs + DejaGnu `tmpdir` isolation; tests |
| H04 exit-code conflation | H06 misclassifies ICE as usage/unsupported | WO-H04-02; frozen taxonomy |
| H05 overclaim of isolation | Users believe programs are sandboxed | Explicit threat model: per-instance dir/process control, not a security sandbox |
| macOS signal semantics | Wrong classification | Platform guard; Linux-only acceptance for signals/rlimits |
| Unsafe-code ban vs rlimits | Cannot implement directly | Approved boundary or pinned external supervisor; no incidental unsafe |
| Ownership collision on `candidate.rs` | Overwrite user work | One H04 writer; no edits in this batch; integrator reviews |
| H05 package/CI collision | Broken shared workspace | New package + integrator-owned CI; fallback requires H00 sign-off |

## 13. Unrun Checks and Open Questions

### 13.1 Unrun checks

| Check | Status | Reason |
|---|---|---|
| Full compiler test suite | NOT RUN | Task scope; only targeted H04 test run |
| Root/torture test suites | NOT RUN | Task scope |
| `fmt`/`clippy` | NOT RUN in this audit | Documentation-only task; no source changes |
| Full H04 flag matrix tests | NOT RUN | Tests do not exist |
| Non-UTF-8 test in suite | NOT RUN as a test; manually reproduced panic | Defect F-01 |
| `-I` behavior | NOT RUN | Refused; no implementation |
| Multi-source behavior | NOT RUN | Refused; no implementation |
| `-S`/`-c` after attestation | NOT RUN | Target unverified; no codegen |
| External assembler/linker | NOT RUN | No Part B substrate/codegen |
| H01 probe | NOT RUN | Harness exists, never run |
| C02 attestation | NOT RUN | No report/hash |
| Torture corpus enumeration | NOT RUN | H00 lock scaffold only |
| H02/H03 official driver | NOT RUN | Planned only |
| H05 hostile fixtures | NOT RUN | H05 absent |
| Linux target execution | NOT RUN | No substrate |
| Three-run H05 replay | NOT RUN | H05 absent |

### 13.2 Open questions

| ID | Question | Owner | Blocking |
|---|---|---|---|
| Q-01 | Exact exit-code taxonomy (keep 0/1/2 or add unsupported/ICE/cancel codes) | H04/T01 | H04 acceptance, H06 mapping |
| Q-02 | Non-UTF-8 argv policy: byte-safe paths vs explicit rejection | H04/T01 | H04 driver security |
| Q-03 | `--`, `-`, `@file`, `-o -` semantics | H04 | Quoting matrix |
| Q-04 | `-D` separate-form consumption and function-like command-line macros | H04/T03 | Flag matrix |
| Q-05 | Default dialect and `-std` list from the reference probe | H07/T01 | Flag matrix, semantics |
| Q-06 | `-O`/`-g`/`-f*` acceptance: record-only vs observable semantics | H03/H04/T01 | Torture rows |
| Q-07 | `-c` orchestration: candidate assembles vs harness composes | H04/H01 | Compile gate |
| Q-08 | `-S`/`-c` output naming, `-o` multi-input rules, mode precedence | H04 | Driver contract |
| Q-09 | `-I` search order, quote/angle divergence, system roots | H01/T01/T03/H04 | `-I` implementation |
| Q-10 | `-I` sandbox/canonicalization/symlink escape rule | H01/H04 security | `-I` security |
| Q-11 | Source-name scheme for PP17 resolution / duplicate basenames | T03/H04 | `-I`, multi-source |
| Q-12 | `#pragma once` / header identity / once-set carrier | T01/T03 | `-I` correctness |
| Q-13 | Multi-source output naming, `.S`/`.i` handling, aux source order | H03/H04 | Multi-source |
| Q-14 | H05 process-group/rlimit mechanism under `forbid(unsafe_code)` | H05/security/T01 | Timeouts/limits |
| Q-15 | H05 isolation strength: is filesystem/network sandboxing required, or only per-instance dirs/process control? | H05/T00 owner | H05 acceptance/security |
| Q-16 | Output-cap policy: truncate-and-continue vs kill at cap | H05/H02 | Flood tests |
| Q-17 | Cancellation exit code and partial-evidence semantics | H05/H06 | Cancellation tests |
| Q-18 | H05 package location and CI ownership | Integrator/H05 | Work order dispatch |
| Q-19 | H06 evidence schema/field names | H06/T01 | H05 evidence |
| Q-20 | H03 exact flag manifest and whether `-w`/`-g`/`-f*` are accepted as hints | H03/H04 | Torture readiness |
| Q-21 | `-E` GCC-compatibility requirement (line markers/whitespace) | H03/H04 | Harness use |
| Q-22 | Link-only GCC/Clang driver allowed, or direct `ld` only? | H01/H03 | Link rule |
| Q-23 | Target report input spelling/ownership for `-S`/`-c` | T01/H04 | Part B gate |
| Q-24 | Retry budget and infrastructure-error definition | H05/H06 | H09 stability |
| Q-25 | Evidence retention duration and cleanup authority | H06/H05 | Evidence integrity |
| Q-26 | Signal-name normalization and emulator exit propagation | H05/H01 | Classification |
| Q-27 | Environment include variables: ignore vs fail closed | H04/H05 | `-I` determinism |
| Q-28 | Whether `-I` missing roots fail closed or are allowed empty | H04/T01 | `-I` UX/security |

## 14. Recommendation and Next Actions

1. **Do not open `-I`, multi-source, or `-S`/`-c`.** The current fail-closed behavior is correct; the freezes and probe evidence do not exist.
2. **Fix H04 host-only defects first** (WO-H04-01/02/03/06/09): non-UTF-8 panic, exit/IO taxonomy, atomic output, evidence metadata, quoting tests. These do not require target or include freezes.
3. **Freeze the H04 flag/source contract** only with H03's manifest and H07's reference-probe evidence; do not hand-write a matrix.
4. **Build H05 as a separate package** with a minimal process-supervision core, then hostile tests; do not put the runner in the candidate binary.
5. **Keep H05 target execution blocked** until H01/H07/T01 attestation and Linux AArch64 substrate; macOS tests may validate pure classification logic only.
6. **Preserve evidence and failures.** No timeout, crash, unsupported, or cancellation may become a PASS or disappear from the denominator.
7. **Integrator owns shared paths.** New package/CI/docs/contracts require the integrator; implementers touch only the exact paths in the work orders.

## 15. References

- [T00 GCC torture gate](../../tasks/T00_GCC_TORTURE_GATE.md) — H04/H05 acceptance, option matrix, failure classes.
- [M1 target acceptance](../../tasks/M1_TARGET_ACCEPTANCE.md) — P7/P8 gates, fail-closed §5, replay §6, evidence §9.
- [H04 candidate driver](../../tasks/H04_CANDIDATE_DRIVER.md) — current Part A + `-E`/`-D`/`-U` status; deferred rows.
- [H04 `-I`/multi-source draft](../../tasks/H04_I_FREEZE.md) — **DRAFT ONLY**, open search/quoting/once/orchestration proposal.
- [PP include slice](../../tasks/PP_INCLUDE_SLICE.md) — frozen `/25` committed-source include policy.
- [PP pragma slice](../../tasks/PP_PRAGMA_SLICE.md) — frozen `/29` `#pragma once` classification without a bus store.
- [T01 compiler contract](../../tasks/T01_COMPILER_CONTRACT.md) — §2 TU stores, §4 task protocol, §6 target model, §7.1 frozen envelope.
- [T03 preprocessing chips](../../tasks/T03_PREPROCESS_CHIPS.md) — PP17/PP18/PP25 rows.
- [PARALLEL_EXECUTION](../../tasks/PARALLEL_EXECUTION.md) — ownership and wave placement (status prose is stale for H04).
- [MASS_PARALLEL_DISPATCH](../../tasks/MASS_PARALLEL_DISPATCH.md) — Stage A task A-HST-01, gates, work-order template.
- `compiler/src/bin/candidate.rs` — current driver implementation.
- `compiler/src/bin/candidate/macro_flags.rs`, `emit_helper.rs` — current helper behavior.
- `compiler/tests/h04_candidate.rs` — 18 current tests.
- `compiler/src/target.rs` — `TargetSpec::attest`, `ensure_codegen_ready`, private `VerificationState`.
- `compiler/src/snapshot.rs` — config hash in snapshot.
- `compiler/src/limits.rs` — compiler-job limits (not H05 process limits).
- `tools/torture/README.md` — H00 verifier only; no runner.
- `tools/torture/probe/README.md` — H01 reference probe; never run; not a candidate runner.
- `docs/tasks/DISPATCH_LEDGER.md` — A-HST-01 assignment and report path.
- GCC 15.2.0 driver references (read-only, not vendored):
  - `https://raw.githubusercontent.com/gcc-mirror/gcc/releases/gcc-15.2.0/gcc/testsuite/lib/gcc-dg.exp`
  - `https://raw.githubusercontent.com/gcc-mirror/gcc/releases/gcc-15.2.0/gcc/testsuite/lib/c-torture.exp`
  - `https://raw.githubusercontent.com/gcc-mirror/gcc/releases/gcc-15.2.0/gcc/testsuite/lib/gcc.exp`
  - `https://raw.githubusercontent.com/gcc-mirror/gcc/releases/gcc-15.2.0/gcc/testsuite/gcc.c-torture/compile/compile.exp`
  - `https://raw.githubusercontent.com/gcc-mirror/gcc/releases/gcc-15.2.0/gcc/testsuite/gcc.c-torture/execute/execute.exp`
  - `https://raw.githubusercontent.com/gcc-mirror/gcc/releases/gcc-15.2.0/gcc/testsuite/gcc.c-torture/execute/ieee/ieee.exp`
  - `https://raw.githubusercontent.com/gcc-mirror/gcc/releases/gcc-15.2.0/gcc/testsuite/lib/torture-options.exp`
