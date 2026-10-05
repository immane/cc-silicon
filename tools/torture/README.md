# cc-silicon-torture

Offline corpus-lock and provenance verifier for the T00/H00 GCC torture gate.

> **Status: H00 tooling scaffold only.** This is not H00 completion. No authorized
> frozen corpus lock exists yet, so the tool reports a draft/unfrozen lock and
> withholds the H00 verdict. See
> [`docs/tasks/T00_H00_IMPLEMENTATION_STATUS.md`](../../docs/tasks/T00_H00_IMPLEMENTATION_STATUS.md).

This crate is a **standalone host tool**. It is not part of the `cc-silicon`
framework, does not depend on it, and never links the semantic core.

## What it verifies

Given an explicitly supplied local lock and asset tree, it:

- parses the lock (`schema=cc-silicon.torture/lock/v1`);
- validates required metadata: GCC release and full commit SHA, source-archive
  path and digest, profile dialect and option matrix, target identity, execution
  substrate, corpus policy, and reference-baseline policy;
- requires a frozen lock to declare each T00 baseline suite separately
  (`compile`, `execute`, `ieee`);
- requires an explicit cross-linked `asset role="archive"` record for the source
  archive (see below);
- validates asset records: role, corpus-relative path, SHA-256, original license,
  and provenance, plus optional size;
- rejects malformed identifiers (commit SHA must be 40 lowercase hex), malformed
  digests (64 lowercase hex), unknown records/fields/roles, duplicate fields, and
  duplicate paths — nothing is silently ignored;
- rejects unsafe relative paths: absolute paths, `..` traversal, backslashes,
  `:` scheme/drive prefixes, control characters, `.`/empty components, and
  trailing separators;
- rejects symlinks whose canonical target escapes the supplied corpus root;
- compares presence, size, and SHA-256 of every listed file;
- emits deterministic, sorted, timestamp-free diagnostics and a stable verdict.

- rejects two different asset paths that resolve to the same canonical in-root
  file (a symlink alias would otherwise be counted and verified twice):
  `asset.canonical-duplicate`.

With `--check-unlisted` it additionally reports files present in the tree that the
lock does not list. Enumeration is **fail-closed**: any unreadable directory or
entry produces a `tree.enumeration-incomplete` error instead of a silently
partial result, traversal is iterative rather than recursive, and a run is
bounded by `--max-unlisted-entries` (default 1,000,000; `0` disables the bound).
Exceeding the bound stops the walk and errors with `tree.traversal-limit`, never a
clean partial result.

## What it does not do

- It never downloads, clones, or vendors corpus assets.
- It never runs GCC, Clang, DejaGnu, or any candidate test.
- It never produces a pass rate or a compile/execute/ieee result.
- **It cannot verify legal truth.** It checks that license and provenance records
  exist and are non-empty; it cannot confirm the recorded license is correct or
  that redistribution is lawful. Human legal review remains required before any
  freeze.
- A successful run proves only that the local tree matches the supplied lock. It
  does not establish H00 completion or compiler correctness.

## Usage

```sh
cargo run --manifest-path tools/torture/Cargo.toml -- <LOCK> <CORPUS_ROOT> \
    [--check-unlisted] [--max-unlisted-entries <N>]
```

Exit codes:

| Code | Meaning |
|---:|---|
| 0 | frozen lock verified (no errors) |
| 1 | verification failed (errors present) |
| 2 | usage or operational error (unreadable lock file) |
| 3 | lock is structurally valid but draft/unfrozen; H00 cannot be claimed |

Exit code 3 is deliberate: an unfrozen or empty lock must never look like success.

## Encoded corpus policy

These are metadata expectations, not implemented guarantees:

- **Acquisition:** `fetch-on-demand`; assets are pinned by SHA-256 and **must not
  be vendored** into the repository (`policy acquisition=... vendoring=prohibited
  hash-lock=sha256`).
- **Reference baseline:** a reference-only DejaGnu baseline is authorized before
  candidate implementation, labeled `non-candidate`, and its
  `used-for-candidate-pass-rate` flag must stay `false`.
- **Target identity (frozen):** `aarch64-unknown-linux-gnu`, ELF, LP64,
  little-endian, AAPCS64.
