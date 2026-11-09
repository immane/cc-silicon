# A24 Security, Sandboxing, and Secret-Handling Review

| Field | Value |
|---|---|
| Task | A-REV-03, Stage A under [MASS_PARALLEL_DISPATCH.md](../../tasks/MASS_PARALLEL_DISPATCH.md) |
| Scope | Security, sandboxing, and secret handling for compiler-development and Host tooling: include paths, untrusted source/test directives, subprocess/toolchain/runner isolation, timeouts and resource limits, output paths, diagnostics, environment/secrets, network/corpus acquisition, and CI trust boundaries |
| Status | **Read-only review.** No source, test, contract, Cargo, module, index, or other file was modified by this task. This report is the only file created by this task. No commit or push. No agent was started. |
| Date | 2026-11-07 |
| Baseline | `HEAD 430ae29`; `compiler/contracts/CONTRACT_VERSION` = `t01-c01-c06/39`, hash `07f4eaded698c13d6a85071159c0833edc9ee2f81702eeaf778f8db346f2fa24`; target `aarch64-unknown-linux-gnu`, `target_verification=unverified` |
| Working tree | Pre-existing modified/untracked parse/lex/freeze files were present; none were written, staged, or removed by this review |
| Method | Static inspection only: required documents, source, tests, probes, and workflows; targeted `rg`/`git status` checks. No `cargo` build/test, no external compiler, no download, and no untrusted program was run. |
| Authority | `AGENTS.md` §9, T00 H05, T01 §4.1, the H04 draft freeze, and the repository security rules. This report proposes controls; it does not authorize implementation, a freeze, or a version bump. |

**Evidence convention.** Each finding separates **Observed gap (static)** from **Required controls (recommendation)**. Line references are to the checked-out revision above. A finding marked “future” is not exploitable in the current tree because the relevant execution path does not exist; it is still a gate on the planned path. No claim is made that this review is exhaustive or that static lint is a security proof.

## 1. Executive Summary

No **Critical** finding was observed. The current compiler application has a small, mostly fail-closed Host surface: the candidate refuses `-S`/`-c` and `-I`, never invokes another C compiler, and has no network/subprocess/environment-variable-sampling code; the torture verifier is offline and never invokes GCC/DejaGnu. That is the good news.

The security posture changes sharply as soon as T00 H02/H05 execute corpus tests. **H05 is absent**, so there is no isolated execution, timeout, resource limit, cancellation, or crash classification for candidate-generated binaries or for the DejaGnu/Tcl test directives themselves. This is the highest-priority gate. The next largest clusters are Host boundary hardening in `candidate` (unbounded input read, arbitrary/clobbering `-o`, unescaped diagnostics and evidence paths), static-lint gaps in `tools/chip-lint` (including symlink traversal in CI), verifier input bounds, and CI supply-chain/trust boundaries.

Finding counts: **1 High, 9 Medium, 2 Low, 0 Critical.** A High finding is a gate on planned execution, not a claim that the current tree is remotely exploitable today.

Other Stage A dispatch reports are present in this directory. This review independently checked the source; where it overlaps, it is a security-acceptance companion, not a duplicate claim. In particular, `A10_HOST_H04_H05_PLAN.md` already records H05 absence, direct `fs::write` partial-output risk, raw diagnostic escaping, and the `forbid(unsafe_code)`/process-control tension; `A01_FRAMEWORK_BASELINE.md` records lint coverage limits. The integrator should deduplicate these work orders before dispatch.

**Recommended stop points:**

1. Do not begin H02/H05 suite execution until A24-SEC-01 is accepted and its isolation tests pass on Linux.
2. Do not open `-I`/multi-source/include loading until A24-SEC-07 freezes the escape rule and the source/aux-root rule.
3. Do not treat an H01 report hash or the caller-supplied reference-GCC hash as proof of provenance; they are integrity metadata only.
4. Keep the current fail-closed refusals (`-S`, `-c`, `-I`) while the target and include policies remain unverified.

## 2. Threat-Boundary Table

| ID | Boundary | Untrusted data or actor | Current enforcement (observed) | Evidence | Residual risk |
|---|---|---|---|---|---|
| TB-1 | C source/macro/directive text → semantic chips | Source bytes, macro replacement text, `#error` text, test directives | Chips are Host-passive in design; candidate imports source bytes; includes are explicitly `Unsupported` in PP05; in-process record/source limits exist | `compiler/src/bin/candidate.rs:576-600`, `compiler/src/limits.rs:87-105`, `compiler/src/bus.rs:1102-1120`, `compiler/src/chips/preprocess/pp_directive.rs:553-557` | No OS-level limits; diagnostic text is not escaped at the Host boundary (M-03); future include loading is open (M-08) |
| TB-2 | CLI arguments and file paths → `candidate` Host | Untrusted/awkward paths, `-D`/`-U` text, `-o` | Single-source whitelist; `-S`/`-c`/`-I` refused; `-D` names are ASCII identifiers; no shell | `candidate.rs:100-198`, `candidate.rs:901` | Unbounded `fs::read` (M-01), arbitrary `-o` (M-02), non-UTF-8 argv panic (L-01) |
| TB-3 | Candidate evidence/`-E` output → filesystem/stdout | Source text, paths, macro values, diagnostics | `fs::write` to a caller path; stdout otherwise | `candidate.rs:915-931`, `candidate.rs:959-966` | Clobber/symlink/partial write and raw-path/log injection (M-02, M-03) |
| TB-4 | Include search path → Host filesystem (future) | `-I`, sysroot, `#include` spellings, symlinks | `-I` refused; `#include` is `Unsupported`; H04 draft states “configured include roots only” | `candidate.rs:140-141`, `pp_directive.rs:553-557`, `docs/tasks/H04_I_FREEZE.md:44-47` | Exact escape rule, absolute headers, source/aux-root confinement, and tests are absent (M-08) |
| TB-5 | Lock + corpus tree + symlinks → verifier | Untrusted lock text, paths, tree entries, symlinks | Lexical path validation; canonical symlink-escape check; canonical duplicate detection; deterministic escaping; bounded traversal default | `tools/torture/src/path.rs:59-87`, `tools/torture/src/verify.rs:338-383`, `verify.rs:541-606`, `tools/torture/src/diagnostic.rs:109-126` | TOCTOU at open; unbounded lock read; root can be `/`; `--check-unlisted` skips symlinks (M-06) |
| TB-6 | Reference toolchain + probe → H01 evidence | Caller-supplied GCC path/version/hash, environment, output directory | Resolved path (absolute or PATH command); exact version and binary digest check; Linux/aarch64 guard; no network | `tools/torture/probe/run-probe.sh:69-125`, `tools/torture/probe/lib/guard.sh`, `tools/torture/probe/README.md:131-142` | Pin is self-asserted; GCC environment is inherited; metadata/report fields are not universally escaped (M-09) |
| TB-7 | Candidate/generated program + DejaGnu Tcl → execution substrate (future H05) | Compiled C, runtime file/network/process actions, `.exp/.x` Tcl scripts | **None.** H05 is absent; only in-process limits exist | `docs/tasks/T00_GCC_TORTURE_GATE.md:72,115-117`, `docs/tasks/PARALLEL_EXECUTION.md:9`, `docs/tasks/MASS_PARALLEL_DISPATCH.md:244-247`, `candidate.rs:323-336` | High until H05 lands: host privileges, unbounded wall time/memory/output, network, corpus writes, same-basename collisions |
| TB-8 | Network/corpus acquisition → host (future) | Remote archives, redirects, signatures, mirrors | None; only the fetch-on-demand/hash-lock policy and offline verifier exist | `docs/tasks/T00_GCC_TORTURE_GATE.md:67`, `tools/torture/README.md:49-59`, `docs/tasks/T00_H00_IMPLEMENTATION_STATUS.md:127` | No fetcher to review; future implementation must be gated (G-01) |
| TB-9 | PR branch + Actions + dispatch inputs → CI runner | PR source/tests, mutable action tags, workflow-dispatch inputs | `contents: read`; manual probe only; hard-coded `ubuntu-24.04-arm`; no `secrets.*` references | `.github/workflows/ci.yml:7-8,15-22,60-61`, `.github/workflows/t00-target-probes.yml:13-25,69-79` | Mutable action tags, PR code execution, no per-job timeout, persisted checkout credentials (M-07) |
| TB-10 | Chip source → static lint / CI gate | PR-controlled Rust syntax, modules, impls, symlinks | AST checks for direct `RestrictedChip`/Worker `compute` Host APIs, unsafe, macros, imports | `tools/chip-lint/src/lib.rs:109-210,312-439` | Module/inherent-helper/multi-segment call bypass and symlink traversal (M-04, M-05) |
| TB-11 | Developer environment/secrets → tools | `HOME`, `PATH`, toolchain env, CI token | No environment-variable sampling in `candidate` (only `std::env::args` for argv); verifier reads no environment variables and uses `args_os`; workflows reference no repository secrets (implicit token is read-only) | `tools/torture/src/main.rs:30`, `rg` absence of `env::var` in production Host code, workflow inspection | Future runner must sanitize env; no secret-scanning gate; test temp dirs are predictable (residual) |

