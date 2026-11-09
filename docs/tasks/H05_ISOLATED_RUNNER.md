# H05 — Per-Instance Isolated Process Runner

| Field | Value |
|---|---|
| Status | **IMPLEMENTED subset; release-blocking audit findings (2026-11-10)** — host tooling; no compiler/contract change |
| Crate | `tools/runner` (`cc-silicon-runner`) |
| Scope | Per-instance workdir, deterministic env, timeout, output caps, cancellation, Linux prlimit bounds, process-group termination, outcome classification |
| Non-claims | No target compiler/codegen, no torture corpus/rate, no kernel-level sandbox |

### 1A. 2026-11-10 read-only audit findings

These findings were reported by a read-only audit and have not been fixed or
independently re-run as a release gate. They block treating H05 as a T00 native
acceptance boundary, though the compiler's local smoke tests do not depend on
the runner.

- The CLI prints the mapped runner exit code, not the child's exit code or
  signal; `Signaled{0}` can be fabricated when status is unavailable.
- A fast process that exceeds the output cap can be classified `completed`;
  output exactly equal to the cap can be falsely marked truncated; the two
  streams race on one flag.
- Post-kill `wait()` and reader-thread joins are unbounded; elapsed time is
  captured before joins; descendants can keep pipes open.
- Requested `prlimit` is silently ignored when `setsid` is unavailable; tool
  presence is checked by spawn, not successful execution.
- Log writes follow symlinks; `PATH` is host-dependent; environment allowlisting
  is incomplete for reproducible target runs.
- H05 is not a filesystem/network sandbox and does not provide cgroups,
  filesystem quotas, core suppression, or protection from daemonizing
  descendants.

Small file-exclusive follow-up orders are H05-R1 (`lib.rs` containment),
H05-R2 (`main.rs` report), H05-R3 (`runner.rs` regressions), and H05-R4 (new CLI
tests). H05-R1 and H05-R5 both touch `lib.rs` and must not run concurrently.
No compiler/tools dependency is implied; H02/H03 own judging and H06 owns
aggregation.

## 1. Purpose

H05 is the host boundary for executing an untrusted candidate/compiler command
inside one test-instance directory. It is deliberately separate from the
semantic core and from the T00 corpus verifier. It does not execute candidate C
by itself; it supervises a command supplied by the future H02/H03 runner.

## 2. CLI

```text
cc-silicon-runner --workdir <dir> [options] -- <program> [args...]

--timeout-ms <n>          wall-clock timeout (default 10000)
--stdout-limit <n>        stdout byte cap, 0 = unlimited (default 1048576)
--stderr-limit <n>        stderr byte cap, 0 = unlimited (default 1048576)
--cancel-file <path>      terminate when this marker appears
--rlimit-as <bytes>       RLIMIT_AS (Linux prlimit)
--rlimit-cpu <seconds>    RLIMIT_CPU
--rlimit-nofile <n>       RLIMIT_NOFILE
--rlimit-nproc <n>        RLIMIT_NPROC
```

The command is executed directly with `std::process::Command`; no shell is
injected by the runner. The environment is cleared to:

```text
PATH (host PATH), LC_ALL=C, LANG=C, HOME=<workdir>, TMPDIR=<workdir>/tmp
```

The runner creates `<workdir>` and `<workdir>/tmp` when absent. It never deletes
files in the workdir; `stdout.log` and `stderr.log` are overwritten.

## 3. Isolation behavior

- **Linux process group:** when `setsid` is available, the command is started in
  a new session and termination signals target the process group. This stops
  descendants as well as the direct child.
- **Resource bounds:** when any rlimit is requested, Linux `prlimit` must be
  available; otherwise the run fails as a setup error. Missing `prlimit` never
  degrades to an unbounded run.
- **Output bounds:** stdout/stderr are read by dedicated threads and capped at
  the requested byte count. Exceeding a cap triggers termination and an
  `output-limit` outcome; the log contains at most the cap.
- **Timeout/cancellation:** the supervisor polls the child, the wall clock, and
  the optional cancellation marker; termination is followed by a bounded wait.
- **Determinism:** no clock/environment values are mixed into the report except
  `elapsed_ms`; exit/outcome classification is from the process status and the
  explicit stop reason.

## 4. Outcomes and exit codes

| Outcome | Runner exit |
|---|---:|
| completed with code 0 | 0 |
| completed with nonzero code | 1 |
| setup/usage error | 2 |
| timeout | 3 |
| cancelled | 4 |
| output limit | 5 |
| terminated by signal | 6 |

Reports are stable `key=value` lines: outcome, exit code, elapsed ms, workdir,
log paths, and truncation flags.

## 5. Acceptance

macOS development host (8 tests; Linux-only tests skipped):

```sh
cargo test --manifest-path tools/runner/Cargo.toml
```

Linux aarch64 container (10 tests, including `prlimit` and process-group
termination):

```sh
docker run --rm --platform linux/arm64 \
  -v "$PWD/tools/runner":/src -w /src \
  -e CARGO_TARGET_DIR=/tmp/target \
  rust:1.85-bookworm /usr/local/cargo/bin/cargo test
```

Coverage: completion/logs, nonzero exit, timeout, output cap/truncation,
cancellation marker, signal classification, missing program, invalid workdir,
deterministic environment, Linux prlimit visibility, and process-group cleanup.

## 6. Explicit limitations

- No kernel namespace/container/seccomp isolation; H05 is a process supervisor,
  not a security boundary against a malicious kernel exploit.
- `setsid`/`prlimit` are external Linux tools. On non-Linux hosts the runner
  falls back to direct child killing and rejects requested rlimits.
- No filesystem quota; output caps apply to captured streams, not arbitrary
  artifact writes. A later sandbox/profile may add filesystem quotas.
- No target compiler or candidate execution is provided; H02/H03/H10 still own
  corpus/instance orchestration and aggregation.
- No torture pass rate may be claimed from H05 alone.