- **Execution substrate:** `linux-ci-or-vm`. The concrete runner, toolchain,
  sysroot, assembler, and linker are **not established yet**. They remain explicit
  unfilled inputs, so any honest lock is still `draft`.

### F1: suites, dialect, and options are explicit

A frozen lock must list `compile`, `execute`, and `ieee` as separate
`profile.suite` values. The current schema is a single profile with a suite list;
T00 does not define a per-suite lock-profile decomposition, so omitting one would
silently shrink the accepted scope. If T00 later defines per-suite profiles, this
rule should be revisited and relaxed to a per-profile requirement.

The tool never assumes a default dialect or option matrix. `profile.dialect` must
be present, and `profile.configuration` must be an explicitly listed matrix: an
absent configuration list is a **warning** for a draft lock and an **error** for a
frozen lock (`profile.configuration.missing`). T00 must still verify the exact
dialect and option semantics against the reference release driver; the lock
records those decisions, it does not derive them.

### F2: license, provenance, and the source archive

Every asset, including the source archive, must carry an explicit `license` and
`provenance`. Provenance should embed an immutable revision (commit SHA or tag
digest); provenance without a revision-like token produces a
`provenance.revision-unresolved` **warning** to prompt strong linkage. This is a
policy nudge, not a legal determination.

The source archive is itself a corpus asset. `release.source-archive` must also
appear as an `asset` record with `role="archive"`, matching path and matching
digest. This keeps the archive inside the per-asset license/provenance checks and
the duplicate-path check, and ensures it is counted and verified exactly once.
Non-archive assets claiming the release-archive path are rejected as collisions.

### Published vs fetched digests and detached signatures

Official-source facts and locally fetched assets are kept distinct:

- `release.commit-sha`, `release.release-date`, `release.gcc-release`,
  `release.source-archive-sha512`, and `release.provenance` are **recorded
  official facts**. For GCC 15.2, `release.source-archive-sha512` is the value
  published in the release `sha512.sum`. The tool never contacts the network, so
  these facts are metadata and are **not** network-verified.
- `release.source-archive-sha256` and every asset `sha256`/`sha512` are the
  **locally computed or expected** values checked against the supplied tree. GCC
  15.2 publishes no SHA-256, so the recorded SHA-256 is a locally established
  fetch expectation, not an official publication.
- A frozen lock must record `release.source-archive-sha512` (GCC 15.2 publishes
  it in `sha512.sum`) and the cross-linked archive asset must record a matching
  `sha512` (`archive.sha512-missing` / `archive.sha512-mismatch`). A draft lock
  may leave it unfilled and receives a warning.
- `release.provenance` records the official source location. The verifier hashes
  the local archive with SHA-512 to confirm the published value. SHA-256 and
  SHA-512 are computed in a single pass over one opened file handle.
- Supplying an optional `sha512` or `size` field as an unresolved marker is an
  error on a frozen lock and a warning on a draft, so optional fields cannot be
  used to bypass a check.
- The tool does **not** claim that recorded official metadata was locally
  verified: official facts are metadata; only the local file bytes are hashed.
- A detached OpenPGP signature is represented as an ordinary fetched asset with
  `role="signature"`. **The tool does not verify signatures** and does not inspect
  key material; signature verification is a separate, explicitly authorized step.
- The rendered report prints a note making this boundary explicit: official
  release facts are not network-verified and only local assets are hashed.

## Lock format

Line-oriented UTF-8 text. `#` begins a full-line comment. A line containing `=`
before its first space is a top-level scalar; otherwise the first word names a
record and the rest is `key=value` pairs. Values with spaces must be
double-quoted; `\"` and `\\` are the only escapes, and a quoted value must be
followed by whitespace or end of line.