## 3. Findings

### H-01 — No H05 execution isolation, timeout, or resource/cancellation boundary

**Severity:** High (gate). **Boundary:** TB-7. **Current exploitability:** none, because no candidate runner exists; the finding blocks the planned H02/H05 execution path.

**Observed gap (static).** T00 defines H05 as isolated execution with a separate directory per instance, timeouts, output/resource limits, cancellation, and crash classification, and states H05 is planned only. `PARALLEL_EXECUTION.md` records H05 as absent; `MASS_PARALLEL_DISPATCH.md` says isolation must precede large-scale execution. The tree contains no runner: `tools/torture/src/` has lock/verify/path/digest/diagnostic/CLI only. The candidate has only a 16-tick drain per step and in-process record budgets; it directly reads and writes the Host filesystem. No OS-level wall-clock deadline, memory cap, process cap, output cap, network denial, or kill path exists; the in-process budgets are not a substitute for an execution sandbox.

**Evidence:**
- `docs/tasks/T00_GCC_TORTURE_GATE.md:72` (H05 acceptance: “A malicious hang does not block the whole suite; identical basenames do not collide”) and `:115-117` (“H05–H10 are planned only”).
- `docs/tasks/PARALLEL_EXECUTION.md:9` (“H05’s isolated execution … is absent”).
- `docs/tasks/MASS_PARALLEL_DISPATCH.md:244-247` (H05 must precede large-scale execution).
- `compiler/src/bin/candidate.rs:58-59` (`DRAIN_BUDGET`), `:323-336` (tick budget only), `:576` (direct source read), `:915-931`/`:959-966` (direct output writes).
- `compiler/src/limits.rs:87-105` (process-local record limits, not OS limits).

**Impact.** Once H02/H05 runs the corpus, untrusted C test programs, generated binaries, and DejaGnu/Tcl `.exp/.x` directives would execute with the runner user’s filesystem, process, and network privileges. A hang, fork bomb, memory bomb, huge output, same-basename collision, or corpus write can block or corrupt the suite. Timeout/crash classification is also an acceptance-integrity issue: a sandbox kill must not become a pass or disappear from the denominator.

**Required controls (recommendation).**
1. H05 design/ADR before code: Linux substrate, per-instance directory under a private root, unique instance IDs, read-only corpus mount, private writable temp, sanitized environment (`PATH`, `HOME`, `TMPDIR`, `LD_*`, locale), no inherited file descriptors, no network, no shell (argv arrays and absolute tool paths for assembler/linker/runner), process-group kill, wall timeout, CPU/memory/file-size/process/output quotas, cancellation, and crash/timeout classification.
2. Run the entire harness, not only the candidate binary: DejaGnu/Tcl `.exp/.x`, auxiliary sources, assembler/linker, and generated programs must all be inside the isolation boundary. Treat test directives as executable code, not data.
3. Fail closed if the sandbox cannot be established (kernel features unavailable, macOS host, no Linux CI). Never fall back to unsandboxed execution.
4. H02/H03 must set the official driver’s `tmpdir` per instance as well as using H05 directories; per-instance directories alone do not prevent the driver from reusing a basename. (See the related A10 F-17 observation.)
5. Keep timeouts and sandbox kills in the T00 denominator; record the resource class and kill reason.

**Acceptance tests (proposed, to be implemented under H05):**
- A synthetic hang is killed within the configured deadline; the suite continues.
- A process/fork cap and a memory cap are enforced; exhaustion returns a classified timeout/resource failure, not a pass.
- A write outside the instance directory is denied; the corpus tree remains byte-identical.
- A network connect attempt is denied and classified.
- Two instances with the same basename do not collide and do not read each other’s output.
- An output-size quota is enforced without truncating a “pass” result.
- A synthetic, non-corpus `.x`/`.exp` fixture cannot `exec`, write outside the instance, or read outside the corpus.
- The same tests run on Linux CI; macOS-only results do not establish Linux isolation.

**Work order:** A24-SEC-01.

### M-01 — Candidate reads the whole file before the source-byte limit and accepts special files

**Severity:** Medium (High when the harness later accepts untrusted paths). **Boundary:** TB-2.

**Observed gap (static).** `candidate` calls `std::fs::read(input)` and only then passes the bytes to `alloc_source`, whose `ensure_source_bytes` check occurs after the allocation. There is no regular-file check, no metadata-size precheck, no bounded `Read::take`, and no wall-clock deadline. A multi-gigabyte file can exhaust memory before the 1 MiB fixture limit is consulted; a FIFO can block indefinitely; `/dev/zero` or another special/procfs source can stream or block without a finite-length check.

**Evidence:** `compiler/src/bin/candidate.rs:576`; `compiler/src/bus.rs:1102-1120` (`alloc_source` checks after receiving the `Vec`); `compiler/src/limits.rs:87-105` (`max_source_bytes = 1 << 20` is a bus limit, not a read limit). No `metadata`/`is_file`/`take` call is present.

**Impact.** A single hostile or accidental path can hang or OOM a developer/CI runner. When the source path comes from an untrusted manifest, this is a denial-of-service boundary failure.

**Required controls (recommendation).**
- Reject non-regular files (`metadata.is_file()` after a defined symlink policy).
- Check `metadata.len()` against the configured source limit before reading.
- Use `File::open` plus `Read::take(limit + 1)` and reject overflow, so procfs/FIFO/reporting-size-zero sources cannot evade the bound.
- Apply a Host wall-clock deadline around the whole pipeline; H05 adds the outer kill.
- Return a structured driver error (exit 2) or candidate diagnostic (exit 1), never a panic or partial evidence.

**Acceptance tests:** regular file just below/above the limit; FIFO; `/dev/zero` on Linux; a zero-size procfs path; a symlink to a regular file; a non-UTF-8 path. Assert bounded memory, a deadline, and a structured exit with no evidence.

**Work order:** A24-SEC-02.

### M-02 — Candidate `-o` allows clobbering, symlink following, input aliasing, and non-atomic writes

**Severity:** Medium. **Boundary:** TB-3.

**Observed gap (static).** Any `-o` path is accepted. `fs::write` truncates an existing file, follows a symlink, and can target the input source itself or a corpus/test file. There is no output-root confinement, no `input == output` check, no rejection of symlinks/special files, no temporary-file-plus-rename, and no explicit permission mode. A crash or ENOSPC can leave a partial evidence or `.i` file. The H04 draft says multi-TU `-E` must not clobber and H05 must use separate directories, but the single-TU driver does not implement that discipline.

**Evidence:** `compiler/src/bin/candidate.rs:110-115` (`-o` accepts any string), `:915-931` and `:959-966` (`fs::write`), `docs/tasks/H04_I_FREEZE.md:82` (“no clobbering; H05 separate-directory discipline”), `docs/tasks/H04_CANDIDATE_DRIVER.md:25` (`-o <path>` writes the evidence report).

**Impact.** A harness bug or malicious path can overwrite the source, another test instance’s output, or a corpus asset, corrupting acceptance evidence. A symlink in an attacker-controlled output directory can redirect the write. Partial files can be mistaken for successful runs.

**Required controls (recommendation).**
- Require output under a harness-supplied `--output-root` (or H05 instance directory); canonicalize the parent and reject escapes.
- Reject `input`/`output` alias after canonicalization; reject output symlinks and non-regular existing targets; define whether existing regular files are replaceable.
- Write to a uniquely named temporary file with `create_new`, flush, then rename atomically; do not leave a partial final path on failure.
- Use a private default mode (for example 0600/0640) or document the umask contract; H05 should provide the private directory.
- Reject directory paths and missing parents with a structured exit 2.

**Acceptance tests:** `-o input.c input.c` leaves the source byte-identical and exits 2; `-o` to an in-root symlink is rejected and its target unchanged; existing target policy is enforced; output outside the root is rejected; ENOSPC/crash simulation leaves no final partial file; same-basename instances write to distinct directories.

**Work order:** A24-SEC-02.

### M-03 — Candidate diagnostics and evidence are not control-escaped; absolute Host paths leak

**Severity:** Medium. **Boundary:** TB-1/TB-3.

**Observed gap (static).** The candidate prints raw dynamic strings: the input path in the evidence report, `#error` text from source tokens, and `std::io::Error` messages. A source containing `#error` followed by newlines, ANSI escapes, C0/C1 controls, or Unicode line separators can forge additional evidence/log lines or inject terminal control sequences. A newline in the input path has the same effect on the `input:` line. The report also exposes the caller’s absolute path, which is unnecessary host-layout leakage for a corpus-relative test identity. By contrast, the torture verifier already has a correct `escape_control` model.

**Evidence:**
- `compiler/src/bin/candidate.rs:576` (OS error includes `{input}`), `:909-910` (usage error), `:917-918`/`:961-963` (write errors), `:940` (`input: {}` raw), `:970` (diagnostic raw).
- `compiler/src/chips/preprocess/pp_diagnostic.rs:123-142` (message built from raw token spellings) and `:180-182` (emitted as the failure product).
- `compiler/src/chips/preprocess/pp_directive.rs:283-288` (embeds the child message in the parent failure).
- Positive comparison: `tools/torture/src/diagnostic.rs:109-126` (`escape_control`) and `tools/torture/src/verify.rs:103-138` (escaped report rendering).

**Impact.** CI logs and evidence files can be forged or made unreadable; terminal escape sequences can manipulate a developer’s terminal; absolute paths leak usernames/workstation layout into published evidence. The raw semantic message should remain available in the typed diagnostic record, but the Host rendering must be escaped.

**Required controls (recommendation).**
- Add one Host rendering function that escapes `\n`, `\r`, `\t`, C0/C1 controls, U+2028/U+2029, and optionally ANSI ESC; apply it to every dynamic field in stdout, stderr, and `-o` evidence.
- Render `input:` as a corpus-relative logical name where H03 provides one; otherwise escape the absolute path or replace known prefixes with placeholders. Do not print environment or unrelated paths.
- Cap the rendered message length and avoid unrestricted backtraces.
- Keep raw source text in the diagnostic record for tools; escape only at the presentation boundary.

**Acceptance tests:** source `#error` containing `\x1b[2J`, `\n`, `\r`, and `\u{2028}` produces a single escaped `candidate:` line and no ESC; a path containing `\n` cannot create a second `snapshot:`/`interpret:` line; `-D FOO=<control>` errors are escaped; the `-o` report has exactly one `input:` line and no raw absolute home path.

**Work order:** A24-SEC-02.

### M-04 — Chip-lint static boundary is bypassable through modules, helper impls, and multi-segment calls

**Severity:** Medium (defense-in-depth; not a runtime sandbox). **Boundary:** TB-10.

**Observed gap (static).** `lint_file` inspects only top-level `syntax.items`: `Item::Use` at top level, direct `RestrictedChip` impls, and inherent `impl XChip { fn compute }`. It does not recurse into `Item::Mod`, so a nested module can import `std::env`/`std::fs` undetected. Inherent methods other than `compute` are not linted, so `impl Chip { fn helper(&self) { std::env::var(...) } }` can be called from `compute`. In `visit_expr_call`, a multi-segment path such as `helper::leak()` has `get_ident() == None` and is treated as resolved, so the call is not flagged. A direct Worker `compute` with `std::env::var` is caught (good), but these compositions evade the same check.