```text
schema=cc-silicon.torture/lock/v1
status=frozen

release gcc-release="releases/gcc-15.2.0" release-date="2025-08-08" \
        commit-sha="<40 hex>" source-archive="gcc-15.2.0.tar.xz" \
        source-archive-sha256="<64 hex>" source-archive-sha512="<128 hex>" \
        provenance="https://gcc.gnu.org/pub/gcc/releases/gcc-15.2.0/"
profile name="linux-aarch64-lp64" dialect="gnu17" \
        configuration="-O0" configuration="-O1" \
        suite="compile" suite="execute" suite="ieee"
target triple="aarch64-unknown-linux-gnu" abi="AAPCS64" format="ELF" \
       data-model="LP64" endianness="little"
substrate execution="linux-ci-or-vm" runner="<runner id>" toolchain="<toolchain id>" \
          sysroot="<sysroot id>" assembler="<assembler id>" linker="<linker id>"
policy acquisition="fetch-on-demand" vendoring="prohibited" hash-lock="sha256" \
       license-distribution="<policy text>"
reference label="non-candidate" purpose="reference-only" authorized="true" \
          used-for-candidate-pass-rate="false"
asset role="archive" path="gcc-15.2.0.tar.xz" sha256="<64 hex>" sha512="<128 hex>" \
      license="GPL-3.0-or-later" provenance="gcc-mirror/gcc@<40 hex>"
asset role="driver" path="gcc/testsuite/lib/c-torture.exp" sha256="<64 hex>" \
      license="GPL-3.0-or-later" provenance="gcc-mirror/gcc@<40 hex>" \
      suite="compile" size="<bytes>"
asset role="signature" path="gcc-15.2.0.tar.xz.sig" sha256="<64 hex>" \
      license="GPL-3.0-or-later" provenance="gcc-mirror/gcc@<40 hex>"
```

(The `\` continuations above are documentation only; each record is one physical
line.)

- Repeatable fields: `profile.configuration`, `profile.suite` (ordered).
- Asset roles: `suite-test`, `aux`, `header`, `runtime`, `driver`, `archive`,
  `signature`, `license`, `provenance`, `other`.
- Asset `sha512` is optional and, when present, checked against the local file;
  it is required on the archive asset whenever `release.source-archive-sha512` is
  published.
- **Unfilled sentinels (F8):** a value is treated as unresolved only when it is
  blank or spelled exactly `unfilled`, `tbd`, `todo`, `fixme`, or `unresolved`
  (case-insensitive), optionally wrapped as `<unfilled>`. Ordinary strings such
  as `example` or `unknown` are valid values. Sentinels are **errors** for a
  frozen lock and **warnings** for a draft lock.
- `release.source-archive` is a corpus-relative path; it is verified through its
  required `asset role="archive"` record, not twice through the release field.

## Security model and threat assumptions (F10)

- **Trust boundary:** the lock and the corpus tree are treated as untrusted
  *data*. The tool parses them defensively but does not sandbox file contents.
- **Concurrency / TOCTOU:** verification assumes the corpus tree is **not
  concurrently mutated** during a run. After canonicalizing a path and checking
  it stays within the corpus root, the tool opens the file once and hashes that
  opened handle, so the bytes compared come from a single descriptor and the path
  is not re-resolved for hashing. A writer racing the verifier between
  canonicalization and open is not fully defendable with safe `std` APIs
  (no `O_NOFOLLOW`); re-run verification on a quiescent tree for an authoritative
  result.
- **Fail-closed enumeration:** `--check-unlisted` reports every directory/entry
  read failure; an incomplete traversal is an error, never a clean result.
- **Report injection (F5):** all dynamic labels, paths, and messages are
  control-character escaped in the rendered report, so a hostile filename or
  argument cannot forge additional diagnostic lines.
- **Non-UTF-8 arguments (F9):** the CLI reads `args_os`; invalid UTF-8 is treated
  as a path and reported as an operational error, never a panic.

## Dependencies (F11)

The only **direct** dependency is [`sha2`](https://crates.io/crates/sha2) for
SHA-256. `Cargo.lock` is committed. From that lock, sha2's transitive closure is
`block-buffer`, `cfg-if`, `cpufeatures`, `crypto-common`, `digest`,
`generic-array`, `libc`, `typenum`, and `version_check`. There is no dependency on
`cc-silicon`, a parser/CLI framework, or any compiler toolchain.

## Testing

```sh
cargo fmt --manifest-path tools/torture/Cargo.toml -- --check
cargo clippy --manifest-path tools/torture/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/torture/Cargo.toml
```

Fixtures are generated by the tests at runtime under the OS temp directory. They
contain synthetic bytes only; no GCC corpus content or GPL material is copied in.