**Evidence:** `tools/chip-lint/src/lib.rs:94-107` (recursive file collection), `:153-208` (top-level items only), `:312-336` (`None => true` for multi-segment call), `:425-439` (only the `compute` method of an inherent impl), `tools/chip-lint/tests/lint.rs:176-186` (direct Worker case only). T01 §4.1 requires the strict lint to block unknown/indirect calls rather than silently accepting them.

**Impact.** A chip could access the environment, filesystem, clock, randomness, or subprocesses while CI reports a clean strict scan, violating the purity/isolation claim. The chip-lint tool explicitly says it is not a formal proof, but CI currently treats a clean run as a gate.

**Required controls (recommendation).**
- Recurse through `Item::Mod` and lint nested imports/items; apply the same forbidden-path rules at every module level.
- Lint all inherent methods and all impl blocks, or build a call graph from `compute` and lint every reachable same-file function/method.
- Resolve `crate::`, `self::`, and `super::` calls to known same-file functions; reject unresolved multi-segment calls instead of treating them as resolved.
- Add negative fixtures for nested modules, helper impls, `self.helper()`, `crate::host::read()`, macros, and aliased imports.
- Keep the review requirement: lint is a mechanical aid, not proof.

**Acceptance tests:** each negative fixture produces a diagnostic; the current positive fixtures remain clean; `cargo test --manifest-path tools/chip-lint/Cargo.toml` passes; the CI scan command reports the nested-module and helper-impl fixtures.

**Work order:** A24-SEC-04.

### M-05 — Chip-lint follows symlinks and has no depth/visited bound in CI

**Severity:** Medium. **Boundary:** TB-10/TB-9.

**Observed gap (static).** `collect_rust_files` uses `path.is_dir()` (which follows symlinks) and recursively descends with no visited set, depth cap, file count, or total-byte bound. CI runs the scanner on the PR-controlled `compiler/src/chips` tree. A committed symlink can point outside the repository (reading unrelated `.rs` files), or create a loop that causes unbounded recursion/stack exhaustion; a symlink to a large tree can make the job scan until its (currently absent) timeout.

**Evidence:** `tools/chip-lint/src/lib.rs:94-107`; `.github/workflows/ci.yml:60-61` (scan command). The tool has no `symlink_metadata` call and no visited set.

**Impact.** A PR can cause a CI denial of service, scan files outside the repository, or leak path information through lint diagnostics. It does not execute the linked files, so the impact is bounded to the CI job and file-read oracle, not arbitrary code execution.

**Required controls (recommendation).**
- Use `symlink_metadata`; reject or skip symlinks, and record a diagnostic if a symlink appears under the chip tree.
- Canonicalize the scan root and require every visited path to remain under it; maintain a visited `(dev, ino)`/canonical-path set.
- Use an iterative walk with max depth, max files, and max total bytes; fail closed on exceeding a bound.
- Add a per-job CI timeout and run the scanner with a read-only checkout/least privilege.

**Acceptance tests:** symlink to a directory outside the root is rejected (or reported, never followed); symlink loop terminates with a bounded diagnostic; symlink to a `.rs` file is not read; a normal tree still scans; the max-entry limit test fails closed.

**Work order:** A24-SEC-04.

### M-06 — Torture verifier has no lock-size bound, accepts any root, and leaves some boundary errors unescaped

**Severity:** Medium. **Boundary:** TB-5.

**Observed gap (static).** The CLI and library read the whole lock with `read_to_string` and impose no maximum byte/line/field count; a crafted lock can cause memory exhaustion before structural validation. `resolve_root` accepts any existing directory and canonicalizes it; a root of `/` (or the repository/home) combined with an untrusted lock can make the verifier open and hash arbitrary readable files. On a digest mismatch the report prints the actual digest, creating an existence/hash oracle for files the operator’s root exposes. Operational errors in `main.rs` print the raw lock path. `--check-unlisted` deliberately skips symlinks, so an unlisted symlink entry is not reported at all. The TOCTOU between canonicalization and `File::open` is documented as a residual limitation.

**Evidence:** `tools/torture/src/main.rs:83-92` (unbounded `read_to_string`, raw error path), `tools/torture/src/verify.rs:204-216` (library equivalent), `:238-280` (any root accepted), `:338-383` (canonicalize then open), `:424-442` (actual digest printed), `:580-583` (symlinks skipped), `tools/torture/README.md:208-226` (documented trust/TOCTOU assumptions).

**Impact.** An untrusted lock can OOM the verifier or, with an overly broad root, read files outside any corpus. The CLI can be made to forge an operational-error line with a newline in the lock path. The hash oracle can confirm guesses about file contents. The tool already fails closed on structural errors and does not print file contents, so this is a bounded confidentiality/DoS risk, not arbitrary code execution.

**Required controls (recommendation).**
- Add `max_lock_bytes`, `max_lock_lines`, `max_fields_per_line`, and `max_assets`; reject non-regular lock files and bound the read before parsing.
- Require the corpus root to be an explicitly operator-approved corpus directory; reject `/`, the repository root, and the user home unless an explicit override is recorded in evidence. Alternatively require a root pin in the H00 lock/manifest.
- Escape every dynamic operational-error path with the existing `escape_control` function.
- Report symlink entries in `--check-unlisted` as `tree.symlink` (or a separate informational finding) rather than silently skipping them.
- Keep the actual digest for listed assets when needed for debugging, but document the oracle and consider reporting only a mismatch code plus expected digest on mismatch.
- Keep the quiescent-tree requirement and close the TOCTOU with an approved platform mechanism if one is authorized (see residual risks).

**Acceptance tests:** a 64 MiB sparse/one-line lock exits 2 within a bounded time/memory; a FIFO lock exits 2; a root of `/` is rejected (or requires an explicit override); a lock path containing `\n` produces one escaped error line; a tree containing a symlink produces a `tree.symlink` finding; a symlink to a file outside the root is rejected by asset verification.

**Work order:** A24-SEC-03.

### M-07 — CI actions are mutable tags; PR code runs with persisted checkout credentials and no job timeout

**Severity:** Medium. **Boundary:** TB-9.

**Observed gap (static).** `ci.yml` and `t00-target-probes.yml` use `actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, and `actions/upload-artifact@v4` — mutable major tags, not immutable commit SHAs. The probe workflow header explicitly records this as unresolved. CI runs `cargo test` and the chip-lint scan on pull-request code, with checkout’s default persisted credential, and neither workflow sets `timeout-minutes`. The root package’s `cargo clippy`/`cargo test` steps are not `--locked`; the other packages’ clippy/test steps are. There is no `cargo audit`/`cargo deny` or secret-scanning gate.

**Evidence:** `.github/workflows/ci.yml:15-22` (action tags), `:24-58` (PR code tests; no timeout/`--locked` on root), `:60-61` (PR-controlled lint path); `.github/workflows/t00-target-probes.yml:20-22` (tags documented as mutable), `:64`, `:79`, `:140-145`; `permissions: contents: read` is present at `ci.yml:7-8` and probe `:55-56` (positive control). No `secrets.*` reference was found in either workflow.

**Impact.** A compromised/mistakenly retagged action can execute arbitrary code with the workflow token and repository context. PR code can read the persisted checkout token (read-only) and run until the GitHub default timeout, consuming runner resources. A mutable dependency/toolchain resolution can make CI nondeterministic.

**Required controls (recommendation).**
- Pin every `uses:` action to a verified full 40-hex commit SHA, with the human-readable tag in a comment; document the verification method and update process.
- Set `persist-credentials: false` on checkouts in jobs that execute PR code.
- Add per-job `timeout-minutes`; add `--locked` consistently to root `fmt`/`clippy`/`test`.
- Add `cargo audit`/`cargo deny` and a secret scan as reviewed, pinned steps (or document why they are intentionally absent).
- Keep `contents: read`, avoid `pull_request_target`, and do not expose repository secrets to PR-triggered jobs.

**Acceptance tests:** a workflow check rejects any `uses:` value ending in a mutable `@vN` tag; `actionlint` (if adopted) passes; the pinned SHA table is reviewed and recorded; a synthetic job with an unbounded test is killed by `timeout-minutes`; `cargo audit`/`deny` runs on the locked graph.

**Work order:** A24-SEC-05.

### M-08 — Include-path escape policy and source/aux-root confinement are still open in the H04 draft

**Severity:** Medium (future design gate; currently fail-closed). **Boundary:** TB-4.

**Observed gap (static).** The current candidate refuses `-I`, and PP05 reports `#include`/`#include_next` as `Unsupported`, so there is no current filesystem include traversal. The H04 draft, however, leaves the exact escape-detection rule as an H01/H04 co-freeze item, and its orchestration step 4 loads command-line and H03-manifest `.c`/`.S`/auxiliary sources without stating that those paths must be canonicalized and confined to an authorized source root. The draft also leaves absolute header names, symlink policy, case/Unicode behavior, empty `-I ""`, `--`, `=` quoting, and `include_next` behavior open.

**Evidence:** `docs/tasks/H04_I_FREEZE.md:27-47` (search order; `:47` “Exact escape-detection rule is an H01/H04 co-freeze item”), `:79-83` (future substrate pin, source/aux load, include loop, `-E` no-clobber), `compiler/src/bin/candidate.rs:140-141` (current refusal), `compiler/src/chips/preprocess/pp_directive.rs:553-557` (include `Unsupported`).

**Impact.** If `-I`/host include loading is implemented without the frozen rule, a header spelling or `-I` entry can escape the pinned roots via `..`, an absolute path, a symlink, or a canonicalization mismatch, reading arbitrary host files into the compiler bus. A manifest-provided source/aux path can have the same effect even before includes are enabled if the loader is not confined.

**Required controls (recommendation).**
- Freeze, before code: the exact root set, canonicalization algorithm, absolute-name policy, `..`/symlink rule, case/Unicode rule, empty `-I`/`--`/quoting rules, include-depth/retry budget, and failure taxonomy.
- Pin all roots at startup; reject any candidate whose canonical target is not under a pinned root; do not use `..` normalization as a substitute for canonical containment.
- Apply the same confinement to primary and auxiliary `.c`/`.S` sources, or document an explicit operator-authorized exception.
- Keep the current `-I` refusal and `#include` Unsupported behavior until the freeze is accepted and tested.
- Record the searched root list/order in evidence without dumping file contents.

**Acceptance tests:** quote/angle order matrix; `-I` order and duplicates; `..` and absolute header names; symlink inside/outside the root; `-I ""`, spaces, `--`, `=`; case and Unicode spellings; macro-generated header name remains `Unsupported`; source/aux path outside the authorized root is rejected; include depth/retry exhaustion is a structured failure.

**Work order:** A24-SEC-07.

### M-09 — H01 reference-toolchain “pin” is caller-asserted and the probe inherits the caller environment

**Severity:** Medium (documented residual; integrity/trust issue, not a hidden code bug). **Boundary:** TB-6.

**Observed gap (static).** The manual probe workflow requires a caller-supplied GCC path, version, SHA-256, and a “toolchain digest” metadata string. The script computes the binary’s SHA-256 and compares it to the same caller-supplied value, so a dispatcher can provide any executable plus a matching hash. The workflow header correctly says the digest is recorded metadata, not verified provenance, and no SHA is fabricated. The probe also inherits the caller’s environment (`PATH`, `GCC_EXEC_PREFIX`, `COMPILER_PATH`, `LIBRARY_PATH`, `CPATH`, `LD_PRELOAD`, locale, etc.); it only forces `LC_ALL=C`. A malicious or contaminated environment can change GCC’s cc1/assembler/linker selection or preload code, so the report may not describe the pinned toolchain. A dispatch input is written into `toolchain-metadata.txt` with `%s` and no control-character validation; `run-probe.sh` writes the output directory and records several toolchain paths without a universal escaping/allowlist rule.

**Evidence:** `.github/workflows/t00-target-probes.yml:13-25` (trust-boundary notes), `:81-121` (caller inputs; probe runs `bash` from the selected branch), `:123-136` (metadata file), `tools/torture/probe/run-probe.sh:36-39` (`LC_ALL=C` only), `:69-125` (caller path/version/hash), `:127-137` (outdir, HOME/repo replacements), `:183-197` and `:224-243` (path fields), `tools/torture/probe/normalize/normalize.py:468-483` (outdir creation/writes), `tools/torture/probe/README.md:131-142` (metadata-only disclosure).

**Impact.** A compromised/unauthorized dispatcher or contaminated runner can produce a report that appears to be a pinned reference probe but is not. A newline in a dispatch input can forge lines in the uploaded metadata file. An outdir symlink can redirect probe outputs. A malicious environment can execute code via `LD_PRELOAD` or alter the toolchain path; the hash check then only proves self-consistency, not provenance. H01/H07 acceptance could inherit a false target attestation.

**Required controls (recommendation).**
- Verify the reference toolchain against a repository-owned allowlist or a separately published, reviewed pin, not only caller-supplied values; record cc1/as/ld paths and digests as well.
- Restrict `workflow_dispatch` to trusted maintainers/protected branches; keep the hard-coded runner label; treat the uploaded artifact as untrusted evidence until independently reviewed/attested.
- Run the probe under a sanitized environment (`env -i` plus an explicit allowlist) and verify `gcc -print-prog-name=cc1`/as/ld are under the pinned toolchain/sysroot.
- Reject control characters/newlines in `toolchain_digest` and path inputs, or escape them in the metadata/report; reject a symlink or non-private output directory.
- Keep the report hash as integrity evidence, not authenticity; H07/T01 attestation must document the provenance limit.

**Acceptance tests:** wrong version/hash fails before compilation; guard tests for a fake `gcc` earlier in `PATH`; a sentinel `GCC_EXEC_PREFIX`/`LD_PRELOAD` is ignored or the run fails; a newline in `toolchain_digest` produces a single escaped metadata line; a symlink `probe-report/run` is rejected; normalized report contains no absolute `HOME`/workstation paths or secrets; an allowed pinned toolchain passes.

**Work order:** A24-SEC-06.

### L-01 — Candidate uses `std::env::args()` and `String` paths, so non-UTF-8 arguments can panic

**Severity:** Low. **Boundary:** TB-2.

**Observed gap (static).** `std::env::args()` panics while iterating if any argument is not valid Unicode. The input and output paths are stored as `String`, so a non-UTF-8 path cannot be represented or opened, even though Unix pathnames are arbitrary bytes. `tools/torture` already uses `args_os`, which is the correct model.

**Evidence:** `compiler/src/bin/candidate.rs:64` (`input: String`), `:901` (`std::env::args()`); positive comparison `tools/torture/src/main.rs:30` (`args_os`).

**Impact.** A harness that passes a non-UTF-8 corpus path gets a panic/backtrace instead of a structured driver error, and loses the ability to process such files. This is a robustness and diagnostic-cleanliness issue, not a direct host compromise.

**Required controls (recommendation).**
- Parse with `args_os`; keep flags as UTF-8 where required, but store file paths as `PathBuf`/`OsString`.
- Return a structured driver error for a flag that is not UTF-8; never panic on a path argument.
- Test on Unix with an `OsStr::from_bytes` path containing invalid UTF-8.

**Acceptance tests:** invalid UTF-8 input path exits 2 with an escaped diagnostic and no panic; a valid non-UTF-8 path is read according to the defined policy; `-o` with a non-UTF-8 path follows the same policy.

**Work order:** A24-SEC-02.

### L-02 — `#![forbid(unsafe_code)]` covers the library targets but not the Host binaries

**Severity:** Low (defense-in-depth). **Boundary:** TB-2/TB-10.

**Observed gap (static).** The compiler library and torture library each have `#![forbid(unsafe_code)]`, but each package’s binary target is a separate crate. `compiler/src/bin/candidate.rs` and `tools/torture/src/main.rs` do not carry the attribute, and no package-level `[lints]` policy enforces it. The current binaries contain no `unsafe`, so this is a preventive gap. A future sandbox implementation may legitimately need a reviewed exception (seccomp/landlock/`O_NOFOLLOW` helpers), but that should be an explicit, documented boundary rather than an incidental edit.

**Evidence:** `compiler/src/lib.rs:16`, `tools/torture/src/lib.rs:1`; absence of the attribute in the two binary entry points. `AGENTS.md` §3 item 5 requires retaining `#![forbid(unsafe_code)]` at the framework boundary; this finding asks for an explicit policy for the Host binaries.

**Impact.** A future Host edit could introduce `unsafe` without a compile-time failure or an explicit review decision. No current runtime impact.

**Required controls (recommendation).**
- Add `#![forbid(unsafe_code)]` to the Host binaries or package-level lints, with a documented exception process for an approved sandbox helper.
- Keep any sandbox exception in a small, reviewed module with tests and a safety rationale; do not scatter `unsafe` through Host orchestration.

**Acceptance tests:** a compile-fail fixture containing `unsafe` in a Host binary is rejected; the approved sandbox exception, if any, is isolated and documented.

**Work order:** A24-SEC-02 (binary policy) / A24-SEC-01 (sandbox exception decision).

## 4. Current Positive Controls

These are observed as present; they reduce current exposure but do not replace the missing controls above.

- **Fail-closed target emission:** `-S`/`-c` are refused while the target is unverified (`candidate.rs:104-107`); `-I` is refused (`:140-141`); unknown flags are exit 2.
- **No compiler delegation:** the candidate never shells out to GCC/Clang; no `Command`/subprocess invocation exists in `compiler/src/bin` or `tools/torture/src`. (`std::process::exit` in the candidate is process termination, not subprocess execution.)
- **No network or environment-variable sampling in production Host code:** no `std::net`/HTTP client/`env::var` in the candidate or verifier; the torture verifier is offline (`tools/torture/README.md:49-59`).
- **Verifier path controls:** lexical rejection of absolute/`..`/backslash/colon/control/dot/empty components (`path.rs:59-87`); canonical symlink-escape check and canonical-duplicate detection (`verify.rs:338-383`); bounded, fail-closed enumeration (`:541-606`); deterministic control-character escaping in reports (`diagnostic.rs:109-126`, `verify.rs:103-138`).
- **CI least privilege:** `contents: read` on both workflows; no `secrets.*` references; probe is manual-only with a hard-coded GitHub-hosted runner label and no downloads.
- **Library unsafe policy:** both Rust libraries forbid unsafe code.
- **Chip lint baseline:** direct `RestrictedChip` and Worker `compute` Host API references, unsafe blocks, opaque macros, legacy bus-mutating chips, and bare module imports are rejected; the direct Worker case is tested.
- **No include exposure today:** PP05 reports include directives as `Unsupported`; no host include loader is wired.

### 4.1 Secret-Handling Assessment

**Observed.** A lexical scan found no hardcoded credential, private key, or `secrets.*` reference in the production source or workflows (this is a search, not a proof). The candidate does not call `std::env::var` or dump the environment; the verifier’s CLI uses `args_os` and its temp directories are test-only. CI jobs have `contents: read`; they reference no repository secrets, although the implicit workflow token is still a credential (M-07). These are good defaults.

**Residual and recommendation.** The H01 probe inherits its caller’s environment (M-09), the checkout action persists a token that PR-executed scripts can read (M-07), and `-D`/`-U` values plus `-E` output can carry whatever the caller places on the command line. There is no secret-scanning gate (R-09). Policy should state that secrets must never be passed via argv, `-D`, macros, evidence, or logs; H01/H05 should run with an explicit environment allowlist; and any future secret-bearing configuration needs an approved storage mechanism plus a scan/test. No evidence of an actual secret was found, so this is a policy/hardening gap rather than a disclosure.

## 5. Work Orders

These are proposed security work orders, not implementation authorizations; the `A24-SEC-*` and `G-01` labels are planning IDs, not `ChipId`/`TaskKind` values. Each is independently testable. Ownership and contract gates remain as in `PARALLEL_EXECUTION.md` and `MASS_PARALLEL_DISPATCH.md`; shared interfaces still require the T01 integrator. A24-SEC-01 overlaps the H04/H05 plan in `A10_HOST_H04_H05_PLAN.md`; A24-SEC-04 overlaps the lint follow-through in `A01_FRAMEWORK_BASELINE.md`. The integrator should merge or explicitly reject duplicates rather than dispatch both.

### A24-SEC-01 — H05 execution isolation and resource governance (P0)

**Owner:** H05 owner; Linux substrate/runner coordination with H01. **Paths (proposed):**
- `tools/torture/src/runner.rs` (new; isolation/process supervisor)
- `tools/torture/src/lib.rs` (module registration)
- `tools/torture/tests/runner.rs` (new; isolation acceptance)
- `.github/workflows/torture.yml` (new; Linux-only isolation job, if H05 owns a separate workflow)
- `docs/tasks/T00_GCC_TORTURE_GATE.md` (H05 acceptance/status update after implementation)

**Controls:** per-instance directory; per-instance official-driver `tmpdir`; read-only corpus; private writable temp; unique instance IDs; no network; sanitized environment; no shell (argv arrays and absolute tool paths); process-group kill; wall/CPU/memory/file/process/output quotas; cancellation; crash/timeout classification; Tcl/DejaGnu sandbox or safe interpreter; fail closed if isolation is unavailable. The design must state the mechanism (namespaces/landlock/seccomp/cgroups/rlimits or an approved equivalent) and the dependency/unsafe authorization before code.

**Acceptance:** the H-01 test list, run on Linux; `cargo test --manifest-path tools/torture/Cargo.toml --test runner`; `cargo clippy --locked --manifest-path tools/torture/Cargo.toml --all-targets -- -D warnings`. A macOS-only result is not acceptance.

**Gate:** blocks H02/H05 suite execution and any claim of an isolated torture run.

### A24-SEC-02 — Candidate Host I/O and diagnostic hardening (P1)

**Owner:** H04 owner; H05 coordinates the output-root/instance-directory contract. **Paths (proposed):**
- `compiler/src/bin/candidate.rs`
- `compiler/src/bin/candidate/host_io.rs` (new; bounded read, output policy, escaping)
- `compiler/tests/h04_candidate.rs` (new acceptance cases)

**Controls:** `args_os`/`PathBuf`; regular-file and size checks; bounded `Read::take`; structured errors; input/output alias rejection; output-root confinement; symlink rejection; atomic temp+rename; private mode; control-character escaping; logical/relative evidence names; unsafe policy for the binary. Coordinate the output-root field with the H05/H03 contract; until then, at minimum reject input aliasing, symlinks, and unbounded reads.

**Acceptance:** M-01, M-02, M-03, L-01 tests; `cargo test --manifest-path compiler/Cargo.toml --test h04_candidate`; the existing H04 golden/determinism tests remain byte-identical where no path is printed.

**Gate:** no output-root semantics that conflict with H04/I freeze; no contract version change without the integrator.

### A24-SEC-03 — Torture verifier boundary hardening (P1)

**Owner:** H00 verifier owner. **Paths (proposed):**
- `tools/torture/src/main.rs`
- `tools/torture/src/verify.rs`
- `tools/torture/src/lock.rs` (size/line/field limits, if parser-level)
- `tools/torture/tests/cli.rs`
- `tools/torture/tests/verify.rs`
- `tools/torture/README.md` (security model update)

**Controls:** max lock bytes/lines/fields/assets; regular-file check; bounded read; root allowlist/approval; escaped operational errors; symlink reporting in `--check-unlisted`; documented hash-oracle/TOCTOU residual; no content output.

**Acceptance:** M-06 tests; `cargo test --manifest-path tools/torture/Cargo.toml --test cli --test verify`; existing lock/path/symlink fixtures remain green.

### A24-SEC-04 — Chip-lint static boundary closure and symlink-safe traversal (P1)

**Owner:** chip-lint/CI tooling owner; compiler integrator reviews the false-positive impact. **Paths (proposed):**
- `tools/chip-lint/src/lib.rs`
- `tools/chip-lint/tests/lint.rs`

**Controls:** recursive module/impl/function lint; call-graph reachability from `compute`; unresolved multi-segment calls rejected; nested `use`/`mod` checks; symlink-safe iterative traversal with depth/file/byte bounds and a visited set; CI timeout.

**Acceptance:** M-04 and M-05 negative fixtures; `cargo test --manifest-path tools/chip-lint/Cargo.toml`; `cargo run --locked --manifest-path tools/chip-lint/Cargo.toml -- compiler/src/chips`; no symlink is followed.

### A24-SEC-05 — CI supply-chain and trust-boundary hardening (P1)

**Owner:** repository/CI owner; action-SHA changes require the same review as code. **Paths (proposed):**
- `.github/workflows/ci.yml`
- `.github/workflows/t00-target-probes.yml`
- `tools/ci/check-workflows.sh` (new; optional static pin/allowlist check)
- `tools/ci/tests/check-workflows.sh` (new; optional self-test)

**Controls:** full-SHA action pins; `persist-credentials: false`; per-job timeout; `--locked` consistently; cargo audit/deny and secret-scan decision; no `pull_request_target`; no secrets in PR jobs; document the GitHub-hosted runner trust boundary.

**Acceptance:** `rg -n 'uses:\s+\S+@v[0-9]' .github/workflows` returns no matches; `actionlint` (if adopted) passes; the pinned SHA table is reviewed; the audit/secret-scan decision is recorded; a synthetic long-running job hits its timeout.

### A24-SEC-06 — H01 probe trust, environment, and metadata hardening (P2)

**Owner:** H01 owner; T01 integrator/H07 consume the attestation. **Paths (proposed):**
- `tools/torture/probe/run-probe.sh`
- `tools/torture/probe/lib/guard.sh`
- `tools/torture/probe/tests/run-tests.sh`
- `tools/torture/probe/normalize/normalize.py`
- `tools/torture/probe/README.md`
- `.github/workflows/t00-target-probes.yml` (metadata input handling; coordinate with A24-SEC-05)

**Controls:** sanitized environment; repo-owned or independently published toolchain pin; cc1/as/ld verification; control-character rejection/escaping; output-dir symlink/private-dir checks; path/allowlist normalization; provenance limitation recorded.

**Acceptance:** M-09 tests; `bash tools/torture/probe/tests/run-tests.sh`; a wrong pin fails before compilation; a symlink outdir is rejected; metadata remains one line per field.

### A24-SEC-07 — Include-path and source-root security freeze (P2, design gate)

**Owner:** H04 include owner; H01 substrate pin owner; T01 integrator for config/contract fields. **Paths (proposed):**
- `docs/tasks/H04_I_FREEZE.md` (draft; resolve the open security rule)
- `docs/tasks/H04_CANDIDATE_DRIVER.md` (future acceptance reference)
- `compiler/tests/h04_candidate.rs` (future include/source-root matrix, only after freeze)

**Controls:** exact canonical root policy; absolute/`..`/symlink/Unicode/case rules; source/aux confinement; include depth/retry budget; loud typed failures; no environment-derived defaults; no fallback preprocessor.

**Acceptance:** the M-08 policy matrix is written and reviewed by H01/T01/H04; no `-I` code lands before acceptance; tests cover every matrix row and the current `-I` refusal remains while unfrozen.

### G-01 — H00 corpus acquisition security gate (future, design-only)

**Owner:** H00 owner; legal/provenance review as required. **Paths (proposed):**
- `docs/tasks/T00_H00_IMPLEMENTATION_STATUS.md`
- `tools/torture/README.md`
- `tools/torture/src/fetch.rs` (new only after authorization)
- `tools/torture/tests/fetch.rs` (new only after authorization)

**Gate requirements:** HTTPS with certificate validation; approved host/redirect allowlist; size cap; SHA-256/512 verification before use (published signature verification as a separate reviewed step); no credentials in URLs; temp file + atomic rename; no symlink following; provenance/license recording; offline mode; no vendoring into the MIT tree. No fetcher exists today; this gate prevents an unreviewed downloader from becoming the first network boundary.

## 6. Residual Risks

| ID | Residual risk | Treatment |
|---|---|---|
| R-01 | Verifier TOCTOU between canonicalization and `File::open`; safe `std` has no `O_NOFOLLOW`/`openat2` equivalent | Documented in `tools/torture/README.md:208-226`. Require a quiescent tree and a private/verified corpus root; if a stronger guarantee is required, approve a small `cap-std`/`rustix` boundary in a separate ADR. |
| R-02 | H01 reference-GCC pin is self-asserted; the report hash proves integrity, not authenticity or provenance | Keep the “metadata, not proof” wording; add a repo-owned pin/allowlist and independent attestation review. |
| R-03 | GitHub-hosted runner image and declared `toolchain_digest` cannot be verified by the workflow | Recorded metadata only; do not treat it as a frozen toolchain. Pin actions by SHA and use protected dispatch. |
| R-04 | Static lint is not a proof; indirect dependencies, macros, aliases, or future code can hide Host access | Retain code review, the Host/chip crate boundary, and negative tests; do not claim formal purity. |
| R-05 | H05 isolation mechanisms may be unavailable on macOS or unprivileged CI; a fallback would be unsafe | Fail closed; Linux-only evidence; do not run the suite unsandboxed. |
| R-06 | Corpus `.exp/.x` directives are executable Tcl; hash locking does not make them safe | H02/H05 must sandbox the harness/Tcl and treat the corpus as untrusted code. |
| R-07 | Evidence artifacts are not signed; a compromised Host can forge a report and matching hash | Review/protected artifacts and independent provenance; artifact signing is a separate decision. |
| R-08 | macOS/Linux path semantics differ (case folding, Unicode normalization, symlink behavior) | Validate isolation/path tests on the Linux substrate; do not infer Linux behavior from macOS. |
| R-09 | No secret-scanning gate is present; a future Host/runner could add credential-bearing env or logs | Add a reviewed secret scan and an env allowlist when secret-bearing configuration is introduced. |
| R-10 | Test temp directories are predictable or only weakly unique (`temp_dir` + PID/counter/nanotime) and test-only | Use private/random creation for new multi-user tests; not a production boundary, but a local test race exists. |
| R-11 | No network/corpus fetcher exists; the fetch policy is metadata only | G-01 must be accepted before any downloader code. |

## 7. Open Questions

1. **Sandbox authorization.** Which Linux mechanism (namespaces, landlock, seccomp, cgroups/rlimits) and which dependency (`rustix`, `nix`, `libc`, external helper) is approved? `#![forbid(unsafe_code)]` and the no-unsafe policy need an explicit exception process if a helper requires it.
2. **Tcl scope.** Does H05 sandbox the entire DejaGnu/Tcl harness, or only the candidate/generated binary? Who owns the H02 wrapper’s trust boundary and safe-interpreter policy?
3. **Path authority.** Should `candidate` accept arbitrary absolute input/output paths, or only paths under a harness-provided source/output root? The H04 draft currently constrains includes but not primary/aux sources.
4. **Include policy.** What are the frozen answers for absolute header names, symlinks inside a root, case/Unicode normalization, `-I ""`, `--`, `=` in paths, duplicate roots, and `include_next`?
5. **Verifier root.** Should the verifier reject `/`, the repository root, and `$HOME` unconditionally, or require an operator-approved corpus root from the H00 lock/manifest?
6. **Attestation.** What independent evidence—beyond a caller-supplied GCC path/hash and an uploaded report—will the T01 integrator require before `TargetSpec::attest` or H01 acceptance?
7. **CI policy.** Is `cargo audit`/`cargo deny`/secret scanning approved in CI, and who owns the pinned-action/SHA update process?
8. **Evidence naming.** Will H06 use corpus-relative logical names and path placeholders, or permit absolute Host paths in published evidence? This affects M-03 and H01 normalization.
9. **Untrusted code classification.** Are `.S`, auxiliary sources, `.x`/`.exp`, and test runtime data all formally classified as untrusted executable/code inputs? The answer determines the H02/H05 sandbox scope.
10. **Secret handling.** Are `-D`/`-U` values or argv ever allowed to carry secrets? If so, they must not reach `-E`, evidence, logs, or snapshots; a policy/test is needed.
11. **Host crate boundary.** T01 §4.1 calls for a separate Host crate with I/O permissions; should `candidate` remain a binary target or move behind a capability-scoped Host module before `-I`/link/runner work grows?
12. **Provenance of test assets.** How are GCC test-suite licenses, signatures, and mirror provenance recorded and reviewed independently of the hash lock?

## 8. Commands and Evidence Used

Read-only checks only:

```sh
git status --short
git rev-parse --short HEAD
cat compiler/contracts/CONTRACT_VERSION
rg -n --glob '!**/target/**' 'std::process::Command|Command::new|fs::write|fs::read|env::var|canonicalize|read_to_string|remove_file|remove_dir_all|create_dir|OpenOptions' .
rg -n --glob '!**/target/**' 'forbid\(unsafe_code\)|unsafe' compiler/src tools/torture/src tools/chip-lint/src
rg -n -i 'secrets|token|password|credential|private.key|api.key' .github/workflows tools/torture compiler/src/bin
rg -n 'std::env|env::|Command::|std::fs|std::net|Tcp|Udp|reqwest|curl|wget' compiler/src tools/torture/src
```

No `cargo fmt`, `clippy`, `test`, compiler invocation, download, or untrusted program execution was performed. Therefore this review reports **static observations**, not runtime security evidence; the acceptance tests above are proposed and unrun.

## 9. Revision Record

| Date | Change | Author |
|---|---|---|
| 2026-11-07 | Initial A-REV-03 security review: threat boundaries, 12 findings (1 High, 9 Medium, 2 Low), control/test matrix, 7 work orders plus one future acquisition gate, residuals, and open questions. Read-only; no source/test/contract change, no commit, no test run. | A-REV-03 subagent |
